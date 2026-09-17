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

    async fn test_app_state() -> (Router, AppState) {
        test_app_state_with(Config {
            bind_addr: "127.0.0.1:0".into(),
            database_url: "sqlite::memory:?cache=shared".into(),
            static_dir: None,
            vite_dev_proxy: None,
            cookie_secure: false,
            session_ttl_hours: 24,
        })
        .await
    }

    async fn test_app_state_with(config: Config) -> (Router, AppState) {
        let state = AppState::new(&config).await.expect("state");
        (router(state.clone()), state)
    }

    async fn test_app() -> Router {
        test_app_state().await.0
    }

    async fn body_text(response: axum::response::Response) -> String {
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    async fn body_json(response: axum::response::Response) -> serde_json::Value {
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    fn set_cookie_headers(response: &axum::response::Response) -> Vec<String> {
        response
            .headers()
            .get_all("set-cookie")
            .iter()
            .map(|value| value.to_str().unwrap().to_string())
            .collect()
    }

    fn cookie_has_secure(set_cookie: &str) -> bool {
        set_cookie
            .split(';')
            .any(|part| part.trim().eq_ignore_ascii_case("secure"))
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
        let body = body_text(response).await;
        assert!(body.contains("ChemLab frontend not configured"));
        assert!(body.contains("CHEMLAB_STATIC_DIR"));
    }

    #[tokio::test]
    async fn cookie_secure_true_sets_secure_on_csrf_session_and_clear() {
        let (app, _state) = test_app_state_with(Config {
            bind_addr: "127.0.0.1:0".into(),
            database_url: "sqlite::memory:?cache=shared".into(),
            static_dir: None,
            vite_dev_proxy: None,
            cookie_secure: true,
            session_ttl_hours: 24,
        })
        .await;

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
        assert_eq!(csrf.status(), StatusCode::OK);
        let csrf_headers = set_cookie_headers(&csrf);
        assert!(
            csrf_headers
                .iter()
                .any(|c| c.contains("chemlab_csrf=") && cookie_has_secure(c)),
            "csrf Set-Cookie should include Secure: {csrf_headers:?}"
        );
        let csrf_json = body_json(csrf).await;
        let csrf_token = csrf_json["csrf_token"].as_str().unwrap().to_string();
        let csrf_cookie = cookie_pair(
            csrf_headers
                .iter()
                .find(|c| c.contains("chemlab_csrf="))
                .unwrap(),
        );

        let register = post_register_body(
            &app,
            &csrf_token,
            &csrf_cookie,
            r#"{"email":"secure@chemlab.local","password":"secret123","display_name":"Secure"}"#
                .into(),
        )
        .await;
        assert_eq!(register.status(), StatusCode::CREATED);
        let register_cookies = set_cookie_headers(&register);
        assert!(
            register_cookies
                .iter()
                .any(|c| c.contains("chemlab_session=") && cookie_has_secure(c)),
            "session Set-Cookie should include Secure: {register_cookies:?}"
        );
        let session_cookie = session_cookie_pair(&register);
        let (csrf_token, csrf_cookie) = csrf_pair_from_response(&register);

        let logout = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/logout")
                    .header("content-type", "application/json")
                    .header("cookie", format!("{session_cookie}; {csrf_cookie}"))
                    .header("x-csrf-token", &csrf_token)
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(logout.status(), StatusCode::NO_CONTENT);
        let clear_cookies = set_cookie_headers(&logout);
        assert!(
            clear_cookies
                .iter()
                .any(|c| c.contains("chemlab_session=") && cookie_has_secure(c)),
            "clear session Set-Cookie should include Secure: {clear_cookies:?}"
        );
        assert!(
            clear_cookies
                .iter()
                .any(|c| set_cookie_clears_cookie(c, "chemlab_csrf") && cookie_has_secure(c)),
            "clear csrf Set-Cookie should include Secure: {clear_cookies:?}"
        );
    }

    #[tokio::test]
    async fn cookie_secure_false_omits_secure_attribute() {
        let app = test_app().await;
        let csrf = app
            .oneshot(
                Request::builder()
                    .uri("/api/auth/csrf")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let headers = set_cookie_headers(&csrf);
        assert!(
            headers.iter().any(|c| c.contains("chemlab_csrf=")),
            "{headers:?}"
        );
        assert!(
            headers.iter().all(|c| !cookie_has_secure(c)),
            "Secure must be omitted when cookie_secure=false: {headers:?}"
        );
    }

    #[tokio::test]
    async fn static_dir_serves_spa_index_html() {
        let dir = tempfile::tempdir().unwrap();
        let index_path = dir.path().join("index.html");
        std::fs::write(
            &index_path,
            "<!doctype html><html><body>ChemLab SPA</body></html>",
        )
        .unwrap();
        let asset_path = dir.path().join("asset.txt");
        std::fs::write(&asset_path, "asset-ok").unwrap();

        let (app, _state) = test_app_state_with(Config {
            bind_addr: "127.0.0.1:0".into(),
            database_url: "sqlite::memory:?cache=shared".into(),
            static_dir: Some(dir.path().to_path_buf()),
            vite_dev_proxy: None,
            cookie_secure: false,
            session_ttl_hours: 24,
        })
        .await;

        let root = app
            .clone()
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(root.status(), StatusCode::OK);
        assert!(body_text(root).await.contains("ChemLab SPA"));

        let asset = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/asset.txt")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(asset.status(), StatusCode::OK);
        assert_eq!(body_text(asset).await, "asset-ok");

        // Unknown SPA path falls back to index.html for client routing.
        let spa = app
            .oneshot(
                Request::builder()
                    .uri("/lab/bench")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(spa.status(), StatusCode::OK);
        assert!(body_text(spa).await.contains("ChemLab SPA"));
    }

    #[tokio::test]
    async fn static_dir_missing_index_returns_sensible_404_html() {
        let dir = tempfile::tempdir().unwrap();
        // Directory exists but has no index.html (misconfigured install).
        let (app, _state) = test_app_state_with(Config {
            bind_addr: "127.0.0.1:0".into(),
            database_url: "sqlite::memory:?cache=shared".into(),
            static_dir: Some(dir.path().to_path_buf()),
            vite_dev_proxy: None,
            cookie_secure: false,
            session_ttl_hours: 24,
        })
        .await;

        let root = app
            .clone()
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(root.status(), StatusCode::NOT_FOUND);
        let root_body = body_text(root).await;
        assert!(root_body.contains("ChemLab frontend not configured"));
        assert!(root_body.contains("index.html"));

        let missing = app
            .oneshot(
                Request::builder()
                    .uri("/anything")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(missing.status(), StatusCode::NOT_FOUND);
        assert!(body_text(missing).await.contains("CHEMLAB_STATIC_DIR"));
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

    fn without_clock(scene: &serde_json::Value) -> serde_json::Value {
        let mut scene = scene.clone();
        if let Some(obj) = scene.as_object_mut() {
            obj.remove("last_applied_unix_ms");
        }
        scene
    }

    fn cookie_pair(set_cookie: &str) -> String {
        set_cookie.split(';').next().unwrap().to_string()
    }

    fn csrf_pair_from_response(response: &axum::response::Response) -> (String, String) {
        let set_cookie = set_cookie_headers(response)
            .into_iter()
            .find(|value| value.contains("chemlab_csrf="))
            .expect("chemlab_csrf set-cookie");
        let cookie = cookie_pair(&set_cookie);
        let token = cookie
            .split_once('=')
            .expect("csrf cookie value")
            .1
            .to_string();
        (token, cookie)
    }

    fn set_cookie_clears_cookie(set_cookie: &str, name: &str) -> bool {
        set_cookie.starts_with(&format!("{name}="))
            && set_cookie.to_ascii_lowercase().contains("max-age=0")
    }

    fn session_cookie_pair(response: &axum::response::Response) -> String {
        response
            .headers()
            .get_all("set-cookie")
            .iter()
            .filter_map(|value| value.to_str().ok())
            .find(|value| value.starts_with("chemlab_session="))
            .map(cookie_pair)
            .expect("chemlab_session set-cookie")
    }

    async fn get_me(app: &Router, cookie: &str) -> serde_json::Value {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/auth/me")
                    .header("cookie", cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        body_json(response).await
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
    async fn csrf_endpoint_reuses_existing_cookie_token() {
        let app = test_app().await;
        let (token, cookie) = issue_csrf(&app).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/auth/csrf")
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.headers().get("set-cookie").is_none());
        let json = body_json(response).await;
        assert_eq!(json["csrf_token"], token);
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
        let session_cookie = session_cookie_pair(&register);
        assert!(session_cookie.contains("chemlab_session="));
        let (csrf_token, csrf_cookie) = csrf_pair_from_response(&register);

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
        let logout_cookies = set_cookie_headers(&logout);
        assert!(
            logout_cookies
                .iter()
                .any(|c| set_cookie_clears_cookie(c, "chemlab_csrf")),
            "logout should clear chemlab_csrf: {logout_cookies:?}"
        );

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

    #[tokio::test]
    async fn login_rotates_session_old_cookie_rejected_new_cookie_works() {
        let app = test_app().await;
        let (csrf_token, csrf_cookie, old_session) = register_user(&app, "ada@chemlab.local").await;
        assert_eq!(get_me(&app, &old_session).await["authenticated"], true);

        let login = post_login(
            &app,
            &csrf_token,
            &csrf_cookie,
            "ada@chemlab.local",
            "secret123",
            None,
        )
        .await;
        assert_eq!(login.status(), StatusCode::OK);
        let new_session = session_cookie_pair(&login);
        assert_ne!(new_session, old_session);

        let me_old = get_me(&app, &old_session).await;
        assert_eq!(me_old["authenticated"], false);

        let me_new = get_me(&app, &new_session).await;
        assert_eq!(me_new["authenticated"], true);
        assert_eq!(me_new["user"]["email"], "ada@chemlab.local");
    }

    #[tokio::test]
    async fn login_rotates_csrf_old_token_rejected_new_token_works() {
        let app = test_app().await;
        let (csrf_token, csrf_cookie, _session_cookie) =
            register_user(&app, "csrf-rotate@chemlab.local").await;

        let login = post_login(
            &app,
            &csrf_token,
            &csrf_cookie,
            "csrf-rotate@chemlab.local",
            "secret123",
            None,
        )
        .await;
        assert_eq!(login.status(), StatusCode::OK);
        let (new_csrf_token, new_csrf_cookie) = csrf_pair_from_response(&login);
        assert_ne!(new_csrf_token, csrf_token);
        let new_session = session_cookie_pair(&login);

        let logout_old_csrf = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/logout")
                    .header("cookie", format!("{new_session}; {new_csrf_cookie}"))
                    .header("x-csrf-token", &csrf_token)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(logout_old_csrf.status(), StatusCode::FORBIDDEN);

        let logout_new_csrf = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/logout")
                    .header("cookie", format!("{new_session}; {new_csrf_cookie}"))
                    .header("x-csrf-token", &new_csrf_token)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(logout_new_csrf.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn register_duplicate_email_returns_conflict() {
        let app = test_app().await;
        let (csrf_token, csrf_cookie, _session) = register_user(&app, "dup@chemlab.local").await;

        let response = post_register(&app, &csrf_token, &csrf_cookie, "dup@chemlab.local").await;
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert_eq!(body_json(response).await["code"], "email_taken");
    }

    #[tokio::test]
    async fn login_wrong_password_for_existing_user_is_unauthorized() {
        let app = test_app().await;
        let (csrf_token, csrf_cookie, _session) =
            register_user(&app, "wrong-pw@chemlab.local").await;

        let response = post_login(
            &app,
            &csrf_token,
            &csrf_cookie,
            "wrong-pw@chemlab.local",
            "not-the-password",
            None,
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(body_json(response).await["code"], "invalid_credentials");
    }

    #[tokio::test]
    async fn register_validation_edges_return_bad_request() {
        let app = test_app().await;
        let (csrf_token, csrf_cookie) = issue_csrf(&app).await;
        let long_password = "x".repeat(129);
        let long_name = "n".repeat(65);
        let cases = [
            r#"{"email":"not-an-email","password":"secret123","display_name":"Ada"}"#.to_string(),
            r#"{"email":"short@chemlab.local","password":"short","display_name":"Ada"}"#
                .to_string(),
            format!(
                r#"{{"email":"longpw@chemlab.local","password":"{long_password}","display_name":"Ada"}}"#
            ),
            r#"{"email":"empty-name@chemlab.local","password":"secret123","display_name":"   "}"#
                .to_string(),
            format!(
                r#"{{"email":"longname@chemlab.local","password":"secret123","display_name":"{long_name}"}}"#
            ),
        ];

        for body in cases {
            let response = post_register_body(&app, &csrf_token, &csrf_cookie, body.clone()).await;
            assert_eq!(response.status(), StatusCode::BAD_REQUEST, "body={body}");
            assert_eq!(
                body_json(response).await["code"],
                "validation",
                "body={body}"
            );
        }
    }

    #[tokio::test]
    async fn login_validation_edges_return_bad_request() {
        let app = test_app().await;
        let (csrf_token, csrf_cookie) = issue_csrf(&app).await;

        let response = post_login(
            &app,
            &csrf_token,
            &csrf_cookie,
            "not-an-email",
            "secret123",
            None,
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(body_json(response).await["code"], "validation");

        let response = post_login(
            &app,
            &csrf_token,
            &csrf_cookie,
            "short@chemlab.local",
            "short",
            None,
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(body_json(response).await["code"], "validation");
    }

    async fn post_login(
        app: &Router,
        csrf_token: &str,
        csrf_cookie: &str,
        email: &str,
        password: &str,
        forwarded_for: Option<&str>,
    ) -> axum::response::Response {
        let mut builder = Request::builder()
            .method("POST")
            .uri("/api/auth/login")
            .header("content-type", "application/json")
            .header("cookie", csrf_cookie)
            .header("x-csrf-token", csrf_token);
        if let Some(ip) = forwarded_for {
            builder = builder.header("x-forwarded-for", ip);
        }
        let body = format!(r#"{{"email":"{email}","password":"{password}"}}"#);
        app.clone()
            .oneshot(builder.body(Body::from(body)).unwrap())
            .await
            .unwrap()
    }

    async fn post_register(
        app: &Router,
        csrf_token: &str,
        csrf_cookie: &str,
        email: &str,
    ) -> axum::response::Response {
        let body = format!(r#"{{"email":"{email}","password":"secret123","display_name":"Ada"}}"#);
        post_register_body(app, csrf_token, csrf_cookie, body).await
    }

    async fn post_register_body(
        app: &Router,
        csrf_token: &str,
        csrf_cookie: &str,
        body: String,
    ) -> axum::response::Response {
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/register")
                    .header("content-type", "application/json")
                    .header("cookie", csrf_cookie)
                    .header("x-csrf-token", csrf_token)
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    async fn register_user(app: &Router, email: &str) -> (String, String, String) {
        let (csrf_token, csrf_cookie) = issue_csrf(app).await;
        let register = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/register")
                    .header("content-type", "application/json")
                    .header("cookie", &csrf_cookie)
                    .header("x-csrf-token", &csrf_token)
                    .body(Body::from(format!(
                        r#"{{"email":"{email}","password":"secret123","display_name":"Ada"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(register.status(), StatusCode::CREATED);
        let session_cookie = session_cookie_pair(&register);
        let (csrf_token, csrf_cookie) = csrf_pair_from_response(&register);
        (csrf_token, csrf_cookie, session_cookie)
    }

    fn dissolve_json(substance_id: &str, solvent_id: &str, temperature_c: i32) -> String {
        serde_json::json!({
            "substance_id": substance_id,
            "solvent_id": solvent_id,
            "temperature_c": temperature_c,
        })
        .to_string()
    }

    async fn post_dissolve(
        app: &Router,
        cookie: &str,
        csrf_token: Option<&str>,
        body: String,
    ) -> axum::response::Response {
        let mut builder = Request::builder()
            .method("POST")
            .uri("/api/lab/dissolve")
            .header("content-type", "application/json")
            .header("cookie", cookie);
        if let Some(token) = csrf_token {
            builder = builder.header("x-csrf-token", token);
        }
        app.clone()
            .oneshot(builder.body(Body::from(body)).unwrap())
            .await
            .unwrap()
    }

    async fn get_scene(app: &Router, cookie: Option<&str>) -> axum::response::Response {
        let mut builder = Request::builder().uri("/api/lab/scene");
        if let Some(cookie) = cookie {
            builder = builder.header("cookie", cookie);
        }
        app.clone()
            .oneshot(builder.body(Body::empty()).unwrap())
            .await
            .unwrap()
    }

    async fn post_action(
        app: &Router,
        cookie: &str,
        csrf_token: Option<&str>,
        action: serde_json::Value,
    ) -> axum::response::Response {
        let mut builder = Request::builder()
            .method("POST")
            .uri("/api/lab/action")
            .header("content-type", "application/json")
            .header("cookie", cookie);
        if let Some(token) = csrf_token {
            builder = builder.header("x-csrf-token", token);
        }
        app.clone()
            .oneshot(builder.body(Body::from(action.to_string())).unwrap())
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn scene_without_session_is_unauthorized() {
        let app = test_app().await;
        let response = get_scene(&app, None).await;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(body_json(response).await["code"], "unauthenticated");
    }

    #[tokio::test]
    async fn authenticated_scene_get_creates_initial_water_beaker() {
        let app = test_app().await;
        let (_csrf_token, _csrf_cookie, session_cookie) =
            register_user(&app, "scene@chemlab.local").await;
        let response = get_scene(&app, Some(&session_cookie)).await;

        assert_eq!(response.status(), StatusCode::OK);
        let scene = body_json(response).await;
        assert!(!scene["lab_id"].as_str().unwrap_or("").is_empty());
        assert_eq!(scene["version"], 0);
        let water = scene["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["id"] == "beaker-water")
            .unwrap();
        assert_eq!(water["properties"]["volume_ml"], 250.0);
        assert_eq!(water["label"], "Beaker");
        assert_eq!(water["properties"]["fill_ml"], 0.0);
        assert_eq!(water["properties"]["transparent"], true);
        assert_eq!(water["properties"]["colourless"], true);
        assert_eq!(water["properties"]["temperature_c"], 20.0);
        assert!(
            water["properties"].get("composition").is_none()
                || water["properties"]["composition"]
                    .as_array()
                    .is_some_and(|composition| composition.is_empty())
        );
    }

    #[tokio::test]
    async fn action_without_csrf_is_forbidden() {
        let app = test_app().await;
        let (_csrf_token, csrf_cookie, session_cookie) =
            register_user(&app, "no-csrf-action@chemlab.local").await;
        let response = post_action(
            &app,
            &format!("{session_cookie}; {csrf_cookie}"),
            None,
            serde_json::json!({
                "type": "use_tool",
                "tool_item_id": "spoon-1",
                "target_item_id": "beaker-nacl"
            }),
        )
        .await;

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(body_json(response).await["code"], "csrf");
    }

    #[tokio::test]
    async fn use_tool_action_persists_spoon_holding_across_get() {
        let app = test_app().await;
        let (csrf_token, csrf_cookie, session_cookie) =
            register_user(&app, "scoop@chemlab.local").await;
        let cookies = format!("{session_cookie}; {csrf_cookie}");
        let response = post_action(
            &app,
            &cookies,
            Some(&csrf_token),
            serde_json::json!({
                "type": "use_tool",
                "tool_item_id": "spoon-1",
                "target_item_id": "beaker-nacl"
            }),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(body_json(response).await["scene"]["version"], 1);

        let scene = body_json(get_scene(&app, Some(&cookies)).await).await;
        let spoon = scene["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["id"] == "spoon-1")
            .unwrap();
        assert_eq!(scene["version"], 1);
        assert_eq!(spoon["properties"]["holding"][0]["substance_id"], "nacl");
        assert_eq!(spoon["properties"]["holding"][0]["amount_g"], 0.2);
        let nacl = scene["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["id"] == "beaker-nacl")
            .unwrap();
        assert_eq!(nacl["properties"]["composition"][0]["amount_scoop"], 9);
        assert_eq!(nacl["properties"]["composition"][0]["amount_g"], 1.8);
    }

    #[tokio::test]
    async fn use_tool_put_back_restores_stock_and_clears_holding() {
        let app = test_app().await;
        let (csrf_token, csrf_cookie, session_cookie) =
            register_user(&app, "putback@chemlab.local").await;
        let cookies = format!("{session_cookie}; {csrf_cookie}");
        let scoop = serde_json::json!({
            "type": "use_tool",
            "tool_item_id": "spoon-1",
            "target_item_id": "beaker-nacl"
        });
        assert_eq!(
            post_action(&app, &cookies, Some(&csrf_token), scoop.clone())
                .await
                .status(),
            StatusCode::OK
        );

        let response = post_action(&app, &cookies, Some(&csrf_token), scoop).await;
        assert_eq!(response.status(), StatusCode::OK);
        let scene = body_json(response).await["scene"].clone();
        assert_eq!(scene["last_events"][0]["kind"], "returned");
        let spoon = scene["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["id"] == "spoon-1")
            .unwrap();
        assert!(
            spoon["properties"].get("holding").is_none()
                || spoon["properties"]["holding"]
                    .as_array()
                    .is_some_and(|holding| holding.is_empty())
        );
        let nacl = scene["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["id"] == "beaker-nacl")
            .unwrap();
        assert_eq!(nacl["properties"]["composition"][0]["amount_scoop"], 10);
        assert_eq!(nacl["properties"]["composition"][0]["amount_g"], 2.0);

        let persisted = body_json(get_scene(&app, Some(&cookies)).await).await;
        assert_eq!(without_clock(&persisted), without_clock(&scene));
    }

    #[tokio::test]
    async fn put_away_returns_scoop_to_matching_stock_for_each_solid() {
        let app = test_app().await;
        for (email, beaker_id, substance) in [
            ("putaway-nacl@chemlab.local", "beaker-nacl", "nacl"),
            ("putaway-sand@chemlab.local", "beaker-sand", "sand"),
            ("putaway-cacl2@chemlab.local", "beaker-cacl2", "cacl2"),
        ] {
            let (csrf_token, csrf_cookie, session_cookie) = register_user(&app, email).await;
            let cookies = format!("{session_cookie}; {csrf_cookie}");
            assert_eq!(
                post_action(
                    &app,
                    &cookies,
                    Some(&csrf_token),
                    serde_json::json!({
                        "type": "use_tool",
                        "tool_item_id": "spoon-1",
                        "target_item_id": beaker_id
                    }),
                )
                .await
                .status(),
                StatusCode::OK
            );

            let response = post_action(
                &app,
                &cookies,
                Some(&csrf_token),
                serde_json::json!({
                    "type": "put_away",
                    "tool_item_id": "spoon-1"
                }),
            )
            .await;
            assert_eq!(response.status(), StatusCode::OK);
            let scene = body_json(response).await["scene"].clone();
            assert_eq!(scene["last_events"][0]["kind"], "returned");
            let spoon = scene["items"]
                .as_array()
                .unwrap()
                .iter()
                .find(|item| item["id"] == "spoon-1")
                .unwrap();
            assert_eq!(spoon["location"], "bench");
            assert!(
                spoon["properties"].get("holding").is_none()
                    || spoon["properties"]["holding"]
                        .as_array()
                        .is_some_and(|holding| holding.is_empty())
            );
            let stock = scene["items"]
                .as_array()
                .unwrap()
                .iter()
                .find(|item| item["id"] == beaker_id)
                .unwrap();
            let solid = stock["properties"]["composition"]
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["substance_id"] == substance && c["phase"] == "solid")
                .unwrap();
            assert_eq!(solid["amount_scoop"], 10);
            assert_eq!(solid["amount_g"], 2.0);

            let persisted = body_json(get_scene(&app, Some(&cookies)).await).await;
            assert_eq!(without_clock(&persisted), without_clock(&scene));
        }
    }

    #[tokio::test]
    async fn put_away_empty_spoon_is_noop() {
        let app = test_app().await;
        let (csrf_token, csrf_cookie, session_cookie) =
            register_user(&app, "putaway-empty@chemlab.local").await;
        let cookies = format!("{session_cookie}; {csrf_cookie}");
        let before = body_json(get_scene(&app, Some(&cookies)).await).await;
        let response = post_action(
            &app,
            &cookies,
            Some(&csrf_token),
            serde_json::json!({
                "type": "put_away",
                "tool_item_id": "spoon-1"
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let scene = body_json(response).await["scene"].clone();
        assert!(
            scene.get("last_events").is_none()
                || scene["last_events"]
                    .as_array()
                    .is_some_and(|events| events.is_empty())
        );
        for beaker_id in ["beaker-nacl", "beaker-sand", "beaker-cacl2"] {
            let before_stock = before["items"]
                .as_array()
                .unwrap()
                .iter()
                .find(|item| item["id"] == beaker_id)
                .unwrap();
            let after_stock = scene["items"]
                .as_array()
                .unwrap()
                .iter()
                .find(|item| item["id"] == beaker_id)
                .unwrap();
            assert_eq!(
                after_stock["properties"]["composition"],
                before_stock["properties"]["composition"]
            );
        }
        let spoon = scene["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["id"] == "spoon-1")
            .unwrap();
        assert_eq!(spoon["location"], "bench");
        assert!(
            spoon["properties"].get("holding").is_none()
                || spoon["properties"]["holding"]
                    .as_array()
                    .is_some_and(|holding| holding.is_empty())
        );
    }

    #[tokio::test]
    async fn use_tool_rejects_putting_nacl_into_sand_stock() {
        let app = test_app().await;
        let (csrf_token, csrf_cookie, session_cookie) =
            register_user(&app, "wrong-stock@chemlab.local").await;
        let cookies = format!("{session_cookie}; {csrf_cookie}");
        assert_eq!(
            post_action(
                &app,
                &cookies,
                Some(&csrf_token),
                serde_json::json!({
                    "type": "use_tool",
                    "tool_item_id": "spoon-1",
                    "target_item_id": "beaker-nacl"
                }),
            )
            .await
            .status(),
            StatusCode::OK
        );
        let response = post_action(
            &app,
            &cookies,
            Some(&csrf_token),
            serde_json::json!({
                "type": "use_tool",
                "tool_item_id": "spoon-1",
                "target_item_id": "beaker-sand"
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(body_json(response).await["code"], "invalid_action");
    }

    #[tokio::test]
    async fn pour_nacl_action_persists_dissolve_scene_and_events() {
        let (app, state) = test_app_state().await;
        let (csrf_token, csrf_cookie, session_cookie) =
            register_user(&app, "pour@chemlab.local").await;
        let cookies = format!("{session_cookie}; {csrf_cookie}");
        persist_filled_main_beaker(&state, &app, &cookies).await;
        let scoop = serde_json::json!({
            "type": "use_tool",
            "tool_item_id": "spoon-1",
            "target_item_id": "beaker-nacl"
        });
        assert_eq!(
            post_action(&app, &cookies, Some(&csrf_token), scoop)
                .await
                .status(),
            StatusCode::OK
        );

        let response = post_action(
            &app,
            &cookies,
            Some(&csrf_token),
            serde_json::json!({
                "type": "pour",
                "source_item_id": "spoon-1",
                "target_item_id": "beaker-water"
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let action = body_json(response).await;
        assert_eq!(action["scene"]["version"], 2);
        assert_eq!(action["scene"]["last_events"][0]["kind"], "poured");
        assert_eq!(action["scene"]["last_events"][1]["kind"], "dissolved");
        assert_eq!(
            action["scene"]["last_events"][1]["message"],
            "Sodium chloride (NaCl) dissolves in water at bench temperature."
        );

        let persisted = body_json(get_scene(&app, Some(&cookies)).await).await;
        assert_eq!(without_clock(&persisted), without_clock(&action["scene"]));
        let water = persisted["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["id"] == "beaker-water")
            .unwrap();
        let composition = water["properties"]["composition"].as_array().unwrap();
        assert!(composition
            .iter()
            .any(|entry| entry["substance_id"] == "na+" && entry["phase"] == "aqueous"));
        assert!(composition
            .iter()
            .any(|entry| entry["substance_id"] == "cl-" && entry["phase"] == "aqueous"));
        assert!(!composition
            .iter()
            .any(|entry| entry["substance_id"] == "nacl"));
        let cooled = water["properties"]["temperature_c"].as_f64().unwrap();
        let moles = 0.2 / 58.44;
        let expected = 20.0
            - (moles * chemlab_core::NACL_DELTA_H_SOLUTION_J_PER_MOL)
                / (200.0 * chemlab_core::WATER_SPECIFIC_HEAT_J_PER_G_K);
        assert!(
            (cooled - expected).abs() < 1e-9,
            "expected cooled temperature {expected}, got {cooled}"
        );
        assert!(cooled < 20.0);
    }

    #[tokio::test]
    async fn reset_action_restores_default_scene_and_persists() {
        let (app, state) = test_app_state().await;
        let (csrf_token, csrf_cookie, session_cookie) =
            register_user(&app, "reset@chemlab.local").await;
        let cookies = format!("{session_cookie}; {csrf_cookie}");
        persist_filled_main_beaker(&state, &app, &cookies).await;
        assert_eq!(
            post_action(
                &app,
                &cookies,
                Some(&csrf_token),
                serde_json::json!({
                    "type": "use_tool",
                    "tool_item_id": "spoon-1",
                    "target_item_id": "beaker-nacl"
                }),
            )
            .await
            .status(),
            StatusCode::OK
        );
        assert_eq!(
            post_action(
                &app,
                &cookies,
                Some(&csrf_token),
                serde_json::json!({
                    "type": "pour",
                    "source_item_id": "spoon-1",
                    "target_item_id": "beaker-water"
                }),
            )
            .await
            .status(),
            StatusCode::OK
        );

        let response = post_action(
            &app,
            &cookies,
            Some(&csrf_token),
            serde_json::json!({ "type": "reset" }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let action = body_json(response).await;
        assert_eq!(action["scene"]["version"], 3);
        assert_eq!(action["scene"]["last_events"][0]["kind"], "reset");

        let water = action["scene"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["id"] == "beaker-water")
            .unwrap();
        assert_eq!(water["label"], "Beaker");
        assert_eq!(water["properties"]["fill_ml"], 0.0);
        assert!(
            water["properties"].get("composition").is_none()
                || water["properties"]["composition"]
                    .as_array()
                    .is_some_and(|composition| composition.is_empty())
        );

        let spoon = action["scene"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["id"] == "spoon-1")
            .unwrap();
        assert!(
            spoon["properties"].get("holding").is_none()
                || spoon["properties"]["holding"]
                    .as_array()
                    .is_some_and(|holding| holding.is_empty())
        );

        let persisted = body_json(get_scene(&app, Some(&cookies)).await).await;
        assert_eq!(without_clock(&persisted), without_clock(&action["scene"]));
    }

    #[tokio::test]
    async fn scene_errors_return_stable_bad_request_codes() {
        let app = test_app().await;
        let (csrf_token, csrf_cookie, session_cookie) =
            register_user(&app, "scene-errors@chemlab.local").await;
        let cookies = format!("{session_cookie}; {csrf_cookie}");
        let cases = [
            (
                serde_json::json!({
                    "type": "use_tool",
                    "tool_item_id": "missing",
                    "target_item_id": "beaker-nacl"
                }),
                "unknown_item",
            ),
            (
                serde_json::json!({
                    "type": "pour",
                    "source_item_id": "spoon-1",
                    "target_item_id": "beaker-water"
                }),
                "empty_holding",
            ),
            (
                serde_json::json!({
                    "type": "use_tool",
                    "tool_item_id": "beaker-water",
                    "target_item_id": "beaker-nacl"
                }),
                "invalid_action",
            ),
        ];

        for (action, code) in cases {
            let response = post_action(&app, &cookies, Some(&csrf_token), action).await;
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
            assert_eq!(body_json(response).await["code"], code);
        }
    }

    #[tokio::test]
    async fn pour_into_dry_beaker_returns_invalid_action() {
        let app = test_app().await;
        let (csrf_token, csrf_cookie, session_cookie) =
            register_user(&app, "dry-pour@chemlab.local").await;
        let cookies = format!("{session_cookie}; {csrf_cookie}");
        assert_eq!(
            post_action(
                &app,
                &cookies,
                Some(&csrf_token),
                serde_json::json!({
                    "type": "use_tool",
                    "tool_item_id": "spoon-1",
                    "target_item_id": "beaker-nacl"
                }),
            )
            .await
            .status(),
            StatusCode::OK
        );

        let response = post_action(
            &app,
            &cookies,
            Some(&csrf_token),
            serde_json::json!({
                "type": "pour",
                "source_item_id": "spoon-1",
                "target_item_id": "beaker-sand"
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(body_json(response).await["code"], "invalid_action");
    }

    #[tokio::test]
    async fn pour_sand_at_non_bench_temperature_leaves_undissolved_solid() {
        let (app, state) = test_app_state().await;
        let (csrf_token, csrf_cookie, session_cookie) =
            register_user(&app, "sand-warm@chemlab.local").await;
        let cookies = format!("{session_cookie}; {csrf_cookie}");
        assert_eq!(
            post_action(
                &app,
                &cookies,
                Some(&csrf_token),
                serde_json::json!({
                    "type": "use_tool",
                    "tool_item_id": "spoon-1",
                    "target_item_id": "beaker-sand"
                }),
            )
            .await
            .status(),
            StatusCode::OK
        );

        let scene_response = get_scene(&app, Some(&cookies)).await;
        assert_eq!(scene_response.status(), StatusCode::OK);
        let mut scene = body_json(scene_response).await;
        let items = scene["items"].as_array_mut().expect("items");
        let water = items
            .iter_mut()
            .find(|item| item["id"] == "beaker-water")
            .expect("beaker-water");
        water["properties"]["temperature_c"] = serde_json::json!(21.0);
        water["properties"]["fill_ml"] = serde_json::json!(200.0);
        water["properties"]["composition"] = serde_json::json!([
            {
                "substance_id": "water",
                "phase": "liquid",
                "amount_ml": 200.0
            }
        ]);
        let lab_id = scene["lab_id"].as_str().expect("lab_id").to_string();
        let version = scene["version"].as_u64().expect("version") as i64;
        let blob = serde_json::to_vec(&scene).expect("serialize scene");
        chemlab_db::save_lab_state(state.pool(), &lab_id, &blob, version)
            .await
            .expect("seed warm temperature");

        let response = post_action(
            &app,
            &cookies,
            Some(&csrf_token),
            serde_json::json!({
                "type": "pour",
                "source_item_id": "spoon-1",
                "target_item_id": "beaker-water"
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let action = body_json(response).await;
        assert_eq!(
            action["scene"]["last_events"][1]["kind"],
            "did_not_dissolve"
        );
        assert_eq!(
            action["scene"]["last_events"][1]["message"],
            "Sand (silica) does not dissolve in water at bench temperature."
        );
        let water = action["scene"]["items"]
            .as_array()
            .expect("items")
            .iter()
            .find(|item| item["id"] == "beaker-water")
            .expect("beaker-water");
        assert_eq!(water["properties"]["temperature_c"], 21.0);
        let sand_solid = water["properties"]["composition"]
            .as_array()
            .expect("composition")
            .iter()
            .find(|c| c["substance_id"] == "sand" && c["phase"] == "solid")
            .expect("solid sand leftover");
        assert_eq!(sand_solid["amount_scoop"], 1);
        assert!((sand_solid["amount_g"].as_f64().unwrap() - 0.2).abs() < 1e-12);
    }

    #[tokio::test]
    async fn pour_sand_after_cacl2_heating_succeeds() {
        let (app, state) = test_app_state().await;
        let (csrf_token, csrf_cookie, session_cookie) =
            register_user(&app, "sand-after-heat@chemlab.local").await;
        let cookies = format!("{session_cookie}; {csrf_cookie}");
        persist_filled_main_beaker(&state, &app, &cookies).await;

        // One scoop only raises T by ~0.175 °C (still rounds to 20). Pour until the
        // dissolve lookup sees a non-bench integer °C — the real warm-water bug path.
        let mut after_heat = 20.0_f64;
        for scoop_n in 1..=4 {
            assert_eq!(
                post_action(
                    &app,
                    &cookies,
                    Some(&csrf_token),
                    serde_json::json!({
                        "type": "use_tool",
                        "tool_item_id": "spoon-1",
                        "target_item_id": "beaker-cacl2"
                    }),
                )
                .await
                .status(),
                StatusCode::OK,
                "scoop cacl2 {scoop_n}"
            );
            let heat_response = post_action(
                &app,
                &cookies,
                Some(&csrf_token),
                serde_json::json!({
                    "type": "pour",
                    "source_item_id": "spoon-1",
                    "target_item_id": "beaker-water"
                }),
            )
            .await;
            assert_eq!(
                heat_response.status(),
                StatusCode::OK,
                "pour cacl2 {scoop_n}"
            );
            after_heat = body_json(heat_response).await["scene"]["items"]
                .as_array()
                .expect("items")
                .iter()
                .find(|item| item["id"] == "beaker-water")
                .expect("beaker-water")["properties"]["temperature_c"]
                .as_f64()
                .expect("temperature_c");
        }
        assert!(
            after_heat.round() as i32 != 20,
            "CaCl2 heating must leave a non-bench lookup T, got {after_heat}"
        );

        assert_eq!(
            post_action(
                &app,
                &cookies,
                Some(&csrf_token),
                serde_json::json!({
                    "type": "use_tool",
                    "tool_item_id": "spoon-1",
                    "target_item_id": "beaker-sand"
                }),
            )
            .await
            .status(),
            StatusCode::OK
        );
        let response = post_action(
            &app,
            &cookies,
            Some(&csrf_token),
            serde_json::json!({
                "type": "pour",
                "source_item_id": "spoon-1",
                "target_item_id": "beaker-water"
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let action = body_json(response).await;
        assert_eq!(
            action["scene"]["last_events"][1]["kind"],
            "did_not_dissolve"
        );
        assert_eq!(
            action["scene"]["last_events"][1]["message"],
            "Sand (silica) does not dissolve in water at bench temperature."
        );
        let water = action["scene"]["items"]
            .as_array()
            .expect("items")
            .iter()
            .find(|item| item["id"] == "beaker-water")
            .expect("beaker-water");
        assert_eq!(water["properties"]["temperature_c"], after_heat);
        let sand_solid = water["properties"]["composition"]
            .as_array()
            .expect("composition")
            .iter()
            .find(|c| c["substance_id"] == "sand" && c["phase"] == "solid")
            .expect("solid sand leftover");
        assert_eq!(sand_solid["amount_scoop"], 1);
        assert!((sand_solid["amount_g"].as_f64().unwrap() - 0.2).abs() < 1e-12);
        assert!(
            water["properties"]["composition"]
                .as_array()
                .expect("composition")
                .iter()
                .all(|c| !(c["substance_id"] == "sand" && c["phase"] == "aqueous")),
            "sand must not invent an aqueous phase"
        );
    }

    #[tokio::test]
    async fn pour_second_cacl2_scoop_after_heating_succeeds() {
        let (app, state) = test_app_state().await;
        let (csrf_token, csrf_cookie, session_cookie) =
            register_user(&app, "cacl2-hot@chemlab.local").await;
        let cookies = format!("{session_cookie}; {csrf_cookie}");
        persist_filled_main_beaker(&state, &app, &cookies).await;

        for scoop_n in 1..=2 {
            assert_eq!(
                post_action(
                    &app,
                    &cookies,
                    Some(&csrf_token),
                    serde_json::json!({
                        "type": "use_tool",
                        "tool_item_id": "spoon-1",
                        "target_item_id": "beaker-cacl2"
                    }),
                )
                .await
                .status(),
                StatusCode::OK
            );
            let response = post_action(
                &app,
                &cookies,
                Some(&csrf_token),
                serde_json::json!({
                    "type": "pour",
                    "source_item_id": "spoon-1",
                    "target_item_id": "beaker-water"
                }),
            )
            .await;
            assert_eq!(response.status(), StatusCode::OK, "pour scoop {scoop_n}");
            let action = body_json(response).await;
            assert_eq!(
                action["scene"]["last_events"][1]["kind"], "dissolved",
                "scoop {scoop_n}: {action}"
            );
            assert_eq!(
                action["scene"]["last_events"][1]["message"],
                "Calcium chloride (CaCl2) dissolves in water at bench temperature."
            );
        }

        let scene_response = get_scene(&app, Some(&cookies)).await;
        assert_eq!(scene_response.status(), StatusCode::OK);
        let scene = body_json(scene_response).await;
        let water = scene["items"]
            .as_array()
            .expect("items")
            .iter()
            .find(|item| item["id"] == "beaker-water")
            .expect("beaker-water");
        let temperature = water["properties"]["temperature_c"]
            .as_f64()
            .expect("temperature_c");
        assert!(
            temperature > 20.0,
            "two CaCl2 scoops should leave water above 20 °C, got {temperature}"
        );
        let ca_mol = water["properties"]["composition"]
            .as_array()
            .expect("composition")
            .iter()
            .find(|c| c["substance_id"] == "ca2+" && c["phase"] == "aqueous")
            .and_then(|c| c["amount_mol"].as_f64())
            .expect("ca2+ moles");
        assert!(ca_mol > 0.0);
    }

    #[tokio::test]
    async fn login_burst_is_rate_limited() {
        let app = test_app().await;
        let (csrf_token, csrf_cookie) = issue_csrf(&app).await;

        let mut last_status = StatusCode::OK;
        let mut last_json = serde_json::json!({});
        for i in 0..6 {
            let response = post_login(
                &app,
                &csrf_token,
                &csrf_cookie,
                "burst@chemlab.local",
                "wrong-password",
                Some("203.0.113.10"),
            )
            .await;
            last_status = response.status();
            last_json = body_json(response).await;
            if i < 5 {
                assert_ne!(
                    last_status,
                    StatusCode::TOO_MANY_REQUESTS,
                    "attempt {i} should not be rate-limited yet"
                );
            }
        }

        assert_eq!(last_status, StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(last_json["code"], "rate_limited");
        assert!(last_json["error"]
            .as_str()
            .unwrap_or("")
            .to_ascii_lowercase()
            .contains("too many"));
    }

    #[tokio::test]
    async fn register_burst_is_rate_limited() {
        let app = test_app().await;
        let (csrf_token, csrf_cookie) = issue_csrf(&app).await;

        let mut last_status = StatusCode::OK;
        let mut last_json = serde_json::json!({});
        for i in 0..6 {
            let response = post_register(
                &app,
                &csrf_token,
                &csrf_cookie,
                &format!("burst{i}@chemlab.local"),
            )
            .await;
            last_status = response.status();
            last_json = body_json(response).await;
            if i < 5 {
                assert_eq!(last_status, StatusCode::CREATED, "attempt {i}");
            }
        }

        assert_eq!(last_status, StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(last_json["code"], "rate_limited");
    }

    #[tokio::test]
    async fn logout_is_not_rate_limited() {
        let app = test_app().await;
        let (csrf_token, csrf_cookie) = issue_csrf(&app).await;

        for i in 0..6 {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/auth/logout")
                        .header("cookie", &csrf_cookie)
                        .header("x-csrf-token", &csrf_token)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(
                response.status(),
                StatusCode::NO_CONTENT,
                "logout {i} should stay unlimited"
            );
        }
    }

    #[tokio::test]
    async fn dissolve_without_csrf_is_forbidden() {
        let app = test_app().await;
        let (_csrf_token, csrf_cookie, session_cookie) =
            register_user(&app, "ada@chemlab.local").await;
        let response = post_dissolve(
            &app,
            &format!("{session_cookie}; {csrf_cookie}"),
            None,
            dissolve_json("nacl", "water", 20),
        )
        .await;

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let json = body_json(response).await;
        assert_eq!(json["code"], "csrf");
    }

    #[tokio::test]
    async fn dissolve_without_session_is_unauthorized() {
        let app = test_app().await;
        let (csrf_token, csrf_cookie) = issue_csrf(&app).await;
        let response = post_dissolve(
            &app,
            &csrf_cookie,
            Some(&csrf_token),
            dissolve_json("nacl", "water", 20),
        )
        .await;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let json = body_json(response).await;
        assert_eq!(json["code"], "unauthenticated");
    }

    #[tokio::test]
    async fn dissolve_returns_spec_outcomes_when_authenticated() {
        let app = test_app().await;
        let (csrf_token, csrf_cookie, session_cookie) =
            register_user(&app, "ada@chemlab.local").await;
        let cookies = format!("{session_cookie}; {csrf_cookie}");
        let cases = [
            (
                "nacl",
                "water",
                20,
                true,
                "Sodium chloride (NaCl) dissolves in water at bench temperature.",
            ),
            (
                "cacl2",
                "water",
                20,
                true,
                "Calcium chloride (CaCl2) dissolves in water at bench temperature.",
            ),
            (
                "cacl2",
                "water",
                25,
                true,
                "Calcium chloride (CaCl2) dissolves in water at bench temperature.",
            ),
            (
                "nacl",
                "water",
                19,
                true,
                "Sodium chloride (NaCl) dissolves in water at bench temperature.",
            ),
            (
                "sand",
                "water",
                20,
                false,
                "Sand (silica) does not dissolve in water at bench temperature.",
            ),
            (
                "sand",
                "water",
                21,
                false,
                "Sand (silica) does not dissolve in water at bench temperature.",
            ),
        ];

        for (substance_id, solvent_id, temperature_c, dissolved, explanation) in cases {
            let response = post_dissolve(
                &app,
                &cookies,
                Some(&csrf_token),
                dissolve_json(substance_id, solvent_id, temperature_c),
            )
            .await;
            assert_eq!(
                response.status(),
                StatusCode::OK,
                "{substance_id}/{solvent_id}/{temperature_c}"
            );
            let json = body_json(response).await;
            assert_eq!(
                json["dissolved"].as_bool(),
                Some(dissolved),
                "{substance_id}/{solvent_id}/{temperature_c}"
            );
            assert_eq!(
                json["explanation"].as_str(),
                Some(explanation),
                "{substance_id}/{solvent_id}/{temperature_c}"
            );
        }
    }

    #[tokio::test]
    async fn dissolve_errors_return_spec_codes() {
        let app = test_app().await;
        let (csrf_token, csrf_cookie, session_cookie) =
            register_user(&app, "lab@chemlab.local").await;
        let cookies = format!("{session_cookie}; {csrf_cookie}");
        let cases = [
            ("sugar", "water", 20, "unknown_substance"),
            ("NaCl", "water", 20, "unknown_substance"),
            (" nacl ", "water", 20, "unknown_substance"),
            ("nacl", "ethanol", 20, "unsupported_solvent"),
            ("", "water", 20, "invalid_input"),
            ("nacl", " ", 20, "invalid_input"),
        ];

        for (substance_id, solvent_id, temperature_c, code) in cases {
            let response = post_dissolve(
                &app,
                &cookies,
                Some(&csrf_token),
                dissolve_json(substance_id, solvent_id, temperature_c),
            )
            .await;
            assert_eq!(
                response.status(),
                StatusCode::BAD_REQUEST,
                "{substance_id:?}/{solvent_id:?}/{temperature_c}"
            );
            let json = body_json(response).await;
            assert_eq!(
                json["code"], code,
                "{substance_id:?}/{solvent_id:?}/{temperature_c}"
            );
        }
    }

    fn scene_item<'a>(scene: &'a serde_json::Value, id: &str) -> &'a serde_json::Value {
        scene["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["id"] == id)
            .unwrap_or_else(|| panic!("missing item {id}"))
    }

    fn set_main_beaker_water(scene: &mut serde_json::Value, amount_ml: f64) {
        let water = scene["items"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|item| item["id"] == "beaker-water")
            .expect("beaker-water");
        water["properties"]["fill_ml"] = serde_json::json!(amount_ml);
        water["properties"]["composition"] = serde_json::json!([
            {
                "substance_id": "water",
                "phase": "liquid",
                "amount_ml": amount_ml
            }
        ]);
    }

    async fn save_scene_blob(state: &AppState, scene: &serde_json::Value) {
        let lab_id = scene["lab_id"].as_str().unwrap().to_string();
        let version = scene["version"].as_u64().unwrap() as i64;
        chemlab_db::save_lab_state(
            state.pool(),
            &lab_id,
            &serde_json::to_vec(scene).unwrap(),
            version,
        )
        .await
        .unwrap();
    }

    async fn persist_filled_main_beaker(state: &AppState, app: &Router, cookies: &str) {
        let mut scene = body_json(get_scene(app, Some(cookies)).await).await;
        set_main_beaker_water(&mut scene, 200.0);
        save_scene_blob(state, &scene).await;
    }

    async fn pipette_one_ml_into_dish(app: &Router, cookies: &str, csrf_token: &str) {
        assert_eq!(
            post_action(
                app,
                cookies,
                Some(csrf_token),
                serde_json::json!({
                    "type": "use_tool",
                    "tool_item_id": "pipette-1",
                    "target_item_id": "beaker-h2o"
                }),
            )
            .await
            .status(),
            StatusCode::OK
        );
        assert_eq!(
            post_action(
                app,
                cookies,
                Some(csrf_token),
                serde_json::json!({
                    "type": "pour",
                    "source_item_id": "pipette-1",
                    "target_item_id": "dish-1"
                }),
            )
            .await
            .status(),
            StatusCode::OK
        );
    }

    #[tokio::test]
    async fn toggle_burner_without_csrf_is_forbidden() {
        let app = test_app().await;
        let (_csrf_token, csrf_cookie, session_cookie) =
            register_user(&app, "burner-csrf@chemlab.local").await;
        let response = post_action(
            &app,
            &format!("{session_cookie}; {csrf_cookie}"),
            None,
            serde_json::json!({
                "type": "toggle_burner",
                "burner_item_id": "burner-1"
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(body_json(response).await["code"], "csrf");
    }

    #[tokio::test]
    async fn toggle_burner_without_session_is_unauthorized() {
        let app = test_app().await;
        let (csrf_token, csrf_cookie) = issue_csrf(&app).await;
        let response = post_action(
            &app,
            &csrf_cookie,
            Some(&csrf_token),
            serde_json::json!({
                "type": "toggle_burner",
                "burner_item_id": "burner-1"
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(body_json(response).await["code"], "unauthenticated");
    }

    #[tokio::test]
    async fn toggle_burner_with_csrf_and_session_turns_on_when_dish_has_liquid() {
        let app = test_app().await;
        let (csrf_token, csrf_cookie, session_cookie) =
            register_user(&app, "burner-on@chemlab.local").await;
        let cookies = format!("{session_cookie}; {csrf_cookie}");
        pipette_one_ml_into_dish(&app, &cookies, &csrf_token).await;

        let response = post_action(
            &app,
            &cookies,
            Some(&csrf_token),
            serde_json::json!({
                "type": "toggle_burner",
                "burner_item_id": "burner-1"
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let scene = body_json(response).await["scene"].clone();
        assert_eq!(scene_item(&scene, "burner-1")["properties"]["on"], true);
        assert_eq!(scene["last_events"][0]["kind"], "toggled");
    }

    #[tokio::test]
    async fn get_scene_applies_elapsed_heat_while_burner_on() {
        let (app, state) = test_app_state().await;
        let (csrf_token, csrf_cookie, session_cookie) =
            register_user(&app, "elapsed-get@chemlab.local").await;
        let cookies = format!("{session_cookie}; {csrf_cookie}");
        pipette_one_ml_into_dish(&app, &cookies, &csrf_token).await;
        assert_eq!(
            post_action(
                &app,
                &cookies,
                Some(&csrf_token),
                serde_json::json!({
                    "type": "toggle_burner",
                    "burner_item_id": "burner-1"
                }),
            )
            .await
            .status(),
            StatusCode::OK
        );

        let scene_response = get_scene(&app, Some(&cookies)).await;
        assert_eq!(scene_response.status(), StatusCode::OK);
        let mut scene = body_json(scene_response).await;
        let past_ms = chrono::Utc::now().timestamp_millis() - 1500;
        scene["last_applied_unix_ms"] = serde_json::json!(past_ms);
        let lab_id = scene["lab_id"].as_str().expect("lab_id").to_string();
        let version = scene["version"].as_u64().expect("version") as i64;
        let blob = serde_json::to_vec(&scene).expect("serialize scene");
        chemlab_db::save_lab_state(state.pool(), &lab_id, &blob, version)
            .await
            .expect("seed last_applied");

        let response = get_scene(&app, Some(&cookies)).await;
        assert_eq!(response.status(), StatusCode::OK);
        let after = body_json(response).await;
        let temperature = scene_item(&after, "dish-1")["properties"]["temperature_c"]
            .as_f64()
            .expect("temperature_c");
        let expected = 20.0 + chemlab_core::HEAT_K_PER_S * 1.5;
        assert!(
            (temperature - expected).abs() < 1.0,
            "GET should apply ~1.5 s of heat: expected ~{expected}, got {temperature}"
        );
        assert!(
            after["last_applied_unix_ms"].as_i64().unwrap_or(0) >= past_ms + 1500,
            "GET must persist an updated last_applied_unix_ms"
        );
        let water_ml = scene_item(&after, "dish-1")["properties"]["composition"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["substance_id"] == "water")
            .and_then(|c| c["amount_ml"].as_f64())
            .unwrap_or(0.0);
        assert!(
            (water_ml - 1.0).abs() < 1e-6,
            "no evaporation below 100 °C, got {water_ml} ml"
        );
    }

    #[tokio::test]
    async fn post_action_applies_elapsed_heat_before_user_action() {
        let (app, state) = test_app_state().await;
        let (csrf_token, csrf_cookie, session_cookie) =
            register_user(&app, "elapsed-post@chemlab.local").await;
        let cookies = format!("{session_cookie}; {csrf_cookie}");
        pipette_one_ml_into_dish(&app, &cookies, &csrf_token).await;
        assert_eq!(
            post_action(
                &app,
                &cookies,
                Some(&csrf_token),
                serde_json::json!({
                    "type": "toggle_burner",
                    "burner_item_id": "burner-1"
                }),
            )
            .await
            .status(),
            StatusCode::OK
        );

        let scene_response = get_scene(&app, Some(&cookies)).await;
        let mut scene = body_json(scene_response).await;
        let past_ms = chrono::Utc::now().timestamp_millis() - 1500;
        scene["last_applied_unix_ms"] = serde_json::json!(past_ms);
        let lab_id = scene["lab_id"].as_str().expect("lab_id").to_string();
        let version = scene["version"].as_u64().expect("version") as i64;
        chemlab_db::save_lab_state(
            state.pool(),
            &lab_id,
            &serde_json::to_vec(&scene).unwrap(),
            version,
        )
        .await
        .expect("seed last_applied");

        let response = post_action(
            &app,
            &cookies,
            Some(&csrf_token),
            serde_json::json!({
                "type": "put_away",
                "tool_item_id": "pipette-1"
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let after = body_json(response).await["scene"].clone();
        let temperature = scene_item(&after, "dish-1")["properties"]["temperature_c"]
            .as_f64()
            .expect("temperature_c");
        let expected = 20.0 + chemlab_core::HEAT_K_PER_S * 1.5;
        assert!(
            (temperature - expected).abs() < 1.0,
            "POST should apply elapsed heat before the action: expected ~{expected}, got {temperature}"
        );
        assert_eq!(scene_item(&after, "burner-1")["properties"]["on"], true);
    }

    #[tokio::test]
    async fn get_scene_clamps_elapsed_time_to_two_seconds() {
        let (app, state) = test_app_state().await;
        let (csrf_token, csrf_cookie, session_cookie) =
            register_user(&app, "elapsed-clamp@chemlab.local").await;
        let cookies = format!("{session_cookie}; {csrf_cookie}");
        pipette_one_ml_into_dish(&app, &cookies, &csrf_token).await;
        assert_eq!(
            post_action(
                &app,
                &cookies,
                Some(&csrf_token),
                serde_json::json!({
                    "type": "toggle_burner",
                    "burner_item_id": "burner-1"
                }),
            )
            .await
            .status(),
            StatusCode::OK
        );

        let mut scene = body_json(get_scene(&app, Some(&cookies)).await).await;
        let past_ms = chrono::Utc::now().timestamp_millis() - 10_000;
        scene["last_applied_unix_ms"] = serde_json::json!(past_ms);
        let lab_id = scene["lab_id"].as_str().expect("lab_id").to_string();
        let version = scene["version"].as_u64().expect("version") as i64;
        chemlab_db::save_lab_state(
            state.pool(),
            &lab_id,
            &serde_json::to_vec(&scene).unwrap(),
            version,
        )
        .await
        .unwrap();

        let after = body_json(get_scene(&app, Some(&cookies)).await).await;
        let temperature = scene_item(&after, "dish-1")["properties"]["temperature_c"]
            .as_f64()
            .expect("temperature_c");
        let expected = 20.0 + chemlab_core::HEAT_K_PER_S * 2.0;
        assert!(
            (temperature - expected).abs() < 1.0,
            "elapsed dt must clamp to 2 s: expected ~{expected}, got {temperature}"
        );
    }

    #[tokio::test]
    async fn tongs_use_tool_without_csrf_is_forbidden() {
        let app = test_app().await;
        let (_csrf_token, csrf_cookie, session_cookie) =
            register_user(&app, "tongs-csrf-use@chemlab.local").await;
        let response = post_action(
            &app,
            &format!("{session_cookie}; {csrf_cookie}"),
            None,
            serde_json::json!({
                "type": "use_tool",
                "tool_item_id": "tongs-1",
                "target_item_id": "beaker-water"
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(body_json(response).await["code"], "csrf");
    }

    #[tokio::test]
    async fn tongs_use_tool_without_session_is_unauthorized() {
        let app = test_app().await;
        let (csrf_token, csrf_cookie) = issue_csrf(&app).await;
        let response = post_action(
            &app,
            &csrf_cookie,
            Some(&csrf_token),
            serde_json::json!({
                "type": "use_tool",
                "tool_item_id": "tongs-1",
                "target_item_id": "beaker-water"
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(body_json(response).await["code"], "unauthenticated");
    }

    #[tokio::test]
    async fn tongs_put_away_without_csrf_is_forbidden() {
        let app = test_app().await;
        let (_csrf_token, csrf_cookie, session_cookie) =
            register_user(&app, "tongs-csrf-putaway@chemlab.local").await;
        let response = post_action(
            &app,
            &format!("{session_cookie}; {csrf_cookie}"),
            None,
            serde_json::json!({
                "type": "put_away",
                "tool_item_id": "tongs-1"
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(body_json(response).await["code"], "csrf");
    }

    #[tokio::test]
    async fn tongs_put_away_without_session_is_unauthorized() {
        let app = test_app().await;
        let (csrf_token, csrf_cookie) = issue_csrf(&app).await;
        let response = post_action(
            &app,
            &csrf_cookie,
            Some(&csrf_token),
            serde_json::json!({
                "type": "put_away",
                "tool_item_id": "tongs-1"
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(body_json(response).await["code"], "unauthenticated");
    }

    #[tokio::test]
    async fn tongs_use_tool_and_put_away_with_csrf_and_session_pick_up_and_restore_water() {
        let app = test_app().await;
        let (csrf_token, csrf_cookie, session_cookie) =
            register_user(&app, "tongs-ok@chemlab.local").await;
        let cookies = format!("{session_cookie}; {csrf_cookie}");

        let pickup = post_action(
            &app,
            &cookies,
            Some(&csrf_token),
            serde_json::json!({
                "type": "use_tool",
                "tool_item_id": "tongs-1",
                "target_item_id": "beaker-water"
            }),
        )
        .await;
        assert_eq!(pickup.status(), StatusCode::OK);
        let picked = body_json(pickup).await["scene"].clone();
        assert_eq!(scene_item(&picked, "beaker-water")["location"], "held");
        assert_eq!(
            scene_item(&picked, "tongs-1")["properties"]["source_item_id"],
            "beaker-water"
        );

        let persisted = body_json(get_scene(&app, Some(&cookies)).await).await;
        assert_eq!(
            without_clock(&persisted)["items"],
            without_clock(&picked)["items"]
        );

        let put_away = post_action(
            &app,
            &cookies,
            Some(&csrf_token),
            serde_json::json!({
                "type": "put_away",
                "tool_item_id": "tongs-1"
            }),
        )
        .await;
        assert_eq!(put_away.status(), StatusCode::OK);
        let restored = body_json(put_away).await["scene"].clone();
        assert_eq!(scene_item(&restored, "beaker-water")["location"], "bench");
        assert_eq!(scene_item(&restored, "tongs-1")["location"], "bench");
        assert!(
            scene_item(&restored, "tongs-1")["properties"]
                .get("source_item_id")
                .is_none()
                || scene_item(&restored, "tongs-1")["properties"]["source_item_id"].is_null()
        );
    }

    async fn persist_scene_without_tongs(state: &AppState, app: &Router, cookies: &str) {
        let mut scene = body_json(get_scene(app, Some(cookies)).await).await;
        // Old labs had a filled "Water" beaker; backfill must not wipe that fill.
        scene["items"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|item| item["id"] == "beaker-water")
            .expect("beaker-water")["label"] = serde_json::json!("Water");
        set_main_beaker_water(&mut scene, 200.0);
        scene["items"]
            .as_array_mut()
            .unwrap()
            .retain(|item| item["id"] != "tongs-1");
        assert!(scene["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item["id"] != "tongs-1"));
        save_scene_blob(state, &scene).await;
    }

    #[tokio::test]
    async fn get_scene_backfills_tongs_on_persisted_lab_from_before_tongs() {
        let (app, state) = test_app_state().await;
        let (_csrf_token, csrf_cookie, session_cookie) =
            register_user(&app, "tongs-migrate-get@chemlab.local").await;
        let cookies = format!("{session_cookie}; {csrf_cookie}");
        persist_scene_without_tongs(&state, &app, &cookies).await;

        let loaded = body_json(get_scene(&app, Some(&cookies)).await).await;
        assert_eq!(scene_item(&loaded, "tongs-1")["kind"], "tongs");
        assert_eq!(scene_item(&loaded, "tongs-1")["location"], "bench");
        assert_eq!(
            scene_item(&loaded, "beaker-water")["properties"]["composition"][0]["amount_ml"],
            200.0
        );
        assert_eq!(scene_item(&loaded, "beaker-water")["label"], "Water");
    }

    #[tokio::test]
    async fn tongs_use_tool_on_persisted_lab_without_tongs_picks_up_water() {
        let (app, state) = test_app_state().await;
        let (csrf_token, csrf_cookie, session_cookie) =
            register_user(&app, "tongs-migrate-use@chemlab.local").await;
        let cookies = format!("{session_cookie}; {csrf_cookie}");
        persist_scene_without_tongs(&state, &app, &cookies).await;

        let pickup = post_action(
            &app,
            &cookies,
            Some(&csrf_token),
            serde_json::json!({
                "type": "use_tool",
                "tool_item_id": "tongs-1",
                "target_item_id": "beaker-water"
            }),
        )
        .await;
        assert_eq!(pickup.status(), StatusCode::OK);
        let picked = body_json(pickup).await["scene"].clone();
        assert_eq!(scene_item(&picked, "beaker-water")["location"], "held");
        assert_eq!(
            scene_item(&picked, "tongs-1")["properties"]["source_item_id"],
            "beaker-water"
        );
    }

    async fn persist_dry_dish_solids(state: &AppState, app: &Router, cookies: &str) {
        let mut scene = body_json(get_scene(app, Some(cookies)).await).await;
        let dish = scene["items"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|item| item["id"] == "dish-1")
            .expect("dish-1");
        dish["properties"]["composition"] = serde_json::json!([
            {
                "substance_id": "nacl",
                "phase": "solid",
                "amount_g": 0.6
            },
            {
                "substance_id": "cacl2",
                "phase": "solid",
                "amount_g": 0.4
            }
        ]);
        dish["properties"]["fill_ml"] = serde_json::json!(0.0);
        let lab_id = scene["lab_id"].as_str().unwrap().to_string();
        let version = scene["version"].as_u64().unwrap() as i64;
        chemlab_db::save_lab_state(
            state.pool(),
            &lab_id,
            &serde_json::to_vec(&scene).unwrap(),
            version,
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn dish_spoon_use_tool_without_csrf_is_forbidden() {
        let app = test_app().await;
        let (_csrf_token, csrf_cookie, session_cookie) =
            register_user(&app, "dish-spoon-csrf-use@chemlab.local").await;
        let response = post_action(
            &app,
            &format!("{session_cookie}; {csrf_cookie}"),
            None,
            serde_json::json!({
                "type": "use_tool",
                "tool_item_id": "spoon-1",
                "target_item_id": "dish-1"
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(body_json(response).await["code"], "csrf");
    }

    #[tokio::test]
    async fn dish_spoon_use_tool_without_session_is_unauthorized() {
        let app = test_app().await;
        let (csrf_token, csrf_cookie) = issue_csrf(&app).await;
        let response = post_action(
            &app,
            &csrf_cookie,
            Some(&csrf_token),
            serde_json::json!({
                "type": "use_tool",
                "tool_item_id": "spoon-1",
                "target_item_id": "dish-1"
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(body_json(response).await["code"], "unauthenticated");
    }

    #[tokio::test]
    async fn dish_spoon_put_away_without_csrf_is_forbidden() {
        let app = test_app().await;
        let (_csrf_token, csrf_cookie, session_cookie) =
            register_user(&app, "dish-spoon-csrf-putaway@chemlab.local").await;
        let response = post_action(
            &app,
            &format!("{session_cookie}; {csrf_cookie}"),
            None,
            serde_json::json!({
                "type": "put_away",
                "tool_item_id": "spoon-1"
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(body_json(response).await["code"], "csrf");
    }

    #[tokio::test]
    async fn dish_spoon_put_away_without_session_is_unauthorized() {
        let app = test_app().await;
        let (csrf_token, csrf_cookie) = issue_csrf(&app).await;
        let response = post_action(
            &app,
            &csrf_cookie,
            Some(&csrf_token),
            serde_json::json!({
                "type": "put_away",
                "tool_item_id": "spoon-1"
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(body_json(response).await["code"], "unauthenticated");
    }

    #[tokio::test]
    async fn dish_spoon_use_tool_and_put_away_with_csrf_and_session_scoop_and_restore() {
        let (app, state) = test_app_state().await;
        let (csrf_token, csrf_cookie, session_cookie) =
            register_user(&app, "dish-spoon-ok@chemlab.local").await;
        let cookies = format!("{session_cookie}; {csrf_cookie}");
        persist_dry_dish_solids(&state, &app, &cookies).await;

        let scoop = post_action(
            &app,
            &cookies,
            Some(&csrf_token),
            serde_json::json!({
                "type": "use_tool",
                "tool_item_id": "spoon-1",
                "target_item_id": "dish-1"
            }),
        )
        .await;
        assert_eq!(scoop.status(), StatusCode::OK);
        let scooped = body_json(scoop).await["scene"].clone();
        let spoon = scene_item(&scooped, "spoon-1");
        assert_eq!(spoon["location"], "hand");
        assert_eq!(spoon["properties"]["source_item_id"], "dish-1");
        let holding = spoon["properties"]["holding"].as_array().unwrap();
        assert_eq!(holding.len(), 2);
        let nacl_g = holding
            .iter()
            .find(|entry| entry["substance_id"] == "nacl")
            .unwrap()["amount_g"]
            .as_f64()
            .unwrap();
        let cacl2_g = holding
            .iter()
            .find(|entry| entry["substance_id"] == "cacl2")
            .unwrap()["amount_g"]
            .as_f64()
            .unwrap();
        assert!((nacl_g - 0.12).abs() < 1e-9);
        assert!((cacl2_g - 0.08).abs() < 1e-9);

        let persisted = body_json(get_scene(&app, Some(&cookies)).await).await;
        assert_eq!(
            scene_item(&persisted, "spoon-1")["properties"]["source_item_id"],
            "dish-1"
        );

        let put_away = post_action(
            &app,
            &cookies,
            Some(&csrf_token),
            serde_json::json!({
                "type": "put_away",
                "tool_item_id": "spoon-1"
            }),
        )
        .await;
        assert_eq!(put_away.status(), StatusCode::OK);
        let restored = body_json(put_away).await["scene"].clone();
        let spoon = scene_item(&restored, "spoon-1");
        assert_eq!(spoon["location"], "bench");
        assert!(
            spoon["properties"].get("source_item_id").is_none()
                || spoon["properties"]["source_item_id"].is_null()
        );
        assert!(
            spoon["properties"].get("holding").is_none()
                || spoon["properties"]["holding"]
                    .as_array()
                    .is_some_and(|holding| holding.is_empty())
        );
        let dish_nacl = scene_item(&restored, "dish-1")["properties"]["composition"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["substance_id"] == "nacl" && entry["phase"] == "solid")
            .unwrap()["amount_g"]
            .as_f64()
            .unwrap();
        let dish_cacl2 = scene_item(&restored, "dish-1")["properties"]["composition"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["substance_id"] == "cacl2" && entry["phase"] == "solid")
            .unwrap()["amount_g"]
            .as_f64()
            .unwrap();
        assert!((dish_nacl - 0.6).abs() < 1e-9);
        assert!((dish_cacl2 - 0.4).abs() < 1e-9);
        assert_eq!(
            scene_item(&restored, "beaker-nacl")["properties"]["composition"][0]["amount_g"],
            2.0
        );
    }

    #[tokio::test]
    async fn authenticated_scene_get_includes_full_distilled_water_beaker() {
        let app = test_app().await;
        let (_csrf_token, _csrf_cookie, session_cookie) =
            register_user(&app, "h2o-scene@chemlab.local").await;
        let response = get_scene(&app, Some(&session_cookie)).await;
        assert_eq!(response.status(), StatusCode::OK);
        let scene = body_json(response).await;
        let h2o = scene_item(&scene, "beaker-h2o");
        assert_eq!(h2o["kind"], "beaker");
        assert_eq!(h2o["label"], "Distilled water");
        assert_eq!(h2o["location"], "bench");
        assert_eq!(h2o["properties"]["volume_ml"], 100.0);
        assert_eq!(h2o["properties"]["fill_ml"], 100.0);
        assert_eq!(h2o["properties"]["composition"][0]["substance_id"], "water");
        assert_eq!(h2o["properties"]["composition"][0]["phase"], "liquid");
        assert_eq!(h2o["properties"]["composition"][0]["amount_ml"], 100.0);
    }

    async fn persist_scene_without_beaker_h2o(state: &AppState, app: &Router, cookies: &str) {
        let mut scene = body_json(get_scene(app, Some(cookies)).await).await;
        scene["items"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|item| item["id"] == "beaker-water")
            .expect("beaker-water")["label"] = serde_json::json!("Water");
        set_main_beaker_water(&mut scene, 200.0);
        scene["items"]
            .as_array_mut()
            .unwrap()
            .retain(|item| item["id"] != "beaker-h2o");
        assert!(scene["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item["id"] != "beaker-h2o"));
        save_scene_blob(state, &scene).await;
    }

    #[tokio::test]
    async fn get_scene_backfills_beaker_h2o_on_persisted_lab_from_before_distilled_water() {
        let (app, state) = test_app_state().await;
        let (_csrf_token, csrf_cookie, session_cookie) =
            register_user(&app, "h2o-migrate-get@chemlab.local").await;
        let cookies = format!("{session_cookie}; {csrf_cookie}");
        persist_scene_without_beaker_h2o(&state, &app, &cookies).await;

        let loaded = body_json(get_scene(&app, Some(&cookies)).await).await;
        assert_eq!(scene_item(&loaded, "beaker-h2o")["kind"], "beaker");
        assert_eq!(scene_item(&loaded, "beaker-h2o")["location"], "bench");
        assert_eq!(
            scene_item(&loaded, "beaker-h2o")["properties"]["composition"][0]["amount_ml"],
            100.0
        );
        assert_eq!(
            scene_item(&loaded, "beaker-water")["properties"]["composition"][0]["amount_ml"],
            200.0
        );
        assert_eq!(scene_item(&loaded, "beaker-water")["label"], "Water");
    }

    #[tokio::test]
    async fn tongs_use_tool_picks_up_beaker_h2o() {
        let app = test_app().await;
        let (csrf_token, csrf_cookie, session_cookie) =
            register_user(&app, "h2o-tongs-pick@chemlab.local").await;
        let cookies = format!("{session_cookie}; {csrf_cookie}");
        let pickup = post_action(
            &app,
            &cookies,
            Some(&csrf_token),
            serde_json::json!({
                "type": "use_tool",
                "tool_item_id": "tongs-1",
                "target_item_id": "beaker-h2o"
            }),
        )
        .await;
        assert_eq!(pickup.status(), StatusCode::OK);
        let picked = body_json(pickup).await["scene"].clone();
        assert_eq!(scene_item(&picked, "beaker-h2o")["location"], "held");
        assert_eq!(
            scene_item(&picked, "tongs-1")["properties"]["source_item_id"],
            "beaker-h2o"
        );
    }

    #[tokio::test]
    async fn beaker_h2o_use_tool_without_csrf_is_forbidden() {
        let app = test_app().await;
        let (_csrf_token, csrf_cookie, session_cookie) =
            register_user(&app, "h2o-csrf-use@chemlab.local").await;
        let response = post_action(
            &app,
            &format!("{session_cookie}; {csrf_cookie}"),
            None,
            serde_json::json!({
                "type": "use_tool",
                "tool_item_id": "tongs-1",
                "target_item_id": "beaker-h2o"
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(body_json(response).await["code"], "csrf");
    }

    #[tokio::test]
    async fn beaker_h2o_use_tool_without_session_is_unauthorized() {
        let app = test_app().await;
        let (csrf_token, csrf_cookie) = issue_csrf(&app).await;
        let response = post_action(
            &app,
            &csrf_cookie,
            Some(&csrf_token),
            serde_json::json!({
                "type": "use_tool",
                "tool_item_id": "tongs-1",
                "target_item_id": "beaker-h2o"
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(body_json(response).await["code"], "unauthenticated");
    }

    #[tokio::test]
    async fn beaker_h2o_put_away_without_csrf_is_forbidden() {
        let app = test_app().await;
        let (_csrf_token, csrf_cookie, session_cookie) =
            register_user(&app, "h2o-csrf-putaway@chemlab.local").await;
        let response = post_action(
            &app,
            &format!("{session_cookie}; {csrf_cookie}"),
            None,
            serde_json::json!({
                "type": "put_away",
                "tool_item_id": "tongs-1"
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(body_json(response).await["code"], "csrf");
    }

    #[tokio::test]
    async fn beaker_h2o_put_away_without_session_is_unauthorized() {
        let app = test_app().await;
        let (csrf_token, csrf_cookie) = issue_csrf(&app).await;
        let response = post_action(
            &app,
            &csrf_cookie,
            Some(&csrf_token),
            serde_json::json!({
                "type": "put_away",
                "tool_item_id": "tongs-1"
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(body_json(response).await["code"], "unauthenticated");
    }

    #[tokio::test]
    async fn pipette_beaker_h2o_use_tool_without_csrf_is_forbidden() {
        let app = test_app().await;
        let (_csrf_token, csrf_cookie, session_cookie) =
            register_user(&app, "h2o-pipette-csrf@chemlab.local").await;
        let response = post_action(
            &app,
            &format!("{session_cookie}; {csrf_cookie}"),
            None,
            serde_json::json!({
                "type": "use_tool",
                "tool_item_id": "pipette-1",
                "target_item_id": "beaker-h2o"
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(body_json(response).await["code"], "csrf");
    }

    #[tokio::test]
    async fn beaker_h2o_use_tool_and_put_away_with_csrf_and_session_pick_up_and_restore() {
        let app = test_app().await;
        let (csrf_token, csrf_cookie, session_cookie) =
            register_user(&app, "h2o-tongs-ok@chemlab.local").await;
        let cookies = format!("{session_cookie}; {csrf_cookie}");

        let pickup = post_action(
            &app,
            &cookies,
            Some(&csrf_token),
            serde_json::json!({
                "type": "use_tool",
                "tool_item_id": "tongs-1",
                "target_item_id": "beaker-h2o"
            }),
        )
        .await;
        assert_eq!(pickup.status(), StatusCode::OK);
        let picked = body_json(pickup).await["scene"].clone();
        assert_eq!(scene_item(&picked, "beaker-h2o")["location"], "held");
        assert_eq!(
            scene_item(&picked, "tongs-1")["properties"]["source_item_id"],
            "beaker-h2o"
        );

        let persisted = body_json(get_scene(&app, Some(&cookies)).await).await;
        assert_eq!(
            without_clock(&persisted)["items"],
            without_clock(&picked)["items"]
        );

        let put_away = post_action(
            &app,
            &cookies,
            Some(&csrf_token),
            serde_json::json!({
                "type": "put_away",
                "tool_item_id": "tongs-1"
            }),
        )
        .await;
        assert_eq!(put_away.status(), StatusCode::OK);
        let restored = body_json(put_away).await["scene"].clone();
        assert_eq!(scene_item(&restored, "beaker-h2o")["location"], "bench");
        assert_eq!(scene_item(&restored, "tongs-1")["location"], "bench");
        assert!(
            scene_item(&restored, "tongs-1")["properties"]
                .get("source_item_id")
                .is_none()
                || scene_item(&restored, "tongs-1")["properties"]["source_item_id"].is_null()
        );
    }

    #[tokio::test]
    async fn pipette_beaker_h2o_use_tool_with_csrf_and_session_draws_one_ml() {
        let app = test_app().await;
        let (csrf_token, csrf_cookie, session_cookie) =
            register_user(&app, "h2o-pipette-ok@chemlab.local").await;
        let cookies = format!("{session_cookie}; {csrf_cookie}");
        let fill = post_action(
            &app,
            &cookies,
            Some(&csrf_token),
            serde_json::json!({
                "type": "use_tool",
                "tool_item_id": "pipette-1",
                "target_item_id": "beaker-h2o"
            }),
        )
        .await;
        assert_eq!(fill.status(), StatusCode::OK);
        let scene = body_json(fill).await["scene"].clone();
        assert_eq!(
            scene_item(&scene, "beaker-h2o")["properties"]["composition"][0]["amount_ml"],
            99.0
        );
        let holding = scene_item(&scene, "pipette-1")["properties"]["holding"]
            .as_array()
            .unwrap();
        assert_eq!(holding[0]["substance_id"], "water");
        assert_eq!(holding[0]["phase"], "liquid");
        assert_eq!(holding[0]["amount_ml"], 1.0);
        assert_eq!(
            scene_item(&scene, "pipette-1")["properties"]["source_item_id"],
            "beaker-h2o"
        );
    }

    #[tokio::test]
    async fn authenticated_scene_get_includes_empty_filtrate_beaker_and_filter_paper() {
        let app = test_app().await;
        let (_csrf_token, _csrf_cookie, session_cookie) =
            register_user(&app, "filter-scene@chemlab.local").await;
        let response = get_scene(&app, Some(&session_cookie)).await;
        assert_eq!(response.status(), StatusCode::OK);
        let scene = body_json(response).await;
        let filtrate = scene_item(&scene, "beaker-filtrate");
        assert_eq!(filtrate["kind"], "beaker");
        assert_eq!(filtrate["label"], "Filtrate");
        assert_eq!(filtrate["location"], "bench");
        assert_eq!(filtrate["properties"]["volume_ml"], 250.0);
        assert_eq!(filtrate["properties"]["fill_ml"], 0.0);
        assert!(
            filtrate["properties"].get("composition").is_none()
                || filtrate["properties"]["composition"]
                    .as_array()
                    .is_some_and(|c| c.is_empty())
        );
        let paper = scene_item(&scene, "filter-paper-1");
        assert_eq!(paper["kind"], "filter_paper");
        assert_eq!(paper["label"], "Filter paper");
        assert_eq!(paper["location"], "bench");
    }

    async fn persist_scene_without_filtration(state: &AppState, app: &Router, cookies: &str) {
        let mut scene = body_json(get_scene(app, Some(cookies)).await).await;
        scene["items"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|item| item["id"] == "beaker-water")
            .expect("beaker-water")["label"] = serde_json::json!("Water");
        set_main_beaker_water(&mut scene, 200.0);
        scene["items"]
            .as_array_mut()
            .unwrap()
            .retain(|item| item["id"] != "beaker-filtrate" && item["id"] != "filter-paper-1");
        assert!(scene["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| { item["id"] != "beaker-filtrate" && item["id"] != "filter-paper-1" }));
        save_scene_blob(state, &scene).await;
    }

    #[tokio::test]
    async fn get_scene_backfills_filtration_items_on_persisted_lab() {
        let (app, state) = test_app_state().await;
        let (_csrf_token, csrf_cookie, session_cookie) =
            register_user(&app, "filter-migrate-get@chemlab.local").await;
        let cookies = format!("{session_cookie}; {csrf_cookie}");
        persist_scene_without_filtration(&state, &app, &cookies).await;

        let loaded = body_json(get_scene(&app, Some(&cookies)).await).await;
        assert_eq!(scene_item(&loaded, "beaker-filtrate")["kind"], "beaker");
        assert_eq!(scene_item(&loaded, "beaker-filtrate")["location"], "bench");
        assert_eq!(
            scene_item(&loaded, "beaker-filtrate")["properties"]["fill_ml"],
            0.0
        );
        assert_eq!(
            scene_item(&loaded, "filter-paper-1")["kind"],
            "filter_paper"
        );
        assert_eq!(
            scene_item(&loaded, "beaker-water")["properties"]["composition"][0]["amount_ml"],
            200.0
        );
    }

    #[tokio::test]
    async fn filtration_use_tool_without_csrf_is_forbidden() {
        let app = test_app().await;
        let (_csrf_token, csrf_cookie, session_cookie) =
            register_user(&app, "filter-csrf-use@chemlab.local").await;
        let response = post_action(
            &app,
            &format!("{session_cookie}; {csrf_cookie}"),
            None,
            serde_json::json!({
                "type": "use_tool",
                "tool_item_id": "tongs-1",
                "target_item_id": "filter-paper-1"
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(body_json(response).await["code"], "csrf");
    }

    #[tokio::test]
    async fn filtration_use_tool_without_session_is_unauthorized() {
        let app = test_app().await;
        let (csrf_token, csrf_cookie) = issue_csrf(&app).await;
        let response = post_action(
            &app,
            &csrf_cookie,
            Some(&csrf_token),
            serde_json::json!({
                "type": "use_tool",
                "tool_item_id": "tongs-1",
                "target_item_id": "beaker-filtrate"
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(body_json(response).await["code"], "unauthenticated");
    }

    #[tokio::test]
    async fn filtration_put_away_without_csrf_is_forbidden() {
        let app = test_app().await;
        let (_csrf_token, csrf_cookie, session_cookie) =
            register_user(&app, "filter-csrf-putaway@chemlab.local").await;
        let response = post_action(
            &app,
            &format!("{session_cookie}; {csrf_cookie}"),
            None,
            serde_json::json!({
                "type": "put_away",
                "tool_item_id": "tongs-1"
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(body_json(response).await["code"], "csrf");
    }

    #[tokio::test]
    async fn filtration_put_away_without_session_is_unauthorized() {
        let app = test_app().await;
        let (csrf_token, csrf_cookie) = issue_csrf(&app).await;
        let response = post_action(
            &app,
            &csrf_cookie,
            Some(&csrf_token),
            serde_json::json!({
                "type": "put_away",
                "tool_item_id": "tongs-1"
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(body_json(response).await["code"], "unauthenticated");
    }

    #[tokio::test]
    async fn filtration_use_tool_and_put_away_with_csrf_and_session_pick_up_filtrate() {
        let app = test_app().await;
        let (csrf_token, csrf_cookie, session_cookie) =
            register_user(&app, "filter-tongs-ok@chemlab.local").await;
        let cookies = format!("{session_cookie}; {csrf_cookie}");

        let pickup = post_action(
            &app,
            &cookies,
            Some(&csrf_token),
            serde_json::json!({
                "type": "use_tool",
                "tool_item_id": "tongs-1",
                "target_item_id": "filter-paper-1"
            }),
        )
        .await;
        assert_eq!(pickup.status(), StatusCode::OK);
        let picked = body_json(pickup).await["scene"].clone();
        assert_eq!(scene_item(&picked, "beaker-filtrate")["location"], "held");
        assert_eq!(scene_item(&picked, "filter-paper-1")["location"], "bench");
        assert_eq!(
            scene_item(&picked, "tongs-1")["properties"]["source_item_id"],
            "beaker-filtrate"
        );

        let persisted = body_json(get_scene(&app, Some(&cookies)).await).await;
        assert_eq!(
            without_clock(&persisted)["items"],
            without_clock(&picked)["items"]
        );

        let put_away = post_action(
            &app,
            &cookies,
            Some(&csrf_token),
            serde_json::json!({
                "type": "put_away",
                "tool_item_id": "tongs-1"
            }),
        )
        .await;
        assert_eq!(put_away.status(), StatusCode::OK);
        let restored = body_json(put_away).await["scene"].clone();
        assert_eq!(
            scene_item(&restored, "beaker-filtrate")["location"],
            "bench"
        );
        assert_eq!(scene_item(&restored, "tongs-1")["location"], "bench");
        assert!(
            scene_item(&restored, "tongs-1")["properties"]
                .get("source_item_id")
                .is_none()
                || scene_item(&restored, "tongs-1")["properties"]["source_item_id"].is_null()
        );
    }

    #[tokio::test]
    async fn tongs_use_tool_on_persisted_lab_without_filtration_picks_up_filtrate() {
        let (app, state) = test_app_state().await;
        let (csrf_token, csrf_cookie, session_cookie) =
            register_user(&app, "filter-migrate-use@chemlab.local").await;
        let cookies = format!("{session_cookie}; {csrf_cookie}");
        persist_scene_without_filtration(&state, &app, &cookies).await;

        let pickup = post_action(
            &app,
            &cookies,
            Some(&csrf_token),
            serde_json::json!({
                "type": "use_tool",
                "tool_item_id": "tongs-1",
                "target_item_id": "beaker-filtrate"
            }),
        )
        .await;
        assert_eq!(pickup.status(), StatusCode::OK);
        let picked = body_json(pickup).await["scene"].clone();
        assert_eq!(scene_item(&picked, "beaker-filtrate")["location"], "held");
        assert_eq!(
            scene_item(&picked, "tongs-1")["properties"]["source_item_id"],
            "beaker-filtrate"
        );
    }

    #[tokio::test]
    async fn pipette_filtrate_use_tool_without_csrf_is_forbidden() {
        let app = test_app().await;
        let (_csrf_token, csrf_cookie, session_cookie) =
            register_user(&app, "filter-pipette-csrf@chemlab.local").await;
        let response = post_action(
            &app,
            &format!("{session_cookie}; {csrf_cookie}"),
            None,
            serde_json::json!({
                "type": "use_tool",
                "tool_item_id": "pipette-1",
                "target_item_id": "beaker-filtrate"
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(body_json(response).await["code"], "csrf");
    }

    #[tokio::test]
    async fn spoon_paper_use_tool_without_csrf_is_forbidden() {
        let app = test_app().await;
        let (_csrf_token, csrf_cookie, session_cookie) =
            register_user(&app, "filter-spoon-csrf@chemlab.local").await;
        let response = post_action(
            &app,
            &format!("{session_cookie}; {csrf_cookie}"),
            None,
            serde_json::json!({
                "type": "use_tool",
                "tool_item_id": "spoon-1",
                "target_item_id": "filter-paper-1"
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(body_json(response).await["code"], "csrf");
    }

    #[tokio::test]
    async fn filtration_use_tool_with_csrf_and_session_filter_pours_and_pipettes() {
        let (app, state) = test_app_state().await;
        let (csrf_token, csrf_cookie, session_cookie) =
            register_user(&app, "filter-pour-ok@chemlab.local").await;
        let cookies = format!("{session_cookie}; {csrf_cookie}");
        persist_filled_main_beaker(&state, &app, &cookies).await;

        let scoop = post_action(
            &app,
            &cookies,
            Some(&csrf_token),
            serde_json::json!({
                "type": "use_tool",
                "tool_item_id": "spoon-1",
                "target_item_id": "beaker-sand"
            }),
        )
        .await;
        assert_eq!(scoop.status(), StatusCode::OK);
        let pour_sand = post_action(
            &app,
            &cookies,
            Some(&csrf_token),
            serde_json::json!({
                "type": "pour",
                "source_item_id": "spoon-1",
                "target_item_id": "beaker-water"
            }),
        )
        .await;
        assert_eq!(pour_sand.status(), StatusCode::OK);

        let pick_water = post_action(
            &app,
            &cookies,
            Some(&csrf_token),
            serde_json::json!({
                "type": "use_tool",
                "tool_item_id": "tongs-1",
                "target_item_id": "beaker-water"
            }),
        )
        .await;
        assert_eq!(pick_water.status(), StatusCode::OK);
        let filter_pour = post_action(
            &app,
            &cookies,
            Some(&csrf_token),
            serde_json::json!({
                "type": "use_tool",
                "tool_item_id": "tongs-1",
                "target_item_id": "filter-paper-1"
            }),
        )
        .await;
        assert_eq!(filter_pour.status(), StatusCode::OK);
        let filtered = body_json(filter_pour).await["scene"].clone();
        assert_eq!(
            scene_item(&filtered, "beaker-filtrate")["properties"]["fill_ml"],
            200.0
        );
        let paper_sand = scene_item(&filtered, "filter-paper-1")["properties"]["composition"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["substance_id"] == "sand" && entry["phase"] == "solid")
            .unwrap()["amount_g"]
            .as_f64()
            .unwrap();
        assert!((paper_sand - 0.2).abs() < 1e-9);

        let fill = post_action(
            &app,
            &cookies,
            Some(&csrf_token),
            serde_json::json!({
                "type": "use_tool",
                "tool_item_id": "pipette-1",
                "target_item_id": "beaker-filtrate"
            }),
        )
        .await;
        assert_eq!(fill.status(), StatusCode::OK);
        let pipetted = body_json(fill).await["scene"].clone();
        assert_eq!(
            scene_item(&pipetted, "beaker-filtrate")["properties"]["fill_ml"],
            199.0
        );
        assert_eq!(
            scene_item(&pipetted, "pipette-1")["properties"]["source_item_id"],
            "beaker-filtrate"
        );
        assert_eq!(
            scene_item(&pipetted, "pipette-1")["properties"]["holding"][0]["amount_ml"],
            1.0
        );

        let scoop_paper = post_action(
            &app,
            &cookies,
            Some(&csrf_token),
            serde_json::json!({
                "type": "use_tool",
                "tool_item_id": "spoon-1",
                "target_item_id": "filter-paper-1"
            }),
        )
        .await;
        assert_eq!(scoop_paper.status(), StatusCode::OK);
        let scooped = body_json(scoop_paper).await["scene"].clone();
        assert_eq!(
            scene_item(&scooped, "spoon-1")["properties"]["source_item_id"],
            "filter-paper-1"
        );
        let put_away = post_action(
            &app,
            &cookies,
            Some(&csrf_token),
            serde_json::json!({
                "type": "put_away",
                "tool_item_id": "spoon-1"
            }),
        )
        .await;
        assert_eq!(put_away.status(), StatusCode::OK);
        let restored = body_json(put_away).await["scene"].clone();
        let restored_sand = scene_item(&restored, "filter-paper-1")["properties"]["composition"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["substance_id"] == "sand" && entry["phase"] == "solid")
            .unwrap()["amount_g"]
            .as_f64()
            .unwrap();
        assert!((restored_sand - 0.2).abs() < 1e-9);
    }
}
