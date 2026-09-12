//! In-process sliding-window limiter for `/api/auth/login` and `/api/auth/register`.
//!
//! Single-node (local / Pi) is enough; do not introduce a distributed store.

use crate::error::ApiError;
use axum::extract::{ConnectInfo, FromRequestParts};
use axum::http::request::Parts;
use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Allowed login or register attempts per key per window.
pub const AUTH_RATE_LIMIT_MAX: u32 = 5;
/// Sliding window for [`AUTH_RATE_LIMIT_MAX`].
pub const AUTH_RATE_LIMIT_WINDOW: Duration = Duration::from_secs(60);

pub struct AuthRateLimiter {
    max: u32,
    window: Duration,
    hits: Mutex<HashMap<String, Vec<Instant>>>,
}

impl Default for AuthRateLimiter {
    fn default() -> Self {
        Self::new(AUTH_RATE_LIMIT_MAX, AUTH_RATE_LIMIT_WINDOW)
    }
}

impl AuthRateLimiter {
    pub fn new(max: u32, window: Duration) -> Self {
        Self {
            max,
            window,
            hits: Mutex::new(HashMap::new()),
        }
    }

    /// Record one attempt. `Err` when this key is over the window max.
    pub fn check(&self, key: &str) -> Result<(), Duration> {
        let mut hits = self.hits.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();
        let entry = hits.entry(key.to_string()).or_default();
        entry.retain(|t| now.duration_since(*t) < self.window);
        if entry.len() as u32 >= self.max {
            let retry = entry
                .first()
                .map(|oldest| self.window.saturating_sub(now.duration_since(*oldest)))
                .unwrap_or(self.window);
            return Err(retry);
        }
        entry.push(now);
        Ok(())
    }
}

pub fn client_ip_from_parts(parts: &Parts) -> String {
    if let Some(ConnectInfo(addr)) = parts.extensions.get::<ConnectInfo<SocketAddr>>() {
        return addr.ip().to_string();
    }
    parts
        .headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|ip| !ip.is_empty())
        .unwrap_or("unknown")
        .to_string()
}

/// Peer IP for auth rate limits (`ConnectInfo`, else `X-Forwarded-For`, else `unknown`).
pub struct ClientIp(pub String);

impl<S> FromRequestParts<S> for ClientIp
where
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(Self(client_ip_from_parts(parts)))
    }
}

pub fn normalize_identity(email: &str) -> String {
    email.trim().to_ascii_lowercase()
}

pub fn check_auth_rate_limit(limiter: &AuthRateLimiter, key: &str) -> Result<(), ApiError> {
    limiter.check(key).map_err(|_| {
        ApiError::too_many_requests("rate_limited", "Too many attempts. Try again later.")
    })
}

/// Count this login/register attempt against IP and email keys.
pub fn check_auth_attempt(
    limiter: &AuthRateLimiter,
    action: &str,
    ip: &str,
    email: &str,
) -> Result<(), ApiError> {
    check_auth_rate_limit(limiter, &format!("{action}:ip:{ip}"))?;
    let identity = normalize_identity(email);
    if !identity.is_empty() {
        check_auth_rate_limit(limiter, &format!("{action}:id:{identity}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_up_to_max_then_rejects() {
        let limiter = AuthRateLimiter::new(3, Duration::from_secs(60));
        assert!(limiter.check("login:ip:1.1.1.1").is_ok());
        assert!(limiter.check("login:ip:1.1.1.1").is_ok());
        assert!(limiter.check("login:ip:1.1.1.1").is_ok());
        assert!(limiter.check("login:ip:1.1.1.1").is_err());
    }

    #[test]
    fn keys_are_independent() {
        let limiter = AuthRateLimiter::new(1, Duration::from_secs(60));
        assert!(limiter.check("login:ip:1.1.1.1").is_ok());
        assert!(limiter.check("login:ip:2.2.2.2").is_ok());
        assert!(limiter.check("register:ip:1.1.1.1").is_ok());
        assert!(limiter.check("login:ip:1.1.1.1").is_err());
    }

    #[test]
    fn window_expiry_allows_another_attempt() {
        let limiter = AuthRateLimiter::new(1, Duration::from_millis(30));
        assert!(limiter.check("k").is_ok());
        assert!(limiter.check("k").is_err());
        std::thread::sleep(Duration::from_millis(40));
        assert!(limiter.check("k").is_ok());
    }

    #[test]
    fn normalize_identity_trims_and_lowercases() {
        assert_eq!(
            normalize_identity("  Ada@ChemLab.Local "),
            "ada@chemlab.local"
        );
    }
}
