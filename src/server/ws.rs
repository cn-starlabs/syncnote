use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use sqlx::SqlitePool;
use tower_sessions::Session;

use crate::auth::session as sess;
use crate::models::{MemberRole, PageEdit};
use crate::server_ctx::AppState;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(page_id): Path<i64>,
    State(state): State<AppState>,
    session: Session,
) -> impl IntoResponse {
    let user = match sess::current_user(&session, &state.pool.0).await {
        Ok(Some(u)) => u,
        _ => return StatusCode::UNAUTHORIZED.into_response(),
    };

    let role_row: Option<(String,)> =
        sqlx::query_as("SELECT role FROM shared_page_members WHERE page_id = ? AND user_id = ?")
            .bind(page_id)
            .bind(user.id)
            .fetch_optional(&state.pool.0)
            .await
            .unwrap_or(None);

    let Some(role) = role_row.and_then(|(r,)| MemberRole::parse(&r)) else {
        return StatusCode::FORBIDDEN.into_response();
    };

    ws.on_upgrade(move |socket| handle_socket(socket, page_id, role, state))
}

async fn handle_socket(socket: WebSocket, page_id: i64, role: MemberRole, state: AppState) {
    let (mut ws_tx, mut ws_rx) = socket.split();
    let room_tx = state.rooms.sender(page_id);
    let mut room_rx = room_tx.subscribe();
    let (direct_tx, mut direct_rx) = tokio::sync::mpsc::unbounded_channel::<PageEdit>();

    if let Ok(Some(current)) = fetch_current(&state.pool.0, page_id).await {
        let _ = direct_tx.send(current);
    }

    let mut send_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                edit = room_rx.recv() => {
                    match edit {
                        Ok(edit) => { if send_msg(&mut ws_tx, &edit).await.is_err() { break; } }
                        Err(_) => break,
                    }
                }
                edit = direct_rx.recv() => {
                    match edit {
                        Some(edit) => { if send_msg(&mut ws_tx, &edit).await.is_err() { break; } }
                        None => break,
                    }
                }
            }
        }
    });

    let pool = state.pool.0.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_rx.next().await {
            match msg {
                Message::Text(text) => {
                    let Ok(edit) = serde_json::from_str::<PageEdit>(&text) else { continue };
                    if !role.can_edit() {
                        continue;
                    }
                    match try_apply(&pool, page_id, &edit).await {
                        Ok(Some(new_state)) => {
                            let _ = room_tx.send(new_state);
                        }
                        Ok(None) => {
                            if let Ok(Some(current)) = fetch_current(&pool, page_id).await {
                                let _ = direct_tx.send(current);
                            }
                        }
                        Err(_) => {}
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }
}

async fn send_msg(ws_tx: &mut SplitSink<WebSocket, Message>, edit: &PageEdit) -> Result<(), axum::Error> {
    let json = serde_json::to_string(edit).unwrap_or_default();
    ws_tx.send(Message::Text(json.into())).await
}

async fn try_apply(pool: &SqlitePool, page_id: i64, edit: &PageEdit) -> Result<Option<PageEdit>, sqlx::Error> {
    let row: Option<(String, i64)> = sqlx::query_as(
        "UPDATE shared_pages SET body = ?, version = version + 1, updated_at = datetime('now') \
         WHERE id = ? AND version = ? RETURNING body, version",
    )
    .bind(&edit.body)
    .bind(page_id)
    .bind(edit.version)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(body, version)| PageEdit { body, version }))
}

async fn fetch_current(pool: &SqlitePool, page_id: i64) -> Result<Option<PageEdit>, sqlx::Error> {
    let row: Option<(String, i64)> = sqlx::query_as("SELECT body, version FROM shared_pages WHERE id = ?")
        .bind(page_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|(body, version)| PageEdit { body, version }))
}
