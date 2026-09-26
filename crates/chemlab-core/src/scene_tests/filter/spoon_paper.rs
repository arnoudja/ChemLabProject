//! Spoon interactions with filter paper and filtrate.

use super::super::super::*;
use super::super::helpers::*;

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

