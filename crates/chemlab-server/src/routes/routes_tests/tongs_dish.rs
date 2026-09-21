use super::helpers::*;
use axum::http::StatusCode;

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
