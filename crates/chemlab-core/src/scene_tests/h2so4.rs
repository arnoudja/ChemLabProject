//! Free-mode pure liquid H₂SO₄: stock, ionization, dilution, dish, reactions, SI.

use super::super::*;
use super::helpers::*;
use crate::h2so4::{
    self, H2SO4_REFORM_WATER_PER_ACID, H2SO4_STOCK_CAPACITY_ML, H2SO4_STOCK_H2SO4_MOLES,
};
use crate::hcl::{self, hcl_inventory_moles_entries};

#[test]
fn pipette_into_water_ionizes_with_ka2_speciation() {
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
    let n_s = aqueous_mol(water, "so4^2-") + aqueous_mol(water, "hso4-");
    assert!(
        (n_s - n_added).abs() < 1e-9,
        "sulfur conserved: {n_s} vs {n_added}"
    );
    let n_h = aqueous_mol(water, "h+");
    // Ka2 regime: [H+] between first-proton and fully dissociated (not 2×c).
    assert!(
        n_h > n_added && n_h < 1.5 * n_added,
        "got n_h={n_h}, n_added={n_added}"
    );
    assert!(aqueous_mol(water, "hso4-") > aqueous_mol(water, "so4^2-"));
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
    let n_acid0 = h2so4::H2so4Inventory::from_item(dish).n_h2so4;
    assert!(n_acid0 > 1e-6);
    assert!(hcl_inventory_moles_entries(&dish.properties.composition) < 1e-15);
    // Dilute enough that a short boil stays above the reform threshold.
    assert!(
        h2so4::H2so4Inventory::from_item(dish).water_per_h2so4()
            > H2SO4_REFORM_WATER_PER_ACID * 10.0
    );

    apply_action(
        &mut scene,
        Action::ToggleBurner {
            burner_item_id: "burner-1".into(),
        },
    )
    .unwrap();
    apply_elapsed(&mut scene, 20.0);
    let dish = item(&scene, "dish-1");
    let n_acid1 = h2so4::H2so4Inventory::from_item(dish).n_h2so4;
    assert!(
        (n_acid1 - n_acid0).abs() < 1e-9,
        "H₂SO₄ must be non-volatile: acid {n_acid0} → {n_acid1}"
    );
    assert!(water_ml(dish) < 15.0, "water should leave");
    // Still dilute → stays aqueous ions (SO₄²⁻ and/or HSO₄⁻).
    let n_s = aqueous_mol(dish, "so4^2-") + aqueous_mol(dish, "hso4-");
    assert!(n_s > 1e-6);
    assert!(h2so4::liquid_h2so4_mol_entries(&dish.properties.composition) < 1e-15);
}

