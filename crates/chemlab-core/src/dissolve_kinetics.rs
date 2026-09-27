//! School first-order dissolve kinetics for vessels and filter wash.
//!
//! Rate law: `Δm = min(avail, unsaturated_cap) · (1 − e^(−k(T)·τ))` with a
//! per-salt `k₂₀` table and mild school `k(T)`. Sand never dissolves (`k = 0`).
//! NaOH is highly soluble (no SI capacity gate). Qualitative [`crate::dissolve`]
//! stays a boolean soluble/insoluble flag only.

use crate::composition::{aqueous_mol_entries, solvent_water_ml_for_si};
use crate::scene::{
    apply_dissolution_temperature_change, author_dissolved_salt_ions, heat_capacity_of_entries,
    remove_solid_mass, solid_amount_g, CompositionEntry, SceneItem, AMBIENT_TEMPERATURE_C,
};
use crate::solubility::{unsaturated_capacity_g, Salt};

const AMOUNT_EPS: f64 = 1e-12;

/// Pour / tongs / pipette contact time applied once in finalize (s).
pub(crate) const POUR_CONTACT_TAU_S: f64 = 0.25;

/// Filter-paper wash contact time per ml of transferred fluid (s/ml).
pub(crate) const FILTER_WASH_TAU_S_PER_ML: f64 = 0.05;

/// Cap per kinetic call (matches HTTP clock clamp spirit).
const MAX_TAU_S: f64 = 2.0;

/// Sub-step size so dissolve ΔH updates T (and mild k(T)) within a long tick.
const SUBSTEP_TAU_S: f64 = 0.25;

/// Mild school activation scale (K) for `k(T) = k₂₀ · exp(α·(1/293.15 − 1/T_K))`.
const K_T_ALPHA_K: f64 = 800.0;

const T20_K: f64 = 293.15;

/// Per-salt first-order rate at 20 °C (1/s).
/// NaOH ≫ CaCl₂ > NaCl ≫ gypsum; sand = 0.
fn k20(salt_id: &str) -> f64 {
    match salt_id {
        // Challenge-friendly: scoop / 2 g NaCl loads clear in a few seconds when unsaturated;
        // NaOH scoop finishes within pour-contact τ (≤~1 s).
        "naoh" => 100.0,
        "cacl2" => 12.0,
        "nacl" => 8.0,
        "na2so4" => 2.0,
        "caso4" => 0.05,
        "sand" => 0.0,
        _ => 0.0,
    }
}

/// Temperature-adjusted rate (1/s). Unknown / sand → 0.
pub(crate) fn k_salt_t(salt_id: &str, temperature_c: f64) -> f64 {
    let k0 = k20(salt_id);
    if k0 <= 0.0 || !temperature_c.is_finite() {
        return 0.0;
    }
    let t_k = (temperature_c + 273.15).max(250.0);
    k0 * (K_T_ALPHA_K * (1.0 / T20_K - 1.0 / t_k)).exp()
}

/// First-order dissolved fraction for contact time `τ`.
pub(crate) fn dissolve_fraction(k: f64, tau_s: f64) -> f64 {
    if k <= 0.0 || tau_s <= 0.0 || !k.is_finite() || !tau_s.is_finite() {
        return 0.0;
    }
    1.0 - (-k * tau_s).exp()
}

/// Kinetic dissolve of soluble solids in a wet vessel for contact time `τ`.
///
/// Authors ions + ΔH only for the mass that dissolves this call; remainder stays
/// solid. No-op without liquid water or when `τ ≤ 0`.
pub(crate) fn apply_kinetic_dissolve(item: &mut SceneItem, tau_s: f64) {
    let mut remaining = if tau_s.is_finite() {
        tau_s.clamp(0.0, MAX_TAU_S)
    } else {
        0.0
    };
    while remaining > AMOUNT_EPS {
        let step = remaining.min(SUBSTEP_TAU_S);
        apply_kinetic_dissolve_step(item, step);
        remaining -= step;
    }
}

