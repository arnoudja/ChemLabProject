use super::helpers::*;
use axum::http::StatusCode;

#[tokio::test]
async fn authenticated_scene_get_includes_free_mode_hcl_stock() {
    let app = test_app().await;
    let (_csrf_token, _csrf_cookie, session_cookie) =
        register_user(&app, "hcl-scene@chemlab.local").await;
    let response = get_scene(&app, Some(&session_cookie)).await;
    assert_eq!(response.status(), StatusCode::OK);
    let scene = body_json(response).await;
    let hcl = scene_item(&scene, "beaker-hcl");
    assert_eq!(hcl["kind"], "beaker");
    assert_eq!(hcl["label"], "Hydrochloric acid (30%)");
    assert_eq!(hcl["location"], "bench");
    assert_eq!(hcl["properties"]["volume_ml"], 10.0);
    assert_eq!(hcl["properties"]["fill_ml"], 10.0);
    let composition = hcl["properties"]["composition"].as_array().unwrap();
    assert!(composition.iter().any(|c| {
        c["substance_id"] == "water" && c["phase"] == "liquid" && c["amount_ml"] == 8.043
    }));
    assert!(composition.iter().any(|c| {
        c["substance_id"] == "h+" && c["phase"] == "aqueous" && c["amount_mol"].is_number()
    }));
    assert!(composition.iter().any(|c| {
        c["substance_id"] == "cl-" && c["phase"] == "aqueous" && c["amount_mol"].is_number()
    }));
}

#[tokio::test]
async fn tongs_use_tool_picks_up_beaker_hcl() {
    let app = test_app().await;
    let (csrf_token, csrf_cookie, session_cookie) =
        register_user(&app, "hcl-tongs-pick@chemlab.local").await;
    let cookies = format!("{session_cookie}; {csrf_cookie}");

    let response = post_action(
        &app,
        &cookies,
        Some(&csrf_token),
        serde_json::json!({
            "type": "use_tool",
            "tool_item_id": "tongs-1",
            "target_item_id": "beaker-hcl"
        }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let scene = body_json(response).await["scene"].clone();
    assert_eq!(scene_item(&scene, "beaker-hcl")["location"], "held");
    assert_eq!(
        scene_item(&scene, "tongs-1")["properties"]["source_item_id"],
        "beaker-hcl"
    );
}
