use super::super::*;
use super::helpers::*;

fn dish_c_eff_for_test(water_ml: f64) -> f64 {
    C_DISH + water_ml * WATER_SPECIFIC_HEAT_J_PER_G_K
}

#[test]
fn burner_heats_to_100_then_evaporates_precipitates_and_turns_off() {
    let mut scene = initial_bench_scene("lab-test");
    let dish = scene.items.iter_mut().find(|i| i.id == "dish-1").unwrap();
    dish.properties.temperature_c = Some(20.0);
    dish.properties.composition = vec![
        CompositionEntry {
            substance_id: "water".into(),
            phase: "liquid".into(),
            amount_ml: Some(2.0),
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
    ];
    crate::solubility::enforce_saturation(dish);

    apply_action(
        &mut scene,
        Action::ToggleBurner {
            burner_item_id: "burner-1".into(),
        },
    )
    .unwrap();
    assert_eq!(item(&scene, "burner-1").properties.on, Some(true));

    // Capacity-aware: C_eff = C_DISH + 2·c_p → ~88.4 J/K; ~88 s to reach 100 °C.
    let heat_s = (BOILING_TEMPERATURE_C - 20.0) * dish_c_eff_for_test(2.0) / BURNER_POWER_W + 1.0;
    apply_elapsed(&mut scene, heat_s);
    let dish = item(&scene, "dish-1");
    assert!((dish.properties.temperature_c.unwrap() - 100.0).abs() < 1e-9);
    assert!(
        (water_ml(dish) - 2.0).abs() < 1e-9,
        "no evaporation below 100"
    );

    apply_elapsed(&mut scene, 2.0);
    let dish = item(&scene, "dish-1");
    assert!((water_ml(dish) - 1.0).abs() < 1e-9);
    assert_eq!(dish.properties.temperature_c, Some(100.0));
    assert!(
        dish.properties
            .composition
            .iter()
            .any(|c| c.substance_id == "nacl" && c.phase == "solid"),
        "1 ml cannot hold 0.02 mol NaCl; solid must appear"
    );
    let max_aq =
        crate::solubility::solubility_mol_per_l(crate::solubility::Salt::Nacl, 100.0) * 0.001;
    assert!((aqueous_mol(dish, "na+") - max_aq).abs() < 1e-9);

    apply_elapsed(&mut scene, 2.0);
    let dish = item(&scene, "dish-1");
    assert!(water_ml(dish) < 1e-9);
    assert_eq!(item(&scene, "burner-1").properties.on, Some(false));
    assert!(aqueous_mol(dish, "na+") < 1e-12);
    assert!(dish
        .properties
        .composition
        .iter()
        .any(|c| c.substance_id == "nacl" && c.phase == "solid"));
}

#[test]
fn heating_below_boil_redissolves_nacl_as_solubility_rises() {
    let mut scene = initial_bench_scene("lab-test");
    let dish = scene.items.iter_mut().find(|i| i.id == "dish-1").unwrap();
    dish.properties.temperature_c = Some(20.0);
    dish.properties.composition = vec![
        CompositionEntry {
            substance_id: "water".into(),
            phase: "liquid".into(),
            amount_ml: Some(1.0),
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
    ];
    crate::solubility::enforce_saturation(dish);
    let dish = item(&scene, "dish-1");
    let na_before = aqueous_mol(dish, "na+");
    let solid_before = dish
        .properties
        .composition
        .iter()
        .find(|c| c.substance_id == "nacl" && c.phase == "solid")
        .and_then(|c| c.amount_mol)
        .unwrap_or(0.0);
    assert!(solid_before > 1e-6, "need leftover solid at 20 °C");

    apply_action(
        &mut scene,
        Action::ToggleBurner {
            burner_item_id: "burner-1".into(),
        },
    )
    .unwrap();
    // Solid NaCl from saturation adds thermal mass and redissolves as T rises, so
    // heat in small steps until ~60 °C rather than a closed-form empty-dish time.
    for _ in 0..500 {
        let t = item(&scene, "dish-1").properties.temperature_c.unwrap();
        if t >= 60.0 - 1e-6 {
            break;
        }
        apply_elapsed(&mut scene, 0.25);
    }

    let dish = item(&scene, "dish-1");
    assert!(
        (dish.properties.temperature_c.unwrap() - 60.0).abs() < 0.5,
        "expected near 60 °C, got {}",
        dish.properties.temperature_c.unwrap()
    );
    assert!(
        (water_ml(dish) - 1.0).abs() < 1e-9,
        "no evaporation below 100"
    );
    let na_after = aqueous_mol(dish, "na+");
    let solid_after = dish
        .properties
        .composition
        .iter()
        .find(|c| c.substance_id == "nacl" && c.phase == "solid")
        .and_then(|c| c.amount_mol)
        .unwrap_or(0.0);
    assert!(
        na_after > na_before + 1e-6,
        "heating must redissolve NaCl as s(T) rises; {na_after} vs {na_before}"
    );
    assert!(solid_after < solid_before - 1e-6);
    let t_now = dish.properties.temperature_c.unwrap();
    let max_now =
        crate::solubility::solubility_mol_per_l(crate::solubility::Salt::Nacl, t_now) * 0.001;
    assert!((na_after - max_now).abs() < 1e-9);
}

#[test]
fn evaporating_mixed_dish_precipitates_nacl_before_independent_caps() {
    let mut scene = initial_bench_scene("lab-test");
    let s_nacl = crate::solubility::solubility_mol_per_l(crate::solubility::Salt::Nacl, 100.0);
    let s_cacl2 = crate::solubility::solubility_mol_per_l(crate::solubility::Salt::Cacl2, 100.0);
    let n_nacl = s_nacl * 0.003;
    let n_cacl2 = s_cacl2 * 0.003;
    let dish = scene.items.iter_mut().find(|i| i.id == "dish-1").unwrap();
    dish.properties.temperature_c = Some(100.0);
    dish.properties.composition = vec![
        CompositionEntry {
            substance_id: "water".into(),
            phase: "liquid".into(),
            amount_ml: Some(15.0),
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
            amount_mol: Some(n_nacl),
        },
        CompositionEntry {
            substance_id: "ca2+".into(),
            phase: "aqueous".into(),
            amount_ml: None,
            amount_scoop: None,
            amount_g: None,
            amount_mol: Some(n_cacl2),
        },
        CompositionEntry {
            substance_id: "cl-".into(),
            phase: "aqueous".into(),
            amount_ml: None,
            amount_scoop: None,
            amount_g: None,
            amount_mol: Some(n_nacl + 2.0 * n_cacl2),
        },
    ];
    crate::solubility::enforce_saturation(dish);
    assert!(!item(&scene, "dish-1")
        .properties
        .composition
        .iter()
        .any(|c| c.substance_id == "nacl" && c.phase == "solid"));

    apply_action(
        &mut scene,
        Action::ToggleBurner {
            burner_item_id: "burner-1".into(),
        },
    )
    .unwrap();
    apply_elapsed(&mut scene, 26.0);
    let dish = item(&scene, "dish-1");
    assert!((water_ml(dish) - 2.0).abs() < 1e-9);
    assert!(
        dish.properties
            .composition
            .iter()
            .any(|c| c.substance_id == "nacl" && c.phase == "solid"),
        "mixed evaporation must crash out NaCl"
    );
    let na = aqueous_mol(dish, "na+");
    let ca = aqueous_mol(dish, "ca2+");
    let ind_nacl = s_nacl * 0.002;
    let ind_cacl2 = s_cacl2 * 0.002;
    assert!(na < ind_nacl - 1e-6);
    assert!((na - ind_nacl).abs() > 1e-6 || (ca - ind_cacl2).abs() > 1e-6);
}

#[test]
fn toggle_burner_stays_off_when_dish_has_no_liquid() {
    let mut scene = initial_bench_scene("lab-test");
    apply_action(
        &mut scene,
        Action::ToggleBurner {
            burner_item_id: "burner-1".into(),
        },
    )
    .unwrap();
    assert_eq!(item(&scene, "burner-1").properties.on, Some(false));
    assert!(scene.last_events.is_empty());
}

#[test]
fn reset_clears_dish_burner_and_pipette() {
    let mut scene = bench_with_water("lab-test");
    fill_pipette_from(&mut scene, "beaker-water");
    apply_action(
        &mut scene,
        Action::Pour {
            source_item_id: "pipette-1".into(),
            target_item_id: "dish-1".into(),
        },
    )
    .unwrap();
    apply_action(
        &mut scene,
        Action::ToggleBurner {
            burner_item_id: "burner-1".into(),
        },
    )
    .unwrap();
    apply_action(&mut scene, Action::Reset).unwrap();
    assert!(item(&scene, "dish-1").properties.composition.is_empty());
    assert_eq!(
        item(&scene, "dish-1").properties.temperature_c,
        Some(AMBIENT_TEMPERATURE_C)
    );
    assert_eq!(item(&scene, "burner-1").properties.on, Some(false));
    assert!(item(&scene, "pipette-1").properties.holding.is_empty());
    assert_eq!(item(&scene, "pipette-1").location, "bench");
    assert!((water_ml(item(&scene, "beaker-water")) - 0.0).abs() < 1e-9);
}
