//! ChemLab core domain.
//!
//! Chemistry rules live here. This crate currently exposes a small dissolve
//! lookup (NaCl / CaCl₂ / NaOH / sand in water) plus the lab scene engine — scoop,
//! pipette transfers, burner heat, evaporation, and mixed-salt saturation.

pub mod challenges;
mod composition;
mod dissolve;
mod hcl;
mod scene;
mod solubility;

pub use dissolve::{dissolve, DissolveError, DissolveOutcome};
pub use hcl::{
    hcl_aq_density_g_per_ml, hcl_boil_temperature_c, ph_of_item, solution_volume_ml,
    HCL_AZEOTROPE_BOIL_C, HCL_AZEOTROPE_W_W, HCL_MOLAR_MASS_G_PER_MOL, HCL_STOCK_CAPACITY_ML,
    HCL_STOCK_DENSITY_G_PER_ML, HCL_STOCK_HCL_MASS_G, HCL_STOCK_HCL_MOLES, HCL_STOCK_TOTAL_MASS_G,
    HCL_STOCK_WATER_MASS_G, HCL_STOCK_W_W,
};
pub use scene::{
    apply_action, apply_elapsed, boiling_temperature_c, effective_heat_capacity,
    ensure_default_bench_items, initial_bench_scene, initial_scene_for_mode, water_mole_fraction,
    water_vapor_pressure_bar, Action, CompositionEntry, ItemProperties, Scene, SceneError,
    SceneEvent, SceneItem, AMBIENT_TEMPERATURE_C, ATM_PRESSURE_BAR, BOILING_TEMPERATURE_C,
    BURNER_POWER_W, CP_CACL2, CP_NACL, CP_NAOH, CP_SAND, C_BEAKER, C_DISH, DISH_CAPACITY_ML,
    DISH_EVAP_AREA_M2, DISH_MASS_TRANSFER_COEFF_ML_PER_S_M2, DISTILLED_WATER_CAPACITY_ML,
    FILTRATE_CAPACITY_ML, H_OH_NEUTRALIZATION_J_PER_MOL, NACL_DELTA_H_SOLUTION_J_PER_MOL,
    NAOH_DELTA_H_SOLUTION_J_PER_MOL, PIPETTE_MIN_SOURCE_ML, PIPETTE_VOLUME_ML, RELATIVE_HUMIDITY,
    SPOON_SCOOP_MASS_G, UA_BEAKER, UA_DISH, WATER_ANTOINE_A, WATER_ANTOINE_B, WATER_ANTOINE_C,
    WATER_CAPACITY_ML, WATER_LATENT_HEAT_J_PER_G, WATER_MOLAR_MASS_G_PER_MOL,
    WATER_SPECIFIC_HEAT_J_PER_G_K,
};

/// Semantic version of the ChemLab core crate.
pub const CORE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Placeholder identity for the future lab simulation engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabEngine {
    name: String,
}

impl LabEngine {
    /// Create a named engine placeholder (no simulation yet).
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    /// Engine display name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Whether the engine can run chemistry (false until v1).
    pub fn can_simulate(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_placeholder_is_not_simulating_yet() {
        let engine = LabEngine::new("chemlab-core");
        assert_eq!(engine.name(), "chemlab-core");
        assert!(!engine.can_simulate());
    }

    #[test]
    fn core_version_is_semver_like() {
        assert!(CORE_VERSION.starts_with('0'));
        assert!(CORE_VERSION.contains('.'));
    }
}
