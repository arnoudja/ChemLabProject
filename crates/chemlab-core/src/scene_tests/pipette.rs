use super::super::*;
use super::helpers::*;

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
    let solution_before = crate::hcl::solution_volume_ml(water_before);
    assert!(
        solution_before > liquid_before + 1e-6,
        "dissolved NaCl must increase solution volume above water ml"
    );

    fill_pipette_from(&mut scene, "beaker-water");

    let water = item(&scene, "beaker-water");
    let frac = PIPETTE_VOLUME_ML / solution_before;
    let expected_water = liquid_before * (1.0 - frac);
    assert!((water_ml(water) - expected_water).abs() < 1e-9);
    assert!(
        water_ml(water) > 199.0,
        "1.00 ml solution aliquot must remove <1.00 ml water when V_solution > water"
    );
    assert!((aqueous_mol(water, "na+") - na_before * (1.0 - frac)).abs() < 1e-12);
    assert!((aqueous_mol(water, "cl-") - cl_before * (1.0 - frac)).abs() < 1e-12);
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
    assert!((aqueous_mol_holding(pipette, "na+") - na_before * frac).abs() < 1e-12);
    assert!(pipette
        .properties
        .holding
        .iter()
        .all(|c| c.phase != "solid"));
}

#[test]
fn pipette_fill_from_dish_and_pour_back_to_water_is_consistent() {
    let mut scene = bench_with_water("lab-test");
    for _ in 0..3 {
        fill_pipette_from(&mut scene, "beaker-water");
        apply_action(
            &mut scene,
            Action::Pour {
                source_item_id: "pipette-1".into(),
                target_item_id: "dish-1".into(),
            },
        )
        .unwrap();
    }

    let dish = item(&scene, "dish-1");
    assert!((water_ml(dish) - PIPETTE_MIN_SOURCE_ML).abs() < 1e-9);
    assert_eq!(dish.properties.temperature_c, Some(20.0));
    assert!((water_ml(item(&scene, "beaker-water")) - 197.0).abs() < 1e-9);

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
    assert!((water_ml(water) - 198.0).abs() < 1e-9);
    assert_eq!(water.properties.temperature_c, Some(20.0));
    let dish = item(&scene, "dish-1");
    assert!((water_ml(dish) - 2.0).abs() < 1e-9);
    assert!(item(&scene, "pipette-1").properties.holding.is_empty());
}

#[test]
fn dish_rejects_over_capacity_and_fill_requires_min_source() {
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
    assert_eq!(err, SceneError::NotEnoughFluidAvailable);
}

#[test]
fn pipette_fill_from_empty_vessel_reports_no_fluid() {
    let mut scene = bench_with_water("lab-test");
    let err = apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "pipette-1".into(),
            target_item_id: "dish-1".into(),
        },
    )
    .unwrap_err();
    assert_eq!(err, SceneError::NoFluidAvailable);
    assert_eq!(err.to_string(), "No fluid available.");
    assert_eq!(item(&scene, "pipette-1").location, "bench");
    assert!(item(&scene, "pipette-1").properties.holding.is_empty());
}

#[test]
fn pipette_fill_below_min_source_reports_not_enough_fluid() {
    for available_ml in [1.0, 2.9] {
        let mut scene = bench_with_water("lab-test");
        set_dish_water(&mut scene, available_ml);

        let err = apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "pipette-1".into(),
                target_item_id: "dish-1".into(),
            },
        )
        .unwrap_err();
        assert_eq!(err, SceneError::NotEnoughFluidAvailable);
        assert_eq!(err.to_string(), "Not enough fluid available.");
        assert!((water_ml(item(&scene, "dish-1")) - available_ml).abs() < 1e-9);
        assert!(item(&scene, "pipette-1").properties.holding.is_empty());
    }
}

#[test]
fn pipette_fill_at_min_source_draws_one_ml_and_leaves_remainder() {
    let mut scene = bench_with_water("lab-test");
    set_dish_water(&mut scene, PIPETTE_MIN_SOURCE_ML);

    fill_pipette_from(&mut scene, "dish-1");

    assert!((water_ml(item(&scene, "dish-1")) - (PIPETTE_MIN_SOURCE_ML - 1.0)).abs() < 1e-9);
    let pipette = item(&scene, "pipette-1");
    assert!((pipette_holding_liquid_ml(pipette) - PIPETTE_VOLUME_ML).abs() < 1e-12);
    assert_eq!(pipette.properties.source_item_id.as_deref(), Some("dish-1"));
}

fn set_dish_water(scene: &mut Scene, amount_ml: f64) {
    let dish = scene.items.iter_mut().find(|i| i.id == "dish-1").unwrap();
    dish.properties.composition = vec![CompositionEntry {
        substance_id: "water".into(),
        phase: "liquid".into(),
        amount_ml: Some(amount_ml),
        amount_scoop: None,
        amount_g: None,
        amount_mol: None,
    }];
    crate::solubility::sync_fill_ml(dish);
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
fn pipette_from_saturated_nacl_removes_less_than_one_ml_water() {
    let mut scene = bench_with_water("lab-test");
    // ~35.89 g NaCl / 100 g water at 20 °C in 100 ml water → V_solution ≈ 113.5 ml.
    let water = scene
        .items
        .iter_mut()
        .find(|i| i.id == "beaker-water")
        .expect("beaker-water");
    water.properties.fill_ml = Some(100.0);
    water.properties.composition = vec![CompositionEntry {
        substance_id: "water".into(),
        phase: "liquid".into(),
        amount_ml: Some(100.0),
        amount_scoop: None,
        amount_g: None,
        amount_mol: None,
    }];
    let n_sat = 35.89 / NACL_MOLAR_MASS_G_PER_MOL;
    water.properties.composition.push(CompositionEntry {
        substance_id: "na+".into(),
        phase: "aqueous".into(),
        amount_ml: None,
        amount_scoop: None,
        amount_g: None,
        amount_mol: Some(n_sat),
    });
    water.properties.composition.push(CompositionEntry {
        substance_id: "cl-".into(),
        phase: "aqueous".into(),
        amount_ml: None,
        amount_scoop: None,
        amount_g: None,
        amount_mol: Some(n_sat),
    });
    crate::solubility::enforce_saturation(water);

    let before = item(&scene, "beaker-water");
    let water_before = water_ml(before);
    let v_sol = crate::hcl::solution_volume_ml(before);
    assert!(
        (v_sol - 113.5).abs() < 1.0,
        "sat NaCl solution volume expected ≈113.5 ml, got {v_sol}"
    );
    assert!(v_sol > water_before + 10.0);

    fill_pipette_from(&mut scene, "beaker-water");

    let after = item(&scene, "beaker-water");
    let water_removed = water_before - water_ml(after);
    assert!(
        water_removed < 1.0 - 1e-3,
        "1.00 ml aliquot from brine must remove <1.00 ml water, removed {water_removed}"
    );
    let expected_removed = water_before * (PIPETTE_VOLUME_ML / v_sol);
    assert!((water_removed - expected_removed).abs() < 1e-9);
    assert!(
        (pipette_holding_liquid_ml(item(&scene, "pipette-1")) - PIPETTE_VOLUME_ML).abs() < 1e-6
    );
}
