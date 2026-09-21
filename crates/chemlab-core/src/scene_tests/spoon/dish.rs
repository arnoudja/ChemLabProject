//! Spoon dish-solids scoop / return / deposit scene tests.

use super::super::super::*;
use super::super::helpers::*;

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
