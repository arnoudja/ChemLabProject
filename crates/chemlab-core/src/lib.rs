//! ChemLab core domain.
//!
//! Chemistry rules live here. This crate currently exposes a two-row dissolve
//! lookup (NaCl vs sand in water at bench temperature), not a general simulator.

mod dissolve;
mod scene;

pub use dissolve::{dissolve, DissolveError, DissolveOutcome};
pub use scene::{
    apply_action, initial_bench_scene, Action, CompositionEntry, ItemProperties, Scene, SceneError,
    SPOON_SCOOP_MASS_G,
    SceneEvent, SceneItem,
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
