//! Filter pour and wash kinetics scene tests.

use super::super::super::*;
use super::super::helpers::*;

use super::{filter_pour_water_through_paper, put_solids_on_paper, set_source_water};

#[test]
fn filter_pour_splits_slurry_solids_on_paper_ions_in_filtrate() {
    let mut scene = initial_bench_scene("lab-test");
    set_slurry_in_water(&mut scene);
    let water_before = item(&scene, "beaker-water");
    let source_ml = water_ml(water_before);
    let na_before = aqueous_mol(water_before, "na+");
    let cl_before = aqueous_mol(water_before, "cl-");
    let sand_before = solid_g(water_before, "sand");

    use_tongs(&mut scene, "beaker-water").unwrap();
    use_tongs(&mut scene, "filter-paper-1").unwrap();

    let water = item(&scene, "beaker-water");
    let filtrate = item(&scene, "beaker-filtrate");
    let paper = item(&scene, "filter-paper-1");
    assert_eq!(water.location, "held");
    assert!((water_ml(filtrate) - source_ml).abs() < 1e-9);
    assert!((water_ml(water) - 0.0).abs() < 1e-9);
    assert!((aqueous_mol(filtrate, "na+") - na_before).abs() < 1e-12);
    assert!((aqueous_mol(filtrate, "cl-") - cl_before).abs() < 1e-12);
    assert_eq!(solid_g(filtrate, "sand"), 0.0);
    assert!((solid_g(paper, "sand") - sand_before).abs() < 1e-12);
    assert_eq!(solid_g(water, "sand"), 0.0);
    assert_eq!(aqueous_mol(paper, "na+"), 0.0);
    assert!(scene.last_events.iter().any(|e| e.kind == "poured"));
    assert_eq!(
        item(&scene, "tongs-1").properties.source_item_id.as_deref(),
        Some("beaker-water")
    );
}

#[test]
fn filter_pour_caps_at_250_ml_and_moves_proportional_solids_to_paper() {
    let mut scene = initial_bench_scene("lab-test");
    set_slurry_in_water(&mut scene);
    let dest = scene
        .items
        .iter_mut()
        .find(|i| i.id == "beaker-filtrate")
        .unwrap();
    dest.properties.composition = vec![CompositionEntry {
        substance_id: "water".into(),
        phase: "liquid".into(),
        amount_ml: Some(100.0),
        amount_scoop: None,
        amount_g: None,
        amount_mol: None,
    }];
    dest.properties.temperature_c = Some(40.0);
    crate::solubility::sync_fill_ml(dest);

    let water_before = item(&scene, "beaker-water");
    let source_ml = water_ml(water_before);
    let source_v = crate::hcl::solution_volume_ml(water_before);
    let na_before = aqueous_mol(water_before, "na+");
    let sand_before = solid_g(water_before, "sand");
    let source_t = water_before.properties.temperature_c.unwrap_or(20.0);
    let dest_v_before = 100.0;
    let transferred_v = 250.0 - dest_v_before;
    let frac = transferred_v / source_v;
    let water_transferred = source_ml * frac;

    use_tongs(&mut scene, "beaker-water").unwrap();
    use_tongs(&mut scene, "filter-paper-1").unwrap();

    let water = item(&scene, "beaker-water");
    let filtrate = item(&scene, "beaker-filtrate");
    let paper = item(&scene, "filter-paper-1");
    assert!((crate::hcl::solution_volume_ml(filtrate) - 250.0).abs() < 1e-6);
    assert!((water_ml(filtrate) - (100.0 + water_transferred)).abs() < 1e-9);
    assert!(water_ml(filtrate) < 250.0 - 1e-6);
    assert!((water_ml(water) - (source_ml - water_transferred)).abs() < 1e-9);
    assert!((aqueous_mol(filtrate, "na+") - na_before * frac).abs() < 1e-12);
    assert!((solid_g(paper, "sand") - sand_before * frac).abs() < 1e-12);
    assert_eq!(solid_g(filtrate, "sand"), 0.0);
    assert!((solid_g(water, "sand") - sand_before * (1.0 - frac)).abs() < 1e-12);
    let c_dest = C_BEAKER + 100.0 * WATER_SPECIFIC_HEAT_J_PER_G_K;
    let c_add = water_transferred * WATER_SPECIFIC_HEAT_J_PER_G_K;
    let expected_t = (c_dest * 40.0 + c_add * source_t) / (c_dest + c_add);
    assert!((filtrate.properties.temperature_c.unwrap() - expected_t).abs() < 1e-9);
    assert_eq!(water.location, "held");
}

