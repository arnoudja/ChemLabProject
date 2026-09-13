//! Independent v1 solubility limits for NaCl and CaCl₂ vs dish temperature.
//!
//! Values are formula-unit mol/L at 20 °C and 100 °C (textbook g/100 mL converted with
//! the same molar masses as scoop dissolution). Mixed-salt activity is out of scope:
//! each salt is saturated independently; Cl⁻ is then set from remaining formula units.

use crate::scene::{
    CompositionEntry, SceneItem, CACL2_MOLAR_MASS_G_PER_MOL, NACL_MOLAR_MASS_G_PER_MOL,
};

/// NaCl solubility at 20 °C (35.9 g / 100 mL → mol/L).
pub const NACL_SOLUBILITY_MOL_PER_L_20C: f64 = 35.9 * 10.0 / NACL_MOLAR_MASS_G_PER_MOL;

/// NaCl solubility at 100 °C (39.1 g / 100 mL → mol/L).
pub const NACL_SOLUBILITY_MOL_PER_L_100C: f64 = 39.1 * 10.0 / NACL_MOLAR_MASS_G_PER_MOL;

/// Anhydrous CaCl₂ solubility at 20 °C (74.5 g / 100 mL → mol/L).
pub const CACL2_SOLUBILITY_MOL_PER_L_20C: f64 = 74.5 * 10.0 / CACL2_MOLAR_MASS_G_PER_MOL;

/// Anhydrous CaCl₂ solubility at 100 °C (159 g / 100 mL → mol/L).
pub const CACL2_SOLUBILITY_MOL_PER_L_100C: f64 = 159.0 * 10.0 / CACL2_MOLAR_MASS_G_PER_MOL;

const AMOUNT_EPS: f64 = 1e-12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Salt {
    Nacl,
    Cacl2,
}

