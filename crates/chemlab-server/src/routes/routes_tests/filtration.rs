use super::helpers::*;
use axum::http::StatusCode;

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
            "target_item_id": "beaker-filtrate"
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