#[test]
fn filter_pour_rejects_when_filtrate_is_full_and_stays_holding() {
    let mut scene = initial_bench_scene("lab-test");
    set_slurry_in_water(&mut scene);
    let dest = scene
        .items
        .iter_mut()
        .find(|i| i.id == "beaker-filtrate")
        .unwrap();
    dest.properties.composition = vec![CompositionEntry {
        substance_id: "water".into(),
        phase: "liquid".into(),
        amount_ml: Some(250.0),
        amount_scoop: None,
        amount_g: None,
        amount_mol: None,
    }];
    crate::solubility::sync_fill_ml(dest);

    use_tongs(&mut scene, "beaker-water").unwrap();
    let sand_before = solid_g(item(&scene, "beaker-water"), "sand");
    let err = use_tongs(&mut scene, "filter-paper-1").unwrap_err();
    assert_eq!(err, SceneError::InvalidAction);
    assert_eq!(item(&scene, "beaker-water").location, "held");
    assert!((solid_g(item(&scene, "beaker-water"), "sand") - sand_before).abs() < 1e-12);
    assert_eq!(solid_g(item(&scene, "filter-paper-1"), "sand"), 0.0);
    assert!((water_ml(item(&scene, "beaker-filtrate")) - 250.0).abs() < 1e-9);
}

#[test]
fn filter_pour_empty_source_errors_and_stays_holding() {
    let mut scene = initial_bench_scene("lab-test");
    use_tongs(&mut scene, "dish-1").unwrap();
    let err = use_tongs(&mut scene, "filter-paper-1").unwrap_err();
    assert_eq!(err, SceneError::EmptyHolding);
    assert_eq!(item(&scene, "dish-1").location, "held");
    assert_eq!(
        item(&scene, "tongs-1").properties.source_item_id.as_deref(),
        Some("dish-1")
    );
}

#[test]
fn filter_pour_rejects_when_filtrate_beaker_is_not_home() {
    let mut scene = initial_bench_scene("lab-test");
    use_tongs(&mut scene, "beaker-filtrate").unwrap();
    put_tongs_away(&mut scene).unwrap();
    use_tongs(&mut scene, "beaker-filtrate").unwrap();
    let mut other = initial_bench_scene("lab-test");
    other.items = scene.items.clone();
    other
        .items
        .iter_mut()
        .find(|i| i.id == "beaker-water")
        .unwrap()
        .location = "held".into();
    other
        .items
        .iter_mut()
        .find(|i| i.id == "tongs-1")
        .unwrap()
        .properties
        .source_item_id = Some("beaker-water".into());
    other
        .items
        .iter_mut()
        .find(|i| i.id == "beaker-filtrate")
        .unwrap()
        .location = "held".into();

    let err = use_tongs(&mut other, "filter-paper-1").unwrap_err();
    assert_eq!(err, SceneError::InvalidAction);
    assert_eq!(item(&other, "beaker-filtrate").location, "held");
    assert_eq!(
        item(&other, "tongs-1").properties.source_item_id.as_deref(),
        Some("beaker-water")
    );
}

#[test]
fn filter_pour_washes_unsaturated_nacl_from_paper_into_filtrate() {
    let mut scene = initial_bench_scene("lab-test");
    put_solids_on_paper(&mut scene, vec![solid("nacl", 0.5), solid("sand", 0.3)]);
    set_source_water(&mut scene, 50.0, 20.0);

    filter_pour_water_through_paper(&mut scene);

    let filtrate = item(&scene, "beaker-filtrate");
    let paper = item(&scene, "filter-paper-1");
    let na = aqueous_mol(filtrate, "na+");
    let cl = aqueous_mol(filtrate, "cl-");
    assert!(
        na > 1e-6,
        "expected some NaCl washed into filtrate, got na+={na}"
    );
    assert!((cl - na).abs() < 1e-12);
    assert!(solid_g(paper, "nacl") < 0.5 - 1e-6);
    assert!((solid_g(paper, "nacl") + na * NACL_MOLAR_MASS_G_PER_MOL - 0.5).abs() < 1e-9);
    assert!((solid_g(paper, "sand") - 0.3).abs() < 1e-12);
    assert_eq!(solid_g(filtrate, "sand"), 0.0);
    assert_eq!(solid_g(filtrate, "nacl"), 0.0);
}

