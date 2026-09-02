use std::env;
use tracing::{error, info, warn};

#[cfg(feature = "ssr")]
pub async fn send_email(to_email: &str, subject: &str, body_text: &str, body_html: Option<&str>) -> Result<(), String> {
    use lettre::message::header::ContentType;
    use lettre::message::{MultiPart, SinglePart};
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

    let smtp_server = env::var("SMTP_SERVER").unwrap_or_default();
    let smtp_port: u16 = env::var("SMTP_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(465);
    let smtp_username = env::var("SMTP_USERNAME").unwrap_or_default();
    let smtp_password = env::var("SMTP_PASSWORD").unwrap_or_default();
    let smtp_from = env::var("SMTP_FROM").unwrap_or_else(|_| "support@apich.top".to_string());
    let smtp_from_name = env::var("SMTP_FROM_NAME").unwrap_or_else(|_| "SyncNote".to_string());

    if smtp_server.is_empty() || smtp_username.is_empty() || smtp_password.is_empty() {
        warn!("SMTP is not fully configured; skipping email to {}", to_email);
        return Ok(());
    }

    let from_formatted = format!("{smtp_from_name} <{smtp_from}>");
    let from_mailbox: lettre::message::Mailbox = from_formatted
        .parse()
        .map_err(|e| format!("Invalid FROM mailbox '{from_formatted}': {e}"))?;

    let to_mailbox: lettre::message::Mailbox = to_email
        .parse()
        .map_err(|e| format!("Invalid TO email '{to_email}': {e}"))?;

    let email_builder = Message::builder()
        .from(from_mailbox)
        .to(to_mailbox)
        .subject(subject);

    let email = if let Some(html) = body_html {
        email_builder
            .multipart(
                MultiPart::alternative()
                    .singlepart(
                        SinglePart::builder()
                            .header(ContentType::TEXT_PLAIN)
                            .body(body_text.to_string()),
                    )
                    .singlepart(
                        SinglePart::builder()
                            .header(ContentType::TEXT_HTML)
                            .body(html.to_string()),
                    ),
            )
            .map_err(|e| format!("Failed to build multipart email: {e}"))?
    } else {
        email_builder
            .header(ContentType::TEXT_PLAIN)
            .body(body_text.to_string())
            .map_err(|e| format!("Failed to build text email: {e}"))?
    };

    let creds = Credentials::new(smtp_username, smtp_password);

    let mailer: AsyncSmtpTransport<Tokio1Executor> = if smtp_port == 465 {
        AsyncSmtpTransport::<Tokio1Executor>::relay(&smtp_server)
            .map_err(|e| format!("Failed to create SMTP relay: {e}"))?
            .credentials(creds)
            .build()
    } else {
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&smtp_server)
            .map_err(|e| format!("Failed to create STARTTLS relay: {e}"))?
            .port(smtp_port)
            .credentials(creds)
            .build()
    };

    match mailer.send(email).await {
        Ok(_) => {
            info!("Email sent successfully to {}", to_email);
            Ok(())
        }
        Err(e) => {
            error!("Failed to send email to {}: {:?}", to_email, e);
            Err(format!("SMTP send failed: {e}"))
        }
    }
}
