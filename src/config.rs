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
    /// Overrides whether the session cookie gets the `Secure` flag. Leave
    /// unset (the default) to derive it from `webauthn.rp_origin`'s scheme —
    /// `Secure` when it's `https`, not otherwise. Only set this explicitly to
    /// `false` on a deployment that is NOT also reachable over HTTPS by real
    /// users (e.g. a LAN-only/testing instance) — forcing it off while also
    /// serving real traffic over HTTPS would let the session cookie be sent
    /// unencrypted if that traffic were ever downgraded to plain HTTP.
    #[serde(default)]
    pub cookie_secure: Option<bool>,
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
