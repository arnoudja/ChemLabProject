use super::super::*;
use super::helpers::*;

#[test]
fn use_tool_spoon_scoops_nacl_without_dissolving() {
    let mut scene = initial_bench_scene("lab-test");
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-nacl".into(),
        },
    )
    .unwrap();

    let spoon = item(&scene, "spoon-1");
    assert_eq!(spoon.location, "hand");
    assert_eq!(spoon.properties.holding.len(), 1);
    assert_eq!(spoon.properties.holding[0].substance_id, "nacl");
    assert_eq!(spoon.properties.holding[0].phase, "solid");
    assert_eq!(spoon.properties.holding[0].amount_scoop, Some(1));
    assert_eq!(
        spoon.properties.holding[0].amount_g,
        Some(SPOON_SCOOP_MASS_G)
    );
    assert_eq!(SPOON_SCOOP_MASS_G, 0.2);

    let nacl = item(&scene, "beaker-nacl");
    let stock = nacl
        .properties
        .composition
        .iter()
        .find(|c| c.substance_id == "nacl" && c.phase == "solid")
        .unwrap();
    assert_eq!(stock.amount_scoop, Some(9));
    assert_eq!(stock.amount_g, Some(9.0 * SPOON_SCOOP_MASS_G));

    assert!(scene.last_events.iter().any(|e| e.kind == "scooped"));
    // Scoop does not pour — the empty main beaker stays empty.
    let water = item(&scene, "beaker-water");
    assert!(water.properties.composition.is_empty());
}

#[test]
fn use_tool_fails_when_stock_is_empty() {
    let mut scene = initial_bench_scene("lab-test");
    let nacl = scene
        .items
        .iter_mut()
        .find(|i| i.id == "beaker-nacl")
        .unwrap();
    let stock = nacl
        .properties
        .composition
        .iter_mut()
        .find(|c| c.substance_id == "nacl")
        .unwrap();
    stock.amount_scoop = Some(0);
    stock.amount_g = Some(0.0);

    let err = apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-nacl".into(),
        },
    )
    .unwrap_err();
    assert_eq!(err, SceneError::InvalidAction);
    let spoon = item(&scene, "spoon-1");
    assert!(spoon.properties.holding.is_empty());
}

#[test]
fn scooping_stock_empty_leaves_zero_grams() {
    let mut scene = bench_with_water("lab-test");
    for _ in 0..10 {
        apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "beaker-nacl".into(),
            },
        )
        .unwrap();
        apply_action(
            &mut scene,
            Action::Pour {
                source_item_id: "spoon-1".into(),
                target_item_id: "beaker-water".into(),
            },
        )
        .unwrap();
    }

    let nacl = item(&scene, "beaker-nacl");
    let stock = nacl
        .properties
        .composition
        .iter()
        .find(|c| c.substance_id == "nacl" && c.phase == "solid")
        .unwrap();
    assert_eq!(stock.amount_scoop, Some(0));
    assert_eq!(
        stock.amount_g,
        Some(0.0),
        "leftover grams {:?} would still draw a floor sliver",
        stock.amount_g
    );
}

#[test]
fn use_tool_returns_held_nacl_to_matching_stock() {
    let mut scene = initial_bench_scene("lab-test");
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-nacl".into(),
        },
    )
    .unwrap();
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-nacl".into(),
        },
    )
    .unwrap();

    let spoon = item(&scene, "spoon-1");
    assert!(spoon.properties.holding.is_empty());
    let nacl = item(&scene, "beaker-nacl");
    let stock = nacl
        .properties
        .composition
        .iter()
        .find(|c| c.substance_id == "nacl" && c.phase == "solid")
        .unwrap();
    assert_eq!(stock.amount_scoop, Some(10));
    assert_eq!(stock.amount_g, Some(10.0 * SPOON_SCOOP_MASS_G));
    assert!(scene.last_events.iter().any(|e| e.kind == "returned"));
}

#[test]
fn use_tool_rejects_returning_nacl_into_sand_stock() {
    let mut scene = initial_bench_scene("lab-test");
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-nacl".into(),
        },
    )
    .unwrap();
    let err = apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-sand".into(),
        },
    )
    .unwrap_err();
    assert_eq!(err, SceneError::InvalidAction);
    let spoon = item(&scene, "spoon-1");
    assert_eq!(spoon.properties.holding[0].substance_id, "nacl");
    let sand = item(&scene, "beaker-sand");
    let stock = sand
        .properties
        .composition
        .iter()
        .find(|c| c.substance_id == "sand")
        .unwrap();
    assert_eq!(stock.amount_scoop, Some(10));
    let nacl = item(&scene, "beaker-nacl");
    let nacl_stock = nacl
        .properties
        .composition
        .iter()
        .find(|c| c.substance_id == "nacl")
        .unwrap();
    assert_eq!(nacl_stock.amount_scoop, Some(9));
}

