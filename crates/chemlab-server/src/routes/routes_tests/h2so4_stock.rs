use super::helpers::*;
use axum::http::StatusCode;

#[tokio::test]
async fn authenticated_scene_get_includes_free_mode_h2so4_stock() {
    let app = test_app().await;
    let (_csrf_token, _csrf_cookie, session_cookie) =
        register_user(&app, "h2so4-scene@chemlab.local").await;
    let response = get_scene(&app, Some(&session_cookie)).await;
    assert_eq!(response.status(), StatusCode::OK);
    let scene = body_json(response).await;
    let acid = scene_item(&scene, "beaker-h2so4");
    assert_eq!(acid["kind"], "beaker");
    assert_eq!(acid["label"], "Sulfuric acid");
    assert_eq!(acid["location"], "bench");
    assert_eq!(acid["properties"]["volume_ml"], 10.0);
    assert_eq!(acid["properties"]["fill_ml"], 10.0);
    let composition = acid["properties"]["composition"].as_array().unwrap();
    assert!(composition.iter().any(|c| {
        c["substance_id"] == "h2so4" && c["phase"] == "liquid" && c["amount_ml"] == 10.0
    }));
}

#[tokio::test]
async fn tongs_use_tool_picks_up_beaker_h2so4() {
    let app = test_app().await;
    let (csrf_token, csrf_cookie, session_cookie) =
        register_user(&app, "h2so4-tongs-pick@chemlab.local").await;
    let cookies = format!("{session_cookie}; {csrf_cookie}");

    let response = post_action(
        &app,
        &cookies,
        Some(&csrf_token),
        serde_json::json!({
            "type": "use_tool",
            "tool_item_id": "tongs-1",
            "target_item_id": "beaker-h2so4"
        }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let scene = body_json(response).await["scene"].clone();
    assert_eq!(scene_item(&scene, "beaker-h2so4")["location"], "held");
    assert_eq!(
        scene_item(&scene, "tongs-1")["properties"]["source_item_id"],
        "beaker-h2so4"
    );
}
