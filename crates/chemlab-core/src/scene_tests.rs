use super::*;

fn item<'a>(scene: &'a Scene, id: &str) -> &'a SceneItem {
    scene
        .items
        .iter()
        .find(|i| i.id == id)
        .unwrap_or_else(|| panic!("missing item {id}"))
}

/// Tests that pour, pipette, or dissolve into the main beaker start from a filled vessel.
const FILLED_MAIN_BEAKER_ML: f64 = 200.0;

fn fill_main_beaker(scene: &mut Scene, amount_ml: f64) {
    let water = scene
        .items
        .iter_mut()
        .find(|item| item.id == "beaker-water")
        .expect("beaker-water");
    water.properties.fill_ml = Some(amount_ml);
    water.properties.composition = vec![CompositionEntry {
        substance_id: "water".into(),
        phase: "liquid".into(),
        amount_ml: Some(amount_ml),
        amount_scoop: None,
        amount_g: None,
        amount_mol: None,
    }];
}

fn bench_with_water(lab_id: &str) -> Scene {
    let mut scene = initial_bench_scene(lab_id);
    fill_main_beaker(&mut scene, FILLED_MAIN_BEAKER_ML);
    scene
}

#[test]
fn initial_bench_scene_has_twelve_items_with_water_and_evaporation_bench() {
    let scene = initial_bench_scene("lab-test");
    assert_eq!(scene.lab_id, "lab-test");
    assert_eq!(scene.temperature_c, 20.0);
    assert_eq!(scene.version, 0);
    assert!(scene.last_events.is_empty());
    assert_eq!(scene.last_applied_unix_ms, None);
    assert_eq!(scene.items.len(), 12);

    let ids: Vec<_> = scene.items.iter().map(|i| i.id.as_str()).collect();
    assert!(ids.contains(&"spoon-1"));
    assert!(ids.contains(&"beaker-h2o"));
    assert!(ids.contains(&"beaker-nacl"));
    assert!(ids.contains(&"beaker-cacl2"));
    assert!(ids.contains(&"beaker-sand"));
    assert!(ids.contains(&"beaker-water"));
    assert!(ids.contains(&"pipette-1"));
    assert!(ids.contains(&"dish-1"));
    assert!(ids.contains(&"burner-1"));
    assert!(ids.contains(&"tongs-1"));
    assert!(ids.contains(&"beaker-filtrate"));
    assert!(ids.contains(&"filter-paper-1"));

    let distilled = item(&scene, "beaker-h2o");
    assert_eq!(distilled.kind, "beaker");
    assert_eq!(distilled.label, "Distilled water");
    assert_eq!(distilled.location, "bench");
    assert_eq!(distilled.properties.volume_ml, Some(100.0));
    assert_eq!(distilled.properties.fill_ml, Some(100.0));
    assert_eq!(distilled.properties.transparent, Some(true));
    assert_eq!(distilled.properties.colourless, Some(true));
    assert_eq!(distilled.properties.temperature_c, Some(20.0));
    assert_eq!(distilled.properties.composition.len(), 1);
    assert_eq!(distilled.properties.composition[0].substance_id, "water");
    assert_eq!(distilled.properties.composition[0].phase, "liquid");
    assert_eq!(distilled.properties.composition[0].amount_ml, Some(100.0));

    let water = item(&scene, "beaker-water");
    assert_eq!(water.kind, "beaker");
    assert_eq!(water.label, "Beaker");
    assert_eq!(water.location, "bench");
    assert_eq!(water.properties.volume_ml, Some(WATER_CAPACITY_ML));
    assert_eq!(water.properties.fill_ml, Some(0.0));
    assert_eq!(water.properties.transparent, Some(true));
    assert_eq!(water.properties.colourless, Some(true));
    assert_eq!(water.properties.temperature_c, Some(20.0));
    assert!(water.properties.composition.is_empty());

    let nacl = item(&scene, "beaker-nacl");
    assert!(nacl
        .properties
        .composition
        .iter()
        .any(|c| c.substance_id == "nacl" && c.phase == "solid"));

    let cacl2 = item(&scene, "beaker-cacl2");
    assert_eq!(cacl2.label, "Calcium chloride");
    assert!(cacl2
        .properties
        .composition
        .iter()
        .any(|c| c.substance_id == "cacl2" && c.phase == "solid" && c.amount_g == Some(2.0)));

    let sand = item(&scene, "beaker-sand");
    assert!(sand
        .properties
        .composition
        .iter()
        .any(|c| c.substance_id == "sand" && c.phase == "solid"));

    let spoon = item(&scene, "spoon-1");
    assert_eq!(spoon.kind, "spoon");
    assert!(spoon.properties.holding.is_empty());

    let pipette = item(&scene, "pipette-1");
    assert_eq!(pipette.kind, "pipette");
    assert_eq!(pipette.location, "bench");
    assert!(pipette.properties.holding.is_empty());
    assert_eq!(pipette.properties.source_item_id, None);

    let dish = item(&scene, "dish-1");
    assert_eq!(dish.kind, "evaporation_dish");
    assert_eq!(dish.properties.volume_ml, Some(DISH_CAPACITY_ML));
    assert_eq!(dish.properties.fill_ml, Some(0.0));
    assert_eq!(dish.properties.temperature_c, Some(AMBIENT_TEMPERATURE_C));
    assert!(dish.properties.composition.is_empty());

    let burner = item(&scene, "burner-1");
    assert_eq!(burner.kind, "burner");
    assert_eq!(burner.properties.on, Some(false));

    let tongs = item(&scene, "tongs-1");
    assert_eq!(tongs.kind, "tongs");
    assert_eq!(tongs.label, "Tongs");
    assert_eq!(tongs.location, "bench");
    assert_eq!(tongs.properties.source_item_id, None);
    assert!(tongs.properties.holding.is_empty());
}

#[test]
fn ensure_default_bench_items_restores_beaker_h2o_without_resetting_vessels() {
    let mut scene = initial_bench_scene("lab-test");
    {
        let water = scene
            .items
            .iter_mut()
            .find(|item| item.id == "beaker-water")
            .unwrap();
        water.label = "Water".into();
        water.properties.fill_ml = Some(150.0);
        water.properties.composition = vec![CompositionEntry {
            substance_id: "water".into(),
            phase: "liquid".into(),
            amount_ml: Some(150.0),
            amount_scoop: None,
            amount_g: None,
            amount_mol: None,
        }];
    }
    scene.items.retain(|item| item.id != "beaker-h2o");

    ensure_default_bench_items(&mut scene);

    let distilled = item(&scene, "beaker-h2o");
    assert_eq!(distilled.kind, "beaker");
    assert_eq!(distilled.location, "bench");
    assert_eq!(distilled.properties.volume_ml, Some(100.0));
    assert_eq!(water_ml(distilled), 100.0);
    let water = item(&scene, "beaker-water");
    assert_eq!(water.label, "Water");
    assert_eq!(water.properties.fill_ml, Some(150.0));
    assert!((water_ml(water) - 150.0).abs() < 1e-9);
    assert_eq!(
        scene
            .items
            .iter()
            .filter(|item| item.id == "beaker-h2o")
            .count(),
        1
    );
}

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

fn aqueous_mol(item: &SceneItem, substance_id: &str) -> f64 {
    item.properties
        .composition
        .iter()
        .find(|c| c.substance_id == substance_id && c.phase == "aqueous")
        .and_then(|c| c.amount_mol)
        .unwrap_or(0.0)
}

fn water_ml(item: &SceneItem) -> f64 {
    crate::solubility::liquid_water_ml(item)
}

fn fill_pipette_from(scene: &mut Scene, target_id: &str) {
    apply_action(
        scene,
        Action::UseTool {
            tool_item_id: "pipette-1".into(),
            target_item_id: target_id.into(),
        },
    )
    .unwrap();
}

#[test]
fn pipette_extracts_one_ml_from_water_scaling_aqueous_and_leaving_sand() {
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

    let water_before = item(&scene, "beaker-water");
    let na_before = aqueous_mol(water_before, "na+");
    let cl_before = aqueous_mol(water_before, "cl-");
    let sand_before = water_before
        .properties
        .composition
        .iter()
        .find(|c| c.substance_id == "sand" && c.phase == "solid")
        .unwrap()
        .amount_g
        .unwrap();
    let liquid_before = water_ml(water_before);
    assert!((liquid_before - 200.0).abs() < 1e-12);

    fill_pipette_from(&mut scene, "beaker-water");

    let water = item(&scene, "beaker-water");
    assert!((water_ml(water) - 199.0).abs() < 1e-9);
    let frac = 199.0 / 200.0;
    assert!((aqueous_mol(water, "na+") - na_before * frac).abs() < 1e-12);
    assert!((aqueous_mol(water, "cl-") - cl_before * frac).abs() < 1e-12);
    let sand_after = water
        .properties
        .composition
        .iter()
        .find(|c| c.substance_id == "sand" && c.phase == "solid")
        .unwrap()
        .amount_g
        .unwrap();
    assert!((sand_after - sand_before).abs() < 1e-12);

    let pipette = item(&scene, "pipette-1");
    assert_eq!(pipette.location, "hand");
    assert_eq!(
        pipette.properties.source_item_id.as_deref(),
        Some("beaker-water")
    );
    assert!((pipette_holding_liquid_ml(pipette) - PIPETTE_VOLUME_ML).abs() < 1e-12);
    assert!((aqueous_mol_holding(pipette, "na+") - na_before / 200.0).abs() < 1e-12);
    assert!(pipette
        .properties
        .holding
        .iter()
        .all(|c| c.phase != "solid"));
}

fn aqueous_mol_holding(item: &SceneItem, substance_id: &str) -> f64 {
    item.properties
        .holding
        .iter()
        .find(|c| c.substance_id == substance_id && c.phase == "aqueous")
        .and_then(|c| c.amount_mol)
        .unwrap_or(0.0)
}

