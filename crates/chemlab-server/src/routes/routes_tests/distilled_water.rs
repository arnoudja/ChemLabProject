use super::helpers::*;
use axum::http::StatusCode;

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