#[test]
fn pour_nacl_into_water_dissolves_without_leftover_grains() {
    let mut scene = bench_with_water("lab-test");
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-nacl".into(),
        },
    )
    .unwrap();
    apply_action(
        &mut scene,
        Action::Pour {
            source_item_id: "spoon-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();

    let spoon = item(&scene, "spoon-1");
    assert!(spoon.properties.holding.is_empty());

    let water = item(&scene, "beaker-water");
    assert!(water
        .properties
        .composition
        .iter()
        .any(|c| c.substance_id == "na+" && c.phase == "aqueous"));
    assert!(water
        .properties
        .composition
        .iter()
        .any(|c| c.substance_id == "cl-" && c.phase == "aqueous"));
    let expected_mol = SPOON_SCOOP_MASS_G / NACL_MOLAR_MASS_G_PER_MOL;
    let na = water
        .properties
        .composition
        .iter()
        .find(|c| c.substance_id == "na+" && c.phase == "aqueous")
        .unwrap();
    let cl = water
        .properties
        .composition
        .iter()
        .find(|c| c.substance_id == "cl-" && c.phase == "aqueous")
        .unwrap();
    assert!((na.amount_mol.unwrap() - expected_mol).abs() < 1e-12);
    assert!((cl.amount_mol.unwrap() - expected_mol).abs() < 1e-12);
    assert!(!water
        .properties
        .composition
        .iter()
        .any(|c| c.substance_id == "nacl"));
    assert!(scene.last_events.iter().any(|e| {
        e.kind == "dissolved"
            && e.message == "Sodium chloride (NaCl) dissolves in water at bench temperature."
    }));
}

#[test]
fn pour_nacl_into_water_cools_solution_endothermically() {
    let mut scene = bench_with_water("lab-test");
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-nacl".into(),
        },
    )
    .unwrap();
    apply_action(
        &mut scene,
        Action::Pour {
            source_item_id: "spoon-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();

    let water = item(&scene, "beaker-water");
    let moles = SPOON_SCOOP_MASS_G / NACL_MOLAR_MASS_G_PER_MOL;
    let expected_delta_t =
        -(moles * NACL_DELTA_H_SOLUTION_J_PER_MOL) / (200.0 * WATER_SPECIFIC_HEAT_J_PER_G_K);
    let expected_t = 20.0 + expected_delta_t;
    assert!(
        expected_delta_t < 0.0,
        "NaCl dissolution must be endothermic (negative ΔT)"
    );
    let actual = water
        .properties
        .temperature_c
        .expect("water beaker should keep a temperature");
    assert!(
        (actual - expected_t).abs() < 1e-9,
        "expected {expected_t}, got {actual}"
    );
    // Ambient bench temperature is unchanged; only the solution cools.
    assert_eq!(scene.temperature_c, 20.0);
}

#[test]
fn pour_cacl2_into_water_dissolves_with_ions_and_heats_exothermically() {
    let mut scene = bench_with_water("lab-test");
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-cacl2".into(),
        },
    )
    .unwrap();

    let stock = item(&scene, "beaker-cacl2")
        .properties
        .composition
        .iter()
        .find(|c| c.substance_id == "cacl2" && c.phase == "solid")
        .unwrap();
    assert_eq!(stock.amount_scoop, Some(9));
    assert_eq!(stock.amount_g, Some(9.0 * SPOON_SCOOP_MASS_G));

    apply_action(
        &mut scene,
        Action::Pour {
            source_item_id: "spoon-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();

    let water = item(&scene, "beaker-water");
    let moles = SPOON_SCOOP_MASS_G / CACL2_MOLAR_MASS_G_PER_MOL;
    let ca = water
        .properties
        .composition
        .iter()
        .find(|c| c.substance_id == "ca2+" && c.phase == "aqueous")
        .unwrap();
    let cl = water
        .properties
        .composition
        .iter()
        .find(|c| c.substance_id == "cl-" && c.phase == "aqueous")
        .unwrap();
    assert!((ca.amount_mol.unwrap() - moles).abs() < 1e-12);
    assert!((cl.amount_mol.unwrap() - 2.0 * moles).abs() < 1e-12);
    assert!(!water
        .properties
        .composition
        .iter()
        .any(|c| c.substance_id == "cacl2"));

    let expected_delta_t =
        -(moles * CACL2_DELTA_H_SOLUTION_J_PER_MOL) / (200.0 * WATER_SPECIFIC_HEAT_J_PER_G_K);
    assert!(
        expected_delta_t > 0.0,
        "CaCl2 dissolution must be exothermic (positive ΔT)"
    );
    let actual = water
        .properties
        .temperature_c
        .expect("water beaker should keep a temperature");
    assert!((actual - (20.0 + expected_delta_t)).abs() < 1e-9);
    assert_eq!(scene.temperature_c, 20.0);
    assert!(scene.last_events.iter().any(|e| {
        e.kind == "dissolved"
            && e.message == "Calcium chloride (CaCl2) dissolves in water at bench temperature."
    }));
}

#[test]
fn use_tool_returns_held_cacl2_to_matching_stock() {
    let mut scene = initial_bench_scene("lab-test");
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-cacl2".into(),
        },
    )
    .unwrap();
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-cacl2".into(),
        },
    )
    .unwrap();

    let spoon = item(&scene, "spoon-1");
    assert!(spoon.properties.holding.is_empty());
    let stock = item(&scene, "beaker-cacl2")
        .properties
        .composition
        .iter()
        .find(|c| c.substance_id == "cacl2" && c.phase == "solid")
        .unwrap();
    assert_eq!(stock.amount_scoop, Some(10));
    assert_eq!(stock.amount_g, Some(10.0 * SPOON_SCOOP_MASS_G));
    assert!(scene.last_events.iter().any(|e| e.kind == "returned"));
}

