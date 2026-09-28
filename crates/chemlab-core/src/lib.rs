//! ChemLab core domain.
//!
//! Chemistry rules live here. This crate currently exposes a small dissolve
//! lookup (NaCl / CaCl₂ / NaOH / sand in water) plus the lab scene engine — scoop,
//! pipette transfers, burner heat, evaporation, and mixed-salt saturation.

mod acid_base;
mod aqueous_pipeline;
pub mod challenges;
mod composition;
mod dissolve;
mod dissolve_kinetics;
mod h2so4;
mod hcl;
mod scene;
mod solubility;

pub use aqueous_pipeline::{
    CHEMICAL_SPIT_FRAC, NA2SO4_DELTA_H_SOLUTION_J_PER_MOL, NACL_DELTA_H_SOLUTION_J_PER_MOL,
    NAOH_DELTA_H_SOLUTION_J_PER_MOL,
};
pub use dissolve::{dissolve, DissolveError, DissolveOutcome};
pub use h2so4::{
    H2SO4_MOLAR_MASS_G_PER_MOL, H2SO4_REFORM_WATER_PER_ACID, H2SO4_STOCK_CAPACITY_ML,
    H2SO4_STOCK_DENSITY_G_PER_ML, H2SO4_STOCK_H2SO4_MASS_G, H2SO4_STOCK_H2SO4_MOLES,
    H2SO4_STOCK_TOTAL_MASS_G, H2SO4_STOCK_W_W,
};
pub use hcl::{
    hcl_aq_density_g_per_ml, hcl_boil_temperature_c, ph_of_item, solution_volume_ml,
    HCL_AZEOTROPE_BOIL_C, HCL_AZEOTROPE_W_W, HCL_MOLAR_MASS_G_PER_MOL, HCL_STOCK_CAPACITY_ML,
    HCL_STOCK_DENSITY_G_PER_ML, HCL_STOCK_HCL_MASS_G, HCL_STOCK_HCL_MOLES, HCL_STOCK_TOTAL_MASS_G,
    HCL_STOCK_WATER_MASS_G, HCL_STOCK_W_W, PHI_V_H2SO4_ML_PER_MOL, PHI_V_NA2SO4_ML_PER_MOL,
    PHI_V_NAHSO4_ML_PER_MOL,
};
pub use scene::{
    apply_action, apply_elapsed, boiling_temperature_c, effective_heat_capacity,
    ensure_default_bench_items, initial_bench_scene, initial_scene_for_mode,
    vessel_boil_temperature_c, water_mole_fraction, water_vapor_pressure_bar, Action,
    CompositionEntry, ItemProperties, Scene, SceneError, SceneEvent, SceneItem,
    AMBIENT_TEMPERATURE_C, ATM_PRESSURE_BAR, BOILING_TEMPERATURE_C, BURNER_POWER_W, CP_CACL2,
    CP_H2SO4_LIQUID, CP_NACL, CP_NAOH, CP_SAND, C_BEAKER, C_DISH, DISH_CAPACITY_ML,
    DISH_EVAP_AREA_M2, DISH_MASS_TRANSFER_COEFF_ML_PER_S_M2, DISTILLED_WATER_CAPACITY_ML,
    DISTILLED_WATER_ID, FILTRATE_CAPACITY_ML, H_OH_NEUTRALIZATION_J_PER_MOL, MAIN_BEAKER_ID,
    PIPETTE_MIN_SOURCE_ML, PIPETTE_VOLUME_ML, RELATIVE_HUMIDITY, SPOON_SCOOP_MASS_G, UA_BEAKER,
    UA_DISH, WATER_ANTOINE_A, WATER_ANTOINE_B, WATER_ANTOINE_C, WATER_CAPACITY_ML,
    WATER_LATENT_HEAT_J_PER_G, WATER_MOLAR_MASS_G_PER_MOL, WATER_SPECIFIC_HEAT_J_PER_G_K,
};

/// Semantic version of the ChemLab core crate.
pub const CORE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_version_is_semver_like() {
        assert!(CORE_VERSION.starts_with('0'));
        assert!(CORE_VERSION.contains('.'));
    }
}
