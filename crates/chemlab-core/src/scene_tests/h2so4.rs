//! Free-mode pure liquid H₂SO₄: stock, ionization, dilution, dish, reactions, SI.

use super::super::*;
use super::helpers::*;
use crate::h2so4::{self, H2SO4_STOCK_CAPACITY_ML, H2SO4_STOCK_H2SO4_MOLES};
use crate::hcl::{self, hcl_inventory_moles_entries};

#[test]
fn pipette_into_water_ionizes_to_two_h_and_so4() {
    let mut scene = initial_bench_scene("lab-test");
    fill_main_beaker(&mut scene, 20.0);
    fill_pipette_from(&mut scene, "beaker-h2so4");
    let n_added = H2SO4_STOCK_H2SO4_MOLES * 0.1;
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "pipette-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();
    let water = item(&scene, "beaker-water");
    assert!((aqueous_mol(water, "so4^2-") - n_added).abs() < 1e-9);
    assert!((aqueous_mol(water, "h+") - 2.0 * n_added).abs() < 1e-9);
    assert!(aqueous_mol(water, "cl-") < 1e-15);
    assert!(hcl_inventory_moles_entries(&water.properties.composition) < 1e-15);
    assert!(crate::hcl::ph_of_item(water).expect("acid pH") < 1.0);
    // Dilution of concentrated acid is exothermic.
    let t = water.properties.temperature_c.unwrap_or(20.0);
    assert!(t > 20.5, "expected dilution warming, got {t}");
}

#[test]
fn pipette_put_back_stock_h2so4_and_rejects_water() {
    let mut scene = initial_bench_scene("lab-test");
    fill_pipette_from(&mut scene, "beaker-h2so4");
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "pipette-1".into(),
            target_item_id: "beaker-h2so4".into(),
        },
    )
    .unwrap();
    assert!((hcl::solution_volume_ml(item(&scene, "beaker-h2so4")) - 10.0).abs() < 1e-6);

    fill_pipette_from(&mut scene, "beaker-h2o");
    let err = apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "pipette-1".into(),
            target_item_id: "beaker-h2so4".into(),
        },
    )
    .unwrap_err();
    assert_eq!(err, SceneError::InvalidAction);
}

#[test]
fn spoon_rejected_on_h2so4_stock() {
    let mut scene = initial_bench_scene("lab-test");
    let err = apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-h2so4".into(),
        },
    )
    .unwrap_err();
    assert_eq!(err, SceneError::InvalidAction);
}

#[test]
fn sulfuric_only_dish_evap_removes_water_keeps_acid() {
    let mut scene = initial_bench_scene("lab-test");
    fill_main_beaker(&mut scene, 15.0);
    fill_pipette_from(&mut scene, "beaker-h2so4");
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "pipette-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();
    use_tongs_pour(&mut scene, "beaker-water", "dish-1");

    let dish = item(&scene, "dish-1");
    let n_h0 = aqueous_mol(dish, "h+");
    let n_so4_0 = aqueous_mol(dish, "so4^2-");
    assert!(n_h0 > 1e-6 && n_so4_0 > 1e-6);
    assert!(hcl_inventory_moles_entries(&dish.properties.composition) < 1e-15);

    apply_action(
        &mut scene,
        Action::ToggleBurner {
            burner_item_id: "burner-1".into(),
        },
    )
    .unwrap();
    apply_elapsed(&mut scene, 20.0);
    let dish = item(&scene, "dish-1");
    let n_h1 = aqueous_mol(dish, "h+");
    let n_so4_1 = aqueous_mol(dish, "so4^2-");
    assert!(
        (n_h1 - n_h0).abs() < 1e-9,
        "H₂SO₄ must be non-volatile: h+ {n_h0} → {n_h1}"
    );
    assert!((n_so4_1 - n_so4_0).abs() < 1e-9);
    assert!(water_ml(dish) < 15.0, "water should leave");
}