#[test]
fn pour_sand_into_water_does_not_change_temperature() {
    let mut scene = bench_with_water("lab-test");
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-sand".into(),
        },
    )
    .unwrap();
    apply_action(
        &mut scene,
        Action::Pour {
            source_item_id: "spoon-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();

    let water = item(&scene, "beaker-water");
    assert_eq!(water.properties.temperature_c, Some(20.0));
}

#[test]
fn second_nacl_pour_updates_existing_ion_moles_without_duplicate_lines() {
    let mut scene = bench_with_water("lab-test");
    for _ in 0..2 {
        apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "beaker-nacl".into(),
            },
        )
        .unwrap();
        apply_action(
            &mut scene,
            Action::Pour {
                source_item_id: "spoon-1".into(),
                target_item_id: "beaker-water".into(),
            },
        )
        .unwrap();
    }

    let water = item(&scene, "beaker-water");
    let na_count = water
        .properties
        .composition
        .iter()
        .filter(|c| c.substance_id == "na+" && c.phase == "aqueous")
        .count();
    let cl_count = water
        .properties
        .composition
        .iter()
        .filter(|c| c.substance_id == "cl-" && c.phase == "aqueous")
        .count();
    assert_eq!(na_count, 1);
    assert_eq!(cl_count, 1);
    let expected_mol = 2.0 * SPOON_SCOOP_MASS_G / NACL_MOLAR_MASS_G_PER_MOL;
    let na = water
        .properties
        .composition
        .iter()
        .find(|c| c.substance_id == "na+")
        .unwrap();
    assert!((na.amount_mol.unwrap() - expected_mol).abs() < 1e-12);
}

#[test]
fn second_sand_pour_aggregates_solid_mass_on_one_line() {
    let mut scene = bench_with_water("lab-test");
    for _ in 0..2 {
        apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "beaker-sand".into(),
            },
        )
        .unwrap();
        apply_action(
            &mut scene,
            Action::Pour {
                source_item_id: "spoon-1".into(),
                target_item_id: "beaker-water".into(),
            },
        )
        .unwrap();
    }

    let water = item(&scene, "beaker-water");
    let sand_rows: Vec<_> = water
        .properties
        .composition
        .iter()
        .filter(|c| c.substance_id == "sand" && c.phase == "solid")
        .collect();
    assert_eq!(sand_rows.len(), 1);
    assert_eq!(sand_rows[0].amount_scoop, Some(2));
    assert!((sand_rows[0].amount_g.unwrap() - 2.0 * SPOON_SCOOP_MASS_G).abs() < 1e-12);
}

#[test]
fn pour_sand_into_water_leaves_undissolved_solid() {
    let mut scene = bench_with_water("lab-test");
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-sand".into(),
        },
    )
    .unwrap();
    apply_action(
        &mut scene,
        Action::Pour {
            source_item_id: "spoon-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();

    let spoon = item(&scene, "spoon-1");
    assert!(spoon.properties.holding.is_empty());

    let water = item(&scene, "beaker-water");
    assert!(water.properties.composition.iter().any(|c| {
        c.substance_id == "sand"
            && c.phase == "solid"
            && c.amount_scoop == Some(1)
            && c.amount_g == Some(SPOON_SCOOP_MASS_G)
    }));
    assert!(!water
        .properties
        .composition
        .iter()
        .any(|c| c.substance_id == "sand" && c.phase == "aqueous"));
    assert!(scene.last_events.iter().any(|e| {
        e.kind == "did_not_dissolve"
            && e.message == "Sand (silica) does not dissolve in water at bench temperature."
    }));
}

#[test]
fn reset_restores_empty_beaker_and_clears_holding_after_pour() {
    let mut scene = bench_with_water("lab-test");
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-nacl".into(),
        },
    )
    .unwrap();
    apply_action(
        &mut scene,
        Action::Pour {
            source_item_id: "spoon-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();
    scene.version = 2;

    apply_action(&mut scene, Action::Reset).unwrap();

    assert_eq!(scene.lab_id, "lab-test");
    assert_eq!(scene.version, 2);
    assert!(scene.last_events.iter().any(|e| e.kind == "reset"));

    let spoon = item(&scene, "spoon-1");
    assert_eq!(spoon.location, "bench");
    assert!(spoon.properties.holding.is_empty());

    let water = item(&scene, "beaker-water");
    assert_eq!(water.label, "Beaker");
    assert_eq!(water.properties.fill_ml, Some(0.0));
    assert!(water.properties.composition.is_empty());
}

#[test]
fn unknown_item_ids_return_scene_error() {
    let mut scene = initial_bench_scene("lab-test");
    let err = apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "no-such-tool".into(),
            target_item_id: "beaker-nacl".into(),
        },
    )
    .unwrap_err();
    assert_eq!(err, SceneError::UnknownItem);

    let err = apply_action(
        &mut scene,
        Action::Pour {
            source_item_id: "spoon-1".into(),
            target_item_id: "no-such-beaker".into(),
        },
    )
    .unwrap_err();
    assert_eq!(err, SceneError::UnknownItem);
}

#[test]
fn pour_with_empty_holding_returns_error() {
    let mut scene = initial_bench_scene("lab-test");
    let err = apply_action(
        &mut scene,
        Action::Pour {
            source_item_id: "spoon-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap_err();
    assert_eq!(err, SceneError::EmptyHolding);
}

#[test]
fn pour_into_dry_beaker_returns_invalid_action() {
    let mut scene = initial_bench_scene("lab-test");
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-nacl".into(),
        },
    )
    .unwrap();

    let err = apply_action(
        &mut scene,
        Action::Pour {
            source_item_id: "spoon-1".into(),
            target_item_id: "beaker-sand".into(),
        },
    )
    .unwrap_err();
    assert_eq!(err, SceneError::InvalidAction);
    let spoon = item(&scene, "spoon-1");
    assert_eq!(spoon.properties.holding[0].substance_id, "nacl");
}

