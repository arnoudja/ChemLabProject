//! Wire-format DTOs shared between Axum and the Vite frontend (via ts-rs).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Liveness / readiness payload for `GET /api/health`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../../apps/web/src/generated/")]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub service: String,
}

impl HealthResponse {
    pub fn ok(version: impl Into<String>) -> Self {
        Self {
            status: "ok".into(),
            version: version.into(),
            service: "chemlab-server".into(),
        }
    }
}

/// Register a new account (v0.1 stub — email + password).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../../apps/web/src/generated/")]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub display_name: String,
}

/// Log in with email + password.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../../apps/web/src/generated/")]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Authenticated user as returned to the client.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../../apps/web/src/generated/")]
pub struct AuthUserResponse {
    pub id: String,
    pub email: String,
    pub display_name: String,
    #[ts(type = "string")]
    pub created_at: DateTime<Utc>,
}

/// Generic API error body.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../../apps/web/src/generated/")]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
}

/// Session probe response for `GET /api/auth/me`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../../apps/web/src/generated/")]
pub struct MeResponse {
    pub authenticated: bool,
    pub user: Option<AuthUserResponse>,
}

/// CSRF synchronizer token from `GET /api/auth/csrf` (cookie + `X-CSRF-Token` header).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../../apps/web/src/generated/")]
pub struct CsrfResponse {
    pub csrf_token: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_response_round_trips() {
        let health = HealthResponse::ok("0.1.0");
        let json = serde_json::to_string(&health).unwrap();
        let back: HealthResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(health, back);
        assert_eq!(back.status, "ok");
    }

    #[test]
    fn register_request_deserializes() {
        let raw = r#"{"email":"a@b.co","password":"secret123","display_name":"Ada"}"#;
        let req: RegisterRequest = serde_json::from_str(raw).unwrap();
        assert_eq!(req.display_name, "Ada");
    }
}
