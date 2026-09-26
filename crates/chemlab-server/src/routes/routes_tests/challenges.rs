use super::helpers::*;
use crate::state::AppState;
use axum::http::StatusCode;

const SEPARATE: &str = "separate-nacl-sio2";

fn select_mode(mode: &str) -> serde_json::Value {
    serde_json::json!({ "type": "select_mode", "mode": mode })
}

fn scene_ids(scene: &serde_json::Value) -> Vec<String> {
    scene["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["id"].as_str().unwrap().to_string())
        .collect()
}

fn solid_g(scene: &serde_json::Value, item_id: &str, substance_id: &str) -> f64 {
    scene_item(scene, item_id)["properties"]["composition"]
        .as_array()
        .map(|entries| {
            entries
                .iter()
                .filter(|entry| entry["phase"] == "solid" && entry["substance_id"] == substance_id)
                .filter_map(|entry| entry["amount_g"].as_f64())
                .sum()
        })
        .unwrap_or(0.0)
}

/// Replace a stock beaker's solid line so the win condition holds.
async fn persist_stock_solid(
    state: &AppState,
    app: &axum::Router,
    cookies: &str,
    item_id: &str,
    substance_id: &str,
    amount_g: f64,
) {
    let mut scene = body_json(get_scene(app, Some(cookies)).await).await;
    let stock = scene["items"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|item| item["id"] == item_id)
        .expect("stock beaker");
    stock["properties"]["composition"] = serde_json::json!([
        {
            "substance_id": substance_id,
            "phase": "solid",
            "amount_g": amount_g
        }
    ]);
    save_scene_blob(state, &scene).await;
}

#[tokio::test]
async fn new_lab_starts_in_free_mode() {
    let app = test_app().await;
    let (_csrf_token, _csrf_cookie, session_cookie) =
        register_user(&app, "mode-default@chemlab.local").await;

    let scene = body_json(get_scene(&app, Some(&session_cookie)).await).await;

    assert_eq!(scene["mode"], "free");
    assert_eq!(scene["challenge_completed"], false);
    assert!(scene_ids(&scene).contains(&"beaker-cacl2".to_string()));
}

#[tokio::test]
async fn select_mode_switches_to_the_challenge_setup_and_persists_it() {
    let app = test_app().await;
    let (csrf_token, csrf_cookie, session_cookie) =
        register_user(&app, "mode-select@chemlab.local").await;
    let cookies = format!("{session_cookie}; {csrf_cookie}");

    let response = post_action(&app, &cookies, Some(&csrf_token), select_mode(SEPARATE)).await;
    assert_eq!(response.status(), StatusCode::OK);
    let scene = body_json(response).await["scene"].clone();

    assert_eq!(scene["mode"], SEPARATE);
    assert_eq!(scene["challenge_completed"], false);
    assert_eq!(scene["last_events"][0]["kind"], "mode_selected");
    let ids = scene_ids(&scene);
    assert!(!ids.contains(&"beaker-cacl2".to_string()));
    assert!(!ids.contains(&"beaker-hcl".to_string()));
    assert!(!ids.contains(&"beaker-naoh".to_string()));
    assert!(ids.contains(&"beaker-h2o".to_string()));
    assert_eq!(
        scene_item(&scene, "beaker-h2o")["properties"]["fill_ml"],
        10.0
    );
    assert_eq!(
        scene_item(&scene, "beaker-h2o")["properties"]["composition"][0]["amount_ml"],
        10.0
    );
    assert_eq!(solid_g(&scene, "beaker-water", "nacl"), 2.0);
    assert_eq!(solid_g(&scene, "beaker-water", "sand"), 2.0);
    assert_eq!(solid_g(&scene, "beaker-nacl", "nacl"), 0.0);
    assert_eq!(solid_g(&scene, "beaker-sand", "sand"), 0.0);

    // The mode rides along in the persisted scene blob.
    let reloaded = body_json(get_scene(&app, Some(&cookies)).await).await;
    assert_eq!(reloaded["mode"], SEPARATE);
    assert!(!scene_ids(&reloaded).contains(&"beaker-cacl2".to_string()));
}

#[tokio::test]
async fn reset_inside_a_challenge_keeps_the_challenge() {
    let app = test_app().await;
    let (csrf_token, csrf_cookie, session_cookie) =
        register_user(&app, "mode-reset@chemlab.local").await;
    let cookies = format!("{session_cookie}; {csrf_cookie}");
    assert_eq!(
        post_action(&app, &cookies, Some(&csrf_token), select_mode(SEPARATE))
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
                "type": "use_tool",
                "tool_item_id": "spoon-1",
                "target_item_id": "beaker-water"
            }),
        )
        .await
        .status(),
        StatusCode::OK
    );

    let reset = post_action(
        &app,
        &cookies,
        Some(&csrf_token),
        serde_json::json!({ "type": "reset" }),
    )
    .await;
    assert_eq!(reset.status(), StatusCode::OK);
    let scene = body_json(reset).await["scene"].clone();

    assert_eq!(scene["mode"], SEPARATE);
    assert!(!scene_ids(&scene).contains(&"beaker-cacl2".to_string()));
    assert_eq!(solid_g(&scene, "beaker-water", "nacl"), 2.0);
    assert!(scene_item(&scene, "spoon-1")["properties"]["holding"]
        .as_array()
        .is_none_or(|holding| holding.is_empty()));
}

