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

#[tokio::test]
async fn pour_nacl_action_persists_dissolve_scene_and_events() {
    let (app, state) = test_app_state().await;
    let (csrf_token, csrf_cookie, session_cookie) = register_user(&app, "pour@chemlab.local").await;
    let cookies = format!("{session_cookie}; {csrf_cookie}");
    persist_filled_main_beaker(&state, &app, &cookies).await;
    let scoop = serde_json::json!({
        "type": "use_tool",
        "tool_item_id": "spoon-1",
        "target_item_id": "beaker-nacl"
    });
    assert_eq!(
        post_action(&app, &cookies, Some(&csrf_token), scoop)
            .await
            .status(),
        StatusCode::OK
    );

    let response = post_action(
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
    assert_eq!(response.status(), StatusCode::OK);
    let action = body_json(response).await;
    assert_eq!(action["scene"]["version"], 2);
    assert_eq!(action["scene"]["last_events"][0]["kind"], "poured");
    assert_eq!(action["scene"]["last_events"][1]["kind"], "dissolved");
    assert_eq!(
        action["scene"]["last_events"][1]["message"],
        "Sodium chloride (NaCl) dissolves in water at bench temperature."
    );

    let persisted = body_json(get_scene(&app, Some(&cookies)).await).await;
    assert_eq!(without_clock(&persisted), without_clock(&action["scene"]));
    let water = persisted["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] == "beaker-water")
        .unwrap();
    let composition = water["properties"]["composition"].as_array().unwrap();
    assert!(composition
        .iter()
        .any(|entry| entry["substance_id"] == "na+" && entry["phase"] == "aqueous"));
    assert!(composition
        .iter()
        .any(|entry| entry["substance_id"] == "cl-" && entry["phase"] == "aqueous"));
    assert!(!composition
        .iter()
        .any(|entry| entry["substance_id"] == "nacl"));
    let cooled = water["properties"]["temperature_c"].as_f64().unwrap();
    let moles = 0.2 / 58.44;
    let expected = 20.0
        - (moles * chemlab_core::NACL_DELTA_H_SOLUTION_J_PER_MOL)
            / (200.0 * chemlab_core::WATER_SPECIFIC_HEAT_J_PER_G_K);
    assert!(
        (cooled - expected).abs() < 1e-9,
        "expected cooled temperature {expected}, got {cooled}"
    );
    assert!(cooled < 20.0);
}

#[tokio::test]
async fn reset_action_restores_default_scene_and_persists() {
    let (app, state) = test_app_state().await;
    let (csrf_token, csrf_cookie, session_cookie) =
        register_user(&app, "reset@chemlab.local").await;
    let cookies = format!("{session_cookie}; {csrf_cookie}");
    persist_filled_main_beaker(&state, &app, &cookies).await;
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
    assert_eq!(
        post_action(
            &app,
            &cookies,
            Some(&csrf_token),
            serde_json::json!({
                "type": "pour",
                "source_item_id": "spoon-1",
                "target_item_id": "beaker-water"
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
        serde_json::json!({ "type": "reset" }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let action = body_json(response).await;
    assert_eq!(action["scene"]["version"], 3);
    assert_eq!(action["scene"]["last_events"][0]["kind"], "reset");

    let water = action["scene"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] == "beaker-water")
        .unwrap();
    assert_eq!(water["label"], "Beaker");
    assert_eq!(water["properties"]["fill_ml"], 0.0);
    assert!(
        water["properties"].get("composition").is_none()
            || water["properties"]["composition"]
                .as_array()
                .is_some_and(|composition| composition.is_empty())
    );

    let spoon = action["scene"]["items"]
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

    let persisted = body_json(get_scene(&app, Some(&cookies)).await).await;
    assert_eq!(without_clock(&persisted), without_clock(&action["scene"]));
}

#[tokio::test]
async fn scene_errors_return_stable_bad_request_codes() {
    let app = test_app().await;
    let (csrf_token, csrf_cookie, session_cookie) =
        register_user(&app, "scene-errors@chemlab.local").await;
    let cookies = format!("{session_cookie}; {csrf_cookie}");
    let cases = [
        (
            serde_json::json!({
                "type": "use_tool",
                "tool_item_id": "missing",
                "target_item_id": "beaker-nacl"
            }),
            "unknown_item",
        ),
        (
            serde_json::json!({
                "type": "pour",
                "source_item_id": "spoon-1",
                "target_item_id": "beaker-water"
            }),
            "empty_holding",
        ),
        (
            serde_json::json!({
                "type": "use_tool",
                "tool_item_id": "beaker-water",
                "target_item_id": "beaker-nacl"
            }),
            "invalid_action",
        ),
    ];

    for (action, code) in cases {
        let response = post_action(&app, &cookies, Some(&csrf_token), action).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(body_json(response).await["code"], code);
    }
}

#[tokio::test]
async fn pour_into_dry_beaker_returns_invalid_action() {
    let app = test_app().await;
    let (csrf_token, csrf_cookie, session_cookie) =
        register_user(&app, "dry-pour@chemlab.local").await;
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
            "type": "pour",
            "source_item_id": "spoon-1",
            "target_item_id": "beaker-sand"
        }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(body_json(response).await["code"], "invalid_action");
}

#[tokio::test]
async fn pour_sand_at_non_bench_temperature_leaves_undissolved_solid() {
    let (app, state) = test_app_state().await;
    let (csrf_token, csrf_cookie, session_cookie) =
        register_user(&app, "sand-warm@chemlab.local").await;
    let cookies = format!("{session_cookie}; {csrf_cookie}");
    assert_eq!(
        post_action(
            &app,
            &cookies,
            Some(&csrf_token),
            serde_json::json!({
                "type": "use_tool",
                "tool_item_id": "spoon-1",
                "target_item_id": "beaker-sand"
            }),
        )
        .await
        .status(),
        StatusCode::OK
    );

    let scene_response = get_scene(&app, Some(&cookies)).await;
    assert_eq!(scene_response.status(), StatusCode::OK);
    let mut scene = body_json(scene_response).await;
    let items = scene["items"].as_array_mut().expect("items");
    let water = items
        .iter_mut()
        .find(|item| item["id"] == "beaker-water")
        .expect("beaker-water");
    water["properties"]["temperature_c"] = serde_json::json!(21.0);
    water["properties"]["fill_ml"] = serde_json::json!(200.0);
    water["properties"]["composition"] = serde_json::json!([
        {
            "substance_id": "water",
            "phase": "liquid",
            "amount_ml": 200.0
        }
    ]);
    let lab_id = scene["lab_id"].as_str().expect("lab_id").to_string();
    let version = scene["version"].as_u64().expect("version") as i64;
    let blob = serde_json::to_vec(&scene).expect("serialize scene");
    chemlab_db::save_lab_state(state.pool(), &lab_id, &blob, version)
        .await
        .expect("seed warm temperature");

    let response = post_action(
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
    assert_eq!(response.status(), StatusCode::OK);
    let action = body_json(response).await;
    assert_eq!(
        action["scene"]["last_events"][1]["kind"],
        "did_not_dissolve"
    );
    assert_eq!(
        action["scene"]["last_events"][1]["message"],
        "Sand (silica) does not dissolve in water at bench temperature."
    );
    let water = action["scene"]["items"]
        .as_array()
        .expect("items")
        .iter()
        .find(|item| item["id"] == "beaker-water")
        .expect("beaker-water");
    assert_eq!(water["properties"]["temperature_c"], 21.0);
    let sand_solid = water["properties"]["composition"]
        .as_array()
        .expect("composition")
        .iter()
        .find(|c| c["substance_id"] == "sand" && c["phase"] == "solid")
        .expect("solid sand leftover");
    assert_eq!(sand_solid["amount_scoop"], 1);
    assert!((sand_solid["amount_g"].as_f64().unwrap() - 0.2).abs() < 1e-12);
}

#[tokio::test]
async fn pour_sand_after_cacl2_heating_succeeds() {
    let (app, state) = test_app_state().await;
    let (csrf_token, csrf_cookie, session_cookie) =
        register_user(&app, "sand-after-heat@chemlab.local").await;
    let cookies = format!("{session_cookie}; {csrf_cookie}");
    persist_filled_main_beaker(&state, &app, &cookies).await;

    // One scoop only raises T by ~0.175 °C (still rounds to 20). Pour until the
    // dissolve lookup sees a non-bench integer °C — the real warm-water bug path.
    let mut after_heat = 20.0_f64;
    for scoop_n in 1..=4 {
        assert_eq!(
            post_action(
                &app,
                &cookies,
                Some(&csrf_token),
                serde_json::json!({
                    "type": "use_tool",
                    "tool_item_id": "spoon-1",
                    "target_item_id": "beaker-cacl2"
                }),
            )
            .await
            .status(),
            StatusCode::OK,
            "scoop cacl2 {scoop_n}"
        );
        let heat_response = post_action(
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
        assert_eq!(
            heat_response.status(),
            StatusCode::OK,
            "pour cacl2 {scoop_n}"
        );
        after_heat = body_json(heat_response).await["scene"]["items"]
            .as_array()
            .expect("items")
            .iter()
            .find(|item| item["id"] == "beaker-water")
            .expect("beaker-water")["properties"]["temperature_c"]
            .as_f64()
            .expect("temperature_c");
    }
    assert!(
        after_heat.round() as i32 != 20,
        "CaCl2 heating must leave a non-bench lookup T, got {after_heat}"
    );

    assert_eq!(
        post_action(
            &app,
            &cookies,
            Some(&csrf_token),
            serde_json::json!({
                "type": "use_tool",
                "tool_item_id": "spoon-1",
                "target_item_id": "beaker-sand"
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
            "type": "pour",
            "source_item_id": "spoon-1",
            "target_item_id": "beaker-water"
        }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let action = body_json(response).await;
    assert_eq!(
        action["scene"]["last_events"][1]["kind"],
        "did_not_dissolve"
    );
    assert_eq!(
        action["scene"]["last_events"][1]["message"],
        "Sand (silica) does not dissolve in water at bench temperature."
    );
    let water = action["scene"]["items"]
        .as_array()
        .expect("items")
        .iter()
        .find(|item| item["id"] == "beaker-water")
        .expect("beaker-water");
    assert_eq!(water["properties"]["temperature_c"], after_heat);
    let sand_solid = water["properties"]["composition"]
        .as_array()
        .expect("composition")
        .iter()
        .find(|c| c["substance_id"] == "sand" && c["phase"] == "solid")
        .expect("solid sand leftover");
    assert_eq!(sand_solid["amount_scoop"], 1);
    assert!((sand_solid["amount_g"].as_f64().unwrap() - 0.2).abs() < 1e-12);
    assert!(
        water["properties"]["composition"]
            .as_array()
            .expect("composition")
            .iter()
            .all(|c| !(c["substance_id"] == "sand" && c["phase"] == "aqueous")),
        "sand must not invent an aqueous phase"
    );
}

#[tokio::test]
async fn pour_second_cacl2_scoop_after_heating_succeeds() {
    let (app, state) = test_app_state().await;
    let (csrf_token, csrf_cookie, session_cookie) =
        register_user(&app, "cacl2-hot@chemlab.local").await;
    let cookies = format!("{session_cookie}; {csrf_cookie}");
    persist_filled_main_beaker(&state, &app, &cookies).await;

    for scoop_n in 1..=2 {
        assert_eq!(
            post_action(
                &app,
                &cookies,
                Some(&csrf_token),
                serde_json::json!({
                    "type": "use_tool",
                    "tool_item_id": "spoon-1",
                    "target_item_id": "beaker-cacl2"
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
                "type": "pour",
                "source_item_id": "spoon-1",
                "target_item_id": "beaker-water"
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK, "pour scoop {scoop_n}");
        let action = body_json(response).await;
        assert_eq!(
            action["scene"]["last_events"][1]["kind"], "dissolved",
            "scoop {scoop_n}: {action}"
        );
        assert_eq!(
            action["scene"]["last_events"][1]["message"],
            "Calcium chloride (CaCl2) dissolves in water at bench temperature."
        );
    }

    let scene_response = get_scene(&app, Some(&cookies)).await;
    assert_eq!(scene_response.status(), StatusCode::OK);
    let scene = body_json(scene_response).await;
    let water = scene["items"]
        .as_array()
        .expect("items")
        .iter()
        .find(|item| item["id"] == "beaker-water")
        .expect("beaker-water");
    let temperature = water["properties"]["temperature_c"]
        .as_f64()
        .expect("temperature_c");
    assert!(
        temperature > 20.0,
        "two CaCl2 scoops should leave water above 20 °C, got {temperature}"
    );
    let ca_mol = water["properties"]["composition"]
        .as_array()
        .expect("composition")
        .iter()
        .find(|c| c["substance_id"] == "ca2+" && c["phase"] == "aqueous")
        .and_then(|c| c["amount_mol"].as_f64())
        .expect("ca2+ moles");
    assert!(ca_mol > 0.0);
}

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
    let expected = 20.0 + chemlab_core::HEAT_K_PER_S * 1.5;
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
    let expected = 20.0 + chemlab_core::HEAT_K_PER_S * 1.5;
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
    let expected = 20.0 + chemlab_core::HEAT_K_PER_S * 2.0;
    assert!(
        (temperature - expected).abs() < 1.0,
        "elapsed dt must clamp to 2 s: expected ~{expected}, got {temperature}"
    );
}