#[test]
fn pipette_fill_from_dish_and_pour_back_to_water_is_consistent() {
    let mut scene = bench_with_water("lab-test");
    fill_pipette_from(&mut scene, "beaker-water");
    apply_action(
        &mut scene,
        Action::Pour {
            source_item_id: "pipette-1".into(),
            target_item_id: "dish-1".into(),
        },
    )
    .unwrap();

    let dish = item(&scene, "dish-1");
    assert!((water_ml(dish) - 1.0).abs() < 1e-9);
    assert_eq!(dish.properties.temperature_c, Some(20.0));

    fill_pipette_from(&mut scene, "dish-1");
    apply_action(
        &mut scene,
        Action::Pour {
            source_item_id: "pipette-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();

    let water = item(&scene, "beaker-water");
    assert!((water_ml(water) - 200.0).abs() < 1e-9);
    assert_eq!(water.properties.temperature_c, Some(20.0));
    let dish = item(&scene, "dish-1");
    assert!(water_ml(dish) < 1e-9);
    assert!(item(&scene, "pipette-1").properties.holding.is_empty());
}

#[test]
fn dish_rejects_over_capacity_and_fill_requires_one_ml() {
    let mut scene = bench_with_water("lab-test");
    let dish = scene.items.iter_mut().find(|i| i.id == "dish-1").unwrap();
    dish.properties.composition.push(CompositionEntry {
        substance_id: "water".into(),
        phase: "liquid".into(),
        amount_ml: Some(24.5),
        amount_scoop: None,
        amount_g: None,
        amount_mol: None,
    });
    dish.properties.fill_ml = Some(24.5);

    fill_pipette_from(&mut scene, "beaker-water");
    let err = apply_action(
        &mut scene,
        Action::Pour {
            source_item_id: "pipette-1".into(),
            target_item_id: "dish-1".into(),
        },
    )
    .unwrap_err();
    assert_eq!(err, SceneError::InvalidAction);
    assert!((pipette_holding_liquid_ml(item(&scene, "pipette-1")) - 1.0).abs() < 1e-12);

    apply_action(
        &mut scene,
        Action::PutAway {
            tool_item_id: "pipette-1".into(),
        },
    )
    .unwrap();

    let dish = scene.items.iter_mut().find(|i| i.id == "dish-1").unwrap();
    dish.properties.composition.clear();
    dish.properties.composition.push(CompositionEntry {
        substance_id: "water".into(),
        phase: "liquid".into(),
        amount_ml: Some(0.5),
        amount_scoop: None,
        amount_g: None,
        amount_mol: None,
    });
    let err = apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "pipette-1".into(),
            target_item_id: "dish-1".into(),
        },
    )
    .unwrap_err();
    assert_eq!(err, SceneError::InvalidAction);
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
fn burner_heats_to_100_then_evaporates_precipitates_and_turns_off() {
    let mut scene = initial_bench_scene("lab-test");
    let dish = scene.items.iter_mut().find(|i| i.id == "dish-1").unwrap();
    dish.properties.temperature_c = Some(20.0);
    dish.properties.composition = vec![
        CompositionEntry {
            substance_id: "water".into(),
            phase: "liquid".into(),
            amount_ml: Some(2.0),
            amount_scoop: None,
            amount_g: None,
            amount_mol: None,
        },
        CompositionEntry {
            substance_id: "na+".into(),
            phase: "aqueous".into(),
            amount_ml: None,
            amount_scoop: None,
            amount_g: None,
            amount_mol: Some(0.02),
        },
        CompositionEntry {
            substance_id: "cl-".into(),
            phase: "aqueous".into(),
            amount_ml: None,
            amount_scoop: None,
            amount_g: None,
            amount_mol: Some(0.02),
        },
    ];
    crate::solubility::enforce_saturation(dish);

    apply_action(
        &mut scene,
        Action::ToggleBurner {
            burner_item_id: "burner-1".into(),
        },
    )
    .unwrap();
    assert_eq!(item(&scene, "burner-1").properties.on, Some(true));

    apply_elapsed(&mut scene, 8.0);
    let dish = item(&scene, "dish-1");
    assert!((dish.properties.temperature_c.unwrap() - 100.0).abs() < 1e-9);
    assert!(
        (water_ml(dish) - 2.0).abs() < 1e-9,
        "no evaporation below 100"
    );

    apply_elapsed(&mut scene, 2.0);
    let dish = item(&scene, "dish-1");
    assert!((water_ml(dish) - 1.0).abs() < 1e-9);
    assert_eq!(dish.properties.temperature_c, Some(100.0));
    assert!(
        dish.properties
            .composition
            .iter()
            .any(|c| c.substance_id == "nacl" && c.phase == "solid"),
        "1 ml cannot hold 0.02 mol NaCl; solid must appear"
    );
    let max_aq =
        crate::solubility::solubility_mol_per_l(crate::solubility::Salt::Nacl, 100.0) * 0.001;
    assert!((aqueous_mol(dish, "na+") - max_aq).abs() < 1e-9);

    apply_elapsed(&mut scene, 2.0);
    let dish = item(&scene, "dish-1");
    assert!(water_ml(dish) < 1e-9);
    assert_eq!(item(&scene, "burner-1").properties.on, Some(false));
    assert!(aqueous_mol(dish, "na+") < 1e-12);
    assert!(dish
        .properties
        .composition
        .iter()
        .any(|c| c.substance_id == "nacl" && c.phase == "solid"));
}

#[test]
fn heating_below_boil_redissolves_nacl_as_solubility_rises() {
    let mut scene = initial_bench_scene("lab-test");
    let dish = scene.items.iter_mut().find(|i| i.id == "dish-1").unwrap();
    dish.properties.temperature_c = Some(20.0);
    dish.properties.composition = vec![
        CompositionEntry {
            substance_id: "water".into(),
            phase: "liquid".into(),
            amount_ml: Some(1.0),
            amount_scoop: None,
            amount_g: None,
            amount_mol: None,
        },
        CompositionEntry {
            substance_id: "na+".into(),
            phase: "aqueous".into(),
            amount_ml: None,
            amount_scoop: None,
            amount_g: None,
            amount_mol: Some(0.02),
        },
        CompositionEntry {
            substance_id: "cl-".into(),
            phase: "aqueous".into(),
            amount_ml: None,
            amount_scoop: None,
            amount_g: None,
            amount_mol: Some(0.02),
        },
    ];
    crate::solubility::enforce_saturation(dish);
    let dish = item(&scene, "dish-1");
    let na_before = aqueous_mol(dish, "na+");
    let solid_before = dish
        .properties
        .composition
        .iter()
        .find(|c| c.substance_id == "nacl" && c.phase == "solid")
        .and_then(|c| c.amount_mol)
        .unwrap_or(0.0);
    assert!(solid_before > 1e-6, "need leftover solid at 20 °C");

    apply_action(
        &mut scene,
        Action::ToggleBurner {
            burner_item_id: "burner-1".into(),
        },
    )
    .unwrap();
    apply_elapsed(&mut scene, 4.0);

    let dish = item(&scene, "dish-1");
    assert!((dish.properties.temperature_c.unwrap() - 60.0).abs() < 1e-9);
    assert!(
        (water_ml(dish) - 1.0).abs() < 1e-9,
        "no evaporation below 100"
    );
    let na_after = aqueous_mol(dish, "na+");
    let solid_after = dish
        .properties
        .composition
        .iter()
        .find(|c| c.substance_id == "nacl" && c.phase == "solid")
        .and_then(|c| c.amount_mol)
        .unwrap_or(0.0);
    assert!(
        na_after > na_before + 1e-6,
        "heating must redissolve NaCl as s(T) rises; {na_after} vs {na_before}"
    );
    assert!(solid_after < solid_before - 1e-6);
    let max_60 =
        crate::solubility::solubility_mol_per_l(crate::solubility::Salt::Nacl, 60.0) * 0.001;
    assert!((na_after - max_60).abs() < 1e-9);
}

#[test]
fn evaporating_mixed_dish_precipitates_nacl_before_independent_caps() {
    let mut scene = initial_bench_scene("lab-test");
    let s_nacl = crate::solubility::solubility_mol_per_l(crate::solubility::Salt::Nacl, 100.0);
    let s_cacl2 = crate::solubility::solubility_mol_per_l(crate::solubility::Salt::Cacl2, 100.0);
    let n_nacl = s_nacl * 0.003;
    let n_cacl2 = s_cacl2 * 0.003;
    let dish = scene.items.iter_mut().find(|i| i.id == "dish-1").unwrap();
    dish.properties.temperature_c = Some(100.0);
    dish.properties.composition = vec![
        CompositionEntry {
            substance_id: "water".into(),
            phase: "liquid".into(),
            amount_ml: Some(15.0),
            amount_scoop: None,
            amount_g: None,
            amount_mol: None,
        },
        CompositionEntry {
            substance_id: "na+".into(),
            phase: "aqueous".into(),
            amount_ml: None,
            amount_scoop: None,
            amount_g: None,
            amount_mol: Some(n_nacl),
        },
        CompositionEntry {
            substance_id: "ca2+".into(),
            phase: "aqueous".into(),
            amount_ml: None,
            amount_scoop: None,
            amount_g: None,
            amount_mol: Some(n_cacl2),
        },
        CompositionEntry {
            substance_id: "cl-".into(),
            phase: "aqueous".into(),
            amount_ml: None,
            amount_scoop: None,
            amount_g: None,
            amount_mol: Some(n_nacl + 2.0 * n_cacl2),
        },
    ];
    crate::solubility::enforce_saturation(dish);
    assert!(!item(&scene, "dish-1")
        .properties
        .composition
        .iter()
        .any(|c| c.substance_id == "nacl" && c.phase == "solid"));

    apply_action(
        &mut scene,
        Action::ToggleBurner {
            burner_item_id: "burner-1".into(),
        },
    )
    .unwrap();
    apply_elapsed(&mut scene, 26.0);
    let dish = item(&scene, "dish-1");
    assert!((water_ml(dish) - 2.0).abs() < 1e-9);
    assert!(
        dish.properties
            .composition
            .iter()
            .any(|c| c.substance_id == "nacl" && c.phase == "solid"),
        "mixed evaporation must crash out NaCl"
    );
    let na = aqueous_mol(dish, "na+");
    let ca = aqueous_mol(dish, "ca2+");
    let ind_nacl = s_nacl * 0.002;
    let ind_cacl2 = s_cacl2 * 0.002;
    assert!(na < ind_nacl - 1e-6);
    assert!((na - ind_nacl).abs() > 1e-6 || (ca - ind_cacl2).abs() > 1e-6);
}

#[test]
fn pipette_in_redissolves_solid_salt_up_to_solubility() {
    let mut scene = bench_with_water("lab-test");
    let dish = scene.items.iter_mut().find(|i| i.id == "dish-1").unwrap();
    dish.properties.composition = vec![CompositionEntry {
        substance_id: "nacl".into(),
        phase: "solid".into(),
        amount_ml: None,
        amount_scoop: None,
        amount_g: Some(0.02 * NACL_MOLAR_MASS_G_PER_MOL),
        amount_mol: Some(0.02),
    }];
    crate::solubility::enforce_saturation(dish);
    assert!(aqueous_mol(item(&scene, "dish-1"), "na+") < 1e-12);

    fill_pipette_from(&mut scene, "beaker-water");
    apply_action(
        &mut scene,
        Action::Pour {
            source_item_id: "pipette-1".into(),
            target_item_id: "dish-1".into(),
        },
    )
    .unwrap();

    let dish = item(&scene, "dish-1");
    let max_aq =
        crate::solubility::solubility_mol_per_l(crate::solubility::Salt::Nacl, 20.0) * 0.001;
    assert!((aqueous_mol(dish, "na+") - max_aq).abs() < 1e-9);
    assert!(
        dish.properties
            .composition
            .iter()
            .any(|c| c.substance_id == "nacl" && c.phase == "solid"),
        "leftover solid remains above 1 ml solubility"
    );
}

#[test]
fn toggle_burner_stays_off_when_dish_has_no_liquid() {
    let mut scene = initial_bench_scene("lab-test");
    apply_action(
        &mut scene,
        Action::ToggleBurner {
            burner_item_id: "burner-1".into(),
        },
    )
    .unwrap();
    assert_eq!(item(&scene, "burner-1").properties.on, Some(false));
    assert!(scene.last_events.is_empty());
}

#[test]
fn pipette_put_away_returns_aliquot_to_last_source() {
    let mut scene = bench_with_water("lab-test");
    fill_pipette_from(&mut scene, "beaker-water");
    apply_action(
        &mut scene,
        Action::PutAway {
            tool_item_id: "pipette-1".into(),
        },
    )
    .unwrap();
    assert!((water_ml(item(&scene, "beaker-water")) - 200.0).abs() < 1e-9);
    let pipette = item(&scene, "pipette-1");
    assert_eq!(pipette.location, "bench");
    assert!(pipette.properties.holding.is_empty());
}

#[test]
fn reset_clears_dish_burner_and_pipette() {
    let mut scene = bench_with_water("lab-test");
    fill_pipette_from(&mut scene, "beaker-water");
    apply_action(
        &mut scene,
        Action::Pour {
            source_item_id: "pipette-1".into(),
            target_item_id: "dish-1".into(),
        },
    )
    .unwrap();
    apply_action(
        &mut scene,
        Action::ToggleBurner {
            burner_item_id: "burner-1".into(),
        },
    )
    .unwrap();
    apply_action(&mut scene, Action::Reset).unwrap();
    assert!(item(&scene, "dish-1").properties.composition.is_empty());
    assert_eq!(
        item(&scene, "dish-1").properties.temperature_c,
        Some(AMBIENT_TEMPERATURE_C)
    );
    assert_eq!(item(&scene, "burner-1").properties.on, Some(false));
    assert!(item(&scene, "pipette-1").properties.holding.is_empty());
    assert_eq!(item(&scene, "pipette-1").location, "bench");
    assert!((water_ml(item(&scene, "beaker-water")) - 0.0).abs() < 1e-9);
}

#[test]
fn use_tool_full_pipette_empties_into_dish() {
    let mut scene = bench_with_water("lab-test");
    fill_pipette_from(&mut scene, "beaker-water");
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "pipette-1".into(),
            target_item_id: "dish-1".into(),
        },
    )
    .unwrap();
    assert!((water_ml(item(&scene, "dish-1")) - 1.0).abs() < 1e-9);
    assert!(item(&scene, "pipette-1").properties.holding.is_empty());
}