fn apply_kinetic_dissolve_step(item: &mut SceneItem, tau_s: f64) {
    let water_ml = solvent_water_ml_for_si(item);
    if water_ml <= AMOUNT_EPS || tau_s <= AMOUNT_EPS {
        return;
    }
    let t = item
        .properties
        .temperature_c
        .unwrap_or(AMBIENT_TEMPERATURE_C);

    for salt_id in ["naoh", "cacl2", "nacl", "na2so4", "caso4"] {
        let avail = item
            .properties
            .composition
            .iter()
            .find(|c| c.substance_id == salt_id && c.phase == "solid")
            .map(solid_amount_g)
            .unwrap_or(0.0);
        if avail <= AMOUNT_EPS {
            continue;
        }
        let k = k_salt_t(salt_id, t);
        let frac = dissolve_fraction(k, tau_s);
        if frac <= AMOUNT_EPS {
            continue;
        }
        let cap = vessel_unsaturated_cap_g(item, salt_id, water_ml, t);
        let mut m_diss = if salt_id == "naoh" {
            avail * frac
        } else {
            avail.min(cap) * frac
        };
        // Snap microscopic leftovers so challenge Exact mass and ion asserts stay clean.
        let max_diss = if salt_id == "naoh" {
            avail
        } else {
            avail.min(cap)
        };
        if max_diss - m_diss > 0.0 && max_diss - m_diss < 1e-7 {
            m_diss = max_diss;
        }
        if m_diss <= AMOUNT_EPS {
            continue;
        }
        remove_solid_mass(item, salt_id, m_diss);
        if let Some((moles, delta_h)) =
            author_dissolved_salt_ions(&mut item.properties.composition, salt_id, m_diss)
        {
            apply_dissolution_temperature_change(item, moles, delta_h, t);
        }
    }
}

/// Wash soluble solids from filter paper into a fluid parcel (same rate law).
///
/// Updates `fluid` ions and `fluid_t` with dissolve ΔH. Sand is never touched.
pub(crate) fn wash_paper_solids_into_fluid(
    paper: &mut SceneItem,
    fluid: &mut Vec<CompositionEntry>,
    fluid_t: &mut f64,
) {
    let v_fluid = crate::composition::solvent_water_ml_for_si_entries(fluid);
    if v_fluid <= AMOUNT_EPS {
        return;
    }

    let c_fluid = heat_capacity_of_entries(fluid);
    let c_paper = heat_capacity_of_entries(&paper.properties.composition);
    let paper_t = paper
        .properties
        .temperature_c
        .unwrap_or(AMBIENT_TEMPERATURE_C);
    let t_wash = if c_fluid + c_paper > AMOUNT_EPS {
        (c_fluid * *fluid_t + c_paper * paper_t) / (c_fluid + c_paper)
    } else {
        *fluid_t
    };

    let tau = FILTER_WASH_TAU_S_PER_ML * v_fluid;

    for salt_id in ["naoh", "cacl2", "nacl", "na2so4", "caso4"] {
        let avail = paper
            .properties
            .composition
            .iter()
            .find(|c| c.substance_id == salt_id && c.phase == "solid")
            .map(solid_amount_g)
            .unwrap_or(0.0);
        if avail <= AMOUNT_EPS {
            continue;
        }
        let k = k_salt_t(salt_id, t_wash);
        let frac = dissolve_fraction(k, tau);
        if frac <= AMOUNT_EPS {
            continue;
        }
        let cap = entries_unsaturated_cap_g(fluid, salt_id, v_fluid, t_wash);
        let mut m_diss = if salt_id == "naoh" {
            avail * frac
        } else {
            avail.min(cap) * frac
        };
        let max_diss = if salt_id == "naoh" {
            avail
        } else {
            avail.min(cap)
        };
        if max_diss - m_diss > 0.0 && max_diss - m_diss < 1e-7 {
            m_diss = max_diss;
        }
        if m_diss <= AMOUNT_EPS {
            continue;
        }
        remove_solid_mass(paper, salt_id, m_diss);
        if let Some((moles, delta_h)) = author_dissolved_salt_ions(fluid, salt_id, m_diss) {
            let c_eff = heat_capacity_of_entries(fluid);
            if c_eff > AMOUNT_EPS {
                *fluid_t -= moles * delta_h / c_eff;
            }
        }
    }
}

fn vessel_unsaturated_cap_g(item: &SceneItem, salt_id: &str, water_ml: f64, t: f64) -> f64 {
    entries_unsaturated_cap_g(&item.properties.composition, salt_id, water_ml, t)
}

fn entries_unsaturated_cap_g(
    entries: &[CompositionEntry],
    salt_id: &str,
    water_ml: f64,
    t: f64,
) -> f64 {
    let salt = match salt_id {
        "nacl" => Salt::Nacl,
        "cacl2" => Salt::Cacl2,
        "na2so4" => Salt::Na2so4,
        "caso4" => Salt::Caso4,
        _ => return f64::INFINITY, // NaOH / unknown: no SI gate
    };
    let n_oh = aqueous_mol_entries(entries, "oh-");
    let n_h_free = aqueous_mol_entries(entries, "h+");
    let n_naoh = if n_oh > n_h_free + 1e-9 {
        (n_oh - n_h_free).max(0.0)
    } else {
        0.0
    };
    let n_na = (aqueous_mol_entries(entries, "na+") - n_naoh).max(0.0);
    let n_ca = aqueous_mol_entries(entries, "ca2+");
    let n_hso4 = aqueous_mol_entries(entries, "hso4-");
    let n_h = n_h_free;
    let n_cl = aqueous_mol_entries(entries, "cl-");
    let n_so4 = aqueous_mol_entries(entries, "so4^2-") + n_hso4;
    unsaturated_capacity_g(salt, water_ml, n_na, n_ca, n_h, n_cl, n_oh, n_so4, t)
}

