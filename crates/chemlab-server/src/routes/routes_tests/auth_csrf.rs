use super::helpers::*;
use crate::config::Config;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

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
async fn register_disabled_returns_forbidden_and_does_not_insert_user() {
    let (app, state) = test_app_state_with(Config {
        signup_enabled: false,
        ..test_config()
    })
    .await;
    let (csrf_token, csrf_cookie) = issue_csrf(&app).await;

    let response = post_register(&app, &csrf_token, &csrf_cookie, "blocked@chemlab.local").await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let json = body_json(response).await;
    assert_eq!(json["code"], "signup_disabled");

    let found = chemlab_db::find_user_by_email(state.pool(), "blocked@chemlab.local").await;
    assert!(
        matches!(found, Err(chemlab_db::DbError::UserNotFound)),
        "disabled register must not insert a user: {found:?}"
    );
}

#[tokio::test]
async fn register_disabled_still_requires_csrf() {
    let (app, _state) = test_app_state_with(Config {
        signup_enabled: false,
        ..test_config()
    })
    .await;

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
    let (csrf_token, csrf_cookie, _session) = register_user(&app, "wrong-pw@chemlab.local").await;

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
        r#"{"email":"short@chemlab.local","password":"short","display_name":"Ada"}"#.to_string(),
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