#[test]
fn neutralize_h2so4_with_naoh_two_to_one() {
    let mut scene = initial_bench_scene("lab-test");
    fill_main_beaker(&mut scene, 50.0);
    // Add ~0.05 mol H₂SO₄ via tongs fraction of stock.
    use_tongs_pour(&mut scene, "beaker-h2so4", "beaker-water");
    let water = item(&scene, "beaker-water");
    let n_h2so4 = aqueous_mol(water, "so4^2-") + aqueous_mol(water, "hso4-");
    assert!(n_h2so4 > 0.1);

    // Dump all NaOH (2.00 g = 0.05 mol) — not enough for full neut of full stock.
    use_tongs_pour(&mut scene, "beaker-naoh", "beaker-water");
    let water = item(&scene, "beaker-water");
    let n_h = aqueous_mol(water, "h+");
    let n_oh = aqueous_mol(water, "oh-");
    let n_na = aqueous_mol(water, "na+");
    let n_s = aqueous_mol(water, "so4^2-") + aqueous_mol(water, "hso4-");
    let n_hso4 = aqueous_mol(water, "hso4-");
    assert!(
        n_oh < 1e-6,
        "OH should be consumed (Kw residual OK), got {n_oh}"
    );
    // Charge/mass: acidic hydrogens left ≈ 2*n_S − n_Na (school stoich after Ka2).
    let acid_h = n_h + n_hso4;
    let expected_acid_h = (2.0 * n_s - n_na).max(0.0);
    assert!(
        (acid_h - expected_acid_h).abs() < 1e-5,
        "stoich acid H left: got {acid_h} expected ~{expected_acid_h}"
    );
    assert!((n_na - 0.05).abs() < 1e-6);
    assert!(crate::acid_base::aqueous_charge_mol(water).abs() < 1e-6);
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
    let n_s = aqueous_mol(water, "so4^2-") + aqueous_mol(water, "hso4-");
    assert!((n_s - H2SO4_STOCK_H2SO4_MOLES * 0.1).abs() < 1e-9);
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
    let n_s = aqueous_mol(water, "so4^2-") + aqueous_mol(water, "hso4-");
    assert!(n_s > 1e-6);
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

fn set_dish_water_ml(scene: &mut Scene, water_ml: f64) {
    let dish = scene
        .items
        .iter_mut()
        .find(|i| i.id == "dish-1")
        .expect("dish-1");
    dish.properties
        .composition
        .retain(|c| !(c.substance_id == "water" && c.phase == "liquid"));
    if water_ml > 1e-15 {
        dish.properties.composition.push(CompositionEntry {
            substance_id: "water".into(),
            phase: "liquid".into(),
            amount_ml: Some(water_ml),
            amount_scoop: None,
            amount_g: None,
            amount_mol: None,
        });
    }
    crate::solubility::sync_fill_ml(dish);
}

#[test]
fn dish_concentrate_below_threshold_reforms_liquid_h2so4() {
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

    let n_acid = h2so4::H2so4Inventory::from_item(item(&scene, "dish-1")).n_h2so4;
    let w_below = H2SO4_REFORM_WATER_PER_ACID * n_acid * WATER_MOLAR_MASS_G_PER_MOL * 0.5;
    set_dish_water_ml(&mut scene, w_below);
    // Tiny elapsed tick runs finalize (ionize → neut → SI → reform).
    apply_elapsed(&mut scene, 1e-6);

    let dish = item(&scene, "dish-1");
    assert!(
        (h2so4::liquid_h2so4_mol_entries(&dish.properties.composition) - n_acid).abs() < 1e-9,
        "expected liquid H₂SO₄, got {}",
        h2so4::liquid_h2so4_mol_entries(&dish.properties.composition)
    );
    assert!(aqueous_mol(dish, "so4^2-") < 1e-12);
    assert!(
        aqueous_mol(dish, "h+") < 1e-9,
        "free H⁺ after reform should be Kw-scale at most, got {}",
        aqueous_mol(dish, "h+")
    );
    // Reform changes Φ_V (aq ions → liquid h2so4); finalize must re-sync fill_ml.
    let fill = dish.properties.fill_ml.unwrap_or(0.0);
    let phi_v = crate::hcl::solution_volume_ml(dish);
    assert!(
        (fill - phi_v).abs() < 1e-9,
        "post-reform fill_ml desync: fill_ml={fill} Φ_V={phi_v}"
    );
    assert_fill_ml_matches_phi_v(&scene);
}

#[test]
fn dish_dry_out_conserves_acid_as_liquid_h2so4() {
    let mut scene = initial_bench_scene("lab-test");
    fill_main_beaker(&mut scene, 10.0);
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
    let n_acid = h2so4::H2so4Inventory::from_item(item(&scene, "dish-1")).n_h2so4;

    // Reform while a trace of water remains (finalize only runs when dish_has_liquid).
    let w_below = H2SO4_REFORM_WATER_PER_ACID * n_acid * WATER_MOLAR_MASS_G_PER_MOL * 0.25;
    set_dish_water_ml(&mut scene, w_below);
    apply_elapsed(&mut scene, 1e-6);
    assert!(
        (h2so4::liquid_h2so4_mol_entries(&item(&scene, "dish-1").properties.composition) - n_acid)
            .abs()
            < 1e-9
    );

    // Strip residual water — liquid acid still counts as dish liquid; moles conserved.
    set_dish_water_ml(&mut scene, 0.0);
    apply_elapsed(&mut scene, 1e-6);

    let dish = item(&scene, "dish-1");
    assert!(water_ml(dish) < 1e-12);
    assert!((h2so4::liquid_h2so4_mol_entries(&dish.properties.composition) - n_acid).abs() < 1e-9);
    assert!(aqueous_mol(dish, "so4^2-") < 1e-12);
    assert!(h2so4::composition_is_stock_h2so4(
        &dish.properties.composition
    ));
    assert!(crate::solubility::dish_has_liquid(dish));
}

#[test]
fn burner_turns_off_when_dish_water_gone_even_if_liquid_h2so4_remains() {
    // Dilute H₂SO₄ + burner on → evaporate until no liquid water → burner off.
    // Leftover liquid molecular H₂SO₄ must not keep the heater on.
    let mut scene = initial_bench_scene("lab-test");
    fill_main_beaker(&mut scene, 4.0);
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
    let n_acid0 = h2so4::H2so4Inventory::from_item(item(&scene, "dish-1")).n_h2so4;
    assert!(n_acid0 > 1e-6);
    assert!(water_ml(item(&scene, "dish-1")) > 1.0);

    apply_action(
        &mut scene,
        Action::ToggleBurner {
            burner_item_id: "burner-1".into(),
        },
    )
    .unwrap();
    assert_eq!(item(&scene, "burner-1").properties.on, Some(true));

    for _ in 0..20_000 {
        if water_ml(item(&scene, "dish-1")) < 1e-9 {
            break;
        }
        apply_elapsed(&mut scene, 0.5);
    }

    let dish = item(&scene, "dish-1");
    assert!(
        water_ml(dish) < 1e-9,
        "expected water fully evaporated, got {}",
        water_ml(dish)
    );
    let n_liquid = h2so4::liquid_h2so4_mol_entries(&dish.properties.composition);
    assert!(
        n_liquid > 1e-6,
        "liquid H₂SO₄ should remain after dry-out, got {n_liquid}"
    );
    assert!(
        (h2so4::H2so4Inventory::from_item(dish).n_h2so4 - n_acid0).abs() < 1e-9,
        "acid inventory must be conserved"
    );
    assert_eq!(
        item(&scene, "burner-1").properties.on,
        Some(false),
        "burner must auto-off when liquid water is gone (liquid H₂SO₄ alone must not keep it on)"
    );
}

#[test]
fn hcl_plus_h2so4_concentrate_reforms_only_sulfuric() {
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
    fill_pipette_from(&mut scene, "beaker-hcl");
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
    let n_h2so4 = h2so4::aqueous_h2so4_moles_entries(&dish.properties.composition);
    let n_hcl = hcl_inventory_moles_entries(&dish.properties.composition);
    assert!(n_h2so4 > 1e-6 && n_hcl > 1e-6);

    let w_below = H2SO4_REFORM_WATER_PER_ACID * n_h2so4 * WATER_MOLAR_MASS_G_PER_MOL * 0.5;
    set_dish_water_ml(&mut scene, w_below);
    apply_elapsed(&mut scene, 1e-6);

    let dish = item(&scene, "dish-1");
    assert!((h2so4::liquid_h2so4_mol_entries(&dish.properties.composition) - n_h2so4).abs() < 1e-9);
    assert!((hcl_inventory_moles_entries(&dish.properties.composition) - n_hcl).abs() < 1e-9);
    assert!(aqueous_mol(dish, "so4^2-") < 1e-12);
    assert!((aqueous_mol(dish, "h+") - n_hcl).abs() < 1e-9);
    assert!((aqueous_mol(dish, "cl-") - n_hcl).abs() < 1e-9);
}

#[test]
fn reformed_liquid_redilutes_and_ionizes() {
    let mut scene = initial_bench_scene("lab-test");
    // Dry-reformed pure acid in the dish.
    {
        let dish = scene
            .items
            .iter_mut()
            .find(|i| i.id == "dish-1")
            .expect("dish-1");
        let n = H2SO4_STOCK_H2SO4_MOLES * 0.1;
        dish.properties.composition = vec![CompositionEntry {
            substance_id: "h2so4".into(),
            phase: "liquid".into(),
            amount_ml: Some(h2so4::liquid_h2so4_ml_from_moles(n)),
            amount_scoop: None,
            amount_g: Some(n * h2so4::H2SO4_MOLAR_MASS_G_PER_MOL),
            amount_mol: Some(n),
        }];
        crate::solubility::sync_fill_ml(dish);
    }
    fill_main_beaker(&mut scene, 25.0);
    use_tongs_pour(&mut scene, "beaker-water", "dish-1");

    let dish = item(&scene, "dish-1");
    let n = H2SO4_STOCK_H2SO4_MOLES * 0.1;
    assert!(h2so4::liquid_h2so4_mol_entries(&dish.properties.composition) < 1e-15);
    let n_s = aqueous_mol(dish, "so4^2-") + aqueous_mol(dish, "hso4-");
    assert!((n_s - n).abs() < 1e-9);
    let n_h = aqueous_mol(dish, "h+");
    assert!(n_h > n && n_h < 1.5 * n, "Ka2 regime n_h={n_h}");
    let t = dish.properties.temperature_c.unwrap_or(20.0);
    assert!(t > 20.5, "re-dilution should warm, got {t}");
}
