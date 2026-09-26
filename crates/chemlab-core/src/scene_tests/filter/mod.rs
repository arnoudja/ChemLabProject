//! Filter / filtration scene tests (split from former `filter.rs`).

mod catalog;
mod pour_wash;
mod spoon_paper;
mod tongs_vessels;

use super::super::*;
use super::helpers::*;

pub(super) fn put_solids_on_paper(scene: &mut Scene, solids: Vec<CompositionEntry>) {
    let paper = scene
        .items
        .iter_mut()
        .find(|i| i.id == "filter-paper-1")
        .unwrap();
    paper.properties.composition = solids;
}

pub(super) fn set_source_water(scene: &mut Scene, ml: f64, temperature_c: f64) {
    let water = scene
        .items
        .iter_mut()
        .find(|i| i.id == "beaker-water")
        .unwrap();
    water.properties.composition = vec![CompositionEntry {
        substance_id: "water".into(),
        phase: "liquid".into(),
        amount_ml: Some(ml),
        amount_scoop: None,
        amount_g: None,
        amount_mol: None,
    }];
    water.properties.temperature_c = Some(temperature_c);
    crate::solubility::sync_fill_ml(water);
}

pub(super) fn filter_pour_water_through_paper(scene: &mut Scene) {
    use_tongs(scene, "beaker-water").unwrap();
    use_tongs(scene, "filter-paper-1").unwrap();
}
