use super::helpers::*;
use axum::http::StatusCode;

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
    // 1 ml water + dish body; power / C_eff ≈ 80 / 84.184 ≈ 0.95 °C/s.
    let c_eff = chemlab_core::C_DISH + 1.0 * chemlab_core::WATER_SPECIFIC_HEAT_J_PER_G_K;
    let expected = 20.0 + (chemlab_core::BURNER_POWER_W / c_eff) * 1.5;
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
    let c_eff = chemlab_core::C_DISH + 1.0 * chemlab_core::WATER_SPECIFIC_HEAT_J_PER_G_K;
    let expected = 20.0 + (chemlab_core::BURNER_POWER_W / c_eff) * 1.5;
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
    let c_eff = chemlab_core::C_DISH + 1.0 * chemlab_core::WATER_SPECIFIC_HEAT_J_PER_G_K;
    let expected = 20.0 + (chemlab_core::BURNER_POWER_W / c_eff) * 2.0;
    assert!(
        (temperature - expected).abs() < 1.0,
        "elapsed dt must clamp to 2 s: expected ~{expected}, got {temperature}"
    );
}