fn use_tongs(scene: &mut Scene, target_id: &str) -> Result<(), SceneError> {
    apply_action(
        scene,
        Action::UseTool {
            tool_item_id: "tongs-1".into(),
            target_item_id: target_id.into(),
        },
    )
}

fn put_tongs_away(scene: &mut Scene) -> Result<(), SceneError> {
    apply_action(
        scene,
        Action::PutAway {
            tool_item_id: "tongs-1".into(),
        },
    )
}

fn solid_g(item: &SceneItem, substance_id: &str) -> f64 {
    item.properties
        .composition
        .iter()
        .find(|c| c.substance_id == substance_id && c.phase == "solid")
        .and_then(|c| c.amount_g)
        .unwrap_or(0.0)
}

#[test]
fn tongs_use_without_tongs_item_returns_unknown_item() {
    let mut scene = initial_bench_scene("lab-test");
    scene.items.retain(|item| item.id != "tongs-1");
    assert_eq!(
        use_tongs(&mut scene, "beaker-water").unwrap_err(),
        SceneError::UnknownItem
    );
}

#[test]
fn ensure_default_bench_items_is_idempotent_when_catalog_is_complete() {
    let mut scene = initial_bench_scene("lab-test");
    let before = scene.items.len();
    ensure_default_bench_items(&mut scene);
    ensure_default_bench_items(&mut scene);
    assert_eq!(scene.items.len(), before);
    assert_eq!(
        scene
            .items
            .iter()
            .filter(|item| item.id == "tongs-1")
            .count(),
        1
    );
}

#[test]
fn ensure_default_bench_items_restores_tongs_without_resetting_vessels() {
    let mut scene = initial_bench_scene("lab-test");
    scene
        .items
        .iter_mut()
        .find(|item| item.id == "beaker-water")
        .unwrap()
        .properties
        .fill_ml = Some(150.0);
    scene.items.retain(|item| item.id != "tongs-1");

    ensure_default_bench_items(&mut scene);

    let tongs = item(&scene, "tongs-1");
    assert_eq!(tongs.kind, "tongs");
    assert_eq!(tongs.location, "bench");
    assert_eq!(tongs.properties.source_item_id, None);
    assert_eq!(item(&scene, "beaker-water").properties.fill_ml, Some(150.0));

    use_tongs(&mut scene, "beaker-water").unwrap();
    assert_eq!(item(&scene, "beaker-water").location, "held");
    assert_eq!(
        item(&scene, "tongs-1").properties.source_item_id.as_deref(),
        Some("beaker-water")
    );
}

#[test]
fn tongs_pick_up_water_sets_held_location_and_records_source() {
    let mut scene = initial_bench_scene("lab-test");
    use_tongs(&mut scene, "beaker-water").unwrap();

    let water = item(&scene, "beaker-water");
    assert_eq!(water.location, "held");
    let tongs = item(&scene, "tongs-1");
    assert_eq!(tongs.location, "hand");
    assert_eq!(
        tongs.properties.source_item_id.as_deref(),
        Some("beaker-water")
    );
    assert_eq!(item(&scene, "dish-1").location, "bench");
}

#[test]
fn tongs_pick_up_dish_turns_burner_off() {
    let mut scene = bench_with_water("lab-test");
    fill_pipette_from(&mut scene, "beaker-water");
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "pipette-1".into(),
            target_item_id: "dish-1".into(),
        },
    )
    .unwrap();
    apply_action(
        &mut scene,
        Action::ToggleBurner {
            burner_item_id: "burner-1".into(),
        },
    )
    .unwrap();
    assert_eq!(item(&scene, "burner-1").properties.on, Some(true));

    use_tongs(&mut scene, "dish-1").unwrap();

    assert_eq!(item(&scene, "dish-1").location, "held");
    assert_eq!(item(&scene, "burner-1").properties.on, Some(false));
    assert_eq!(
        item(&scene, "tongs-1").properties.source_item_id.as_deref(),
        Some("dish-1")
    );
}

#[test]
fn tongs_pour_water_into_dish_fills_to_capacity_and_scales_ions_and_solids() {
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

    let water_before = item(&scene, "beaker-water");
    let source_ml = water_ml(water_before);
    let na_before = aqueous_mol(water_before, "na+");
    let cl_before = aqueous_mol(water_before, "cl-");
    let sand_before = solid_g(water_before, "sand");
    let frac = DISH_CAPACITY_ML / source_ml;

    use_tongs(&mut scene, "beaker-water").unwrap();
    use_tongs(&mut scene, "dish-1").unwrap();

    let water = item(&scene, "beaker-water");
    let dish = item(&scene, "dish-1");
    assert_eq!(water.location, "held");
    assert!((water_ml(dish) - DISH_CAPACITY_ML).abs() < 1e-9);
    assert!((water_ml(water) - (source_ml - DISH_CAPACITY_ML)).abs() < 1e-9);
    assert!((aqueous_mol(dish, "na+") - na_before * frac).abs() < 1e-12);
    assert!((aqueous_mol(dish, "cl-") - cl_before * frac).abs() < 1e-12);
    assert!((solid_g(dish, "sand") - sand_before * frac).abs() < 1e-12);
    assert!((aqueous_mol(water, "na+") - na_before * (1.0 - frac)).abs() < 1e-12);
    assert!((solid_g(water, "sand") - sand_before * (1.0 - frac)).abs() < 1e-12);
    assert!(scene.last_events.iter().any(|e| e.kind == "poured"));
    assert_eq!(
        item(&scene, "tongs-1").properties.source_item_id.as_deref(),
        Some("beaker-water")
    );
}

#[test]
fn tongs_pour_dish_into_water_moves_contents_and_keeps_holding() {
    let mut scene = bench_with_water("lab-test");
    fill_pipette_from(&mut scene, "beaker-water");
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "pipette-1".into(),
            target_item_id: "dish-1".into(),
        },
    )
    .unwrap();
    assert!((water_ml(item(&scene, "dish-1")) - 1.0).abs() < 1e-9);

    use_tongs(&mut scene, "dish-1").unwrap();
    use_tongs(&mut scene, "beaker-water").unwrap();

    assert!((water_ml(item(&scene, "dish-1")) - 0.0).abs() < 1e-9);
    assert!((water_ml(item(&scene, "beaker-water")) - 200.0).abs() < 1e-9);
    assert_eq!(item(&scene, "dish-1").location, "held");
    assert_eq!(
        item(&scene, "tongs-1").properties.source_item_id.as_deref(),
        Some("dish-1")
    );
}

#[test]
fn tongs_pour_stops_when_destination_is_full() {
    let mut scene = bench_with_water("lab-test");
    use_tongs(&mut scene, "beaker-water").unwrap();
    use_tongs(&mut scene, "dish-1").unwrap();
    assert!((water_ml(item(&scene, "dish-1")) - DISH_CAPACITY_ML).abs() < 1e-9);

    let err = use_tongs(&mut scene, "dish-1").unwrap_err();
    assert_eq!(err, SceneError::InvalidAction);
    assert_eq!(item(&scene, "beaker-water").location, "held");
    assert!((water_ml(item(&scene, "dish-1")) - DISH_CAPACITY_ML).abs() < 1e-9);
    assert!((water_ml(item(&scene, "beaker-water")) - 175.0).abs() < 1e-9);
}

