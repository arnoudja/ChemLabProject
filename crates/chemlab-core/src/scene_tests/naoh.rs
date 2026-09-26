//! Free-mode solid NaOH stock: scoop/tongs, dissolve + ΔT, neutralization, pH.

use super::super::*;
use super::helpers::*;

fn scoop_naoh(scene: &mut Scene) {
    apply_action(
        scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-naoh".into(),
        },
    )
    .unwrap();
}

fn put_water_in_main(scene: &mut Scene, ml: f64) {
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
    water.properties.fill_ml = Some(ml);
    water.properties.temperature_c = Some(20.0);
}

#[test]
fn free_bench_includes_naoh_stock_at_two_grams() {
    let scene = initial_bench_scene("lab-test");
    let stock = item(&scene, "beaker-naoh");
    assert_eq!(stock.label, "Sodium hydroxide");
    assert_eq!(solid_g(stock, "naoh"), 2.0);
    let entry = stock
        .properties
        .composition
        .iter()
        .find(|c| c.substance_id == "naoh")
        .unwrap();
    assert_eq!(entry.amount_scoop, Some(10));
}

#[test]
fn spoon_scoops_and_returns_naoh() {
    let mut scene = initial_bench_scene("lab-test");
    scoop_naoh(&mut scene);
    assert!((solid_g(item(&scene, "beaker-naoh"), "naoh") - 1.8).abs() < 1e-12);
    let spoon = item(&scene, "spoon-1");
    assert_eq!(spoon.location, "hand");
    assert_eq!(spoon.properties.holding[0].substance_id, "naoh");
    assert!((spoon.properties.holding[0].amount_g.unwrap() - SPOON_SCOOP_MASS_G).abs() < 1e-12);

    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-naoh".into(),
        },
    )
    .unwrap();
    assert!((solid_g(item(&scene, "beaker-naoh"), "naoh") - 2.0).abs() < 1e-12);
    assert!(item(&scene, "spoon-1").properties.holding.is_empty());
}

#[test]
fn spoon_rejects_naoh_onto_nacl_stock() {
    let mut scene = initial_bench_scene("lab-test");
    scoop_naoh(&mut scene);
    let err = apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-nacl".into(),
        },
    )
    .unwrap_err();
    assert_eq!(err, SceneError::InvalidAction);
}

#[test]
fn naoh_dissolves_to_na_and_oh_with_exothermic_delta_t() {
    let mut scene = initial_bench_scene("lab-test");
    put_water_in_main(&mut scene, 200.0);
    scoop_naoh(&mut scene);
    let t_before = item(&scene, "beaker-water")
        .properties
        .temperature_c
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
    let moles = SPOON_SCOOP_MASS_G / 40.0;
    assert!((aqueous_mol(water, "na+") - moles).abs() < 1e-9);
    assert!((aqueous_mol(water, "oh-") - moles).abs() < 1e-9);
    assert!(aqueous_mol(water, "cl-") < 1e-12);
    assert!(!water
        .properties
        .composition
        .iter()
        .any(|c| c.substance_id == "naoh"));
    let t_after = water.properties.temperature_c.unwrap();
    assert!(
        t_after > t_before + 0.01,
        "NaOH dissolve must heat the beaker ({t_before} → {t_after})"
    );
    let ph = crate::hcl::ph_of_item(water).expect("base pH");
    assert!(ph > 12.0, "strong base pH, got {ph}");
    assert!(scene
        .last_events
        .iter()
        .any(|e| e.kind == "dissolved" && e.message.contains("Sodium hydroxide (NaOH) dissolves")));
}

#[test]
fn naoh_into_hcl_neutralizes_to_salt_and_heats() {
    let mut scene = initial_bench_scene("lab-test");
    let water = scene
        .items
        .iter_mut()
        .find(|i| i.id == "beaker-water")
        .unwrap();
    water.properties.composition = vec![
        CompositionEntry {
            substance_id: "water".into(),
            phase: "liquid".into(),
            amount_ml: Some(100.0),
            amount_scoop: None,
            amount_g: None,
            amount_mol: None,
        },
        CompositionEntry {
            substance_id: "h+".into(),
            phase: "aqueous".into(),
            amount_ml: None,
            amount_scoop: None,
            amount_g: None,
            amount_mol: Some(0.01),
        },
        CompositionEntry {
            substance_id: "cl-".into(),
            phase: "aqueous".into(),
            amount_ml: None,
            amount_scoop: None,
            amount_g: None,
            amount_mol: Some(0.01),
        },
    ];
    water.properties.fill_ml = Some(100.0);
    water.properties.temperature_c = Some(20.0);

    scoop_naoh(&mut scene);
    let t_before = item(&scene, "beaker-water")
        .properties
        .temperature_c
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
    let n_naoh = SPOON_SCOOP_MASS_G / 40.0;
    assert!((aqueous_mol(water, "h+") - (0.01 - n_naoh)).abs() < 1e-9);
    assert!(aqueous_mol(water, "oh-") < 1e-12);
    assert!((aqueous_mol(water, "na+") - n_naoh).abs() < 1e-9);
    assert!((aqueous_mol(water, "cl-") - 0.01).abs() < 1e-9);
    let t_after = water.properties.temperature_c.unwrap();
    assert!(
        t_after > t_before + 0.05,
        "dissolve + neutralization must heat ({t_before} → {t_after})"
    );
    let ph = crate::hcl::ph_of_item(water).expect("residual acid pH");
    assert!(ph < 7.0);
}

#[test]
fn tongs_dump_naoh_into_empty_beaker_stays_solid() {
    let mut scene = initial_bench_scene("lab-test");
    let stock_g = solid_g(item(&scene, "beaker-naoh"), "naoh");
    use_tongs(&mut scene, "beaker-naoh").unwrap();
    use_tongs(&mut scene, "beaker-water").unwrap();
    assert_eq!(solid_g(item(&scene, "beaker-naoh"), "naoh"), 0.0);
    assert!((solid_g(item(&scene, "beaker-water"), "naoh") - stock_g).abs() < 1e-12);
    assert_eq!(aqueous_mol(item(&scene, "beaker-water"), "oh-"), 0.0);
}

#[test]
fn tongs_return_naoh_to_matching_stock_only() {
    let mut scene = initial_bench_scene("lab-test");
    use_tongs(&mut scene, "beaker-naoh").unwrap();
    use_tongs(&mut scene, "dish-1").unwrap();
    put_tongs_away(&mut scene).unwrap();
    assert_eq!(solid_g(item(&scene, "beaker-naoh"), "naoh"), 0.0);
    assert!((solid_g(item(&scene, "dish-1"), "naoh") - 2.0).abs() < 1e-12);

    use_tongs(&mut scene, "dish-1").unwrap();
    use_tongs(&mut scene, "beaker-naoh").unwrap();
    assert_eq!(solid_g(item(&scene, "dish-1"), "naoh"), 0.0);
    assert!((solid_g(item(&scene, "beaker-naoh"), "naoh") - 2.0).abs() < 1e-12);
    put_tongs_away(&mut scene).unwrap();

    // Holding the NaOH stock, dumping into the NaCl stock is rejected.
    use_tongs(&mut scene, "beaker-naoh").unwrap();
    let err = use_tongs(&mut scene, "beaker-nacl").unwrap_err();
    assert_eq!(err, SceneError::InvalidAction);
}
