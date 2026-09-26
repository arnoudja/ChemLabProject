//! Free-mode aqueous HCl stock: transfers, dilution heat, dissolve, pH, dish azeotrope.

use super::super::*;
use super::helpers::*;
use crate::hcl::{
    self, HCL_AZEOTROPE_BOIL_C, HCL_AZEOTROPE_W_W, HCL_STOCK_CAPACITY_ML, HCL_STOCK_HCL_MOLES,
    HCL_STOCK_WATER_MASS_G,
};

fn hcl_w_w(item: &SceneItem) -> f64 {
    hcl::HclInventory::from_item(item).w_hcl()
}

#[test]
fn pipette_transfers_hcl_aliquot_with_proportional_ions() {
    let mut scene = initial_bench_scene("lab-test");
    fill_pipette_from(&mut scene, "beaker-hcl");
    let stock = item(&scene, "beaker-hcl");
    assert!((hcl::solution_volume_ml(stock) - 9.0).abs() < 1e-6);
    assert!((aqueous_mol(stock, "h+") - HCL_STOCK_HCL_MOLES * 0.9).abs() < 1e-9);

    let pipette = item(&scene, "pipette-1");
    assert!((pipette_holding_liquid_ml(pipette) - PIPETTE_VOLUME_ML).abs() < 1e-6);
    assert!((aqueous_mol_holding(pipette, "h+") - HCL_STOCK_HCL_MOLES * 0.1).abs() < 1e-9);
    assert!((aqueous_mol_holding(pipette, "cl-") - HCL_STOCK_HCL_MOLES * 0.1).abs() < 1e-9);

    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "pipette-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();
    let water = item(&scene, "beaker-water");
    assert!((aqueous_mol(water, "h+") - HCL_STOCK_HCL_MOLES * 0.1).abs() < 1e-9);
    assert!(crate::hcl::ph_of_item(water).expect("dilute acid pH") < 1.0);
}

#[test]
fn pipette_put_back_stock_hcl_and_rejects_pure_water() {
    let mut scene = initial_bench_scene("lab-test");
    fill_pipette_from(&mut scene, "beaker-hcl");
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "pipette-1".into(),
            target_item_id: "beaker-hcl".into(),
        },
    )
    .unwrap();
    assert!((hcl::solution_volume_ml(item(&scene, "beaker-hcl")) - 10.0).abs() < 1e-6);

    fill_pipette_from(&mut scene, "beaker-h2o");
    let err = apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "pipette-1".into(),
            target_item_id: "beaker-hcl".into(),
        },
    )
    .unwrap_err();
    assert_eq!(err, SceneError::InvalidAction);
}

#[test]
fn pipette_rejects_hcl_put_back_into_distilled_water() {
    let mut scene = initial_bench_scene("lab-test");
    fill_pipette_from(&mut scene, "beaker-hcl");
    let err = apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "pipette-1".into(),
            target_item_id: "beaker-h2o".into(),
        },
    )
    .unwrap_err();
    assert_eq!(err, SceneError::InvalidAction);
}

#[test]
fn spoon_rejected_on_hcl_stock() {
    let mut scene = initial_bench_scene("lab-test");
    let err = apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-hcl".into(),
        },
    )
    .unwrap_err();
    assert_eq!(err, SceneError::InvalidAction);
}

#[test]
fn tongs_pour_hcl_into_water_and_put_back_stock() {
    let mut scene = initial_bench_scene("lab-test");
    use_tongs_pour(&mut scene, "beaker-hcl", "beaker-water");
    assert!(aqueous_mol(item(&scene, "beaker-water"), "h+") > 1e-6);
    assert!(hcl::solution_volume_ml(item(&scene, "beaker-hcl")) < 1e-6);

    // Pour the intact stock-composition acid back into the empty HCl stock.
    use_tongs_pour(&mut scene, "beaker-water", "beaker-hcl");
    assert!(
        (hcl::solution_volume_ml(item(&scene, "beaker-hcl")) - HCL_STOCK_CAPACITY_ML).abs() < 1e-4
    );
}

#[test]
fn diluting_hcl_into_water_raises_temperature() {
    let mut scene = initial_bench_scene("lab-test");
    fill_main_beaker(&mut scene, 50.0);
    let t0 = item(&scene, "beaker-water")
        .properties
        .temperature_c
        .unwrap_or(20.0);
    fill_pipette_from(&mut scene, "beaker-hcl");
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "pipette-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();
    let t1 = item(&scene, "beaker-water")
        .properties
        .temperature_c
        .unwrap_or(20.0);
    assert!(t1 > t0 + 0.01, "expected dilution heat, t0={t0} t1={t1}");
}

