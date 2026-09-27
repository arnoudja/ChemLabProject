//! Tongs and pipette with paper / filtrate vessels.

use super::super::super::*;
use super::super::helpers::*;

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
