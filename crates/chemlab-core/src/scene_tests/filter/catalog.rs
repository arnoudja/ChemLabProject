//! Bench / catalog setup for filtration scene tests.

use super::super::super::*;
use super::super::helpers::*;

#[test]
fn initial_bench_scene_spawns_empty_filtrate_beaker_and_filter_paper() {
    let scene = initial_bench_scene("lab-test");
    assert!(scene.items.iter().any(|i| i.id == "beaker-filtrate"));
    assert!(scene.items.iter().any(|i| i.id == "filter-paper-1"));

    let filtrate = item(&scene, "beaker-filtrate");
    assert_eq!(filtrate.kind, "beaker");
    assert_eq!(filtrate.label, "Filtrate");
    assert_eq!(filtrate.location, "bench");
    assert_eq!(filtrate.properties.volume_ml, Some(250.0));
    assert_eq!(filtrate.properties.fill_ml, Some(0.0));
    assert_eq!(filtrate.properties.transparent, Some(true));
    assert_eq!(filtrate.properties.colourless, Some(true));
    assert_eq!(
        filtrate.properties.temperature_c,
        Some(AMBIENT_TEMPERATURE_C)
    );
    assert!(filtrate.properties.composition.is_empty());

    let paper = item(&scene, "filter-paper-1");
    assert_eq!(paper.kind, "filter_paper");
    assert_eq!(paper.label, "Filter paper");
    assert_eq!(paper.location, "bench");
    assert!(paper.properties.composition.is_empty());
}

#[test]
fn ensure_default_bench_items_restores_filtration_catalog_without_resetting_vessels() {
    let mut scene = initial_bench_scene("lab-test");
    scene
        .items
        .iter_mut()
        .find(|item| item.id == "beaker-water")
        .unwrap()
        .properties
        .fill_ml = Some(150.0);
    scene
        .items
        .retain(|item| item.id != "beaker-filtrate" && item.id != "filter-paper-1");

    ensure_default_bench_items(&mut scene);

    let filtrate = item(&scene, "beaker-filtrate");
    assert_eq!(filtrate.kind, "beaker");
    assert_eq!(filtrate.location, "bench");
    assert_eq!(filtrate.properties.volume_ml, Some(250.0));
    assert_eq!(filtrate.properties.fill_ml, Some(0.0));
    let paper = item(&scene, "filter-paper-1");
    assert_eq!(paper.kind, "filter_paper");
    assert!(paper.properties.composition.is_empty());
    assert_eq!(item(&scene, "beaker-water").properties.fill_ml, Some(150.0));
    assert_eq!(
        scene
            .items
            .iter()
            .filter(|item| item.id == "beaker-filtrate")
            .count(),
        1
    );
    assert_eq!(
        scene
            .items
            .iter()
            .filter(|item| item.id == "filter-paper-1")
            .count(),
        1
    );
}

