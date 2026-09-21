use super::helpers::*;
use axum::http::StatusCode;

#[tokio::test]
async fn scene_without_session_is_unauthorized() {
    let app = test_app().await;
    let response = get_scene(&app, None).await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(body_json(response).await["code"], "unauthenticated");
}

#[tokio::test]
async fn authenticated_scene_get_creates_initial_water_beaker() {
    let app = test_app().await;
    let (_csrf_token, _csrf_cookie, session_cookie) =
        register_user(&app, "scene@chemlab.local").await;
    let response = get_scene(&app, Some(&session_cookie)).await;

    assert_eq!(response.status(), StatusCode::OK);
    let scene = body_json(response).await;
    assert!(!scene["lab_id"].as_str().unwrap_or("").is_empty());
    assert_eq!(scene["version"], 0);
    let water = scene["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] == "beaker-water")
        .unwrap();
    assert_eq!(water["properties"]["volume_ml"], 250.0);
    assert_eq!(water["label"], "Beaker");
    assert_eq!(water["properties"]["fill_ml"], 0.0);
    assert_eq!(water["properties"]["transparent"], true);
    assert_eq!(water["properties"]["colourless"], true);
    assert_eq!(water["properties"]["temperature_c"], 20.0);
    assert!(
        water["properties"].get("composition").is_none()
            || water["properties"]["composition"]
                .as_array()
                .is_some_and(|composition| composition.is_empty())
    );
}

#[tokio::test]
async fn action_without_csrf_is_forbidden() {
    let app = test_app().await;
    let (_csrf_token, csrf_cookie, session_cookie) =
        register_user(&app, "no-csrf-action@chemlab.local").await;
    let response = post_action(
        &app,
        &format!("{session_cookie}; {csrf_cookie}"),
        None,
        serde_json::json!({
            "type": "use_tool",
            "tool_item_id": "spoon-1",
            "target_item_id": "beaker-nacl"
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(body_json(response).await["code"], "csrf");
}

#[tokio::test]
async fn use_tool_action_persists_spoon_holding_across_get() {
    let app = test_app().await;
    let (csrf_token, csrf_cookie, session_cookie) =
        register_user(&app, "scoop@chemlab.local").await;
    let cookies = format!("{session_cookie}; {csrf_cookie}");
    let response = post_action(
        &app,
        &cookies,
        Some(&csrf_token),
        serde_json::json!({
            "type": "use_tool",
            "tool_item_id": "spoon-1",
            "target_item_id": "beaker-nacl"
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body_json(response).await["scene"]["version"], 1);

    let scene = body_json(get_scene(&app, Some(&cookies)).await).await;
    let spoon = scene["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] == "spoon-1")
        .unwrap();
    assert_eq!(scene["version"], 1);
    assert_eq!(spoon["properties"]["holding"][0]["substance_id"], "nacl");
    assert_eq!(spoon["properties"]["holding"][0]["amount_g"], 0.2);
    let nacl = scene["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] == "beaker-nacl")
        .unwrap();
    assert_eq!(nacl["properties"]["composition"][0]["amount_scoop"], 9);
    assert_eq!(nacl["properties"]["composition"][0]["amount_g"], 1.8);
}

#[tokio::test]
async fn use_tool_put_back_restores_stock_and_clears_holding() {
    let app = test_app().await;
    let (csrf_token, csrf_cookie, session_cookie) =
        register_user(&app, "putback@chemlab.local").await;
    let cookies = format!("{session_cookie}; {csrf_cookie}");
    let scoop = serde_json::json!({
        "type": "use_tool",
        "tool_item_id": "spoon-1",
        "target_item_id": "beaker-nacl"
    });
    assert_eq!(
        post_action(&app, &cookies, Some(&csrf_token), scoop.clone())
            .await
            .status(),
        StatusCode::OK
    );

    let response = post_action(&app, &cookies, Some(&csrf_token), scoop).await;
    assert_eq!(response.status(), StatusCode::OK);
    let scene = body_json(response).await["scene"].clone();
    assert_eq!(scene["last_events"][0]["kind"], "returned");
    let spoon = scene["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] == "spoon-1")
        .unwrap();
    assert!(
        spoon["properties"].get("holding").is_none()
            || spoon["properties"]["holding"]
                .as_array()
                .is_some_and(|holding| holding.is_empty())
    );
    let nacl = scene["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] == "beaker-nacl")
        .unwrap();
    assert_eq!(nacl["properties"]["composition"][0]["amount_scoop"], 10);
    assert_eq!(nacl["properties"]["composition"][0]["amount_g"], 2.0);

    let persisted = body_json(get_scene(&app, Some(&cookies)).await).await;
    assert_eq!(without_clock(&persisted), without_clock(&scene));
}

#[tokio::test]
async fn put_away_returns_scoop_to_matching_stock_for_each_solid() {
    let app = test_app().await;
    for (email, beaker_id, substance) in [
        ("putaway-nacl@chemlab.local", "beaker-nacl", "nacl"),
        ("putaway-sand@chemlab.local", "beaker-sand", "sand"),
        ("putaway-cacl2@chemlab.local", "beaker-cacl2", "cacl2"),
    ] {
        let (csrf_token, csrf_cookie, session_cookie) = register_user(&app, email).await;
        let cookies = format!("{session_cookie}; {csrf_cookie}");
        assert_eq!(
            post_action(
                &app,
                &cookies,
                Some(&csrf_token),
                serde_json::json!({
                    "type": "use_tool",
                    "tool_item_id": "spoon-1",
                    "target_item_id": beaker_id
                }),
            )
            .await
            .status(),
            StatusCode::OK
        );

        let response = post_action(
            &app,
            &cookies,
            Some(&csrf_token),
            serde_json::json!({
                "type": "put_away",
                "tool_item_id": "spoon-1"
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let scene = body_json(response).await["scene"].clone();
        assert_eq!(scene["last_events"][0]["kind"], "returned");
        let spoon = scene["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["id"] == "spoon-1")
            .unwrap();
        assert_eq!(spoon["location"], "bench");
        assert!(
            spoon["properties"].get("holding").is_none()
                || spoon["properties"]["holding"]
                    .as_array()
                    .is_some_and(|holding| holding.is_empty())
        );
        let stock = scene["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["id"] == beaker_id)
            .unwrap();
        let solid = stock["properties"]["composition"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["substance_id"] == substance && c["phase"] == "solid")
            .unwrap();
        assert_eq!(solid["amount_scoop"], 10);
        assert_eq!(solid["amount_g"], 2.0);

        let persisted = body_json(get_scene(&app, Some(&cookies)).await).await;
        assert_eq!(without_clock(&persisted), without_clock(&scene));
    }
}

#[tokio::test]
async fn put_away_empty_spoon_is_noop() {
    let app = test_app().await;
    let (csrf_token, csrf_cookie, session_cookie) =
        register_user(&app, "putaway-empty@chemlab.local").await;
    let cookies = format!("{session_cookie}; {csrf_cookie}");
    let before = body_json(get_scene(&app, Some(&cookies)).await).await;
    let response = post_action(
        &app,
        &cookies,
        Some(&csrf_token),
        serde_json::json!({
            "type": "put_away",
            "tool_item_id": "spoon-1"
        }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let scene = body_json(response).await["scene"].clone();
    assert!(
        scene.get("last_events").is_none()
            || scene["last_events"]
                .as_array()
                .is_some_and(|events| events.is_empty())
    );
    for beaker_id in ["beaker-nacl", "beaker-sand", "beaker-cacl2"] {
        let before_stock = before["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["id"] == beaker_id)
            .unwrap();
        let after_stock = scene["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["id"] == beaker_id)
            .unwrap();
        assert_eq!(
            after_stock["properties"]["composition"],
            before_stock["properties"]["composition"]
        );
    }
    let spoon = scene["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] == "spoon-1")
        .unwrap();
    assert_eq!(spoon["location"], "bench");
    assert!(
        spoon["properties"].get("holding").is_none()
            || spoon["properties"]["holding"]
                .as_array()
                .is_some_and(|holding| holding.is_empty())
    );
}

#[tokio::test]
async fn use_tool_rejects_putting_nacl_into_sand_stock() {
    let app = test_app().await;
    let (csrf_token, csrf_cookie, session_cookie) =
        register_user(&app, "wrong-stock@chemlab.local").await;
    let cookies = format!("{session_cookie}; {csrf_cookie}");
    assert_eq!(
        post_action(
            &app,
            &cookies,
            Some(&csrf_token),
            serde_json::json!({
                "type": "use_tool",
                "tool_item_id": "spoon-1",
                "target_item_id": "beaker-nacl"
            }),
        )
        .await
        .status(),
        StatusCode::OK
    );
    let response = post_action(
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
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(body_json(response).await["code"], "invalid_action");
}
