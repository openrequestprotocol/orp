use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub bind: SocketAddr,
    pub database_url: String,
    pub public_endpoint: String,
    pub domain: String,
    pub signing_seed: Option<[u8; 32]>,
    pub shared_secret: Option<String>,
    pub webhook_url: Option<String>,
    /// lettre SMTP URL for the email bridge, e.g. `smtp://host:1025`. When
    /// unset, bridged mail is queued rather than sent.
    pub smtp_url: Option<String>,
}

impl ServerConfig {
    pub fn from_env() -> Self {
        let port: u16 = std::env::var("ORP_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8787);
        let domain = std::env::var("ORP_DOMAIN").unwrap_or_else(|_| "localhost".into());
        let public_endpoint = std::env::var("ORP_PUBLIC_ENDPOINT")
            .unwrap_or_else(|_| format!("http://{domain}:{port}"));
        Self {
            bind: format!("0.0.0.0:{port}").parse().unwrap(),
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://localhost/orp".into()),
            public_endpoint,
            domain,
            signing_seed: None,
            shared_secret: std::env::var("ORP_SHARED_SECRET").ok().filter(|s| !s.is_empty()),
            webhook_url: std::env::var("ORP_WEBHOOK_URL").ok().filter(|s| !s.is_empty()),
            smtp_url: std::env::var("ORP_SMTP_URL").ok().filter(|s| !s.is_empty()),
        }
    }
}