/// True when the vessel has liquid water and a kinetically soluble solid.
pub(crate) fn vessel_needs_kinetic_dissolve(item: &SceneItem) -> bool {
    if solvent_water_ml_for_si(item) <= AMOUNT_EPS {
        return false;
    }
    item.properties.composition.iter().any(|c| {
        c.phase == "solid"
            && matches!(
                c.substance_id.as_str(),
                "nacl" | "cacl2" | "naoh" | "na2so4" | "caso4"
            )
            && solid_amount_g(c) > AMOUNT_EPS
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composition::aqueous_mol;
    use crate::scene::ItemProperties;

    #[test]
    fn dissolve_fraction_matches_first_order() {
        let k: f64 = 0.8;
        let tau: f64 = 0.5;
        let expected = 1.0 - (-k * tau).exp();
        assert!((dissolve_fraction(k, tau) - expected).abs() < 1e-12);
        assert_eq!(dissolve_fraction(0.0, 1.0), 0.0);
        assert_eq!(dissolve_fraction(1.0, 0.0), 0.0);
    }

    #[test]
    fn k_increases_with_temperature() {
        let k20 = k_salt_t("nacl", 20.0);
        let k80 = k_salt_t("nacl", 80.0);
        assert!(k80 > k20 + 1e-9);
        assert_eq!(k_salt_t("sand", 80.0), 0.0);
    }

    #[test]
    fn salt_rate_ordering() {
        let t = 20.0;
        assert!(k_salt_t("naoh", t) > k_salt_t("cacl2", t));
        assert!(k_salt_t("cacl2", t) > k_salt_t("nacl", t));
        assert!(k_salt_t("nacl", t) > k_salt_t("caso4", t));
        assert_eq!(k_salt_t("sand", t), 0.0);
    }

    #[test]
    fn sand_and_saturated_cap_dissolve_nothing() {
        let mut beaker = SceneItem {
            id: "beaker".into(),
            kind: "beaker".into(),
            label: "Beaker".into(),
            location: "bench".into(),
            properties: ItemProperties {
                temperature_c: Some(20.0),
                composition: vec![
                    CompositionEntry {
                        substance_id: "water".into(),
                        phase: "liquid".into(),
                        amount_ml: Some(200.0),
                        amount_scoop: None,
                        amount_g: None,
                        amount_mol: None,
                    },
                    CompositionEntry {
                        substance_id: "sand".into(),
                        phase: "solid".into(),
                        amount_ml: None,
                        amount_scoop: Some(1),
                        amount_g: Some(0.2),
                        amount_mol: None,
                    },
                ],
                ..ItemProperties::default()
            },
        };
        apply_kinetic_dissolve(&mut beaker, 2.0);
        assert!(
            (solid_amount_g(
                beaker
                    .properties
                    .composition
                    .iter()
                    .find(|c| c.substance_id == "sand")
                    .unwrap()
            ) - 0.2)
                .abs()
                < 1e-12
        );
        assert!(aqueous_mol(&beaker, "na+") < 1e-12);

        let mut dish = SceneItem {
            id: "dish".into(),
            kind: "evaporation_dish".into(),
            label: "Dish".into(),
            location: "bench".into(),
            properties: ItemProperties {
                temperature_c: Some(20.0),
                composition: vec![
                    CompositionEntry {
                        substance_id: "water".into(),
                        phase: "liquid".into(),
                        amount_ml: Some(1.0),
                        amount_scoop: None,
                        amount_g: None,
                        amount_mol: None,
                    },
                    CompositionEntry {
                        substance_id: "nacl".into(),
                        phase: "solid".into(),
                        amount_ml: None,
                        amount_scoop: None,
                        amount_g: Some(1.0),
                        amount_mol: None,
                    },
                ],
                ..ItemProperties::default()
            },
        };
        // Long contact approaches the SI capacity; further time adds negligibly.
        apply_kinetic_dissolve(&mut dish, 2.0);
        apply_kinetic_dissolve(&mut dish, 2.0);
        let na = aqueous_mol(&dish, "na+");
        let cap_mol =
            crate::solubility::solubility_mol_per_l(crate::solubility::Salt::Nacl, 20.0) * 0.001;
        assert!(na > 1e-6);
        assert!(
            (na - cap_mol).abs() < 1e-4,
            "should sit near SI capacity: na={na} cap={cap_mol}"
        );
        let solid_left = dish
            .properties
            .composition
            .iter()
            .find(|c| c.substance_id == "nacl" && c.phase == "solid")
            .map(solid_amount_g)
            .unwrap_or(0.0);
        assert!(
            solid_left > 0.5,
            "excess solid remains when capacity-limited"
        );
    }
}