#[test]
fn pour_sand_at_non_bench_temperature_leaves_undissolved_solid() {
    let mut scene = bench_with_water("lab-test");
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-sand".into(),
        },
    )
    .unwrap();

    let water = scene
        .items
        .iter_mut()
        .find(|i| i.id == "beaker-water")
        .unwrap();
    water.properties.temperature_c = Some(21.0);

    apply_action(
        &mut scene,
        Action::Pour {
            source_item_id: "spoon-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();

    let water = item(&scene, "beaker-water");
    assert_eq!(water.properties.temperature_c, Some(21.0));
    assert!(water.properties.composition.iter().any(|c| {
        c.substance_id == "sand"
            && c.phase == "solid"
            && c.amount_scoop == Some(1)
            && (c.amount_g.unwrap() - SPOON_SCOOP_MASS_G).abs() < 1e-12
    }));
    assert!(!water
        .properties
        .composition
        .iter()
        .any(|c| c.substance_id == "sand" && c.phase == "aqueous"));
    assert!(scene.last_events.iter().any(|e| {
        e.kind == "did_not_dissolve"
            && e.message == "Sand (silica) does not dissolve in water at bench temperature."
    }));
    assert!(item(&scene, "spoon-1").properties.holding.is_empty());
}

#[test]
fn pour_sand_succeeds_after_cacl2_exothermic_heating() {
    let mut scene = bench_with_water("lab-test");

    // One scoop only raises T by ~0.175 °C (still rounds to 20). Pour until the
    // dissolve lookup sees a non-bench integer °C — the real warm-water bug path.
    let mut after_heat = 20.0;
    for _ in 0..4 {
        apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "beaker-cacl2".into(),
            },
        )
        .unwrap();
        apply_action(
            &mut scene,
            Action::Pour {
                source_item_id: "spoon-1".into(),
                target_item_id: "beaker-water".into(),
            },
        )
        .unwrap();
        after_heat = item(&scene, "beaker-water")
            .properties
            .temperature_c
            .expect("water beaker should keep a temperature");
    }
    assert!(
        after_heat.round() as i32 != 20,
        "CaCl2 heating must leave a non-bench lookup T, got {after_heat}"
    );

    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-sand".into(),
        },
    )
    .unwrap();
    apply_action(
        &mut scene,
        Action::Pour {
            source_item_id: "spoon-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();

    let water = item(&scene, "beaker-water");
    assert_eq!(water.properties.temperature_c, Some(after_heat));
    assert!(water.properties.composition.iter().any(|c| {
        c.substance_id == "sand"
            && c.phase == "solid"
            && c.amount_scoop == Some(1)
            && (c.amount_g.unwrap() - SPOON_SCOOP_MASS_G).abs() < 1e-12
    }));
    assert!(!water
        .properties
        .composition
        .iter()
        .any(|c| c.substance_id == "sand" && c.phase == "aqueous"));
    assert!(scene.last_events.iter().any(|e| {
        e.kind == "did_not_dissolve"
            && e.message == "Sand (silica) does not dissolve in water at bench temperature."
    }));
}

#[test]
fn pour_second_cacl2_scoop_succeeds_after_exothermic_heating() {
    let mut scene = bench_with_water("lab-test");

    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-cacl2".into(),
        },
    )
    .unwrap();
    apply_action(
        &mut scene,
        Action::Pour {
            source_item_id: "spoon-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();

    let after_first = item(&scene, "beaker-water")
        .properties
        .temperature_c
        .expect("water beaker should keep a temperature");
    assert!(
        after_first > 20.0,
        "first CaCl2 scoop must raise beaker T above 20 °C, got {after_first}"
    );

    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-cacl2".into(),
        },
    )
    .unwrap();
    apply_action(
        &mut scene,
        Action::Pour {
            source_item_id: "spoon-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();

    let water = item(&scene, "beaker-water");
    let moles = 2.0 * (SPOON_SCOOP_MASS_G / CACL2_MOLAR_MASS_G_PER_MOL);
    let ca = water
        .properties
        .composition
        .iter()
        .find(|c| c.substance_id == "ca2+" && c.phase == "aqueous")
        .unwrap();
    assert!((ca.amount_mol.unwrap() - moles).abs() < 1e-12);

    let after_second = water
        .properties
        .temperature_c
        .expect("water beaker should keep a temperature");
    assert!(
        after_second > after_first,
        "second CaCl2 scoop should heat further: {after_first} -> {after_second}"
    );
    assert!(scene.last_events.iter().any(|e| {
        e.kind == "dissolved"
            && e.message == "Calcium chloride (CaCl2) dissolves in water at bench temperature."
    }));
    assert!(item(&scene, "spoon-1").properties.holding.is_empty());
}

#[test]
fn pour_nacl_succeeds_after_mild_heating_above_bench() {
    let mut scene = bench_with_water("lab-test");
    let water = scene
        .items
        .iter_mut()
        .find(|i| i.id == "beaker-water")
        .unwrap();
    water.properties.temperature_c = Some(21.5);

    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-nacl".into(),
        },
    )
    .unwrap();
    apply_action(
        &mut scene,
        Action::Pour {
            source_item_id: "spoon-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();

    let water = item(&scene, "beaker-water");
    let moles = SPOON_SCOOP_MASS_G / NACL_MOLAR_MASS_G_PER_MOL;
    let expected_t =
        21.5 - (moles * NACL_DELTA_H_SOLUTION_J_PER_MOL) / (200.0 * WATER_SPECIFIC_HEAT_J_PER_G_K);
    let actual = water
        .properties
        .temperature_c
        .expect("water beaker should keep a temperature");
    assert!((actual - expected_t).abs() < 1e-9);
    assert!(scene.last_events.iter().any(|e| {
        e.kind == "dissolved"
            && e.message == "Sodium chloride (NaCl) dissolves in water at bench temperature."
    }));
}

#[test]
fn put_away_returns_held_nacl_to_stock_and_empties_spoon() {
    let mut scene = initial_bench_scene("lab-test");
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-nacl".into(),
        },
    )
    .unwrap();
    apply_action(
        &mut scene,
        Action::PutAway {
            tool_item_id: "spoon-1".into(),
        },
    )
    .unwrap();

    let spoon = item(&scene, "spoon-1");
    assert!(spoon.properties.holding.is_empty());
    assert_eq!(spoon.location, "bench");
    let stock = item(&scene, "beaker-nacl")
        .properties
        .composition
        .iter()
        .find(|c| c.substance_id == "nacl" && c.phase == "solid")
        .unwrap();
    assert_eq!(stock.amount_scoop, Some(10));
    assert_eq!(stock.amount_g, Some(10.0 * SPOON_SCOOP_MASS_G));
    assert!(scene.last_events.iter().any(|e| e.kind == "returned"));
}

