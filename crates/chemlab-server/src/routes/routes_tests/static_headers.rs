use super::helpers::*;
use crate::config::Config;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

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
    assert!(response.headers().get("content-security-policy").is_none());
    let json = body_json(response).await;
    assert_eq!(json["status"], "ok");
    assert_eq!(json["service"], "chemlab-server");
    assert_eq!(json["signup_enabled"], true);
}

#[tokio::test]
async fn health_reports_signup_enabled_false_when_disabled() {
    let (app, _state) = test_app_state_with(Config {
        signup_enabled: false,
        ..test_config()
    })
    .await;
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
    assert_eq!(json["signup_enabled"], false);
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
    assert_html_security_headers(&response);
    let body = body_text(response).await;
    assert!(body.contains("ChemLab frontend not configured"));
    assert!(body.contains("CHEMLAB_STATIC_DIR"));
}

#[tokio::test]
async fn embedded_welcome_includes_html_security_headers() {
    let app = test_app().await;
    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_html_security_headers(&response);
}

#[tokio::test]
async fn cookie_secure_true_sets_secure_on_csrf_session_and_clear() {
    let (app, _state) = test_app_state_with(Config {
        cookie_secure: true,
        ..test_config()
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
        r#"{"email":"secure@chemlab.local","password":"secret123","display_name":"Secure"}"#.into(),
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
        static_dir: Some(dir.path().to_path_buf()),
        ..test_config()
    })
    .await;

    let root = app
        .clone()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(root.status(), StatusCode::OK);
    assert_html_security_headers(&root);
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
    assert!(asset.headers().get("content-security-policy").is_none());
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
    assert_html_security_headers(&spa);
    assert!(body_text(spa).await.contains("ChemLab SPA"));
}

#[tokio::test]
async fn static_dir_missing_index_returns_sensible_404_html() {
    let dir = tempfile::tempdir().unwrap();
    // Directory exists but has no index.html (misconfigured install).
    let (app, _state) = test_app_state_with(Config {
        static_dir: Some(dir.path().to_path_buf()),
        ..test_config()
    })
    .await;

    let root = app
        .clone()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(root.status(), StatusCode::NOT_FOUND);
    assert_html_security_headers(&root);
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
    assert_html_security_headers(&missing);
    assert!(body_text(missing).await.contains("CHEMLAB_STATIC_DIR"));
}
