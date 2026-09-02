use serde::Deserialize;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub webauthn: WebauthnConfig,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub ip: IpAddr,
    pub port: u16,
    pub site_root: String,
}

#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    pub path: PathBuf,
    pub uploads_dir: PathBuf,
}

/// Passkeys (WebAuthn) bind to the exact domain the browser sees — `rp_id`
/// must be that domain (no scheme/port), and `rp_origin` the full origin
/// (scheme + domain + port) the app is served on. Both change together when
/// you move from local dev to a real deployment.
#[derive(Debug, Deserialize)]
pub struct WebauthnConfig {
    pub rp_id: String,
    pub rp_origin: String,
}

impl AppConfig {
    /// Load `config.toml` from the current working directory.
    pub fn load() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let raw = std::fs::read_to_string("config.toml").map_err(|e| format!("cannot read config.toml: {e}"))?;
        let cfg: AppConfig = toml::from_str(&raw).map_err(|e| format!("config.toml parse error: {e}"))?;
        Ok(cfg)
    }

    pub fn site_addr(&self) -> SocketAddr {
        SocketAddr::new(self.server.ip, self.server.port)
    }
}