#[test]
fn put_away_returns_held_sand_to_stock_and_empties_spoon() {
    let mut scene = initial_bench_scene("lab-test");
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-sand".into(),
        },
    )
    .unwrap();
    apply_action(
        &mut scene,
        Action::PutAway {
            tool_item_id: "spoon-1".into(),
        },
    )
    .unwrap();

    let spoon = item(&scene, "spoon-1");
    assert!(spoon.properties.holding.is_empty());
    assert_eq!(spoon.location, "bench");
    let stock = item(&scene, "beaker-sand")
        .properties
        .composition
        .iter()
        .find(|c| c.substance_id == "sand" && c.phase == "solid")
        .unwrap();
    assert_eq!(stock.amount_scoop, Some(10));
    assert_eq!(stock.amount_g, Some(10.0 * SPOON_SCOOP_MASS_G));
    assert!(scene.last_events.iter().any(|e| e.kind == "returned"));
}

#[test]
fn put_away_returns_held_cacl2_to_stock_and_empties_spoon() {
    let mut scene = initial_bench_scene("lab-test");
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-cacl2".into(),
        },
    )
    .unwrap();
    apply_action(
        &mut scene,
        Action::PutAway {
            tool_item_id: "spoon-1".into(),
        },
    )
    .unwrap();

    let spoon = item(&scene, "spoon-1");
    assert!(spoon.properties.holding.is_empty());
    assert_eq!(spoon.location, "bench");
    let stock = item(&scene, "beaker-cacl2")
        .properties
        .composition
        .iter()
        .find(|c| c.substance_id == "cacl2" && c.phase == "solid")
        .unwrap();
    assert_eq!(stock.amount_scoop, Some(10));
    assert_eq!(stock.amount_g, Some(10.0 * SPOON_SCOOP_MASS_G));
    assert!(scene.last_events.iter().any(|e| e.kind == "returned"));
}

#[test]
fn put_away_empty_spoon_is_noop() {
    let mut scene = initial_bench_scene("lab-test");
    let before = scene.clone();
    apply_action(
        &mut scene,
        Action::PutAway {
            tool_item_id: "spoon-1".into(),
        },
    )
    .unwrap();

    assert_eq!(item(&scene, "spoon-1").properties.holding, Vec::new());
    assert_eq!(item(&scene, "spoon-1").location, "bench");
    assert_eq!(
        item(&scene, "beaker-nacl").properties.composition,
        item(&before, "beaker-nacl").properties.composition
    );
    assert_eq!(
        item(&scene, "beaker-sand").properties.composition,
        item(&before, "beaker-sand").properties.composition
    );
    assert_eq!(
        item(&scene, "beaker-cacl2").properties.composition,
        item(&before, "beaker-cacl2").properties.composition
    );
    assert!(scene.last_events.is_empty());
}

#[test]
fn use_tool_spoon_returns_evaporated_nacl_to_emptied_stock() {
    // Tongs can dump solids into an emptied exact-type stock; spoon must too after
    // evaporating aqueous NaCl back to dry solid (stock composition is empty).
    let mut scene = initial_bench_scene("lab-test");
    use_tongs(&mut scene, "beaker-nacl").unwrap();
    use_tongs(&mut scene, "dish-1").unwrap();
    put_tongs_away(&mut scene).unwrap();
    assert_eq!(solid_g(item(&scene, "beaker-nacl"), "nacl"), 0.0);
    assert!(item(&scene, "beaker-nacl")
        .properties
        .composition
        .iter()
        .all(|c| !(c.phase == "solid" && c.substance_id == "nacl")));

    let dish = scene.items.iter_mut().find(|i| i.id == "dish-1").unwrap();
    dish.properties.temperature_c = Some(20.0);
    dish.properties.composition.insert(
        0,
        CompositionEntry {
            substance_id: "water".into(),
            phase: "liquid".into(),
            amount_ml: Some(5.0),
            amount_scoop: None,
            amount_g: None,
            amount_mol: None,
        },
    );
    crate::solubility::enforce_saturation(dish);

    apply_action(
        &mut scene,
        Action::ToggleBurner {
            burner_item_id: "burner-1".into(),
        },
    )
    .unwrap();
    apply_elapsed(&mut scene, 8.0);
    apply_elapsed(&mut scene, 12.0);

    assert!(water_ml(item(&scene, "dish-1")) < 1e-9);
    assert!((solid_g(item(&scene, "dish-1"), "nacl") - 2.0).abs() < 1e-9);

    scoop_dish(&mut scene).unwrap();
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-nacl".into(),
        },
    )
    .unwrap();

    assert!(item(&scene, "spoon-1").properties.holding.is_empty());
    assert!((solid_g(item(&scene, "beaker-nacl"), "nacl") - SPOON_SCOOP_MASS_G).abs() < 1e-12);
    assert!((solid_g(item(&scene, "dish-1"), "nacl") - (2.0 - SPOON_SCOOP_MASS_G)).abs() < 1e-9);
    assert!(scene.last_events.iter().any(|e| e.kind == "returned"));
}