#[test]
fn tongs_solids_only_dump_moves_all_solids_even_into_full_destination() {
    let mut scene = initial_bench_scene("lab-test");
    let water = scene
        .items
        .iter_mut()
        .find(|i| i.id == "beaker-water")
        .unwrap();
    water.properties.composition = vec![CompositionEntry {
        substance_id: "water".into(),
        phase: "liquid".into(),
        amount_ml: Some(250.0),
        amount_scoop: None,
        amount_g: None,
        amount_mol: None,
    }];
    crate::solubility::sync_fill_ml(water);

    let dish = scene.items.iter_mut().find(|i| i.id == "dish-1").unwrap();
    dish.properties.composition = vec![CompositionEntry {
        substance_id: "sand".into(),
        phase: "solid".into(),
        amount_ml: None,
        amount_scoop: Some(2),
        amount_g: Some(2.0 * SPOON_SCOOP_MASS_G),
        amount_mol: None,
    }];
    dish.properties.fill_ml = Some(0.0);

    use_tongs(&mut scene, "dish-1").unwrap();
    use_tongs(&mut scene, "beaker-water").unwrap();

    assert!((water_ml(item(&scene, "beaker-water")) - 250.0).abs() < 1e-9);
    assert!(
        (solid_g(item(&scene, "beaker-water"), "sand") - 2.0 * SPOON_SCOOP_MASS_G).abs() < 1e-12
    );
    assert_eq!(solid_g(item(&scene, "dish-1"), "sand"), 0.0);
    assert!(item(&scene, "dish-1")
        .properties
        .composition
        .iter()
        .all(|c| c.phase != "solid"));
    assert_eq!(item(&scene, "dish-1").location, "held");
}

#[test]
fn tongs_full_dest_dumps_all_solids_from_wet_source_and_keeps_liquid() {
    let mut scene = initial_bench_scene("lab-test");
    let water = scene
        .items
        .iter_mut()
        .find(|i| i.id == "beaker-water")
        .unwrap();
    water.properties.composition = vec![CompositionEntry {
        substance_id: "water".into(),
        phase: "liquid".into(),
        amount_ml: Some(250.0),
        amount_scoop: None,
        amount_g: None,
        amount_mol: None,
    }];
    crate::solubility::sync_fill_ml(water);

    let dish = scene.items.iter_mut().find(|i| i.id == "dish-1").unwrap();
    dish.properties.composition = vec![
        CompositionEntry {
            substance_id: "water".into(),
            phase: "liquid".into(),
            amount_ml: Some(10.0),
            amount_scoop: None,
            amount_g: None,
            amount_mol: None,
        },
        CompositionEntry {
            substance_id: "sand".into(),
            phase: "solid".into(),
            amount_ml: None,
            amount_scoop: Some(2),
            amount_g: Some(2.0 * SPOON_SCOOP_MASS_G),
            amount_mol: None,
        },
    ];
    crate::solubility::sync_fill_ml(dish);

    use_tongs(&mut scene, "dish-1").unwrap();
    use_tongs(&mut scene, "beaker-water").unwrap();

    assert!((water_ml(item(&scene, "beaker-water")) - 250.0).abs() < 1e-9);
    assert!((water_ml(item(&scene, "dish-1")) - 10.0).abs() < 1e-9);
    assert!(
        (solid_g(item(&scene, "beaker-water"), "sand") - 2.0 * SPOON_SCOOP_MASS_G).abs() < 1e-12
    );
    assert_eq!(solid_g(item(&scene, "dish-1"), "sand"), 0.0);
    assert_eq!(item(&scene, "dish-1").location, "held");
    assert_eq!(
        item(&scene, "tongs-1").properties.source_item_id.as_deref(),
        Some("dish-1")
    );
}

#[test]
fn tongs_put_away_returns_water_and_dish_home() {
    let mut scene = initial_bench_scene("lab-test");
    use_tongs(&mut scene, "beaker-water").unwrap();
    put_tongs_away(&mut scene).unwrap();
    assert_eq!(item(&scene, "beaker-water").location, "bench");
    assert_eq!(item(&scene, "tongs-1").location, "bench");
    assert_eq!(item(&scene, "tongs-1").properties.source_item_id, None);

    use_tongs(&mut scene, "dish-1").unwrap();
    put_tongs_away(&mut scene).unwrap();
    assert_eq!(item(&scene, "dish-1").location, "bench");
    assert_eq!(item(&scene, "tongs-1").location, "bench");
    assert_eq!(item(&scene, "tongs-1").properties.source_item_id, None);
}

#[test]
fn tongs_pick_up_and_put_away_each_solid_stock() {
    for stock_id in ["beaker-nacl", "beaker-cacl2", "beaker-sand"] {
        let mut scene = initial_bench_scene("lab-test");
        use_tongs(&mut scene, stock_id).unwrap();
        assert_eq!(item(&scene, stock_id).location, "held");
        assert_eq!(item(&scene, "tongs-1").location, "hand");
        assert_eq!(
            item(&scene, "tongs-1").properties.source_item_id.as_deref(),
            Some(stock_id)
        );

        put_tongs_away(&mut scene).unwrap();
        assert_eq!(item(&scene, stock_id).location, "bench");
        assert_eq!(item(&scene, "tongs-1").location, "bench");
        assert_eq!(item(&scene, "tongs-1").properties.source_item_id, None);
    }
}

#[test]
fn tongs_dump_all_nacl_into_empty_beaker() {
    let mut scene = initial_bench_scene("lab-test");
    let stock_g = solid_g(item(&scene, "beaker-nacl"), "nacl");
    assert!(stock_g > 0.0);

    use_tongs(&mut scene, "beaker-nacl").unwrap();
    use_tongs(&mut scene, "beaker-water").unwrap();

    assert_eq!(solid_g(item(&scene, "beaker-nacl"), "nacl"), 0.0);
    assert!((solid_g(item(&scene, "beaker-water"), "nacl") - stock_g).abs() < 1e-12);
    assert_eq!(aqueous_mol(item(&scene, "beaker-water"), "na+"), 0.0);
    assert_eq!(item(&scene, "beaker-nacl").location, "held");
    assert_eq!(
        item(&scene, "tongs-1").properties.source_item_id.as_deref(),
        Some("beaker-nacl")
    );
    assert!(scene.last_events.iter().any(|e| e.kind == "poured"));
    assert!(!scene.last_events.iter().any(|e| e.kind == "dissolved"));
}

#[test]
fn tongs_dump_nacl_into_solids_only_beaker() {
    let mut scene = initial_bench_scene("lab-test");
    use_tongs(&mut scene, "beaker-sand").unwrap();
    use_tongs(&mut scene, "beaker-water").unwrap();
    put_tongs_away(&mut scene).unwrap();

    let sand_g = solid_g(item(&scene, "beaker-water"), "sand");
    let nacl_g = solid_g(item(&scene, "beaker-nacl"), "nacl");
    use_tongs(&mut scene, "beaker-nacl").unwrap();
    use_tongs(&mut scene, "beaker-water").unwrap();

    assert!((solid_g(item(&scene, "beaker-water"), "sand") - sand_g).abs() < 1e-12);
    assert!((solid_g(item(&scene, "beaker-water"), "nacl") - nacl_g).abs() < 1e-12);
    assert_eq!(aqueous_mol(item(&scene, "beaker-water"), "na+"), 0.0);
    assert_eq!(water_ml(item(&scene, "beaker-water")), 0.0);
    assert_eq!(item(&scene, "beaker-nacl").location, "held");
}

#[test]
fn tongs_dump_nacl_into_filled_beaker_enforces_saturation() {
    let mut scene = bench_with_water("lab-test");
    let nacl_g = solid_g(item(&scene, "beaker-nacl"), "nacl");

    use_tongs(&mut scene, "beaker-nacl").unwrap();
    use_tongs(&mut scene, "beaker-water").unwrap();

    let expected = nacl_g / NACL_MOLAR_MASS_G_PER_MOL;
    assert!((aqueous_mol(item(&scene, "beaker-water"), "na+") - expected).abs() < 1e-9);
    assert!((aqueous_mol(item(&scene, "beaker-water"), "cl-") - expected).abs() < 1e-9);
    assert_eq!(solid_g(item(&scene, "beaker-water"), "nacl"), 0.0);
    assert_eq!(solid_g(item(&scene, "beaker-nacl"), "nacl"), 0.0);
    assert_eq!(item(&scene, "beaker-nacl").location, "held");
}

#[test]
fn tongs_empty_stock_pour_is_empty_holding() {
    let mut scene = initial_bench_scene("lab-test");
    use_tongs(&mut scene, "beaker-nacl").unwrap();
    use_tongs(&mut scene, "beaker-water").unwrap();
    let err = use_tongs(&mut scene, "beaker-water").unwrap_err();
    assert_eq!(err, SceneError::EmptyHolding);
    assert_eq!(item(&scene, "beaker-nacl").location, "held");
    assert_eq!(
        item(&scene, "tongs-1").properties.source_item_id.as_deref(),
        Some("beaker-nacl")
    );
}

#[test]
fn tongs_reject_held_solid_stock_into_h2o_wrong_stock_and_burner() {
    let mut scene = initial_bench_scene("lab-test");
    use_tongs(&mut scene, "beaker-nacl").unwrap();
    let stock_g = solid_g(item(&scene, "beaker-nacl"), "nacl");
    for dest in ["beaker-h2o", "beaker-cacl2", "burner-1"] {
        assert_eq!(
            use_tongs(&mut scene, dest).unwrap_err(),
            SceneError::InvalidAction
        );
    }
    assert_eq!(item(&scene, "beaker-nacl").location, "held");
    assert!((solid_g(item(&scene, "beaker-nacl"), "nacl") - stock_g).abs() < 1e-12);
}

#[test]
fn tongs_dump_exact_type_solids_into_matching_stock() {
    let mut scene = initial_bench_scene("lab-test");
    use_tongs(&mut scene, "beaker-nacl").unwrap();
    use_tongs(&mut scene, "dish-1").unwrap();
    put_tongs_away(&mut scene).unwrap();
    assert_eq!(solid_g(item(&scene, "beaker-nacl"), "nacl"), 0.0);
    assert!((solid_g(item(&scene, "dish-1"), "nacl") - 2.0).abs() < 1e-12);

    use_tongs(&mut scene, "dish-1").unwrap();
    use_tongs(&mut scene, "beaker-nacl").unwrap();
    assert_eq!(solid_g(item(&scene, "dish-1"), "nacl"), 0.0);
    assert!((solid_g(item(&scene, "beaker-nacl"), "nacl") - 2.0).abs() < 1e-12);
    assert_eq!(item(&scene, "dish-1").location, "held");
}

