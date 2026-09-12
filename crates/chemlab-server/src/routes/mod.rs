mod api;
mod frontend;

use crate::state::AppState;
use axum::Router;
use tower_http::trace::TraceLayer;

pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(api::router())
        .merge(frontend::router(state.clone()))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn test_app() -> Router {
        let config = Config {
            bind_addr: "127.0.0.1:0".into(),
            database_url: "sqlite::memory:?cache=shared".into(),
            static_dir: None,
            vite_dev_proxy: None,
            cookie_secure: false,
            session_ttl_hours: 24,
        };
        let state = AppState::new(&config).await.expect("state");
        router(state)
    }

    async fn body_json(response: axum::response::Response) -> serde_json::Value {
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn health_returns_ok() {
        let app = test_app().await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["status"], "ok");
        assert_eq!(json["service"], "chemlab-server");
    }

    #[tokio::test]
    async fn favicon_is_served_without_spa() {
        let app = test_app().await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/favicon.ico")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "image/svg+xml"
        );
    }

    #[tokio::test]
    async fn unknown_path_is_not_found_not_unavailable() {
        let app = test_app().await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/does-not-exist")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    fn first_set_cookie(response: &axum::response::Response) -> String {
        response
            .headers()
            .get("set-cookie")
            .expect("set-cookie")
            .to_str()
            .unwrap()
            .to_string()
    }

    fn cookie_pair(set_cookie: &str) -> String {
        set_cookie.split(';').next().unwrap().to_string()
    }

    async fn issue_csrf(app: &Router) -> (String, String) {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/auth/csrf")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let cookie = cookie_pair(&first_set_cookie(&response));
        let json = body_json(response).await;
        let token = json["csrf_token"].as_str().expect("csrf_token").to_string();
        (token, cookie)
    }

    #[tokio::test]
    async fn csrf_endpoint_sets_cookie_and_returns_token() {
        let app = test_app().await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/auth/csrf")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let set_cookie = first_set_cookie(&response);
        assert!(set_cookie.contains("chemlab_csrf="));
        assert!(set_cookie.to_ascii_lowercase().contains("httponly"));
        let json = body_json(response).await;
        let token = json["csrf_token"].as_str().expect("csrf_token");
        assert!(!token.is_empty());
        assert!(set_cookie.contains(token));
    }

    #[tokio::test]
    async fn register_without_csrf_is_forbidden() {
        let app = test_app().await;
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/register")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"email":"ada@chemlab.local","password":"secret123","display_name":"Ada"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let json = body_json(response).await;
        assert_eq!(json["code"], "csrf");
    }

    #[tokio::test]
    async fn login_without_csrf_is_forbidden() {
        let app = test_app().await;
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"email":"ada@chemlab.local","password":"secret123"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let json = body_json(response).await;
        assert_eq!(json["code"], "csrf");
    }

    #[tokio::test]
    async fn logout_without_csrf_is_forbidden() {
        let app = test_app().await;
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/logout")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let json = body_json(response).await;
        assert_eq!(json["code"], "csrf");
    }

    #[tokio::test]
    async fn register_rejects_mismatched_csrf_token() {
        let app = test_app().await;
        let csrf = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/auth/csrf")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let csrf_cookie = cookie_pair(&first_set_cookie(&csrf));

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/register")
                    .header("content-type", "application/json")
                    .header("cookie", csrf_cookie)
                    .header("x-csrf-token", "not-the-cookie-value")
                    .body(Body::from(
                        r#"{"email":"ada@chemlab.local","password":"secret123","display_name":"Ada"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let json = body_json(response).await;
        assert_eq!(json["code"], "csrf");
    }

    #[tokio::test]
    async fn me_does_not_require_csrf() {
        let app = test_app().await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/auth/me")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["authenticated"], false);
    }

    #[tokio::test]
    async fn register_login_me_logout_flow() {
        let app = test_app().await;
        let (csrf_token, csrf_cookie) = issue_csrf(&app).await;

        let register = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/register")
                    .header("content-type", "application/json")
                    .header("cookie", &csrf_cookie)
                    .header("x-csrf-token", &csrf_token)
                    .body(Body::from(
                        r#"{"email":"ada@chemlab.local","password":"secret123","display_name":"Ada"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(register.status(), StatusCode::CREATED);
        let session_cookie = cookie_pair(&first_set_cookie(&register));
        assert!(session_cookie.contains("chemlab_session="));

        let me = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/auth/me")
                    .header("cookie", &session_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(me.status(), StatusCode::OK);
        let me_json = body_json(me).await;
        assert_eq!(me_json["authenticated"], true);
        assert_eq!(me_json["user"]["display_name"], "Ada");

        let logout = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/logout")
                    .header("cookie", format!("{session_cookie}; {csrf_cookie}"))
                    .header("x-csrf-token", &csrf_token)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(logout.status(), StatusCode::NO_CONTENT);

        let login = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/login")
                    .header("content-type", "application/json")
                    .header("cookie", &csrf_cookie)
                    .header("x-csrf-token", &csrf_token)
                    .body(Body::from(
                        r#"{"email":"ada@chemlab.local","password":"secret123"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(login.status(), StatusCode::OK);
        let login_json = body_json(login).await;
        assert_eq!(login_json["email"], "ada@chemlab.local");
    }
}
