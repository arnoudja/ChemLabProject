use crate::config::Config;
use crate::rate_limit::AuthRateLimiter;
use chemlab_db::{connect, DbPool};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub inner: Arc<AppStateInner>,
}

pub struct AppStateInner {
    pub pool: DbPool,
    pub cookie_secure: bool,
    pub session_ttl_hours: i64,
    pub static_dir: Option<std::path::PathBuf>,
    pub vite_dev_proxy: Option<String>,
    pub auth_rate_limiter: AuthRateLimiter,
}

impl AppState {
    pub async fn new(config: &Config) -> anyhow::Result<Self> {
        let pool = connect(&config.database_url).await?;
        Ok(Self {
            inner: Arc::new(AppStateInner {
                pool,
                cookie_secure: config.cookie_secure,
                session_ttl_hours: config.session_ttl_hours,
                static_dir: config.static_dir.clone(),
                vite_dev_proxy: config.vite_dev_proxy.clone(),
                auth_rate_limiter: AuthRateLimiter::default(),
            }),
        })
    }

    pub fn pool(&self) -> &DbPool {
        &self.inner.pool
    }
}