#[test]
fn tongs_dump_held_solid_stock_into_dish_paper_and_filtrate() {
    let mut scene = initial_bench_scene("lab-test");
    use_tongs(&mut scene, "beaker-nacl").unwrap();
    use_tongs(&mut scene, "dish-1").unwrap();
    assert!((solid_g(item(&scene, "dish-1"), "nacl") - 2.0).abs() < 1e-12);
    assert_eq!(solid_g(item(&scene, "beaker-nacl"), "nacl"), 0.0);
    assert_eq!(item(&scene, "beaker-nacl").location, "held");
    put_tongs_away(&mut scene).unwrap();

    use_tongs(&mut scene, "beaker-sand").unwrap();
    use_tongs(&mut scene, "filter-paper-1").unwrap();
    assert!((solid_g(item(&scene, "filter-paper-1"), "sand") - 2.0).abs() < 1e-12);
    assert_eq!(solid_g(item(&scene, "beaker-sand"), "sand"), 0.0);
    put_tongs_away(&mut scene).unwrap();

    use_tongs(&mut scene, "beaker-cacl2").unwrap();
    use_tongs(&mut scene, "beaker-filtrate").unwrap();
    assert!((solid_g(item(&scene, "beaker-filtrate"), "cacl2") - 2.0).abs() < 1e-12);
    assert_eq!(solid_g(item(&scene, "beaker-cacl2"), "cacl2"), 0.0);
}

#[test]
fn tongs_reject_pouring_into_sitting_solid_stock() {
    let mut scene = bench_with_water("lab-test");
    use_tongs(&mut scene, "beaker-water").unwrap();
    assert_eq!(
        use_tongs(&mut scene, "beaker-nacl").unwrap_err(),
        SceneError::InvalidAction
    );
    assert_eq!(item(&scene, "beaker-water").location, "held");
    assert!(solid_g(item(&scene, "beaker-nacl"), "nacl") > 0.0);
}

#[test]
fn tongs_reject_burner_and_already_held_vessel() {
    let mut scene = initial_bench_scene("lab-test");
    assert_eq!(
        use_tongs(&mut scene, "burner-1").unwrap_err(),
        SceneError::InvalidAction
    );

    use_tongs(&mut scene, "beaker-water").unwrap();
    assert_eq!(
        use_tongs(&mut scene, "beaker-water").unwrap_err(),
        SceneError::InvalidAction
    );
    assert_eq!(item(&scene, "beaker-water").location, "held");
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
fn tongs_empty_source_with_nothing_to_move_errors_and_stays_holding() {
    let mut scene = initial_bench_scene("lab-test");
    use_tongs(&mut scene, "dish-1").unwrap();
    let err = use_tongs(&mut scene, "beaker-water").unwrap_err();
    assert_eq!(err, SceneError::EmptyHolding);
    assert_eq!(item(&scene, "dish-1").location, "held");
    assert_eq!(
        item(&scene, "tongs-1").properties.source_item_id.as_deref(),
        Some("dish-1")
    );
}

#[test]
fn tongs_pour_uses_volume_weighted_destination_temperature() {
    let mut scene = bench_with_water("lab-test");
    let dish = scene.items.iter_mut().find(|i| i.id == "dish-1").unwrap();
    dish.properties.temperature_c = Some(80.0);
    dish.properties.composition = vec![CompositionEntry {
        substance_id: "water".into(),
        phase: "liquid".into(),
        amount_ml: Some(10.0),
        amount_scoop: None,
        amount_g: None,
        amount_mol: None,
    }];
    crate::solubility::sync_fill_ml(dish);

    use_tongs(&mut scene, "beaker-water").unwrap();
    use_tongs(&mut scene, "dish-1").unwrap();

    let dish = item(&scene, "dish-1");
    assert!((water_ml(dish) - 25.0).abs() < 1e-9);
    let expected_t = (10.0 * 80.0 + 15.0 * 20.0) / 25.0;
    assert!((dish.properties.temperature_c.unwrap() - expected_t).abs() < 1e-9);
    assert_eq!(
        item(&scene, "beaker-water").properties.temperature_c,
        Some(20.0)
    );
}

fn solid(substance_id: &str, grams: f64) -> CompositionEntry {
    CompositionEntry {
        substance_id: substance_id.into(),
        phase: "solid".into(),
        amount_ml: None,
        amount_scoop: None,
        amount_g: Some(grams),
        amount_mol: None,
    }
}

fn set_dry_dish_solids(scene: &mut Scene, solids: Vec<CompositionEntry>) {
    let dish = scene.items.iter_mut().find(|i| i.id == "dish-1").unwrap();
    dish.properties.composition = solids;
    crate::solubility::sync_fill_ml(dish);
}

fn scoop_dish(scene: &mut Scene) -> Result<(), SceneError> {
    apply_action(
        scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "dish-1".into(),
        },
    )
}

fn empty_nacl_stock(scene: &mut Scene) {
    let nacl = scene
        .items
        .iter_mut()
        .find(|i| i.id == "beaker-nacl")
        .unwrap();
    let stock = nacl
        .properties
        .composition
        .iter_mut()
        .find(|c| c.substance_id == "nacl" && c.phase == "solid")
        .unwrap();
    stock.amount_scoop = Some(0);
    stock.amount_g = Some(0.0);
    stock.amount_mol = None;
}

fn scoop_nacl_stock(scene: &mut Scene) -> Result<(), SceneError> {
    apply_action(
        scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-nacl".into(),
        },
    )
}

fn holding_g(item: &SceneItem, substance_id: &str) -> f64 {
    item.properties
        .holding
        .iter()
        .find(|c| c.substance_id == substance_id && c.phase == "solid")
        .and_then(|c| c.amount_g)
        .unwrap_or(0.0)
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
fn tongs_pick_up_beaker_h2o_and_pour_into_dish_and_water() {
    let mut scene = initial_bench_scene("lab-test");
    use_tongs(&mut scene, "beaker-h2o").unwrap();
    assert_eq!(item(&scene, "beaker-h2o").location, "held");
    assert_eq!(
        item(&scene, "tongs-1").properties.source_item_id.as_deref(),
        Some("beaker-h2o")
    );

    use_tongs(&mut scene, "dish-1").unwrap();
    assert_eq!(item(&scene, "beaker-h2o").location, "held");
    assert!((water_ml(item(&scene, "dish-1")) - DISH_CAPACITY_ML).abs() < 1e-9);
    assert!((water_ml(item(&scene, "beaker-h2o")) - 75.0).abs() < 1e-9);

    use_tongs(&mut scene, "beaker-water").unwrap();
    assert!((water_ml(item(&scene, "beaker-water")) - 75.0).abs() < 1e-9);
    assert!((water_ml(item(&scene, "beaker-h2o")) - 0.0).abs() < 1e-9);
}

#[test]
fn tongs_put_away_returns_beaker_h2o_home() {
    let mut scene = initial_bench_scene("lab-test");
    use_tongs(&mut scene, "beaker-h2o").unwrap();
    put_tongs_away(&mut scene).unwrap();
    assert_eq!(item(&scene, "beaker-h2o").location, "bench");
    assert_eq!(item(&scene, "tongs-1").location, "bench");
    assert_eq!(item(&scene, "tongs-1").properties.source_item_id, None);
}

#[test]
fn pipette_extracts_one_ml_from_beaker_h2o_and_dumps_into_water() {
    let mut scene = initial_bench_scene("lab-test");
    fill_pipette_from(&mut scene, "beaker-h2o");
    assert!((water_ml(item(&scene, "beaker-h2o")) - 99.0).abs() < 1e-9);
    let pipette = item(&scene, "pipette-1");
    assert!((pipette_holding_liquid_ml(pipette) - PIPETTE_VOLUME_ML).abs() < 1e-12);
    assert!(pipette
        .properties
        .holding
        .iter()
        .all(|c| { c.substance_id == "water" && c.phase == "liquid" }));
    assert_eq!(
        pipette.properties.source_item_id.as_deref(),
        Some("beaker-h2o")
    );

    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "pipette-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();
    assert!((water_ml(item(&scene, "beaker-water")) - 1.0).abs() < 1e-9);
    assert!(item(&scene, "pipette-1").properties.holding.is_empty());
}

#[test]
fn pipette_put_back_pure_water_into_beaker_h2o_up_to_capacity() {
    let mut scene = initial_bench_scene("lab-test");
    fill_pipette_from(&mut scene, "beaker-h2o");
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "pipette-1".into(),
            target_item_id: "beaker-h2o".into(),
        },
    )
    .unwrap();
    assert!((water_ml(item(&scene, "beaker-h2o")) - 100.0).abs() < 1e-9);
    assert!(item(&scene, "pipette-1").properties.holding.is_empty());
}

#[test]
fn tongs_pour_pure_water_into_beaker_h2o_stops_at_capacity() {
    let mut scene = bench_with_water("lab-test");
    fill_pipette_from(&mut scene, "beaker-h2o");
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "pipette-1".into(),
            target_item_id: "dish-1".into(),
        },
    )
    .unwrap();
    assert!((water_ml(item(&scene, "beaker-h2o")) - 99.0).abs() < 1e-9);

    use_tongs(&mut scene, "beaker-water").unwrap();
    use_tongs(&mut scene, "beaker-h2o").unwrap();

    assert!((water_ml(item(&scene, "beaker-h2o")) - 100.0).abs() < 1e-9);
    assert!((water_ml(item(&scene, "beaker-water")) - 199.0).abs() < 1e-9);
    assert_eq!(item(&scene, "beaker-water").location, "held");
}

#[test]
fn pipette_and_tongs_reject_put_back_when_beaker_h2o_is_full() {
    let mut scene = bench_with_water("lab-test");
    fill_pipette_from(&mut scene, "beaker-water");
    let err = apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "pipette-1".into(),
            target_item_id: "beaker-h2o".into(),
        },
    )
    .unwrap_err();
    assert_eq!(err, SceneError::InvalidAction);
    assert!((pipette_holding_liquid_ml(item(&scene, "pipette-1")) - 1.0).abs() < 1e-9);
    assert!((water_ml(item(&scene, "beaker-h2o")) - 100.0).abs() < 1e-9);

    apply_action(
        &mut scene,
        Action::PutAway {
            tool_item_id: "pipette-1".into(),
        },
    )
    .unwrap();

    use_tongs(&mut scene, "beaker-water").unwrap();
    let before_water = water_ml(item(&scene, "beaker-water"));
    let err = use_tongs(&mut scene, "beaker-h2o").unwrap_err();
    assert_eq!(err, SceneError::InvalidAction);
    assert!((water_ml(item(&scene, "beaker-h2o")) - 100.0).abs() < 1e-9);
    assert!((water_ml(item(&scene, "beaker-water")) - before_water).abs() < 1e-9);
    assert_eq!(item(&scene, "beaker-water").location, "held");
}