/// Formula-unit solubility (mol/L) at dish T, linearly interpolated between 20 °C and 100 °C.
pub fn solubility_mol_per_l(salt: Salt, temperature_c: f64) -> f64 {
    let (y0, y1) = match salt {
        Salt::Nacl => (
            NACL_SOLUBILITY_MOL_PER_L_20C,
            NACL_SOLUBILITY_MOL_PER_L_100C,
        ),
        Salt::Cacl2 => (
            CACL2_SOLUBILITY_MOL_PER_L_20C,
            CACL2_SOLUBILITY_MOL_PER_L_100C,
        ),
    };
    let frac = ((temperature_c - 20.0) / 80.0).clamp(0.0, 1.0);
    y0 + frac * (y1 - y0)
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

pub fn dish_has_liquid(item: &SceneItem) -> bool {
    liquid_water_ml(item) > AMOUNT_EPS
}

pub fn sync_fill_ml(item: &mut SceneItem) {
    if item.kind == "beaker" || item.kind == "evaporation_dish" || item.kind == "pipette" {
        item.properties.fill_ml = Some(liquid_water_ml(item));
    }
}

fn aqueous_mol(item: &SceneItem, substance_id: &str) -> f64 {
    item.properties
        .composition
        .iter()
        .find(|c| c.substance_id == substance_id && c.phase == "aqueous")
        .and_then(|c| c.amount_mol)
        .unwrap_or(0.0)
        .max(0.0)
}

fn solid_mol(item: &SceneItem, substance_id: &str, molar_mass: f64) -> f64 {
    item.properties
        .composition
        .iter()
        .find(|c| c.substance_id == substance_id && c.phase == "solid")
        .and_then(|c| c.amount_g)
        .map(|g| (g.max(0.0)) / molar_mass)
        .unwrap_or(0.0)
}

fn set_aqueous_mol(item: &mut SceneItem, substance_id: &str, moles: f64) {
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

fn set_salt_solid(item: &mut SceneItem, substance_id: &str, moles: f64, molar_mass: f64) {
    let grams = moles * molar_mass;
    if grams <= AMOUNT_EPS {
        item.properties
            .composition
            .retain(|c| !(c.substance_id == substance_id && c.phase == "solid"));
        return;
    }
    if let Some(existing) = item
        .properties
        .composition
        .iter_mut()
        .find(|c| c.substance_id == substance_id && c.phase == "solid")
    {
        existing.amount_g = Some(grams);
        existing.amount_mol = Some(moles);
        existing.amount_scoop = None;
        return;
    }
    item.properties.composition.push(CompositionEntry {
        substance_id: substance_id.into(),
        phase: "solid".into(),
        amount_ml: None,
        amount_scoop: None,
        amount_g: Some(grams),
        amount_mol: Some(moles),
    });
}

fn remove_near_zero_water(item: &mut SceneItem) {
    if liquid_water_ml(item) <= AMOUNT_EPS {
        item.properties
            .composition
            .retain(|c| !(c.substance_id == "water" && c.phase == "liquid"));
    }
}

/// Convert aqueous ↔ solid so each salt is at most saturated at the item's current T.
pub fn enforce_saturation(item: &mut SceneItem) {
    let temperature_c = item.properties.temperature_c.unwrap_or(20.0);
    let water_ml = liquid_water_ml(item);
    let litres = water_ml / 1000.0;

    let total_nacl = aqueous_mol(item, "na+") + solid_mol(item, "nacl", NACL_MOLAR_MASS_G_PER_MOL);
    let total_cacl2 =
        aqueous_mol(item, "ca2+") + solid_mol(item, "cacl2", CACL2_MOLAR_MASS_G_PER_MOL);

    let (aq_nacl, solid_nacl, aq_cacl2, solid_cacl2) = if litres <= AMOUNT_EPS {
        (0.0, total_nacl, 0.0, total_cacl2)
    } else {
        let max_nacl = solubility_mol_per_l(Salt::Nacl, temperature_c) * litres;
        let max_cacl2 = solubility_mol_per_l(Salt::Cacl2, temperature_c) * litres;
        let aq_nacl = total_nacl.min(max_nacl);
        let aq_cacl2 = total_cacl2.min(max_cacl2);
        (
            aq_nacl,
            (total_nacl - aq_nacl).max(0.0),
            aq_cacl2,
            (total_cacl2 - aq_cacl2).max(0.0),
        )
    };

    set_aqueous_mol(item, "na+", aq_nacl);
    set_aqueous_mol(item, "ca2+", aq_cacl2);
    set_aqueous_mol(item, "cl-", aq_nacl + 2.0 * aq_cacl2);
    set_salt_solid(item, "nacl", solid_nacl, NACL_MOLAR_MASS_G_PER_MOL);
    set_salt_solid(item, "cacl2", solid_cacl2, CACL2_MOLAR_MASS_G_PER_MOL);
    remove_near_zero_water(item);
    sync_fill_ml(item);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::ItemProperties;

    fn dish_at(temperature_c: f64, water_ml: f64) -> SceneItem {
        SceneItem {
            id: "dish-1".into(),
            kind: "evaporation_dish".into(),
            label: "Evaporation dish".into(),
            location: "bench".into(),
            properties: ItemProperties {
                volume_ml: Some(25.0),
                fill_ml: Some(water_ml),
                transparent: Some(true),
                colourless: Some(true),
                temperature_c: Some(temperature_c),
                composition: if water_ml > 0.0 {
                    vec![CompositionEntry {
                        substance_id: "water".into(),
                        phase: "liquid".into(),
                        amount_ml: Some(water_ml),
                        amount_scoop: None,
                        amount_g: None,
                        amount_mol: None,
                    }]
                } else {
                    vec![]
                },
                ..ItemProperties::default()
            },
        }
    }

    #[test]
    fn nacl_solubility_interpolates_between_table_points() {
        let s20 = solubility_mol_per_l(Salt::Nacl, 20.0);
        let s100 = solubility_mol_per_l(Salt::Nacl, 100.0);
        let s60 = solubility_mol_per_l(Salt::Nacl, 60.0);
        assert!((s20 - NACL_SOLUBILITY_MOL_PER_L_20C).abs() < 1e-12);
        assert!((s100 - NACL_SOLUBILITY_MOL_PER_L_100C).abs() < 1e-12);
        assert!((s60 - 0.5 * (s20 + s100)).abs() < 1e-12);
        assert!(s100 > s20);
    }

    #[test]
    fn cacl2_solubility_is_higher_at_100c_than_20c() {
        assert!(solubility_mol_per_l(Salt::Cacl2, 100.0) > solubility_mol_per_l(Salt::Cacl2, 20.0));
    }

    #[test]
    fn excess_nacl_precipitates_and_aqueous_caps_at_saturation() {
        let mut dish = dish_at(20.0, 1.0);
        let max = solubility_mol_per_l(Salt::Nacl, 20.0) * 0.001;
        dish.properties.composition.push(CompositionEntry {
            substance_id: "na+".into(),
            phase: "aqueous".into(),
            amount_ml: None,
            amount_scoop: None,
            amount_g: None,
            amount_mol: Some(max + 0.01),
        });
        dish.properties.composition.push(CompositionEntry {
            substance_id: "cl-".into(),
            phase: "aqueous".into(),
            amount_ml: None,
            amount_scoop: None,
            amount_g: None,
            amount_mol: Some(max + 0.01),
        });
        enforce_saturation(&mut dish);
        let na = aqueous_mol(&dish, "na+");
        assert!((na - max).abs() < 1e-9, "aqueous na+ {na} vs cap {max}");
        let solid = dish
            .properties
            .composition
            .iter()
            .find(|c| c.substance_id == "nacl" && c.phase == "solid")
            .expect("solid nacl");
        assert!((solid.amount_mol.unwrap() - 0.01).abs() < 1e-9);
    }

    #[test]
    fn adding_water_redissolves_solid_nacl_up_to_solubility() {
        let mut dish = dish_at(20.0, 1.0);
        let max_1ml = solubility_mol_per_l(Salt::Nacl, 20.0) * 0.001;
        dish.properties.composition.push(CompositionEntry {
            substance_id: "nacl".into(),
            phase: "solid".into(),
            amount_ml: None,
            amount_scoop: None,
            amount_g: Some(0.02 * NACL_MOLAR_MASS_G_PER_MOL),
            amount_mol: Some(0.02),
        });
        enforce_saturation(&mut dish);
        assert!((aqueous_mol(&dish, "na+") - max_1ml).abs() < 1e-9);

        if let Some(water) = dish
            .properties
            .composition
            .iter_mut()
            .find(|c| c.substance_id == "water")
        {
            water.amount_ml = Some(10.0);
        }
        enforce_saturation(&mut dish);
        let max_10ml = solubility_mol_per_l(Salt::Nacl, 20.0) * 0.010;
        let expected_aq = 0.02_f64.min(max_10ml);
        assert!((aqueous_mol(&dish, "na+") - expected_aq).abs() < 1e-9);
        assert!(
            dish.properties
                .composition
                .iter()
                .all(|c| !(c.substance_id == "nacl" && c.phase == "solid")),
            "10 ml should dissolve 0.02 mol NaCl at 20 °C"
        );
    }
}
