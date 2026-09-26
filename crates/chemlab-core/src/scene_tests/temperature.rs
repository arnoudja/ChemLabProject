use super::super::*;
use super::helpers::*;

fn dish_c_eff(water_ml: f64) -> f64 {
    C_DISH + water_ml * WATER_SPECIFIC_HEAT_J_PER_G_K
}

fn beaker_c_eff(water_ml: f64) -> f64 {
    C_BEAKER + water_ml * WATER_SPECIFIC_HEAT_J_PER_G_K
}

fn set_dish_water(scene: &mut Scene, ml: f64, temperature_c: f64) {
    let dish = scene.items.iter_mut().find(|i| i.id == "dish-1").unwrap();
    dish.properties.temperature_c = Some(temperature_c);
    dish.properties.composition = vec![CompositionEntry {
        substance_id: "water".into(),
        phase: "liquid".into(),
        amount_ml: Some(ml),
        amount_scoop: None,
        amount_g: None,
        amount_mol: None,
    }];
    crate::solubility::sync_fill_ml(dish);
}

#[test]
fn ambient_cool_moves_hot_dry_dish_toward_20_when_burner_off() {
    let mut scene = initial_bench_scene("lab-test");
    // Dry dish: Newton cool only (no evaporative latent).
    {
        let dish = scene.items.iter_mut().find(|i| i.id == "dish-1").unwrap();
        dish.properties.temperature_c = Some(80.0);
        dish.properties.composition.clear();
        crate::solubility::sync_fill_ml(dish);
    }
    assert_eq!(item(&scene, "burner-1").properties.on, Some(false));

    let c_eff = C_DISH;
    let expected = 80.0 - (UA_DISH / c_eff) * (80.0 - AMBIENT_TEMPERATURE_C) * 2.0;
    apply_elapsed(&mut scene, 2.0);
    let actual = item(&scene, "dish-1").properties.temperature_c.unwrap();
    assert!(
        (actual - expected).abs() < 1e-9,
        "expected {expected}, got {actual}"
    );
    assert!(actual < 80.0);
    assert!(actual > AMBIENT_TEMPERATURE_C);
}

#[test]
fn ambient_cool_snaps_near_ambient() {
    let mut scene = initial_bench_scene("lab-test");
    // Dry dish so ambient MT / latent does not pull T away from the snap.
    {
        let dish = scene.items.iter_mut().find(|i| i.id == "dish-1").unwrap();
        dish.properties.temperature_c = Some(20.004);
        dish.properties.composition.clear();
        crate::solubility::sync_fill_ml(dish);
    }
    apply_elapsed(&mut scene, 1.0);
    assert_eq!(
        item(&scene, "dish-1").properties.temperature_c,
        Some(AMBIENT_TEMPERATURE_C)
    );
}

#[test]
fn dish_does_not_ambient_cool_while_burner_heats() {
    let mut scene = initial_bench_scene("lab-test");
    set_dish_water(&mut scene, 5.0, 50.0);
    apply_action(
        &mut scene,
        Action::ToggleBurner {
            burner_item_id: "burner-1".into(),
        },
    )
    .unwrap();

    let c_eff = dish_c_eff(5.0);
    let expected = (50.0 + (BURNER_POWER_W / c_eff) * 1.0).min(boiling_temperature_c(1.0));
    apply_elapsed(&mut scene, 1.0);
    let actual = item(&scene, "dish-1").properties.temperature_c.unwrap();
    assert!((actual - expected).abs() < 1e-9);
    assert!(actual > 50.0);
    assert!(
        (water_ml(item(&scene, "dish-1")) - 5.0).abs() < 1e-9,
        "no mass-transfer while burner heats below boil"
    );
}

#[test]
fn other_vessels_still_cool_while_burner_heats_dish() {
    let mut scene = bench_with_water("lab-test");
    set_dish_water(&mut scene, 5.0, 40.0);
    {
        let water = scene
            .items
            .iter_mut()
            .find(|i| i.id == "beaker-water")
            .unwrap();
        water.properties.temperature_c = Some(60.0);
    }
    apply_action(
        &mut scene,
        Action::ToggleBurner {
            burner_item_id: "burner-1".into(),
        },
    )
    .unwrap();

    let c_water = beaker_c_eff(FILLED_MAIN_BEAKER_ML);
    let expected_water = 60.0 - (UA_BEAKER / c_water) * (60.0 - AMBIENT_TEMPERATURE_C) * 1.0;
    apply_elapsed(&mut scene, 1.0);
    let actual = item(&scene, "beaker-water")
        .properties
        .temperature_c
        .unwrap();
    assert!((actual - expected_water).abs() < 1e-9);
    assert!(item(&scene, "dish-1").properties.temperature_c.unwrap() > 40.0);
}