#[test]
fn pipette_and_tongs_reject_impure_put_back_into_beaker_h2o() {
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
    fill_pipette_from(&mut scene, "beaker-h2o");
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "pipette-1".into(),
            target_item_id: "dish-1".into(),
        },
    )
    .unwrap();

    fill_pipette_from(&mut scene, "beaker-water");
    let err = apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "pipette-1".into(),
            target_item_id: "beaker-h2o".into(),
        },
    )
    .unwrap_err();
    assert_eq!(err, SceneError::InvalidAction);
    assert!(pipette_holding_liquid_ml(item(&scene, "pipette-1")) > AMOUNT_EPS);
    assert!((water_ml(item(&scene, "beaker-h2o")) - 99.0).abs() < 1e-9);

    apply_action(
        &mut scene,
        Action::PutAway {
            tool_item_id: "pipette-1".into(),
        },
    )
    .unwrap();

    use_tongs(&mut scene, "beaker-water").unwrap();
    let err = use_tongs(&mut scene, "beaker-h2o").unwrap_err();
    assert_eq!(err, SceneError::InvalidAction);
    assert!((water_ml(item(&scene, "beaker-h2o")) - 99.0).abs() < 1e-9);
    assert_eq!(item(&scene, "beaker-water").location, "held");
    assert!(aqueous_mol(item(&scene, "beaker-water"), "na+") > AMOUNT_EPS);
}

#[test]
fn tongs_reject_solids_only_dump_into_beaker_h2o() {
    let mut scene = initial_bench_scene("lab-test");
    let water = scene
        .items
        .iter_mut()
        .find(|i| i.id == "beaker-water")
        .unwrap();
    water.properties.composition = vec![solid("sand", 0.4)];
    crate::solubility::sync_fill_ml(water);

    use_tongs(&mut scene, "beaker-water").unwrap();
    let err = use_tongs(&mut scene, "beaker-h2o").unwrap_err();
    assert_eq!(err, SceneError::InvalidAction);
    assert!((solid_g(item(&scene, "beaker-water"), "sand") - 0.4).abs() < 1e-12);
    assert_eq!(solid_g(item(&scene, "beaker-h2o"), "sand"), 0.0);
    assert_eq!(item(&scene, "beaker-water").location, "held");
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

fn set_paper_solids(scene: &mut Scene, solids: Vec<CompositionEntry>) {
    let paper = scene
        .items
        .iter_mut()
        .find(|i| i.id == "filter-paper-1")
        .unwrap();
    paper.properties.composition = solids;
}

fn scoop_paper(scene: &mut Scene) -> Result<(), SceneError> {
    apply_action(
        scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "filter-paper-1".into(),
        },
    )
}

