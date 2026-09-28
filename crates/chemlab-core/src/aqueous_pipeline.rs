//! Aqueous vessel finalize pipeline: ionize → kinetic dissolve → speciate → SI → reform.
//!
//! Owns salt ion authorship and dissolve ΔH so [`crate::dissolve_kinetics`] does not
//! depend on mutator-heavy `scene` helpers for ion writing.

use crate::scene::{
    add_or_increase_mol_in, effective_heat_capacity, take_composition_fraction,
    vessel_boil_temperature_c, CompositionEntry, SceneItem, AMBIENT_TEMPERATURE_C, AMOUNT_EPS,
    CACL2_MOLAR_MASS_G_PER_MOL, CASO4_MOLAR_MASS_G_PER_MOL, NA2SO4_MOLAR_MASS_G_PER_MOL,
    NACL_MOLAR_MASS_G_PER_MOL, NAOH_MOLAR_MASS_G_PER_MOL,
};

/// School fraction of vessel composition discarded when chemical heat would boil.
///
/// Lost as spray (not continuous evaporation). Mid-range of the 2–5% school band.
pub const CHEMICAL_SPIT_FRAC: f64 = 0.03;

/// Enthalpy of solution of NaCl at bench conditions (endothermic), J/mol.
pub const NACL_DELTA_H_SOLUTION_J_PER_MOL: f64 = 3880.0;

/// Enthalpy of solution of anhydrous CaCl₂ (exothermic), J/mol.
pub const CACL2_DELTA_H_SOLUTION_J_PER_MOL: f64 = -81300.0;

/// Enthalpy of solution of solid NaOH (exothermic), J/mol.
pub const NAOH_DELTA_H_SOLUTION_J_PER_MOL: f64 = -44500.0;

/// Enthalpy of solution of solid Na₂SO₄ (mildly exothermic), J/mol.
/// Shared with the Free-mode sulfate stock (#121) so kinetic redissolve and
/// carousel pour use the same heat.
pub const NA2SO4_DELTA_H_SOLUTION_J_PER_MOL: f64 = -2340.0;

/// Ordered finalize stages (ionize → kinetic → speciate → SI → reform → sync).
///
/// Locked by [`tests::finalize_stage_order_is_ionize_kinetic_speciate_si_reform`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FinalizeStage {
    IonizeH2so4,
    KineticDissolve,
    SpeciateWithHeat,
    EnforceSaturation,
    SpeciateNoHeatAfterSi,
    ReformH2so4,
    SpeciateNoHeatAfterReform,
    SyncFillMl,
}

impl FinalizeStage {
    pub(crate) const ALL: &'static [FinalizeStage] = &[
        FinalizeStage::IonizeH2so4,
        FinalizeStage::KineticDissolve,
        FinalizeStage::SpeciateWithHeat,
        FinalizeStage::EnforceSaturation,
        FinalizeStage::SpeciateNoHeatAfterSi,
        FinalizeStage::ReformH2so4,
        FinalizeStage::SpeciateNoHeatAfterReform,
        FinalizeStage::SyncFillMl,
    ];

    fn apply(self, item: &mut SceneItem, dissolve_tau_s: f64, allow_spit_mass: bool) -> bool {
        match self {
            FinalizeStage::IonizeH2so4 => {
                crate::h2so4::ionize_liquid_h2so4_in_water(item);
                false
            }
            FinalizeStage::KineticDissolve => crate::dissolve_kinetics::apply_kinetic_dissolve(
                item,
                dissolve_tau_s,
                allow_spit_mass,
            ),
            FinalizeStage::SpeciateWithHeat => {
                crate::acid_base::speciate_aqueous_acid_base(item, true, allow_spit_mass)
            }
            FinalizeStage::EnforceSaturation => {
                crate::solubility::enforce_saturation(item);
                false
            }
            FinalizeStage::SpeciateNoHeatAfterSi | FinalizeStage::SpeciateNoHeatAfterReform => {
                let _ = crate::acid_base::speciate_aqueous_acid_base(item, false, false);
                false
            }
            FinalizeStage::ReformH2so4 => {
                crate::h2so4::reform_aqueous_h2so4_to_liquid(item);
                false
            }
            FinalizeStage::SyncFillMl => {
                crate::solubility::sync_fill_ml(item);
                false
            }
        }
    }
}

