use super::super::*;
use super::helpers::*;

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