fn set_slurry_in_water(scene: &mut Scene) {
    fill_main_beaker(scene, FILLED_MAIN_BEAKER_ML);
    apply_action(
        scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-nacl".into(),
        },
    )
    .unwrap();
    apply_action(
        scene,
        Action::Pour {
            source_item_id: "spoon-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();
    apply_action(
        scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-sand".into(),
        },
    )
    .unwrap();
    apply_action(
        scene,
        Action::Pour {
            source_item_id: "spoon-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();
}

#[test]
fn initial_bench_scene_spawns_empty_filtrate_beaker_and_filter_paper() {
    let scene = initial_bench_scene("lab-test");
    assert!(scene.items.iter().any(|i| i.id == "beaker-filtrate"));
    assert!(scene.items.iter().any(|i| i.id == "filter-paper-1"));

    let filtrate = item(&scene, "beaker-filtrate");
    assert_eq!(filtrate.kind, "beaker");
    assert_eq!(filtrate.label, "Filtrate");
    assert_eq!(filtrate.location, "bench");
    assert_eq!(filtrate.properties.volume_ml, Some(250.0));
    assert_eq!(filtrate.properties.fill_ml, Some(0.0));
    assert_eq!(filtrate.properties.transparent, Some(true));
    assert_eq!(filtrate.properties.colourless, Some(true));
    assert_eq!(
        filtrate.properties.temperature_c,
        Some(AMBIENT_TEMPERATURE_C)
    );
    assert!(filtrate.properties.composition.is_empty());

    let paper = item(&scene, "filter-paper-1");
    assert_eq!(paper.kind, "filter_paper");
    assert_eq!(paper.label, "Filter paper");
    assert_eq!(paper.location, "bench");
    assert!(paper.properties.composition.is_empty());
}

#[test]
fn ensure_default_bench_items_restores_filtration_catalog_without_resetting_vessels() {
    let mut scene = initial_bench_scene("lab-test");
    scene
        .items
        .iter_mut()
        .find(|item| item.id == "beaker-water")
        .unwrap()
        .properties
        .fill_ml = Some(150.0);
    scene
        .items
        .retain(|item| item.id != "beaker-filtrate" && item.id != "filter-paper-1");

    ensure_default_bench_items(&mut scene);

    let filtrate = item(&scene, "beaker-filtrate");
    assert_eq!(filtrate.kind, "beaker");
    assert_eq!(filtrate.location, "bench");
    assert_eq!(filtrate.properties.volume_ml, Some(250.0));
    assert_eq!(filtrate.properties.fill_ml, Some(0.0));
    let paper = item(&scene, "filter-paper-1");
    assert_eq!(paper.kind, "filter_paper");
    assert!(paper.properties.composition.is_empty());
    assert_eq!(item(&scene, "beaker-water").properties.fill_ml, Some(150.0));
    assert_eq!(
        scene
            .items
            .iter()
            .filter(|item| item.id == "beaker-filtrate")
            .count(),
        1
    );
    assert_eq!(
        scene
            .items
            .iter()
            .filter(|item| item.id == "filter-paper-1")
            .count(),
        1
    );
}

#[test]
fn tongs_empty_click_paper_and_filtrate_pick_each_separately() {
    let mut scene = initial_bench_scene("lab-test");
    use_tongs(&mut scene, "filter-paper-1").unwrap();
    assert_eq!(item(&scene, "filter-paper-1").location, "held");
    assert_eq!(item(&scene, "beaker-filtrate").location, "bench");
    assert_eq!(
        item(&scene, "tongs-1").properties.source_item_id.as_deref(),
        Some("filter-paper-1")
    );
    put_tongs_away(&mut scene).unwrap();
    assert_eq!(item(&scene, "filter-paper-1").location, "bench");
    assert_eq!(item(&scene, "tongs-1").location, "bench");
    assert_eq!(item(&scene, "tongs-1").properties.source_item_id, None);

    use_tongs(&mut scene, "beaker-filtrate").unwrap();
    assert_eq!(item(&scene, "beaker-filtrate").location, "held");
    assert_eq!(item(&scene, "filter-paper-1").location, "bench");
    put_tongs_away(&mut scene).unwrap();
    assert_eq!(item(&scene, "beaker-filtrate").location, "bench");
}

#[test]
fn tongs_held_filtrate_onto_paper_is_invalid() {
    let mut scene = initial_bench_scene("lab-test");
    use_tongs(&mut scene, "beaker-filtrate").unwrap();
    assert_eq!(
        use_tongs(&mut scene, "filter-paper-1").unwrap_err(),
        SceneError::InvalidAction
    );
    assert_eq!(item(&scene, "beaker-filtrate").location, "held");
    assert_eq!(item(&scene, "filter-paper-1").location, "bench");
}

#[test]
fn filter_pour_splits_slurry_solids_on_paper_ions_in_filtrate() {
    let mut scene = initial_bench_scene("lab-test");
    set_slurry_in_water(&mut scene);
    let water_before = item(&scene, "beaker-water");
    let source_ml = water_ml(water_before);
    let na_before = aqueous_mol(water_before, "na+");
    let cl_before = aqueous_mol(water_before, "cl-");
    let sand_before = solid_g(water_before, "sand");

    use_tongs(&mut scene, "beaker-water").unwrap();
    use_tongs(&mut scene, "filter-paper-1").unwrap();

    let water = item(&scene, "beaker-water");
    let filtrate = item(&scene, "beaker-filtrate");
    let paper = item(&scene, "filter-paper-1");
    assert_eq!(water.location, "held");
    assert!((water_ml(filtrate) - source_ml).abs() < 1e-9);
    assert!((water_ml(water) - 0.0).abs() < 1e-9);
    assert!((aqueous_mol(filtrate, "na+") - na_before).abs() < 1e-12);
    assert!((aqueous_mol(filtrate, "cl-") - cl_before).abs() < 1e-12);
    assert_eq!(solid_g(filtrate, "sand"), 0.0);
    assert!((solid_g(paper, "sand") - sand_before).abs() < 1e-12);
    assert_eq!(solid_g(water, "sand"), 0.0);
    assert_eq!(aqueous_mol(paper, "na+"), 0.0);
    assert!(scene.last_events.iter().any(|e| e.kind == "poured"));
    assert_eq!(
        item(&scene, "tongs-1").properties.source_item_id.as_deref(),
        Some("beaker-water")
    );
}

#[test]
fn filter_pour_caps_at_250_ml_and_moves_proportional_solids_to_paper() {
    let mut scene = initial_bench_scene("lab-test");
    set_slurry_in_water(&mut scene);
    let dest = scene
        .items
        .iter_mut()
        .find(|i| i.id == "beaker-filtrate")
        .unwrap();
    dest.properties.composition = vec![CompositionEntry {
        substance_id: "water".into(),
        phase: "liquid".into(),
        amount_ml: Some(100.0),
        amount_scoop: None,
        amount_g: None,
        amount_mol: None,
    }];
    dest.properties.temperature_c = Some(40.0);
    crate::solubility::sync_fill_ml(dest);

    let water_before = item(&scene, "beaker-water");
    let source_ml = water_ml(water_before);
    let na_before = aqueous_mol(water_before, "na+");
    let sand_before = solid_g(water_before, "sand");
    let source_t = water_before.properties.temperature_c.unwrap_or(20.0);
    let transferred = 250.0 - 100.0;
    let frac = transferred / source_ml;

    use_tongs(&mut scene, "beaker-water").unwrap();
    use_tongs(&mut scene, "filter-paper-1").unwrap();

    let water = item(&scene, "beaker-water");
    let filtrate = item(&scene, "beaker-filtrate");
    let paper = item(&scene, "filter-paper-1");
    assert!((water_ml(filtrate) - 250.0).abs() < 1e-9);
    assert!((water_ml(water) - (source_ml - transferred)).abs() < 1e-9);
    assert!((aqueous_mol(filtrate, "na+") - na_before * frac).abs() < 1e-12);
    assert!((solid_g(paper, "sand") - sand_before * frac).abs() < 1e-12);
    assert_eq!(solid_g(filtrate, "sand"), 0.0);
    assert!((solid_g(water, "sand") - sand_before * (1.0 - frac)).abs() < 1e-12);
    let expected_t = (100.0 * 40.0 + transferred * source_t) / 250.0;
    assert!((filtrate.properties.temperature_c.unwrap() - expected_t).abs() < 1e-9);
    assert_eq!(water.location, "held");
}

#[test]
fn filter_pour_rejects_when_filtrate_is_full_and_stays_holding() {
    let mut scene = initial_bench_scene("lab-test");
    set_slurry_in_water(&mut scene);
    let dest = scene
        .items
        .iter_mut()
        .find(|i| i.id == "beaker-filtrate")
        .unwrap();
    dest.properties.composition = vec![CompositionEntry {
        substance_id: "water".into(),
        phase: "liquid".into(),
        amount_ml: Some(250.0),
        amount_scoop: None,
        amount_g: None,
        amount_mol: None,
    }];
    crate::solubility::sync_fill_ml(dest);

    use_tongs(&mut scene, "beaker-water").unwrap();
    let sand_before = solid_g(item(&scene, "beaker-water"), "sand");
    let err = use_tongs(&mut scene, "filter-paper-1").unwrap_err();
    assert_eq!(err, SceneError::InvalidAction);
    assert_eq!(item(&scene, "beaker-water").location, "held");
    assert!((solid_g(item(&scene, "beaker-water"), "sand") - sand_before).abs() < 1e-12);
    assert_eq!(solid_g(item(&scene, "filter-paper-1"), "sand"), 0.0);
    assert!((water_ml(item(&scene, "beaker-filtrate")) - 250.0).abs() < 1e-9);
}

#[test]
fn tongs_solids_only_dumps_into_filtrate_and_onto_paper() {
    let mut scene = initial_bench_scene("lab-test");
    let water = scene
        .items
        .iter_mut()
        .find(|i| i.id == "beaker-water")
        .unwrap();
    water.properties.composition = vec![solid("sand", 0.4)];
    crate::solubility::sync_fill_ml(water);

    use_tongs(&mut scene, "beaker-water").unwrap();
    use_tongs(&mut scene, "beaker-filtrate").unwrap();
    assert_eq!(item(&scene, "beaker-water").location, "held");
    assert_eq!(solid_g(item(&scene, "beaker-water"), "sand"), 0.0);
    assert!((solid_g(item(&scene, "beaker-filtrate"), "sand") - 0.4).abs() < 1e-12);

    put_tongs_away(&mut scene).unwrap();
    let water = scene
        .items
        .iter_mut()
        .find(|i| i.id == "beaker-water")
        .unwrap();
    water.properties.composition = vec![solid("nacl", 0.3)];
    crate::solubility::sync_fill_ml(water);
    use_tongs(&mut scene, "beaker-water").unwrap();
    use_tongs(&mut scene, "filter-paper-1").unwrap();
    assert_eq!(solid_g(item(&scene, "beaker-water"), "nacl"), 0.0);
    assert!((solid_g(item(&scene, "filter-paper-1"), "nacl") - 0.3).abs() < 1e-12);
}

#[test]
fn filter_pour_empty_source_errors_and_stays_holding() {
    let mut scene = initial_bench_scene("lab-test");
    use_tongs(&mut scene, "dish-1").unwrap();
    let err = use_tongs(&mut scene, "filter-paper-1").unwrap_err();
    assert_eq!(err, SceneError::EmptyHolding);
    assert_eq!(item(&scene, "dish-1").location, "held");
    assert_eq!(
        item(&scene, "tongs-1").properties.source_item_id.as_deref(),
        Some("dish-1")
    );
}

#[test]
fn filter_pour_rejects_when_filtrate_beaker_is_not_home() {
    let mut scene = initial_bench_scene("lab-test");
    use_tongs(&mut scene, "beaker-filtrate").unwrap();
    put_tongs_away(&mut scene).unwrap();
    use_tongs(&mut scene, "beaker-filtrate").unwrap();
    let mut other = initial_bench_scene("lab-test");
    other.items = scene.items.clone();
    other
        .items
        .iter_mut()
        .find(|i| i.id == "beaker-water")
        .unwrap()
        .location = "held".into();
    other
        .items
        .iter_mut()
        .find(|i| i.id == "tongs-1")
        .unwrap()
        .properties
        .source_item_id = Some("beaker-water".into());
    other
        .items
        .iter_mut()
        .find(|i| i.id == "beaker-filtrate")
        .unwrap()
        .location = "held".into();

    let err = use_tongs(&mut other, "filter-paper-1").unwrap_err();
    assert_eq!(err, SceneError::InvalidAction);
    assert_eq!(item(&other, "beaker-filtrate").location, "held");
    assert_eq!(
        item(&other, "tongs-1").properties.source_item_id.as_deref(),
        Some("beaker-water")
    );
}

#[test]
fn tongs_holding_filtrate_pours_into_water_dish_and_h2o() {
    let mut scene = initial_bench_scene("lab-test");
    let filtrate = scene
        .items
        .iter_mut()
        .find(|i| i.id == "beaker-filtrate")
        .unwrap();
    filtrate.properties.composition = vec![CompositionEntry {
        substance_id: "water".into(),
        phase: "liquid".into(),
        amount_ml: Some(30.0),
        amount_scoop: None,
        amount_g: None,
        amount_mol: None,
    }];
    crate::solubility::sync_fill_ml(filtrate);

    use_tongs(&mut scene, "beaker-filtrate").unwrap();
    use_tongs(&mut scene, "dish-1").unwrap();
    assert!((water_ml(item(&scene, "dish-1")) - 25.0).abs() < 1e-9);
    assert!((water_ml(item(&scene, "beaker-filtrate")) - 5.0).abs() < 1e-9);
    assert_eq!(item(&scene, "beaker-filtrate").location, "held");

    use_tongs(&mut scene, "beaker-water").unwrap();
    assert!((water_ml(item(&scene, "beaker-filtrate")) - 0.0).abs() < 1e-9);
    assert!((water_ml(item(&scene, "beaker-water")) - 5.0).abs() < 1e-9);

    put_tongs_away(&mut scene).unwrap();
    let filtrate = scene
        .items
        .iter_mut()
        .find(|i| i.id == "beaker-filtrate")
        .unwrap();
    filtrate.properties.composition = vec![CompositionEntry {
        substance_id: "water".into(),
        phase: "liquid".into(),
        amount_ml: Some(10.0),
        amount_scoop: None,
        amount_g: None,
        amount_mol: None,
    }];
    crate::solubility::sync_fill_ml(filtrate);
    scene
        .items
        .iter_mut()
        .find(|i| i.id == "beaker-h2o")
        .unwrap()
        .properties
        .composition[0]
        .amount_ml = Some(90.0);
    crate::solubility::sync_fill_ml(
        scene
            .items
            .iter_mut()
            .find(|i| i.id == "beaker-h2o")
            .unwrap(),
    );

    use_tongs(&mut scene, "beaker-filtrate").unwrap();
    use_tongs(&mut scene, "beaker-h2o").unwrap();
    assert!((water_ml(item(&scene, "beaker-h2o")) - 100.0).abs() < 1e-9);
    assert!((water_ml(item(&scene, "beaker-filtrate")) - 0.0).abs() < 1e-9);
}

#[test]
fn pipette_extracts_one_ml_from_seated_filtrate() {
    let mut scene = initial_bench_scene("lab-test");
    let filtrate = scene
        .items
        .iter_mut()
        .find(|i| i.id == "beaker-filtrate")
        .unwrap();
    filtrate.properties.composition = vec![CompositionEntry {
        substance_id: "water".into(),
        phase: "liquid".into(),
        amount_ml: Some(10.0),
        amount_scoop: None,
        amount_g: None,
        amount_mol: None,
    }];
    crate::solubility::sync_fill_ml(filtrate);

    fill_pipette_from(&mut scene, "beaker-filtrate");
    assert!((water_ml(item(&scene, "beaker-filtrate")) - 9.0).abs() < 1e-9);
    assert!((pipette_holding_liquid_ml(item(&scene, "pipette-1")) - 1.0).abs() < 1e-9);
    assert_eq!(
        item(&scene, "pipette-1")
            .properties
            .source_item_id
            .as_deref(),
        Some("beaker-filtrate")
    );

    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "pipette-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();
    assert!((water_ml(item(&scene, "beaker-water")) - 1.0).abs() < 1e-9);
    assert!(item(&scene, "pipette-1").properties.holding.is_empty());
}

#[test]
fn pipette_rejects_paper_and_held_filtrate() {
    let mut scene = initial_bench_scene("lab-test");
    let filtrate = scene
        .items
        .iter_mut()
        .find(|i| i.id == "beaker-filtrate")
        .unwrap();
    filtrate.properties.composition = vec![CompositionEntry {
        substance_id: "water".into(),
        phase: "liquid".into(),
        amount_ml: Some(10.0),
        amount_scoop: None,
        amount_g: None,
        amount_mol: None,
    }];
    crate::solubility::sync_fill_ml(filtrate);

    let err = apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "pipette-1".into(),
            target_item_id: "filter-paper-1".into(),
        },
    )
    .unwrap_err();
    assert_eq!(err, SceneError::InvalidAction);

    use_tongs(&mut scene, "beaker-filtrate").unwrap();
    let err = apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "pipette-1".into(),
            target_item_id: "beaker-filtrate".into(),
        },
    )
    .unwrap_err();
    assert_eq!(err, SceneError::InvalidAction);
    assert!((water_ml(item(&scene, "beaker-filtrate")) - 10.0).abs() < 1e-9);
}

