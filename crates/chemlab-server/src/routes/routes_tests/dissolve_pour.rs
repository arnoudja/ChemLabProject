use super::helpers::*;
use axum::http::StatusCode;

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
