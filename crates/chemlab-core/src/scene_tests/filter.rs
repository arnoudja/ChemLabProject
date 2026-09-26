use super::super::*;
use super::helpers::*;

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
    let source_v = crate::hcl::solution_volume_ml(water_before);
    let na_before = aqueous_mol(water_before, "na+");
    let sand_before = solid_g(water_before, "sand");
    let source_t = water_before.properties.temperature_c.unwrap_or(20.0);
    let dest_v_before = 100.0;
    let transferred_v = 250.0 - dest_v_before;
    let frac = transferred_v / source_v;
    let water_transferred = source_ml * frac;

    use_tongs(&mut scene, "beaker-water").unwrap();
    use_tongs(&mut scene, "filter-paper-1").unwrap();

    let water = item(&scene, "beaker-water");
    let filtrate = item(&scene, "beaker-filtrate");
    let paper = item(&scene, "filter-paper-1");
    assert!((crate::hcl::solution_volume_ml(filtrate) - 250.0).abs() < 1e-6);
    assert!((water_ml(filtrate) - (100.0 + water_transferred)).abs() < 1e-9);
    assert!(water_ml(filtrate) < 250.0 - 1e-6);
    assert!((water_ml(water) - (source_ml - water_transferred)).abs() < 1e-9);
    assert!((aqueous_mol(filtrate, "na+") - na_before * frac).abs() < 1e-12);
    assert!((solid_g(paper, "sand") - sand_before * frac).abs() < 1e-12);
    assert_eq!(solid_g(filtrate, "sand"), 0.0);
    assert!((solid_g(water, "sand") - sand_before * (1.0 - frac)).abs() < 1e-12);
    let c_dest = C_BEAKER + 100.0 * WATER_SPECIFIC_HEAT_J_PER_G_K;
    let c_add = water_transferred * WATER_SPECIFIC_HEAT_J_PER_G_K;
    let expected_t = (c_dest * 40.0 + c_add * source_t) / (c_dest + c_add);
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
    let v_sol = 10.0 + 0.02 * crate::hcl::PHI_V_NACL_ML_PER_MOL;
    let frac = PIPETTE_VOLUME_ML / v_sol;
    assert!((pipette_holding_liquid_ml(pipette) - 1.0).abs() < 1e-12);
    assert!((aqueous_mol_holding(pipette, "na+") - 0.02 * frac).abs() < 1e-12);
    assert_eq!(solid_g(pipette, "sand"), 0.0);
    assert!((solid_g(item(&scene, "beaker-filtrate"), "sand") - 0.5).abs() < 1e-12);
    assert!(
        (aqueous_mol(item(&scene, "beaker-filtrate"), "na+") - 0.02 * (1.0 - frac)).abs() < 1e-12
    );
}

fn put_solids_on_paper(scene: &mut Scene, solids: Vec<CompositionEntry>) {
    let paper = scene
        .items
        .iter_mut()
        .find(|i| i.id == "filter-paper-1")
        .unwrap();
    paper.properties.composition = solids;
}

fn set_source_water(scene: &mut Scene, ml: f64, temperature_c: f64) {
    let water = scene
        .items
        .iter_mut()
        .find(|i| i.id == "beaker-water")
        .unwrap();
    water.properties.composition = vec![CompositionEntry {
        substance_id: "water".into(),
        phase: "liquid".into(),
        amount_ml: Some(ml),
        amount_scoop: None,
        amount_g: None,
        amount_mol: None,
    }];
    water.properties.temperature_c = Some(temperature_c);
    crate::solubility::sync_fill_ml(water);
}

fn filter_pour_water_through_paper(scene: &mut Scene) {
    use_tongs(scene, "beaker-water").unwrap();
    use_tongs(scene, "filter-paper-1").unwrap();
}

#[test]
fn filter_pour_washes_unsaturated_nacl_from_paper_into_filtrate() {
    let mut scene = initial_bench_scene("lab-test");
    put_solids_on_paper(&mut scene, vec![solid("nacl", 0.5), solid("sand", 0.3)]);
    set_source_water(&mut scene, 50.0, 20.0);

    filter_pour_water_through_paper(&mut scene);

    let filtrate = item(&scene, "beaker-filtrate");
    let paper = item(&scene, "filter-paper-1");
    let na = aqueous_mol(filtrate, "na+");
    let cl = aqueous_mol(filtrate, "cl-");
    assert!(
        na > 1e-6,
        "expected some NaCl washed into filtrate, got na+={na}"
    );
    assert!((cl - na).abs() < 1e-12);
    assert!(solid_g(paper, "nacl") < 0.5 - 1e-6);
    assert!((solid_g(paper, "nacl") + na * NACL_MOLAR_MASS_G_PER_MOL - 0.5).abs() < 1e-9);
    assert!((solid_g(paper, "sand") - 0.3).abs() < 1e-12);
    assert_eq!(solid_g(filtrate, "sand"), 0.0);
    assert_eq!(solid_g(filtrate, "nacl"), 0.0);
}

