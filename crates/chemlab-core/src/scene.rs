//! Lab scene engine: scoop with tools, pour into vessels, dissolve as a pour consequence.

use thiserror::Error;

use crate::dissolve::{dissolve, DissolveError};

/// Mass of one spoon scoop of solid, in grams.
pub const SPOON_SCOOP_MASS_G: f64 = 0.2;

/// Molar mass of NaCl used when converting scoop mass to aqueous ion moles.
const NACL_MOLAR_MASS_G_PER_MOL: f64 = 58.44;

/// One substance entry in an item's composition or holding list.
#[derive(Debug, Clone, PartialEq)]
pub struct CompositionEntry {
    pub substance_id: String,
    /// `"solid"` | `"liquid"` | `"aqueous"` for this slice.
    pub phase: String,
    pub amount_ml: Option<f64>,
    pub amount_scoop: Option<u32>,
    /// Mass in grams (solids). One spoon scoop is [`SPOON_SCOOP_MASS_G`].
    pub amount_g: Option<f64>,
    /// Amount of substance in moles (aqueous species).
    pub amount_mol: Option<f64>,
}

/// Physical / chemical properties of a lab item (server-authored).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ItemProperties {
    pub volume_ml: Option<f64>,
    pub fill_ml: Option<f64>,
    pub transparent: Option<bool>,
    pub colourless: Option<bool>,
    pub temperature_c: Option<i32>,
    pub composition: Vec<CompositionEntry>,
    pub holding: Vec<CompositionEntry>,
}

/// A single item in the lab scene (beaker, spoon, …).
#[derive(Debug, Clone, PartialEq)]
pub struct SceneItem {
    pub id: String,
    /// `"beaker"` | `"spoon"` | …
    pub kind: String,
    pub label: String,
    /// `"bench"` | `"hand"` | …
    pub location: String,
    pub properties: ItemProperties,
}

/// UI-facing event from the last applied action(s).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneEvent {
    /// e.g. `"scooped"`, `"poured"`, `"dissolved"`, `"did_not_dissolve"`.
    pub kind: String,
    pub message: String,
}

/// In-memory lab scene (domain model; wire conversion is the server's job).
#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    pub lab_id: String,
    pub version: u32,
    pub temperature_c: i32,
    pub items: Vec<SceneItem>,
    pub last_events: Vec<SceneEvent>,
}

/// Action applied to a scene (mirrors wire `LabAction` vocabulary).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    UseTool {
        tool_item_id: String,
        target_item_id: String,
    },
    Pour {
        source_item_id: String,
        target_item_id: String,
    },
    /// Replace the scene with a fresh default bench (same `lab_id` / `version`).
    Reset,
}

/// Errors when an action cannot be applied.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SceneError {
    #[error("unknown item")]
    UnknownItem,
    #[error("invalid action")]
    InvalidAction,
    #[error("empty holding")]
    EmptyHolding,
    #[error(transparent)]
    Dissolve(#[from] DissolveError),
}

