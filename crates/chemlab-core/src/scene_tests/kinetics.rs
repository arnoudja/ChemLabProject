//! Kinetic dissolve theme: pour-contact leftover, ambient clock clear, filter wash.

use super::super::*;
use super::filter::{filter_pour_water_through_paper, put_solids_on_paper, set_source_water};
use super::helpers::*;

#[test]
fn pour_contact_leaves_nacl_solid_leftover_before_clock() {
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
    let solid_pour = solid_g(water, "nacl");
    let na_pour = aqueous_mol(water, "na+");
    assert!(na_pour > 1e-6, "pour-contact must dissolve some NaCl");
    assert!(
        solid_pour > 0.05,
        "pour contact must leave noticeable solid NaCl, got {solid_pour} g"
    );
}

#[test]
fn ambient_clock_clears_unsaturated_nacl_without_burner() {
    let mut scene = bench_with_water("lab-test");
    use_tongs(&mut scene, "beaker-nacl").unwrap();
    use_tongs(&mut scene, "beaker-water").unwrap();

    let solid_after_dump = solid_g(item(&scene, "beaker-water"), "nacl");
    assert!(
        solid_after_dump > 0.1,
        "need leftover solid for the clock path, got {solid_after_dump} g"
    );
    assert_eq!(
        item(&scene, "burner-1").properties.on,
        Some(false),
        "ambient dissolve must not require the burner"
    );

    // Clock alone (no further pours / burner) finishes unsaturated kinetic dissolve.
    finish_kinetic_dissolve(&mut scene);

    let water = item(&scene, "beaker-water");
    assert_eq!(solid_g(water, "nacl"), 0.0);
    // Tongs dump the full Free-mode solid stock (2.00 g).
    let expected_mol = 2.0 / NACL_MOLAR_MASS_G_PER_MOL;
    let na = aqueous_mol(water, "na+");
    assert!(
        (na - expected_mol).abs() < 1e-6,
        "ambient clock must clear 2 g NaCl into Na⁺: na={na} expected={expected_mol}"
    );
}

#[test]
fn filter_wash_ten_ml_nacl_stays_partial() {
    let paper_nacl = 1.0;
    let mut scene = initial_bench_scene("lab-test");
    put_solids_on_paper(&mut scene, vec![solid("nacl", paper_nacl)]);
    set_source_water(&mut scene, 10.0, 20.0);
    filter_pour_water_through_paper(&mut scene);

    let washed = paper_nacl - solid_g(item(&scene, "filter-paper-1"), "nacl");
    let frac = washed / paper_nacl;
    assert!(
        (0.20..0.55).contains(&frac),
        "10 ml NaCl wash must stay partial (~tens of %), got {frac} (washed={washed})"
    );
    assert!(
        aqueous_mol(item(&scene, "beaker-filtrate"), "na+") > 1e-6,
        "some Na⁺ must reach filtrate"
    );
}
