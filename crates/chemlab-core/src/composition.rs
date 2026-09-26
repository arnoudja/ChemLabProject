//! Shared composition accessors for aqueous moles and liquid water.

use crate::scene::{CompositionEntry, SceneItem};

const AMOUNT_EPS: f64 = 1e-12;

pub fn aqueous_mol(item: &SceneItem, substance_id: &str) -> f64 {
    item.properties
        .composition
        .iter()
        .find(|c| c.substance_id == substance_id && c.phase == "aqueous")
        .and_then(|c| c.amount_mol)
        .unwrap_or(0.0)
        .max(0.0)
}

pub fn aqueous_mol_entries(entries: &[CompositionEntry], substance_id: &str) -> f64 {
    entries
        .iter()
        .find(|c| c.substance_id == substance_id && c.phase == "aqueous")
        .and_then(|c| c.amount_mol)
        .unwrap_or(0.0)
        .max(0.0)
}

pub fn liquid_water_ml(item: &SceneItem) -> f64 {
    item.properties
        .composition
        .iter()
        .find(|c| c.substance_id == "water" && c.phase == "liquid")
        .and_then(|c| c.amount_ml)
        .unwrap_or(0.0)
        .max(0.0)
}

pub fn liquid_water_ml_entries(entries: &[CompositionEntry]) -> f64 {
    entries
        .iter()
        .find(|c| c.substance_id == "water" && c.phase == "liquid")
        .and_then(|c| c.amount_ml)
        .unwrap_or(0.0)
        .max(0.0)
}

pub fn set_aqueous_mol(item: &mut SceneItem, substance_id: &str, moles: f64) {
    if moles <= AMOUNT_EPS {
        item.properties
            .composition
            .retain(|c| !(c.substance_id == substance_id && c.phase == "aqueous"));
        return;
    }
    if let Some(existing) = item
        .properties
        .composition
        .iter_mut()
        .find(|c| c.substance_id == substance_id && c.phase == "aqueous")
    {
        existing.amount_mol = Some(moles);
        return;
    }
    item.properties.composition.push(CompositionEntry {
        substance_id: substance_id.into(),
        phase: "aqueous".into(),
        amount_ml: None,
        amount_scoop: None,
        amount_g: None,
        amount_mol: Some(moles),
    });
}