/// Author salt aqueous ions into a composition list (vessel kinetic dissolve and
/// filter-paper wash). Returns `(moles_of_salt, ΔH_sol)` for temperature update.
pub(crate) fn author_dissolved_salt_ions(
    composition: &mut Vec<CompositionEntry>,
    substance_id: &str,
    mass_g: f64,
) -> Option<(f64, f64)> {
    if mass_g <= AMOUNT_EPS {
        return None;
    }
    match substance_id {
        "nacl" => {
            let moles = mass_g / NACL_MOLAR_MASS_G_PER_MOL;
            add_or_increase_mol_in(composition, "na+", "aqueous", moles);
            add_or_increase_mol_in(composition, "cl-", "aqueous", moles);
            Some((moles, NACL_DELTA_H_SOLUTION_J_PER_MOL))
        }
        "cacl2" => {
            let moles = mass_g / CACL2_MOLAR_MASS_G_PER_MOL;
            add_or_increase_mol_in(composition, "ca2+", "aqueous", moles);
            add_or_increase_mol_in(composition, "cl-", "aqueous", 2.0 * moles);
            Some((moles, CACL2_DELTA_H_SOLUTION_J_PER_MOL))
        }
        "naoh" => {
            let moles = mass_g / NAOH_MOLAR_MASS_G_PER_MOL;
            add_or_increase_mol_in(composition, "na+", "aqueous", moles);
            add_or_increase_mol_in(composition, "oh-", "aqueous", moles);
            Some((moles, NAOH_DELTA_H_SOLUTION_J_PER_MOL))
        }
        "na2so4" => {
            let moles = mass_g / NA2SO4_MOLAR_MASS_G_PER_MOL;
            add_or_increase_mol_in(composition, "na+", "aqueous", 2.0 * moles);
            add_or_increase_mol_in(composition, "so4^2-", "aqueous", moles);
            Some((moles, NA2SO4_DELTA_H_SOLUTION_J_PER_MOL))
        }
        "caso4" => {
            let moles = mass_g / CASO4_MOLAR_MASS_G_PER_MOL;
            add_or_increase_mol_in(composition, "ca2+", "aqueous", moles);
            add_or_increase_mol_in(composition, "so4^2-", "aqueous", moles);
            Some((moles, 0.0))
        }
        _ => None,
    }
}

/// Ionize liquid H₂SO₄, kinetic-dissolve solids, speciate acid/base, SI(precip), reform.
///
/// `dissolve_tau_s` is pour-contact or clock `dt` for the kinetic step. Liquid H₂SO₄
/// ionizes to `H⁺ + HSO₄⁻` when the water:acid ratio is above the ~98% w/w reform
/// threshold; [`crate::acid_base::speciate_aqueous_acid_base`] then solves `K_w` +
/// `K_a2`. SI is precipitate-only (under-saturated redissolve is kinetic-owned).
/// Free sulfuric below the reform threshold becomes liquid `h2so4`.
///
/// Returns `true` when any chemical-heat step discarded spit mass (caller emits
/// at most one `spit` event per action).
///
/// `allow_spit_mass`: action paths pass `true` (clamp + spray discard); clock
/// `apply_elapsed` passes `false` (clamp only — spit is action-scoped).
pub(crate) fn finalize_aqueous_vessel(
    item: &mut SceneItem,
    dissolve_tau_s: f64,
    allow_spit_mass: bool,
) -> bool {
    let mut spit = false;
    for stage in FinalizeStage::ALL {
        spit |= stage.apply(item, dissolve_tau_s, allow_spit_mass);
    }
    spit
}