#[test]
fn capacity_aware_heat_reaches_100_slower_with_more_water() {
    let mut light = initial_bench_scene("lab-test");
    set_dish_water(&mut light, 1.0, 20.0);
    apply_action(
        &mut light,
        Action::ToggleBurner {
            burner_item_id: "burner-1".into(),
        },
    )
    .unwrap();

    let mut heavy = initial_bench_scene("lab-test");
    set_dish_water(&mut heavy, 20.0, 20.0);
    apply_action(
        &mut heavy,
        Action::ToggleBurner {
            burner_item_id: "burner-1".into(),
        },
    )
    .unwrap();

    // Short of boil for 1 ml at 1100 W (~6 s to T_boil); keep heat-up-only.
    let dt = 4.0;
    apply_elapsed(&mut light, dt);
    apply_elapsed(&mut heavy, dt);
    let t_light = item(&light, "dish-1").properties.temperature_c.unwrap();
    let t_heavy = item(&heavy, "dish-1").properties.temperature_c.unwrap();
    assert!(t_light > t_heavy + 1.0, "light={t_light} heavy={t_heavy}");

    let c_light = dish_c_eff(1.0);
    let expected_light = (20.0 + (BURNER_POWER_W / c_light) * dt).min(boiling_temperature_c(1.0));
    assert!((t_light - expected_light).abs() < 1e-9);
}

#[test]
fn energy_weighted_tongs_pour_includes_vessel_heat_capacity() {
    let mut scene = bench_with_water("lab-test");
    set_dish_water(&mut scene, 10.0, 80.0);

    use_tongs(&mut scene, "beaker-water").unwrap();
    use_tongs(&mut scene, "dish-1").unwrap();

    // 15 ml from water beaker at 20 °C into dish (10 ml @ 80 °C, C_dish body).
    let c_dest = dish_c_eff(10.0);
    let c_add = 15.0 * WATER_SPECIFIC_HEAT_J_PER_G_K;
    let expected = (c_dest * 80.0 + c_add * 20.0) / (c_dest + c_add);
    let actual = item(&scene, "dish-1").properties.temperature_c.unwrap();
    assert!(
        (actual - expected).abs() < 1e-9,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn energy_weighted_pipette_empty_includes_vessel_heat_capacity() {
    let mut scene = bench_with_water("lab-test");
    {
        let water = scene
            .items
            .iter_mut()
            .find(|i| i.id == "beaker-water")
            .unwrap();
        water.properties.temperature_c = Some(70.0);
    }
    set_dish_water(&mut scene, 5.0, 30.0);

    fill_pipette_from(&mut scene, "beaker-water");
    apply_action(
        &mut scene,
        Action::Pour {
            source_item_id: "pipette-1".into(),
            target_item_id: "dish-1".into(),
        },
    )
    .unwrap();

    let c_dest = dish_c_eff(5.0);
    let c_add = PIPETTE_VOLUME_ML * WATER_SPECIFIC_HEAT_J_PER_G_K;
    let expected = (c_dest * 30.0 + c_add * 70.0) / (c_dest + c_add);
    let actual = item(&scene, "dish-1").properties.temperature_c.unwrap();
    assert!((actual - expected).abs() < 1e-9);
}

#[test]
fn spoon_scoop_carries_source_temperature_into_cooler_target() {
    let mut scene = bench_with_water("lab-test");
    // Dry hot NaCl in the dish, then scoop into cooler water.
    {
        let dish = scene.items.iter_mut().find(|i| i.id == "dish-1").unwrap();
        dish.properties.temperature_c = Some(90.0);
        dish.properties.composition = vec![CompositionEntry {
            substance_id: "nacl".into(),
            phase: "solid".into(),
            amount_ml: None,
            amount_scoop: Some(5),
            amount_g: Some(1.0),
            amount_mol: None,
        }];
        crate::solubility::sync_fill_ml(dish);
    }

    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "dish-1".into(),
        },
    )
    .unwrap();
    assert_eq!(item(&scene, "spoon-1").properties.temperature_c, Some(90.0));

    let c_dest_before = beaker_c_eff(FILLED_MAIN_BEAKER_ML);
    let c_add = SPOON_SCOOP_MASS_G * CP_NACL;
    let t_after_blend = (c_dest_before * 20.0 + c_add * 90.0) / (c_dest_before + c_add);
    let moles = SPOON_SCOOP_MASS_G / NACL_MOLAR_MASS_G_PER_MOL;
    // After blend, dissolve ΔH uses C_eff of water + vessel (no solid term).
    let expected = t_after_blend - (moles * NACL_DELTA_H_SOLUTION_J_PER_MOL) / c_dest_before;

    apply_action(
        &mut scene,
        Action::Pour {
            source_item_id: "spoon-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();

    let actual = item(&scene, "beaker-water")
        .properties
        .temperature_c
        .unwrap();
    assert!(
        (actual - expected).abs() < 1e-9,
        "expected {expected}, got {actual}"
    );
    assert!(item(&scene, "spoon-1").properties.temperature_c.is_none());
}

#[test]
fn dissolve_delta_t_uses_vessel_heat_capacity() {
    let mut scene = bench_with_water("lab-test");
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-nacl".into(),
        },
    )
    .unwrap();
    // Stock scoop is at ambient; blend with ambient solid is a no-op for T.
    apply_action(
        &mut scene,
        Action::Pour {
            source_item_id: "spoon-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();

    let moles = SPOON_SCOOP_MASS_G / NACL_MOLAR_MASS_G_PER_MOL;
    let c_eff = beaker_c_eff(FILLED_MAIN_BEAKER_ML);
    let expected = 20.0 - (moles * NACL_DELTA_H_SOLUTION_J_PER_MOL) / c_eff;
    let actual = item(&scene, "beaker-water")
        .properties
        .temperature_c
        .unwrap();
    assert!((actual - expected).abs() < 1e-9);
}