#[test]
fn pour_scoop_into_empty_beaker_adds_undissolved_solid() {
    let mut scene = initial_bench_scene("lab-test");
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-nacl".into(),
        },
    )
    .unwrap();
    apply_action(
        &mut scene,
        Action::Pour {
            source_item_id: "spoon-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();

    assert!(item(&scene, "spoon-1").properties.holding.is_empty());
    assert!((solid_g(item(&scene, "beaker-water"), "nacl") - SPOON_SCOOP_MASS_G).abs() < 1e-12);
    assert_eq!(aqueous_mol(item(&scene, "beaker-water"), "na+"), 0.0);
    assert!(scene.last_events.iter().any(|e| e.kind == "poured"));
    assert!(!scene.last_events.iter().any(|e| e.kind == "dissolved"));
}

#[test]
fn use_tool_spoon_scoops_dish_solids_in_mass_ratio() {
    let mut scene = initial_bench_scene("lab-test");
    set_dry_dish_solids(&mut scene, vec![solid("nacl", 0.6), solid("cacl2", 0.4)]);
    scoop_dish(&mut scene).unwrap();

    let spoon = item(&scene, "spoon-1");
    assert_eq!(spoon.location, "hand");
    assert_eq!(spoon.properties.source_item_id.as_deref(), Some("dish-1"));
    assert!((holding_g(spoon, "nacl") - 0.12).abs() < 1e-12);
    assert!((holding_g(spoon, "cacl2") - 0.08).abs() < 1e-12);
    assert!(
        (holding_g(spoon, "nacl") + holding_g(spoon, "cacl2") - SPOON_SCOOP_MASS_G).abs() < 1e-12
    );
    assert!((solid_g(item(&scene, "dish-1"), "nacl") - 0.48).abs() < 1e-12);
    assert!((solid_g(item(&scene, "dish-1"), "cacl2") - 0.32).abs() < 1e-12);
    assert!(scene.last_events.iter().any(|e| e.kind == "scooped"));
    assert_eq!(solid_g(item(&scene, "beaker-nacl"), "nacl"), 2.0);
    assert_eq!(solid_g(item(&scene, "beaker-cacl2"), "cacl2"), 2.0);
}

#[test]
fn use_tool_spoon_takes_all_when_dish_solids_below_scoop_mass() {
    let mut scene = initial_bench_scene("lab-test");
    set_dry_dish_solids(&mut scene, vec![solid("nacl", 0.05), solid("sand", 0.05)]);
    scoop_dish(&mut scene).unwrap();

    let spoon = item(&scene, "spoon-1");
    assert!((holding_g(spoon, "nacl") - 0.05).abs() < 1e-12);
    assert!((holding_g(spoon, "sand") - 0.05).abs() < 1e-12);
    assert!(item(&scene, "dish-1")
        .properties
        .composition
        .iter()
        .all(|c| c.phase != "solid"));
    assert_eq!(solid_g(item(&scene, "dish-1"), "nacl"), 0.0);
    assert_eq!(spoon.properties.source_item_id.as_deref(), Some("dish-1"));
}

#[test]
fn pour_dumps_mixed_dish_scoop_into_water_and_clears_spoon() {
    let mut scene = bench_with_water("lab-test");
    set_dry_dish_solids(&mut scene, vec![solid("nacl", 0.6), solid("cacl2", 0.4)]);
    scoop_dish(&mut scene).unwrap();
    apply_action(
        &mut scene,
        Action::Pour {
            source_item_id: "spoon-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();

    let spoon = item(&scene, "spoon-1");
    assert!(spoon.properties.holding.is_empty());
    assert_eq!(spoon.properties.source_item_id, None);
    let water = item(&scene, "beaker-water");
    let expected_na = 0.12 / NACL_MOLAR_MASS_G_PER_MOL;
    let expected_ca = 0.08 / CACL2_MOLAR_MASS_G_PER_MOL;
    assert!((aqueous_mol(water, "na+") - expected_na).abs() < 1e-9);
    assert!((aqueous_mol(water, "ca2+") - expected_ca).abs() < 1e-9);
    assert!((aqueous_mol(water, "cl-") - (expected_na + 2.0 * expected_ca)).abs() < 1e-9);
    assert!(scene.last_events.iter().any(|e| e.kind == "dissolved"));
}

#[test]
fn use_tool_dumps_dish_scoop_into_water() {
    let mut scene = bench_with_water("lab-test");
    set_dry_dish_solids(&mut scene, vec![solid("nacl", 0.5)]);
    scoop_dish(&mut scene).unwrap();
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();

    let spoon = item(&scene, "spoon-1");
    assert!(spoon.properties.holding.is_empty());
    assert_eq!(spoon.properties.source_item_id, None);
    let expected_na = SPOON_SCOOP_MASS_G / NACL_MOLAR_MASS_G_PER_MOL;
    assert!((aqueous_mol(item(&scene, "beaker-water"), "na+") - expected_na).abs() < 1e-9);
    assert!((solid_g(item(&scene, "dish-1"), "nacl") - 0.3).abs() < 1e-12);
}

#[test]
fn use_tool_returns_single_species_dish_scoop_to_matching_stock() {
    let mut scene = initial_bench_scene("lab-test");
    set_dry_dish_solids(&mut scene, vec![solid("nacl", 0.5)]);
    scoop_dish(&mut scene).unwrap();
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-nacl".into(),
        },
    )
    .unwrap();

    let spoon = item(&scene, "spoon-1");
    assert!(spoon.properties.holding.is_empty());
    assert_eq!(spoon.properties.source_item_id, None);
    assert!((solid_g(item(&scene, "beaker-nacl"), "nacl") - 2.2).abs() < 1e-12);
    assert!((solid_g(item(&scene, "dish-1"), "nacl") - 0.3).abs() < 1e-12);
    assert!(scene.last_events.iter().any(|e| e.kind == "returned"));
}

#[test]
fn use_tool_returns_short_dish_scoop_mass_to_matching_stock() {
    let mut scene = initial_bench_scene("lab-test");
    set_dry_dish_solids(&mut scene, vec![solid("nacl", 0.1)]);
    scoop_dish(&mut scene).unwrap();
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-nacl".into(),
        },
    )
    .unwrap();

    assert!(item(&scene, "spoon-1").properties.holding.is_empty());
    assert!((solid_g(item(&scene, "beaker-nacl"), "nacl") - 2.1).abs() < 1e-12);
    assert_eq!(solid_g(item(&scene, "dish-1"), "nacl"), 0.0);
}

