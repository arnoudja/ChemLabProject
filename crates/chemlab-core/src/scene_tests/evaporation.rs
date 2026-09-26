use super::super::*;
use super::helpers::*;

fn dish_c_eff_for_test(water_ml: f64) -> f64 {
    C_DISH + water_ml * WATER_SPECIFIC_HEAT_J_PER_G_K
}

fn heat_limited_evap_ml_per_s() -> f64 {
    BURNER_POWER_W / WATER_LATENT_HEAT_J_PER_G
}

#[test]
fn pure_water_boiling_point_near_100_at_one_bar() {
    let t = boiling_temperature_c(1.0);
    assert!(
        (t - BOILING_TEMPERATURE_C).abs() < 0.05,
        "pure-water T_boil {t} vs const {BOILING_TEMPERATURE_C}"
    );
    assert!((t - 100.0).abs() < 0.5, "expected ≈100 °C, got {t}");
    let p = water_mole_fraction_pressure_at(t, 1.0);
    assert!((p - ATM_PRESSURE_BAR).abs() < 1e-3, "p_w {p}");
}

fn water_mole_fraction_pressure_at(t_c: f64, x_w: f64) -> f64 {
    x_w * water_vapor_pressure_bar(t_c)
}

#[test]
fn dissolved_salt_elevates_boiling_point_sand_does_not() {
    let mut scene = initial_bench_scene("lab-test");
    let dish = scene.items.iter_mut().find(|i| i.id == "dish-1").unwrap();
    dish.properties.temperature_c = Some(20.0);
    dish.properties.composition = vec![
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
            amount_mol: Some(0.05),
        },
        CompositionEntry {
            substance_id: "cl-".into(),
            phase: "aqueous".into(),
            amount_ml: None,
            amount_scoop: None,
            amount_g: None,
            amount_mol: Some(0.05),
        },
        CompositionEntry {
            substance_id: "sand".into(),
            phase: "solid".into(),
            amount_ml: None,
            amount_scoop: Some(5),
            amount_g: Some(1.0),
            amount_mol: None,
        },
    ];
    crate::solubility::enforce_saturation(dish);
    let dish = item(&scene, "dish-1");
    let x_w = water_mole_fraction(dish);
    assert!(x_w < 1.0 - 1e-6, "ions must lower x_w, got {x_w}");
    let t_boil = boiling_temperature_c(x_w);
    assert!(
        t_boil > BOILING_TEMPERATURE_C + 0.2,
        "salt must elevate T_boil: {t_boil} vs {BOILING_TEMPERATURE_C}"
    );
    // Sand is solid — excluding it must match ions-only x_w.
    let n_water = water_ml(dish) / WATER_MOLAR_MASS_G_PER_MOL;
    let n_ions = aqueous_mol(dish, "na+") + aqueous_mol(dish, "cl-");
    let x_ions_only = n_water / (n_water + n_ions);
    assert!((x_w - x_ions_only).abs() < 1e-9);
}

#[test]
fn ambient_evaporation_loses_water_with_burner_off() {
    let mut scene = initial_bench_scene("lab-test");
    let dish = scene.items.iter_mut().find(|i| i.id == "dish-1").unwrap();
    dish.properties.temperature_c = Some(AMBIENT_TEMPERATURE_C);
    dish.properties.composition = vec![CompositionEntry {
        substance_id: "water".into(),
        phase: "liquid".into(),
        amount_ml: Some(5.0),
        amount_scoop: None,
        amount_g: None,
        amount_mol: None,
    }];
    crate::solubility::sync_fill_ml(dish);
    assert_eq!(item(&scene, "burner-1").properties.on, Some(false));

    // ~1 h of ambient drying in 1 s ticks (lab-scale ≈ 2 ml/h pure water).
    for _ in 0..3600 {
        apply_elapsed(&mut scene, 1.0);
    }
    let lost = 5.0 - water_ml(item(&scene, "dish-1"));
    assert!(
        lost > 1.0 && lost < 4.0,
        "expected ~2 ml/h ambient loss, lost {lost} ml"
    );
}