/// Apply dissolution heat to the solvent vessel: ΔT = −(n·ΔH_sol) / C_eff.
///
/// Routes through [`apply_chemical_heat`] so boil/spit gating is shared with
/// dilution and neutralization. Returns `true` when spit mass was discarded on
/// this heat step.
pub(crate) fn apply_dissolution_temperature_change(
    target: &mut SceneItem,
    moles: f64,
    delta_h_j_per_mol: f64,
    allow_spit_mass: bool,
) -> bool {
    let water_ml = target
        .properties
        .composition
        .iter()
        .find(|c| c.substance_id == "water" && c.phase == "liquid")
        .and_then(|c| c.amount_ml)
        .unwrap_or(0.0);
    if water_ml <= 0.0 || moles <= 0.0 {
        return false;
    }
    let heat_j = moles * delta_h_j_per_mol;
    apply_chemical_heat(target, heat_j, allow_spit_mass)
}

/// Apply instant chemical heat `q_j` (negative ⇒ exothermic): `ΔT = −Q / C_eff`.
///
/// When the proposed temperature would reach or exceed [`vessel_boil_temperature_c`],
/// clamp at boil. When `allow_spit_mass` is true (action paths), also discard
/// [`CHEMICAL_SPIT_FRAC`] of composition and return `true` so the caller can emit
/// at most one silent `spit` event per action. Clock ticks pass `false` — clamp
/// only, no spray mass / event (spit is action-scoped). Burner continuous boil
/// stays on the thermal path.
pub(crate) fn apply_chemical_heat(target: &mut SceneItem, q_j: f64, allow_spit_mass: bool) -> bool {
    if q_j.abs() <= AMOUNT_EPS {
        return false;
    }
    let c_eff = effective_heat_capacity(target);
    if c_eff <= AMOUNT_EPS {
        return false;
    }
    let t = target
        .properties
        .temperature_c
        .unwrap_or(AMBIENT_TEMPERATURE_C);
    let t_proposed = t - q_j / c_eff;
    let t_boil = vessel_boil_temperature_c(target);
    // Already plateaued at boil from a prior chemical-heat step this action:
    // clamp only — do not spit mass again.
    if t + 1e-3 >= t_boil {
        target.properties.temperature_c = Some(t_boil);
        return false;
    }
    if t_proposed + 1e-6 < t_boil {
        target.properties.temperature_c = Some(t_proposed);
        return false;
    }
    target.properties.temperature_c = Some(t_boil);
    if !allow_spit_mass {
        return false;
    }
    let _discarded = take_composition_fraction(target, CHEMICAL_SPIT_FRAC);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finalize_stage_order_is_ionize_kinetic_speciate_si_reform() {
        assert_eq!(
            FinalizeStage::ALL,
            &[
                FinalizeStage::IonizeH2so4,
                FinalizeStage::KineticDissolve,
                FinalizeStage::SpeciateWithHeat,
                FinalizeStage::EnforceSaturation,
                FinalizeStage::SpeciateNoHeatAfterSi,
                FinalizeStage::ReformH2so4,
                FinalizeStage::SpeciateNoHeatAfterReform,
                FinalizeStage::SyncFillMl,
            ]
        );
        // Relative order locks (ionize before kinetic, SI before reform).
        let ionize = FinalizeStage::ALL
            .iter()
            .position(|&s| s == FinalizeStage::IonizeH2so4)
            .unwrap();
        let kinetic = FinalizeStage::ALL
            .iter()
            .position(|&s| s == FinalizeStage::KineticDissolve)
            .unwrap();
        let speciate = FinalizeStage::ALL
            .iter()
            .position(|&s| s == FinalizeStage::SpeciateWithHeat)
            .unwrap();
        let si = FinalizeStage::ALL
            .iter()
            .position(|&s| s == FinalizeStage::EnforceSaturation)
            .unwrap();
        let reform = FinalizeStage::ALL
            .iter()
            .position(|&s| s == FinalizeStage::ReformH2so4)
            .unwrap();
        assert!(ionize < kinetic);
        assert!(kinetic < speciate);
        assert!(speciate < si);
        assert!(si < reform);
    }
}
