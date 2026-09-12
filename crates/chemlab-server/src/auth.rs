//! Password hashing, opaque session cookie, and CSRF synchronizer helpers.
//!
//! TODO(v0.2+): session rotation on privilege change, rate limits on /auth/*,
//! Secure cookie default in production deploy docs.

use crate::error::ApiError;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use chrono::Duration;
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};

pub const SESSION_COOKIE: &str = "chemlab_session";
pub const CSRF_COOKIE: &str = "chemlab_csrf";
/// Clients send this header; HTTP field names are case-insensitive.
pub const CSRF_HEADER: &str = "x-csrf-token";

pub fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| e.to_string())
}

pub fn verify_password(password: &str, password_hash: &str) -> Result<bool, String> {
    let parsed = PasswordHash::new(password_hash).map_err(|e| e.to_string())?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

pub fn generate_session_token() -> String {
    let bytes: [u8; 32] = rand::random();
    hex::encode(bytes)
}

pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn session_cookie(token: &str, secure: bool, ttl: Duration) -> Cookie<'static> {
    let mut cookie = Cookie::build((SESSION_COOKIE, token.to_string()))
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(cookie::time::Duration::seconds(ttl.num_seconds()))
        .build();
    if secure {
        cookie.set_secure(true);
    }
    cookie
}

pub fn clear_session_cookie(secure: bool) -> Cookie<'static> {
    let mut cookie = Cookie::build((SESSION_COOKIE, ""))
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(cookie::time::Duration::seconds(0))
        .build();
    if secure {
        cookie.set_secure(true);
    }
    cookie
}

pub fn token_from_jar(jar: &CookieJar) -> Option<String> {
    jar.get(SESSION_COOKIE).map(|c| c.value().to_string())
}

pub fn csrf_cookie(token: &str, secure: bool, ttl: Duration) -> Cookie<'static> {
    let mut cookie = Cookie::build((CSRF_COOKIE, token.to_string()))
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(cookie::time::Duration::seconds(ttl.num_seconds()))
        .build();
    if secure {
        cookie.set_secure(true);
    }
    cookie
}

pub fn csrf_token_from_jar(jar: &CookieJar) -> Option<String> {
    jar.get(CSRF_COOKIE)
        .map(|c| c.value().to_string())
        .filter(|value| !value.is_empty())
}

pub fn csrf_tokens_match(header: &str, cookie: &str) -> bool {
    if header.is_empty() || cookie.is_empty() {
        return false;
    }
    hash_token(header) == hash_token(cookie)
}

/// Double-submit CSRF check for mutating routes (auth today, lab POSTs later).
pub async fn require_csrf(
    jar: CookieJar,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let header = request
        .headers()
        .get(CSRF_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    let cookie = csrf_token_from_jar(&jar).unwrap_or_default();
    if csrf_tokens_match(header, &cookie) {
        Ok(next.run(request).await)
    } else {
        Err(ApiError::forbidden("csrf", "Missing or invalid CSRF token"))
    }
}

pub fn validate_credentials(
    email: &str,
    password: &str,
    display_name: Option<&str>,
) -> Result<(), String> {
    let email = email.trim();
    if email.is_empty() || !email.contains('@') || email.len() > 254 {
        return Err("Enter a valid email address".into());
    }
    if password.len() < 8 {
        return Err("Password must be at least 8 characters".into());
    }
    if password.len() > 128 {
        return Err("Password is too long".into());
    }
    if let Some(name) = display_name {
        let name = name.trim();
        if name.is_empty() || name.len() > 64 {
            return Err("Display name must be 1–64 characters".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_hash_round_trip() {
        let hash = hash_password("correct-horse").unwrap();
        assert!(verify_password("correct-horse", &hash).unwrap());
        assert!(!verify_password("wrong-horse", &hash).unwrap());
    }

    #[test]
    fn token_hash_is_stable() {
        assert_eq!(hash_token("abc"), hash_token("abc"));
        assert_ne!(hash_token("abc"), hash_token("abd"));
    }

    #[test]
    fn csrf_tokens_match_rejects_empty_or_mismatch() {
        assert!(csrf_tokens_match("same-token", "same-token"));
        assert!(!csrf_tokens_match("same-token", "other-token"));
        assert!(!csrf_tokens_match("", "same-token"));
        assert!(!csrf_tokens_match("same-token", ""));
        assert!(!csrf_tokens_match("", ""));
    }
}