#[test]
fn salt_dissolves_into_dilute_hcl_preserving_h_and_cl() {
    let mut scene = initial_bench_scene("lab-test");
    fill_pipette_from(&mut scene, "beaker-hcl");
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "pipette-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();
    // Add more water so pipette min source is met for further work; dissolve via spoon.
    fill_main_beaker_keep_ions(&mut scene, 50.0);
    let n_h_before = aqueous_mol(item(&scene, "beaker-water"), "h+");
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
    assert!((aqueous_mol(water, "h+") - n_h_before).abs() < 1e-9);
    assert!(aqueous_mol(water, "na+") > 1e-6);
    assert!(aqueous_mol(water, "cl-") >= n_h_before + aqueous_mol(water, "na+") - 1e-9);
}

fn fill_main_beaker_keep_ions(scene: &mut Scene, target_water_ml: f64) {
    let water = scene
        .items
        .iter_mut()
        .find(|item| item.id == "beaker-water")
        .expect("beaker-water");
    let current = crate::solubility::liquid_water_ml(water);
    if let Some(entry) = water
        .properties
        .composition
        .iter_mut()
        .find(|c| c.substance_id == "water" && c.phase == "liquid")
    {
        entry.amount_ml = Some(target_water_ml.max(current));
    } else {
        water.properties.composition.push(CompositionEntry {
            substance_id: "water".into(),
            phase: "liquid".into(),
            amount_ml: Some(target_water_ml),
            amount_scoop: None,
            amount_g: None,
            amount_mol: None,
        });
    }
    crate::solubility::sync_fill_ml(water);
}

#[test]
fn dish_boil_with_hcl_uses_azeotrope_temperature_and_removes_acid() {
    let mut scene = initial_bench_scene("lab-test");
    // Move all HCl stock into the dish via tongs.
    use_tongs_pour(&mut scene, "beaker-hcl", "dish-1");

    let dish = item(&scene, "dish-1");
    assert!(hcl_w_w(dish) > 0.25); // stock is 30% w/w

    apply_action(
        &mut scene,
        Action::ToggleBurner {
            burner_item_id: "burner-1".into(),
        },
    )
    .unwrap();
    // Heat to the azeotrope plateau, then evaporate a short boil interval.
    apply_elapsed(&mut scene, 12.0);
    let dish = item(&scene, "dish-1");
    let t = dish.properties.temperature_c.unwrap_or(0.0);
    assert!(
        (t - HCL_AZEOTROPE_BOIL_C).abs() < 1.0,
        "expected azeotrope boil plateau near {HCL_AZEOTROPE_BOIL_C}, got {t}"
    );
    let n_h0 = aqueous_mol(dish, "h+");
    let water0 = water_ml(dish);
    let w0 = hcl_w_w(dish);
    apply_elapsed(&mut scene, 2.0);
    let dish = item(&scene, "dish-1");
    let n_h1 = aqueous_mol(dish, "h+");
    let water1 = water_ml(dish);
    assert!(n_h1 < n_h0, "HCl should leave with vapor");
    assert!(water1 < water0, "water should leave with vapor");
    let w1 = hcl_w_w(dish);
    // Concentrated stock (> azeotrope) should lean toward the azeotrope.
    assert!(
        w1 < w0 || (w1 - HCL_AZEOTROPE_W_W).abs() < (w0 - HCL_AZEOTROPE_W_W).abs(),
        "liquid should move toward azeotrope: w0={w0} w1={w1}"
    );
}

#[test]
fn challenge_scene_omits_hcl_stock() {
    let scene = initial_scene_for_mode("lab-test", "separate-nacl-sio2").unwrap();
    assert!(scene.items.iter().all(|i| i.id != "beaker-hcl"));
}

#[test]
fn hcl_common_ion_reduces_nacl_unsaturated_capacity() {
    let water_ml = 100.0;
    let t = 20.0;
    let pure = crate::solubility::unsaturated_capacity_g(
        crate::solubility::Salt::Nacl,
        water_ml,
        0.0,
        0.0,
        0.0,
        t,
    );
    let with_hcl = crate::solubility::unsaturated_capacity_g(
        crate::solubility::Salt::Nacl,
        water_ml,
        0.0,
        0.0,
        0.5, // 0.5 mol HCl in 0.1 L → 5 M Cl⁻
        t,
    );
    assert!(
        with_hcl < pure - 0.5,
        "common-ion should suppress NaCl: pure={pure} acid={with_hcl}"
    );
}

#[test]
fn stock_water_mass_matches_locked_constant() {
    let scene = initial_bench_scene("lab-test");
    assert!((water_ml(item(&scene, "beaker-hcl")) - HCL_STOCK_WATER_MASS_G).abs() < 1e-12);
}