#[test]
fn filter_pour_wash_dissolves_nothing_when_fluid_already_saturated() {
    let mut scene = initial_bench_scene("lab-test");
    put_solids_on_paper(&mut scene, vec![solid("nacl", 0.5)]);
    let s20 = crate::solubility::solubility_mol_per_l(crate::solubility::Salt::Nacl, 20.0);
    let water_ml = 20.0;
    let sat_mol = s20 * (water_ml / 1000.0);
    let water = scene
        .items
        .iter_mut()
        .find(|i| i.id == "beaker-water")
        .unwrap();
    water.properties.composition = vec![
        CompositionEntry {
            substance_id: "water".into(),
            phase: "liquid".into(),
            amount_ml: Some(water_ml),
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
            amount_mol: Some(sat_mol),
        },
        CompositionEntry {
            substance_id: "cl-".into(),
            phase: "aqueous".into(),
            amount_ml: None,
            amount_scoop: None,
            amount_g: None,
            amount_mol: Some(sat_mol),
        },
    ];
    water.properties.temperature_c = Some(20.0);
    crate::solubility::sync_fill_ml(water);
    crate::solubility::enforce_saturation(water);

    let paper_nacl_before = solid_g(item(&scene, "filter-paper-1"), "nacl");
    filter_pour_water_through_paper(&mut scene);

    let filtrate = item(&scene, "beaker-filtrate");
    let paper = item(&scene, "filter-paper-1");
    // Saturated rinse: wash adds ~0 solid; filtrate may still hold the poured ions.
    assert!((solid_g(paper, "nacl") - paper_nacl_before).abs() < 1e-6);
    let na_from_source = aqueous_mol(filtrate, "na+");
    // No extra Na from paper beyond what the saturated source already carried.
    assert!((na_from_source - sat_mol).abs() < 1e-4);
}

#[test]
fn filter_pour_wash_leaves_sand_on_paper() {
    let mut scene = initial_bench_scene("lab-test");
    put_solids_on_paper(&mut scene, vec![solid("sand", 0.8)]);
    set_source_water(&mut scene, 40.0, 20.0);
    filter_pour_water_through_paper(&mut scene);
    assert!((solid_g(item(&scene, "filter-paper-1"), "sand") - 0.8).abs() < 1e-12);
    assert_eq!(solid_g(item(&scene, "beaker-filtrate"), "sand"), 0.0);
    assert_eq!(aqueous_mol(item(&scene, "beaker-filtrate"), "na+"), 0.0);
}

#[test]
fn filter_pour_larger_volume_washes_more_nacl_than_smaller() {
    let paper_nacl = 1.0;
    let mut small = initial_bench_scene("lab-test");
    put_solids_on_paper(&mut small, vec![solid("nacl", paper_nacl)]);
    set_source_water(&mut small, 10.0, 20.0);
    filter_pour_water_through_paper(&mut small);
    let washed_small = paper_nacl - solid_g(item(&small, "filter-paper-1"), "nacl");

    let mut large = initial_bench_scene("lab-test");
    put_solids_on_paper(&mut large, vec![solid("nacl", paper_nacl)]);
    set_source_water(&mut large, 200.0, 20.0);
    filter_pour_water_through_paper(&mut large);
    let washed_large = paper_nacl - solid_g(item(&large, "filter-paper-1"), "nacl");

    assert!(washed_small > 1e-6, "small rinse should dissolve some salt");
    assert!(
        washed_large > washed_small + 1e-4,
        "larger pour should wash more: small={washed_small} large={washed_large}"
    );
}

#[test]
fn filter_pour_hot_rinse_has_higher_nacl_wash_capacity_than_cold() {
    // Near-saturation solid load so capacity (not kinetics alone) differs with T.
    let paper_nacl = 8.0;
    let rinse_ml = 20.0;

    let mut cold = initial_bench_scene("lab-test");
    put_solids_on_paper(&mut cold, vec![solid("nacl", paper_nacl)]);
    set_source_water(&mut cold, rinse_ml, 20.0);
    filter_pour_water_through_paper(&mut cold);
    let washed_cold = paper_nacl - solid_g(item(&cold, "filter-paper-1"), "nacl");

    let mut hot = initial_bench_scene("lab-test");
    put_solids_on_paper(&mut hot, vec![solid("nacl", paper_nacl)]);
    set_source_water(&mut hot, rinse_ml, 80.0);
    filter_pour_water_through_paper(&mut hot);
    let washed_hot = paper_nacl - solid_g(item(&hot, "filter-paper-1"), "nacl");

    assert!(
        washed_hot > washed_cold + 1e-4,
        "hot rinse should dissolve more when capacity-limited: cold={washed_cold} hot={washed_hot}"
    );
}