#[test]
fn filter_pour_wash_dissolves_nothing_when_fluid_already_saturated() {
    let mut scene = initial_bench_scene("lab-test");
    put_solids_on_paper(&mut scene, vec![solid("nacl", 0.5)]);
    let s20 = crate::solubility::solubility_mol_per_l(crate::solubility::Salt::Nacl, 20.0);
    let water_ml = 20.0;
    let sat_mol = s20 * (water_ml / 1000.0);
    let water = scene
        .items
        .iter_mut()
        .find(|i| i.id == "beaker-water")
        .unwrap();
    water.properties.composition = vec![
        CompositionEntry {
            substance_id: "water".into(),
            phase: "liquid".into(),
            amount_ml: Some(water_ml),
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
            amount_mol: Some(sat_mol),
        },
        CompositionEntry {
            substance_id: "cl-".into(),
            phase: "aqueous".into(),
            amount_ml: None,
            amount_scoop: None,
            amount_g: None,
            amount_mol: Some(sat_mol),
        },
    ];
    water.properties.temperature_c = Some(20.0);
    crate::solubility::sync_fill_ml(water);
    crate::solubility::enforce_saturation(water);

    let paper_nacl_before = solid_g(item(&scene, "filter-paper-1"), "nacl");
    filter_pour_water_through_paper(&mut scene);

    let filtrate = item(&scene, "beaker-filtrate");
    let paper = item(&scene, "filter-paper-1");
    // Saturated rinse: wash adds ~0 solid; filtrate may still hold the poured ions.
    assert!((solid_g(paper, "nacl") - paper_nacl_before).abs() < 1e-6);
    let na_from_source = aqueous_mol(filtrate, "na+");
    // No extra Na from paper beyond what the saturated source already carried.
    assert!((na_from_source - sat_mol).abs() < 1e-4);
}

#[test]
fn filter_pour_wash_leaves_sand_on_paper() {
    let mut scene = initial_bench_scene("lab-test");
    put_solids_on_paper(&mut scene, vec![solid("sand", 0.8)]);
    set_source_water(&mut scene, 40.0, 20.0);
    filter_pour_water_through_paper(&mut scene);
    assert!((solid_g(item(&scene, "filter-paper-1"), "sand") - 0.8).abs() < 1e-12);
    assert_eq!(solid_g(item(&scene, "beaker-filtrate"), "sand"), 0.0);
    assert_eq!(aqueous_mol(item(&scene, "beaker-filtrate"), "na+"), 0.0);
}

#[test]
fn filter_pour_larger_volume_washes_more_nacl_than_smaller() {
    let paper_nacl = 1.0;
    let mut small = initial_bench_scene("lab-test");
    put_solids_on_paper(&mut small, vec![solid("nacl", paper_nacl)]);
    set_source_water(&mut small, 10.0, 20.0);
    filter_pour_water_through_paper(&mut small);
    let washed_small = paper_nacl - solid_g(item(&small, "filter-paper-1"), "nacl");

    let mut large = initial_bench_scene("lab-test");
    put_solids_on_paper(&mut large, vec![solid("nacl", paper_nacl)]);
    set_source_water(&mut large, 200.0, 20.0);
    filter_pour_water_through_paper(&mut large);
    let washed_large = paper_nacl - solid_g(item(&large, "filter-paper-1"), "nacl");

    assert!(washed_small > 1e-6, "small rinse should dissolve some salt");
    assert!(
        washed_large > washed_small + 1e-4,
        "larger pour should wash more: small={washed_small} large={washed_large}"
    );
}

#[test]
fn filter_pour_hot_rinse_has_higher_nacl_wash_capacity_than_cold() {
    // Near-saturation solid load so capacity (not kinetics alone) differs with T.
    let paper_nacl = 8.0;
    let rinse_ml = 20.0;

    let mut cold = initial_bench_scene("lab-test");
    put_solids_on_paper(&mut cold, vec![solid("nacl", paper_nacl)]);
    set_source_water(&mut cold, rinse_ml, 20.0);
    filter_pour_water_through_paper(&mut cold);
    let washed_cold = paper_nacl - solid_g(item(&cold, "filter-paper-1"), "nacl");

    let mut hot = initial_bench_scene("lab-test");
    put_solids_on_paper(&mut hot, vec![solid("nacl", paper_nacl)]);
    set_source_water(&mut hot, rinse_ml, 80.0);
    filter_pour_water_through_paper(&mut hot);
    let washed_hot = paper_nacl - solid_g(item(&hot, "filter-paper-1"), "nacl");

    assert!(
        washed_hot > washed_cold + 1e-4,
        "hot rinse should dissolve more when capacity-limited: cold={washed_cold} hot={washed_hot}"
    );
}

