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
