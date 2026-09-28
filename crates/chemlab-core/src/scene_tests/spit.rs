//! Chemical-heat spit policy: action spray vs clock clamp-only.

use super::super::*;
use super::helpers::*;

fn spit_count(scene: &Scene) -> usize {
    scene
        .last_events
        .iter()
        .filter(|e| e.kind == "spit")
        .count()
}

fn solution_ml(id: &str, scene: &Scene) -> f64 {
    crate::hcl::solution_volume_ml(item(scene, id))
}

#[test]
fn chemical_heat_overshoot_spits_and_drops_mass_at_boil() {
    let mut scene = initial_bench_scene("lab-test");
    fill_main_beaker(&mut scene, 10.0);
    {
        let water = scene
            .items
            .iter_mut()
            .find(|i| i.id == "beaker-water")
            .unwrap();
        water.properties.temperature_c = Some(95.0);
    }
    fill_pipette_from(&mut scene, "beaker-h2so4");
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "pipette-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();

    assert_eq!(spit_count(&scene), 1);
    let spit = scene
        .last_events
        .iter()
        .find(|e| e.kind == "spit")
        .expect("spit event");
    assert_eq!(spit.message, "");

    let water = item(&scene, "beaker-water");
    let t = water.properties.temperature_c.unwrap();
    let t_boil = boiling_temperature_c(water_mole_fraction(water));
    assert!(
        (t - t_boil).abs() < 0.05,
        "T should clamp at boil: T={t} T_boil={t_boil}"
    );
    let v = solution_ml("beaker-water", &scene);
    assert!(
        v < 10.0 + PIPETTE_VOLUME_ML - 0.15,
        "spit should drop sprayable volume, got {v}"
    );
    assert_fill_ml_matches_phi_v(&scene);
}

#[test]
fn mild_dilution_does_not_spit() {
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
    assert_eq!(spit_count(&scene), 0);
    let t = item(&scene, "beaker-water")
        .properties
        .temperature_c
        .unwrap_or(20.0);
    assert!(t > 20.5 && t < 80.0, "mild warming only, got {t}");
}

#[test]
fn stacked_chemical_heats_emit_at_most_one_spit() {
    let mut scene = initial_bench_scene("lab-test");
    fill_main_beaker(&mut scene, 10.0);
    {
        let water = scene
            .items
            .iter_mut()
            .find(|i| i.id == "beaker-water")
            .unwrap();
        water.properties.temperature_c = Some(95.0);
        water.properties.composition.push(CompositionEntry {
            substance_id: "naoh".into(),
            phase: "solid".into(),
            amount_ml: None,
            amount_scoop: Some(5),
            amount_g: Some(1.0),
            amount_mol: None,
        });
    }
    fill_pipette_from(&mut scene, "beaker-h2so4");
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "pipette-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();
    assert_eq!(
        spit_count(&scene),
        1,
        "dilution + kinetic + neutralization must share one spit: {:?}",
        scene.last_events
    );
    let water = item(&scene, "beaker-water");
    let t = water.properties.temperature_c.unwrap();
    let t_boil = boiling_temperature_c(water_mole_fraction(water));
    assert!((t - t_boil).abs() < 0.05, "T={t} T_boil={t_boil}");
}

#[test]
fn chemical_heat_clock_mode_clamps_without_spit_mass() {
    // allow_spit_mass=false (clock): clamp at boil, no spray discard, no spit flag.
    let mut item = item(&initial_bench_scene("lab-test"), "beaker-water").clone();
    item.properties.temperature_c = Some(95.0);
    item.properties.fill_ml = Some(10.0);
    item.properties.composition = vec![CompositionEntry {
        substance_id: "water".into(),
        phase: "liquid".into(),
        amount_ml: Some(10.0),
        amount_scoop: None,
        amount_g: None,
        amount_mol: None,
    }];
    crate::solubility::sync_fill_ml(&mut item);
    let v0 = crate::hcl::solution_volume_ml(&item);
    let c = effective_heat_capacity(&item);
    let q = -c * 20.0; // proposed T = 115 °C → overshoot
    let spit = crate::aqueous_pipeline::apply_chemical_heat(&mut item, q, false);
    assert!(!spit);
    let t_boil = vessel_boil_temperature_c(&item);
    assert!(
        (item.properties.temperature_c.unwrap() - t_boil).abs() < 0.05,
        "expected clamp at boil"
    );
    assert!(
        (crate::hcl::solution_volume_ml(&item) - v0).abs() < 1e-9,
        "clock mode must not discard spit fraction"
    );

    let mut item2 = item.clone();
    item2.properties.temperature_c = Some(95.0);
    crate::solubility::sync_fill_ml(&mut item2);
    let spit_action = crate::aqueous_pipeline::apply_chemical_heat(&mut item2, q, true);
    assert!(spit_action);
    assert!(crate::hcl::solution_volume_ml(&item2) < v0 * (1.0 - crate::CHEMICAL_SPIT_FRAC * 0.5));
}

#[test]
fn kinetic_naoh_hot_dump_spits_at_most_once_clock_keeps_mass() {
    // Kinetic dissolve heat on an action path: ≤1 spit + spray mass.
    let mut scene = initial_bench_scene("lab-test");
    fill_main_beaker(&mut scene, 10.0);
    {
        let water = scene
            .items
            .iter_mut()
            .find(|i| i.id == "beaker-water")
            .unwrap();
        water.properties.temperature_c = Some(95.0);
    }
    use_tongs(&mut scene, "beaker-naoh").unwrap();
    use_tongs(&mut scene, "beaker-water").unwrap();

    assert_eq!(
        spit_count(&scene),
        1,
        "kinetic NaOH dump into near-boil water must spit once: {:?}",
        scene.last_events
    );
    let water = item(&scene, "beaker-water");
    let t = water.properties.temperature_c.unwrap();
    let t_boil = vessel_boil_temperature_c(water);
    assert!((t - t_boil).abs() < 0.05, "T={t} T_boil={t_boil}");
    let v_action = solution_ml("beaker-water", &scene);
    assert!(
        v_action < 10.0,
        "action spit should drop sprayable volume, got {v_action}"
    );

    // Clock finalize (allow_spit_mass=false): no new spit event, no further spray discard.
    apply_elapsed(&mut scene, 2.0);
    assert_eq!(
        spit_count(&scene),
        1,
        "clock must not emit spit: {:?}",
        scene.last_events
    );
    assert!(
        (solution_ml("beaker-water", &scene) - v_action).abs() < 1e-9,
        "clock must not discard spit fraction"
    );
}
