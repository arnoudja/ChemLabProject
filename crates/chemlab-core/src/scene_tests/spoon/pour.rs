//! Spoon pour / dissolve / temperature scene tests.

use super::super::super::*;
use super::super::helpers::*;

#[test]
fn pour_nacl_into_water_dissolves_without_leftover_grains() {
    let mut scene = bench_with_water("lab-test");
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

    let spoon = item(&scene, "spoon-1");
    assert!(spoon.properties.holding.is_empty());
    assert!(scene.last_events.iter().any(|e| {
        e.kind == "dissolved"
            && e.message == "Sodium chloride (NaCl) dissolves in water at bench temperature."
    }));

    // Pour contact is partial — measurable leftover solid before clock ticks.
    let water_after_pour = item(&scene, "beaker-water");
    let na_pour = aqueous_mol(water_after_pour, "na+");
    let solid_pour = solid_g(water_after_pour, "nacl");
    assert!(na_pour > 1e-6);
    assert!(
        solid_pour > 0.05,
        "pour contact must leave noticeable solid NaCl, got {solid_pour} g"
    );
    finish_kinetic_dissolve(&mut scene);

    let water = item(&scene, "beaker-water");
    let expected_mol = SPOON_SCOOP_MASS_G / NACL_MOLAR_MASS_G_PER_MOL;
    assert!((aqueous_mol(water, "na+") - expected_mol).abs() < 1e-9);
    assert!((aqueous_mol(water, "cl-") - expected_mol).abs() < 1e-9);
    assert_eq!(solid_g(water, "nacl"), 0.0);
}

#[test]
fn pour_nacl_into_water_cools_solution_endothermically() {
    let mut scene = bench_with_water("lab-test");
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
    let moles = aqueous_mol(water, "na+");
    assert!(moles > 1e-6, "pour-contact must dissolve some NaCl");
    let actual = water
        .properties
        .temperature_c
        .expect("water beaker should keep a temperature");
    assert!(actual < 20.0, "NaCl dissolution must cool the beaker");
    // ΔH applies only to moles dissolved this step (not the full scoop if leftover solid).
    let c_eff = effective_heat_capacity(water);
    let expected_delta = -(moles * NACL_DELTA_H_SOLUTION_J_PER_MOL) / c_eff;
    // C_eff after dissolve excludes dissolved mass from solid term; allow small model slack.
    assert!(
        (actual - (20.0 + expected_delta)).abs() < 0.05,
        "ΔT should track dissolved moles: T={actual}, moles={moles}"
    );
    assert_eq!(scene.temperature_c, 20.0);
}