/// Build the default four-item bench scene for a lab.
pub fn initial_bench_scene(lab_id: impl Into<String>) -> Scene {
    Scene {
        lab_id: lab_id.into(),
        version: 0,
        temperature_c: 20,
        last_events: Vec::new(),
        items: vec![
            SceneItem {
                id: "spoon-1".into(),
                kind: "spoon".into(),
                label: "Spoon".into(),
                location: "bench".into(),
                properties: ItemProperties::default(),
            },
            SceneItem {
                id: "beaker-nacl".into(),
                kind: "beaker".into(),
                label: "Sodium chloride".into(),
                location: "bench".into(),
                properties: ItemProperties {
                    volume_ml: Some(250.0),
                    fill_ml: Some(100.0),
                    transparent: Some(true),
                    colourless: Some(true),
                    temperature_c: Some(20),
                    composition: vec![CompositionEntry {
                        substance_id: "nacl".into(),
                        phase: "solid".into(),
                        amount_ml: None,
                        amount_scoop: Some(10),
                        amount_g: Some(10.0 * SPOON_SCOOP_MASS_G),
                        amount_mol: None,
                    }],
                    holding: Vec::new(),
                },
            },
            SceneItem {
                id: "beaker-sand".into(),
                kind: "beaker".into(),
                label: "Sand".into(),
                location: "bench".into(),
                properties: ItemProperties {
                    volume_ml: Some(250.0),
                    fill_ml: Some(100.0),
                    transparent: Some(true),
                    colourless: Some(true),
                    temperature_c: Some(20),
                    composition: vec![CompositionEntry {
                        substance_id: "sand".into(),
                        phase: "solid".into(),
                        amount_ml: None,
                        amount_scoop: Some(10),
                        amount_g: Some(10.0 * SPOON_SCOOP_MASS_G),
                        amount_mol: None,
                    }],
                    holding: Vec::new(),
                },
            },
            SceneItem {
                id: "beaker-water".into(),
                kind: "beaker".into(),
                label: "Water".into(),
                location: "bench".into(),
                properties: ItemProperties {
                    volume_ml: Some(250.0),
                    fill_ml: Some(200.0),
                    transparent: Some(true),
                    colourless: Some(true),
                    temperature_c: Some(20),
                    composition: vec![CompositionEntry {
                        substance_id: "water".into(),
                        phase: "liquid".into(),
                        amount_ml: Some(200.0),
                        amount_scoop: None,
                        amount_g: None,
                        amount_mol: None,
                    }],
                    holding: Vec::new(),
                },
            },
        ],
    }
}

/// Apply a single action, mutating the scene in place.
pub fn apply_action(scene: &mut Scene, action: Action) -> Result<(), SceneError> {
    scene.last_events.clear();
    match action {
        Action::UseTool {
            tool_item_id,
            target_item_id,
        } => apply_use_tool(scene, &tool_item_id, &target_item_id),
        Action::Pour {
            source_item_id,
            target_item_id,
        } => apply_pour(scene, &source_item_id, &target_item_id),
        Action::Reset => {
            apply_reset(scene);
            Ok(())
        }
    }
}

fn apply_reset(scene: &mut Scene) {
    let lab_id = scene.lab_id.clone();
    let version = scene.version;
    *scene = initial_bench_scene(lab_id);
    scene.version = version;
    scene.last_events.push(SceneEvent {
        kind: "reset".into(),
        message: "Lab reset to the starting bench.".into(),
    });
}

fn find_item_index(scene: &Scene, id: &str) -> Result<usize, SceneError> {
    scene
        .items
        .iter()
        .position(|i| i.id == id)
        .ok_or(SceneError::UnknownItem)
}

fn apply_use_tool(
    scene: &mut Scene,
    tool_item_id: &str,
    target_item_id: &str,
) -> Result<(), SceneError> {
    let tool_idx = find_item_index(scene, tool_item_id)?;
    let target_idx = find_item_index(scene, target_item_id)?;

    let tool_kind = scene.items[tool_idx].kind.as_str();
    if tool_kind != "spoon" {
        return Err(SceneError::InvalidAction);
    }

    let solid = scene.items[target_idx]
        .properties
        .composition
        .iter()
        .find(|c| c.phase == "solid" && (c.substance_id == "nacl" || c.substance_id == "sand"))
        .cloned()
        .ok_or(SceneError::InvalidAction)?;

    let scoop = CompositionEntry {
        substance_id: solid.substance_id.clone(),
        phase: "solid".into(),
        amount_ml: None,
        amount_scoop: Some(1),
        amount_g: Some(SPOON_SCOOP_MASS_G),
        amount_mol: None,
    };

    let tool = &mut scene.items[tool_idx];
    tool.location = "hand".into();
    tool.properties.holding = vec![scoop];

    scene.last_events.push(SceneEvent {
        kind: "scooped".into(),
        message: format!("Scooped {} onto the spoon.", solid.substance_id),
    });
    Ok(())
}

