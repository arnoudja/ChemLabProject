use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub bind_addr: String,
    pub database_url: String,
    pub static_dir: Option<PathBuf>,
    pub vite_dev_proxy: Option<String>,
    pub cookie_secure: bool,
    pub session_ttl_hours: i64,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let bind_addr = env::var("CHEMLAB_BIND").unwrap_or_else(|_| "127.0.0.1:3847".into());
        let database_url =
            env::var("CHEMLAB_DATABASE_URL").unwrap_or_else(|_| "sqlite://chemlab.db".into());
        let static_dir = env::var("CHEMLAB_STATIC_DIR").ok().map(PathBuf::from);
        let vite_dev_proxy = env::var("CHEMLAB_VITE_PROXY").ok();
        let cookie_secure = env::var("CHEMLAB_COOKIE_SECURE")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        let session_ttl_hours = env::var("CHEMLAB_SESSION_TTL_HOURS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(24 * 14);

        Ok(Self {
            bind_addr,
            database_url,
            static_dir,
            vite_dev_proxy,
            cookie_secure,
            session_ttl_hours,
        })
    }
}