#[test]
fn use_tool_rejects_mixed_dish_scoop_into_stock() {
    let mut scene = initial_bench_scene("lab-test");
    set_dry_dish_solids(&mut scene, vec![solid("nacl", 0.6), solid("cacl2", 0.4)]);
    scoop_dish(&mut scene).unwrap();
    let err = apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-nacl".into(),
        },
    )
    .unwrap_err();
    assert_eq!(err, SceneError::InvalidAction);
    let spoon = item(&scene, "spoon-1");
    assert!((holding_g(spoon, "nacl") - 0.12).abs() < 1e-12);
    assert!((holding_g(spoon, "cacl2") - 0.08).abs() < 1e-12);
    assert_eq!(solid_g(item(&scene, "beaker-nacl"), "nacl"), 2.0);
}

#[test]
fn use_tool_rejects_wet_dish_and_stays_empty() {
    let mut scene = initial_bench_scene("lab-test");
    let dish = scene.items.iter_mut().find(|i| i.id == "dish-1").unwrap();
    dish.properties.composition = vec![
        CompositionEntry {
            substance_id: "water".into(),
            phase: "liquid".into(),
            amount_ml: Some(5.0),
            amount_scoop: None,
            amount_g: None,
            amount_mol: None,
        },
        solid("nacl", 0.5),
    ];
    crate::solubility::sync_fill_ml(dish);

    let err = scoop_dish(&mut scene).unwrap_err();
    assert_eq!(err, SceneError::InvalidAction);
    let spoon = item(&scene, "spoon-1");
    assert!(spoon.properties.holding.is_empty());
    assert_eq!(spoon.properties.source_item_id, None);
    assert!((solid_g(item(&scene, "dish-1"), "nacl") - 0.5).abs() < 1e-12);
    assert!((water_ml(item(&scene, "dish-1")) - 5.0).abs() < 1e-12);
}

#[test]
fn use_tool_rejects_empty_dish_and_tongs_held_dish() {
    let mut scene = initial_bench_scene("lab-test");
    assert_eq!(
        scoop_dish(&mut scene).unwrap_err(),
        SceneError::InvalidAction
    );

    set_dry_dish_solids(&mut scene, vec![solid("sand", 0.4)]);
    scene
        .items
        .iter_mut()
        .find(|i| i.id == "dish-1")
        .unwrap()
        .location = "held".into();
    assert_eq!(
        scoop_dish(&mut scene).unwrap_err(),
        SceneError::InvalidAction
    );
    assert!(item(&scene, "spoon-1").properties.holding.is_empty());
    assert!((solid_g(item(&scene, "dish-1"), "sand") - 0.4).abs() < 1e-12);
}

#[test]
fn put_away_returns_dish_scoop_to_dish_not_stock() {
    let mut scene = initial_bench_scene("lab-test");
    set_dry_dish_solids(&mut scene, vec![solid("nacl", 0.6), solid("cacl2", 0.4)]);
    scoop_dish(&mut scene).unwrap();
    apply_action(
        &mut scene,
        Action::PutAway {
            tool_item_id: "spoon-1".into(),
        },
    )
    .unwrap();

    let spoon = item(&scene, "spoon-1");
    assert!(spoon.properties.holding.is_empty());
    assert_eq!(spoon.location, "bench");
    assert_eq!(spoon.properties.source_item_id, None);
    assert!((solid_g(item(&scene, "dish-1"), "nacl") - 0.6).abs() < 1e-12);
    assert!((solid_g(item(&scene, "dish-1"), "cacl2") - 0.4).abs() < 1e-12);
    assert_eq!(solid_g(item(&scene, "beaker-nacl"), "nacl"), 2.0);
    assert_eq!(solid_g(item(&scene, "beaker-cacl2"), "cacl2"), 2.0);
}