#[test]
fn use_tool_spoon_scoops_paper_solids_in_mass_ratio() {
    let mut scene = initial_bench_scene("lab-test");
    set_paper_solids(&mut scene, vec![solid("nacl", 0.6), solid("cacl2", 0.4)]);
    scoop_paper(&mut scene).unwrap();

    let spoon = item(&scene, "spoon-1");
    assert_eq!(spoon.location, "hand");
    assert_eq!(
        spoon.properties.source_item_id.as_deref(),
        Some("filter-paper-1")
    );
    assert!((holding_g(spoon, "nacl") - 0.12).abs() < 1e-12);
    assert!((holding_g(spoon, "cacl2") - 0.08).abs() < 1e-12);
    assert!((solid_g(item(&scene, "filter-paper-1"), "nacl") - 0.48).abs() < 1e-12);
    assert!((solid_g(item(&scene, "filter-paper-1"), "cacl2") - 0.32).abs() < 1e-12);
    assert!(scene.last_events.iter().any(|e| e.kind == "scooped"));
}

#[test]
fn use_tool_spoon_takes_all_when_paper_solids_below_scoop_mass() {
    let mut scene = initial_bench_scene("lab-test");
    set_paper_solids(&mut scene, vec![solid("sand", 0.1)]);
    scoop_paper(&mut scene).unwrap();
    assert!((holding_g(item(&scene, "spoon-1"), "sand") - 0.1).abs() < 1e-12);
    assert_eq!(solid_g(item(&scene, "filter-paper-1"), "sand"), 0.0);
}

#[test]
fn put_away_returns_paper_scoop_to_paper_not_stock() {
    let mut scene = initial_bench_scene("lab-test");
    set_paper_solids(&mut scene, vec![solid("nacl", 0.5)]);
    scoop_paper(&mut scene).unwrap();
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
    assert!((solid_g(item(&scene, "filter-paper-1"), "nacl") - 0.5).abs() < 1e-12);
    assert_eq!(solid_g(item(&scene, "beaker-nacl"), "nacl"), 2.0);
    assert!(scene.last_events.iter().any(|e| e.kind == "returned"));
}

#[test]
fn use_tool_returns_single_species_paper_scoop_to_matching_stock() {
    let mut scene = initial_bench_scene("lab-test");
    set_paper_solids(&mut scene, vec![solid("nacl", 0.5)]);
    scoop_paper(&mut scene).unwrap();
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-nacl".into(),
        },
    )
    .unwrap();
    assert!(item(&scene, "spoon-1").properties.holding.is_empty());
    assert!((solid_g(item(&scene, "beaker-nacl"), "nacl") - 2.2).abs() < 1e-12);
    assert!((solid_g(item(&scene, "filter-paper-1"), "nacl") - 0.3).abs() < 1e-12);
}

#[test]
fn use_tool_rejects_empty_paper_and_wet_filtrate_scoop() {
    let mut scene = initial_bench_scene("lab-test");
    assert_eq!(
        scoop_paper(&mut scene).unwrap_err(),
        SceneError::InvalidAction
    );

    let filtrate = scene
        .items
        .iter_mut()
        .find(|i| i.id == "beaker-filtrate")
        .unwrap();
    filtrate.properties.composition = vec![
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
    crate::solubility::sync_fill_ml(filtrate);
    let err = apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-filtrate".into(),
        },
    )
    .unwrap_err();
    assert_eq!(err, SceneError::InvalidAction);
    assert!(item(&scene, "spoon-1").properties.holding.is_empty());
    assert!((solid_g(item(&scene, "beaker-filtrate"), "nacl") - 0.5).abs() < 1e-12);
}

#[test]
fn use_tool_spoon_scoops_dry_beaker_and_filtrate_in_mass_ratio() {
    let mut scene = initial_bench_scene("lab-test");
    let water = scene
        .items
        .iter_mut()
        .find(|i| i.id == "beaker-water")
        .unwrap();
    water.properties.composition = vec![solid("nacl", 0.6), solid("cacl2", 0.4)];
    crate::solubility::sync_fill_ml(water);

    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();
    let spoon = item(&scene, "spoon-1");
    assert!((holding_g(spoon, "nacl") - 0.12).abs() < 1e-12);
    assert!((holding_g(spoon, "cacl2") - 0.08).abs() < 1e-12);
    assert_eq!(
        spoon.properties.source_item_id.as_deref(),
        Some("beaker-water")
    );
    apply_action(
        &mut scene,
        Action::PutAway {
            tool_item_id: "spoon-1".into(),
        },
    )
    .unwrap();

    let filtrate = scene
        .items
        .iter_mut()
        .find(|i| i.id == "beaker-filtrate")
        .unwrap();
    filtrate.properties.composition = vec![solid("sand", 0.15)];
    crate::solubility::sync_fill_ml(filtrate);
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-filtrate".into(),
        },
    )
    .unwrap();
    assert!((holding_g(item(&scene, "spoon-1"), "sand") - 0.15).abs() < 1e-12);
    assert_eq!(solid_g(item(&scene, "beaker-filtrate"), "sand"), 0.0);
}

#[test]
fn use_tool_spoon_rejects_wet_beaker_scoop() {
    let mut scene = bench_with_water("lab-test");
    let water = scene
        .items
        .iter_mut()
        .find(|i| i.id == "beaker-water")
        .unwrap();
    water.properties.composition.push(solid("nacl", 0.5));
    assert_eq!(
        apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "beaker-water".into(),
            },
        )
        .unwrap_err(),
        SceneError::InvalidAction
    );
    assert!(item(&scene, "spoon-1").properties.holding.is_empty());
}

#[test]
fn use_tool_spoon_deposits_into_paper_filtrate_and_rejects_h2o() {
    let mut scene = initial_bench_scene("lab-test");
    scoop_nacl_stock(&mut scene).unwrap();
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "filter-paper-1".into(),
        },
    )
    .unwrap();
    assert!(item(&scene, "spoon-1").properties.holding.is_empty());
    assert!((solid_g(item(&scene, "filter-paper-1"), "nacl") - SPOON_SCOOP_MASS_G).abs() < 1e-12);

    scoop_nacl_stock(&mut scene).unwrap();
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-filtrate".into(),
        },
    )
    .unwrap();
    assert!((solid_g(item(&scene, "beaker-filtrate"), "nacl") - SPOON_SCOOP_MASS_G).abs() < 1e-12);

    scoop_nacl_stock(&mut scene).unwrap();
    assert_eq!(
        apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "beaker-h2o".into(),
            },
        )
        .unwrap_err(),
        SceneError::InvalidAction
    );
    assert!((holding_g(item(&scene, "spoon-1"), "nacl") - SPOON_SCOOP_MASS_G).abs() < 1e-12);
}

#[test]
fn use_tool_spoon_dissolves_into_wet_filtrate() {
    let mut scene = initial_bench_scene("lab-test");
    let filtrate = scene
        .items
        .iter_mut()
        .find(|i| i.id == "beaker-filtrate")
        .unwrap();
    filtrate.properties.composition = vec![CompositionEntry {
        substance_id: "water".into(),
        phase: "liquid".into(),
        amount_ml: Some(10.0),
        amount_scoop: None,
        amount_g: None,
        amount_mol: None,
    }];
    crate::solubility::sync_fill_ml(filtrate);
    scoop_nacl_stock(&mut scene).unwrap();
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-filtrate".into(),
        },
    )
    .unwrap();
    assert!(item(&scene, "spoon-1").properties.holding.is_empty());
    assert!(aqueous_mol(item(&scene, "beaker-filtrate"), "na+") > 0.0);
    assert_eq!(solid_g(item(&scene, "beaker-filtrate"), "nacl"), 0.0);
}

#[test]
fn tongs_vessel_pours_into_filtrate_bypassing_paper() {
    let mut scene = bench_with_water("lab-test");
    use_tongs(&mut scene, "beaker-water").unwrap();
    use_tongs(&mut scene, "beaker-filtrate").unwrap();
    assert!(
        (water_ml(item(&scene, "beaker-filtrate")) - FILLED_MAIN_BEAKER_ML.min(250.0)).abs() < 1e-9
    );
    assert_eq!(solid_g(item(&scene, "filter-paper-1"), "sand"), 0.0);
    assert_eq!(item(&scene, "beaker-water").location, "held");
}

#[test]
fn pipette_dumps_into_filtrate_and_rejects_paper() {
    let mut scene = initial_bench_scene("lab-test");
    fill_pipette_from(&mut scene, "beaker-h2o");
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "pipette-1".into(),
            target_item_id: "beaker-filtrate".into(),
        },
    )
    .unwrap();
    assert!((water_ml(item(&scene, "beaker-filtrate")) - 1.0).abs() < 1e-9);
    assert!(item(&scene, "pipette-1").properties.holding.is_empty());

    fill_pipette_from(&mut scene, "beaker-h2o");
    assert_eq!(
        apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "pipette-1".into(),
                target_item_id: "filter-paper-1".into(),
            },
        )
        .unwrap_err(),
        SceneError::InvalidAction
    );
}

#[test]
fn pipette_from_filtrate_leaves_solids_and_scales_ions() {
    let mut scene = initial_bench_scene("lab-test");
    let filtrate = scene
        .items
        .iter_mut()
        .find(|i| i.id == "beaker-filtrate")
        .unwrap();
    filtrate.properties.composition = vec![
        CompositionEntry {
            substance_id: "water".into(),
            phase: "liquid".into(),
            amount_ml: Some(10.0),
            amount_scoop: None,
            amount_g: None,
            amount_mol: None,
        },
        CompositionEntry {
            substance_id: "na+".into(),
            phase: "aqueous".into(),
            amount_ml: None,
            amount_scoop: None,
            amount_g: None,
            amount_mol: Some(0.02),
        },
        CompositionEntry {
            substance_id: "cl-".into(),
            phase: "aqueous".into(),
            amount_ml: None,
            amount_scoop: None,
            amount_g: None,
            amount_mol: Some(0.02),
        },
        solid("sand", 0.5),
    ];
    crate::solubility::sync_fill_ml(filtrate);

    fill_pipette_from(&mut scene, "beaker-filtrate");
    let pipette = item(&scene, "pipette-1");
    assert!((pipette_holding_liquid_ml(pipette) - 1.0).abs() < 1e-12);
    assert!((aqueous_mol_holding(pipette, "na+") - 0.002).abs() < 1e-12);
    assert_eq!(solid_g(pipette, "sand"), 0.0);
    assert!((solid_g(item(&scene, "beaker-filtrate"), "sand") - 0.5).abs() < 1e-12);
    assert!((aqueous_mol(item(&scene, "beaker-filtrate"), "na+") - 0.018).abs() < 1e-12);
}