#[test]
fn filter_pour_washes_soluble_solids_arriving_in_same_pour() {
    let mut scene = initial_bench_scene("lab-test");
    let source_nacl = 0.4;
    let water = scene
        .items
        .iter_mut()
        .find(|i| i.id == "beaker-water")
        .unwrap();
    water.properties.composition = vec![
        CompositionEntry {
            substance_id: "water".into(),
            phase: "liquid".into(),
            amount_ml: Some(50.0),
            amount_scoop: None,
            amount_g: None,
            amount_mol: None,
        },
        solid("nacl", source_nacl),
    ];
    water.properties.temperature_c = Some(20.0);
    crate::solubility::sync_fill_ml(water);

    filter_pour_water_through_paper(&mut scene);

    let filtrate = item(&scene, "beaker-filtrate");
    let paper = item(&scene, "filter-paper-1");
    let na = aqueous_mol(filtrate, "na+");
    assert!(
        na > 1e-6,
        "same-pour solid NaCl should partially wash into filtrate, na+={na}"
    );
    assert!(solid_g(paper, "nacl") < source_nacl - 1e-6);
    assert!((solid_g(paper, "nacl") + na * NACL_MOLAR_MASS_G_PER_MOL - source_nacl).abs() < 1e-9);
    assert_eq!(solid_g(item(&scene, "beaker-water"), "nacl"), 0.0);
}

#[test]
fn filter_pour_washes_unsaturated_cacl2_from_paper_into_filtrate() {
    let mut scene = initial_bench_scene("lab-test");
    put_solids_on_paper(&mut scene, vec![solid("cacl2", 0.5)]);
    set_source_water(&mut scene, 50.0, 20.0);

    filter_pour_water_through_paper(&mut scene);

    let filtrate = item(&scene, "beaker-filtrate");
    let paper = item(&scene, "filter-paper-1");
    let ca = aqueous_mol(filtrate, "ca2+");
    let cl = aqueous_mol(filtrate, "cl-");
    assert!(
        ca > 1e-6,
        "expected some CaCl2 washed into filtrate, ca2+={ca}"
    );
    assert!((cl - 2.0 * ca).abs() < 1e-12);
    assert!(solid_g(paper, "cacl2") < 0.5 - 1e-6);
    assert!((solid_g(paper, "cacl2") + ca * CACL2_MOLAR_MASS_G_PER_MOL - 0.5).abs() < 1e-9);
    assert_eq!(solid_g(filtrate, "cacl2"), 0.0);
}

#[test]
fn filter_pour_washes_naoh_from_paper_into_filtrate() {
    let mut scene = initial_bench_scene("lab-test");
    put_solids_on_paper(&mut scene, vec![solid("naoh", 0.5), solid("sand", 0.2)]);
    set_source_water(&mut scene, 50.0, 20.0);

    filter_pour_water_through_paper(&mut scene);

    let filtrate = item(&scene, "beaker-filtrate");
    let paper = item(&scene, "filter-paper-1");
    let na = aqueous_mol(filtrate, "na+");
    let oh = aqueous_mol(filtrate, "oh-");
    assert!(
        oh > 1e-6,
        "expected some NaOH washed into filtrate, oh-={oh}"
    );
    assert!((na - oh).abs() < 1e-12);
    assert!(solid_g(paper, "naoh") < 0.5 - 1e-6);
    assert!((solid_g(paper, "naoh") + oh * 40.0 - 0.5).abs() < 1e-9);
    assert!((solid_g(paper, "sand") - 0.2).abs() < 1e-12);
    assert_eq!(solid_g(filtrate, "sand"), 0.0);
    assert_eq!(solid_g(filtrate, "naoh"), 0.0);
    assert!(aqueous_mol(filtrate, "cl-") < 1e-12);
}

#[test]
fn filter_pour_wash_cools_filtrate_when_nacl_dissolves() {
    let mut with_salt = initial_bench_scene("lab-test");
    put_solids_on_paper(&mut with_salt, vec![solid("nacl", 1.0)]);
    set_source_water(&mut with_salt, 50.0, 20.0);
    filter_pour_water_through_paper(&mut with_salt);
    let t_with = item(&with_salt, "beaker-filtrate")
        .properties
        .temperature_c
        .unwrap();

    let mut plain = initial_bench_scene("lab-test");
    set_source_water(&mut plain, 50.0, 20.0);
    filter_pour_water_through_paper(&mut plain);
    let t_plain = item(&plain, "beaker-filtrate")
        .properties
        .temperature_c
        .unwrap();

    assert!(
        aqueous_mol(item(&with_salt, "beaker-filtrate"), "na+") > 1e-6,
        "wash must dissolve some NaCl for ΔT check"
    );
    assert!(
        t_with < t_plain - 1e-6,
        "endothermic NaCl wash should cool filtrate vs plain rinse: with={t_with} plain={t_plain}"
    );
}