fn apply_pour(
    scene: &mut Scene,
    source_item_id: &str,
    target_item_id: &str,
) -> Result<(), SceneError> {
    let source_idx = find_item_index(scene, source_item_id)?;
    let target_idx = find_item_index(scene, target_item_id)?;

    if scene.items[source_idx].properties.holding.is_empty() {
        return Err(SceneError::EmptyHolding);
    }

    let held = scene.items[source_idx].properties.holding[0].clone();
    if held.phase != "solid" {
        return Err(SceneError::InvalidAction);
    }

    let target = &scene.items[target_idx];
    let has_water = target
        .properties
        .composition
        .iter()
        .any(|c| c.substance_id == "water" && c.phase == "liquid");
    if !has_water {
        return Err(SceneError::InvalidAction);
    }

    let temperature_c = target
        .properties
        .temperature_c
        .unwrap_or(scene.temperature_c);

    let outcome = dissolve(&held.substance_id, "water", temperature_c)?;

    // Clear source holding before mutating target (indices stay valid).
    scene.items[source_idx].properties.holding.clear();

    scene.last_events.push(SceneEvent {
        kind: "poured".into(),
        message: format!("Poured {} into the target.", held.substance_id),
    });

    let target = &mut scene.items[target_idx];
    if outcome.dissolved {
        // Server-authored composition for inspection. Dissolved NaCl is exposed as
        // aqueous ions (not a client-side dissociation of a substance_id blob).
        let mass_g = held.amount_g.unwrap_or(SPOON_SCOOP_MASS_G);
        if held.substance_id == "nacl" {
            let moles = mass_g / NACL_MOLAR_MASS_G_PER_MOL;
            add_or_increase_mol(target, "na+", "aqueous", moles);
            add_or_increase_mol(target, "cl-", "aqueous", moles);
        } else if let Some(existing) = target
            .properties
            .composition
            .iter_mut()
            .find(|c| c.substance_id == held.substance_id && c.phase == "aqueous")
        {
            existing.amount_scoop = Some(
                existing.amount_scoop.unwrap_or(0) + held.amount_scoop.unwrap_or(1),
            );
            existing.amount_g = Some(existing.amount_g.unwrap_or(0.0) + mass_g);
        } else {
            target.properties.composition.push(CompositionEntry {
                substance_id: held.substance_id,
                phase: "aqueous".into(),
                amount_ml: None,
                amount_scoop: held.amount_scoop,
                amount_g: Some(mass_g),
                amount_mol: None,
            });
        }
        scene.last_events.push(SceneEvent {
            kind: "dissolved".into(),
            message: outcome.explanation.into(),
        });
    } else {
        let scoops = held.amount_scoop.or(Some(1));
        let mass_g = held.amount_g.or(Some(SPOON_SCOOP_MASS_G));
        add_or_increase_solid(target, &held.substance_id, scoops, mass_g);
        scene.last_events.push(SceneEvent {
            kind: "did_not_dissolve".into(),
            message: outcome.explanation.into(),
        });
    }

    Ok(())
}

fn add_or_increase_mol(target: &mut SceneItem, substance_id: &str, phase: &str, moles: f64) {
    if let Some(existing) = target
        .properties
        .composition
        .iter_mut()
        .find(|c| c.substance_id == substance_id && c.phase == phase)
    {
        existing.amount_mol = Some(existing.amount_mol.unwrap_or(0.0) + moles);
        return;
    }
    target.properties.composition.push(CompositionEntry {
        substance_id: substance_id.into(),
        phase: phase.into(),
        amount_ml: None,
        amount_scoop: None,
        amount_g: None,
        amount_mol: Some(moles),
    });
}