#[test]
fn filter_pour_washes_soluble_solids_arriving_in_same_pour() {
    let mut scene = initial_bench_scene("lab-test");
    let source_nacl = 0.4;
    let water = scene
        .items
        .iter_mut()
        .find(|i| i.id == "beaker-water")
        .unwrap();
    water.properties.composition = vec![
        CompositionEntry {
            substance_id: "water".into(),
            phase: "liquid".into(),
            amount_ml: Some(50.0),
            amount_scoop: None,
            amount_g: None,
            amount_mol: None,
        },
        solid("nacl", source_nacl),
    ];
    water.properties.temperature_c = Some(20.0);
    crate::solubility::sync_fill_ml(water);

    filter_pour_water_through_paper(&mut scene);

    let filtrate = item(&scene, "beaker-filtrate");
    let paper = item(&scene, "filter-paper-1");
    let na = aqueous_mol(filtrate, "na+");
    assert!(
        na > 1e-6,
        "same-pour solid NaCl should partially wash into filtrate, na+={na}"
    );
    assert!(solid_g(paper, "nacl") < source_nacl - 1e-6);
    assert!((solid_g(paper, "nacl") + na * NACL_MOLAR_MASS_G_PER_MOL - source_nacl).abs() < 1e-9);
    assert_eq!(solid_g(item(&scene, "beaker-water"), "nacl"), 0.0);
}

#[test]
fn filter_pour_washes_unsaturated_cacl2_from_paper_into_filtrate() {
    let mut scene = initial_bench_scene("lab-test");
    put_solids_on_paper(&mut scene, vec![solid("cacl2", 0.5)]);
    set_source_water(&mut scene, 50.0, 20.0);

    filter_pour_water_through_paper(&mut scene);

    let filtrate = item(&scene, "beaker-filtrate");
    let paper = item(&scene, "filter-paper-1");
    let ca = aqueous_mol(filtrate, "ca2+");
    let cl = aqueous_mol(filtrate, "cl-");
    assert!(
        ca > 1e-6,
        "expected some CaCl2 washed into filtrate, ca2+={ca}"
    );
    assert!((cl - 2.0 * ca).abs() < 1e-12);
    assert!(solid_g(paper, "cacl2") < 0.5 - 1e-6);
    assert!((solid_g(paper, "cacl2") + ca * CACL2_MOLAR_MASS_G_PER_MOL - 0.5).abs() < 1e-9);
    assert_eq!(solid_g(filtrate, "cacl2"), 0.0);
}

#[test]
fn filter_pour_washes_naoh_from_paper_into_filtrate() {
    let mut scene = initial_bench_scene("lab-test");
    put_solids_on_paper(&mut scene, vec![solid("naoh", 0.5), solid("sand", 0.2)]);
    set_source_water(&mut scene, 50.0, 20.0);

    filter_pour_water_through_paper(&mut scene);

    let filtrate = item(&scene, "beaker-filtrate");
    let paper = item(&scene, "filter-paper-1");
    let na = aqueous_mol(filtrate, "na+");
    let oh = aqueous_mol(filtrate, "oh-");
    assert!(
        oh > 1e-6,
        "expected some NaOH washed into filtrate, oh-={oh}"
    );
    assert!((na - oh).abs() < 1e-12);
    assert!(solid_g(paper, "naoh") < 0.5 - 1e-6);
    assert!((solid_g(paper, "naoh") + oh * 40.0 - 0.5).abs() < 1e-9);
    assert!((solid_g(paper, "sand") - 0.2).abs() < 1e-12);
    assert_eq!(solid_g(filtrate, "sand"), 0.0);
    assert_eq!(solid_g(filtrate, "naoh"), 0.0);
    assert!(aqueous_mol(filtrate, "cl-") < 1e-12);
}

#[test]
fn filter_pour_wash_cools_filtrate_when_nacl_dissolves() {
    let mut with_salt = initial_bench_scene("lab-test");
    put_solids_on_paper(&mut with_salt, vec![solid("nacl", 1.0)]);
    set_source_water(&mut with_salt, 50.0, 20.0);
    filter_pour_water_through_paper(&mut with_salt);
    let t_with = item(&with_salt, "beaker-filtrate")
        .properties
        .temperature_c
        .unwrap();

    let mut plain = initial_bench_scene("lab-test");
    set_source_water(&mut plain, 50.0, 20.0);
    filter_pour_water_through_paper(&mut plain);
    let t_plain = item(&plain, "beaker-filtrate")
        .properties
        .temperature_c
        .unwrap();

    assert!(
        aqueous_mol(item(&with_salt, "beaker-filtrate"), "na+") > 1e-6,
        "wash must dissolve some NaCl for ΔT check"
    );
    assert!(
        t_with < t_plain - 1e-6,
        "endothermic NaCl wash should cool filtrate vs plain rinse: with={t_with} plain={t_plain}"
    );
}