#[tokio::test]
async fn select_free_mode_restores_the_default_bench() {
    let app = test_app().await;
    let (csrf_token, csrf_cookie, session_cookie) =
        register_user(&app, "mode-free@chemlab.local").await;
    let cookies = format!("{session_cookie}; {csrf_cookie}");
    assert_eq!(
        post_action(&app, &cookies, Some(&csrf_token), select_mode(SEPARATE))
            .await
            .status(),
        StatusCode::OK
    );

    let response = post_action(&app, &cookies, Some(&csrf_token), select_mode("free")).await;
    assert_eq!(response.status(), StatusCode::OK);
    let scene = body_json(response).await["scene"].clone();

    assert_eq!(scene["mode"], "free");
    assert_eq!(scene["challenge_completed"], false);
    assert!(scene_ids(&scene).contains(&"beaker-cacl2".to_string()));
    assert_eq!(solid_g(&scene, "beaker-nacl", "nacl"), 2.0);
    assert_eq!(solid_g(&scene, "beaker-water", "sand"), 0.0);
}

#[tokio::test]
async fn select_mode_rejects_an_unknown_challenge_id() {
    let app = test_app().await;
    let (csrf_token, csrf_cookie, session_cookie) =
        register_user(&app, "mode-unknown@chemlab.local").await;
    let cookies = format!("{session_cookie}; {csrf_cookie}");

    let response = post_action(
        &app,
        &cookies,
        Some(&csrf_token),
        select_mode("no-such-challenge"),
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = body_json(response).await;
    assert_eq!(body["code"], "unknown_mode");
    assert_eq!(
        body_json(get_scene(&app, Some(&cookies)).await).await["mode"],
        "free"
    );
}

#[tokio::test]
async fn challenge_completed_is_derived_from_the_stocks() {
    let (app, state) = test_app_state().await;
    let (csrf_token, csrf_cookie, session_cookie) =
        register_user(&app, "mode-complete@chemlab.local").await;
    let cookies = format!("{session_cookie}; {csrf_cookie}");
    assert_eq!(
        post_action(&app, &cookies, Some(&csrf_token), select_mode(SEPARATE))
            .await
            .status(),
        StatusCode::OK
    );

    persist_stock_solid(&state, &app, &cookies, "beaker-nacl", "nacl", 2.0).await;
    assert_eq!(
        body_json(get_scene(&app, Some(&cookies)).await).await["challenge_completed"],
        false
    );

    persist_stock_solid(&state, &app, &cookies, "beaker-sand", "sand", 2.0).await;
    let scene = body_json(get_scene(&app, Some(&cookies)).await).await;
    assert_eq!(scene["challenge_completed"], true);

    // Reset drops the recovered solids, so completion goes away with them.
    let reset = post_action(
        &app,
        &cookies,
        Some(&csrf_token),
        serde_json::json!({ "type": "reset" }),
    )
    .await;
    assert_eq!(
        body_json(reset).await["scene"]["challenge_completed"],
        false
    );
}