#[test]
fn latent_heat_cools_dish_on_sub_boil_mass_loss() {
    let mut scene = initial_bench_scene("lab-test");
    let dish = scene.items.iter_mut().find(|i| i.id == "dish-1").unwrap();
    dish.properties.temperature_c = Some(40.0);
    dish.properties.composition = vec![CompositionEntry {
        substance_id: "water".into(),
        phase: "liquid".into(),
        amount_ml: Some(10.0),
        amount_scoop: None,
        amount_g: None,
        amount_mol: None,
    }];
    crate::solubility::sync_fill_ml(dish);

    let t0 = 40.0;
    let w0 = 10.0;
    let c0 = dish_c_eff_for_test(w0);
    let p_w = water_vapor_pressure_bar(t0);
    let p_air = RELATIVE_HUMIDITY * water_vapor_pressure_bar(AMBIENT_TEMPERATURE_C);
    let driving = (p_w - p_air).max(0.0) / ATM_PRESSURE_BAR;
    let m_dot = DISH_MASS_TRANSFER_COEFF_ML_PER_S_M2 * DISH_EVAP_AREA_M2 * driving;
    let dt = 10.0;
    let loss = m_dot * dt;
    let t_after_latent = t0 - loss * WATER_LATENT_HEAT_J_PER_G / c0;
    // Ambient Newton cool also runs (burner off).
    let c_mid = dish_c_eff_for_test(w0 - loss);
    let t_expected =
        t_after_latent - (UA_DISH / c_mid) * (t_after_latent - AMBIENT_TEMPERATURE_C) * dt;
    // Order in apply_elapsed: evap+latent first, then ambient cool with post-loss C_eff.
    apply_elapsed(&mut scene, dt);
    let dish = item(&scene, "dish-1");
    assert!((water_ml(dish) - (w0 - loss)).abs() < 1e-6);
    let actual = dish.properties.temperature_c.unwrap();
    assert!(
        (actual - t_expected).abs() < 1e-6,
        "expected {t_expected}, got {actual}"
    );
    assert!(actual < t0);
}

#[test]
fn burner_heats_to_boil_then_heat_limited_evaporates_and_turns_off() {
    let mut scene = initial_bench_scene("lab-test");
    let dish = scene.items.iter_mut().find(|i| i.id == "dish-1").unwrap();
    dish.properties.temperature_c = Some(20.0);
    // Enough water that heat-up + sub-boil MT still leave liquid at first boil.
    dish.properties.composition = vec![
        CompositionEntry {
            substance_id: "water".into(),
            phase: "liquid".into(),
            amount_ml: Some(8.0),
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

    for _ in 0..5000 {
        let dish = item(&scene, "dish-1");
        if water_ml(dish) < 1e-9 {
            panic!("dried via mass transfer before reaching boil");
        }
        let t = dish.properties.temperature_c.unwrap();
        let x = water_mole_fraction(dish);
        if t + 1e-3 >= boiling_temperature_c(x) {
            break;
        }
        apply_elapsed(&mut scene, 0.25);
    }
    let dish = item(&scene, "dish-1");
    let t = dish.properties.temperature_c.unwrap();
    let x = water_mole_fraction(dish);
    let tb = boiling_temperature_c(x);
    assert!(
        (t - tb).abs() < 0.15,
        "expected near T_boil {tb}, got {t}"
    );
    assert!(tb > BOILING_TEMPERATURE_C, "salt should elevate T_boil");
    let water_at_boil = water_ml(dish);
    assert!(water_at_boil > 2.0, "should still have liquid at first boil");

    let rate = heat_limited_evap_ml_per_s();
    assert!(rate > 0.01 && rate < 0.1, "lab-scale boil rate {rate}");
    apply_elapsed(&mut scene, 1.0 / rate);
    let dish = item(&scene, "dish-1");
    assert!((water_ml(dish) - (water_at_boil - 1.0)).abs() < 0.08);

    for _ in 0..10_000 {
        if water_ml(item(&scene, "dish-1")) < 1e-9 {
            break;
        }
        apply_elapsed(&mut scene, 1.0);
    }
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
    let water_before = water_ml(dish);

    apply_action(
        &mut scene,
        Action::ToggleBurner {
            burner_item_id: "burner-1".into(),
        },
    )
    .unwrap();
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
        (water_ml(dish) - water_before).abs() < 1e-9,
        "no mass-transfer while burner heats below boil"
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
        crate::solubility::solubility_mol_per_l(crate::solubility::Salt::Nacl, t_now) * water_ml(dish)
            / 1000.0;
    assert!((na_after - max_now).abs() < 1e-6);
}

#[test]
fn evaporating_mixed_dish_precipitates_nacl_before_independent_caps() {
    let mut scene = initial_bench_scene("lab-test");
    let s_nacl = crate::solubility::solubility_mol_per_l(crate::solubility::Salt::Nacl, 100.0);
    let s_cacl2 = crate::solubility::solubility_mol_per_l(crate::solubility::Salt::Cacl2, 100.0);
    let n_nacl = s_nacl * 0.003;
    let n_cacl2 = s_cacl2 * 0.003;
    let dish = scene.items.iter_mut().find(|i| i.id == "dish-1").unwrap();
    dish.properties.temperature_c = Some(BOILING_TEMPERATURE_C);
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
    // Evaporate until ~2 ml remain (heat-limited ~0.035 ml/s → ~370 s for 13 ml).
    let target = 2.0;
    for _ in 0..20_000 {
        if water_ml(item(&scene, "dish-1")) <= target + 1e-6 {
            break;
        }
        apply_elapsed(&mut scene, 0.5);
    }
    let dish = item(&scene, "dish-1");
    assert!(
        (water_ml(dish) - target).abs() < 0.15,
        "expected ~2 ml, got {}",
        water_ml(dish)
    );
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