fn add_or_increase_solid(
    target: &mut SceneItem,
    substance_id: &str,
    scoops: Option<u32>,
    mass_g: Option<f64>,
) {
    if let Some(existing) = target
        .properties
        .composition
        .iter_mut()
        .find(|c| c.substance_id == substance_id && c.phase == "solid")
    {
        if let Some(add) = scoops {
            existing.amount_scoop = Some(existing.amount_scoop.unwrap_or(0) + add);
        }
        if let Some(add) = mass_g {
            existing.amount_g = Some(existing.amount_g.unwrap_or(0.0) + add);
        }
        return;
    }
    target.properties.composition.push(CompositionEntry {
        substance_id: substance_id.into(),
        phase: "solid".into(),
        amount_ml: None,
        amount_scoop: scoops,
        amount_g: mass_g,
        amount_mol: None,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item<'a>(scene: &'a Scene, id: &str) -> &'a SceneItem {
        scene
            .items
            .iter()
            .find(|i| i.id == id)
            .unwrap_or_else(|| panic!("missing item {id}"))
    }

    #[test]
    fn initial_bench_scene_has_four_items_with_water_properties() {
        let scene = initial_bench_scene("lab-test");
        assert_eq!(scene.lab_id, "lab-test");
        assert_eq!(scene.temperature_c, 20);
        assert_eq!(scene.version, 0);
        assert!(scene.last_events.is_empty());

        let ids: Vec<_> = scene.items.iter().map(|i| i.id.as_str()).collect();
        assert!(ids.contains(&"spoon-1"));
        assert!(ids.contains(&"beaker-nacl"));
        assert!(ids.contains(&"beaker-sand"));
        assert!(ids.contains(&"beaker-water"));

        let water = item(&scene, "beaker-water");
        assert_eq!(water.kind, "beaker");
        assert_eq!(water.label, "Water");
        assert_eq!(water.location, "bench");
        assert_eq!(water.properties.volume_ml, Some(250.0));
        assert_eq!(water.properties.fill_ml, Some(200.0));
        assert_eq!(water.properties.transparent, Some(true));
        assert_eq!(water.properties.colourless, Some(true));
        assert_eq!(water.properties.temperature_c, Some(20));
        assert_eq!(water.properties.composition.len(), 1);
        assert_eq!(water.properties.composition[0].substance_id, "water");
        assert_eq!(water.properties.composition[0].phase, "liquid");
        assert_eq!(water.properties.composition[0].amount_ml, Some(200.0));

        let nacl = item(&scene, "beaker-nacl");
        assert!(nacl
            .properties
            .composition
            .iter()
            .any(|c| c.substance_id == "nacl" && c.phase == "solid"));

        let sand = item(&scene, "beaker-sand");
        assert!(sand
            .properties
            .composition
            .iter()
            .any(|c| c.substance_id == "sand" && c.phase == "solid"));

        let spoon = item(&scene, "spoon-1");
        assert_eq!(spoon.kind, "spoon");
        assert!(spoon.properties.holding.is_empty());
    }

    #[test]
    fn use_tool_spoon_scoops_nacl_without_dissolving() {
        let mut scene = initial_bench_scene("lab-test");
        apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "beaker-nacl".into(),
            },
        )
        .unwrap();

        let spoon = item(&scene, "spoon-1");
        assert_eq!(spoon.location, "hand");
        assert_eq!(spoon.properties.holding.len(), 1);
        assert_eq!(spoon.properties.holding[0].substance_id, "nacl");
        assert_eq!(spoon.properties.holding[0].phase, "solid");
        assert_eq!(spoon.properties.holding[0].amount_scoop, Some(1));
        assert_eq!(spoon.properties.holding[0].amount_g, Some(SPOON_SCOOP_MASS_G));
        assert_eq!(SPOON_SCOOP_MASS_G, 0.2);

        assert!(scene.last_events.iter().any(|e| e.kind == "scooped"));
        // Dissolve must not run yet — water still only water.
        let water = item(&scene, "beaker-water");
        assert!(water
            .properties
            .composition
            .iter()
            .all(|c| c.substance_id == "water"));
    }

    #[test]
    fn pour_nacl_into_water_dissolves_without_leftover_grains() {
        let mut scene = initial_bench_scene("lab-test");
        apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "beaker-nacl".into(),
            },
        )
        .unwrap();
        apply_action(
            &mut scene,
            Action::Pour {
                source_item_id: "spoon-1".into(),
                target_item_id: "beaker-water".into(),
            },
        )
        .unwrap();

        let spoon = item(&scene, "spoon-1");
        assert!(spoon.properties.holding.is_empty());

        let water = item(&scene, "beaker-water");
        assert!(water
            .properties
            .composition
            .iter()
            .any(|c| c.substance_id == "na+" && c.phase == "aqueous"));
        assert!(water
            .properties
            .composition
            .iter()
            .any(|c| c.substance_id == "cl-" && c.phase == "aqueous"));
        let expected_mol = SPOON_SCOOP_MASS_G / NACL_MOLAR_MASS_G_PER_MOL;
        let na = water
            .properties
            .composition
            .iter()
            .find(|c| c.substance_id == "na+" && c.phase == "aqueous")
            .unwrap();
        let cl = water
            .properties
            .composition
            .iter()
            .find(|c| c.substance_id == "cl-" && c.phase == "aqueous")
            .unwrap();
        assert!((na.amount_mol.unwrap() - expected_mol).abs() < 1e-12);
        assert!((cl.amount_mol.unwrap() - expected_mol).abs() < 1e-12);
        assert!(!water
            .properties
            .composition
            .iter()
            .any(|c| c.substance_id == "nacl"));
        assert!(scene.last_events.iter().any(|e| {
            e.kind == "dissolved"
                && e.message == "Sodium chloride (NaCl) dissolves in water at bench temperature."
        }));
    }

    #[test]
    fn second_nacl_pour_updates_existing_ion_moles_without_duplicate_lines() {
        let mut scene = initial_bench_scene("lab-test");
        for _ in 0..2 {
            apply_action(
                &mut scene,
                Action::UseTool {
                    tool_item_id: "spoon-1".into(),
                    target_item_id: "beaker-nacl".into(),
                },
            )
            .unwrap();
            apply_action(
                &mut scene,
                Action::Pour {
                    source_item_id: "spoon-1".into(),
                    target_item_id: "beaker-water".into(),
                },
            )
            .unwrap();
        }

        let water = item(&scene, "beaker-water");
        let na_count = water
            .properties
            .composition
            .iter()
            .filter(|c| c.substance_id == "na+" && c.phase == "aqueous")
            .count();
        let cl_count = water
            .properties
            .composition
            .iter()
            .filter(|c| c.substance_id == "cl-" && c.phase == "aqueous")
            .count();
        assert_eq!(na_count, 1);
        assert_eq!(cl_count, 1);
        let expected_mol = 2.0 * SPOON_SCOOP_MASS_G / NACL_MOLAR_MASS_G_PER_MOL;
        let na = water
            .properties
            .composition
            .iter()
            .find(|c| c.substance_id == "na+")
            .unwrap();
        assert!((na.amount_mol.unwrap() - expected_mol).abs() < 1e-12);
    }

    #[test]
    fn second_sand_pour_aggregates_solid_mass_on_one_line() {
        let mut scene = initial_bench_scene("lab-test");
        for _ in 0..2 {
            apply_action(
                &mut scene,
                Action::UseTool {
                    tool_item_id: "spoon-1".into(),
                    target_item_id: "beaker-sand".into(),
                },
            )
            .unwrap();
            apply_action(
                &mut scene,
                Action::Pour {
                    source_item_id: "spoon-1".into(),
                    target_item_id: "beaker-water".into(),
                },
            )
            .unwrap();
        }

        let water = item(&scene, "beaker-water");
        let sand_rows: Vec<_> = water
            .properties
            .composition
            .iter()
            .filter(|c| c.substance_id == "sand" && c.phase == "solid")
            .collect();
        assert_eq!(sand_rows.len(), 1);
        assert_eq!(sand_rows[0].amount_scoop, Some(2));
        assert!((sand_rows[0].amount_g.unwrap() - 2.0 * SPOON_SCOOP_MASS_G).abs() < 1e-12);
    }

    #[test]
    fn pour_sand_into_water_leaves_undissolved_solid() {
        let mut scene = initial_bench_scene("lab-test");
        apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "beaker-sand".into(),
            },
        )
        .unwrap();
        apply_action(
            &mut scene,
            Action::Pour {
                source_item_id: "spoon-1".into(),
                target_item_id: "beaker-water".into(),
            },
        )
        .unwrap();

        let spoon = item(&scene, "spoon-1");
        assert!(spoon.properties.holding.is_empty());

        let water = item(&scene, "beaker-water");
        assert!(water.properties.composition.iter().any(|c| {
            c.substance_id == "sand"
                && c.phase == "solid"
                && c.amount_scoop == Some(1)
                && c.amount_g == Some(SPOON_SCOOP_MASS_G)
        }));
        assert!(!water
            .properties
            .composition
            .iter()
            .any(|c| c.substance_id == "sand" && c.phase == "aqueous"));
        assert!(scene.last_events.iter().any(|e| {
            e.kind == "did_not_dissolve"
                && e.message == "Sand (silica) does not dissolve in water at bench temperature."
        }));
    }

    #[test]
    fn reset_restores_pure_water_and_clears_holding_after_pour() {
        let mut scene = initial_bench_scene("lab-test");
        apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "beaker-nacl".into(),
            },
        )
        .unwrap();
        apply_action(
            &mut scene,
            Action::Pour {
                source_item_id: "spoon-1".into(),
                target_item_id: "beaker-water".into(),
            },
        )
        .unwrap();
        scene.version = 2;

        apply_action(&mut scene, Action::Reset).unwrap();

        assert_eq!(scene.lab_id, "lab-test");
        assert_eq!(scene.version, 2);
        assert!(scene.last_events.iter().any(|e| e.kind == "reset"));

        let spoon = item(&scene, "spoon-1");
        assert_eq!(spoon.location, "bench");
        assert!(spoon.properties.holding.is_empty());

        let water = item(&scene, "beaker-water");
        assert_eq!(water.properties.composition.len(), 1);
        assert_eq!(water.properties.composition[0].substance_id, "water");
        assert_eq!(water.properties.composition[0].phase, "liquid");
        assert_eq!(water.properties.composition[0].amount_ml, Some(200.0));
    }

    #[test]
    fn unknown_item_ids_return_scene_error() {
        let mut scene = initial_bench_scene("lab-test");
        let err = apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "no-such-tool".into(),
                target_item_id: "beaker-nacl".into(),
            },
        )
        .unwrap_err();
        assert_eq!(err, SceneError::UnknownItem);

        let err = apply_action(
            &mut scene,
            Action::Pour {
                source_item_id: "spoon-1".into(),
                target_item_id: "no-such-beaker".into(),
            },
        )
        .unwrap_err();
        assert_eq!(err, SceneError::UnknownItem);
    }

    #[test]
    fn pour_with_empty_holding_returns_error() {
        let mut scene = initial_bench_scene("lab-test");
        let err = apply_action(
            &mut scene,
            Action::Pour {
                source_item_id: "spoon-1".into(),
                target_item_id: "beaker-water".into(),
            },
        )
        .unwrap_err();
        assert_eq!(err, SceneError::EmptyHolding);
    }

    #[test]
    fn pour_into_wrong_temperature_solvent_surfaces_dissolve_error() {
        let mut scene = initial_bench_scene("lab-test");
        apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "beaker-nacl".into(),
            },
        )
        .unwrap();

        let water = scene
            .items
            .iter_mut()
            .find(|i| i.id == "beaker-water")
            .unwrap();
        water.properties.temperature_c = Some(21);

        let err = apply_action(
            &mut scene,
            Action::Pour {
                source_item_id: "spoon-1".into(),
                target_item_id: "beaker-water".into(),
            },
        )
        .unwrap_err();
        assert_eq!(
            err,
            SceneError::Dissolve(DissolveError::UnsupportedTemperature)
        );
    }
}