#[test]
fn returning_dish_scoop_then_stock_scoop_conserves_nacl_mass() {
    let mut scene = initial_bench_scene("lab-test");
    set_dry_dish_solids(&mut scene, vec![solid("nacl", 0.5)]);
    let start_total =
        solid_g(item(&scene, "beaker-nacl"), "nacl") + solid_g(item(&scene, "dish-1"), "nacl");
    scoop_dish(&mut scene).unwrap();
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-nacl".into(),
        },
    )
    .unwrap();
    assert!((solid_g(item(&scene, "beaker-nacl"), "nacl") - 2.2).abs() < 1e-12);

    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-nacl".into(),
        },
    )
    .unwrap();

    let conserved = solid_g(item(&scene, "beaker-nacl"), "nacl")
        + holding_g(item(&scene, "spoon-1"), "nacl")
        + solid_g(item(&scene, "dish-1"), "nacl");
    assert!(
        (conserved - start_total).abs() < 1e-12,
        "nacl mass leaked: start {start_total}, after {conserved}"
    );
    assert!((solid_g(item(&scene, "beaker-nacl"), "nacl") - 2.0).abs() < 1e-12);
    assert!((holding_g(item(&scene, "spoon-1"), "nacl") - SPOON_SCOOP_MASS_G).abs() < 1e-12);
    assert!((solid_g(item(&scene, "dish-1"), "nacl") - 0.3).abs() < 1e-12);
}

#[test]
fn returning_0_25g_nacl_to_empty_stock_is_scoopable() {
    let mut scene = bench_with_water("lab-test");
    empty_nacl_stock(&mut scene);
    set_dry_dish_solids(&mut scene, vec![solid("nacl", 0.25)]);

    scoop_dish(&mut scene).unwrap();
    scoop_nacl_stock(&mut scene).unwrap();
    scoop_dish(&mut scene).unwrap();
    scoop_nacl_stock(&mut scene).unwrap();

    let stock = item(&scene, "beaker-nacl")
        .properties
        .composition
        .iter()
        .find(|c| c.substance_id == "nacl" && c.phase == "solid")
        .unwrap();
    assert!((solid_g(item(&scene, "beaker-nacl"), "nacl") - 0.25).abs() < 1e-12);
    assert_eq!(stock.amount_scoop.unwrap_or(0), 0);
    assert!(item(&scene, "spoon-1").properties.holding.is_empty());

    scoop_nacl_stock(&mut scene).unwrap();
    assert!((holding_g(item(&scene, "spoon-1"), "nacl") - SPOON_SCOOP_MASS_G).abs() < 1e-12);
    assert!((solid_g(item(&scene, "beaker-nacl"), "nacl") - 0.05).abs() < 1e-12);

    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();

    scoop_nacl_stock(&mut scene).unwrap();
    assert!((holding_g(item(&scene, "spoon-1"), "nacl") - 0.05).abs() < 1e-12);
    assert_eq!(solid_g(item(&scene, "beaker-nacl"), "nacl"), 0.0);
    let emptied = item(&scene, "beaker-nacl")
        .properties
        .composition
        .iter()
        .find(|c| c.substance_id == "nacl" && c.phase == "solid")
        .unwrap();
    assert_eq!(emptied.amount_scoop, Some(0));
    assert_eq!(emptied.amount_g, Some(0.0));
}

#[test]
fn use_tool_holding_stock_scoop_deposits_into_dish() {
    let mut scene = initial_bench_scene("lab-test");
    set_dry_dish_solids(&mut scene, vec![solid("nacl", 0.5)]);
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-nacl".into(),
        },
    )
    .unwrap();

    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "dish-1".into(),
        },
    )
    .unwrap();

    assert!(item(&scene, "spoon-1").properties.holding.is_empty());
    assert!((solid_g(item(&scene, "dish-1"), "nacl") - 0.7).abs() < 1e-12);
    assert!((solid_g(item(&scene, "beaker-nacl"), "nacl") - 1.8).abs() < 1e-12);
    assert!(scene.last_events.iter().any(|e| e.kind == "returned"));
}

#[test]
fn use_tool_spoon_on_beaker_h2o_is_invalid() {
    let mut scene = initial_bench_scene("lab-test");
    let err = apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-h2o".into(),
        },
    )
    .unwrap_err();
    assert_eq!(err, SceneError::InvalidAction);
    assert!(item(&scene, "spoon-1").properties.holding.is_empty());
    assert!((water_ml(item(&scene, "beaker-h2o")) - 100.0).abs() < 1e-9);
}

#[test]
fn pour_held_nacl_into_beaker_h2o_is_invalid() {
    let mut scene = initial_bench_scene("lab-test");
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-nacl".into(),
        },
    )
    .unwrap();

    let err = apply_action(
        &mut scene,
        Action::Pour {
            source_item_id: "spoon-1".into(),
            target_item_id: "beaker-h2o".into(),
        },
    )
    .unwrap_err();
    assert_eq!(err, SceneError::InvalidAction);

    let spoon = item(&scene, "spoon-1");
    assert_eq!(spoon.location, "hand");
    assert!((holding_g(spoon, "nacl") - SPOON_SCOOP_MASS_G).abs() < 1e-12);

    let h2o = item(&scene, "beaker-h2o");
    assert!((water_ml(h2o) - DISTILLED_WATER_CAPACITY_ML).abs() < 1e-9);
    assert!(h2o
        .properties
        .composition
        .iter()
        .all(|c| c.substance_id == "water" && c.phase == "liquid"));
    assert_eq!(aqueous_mol(h2o, "na+"), 0.0);
    assert_eq!(solid_g(item(&scene, "beaker-nacl"), "nacl"), 1.8);
}

#[test]
fn use_tool_holding_spoon_on_beaker_h2o_is_invalid() {
    let mut scene = initial_bench_scene("lab-test");
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-nacl".into(),
        },
    )
    .unwrap();
    let before = scene.clone();

    let err = apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-h2o".into(),
        },
    )
    .unwrap_err();
    assert_eq!(err, SceneError::InvalidAction);
    assert_eq!(scene.items, before.items);
    assert_eq!(scene.temperature_c, before.temperature_c);
    assert_eq!(scene.version, before.version);
}
