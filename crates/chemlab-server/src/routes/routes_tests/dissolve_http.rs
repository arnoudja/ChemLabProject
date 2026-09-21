use super::helpers::*;
use axum::http::StatusCode;

#[tokio::test]
async fn dissolve_without_csrf_is_forbidden() {
    let app = test_app().await;
    let (_csrf_token, csrf_cookie, session_cookie) = register_user(&app, "ada@chemlab.local").await;
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
    let (csrf_token, csrf_cookie, session_cookie) = register_user(&app, "ada@chemlab.local").await;
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
    let (csrf_token, csrf_cookie, session_cookie) = register_user(&app, "lab@chemlab.local").await;
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
