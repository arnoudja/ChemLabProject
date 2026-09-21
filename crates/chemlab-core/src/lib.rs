//! ChemLab core domain.
//!
//! Chemistry rules live here. This crate currently exposes a small dissolve
//! lookup (NaCl / CaCl₂ / sand in water) plus the lab scene engine — scoop,
//! pipette transfers, burner heat, evaporation, and mixed-salt saturation.

pub mod challenges;
mod dissolve;
mod scene;
mod solubility;

pub use dissolve::{dissolve, DissolveError, DissolveOutcome};
pub use scene::{
    apply_action, apply_elapsed, ensure_default_bench_items, initial_bench_scene,
    initial_scene_for_mode, Action, CompositionEntry, ItemProperties, Scene, SceneError,
    SceneEvent, SceneItem, AMBIENT_TEMPERATURE_C, BOILING_TEMPERATURE_C, DISH_CAPACITY_ML,
    DISTILLED_WATER_CAPACITY_ML, EVAP_ML_PER_S, FILTRATE_CAPACITY_ML, HEAT_K_PER_S,
    NACL_DELTA_H_SOLUTION_J_PER_MOL, PIPETTE_VOLUME_ML, SPOON_SCOOP_MASS_G, WATER_CAPACITY_ML,
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