#[test]
fn pour_cacl2_into_water_dissolves_with_ions_and_heats_exothermically() {
    let mut scene = bench_with_water("lab-test");
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-cacl2".into(),
        },
    )
    .unwrap();

    let stock = item(&scene, "beaker-cacl2")
        .properties
        .composition
        .iter()
        .find(|c| c.substance_id == "cacl2" && c.phase == "solid")
        .unwrap();
    assert_eq!(stock.amount_scoop, Some(9));
    assert_eq!(stock.amount_g, Some(9.0 * SPOON_SCOOP_MASS_G));

    apply_action(
        &mut scene,
        Action::Pour {
            source_item_id: "spoon-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();

    let water = item(&scene, "beaker-water");
    let moles_pour = aqueous_mol(water, "ca2+");
    assert!(moles_pour > 1e-6);
    assert!((aqueous_mol(water, "cl-") - 2.0 * moles_pour).abs() < 1e-12);
    assert!(water.properties.temperature_c.unwrap() > 20.0);
    assert!(scene.last_events.iter().any(|e| {
        e.kind == "dissolved"
            && e.message == "Calcium chloride (CaCl2) dissolves in water at bench temperature."
    }));

    finish_kinetic_dissolve(&mut scene);
    let water = item(&scene, "beaker-water");
    let moles = SPOON_SCOOP_MASS_G / CACL2_MOLAR_MASS_G_PER_MOL;
    assert!((aqueous_mol(water, "ca2+") - moles).abs() < 1e-9);
    assert!((aqueous_mol(water, "cl-") - 2.0 * moles).abs() < 1e-9);
    assert_eq!(solid_g(water, "cacl2"), 0.0);
    assert_eq!(scene.temperature_c, 20.0);
}

#[test]
fn pour_sand_into_water_does_not_change_temperature() {
    let mut scene = bench_with_water("lab-test");
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
        Action::Pour {
            source_item_id: "spoon-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();

    let water = item(&scene, "beaker-water");
    assert_eq!(water.properties.temperature_c, Some(20.0));
}

#[test]
fn second_nacl_pour_updates_existing_ion_moles_without_duplicate_lines() {
    let mut scene = bench_with_water("lab-test");
    for _ in 0..2 {
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
    }

    let water = item(&scene, "beaker-water");
    let na_count = water
        .properties
        .composition
        .iter()
        .filter(|c| c.substance_id == "na+" && c.phase == "aqueous")
        .count();
    let cl_count = water
        .properties
        .composition
        .iter()
        .filter(|c| c.substance_id == "cl-" && c.phase == "aqueous")
        .count();
    assert_eq!(na_count, 1);
    assert_eq!(cl_count, 1);
    finish_kinetic_dissolve(&mut scene);
    let water = item(&scene, "beaker-water");
    let expected_mol = 2.0 * SPOON_SCOOP_MASS_G / NACL_MOLAR_MASS_G_PER_MOL;
    assert!((aqueous_mol(water, "na+") - expected_mol).abs() < 1e-9);
}

#[test]
fn second_sand_pour_aggregates_solid_mass_on_one_line() {
    let mut scene = bench_with_water("lab-test");
    for _ in 0..2 {
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
            Action::Pour {
                source_item_id: "spoon-1".into(),
                target_item_id: "beaker-water".into(),
            },
        )
        .unwrap();
    }

    let water = item(&scene, "beaker-water");
    let sand_rows: Vec<_> = water
        .properties
        .composition
        .iter()
        .filter(|c| c.substance_id == "sand" && c.phase == "solid")
        .collect();
    assert_eq!(sand_rows.len(), 1);
    assert_eq!(sand_rows[0].amount_scoop, Some(2));
    assert!((sand_rows[0].amount_g.unwrap() - 2.0 * SPOON_SCOOP_MASS_G).abs() < 1e-12);
}

#[test]
fn pour_sand_into_water_leaves_undissolved_solid() {
    let mut scene = bench_with_water("lab-test");
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
        Action::Pour {
            source_item_id: "spoon-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();

    let spoon = item(&scene, "spoon-1");
    assert!(spoon.properties.holding.is_empty());

    let water = item(&scene, "beaker-water");
    assert!(water.properties.composition.iter().any(|c| {
        c.substance_id == "sand"
            && c.phase == "solid"
            && c.amount_scoop == Some(1)
            && c.amount_g == Some(SPOON_SCOOP_MASS_G)
    }));
    assert!(!water
        .properties
        .composition
        .iter()
        .any(|c| c.substance_id == "sand" && c.phase == "aqueous"));
    assert!(scene.last_events.iter().any(|e| {
        e.kind == "did_not_dissolve"
            && e.message == "Sand (silica) does not dissolve in water at bench temperature."
    }));
}

#[test]
fn reset_restores_empty_beaker_and_clears_holding_after_pour() {
    let mut scene = bench_with_water("lab-test");
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
    scene.version = 2;

    apply_action(&mut scene, Action::Reset).unwrap();

    assert_eq!(scene.lab_id, "lab-test");
    assert_eq!(scene.version, 2);
    assert!(scene.last_events.iter().any(|e| e.kind == "reset"));

    let spoon = item(&scene, "spoon-1");
    assert_eq!(spoon.location, "bench");
    assert!(spoon.properties.holding.is_empty());

    let water = item(&scene, "beaker-water");
    assert_eq!(water.label, "Beaker");
    assert_eq!(water.properties.fill_ml, Some(0.0));
    assert!(water.properties.composition.is_empty());
}

#[test]
fn pour_with_empty_holding_returns_error() {
    let mut scene = initial_bench_scene("lab-test");
    let err = apply_action(
        &mut scene,
        Action::Pour {
            source_item_id: "spoon-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap_err();
    assert_eq!(err, SceneError::EmptyHolding);
}

#[test]
fn pour_into_dry_beaker_returns_invalid_action() {
    let mut scene = initial_bench_scene("lab-test");
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-nacl".into(),
        },
    )
    .unwrap();

    let err = apply_action(
        &mut scene,
        Action::Pour {
            source_item_id: "spoon-1".into(),
            target_item_id: "beaker-sand".into(),
        },
    )
    .unwrap_err();
    assert_eq!(err, SceneError::InvalidAction);
    let spoon = item(&scene, "spoon-1");
    assert_eq!(spoon.properties.holding[0].substance_id, "nacl");
}

#[test]
fn pour_sand_at_non_bench_temperature_leaves_undissolved_solid() {
    let mut scene = bench_with_water("lab-test");
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-sand".into(),
        },
    )
    .unwrap();

    let water = scene
        .items
        .iter_mut()
        .find(|i| i.id == "beaker-water")
        .unwrap();
    water.properties.temperature_c = Some(21.0);

    apply_action(
        &mut scene,
        Action::Pour {
            source_item_id: "spoon-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();

    let water = item(&scene, "beaker-water");
    let c_dest = C_BEAKER + FILLED_MAIN_BEAKER_ML * WATER_SPECIFIC_HEAT_J_PER_G_K;
    let c_add = SPOON_SCOOP_MASS_G * CP_SAND;
    let expected_t = (c_dest * 21.0 + c_add * 20.0) / (c_dest + c_add);
    assert!((water.properties.temperature_c.unwrap() - expected_t).abs() < 1e-9);
    assert!(water.properties.composition.iter().any(|c| {
        c.substance_id == "sand"
            && c.phase == "solid"
            && c.amount_scoop == Some(1)
            && (c.amount_g.unwrap() - SPOON_SCOOP_MASS_G).abs() < 1e-12
    }));
    assert!(!water
        .properties
        .composition
        .iter()
        .any(|c| c.substance_id == "sand" && c.phase == "aqueous"));
    assert!(scene.last_events.iter().any(|e| {
        e.kind == "did_not_dissolve"
            && e.message == "Sand (silica) does not dissolve in water at bench temperature."
    }));
    assert!(item(&scene, "spoon-1").properties.holding.is_empty());
}

#[test]
fn pour_sand_succeeds_after_cacl2_exothermic_heating() {
    let mut scene = bench_with_water("lab-test");

    // Vessel C_eff lowers ΔT/scoop; pour until dissolve lookup sees non-bench °C.
    let after_heat = {
        let mut t = None;
        for _ in 0..12 {
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
                Action::Pour {
                    source_item_id: "spoon-1".into(),
                    target_item_id: "beaker-water".into(),
                },
            )
            .unwrap();
            let now = item(&scene, "beaker-water")
                .properties
                .temperature_c
                .expect("water beaker should keep a temperature");
            t = Some(now);
            if now.round() as i32 != 20 {
                break;
            }
        }
        // Finish kinetic CaCl₂ so the sand pour is a pure sensible blend.
        finish_kinetic_dissolve(&mut scene);
        let _ = t;
        item(&scene, "beaker-water")
            .properties
            .temperature_c
            .expect("water beaker should keep a temperature")
    };
    assert!(
        after_heat.round() as i32 != 20,
        "CaCl2 heating must leave a non-bench lookup T, got {after_heat}"
    );

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
        Action::Pour {
            source_item_id: "spoon-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();

    let water = item(&scene, "beaker-water");
    let c_dest = effective_heat_capacity(water) - SPOON_SCOOP_MASS_G * CP_SAND;
    let c_add = SPOON_SCOOP_MASS_G * CP_SAND;
    let expected_t = (c_dest * after_heat + c_add * 20.0) / (c_dest + c_add);
    assert!((water.properties.temperature_c.unwrap() - expected_t).abs() < 1e-6);
    assert!(water.properties.composition.iter().any(|c| {
        c.substance_id == "sand"
            && c.phase == "solid"
            && c.amount_scoop == Some(1)
            && (c.amount_g.unwrap() - SPOON_SCOOP_MASS_G).abs() < 1e-12
    }));
    assert!(!water
        .properties
        .composition
        .iter()
        .any(|c| c.substance_id == "sand" && c.phase == "aqueous"));
    assert!(scene.last_events.iter().any(|e| {
        e.kind == "did_not_dissolve"
            && e.message == "Sand (silica) does not dissolve in water at bench temperature."
    }));
}

#[test]
fn pour_second_cacl2_scoop_succeeds_after_exothermic_heating() {
    let mut scene = bench_with_water("lab-test");

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
        Action::Pour {
            source_item_id: "spoon-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();

    let after_first = item(&scene, "beaker-water")
        .properties
        .temperature_c
        .expect("water beaker should keep a temperature");
    assert!(
        after_first > 20.0,
        "first CaCl2 scoop must raise beaker T above 20 °C, got {after_first}"
    );

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
        Action::Pour {
            source_item_id: "spoon-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();

    finish_kinetic_dissolve(&mut scene);
    let water = item(&scene, "beaker-water");
    let moles = 2.0 * (SPOON_SCOOP_MASS_G / CACL2_MOLAR_MASS_G_PER_MOL);
    assert!((aqueous_mol(water, "ca2+") - moles).abs() < 1e-9);

    let after_second = water
        .properties
        .temperature_c
        .expect("water beaker should keep a temperature");
    assert!(
        after_second > after_first - 1.0,
        "second CaCl2 scoop should leave the beaker warmer than ambient: {after_first} -> {after_second}"
    );
    assert!(scene.last_events.iter().any(|e| {
        e.kind == "dissolved"
            && e.message == "Calcium chloride (CaCl2) dissolves in water at bench temperature."
    }));
    assert!(item(&scene, "spoon-1").properties.holding.is_empty());
}

#[test]
fn pour_nacl_succeeds_after_mild_heating_above_bench() {
    let mut scene = bench_with_water("lab-test");
    let water = scene
        .items
        .iter_mut()
        .find(|i| i.id == "beaker-water")
        .unwrap();
    water.properties.temperature_c = Some(21.5);

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
    assert!(aqueous_mol(water, "na+") > 1e-6);
    assert!(
        water.properties.temperature_c.unwrap() < 21.5,
        "endothermic dissolve should cool below the pre-pour mild heat"
    );
    assert!(scene.last_events.iter().any(|e| {
        e.kind == "dissolved"
            && e.message == "Sodium chloride (NaCl) dissolves in water at bench temperature."
    }));
}

#[test]
fn pour_scoop_into_empty_beaker_adds_undissolved_solid() {
    let mut scene = initial_bench_scene("lab-test");
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

    assert!(item(&scene, "spoon-1").properties.holding.is_empty());
    assert!((solid_g(item(&scene, "beaker-water"), "nacl") - SPOON_SCOOP_MASS_G).abs() < 1e-12);
    assert_eq!(aqueous_mol(item(&scene, "beaker-water"), "na+"), 0.0);
    assert!(scene.last_events.iter().any(|e| e.kind == "poured"));
    assert!(!scene.last_events.iter().any(|e| e.kind == "dissolved"));
}

#[test]
fn pour_held_nacl_into_beaker_h2o_is_invalid() {
    let mut scene = initial_bench_scene("lab-test");
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-nacl".into(),
        },
    )
    .unwrap();

    let err = apply_action(
        &mut scene,
        Action::Pour {
            source_item_id: "spoon-1".into(),
            target_item_id: "beaker-h2o".into(),
        },
    )
    .unwrap_err();
    assert_eq!(err, SceneError::InvalidAction);

    let spoon = item(&scene, "spoon-1");
    assert_eq!(spoon.location, "hand");
    assert!((holding_g(spoon, "nacl") - SPOON_SCOOP_MASS_G).abs() < 1e-12);

    let h2o = item(&scene, "beaker-h2o");
    assert!((water_ml(h2o) - DISTILLED_WATER_CAPACITY_ML).abs() < 1e-9);
    assert!(h2o
        .properties
        .composition
        .iter()
        .all(|c| c.substance_id == "water" && c.phase == "liquid"));
    assert_eq!(aqueous_mol(h2o, "na+"), 0.0);
    assert_eq!(solid_g(item(&scene, "beaker-nacl"), "nacl"), 1.8);
}
