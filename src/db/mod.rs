use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{Executor, SqlitePool};
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

/// Read-write pool for app state: users, notes, shared pages, invites.
pub async fn open_pool<P: AsRef<Path>>(path: P) -> Result<SqlitePool, sqlx::Error> {
    let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", path.as_ref().display()))?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5));
    SqlitePoolOptions::new()
        .max_connections(8)
        .acquire_timeout(Duration::from_secs(5))
        .connect_with(opts)
        .await
}

/// Apply embedded SQL migrations. Idempotent — each migration runs at most once.
pub async fn run_migrations(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    pool.execute(
        "CREATE TABLE IF NOT EXISTS _migrations \
         (id INTEGER PRIMARY KEY, name TEXT UNIQUE NOT NULL, \
          applied_at TEXT NOT NULL DEFAULT (datetime('now')))",
    )
    .await?;

    let migrations: &[(&str, &str)] = &[
        ("0001_init", include_str!("../../migrations/0001_init.sql")),
        ("0002_attachments", include_str!("../../migrations/0002_attachments.sql")),
        ("0003_admin_invites", include_str!("../../migrations/0003_admin_invites.sql")),
        ("0004_passkeys", include_str!("../../migrations/0004_passkeys.sql")),
        ("0005_user_management", include_str!("../../migrations/0005_user_management.sql")),
        (
            "0006_attachments_owner_index",
            include_str!("../../migrations/0006_attachments_owner_index.sql"),
        ),
        (
            "0007_attachments_library_scope",
            include_str!("../../migrations/0007_attachments_library_scope.sql"),
        ),
    ];

    for (name, sql) in migrations {
        apply_migration(pool, name, sql).await?;
    }

    Ok(())
}

/// One-shot admin bootstrap. Honored only when the `users` table is empty and
/// `SYNCNOTE_BOOTSTRAP_ADMIN` is set to `email:password`. No-ops otherwise —
/// intended to be removed from the environment after first run.
pub async fn bootstrap_admin_if_requested(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let Ok(spec) = std::env::var("SYNCNOTE_BOOTSTRAP_ADMIN") else {
        return Ok(());
    };
    let Some((email, password)) = spec.split_once(':') else {
        tracing::warn!("SYNCNOTE_BOOTSTRAP_ADMIN must be email:password — ignoring");
        return Ok(());
    };

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users").fetch_one(pool).await?;
    if count.0 > 0 {
        tracing::info!("users table non-empty; skipping admin bootstrap");
        return Ok(());
    }

    use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
    use argon2::Argon2;
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| sqlx::Error::Configuration(format!("argon2: {e}").into()))?
        .to_string();

    sqlx::query("INSERT INTO users (email, password_hash, is_admin) VALUES (?, ?, 1)")
        .bind(email)
        .bind(hash)
        .execute(pool)
        .await?;
    tracing::info!("bootstrapped admin {}", email);
    Ok(())
}

async fn apply_migration(pool: &SqlitePool, name: &str, sql: &str) -> Result<(), sqlx::Error> {
    let already: Option<(i64,)> = sqlx::query_as("SELECT id FROM _migrations WHERE name = ?")
        .bind(name)
        .fetch_optional(pool)
        .await?;

    if already.is_some() {
        return Ok(());
    }

    match pool.execute(sql).await {
        Ok(_) => {}
        Err(sqlx::Error::Database(ref e))
            if e.message().contains("duplicate column name") || e.message().contains("already exists") =>
        {
            tracing::warn!("migration {name}: schema already present, marking as applied");
        }
        Err(e) => return Err(e),
    }

    sqlx::query("INSERT INTO _migrations (name) VALUES (?)")
        .bind(name)
        .execute(pool)
        .await?;

    Ok(())
}
