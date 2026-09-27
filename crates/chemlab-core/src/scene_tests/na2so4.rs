//! Free-mode solid Na₂SO₄ stock: scoop/return and qualitative dissolve.

use super::super::*;
use super::helpers::*;

fn scoop_na2so4(scene: &mut Scene) {
    apply_action(
        scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-na2so4".into(),
        },
    )
    .unwrap();
}

#[test]
fn free_bench_includes_na2so4_stock_at_two_grams() {
    let scene = initial_bench_scene("lab-test");
    let stock = item(&scene, "beaker-na2so4");
    assert_eq!(stock.label, "Sodium sulfate");
    assert_eq!(solid_g(stock, "na2so4"), 2.0);
    let entry = stock
        .properties
        .composition
        .iter()
        .find(|c| c.substance_id == "na2so4")
        .unwrap();
    assert_eq!(entry.amount_scoop, Some(10));
}

#[test]
fn spoon_scoops_and_returns_na2so4() {
    let mut scene = initial_bench_scene("lab-test");
    scoop_na2so4(&mut scene);
    assert!((solid_g(item(&scene, "beaker-na2so4"), "na2so4") - 1.8).abs() < 1e-12);
    let spoon = item(&scene, "spoon-1");
    assert_eq!(spoon.location, "hand");
    assert_eq!(spoon.properties.holding[0].substance_id, "na2so4");
    assert!((spoon.properties.holding[0].amount_g.unwrap() - SPOON_SCOOP_MASS_G).abs() < 1e-12);

    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-na2so4".into(),
        },
    )
    .unwrap();
    assert!((solid_g(item(&scene, "beaker-na2so4"), "na2so4") - 2.0).abs() < 1e-12);
    assert!(item(&scene, "spoon-1").properties.holding.is_empty());
}

#[test]
fn spoon_pour_dissolves_na2so4_to_aqueous_ions() {
    let mut scene = bench_with_water("lab-test");
    scoop_na2so4(&mut scene);
    apply_action(
        &mut scene,
        Action::Pour {
            source_item_id: "spoon-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();

    // Pour contact is partial — leftover solid before clock ticks (same as NaCl).
    let water_after_pour = item(&scene, "beaker-water");
    assert!(aqueous_mol(water_after_pour, "na+") > 1e-6);
    assert!(
        solid_g(water_after_pour, "na2so4") > 0.05,
        "pour contact must leave noticeable solid Na₂SO₄"
    );
    assert!(scene.last_events.iter().any(|e| e.kind == "dissolved"));

    finish_kinetic_dissolve(&mut scene);

    let water = item(&scene, "beaker-water");
    let moles = SPOON_SCOOP_MASS_G / NA2SO4_MOLAR_MASS_G_PER_MOL;
    assert!((aqueous_mol(water, "na+") - 2.0 * moles).abs() < 1e-9);
    // Speciation leaves a tiny HSO₄⁻ fraction via Kₐ₂; conserve total sulfur.
    let sulfur = aqueous_mol(water, "so4^2-") + aqueous_mol(water, "hso4-");
    assert!((sulfur - moles).abs() < 1e-9);
    assert_eq!(solid_g(water, "na2so4"), 0.0);
}