#[test]
fn neutralize_h2so4_with_naoh_two_to_one() {
    let mut scene = initial_bench_scene("lab-test");
    fill_main_beaker(&mut scene, 50.0);
    // Add ~0.05 mol H₂SO₄ via tongs fraction of stock.
    use_tongs_pour(&mut scene, "beaker-h2so4", "beaker-water");
    let water = item(&scene, "beaker-water");
    let n_h2so4 = aqueous_mol(water, "so4^2-");
    assert!(n_h2so4 > 0.1);

    // Dump all NaOH (2.00 g = 0.05 mol) — not enough for full neut of full stock.
    use_tongs_pour(&mut scene, "beaker-naoh", "beaker-water");
    let water = item(&scene, "beaker-water");
    let n_h = aqueous_mol(water, "h+");
    let n_oh = aqueous_mol(water, "oh-");
    let n_na = aqueous_mol(water, "na+");
    let n_so4 = aqueous_mol(water, "so4^2-");
    assert!(n_oh < 1e-9, "OH should be consumed");
    // 0.05 mol NaOH neutralizes 0.05 mol H⁺ → remaining H⁺ = 2*n_h2so4 - 0.05
    let expected_h = (2.0 * n_so4 - n_na).max(0.0);
    assert!(
        (n_h - expected_h).abs() < 1e-6,
        "stoich H left: got {n_h} expected ~{expected_h}"
    );
    assert!((n_na - 0.05).abs() < 1e-6);
}

#[test]
fn cacl2_plus_h2so4_precipitates_gypsum() {
    let mut scene = initial_bench_scene("lab-test");
    fill_main_beaker(&mut scene, 50.0);
    // Small H₂SO₄ aliquot
    fill_pipette_from(&mut scene, "beaker-h2so4");
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "pipette-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();
    // Scoop CaCl₂ into acidic water
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-cacl2".into(),
        },
    )
    .unwrap();
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();

    let water = item(&scene, "beaker-water");
    let solid_caso4 = water
        .properties
        .composition
        .iter()
        .find(|c| c.substance_id == "caso4" && c.phase == "solid")
        .and_then(|c| c.amount_mol)
        .unwrap_or(0.0);
    assert!(
        solid_caso4 > 1e-6,
        "expected CaSO₄(s) gypsum precipitate, got {solid_caso4}"
    );
    // Liberated Cl⁻ remains with H⁺ as HCl inventory.
    let n_hcl = hcl_inventory_moles_entries(&water.properties.composition);
    assert!(n_hcl > 1e-6, "expected HCl inventory after gypsum");
}

#[test]
fn sand_inert_with_h2so4() {
    let mut scene = initial_bench_scene("lab-test");
    fill_main_beaker(&mut scene, 20.0);
    fill_pipette_from(&mut scene, "beaker-h2so4");
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "pipette-1".into(),
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
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();
    let water = item(&scene, "beaker-water");
    let sand_g = water
        .properties
        .composition
        .iter()
        .find(|c| c.substance_id == "sand" && c.phase == "solid")
        .and_then(|c| c.amount_g)
        .unwrap_or(0.0);
    assert!((sand_g - SPOON_SCOOP_MASS_G).abs() < 1e-9);
    assert!((aqueous_mol(water, "so4^2-") - H2SO4_STOCK_H2SO4_MOLES * 0.1).abs() < 1e-9);
}

#[test]
fn so4_survives_si_without_inventing_cl() {
    let mut scene = initial_bench_scene("lab-test");
    fill_main_beaker(&mut scene, 100.0);
    fill_pipette_from(&mut scene, "beaker-h2so4");
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "pipette-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();
    let water = item(&scene, "beaker-water");
    assert!(aqueous_mol(water, "so4^2-") > 1e-6);
    assert!(aqueous_mol(water, "cl-") < 1e-15);
    assert!((hcl::solution_volume_ml(water) - H2SO4_STOCK_CAPACITY_ML).abs() > 1.0);
}

#[test]
fn challenges_omit_h2so4_stock() {
    let scene = initial_scene_for_mode("lab-test", "separate-nacl-sio2").unwrap();
    assert!(!scene.items.iter().any(|i| i.id == "beaker-h2so4"));
    let scene = initial_scene_for_mode("lab-test", "create-table-salt").unwrap();
    assert!(!scene.items.iter().any(|i| i.id == "beaker-h2so4"));
}

#[test]
fn stock_volume_is_ten_ml() {
    let scene = initial_bench_scene("lab-test");
    let stock = item(&scene, "beaker-h2so4");
    assert!((hcl::solution_volume_ml(stock) - H2SO4_STOCK_CAPACITY_ML).abs() < 1e-9);
    assert!(h2so4::composition_is_stock_h2so4(
        &stock.properties.composition
    ));
}
