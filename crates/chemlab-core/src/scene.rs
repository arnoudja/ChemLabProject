//! Lab scene engine: scoop with tools, pour into vessels, dissolve as a pour consequence.

use thiserror::Error;

use crate::dissolve::{dissolve, DissolveError};

/// Mass of one spoon scoop of solid, in grams.
pub const SPOON_SCOOP_MASS_G: f64 = 0.2;

/// Molar mass of NaCl used when converting scoop mass to aqueous ion moles.
pub(crate) const NACL_MOLAR_MASS_G_PER_MOL: f64 = 58.44;

/// Molar mass of anhydrous CaCl₂ (g/mol).
pub(crate) const CACL2_MOLAR_MASS_G_PER_MOL: f64 = 110.98;

/// Pipette aliquot volume (ml).
pub const PIPETTE_VOLUME_ML: f64 = 1.00;

/// Evaporation dish capacity (ml).
pub const DISH_CAPACITY_ML: f64 = 25.00;

/// Water beaker liquid capacity (ml).
pub const WATER_CAPACITY_ML: f64 = 250.00;

/// Distilled-water stock beaker liquid capacity (ml).
pub const DISTILLED_WATER_CAPACITY_ML: f64 = 100.00;

/// Filtrate beaker liquid capacity (ml).
pub const FILTRATE_CAPACITY_ML: f64 = 250.00;

/// Burner heating rate while on and the dish is below boiling (°C/s).
pub const HEAT_K_PER_S: f64 = 10.0;

/// Water loss rate while the dish is at 100 °C and the burner is on (ml/s).
pub const EVAP_ML_PER_S: f64 = 0.50;

/// Ambient bench / reset temperature (°C).
pub const AMBIENT_TEMPERATURE_C: f64 = 20.0;

/// Boiling temperature; evaporation does not start below this (°C).
pub const BOILING_TEMPERATURE_C: f64 = 100.0;

const AMOUNT_EPS: f64 = 1e-12;

/// Enthalpy of solution of NaCl at bench conditions (endothermic), J/mol.
pub const NACL_DELTA_H_SOLUTION_J_PER_MOL: f64 = 3880.0;

/// Enthalpy of solution of anhydrous CaCl₂ (exothermic), J/mol.
pub const CACL2_DELTA_H_SOLUTION_J_PER_MOL: f64 = -81300.0;

/// Specific heat capacity of liquid water, J/(g·K). Mass of water ≈ volume in ml.
pub const WATER_SPECIFIC_HEAT_J_PER_G_K: f64 = 4.184;

fn is_stock_solid(substance_id: &str) -> bool {
    matches!(substance_id, "nacl" | "cacl2" | "sand")
}

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
    pub temperature_c: Option<f64>,
    pub composition: Vec<CompositionEntry>,
    pub holding: Vec<CompositionEntry>,
    /// Burner flame; `None` on non-burner items.
    pub on: Option<bool>,
    /// Last vessel a pipette drew from, the vessel tongs currently hold, or the
    /// dish a spoon scoop came from.
    pub source_item_id: Option<String>,
}

/// A single item in the lab scene (beaker, spoon, …).
#[derive(Debug, Clone, PartialEq)]
pub struct SceneItem {
    pub id: String,
    /// `"beaker"` | `"spoon"` | `"pipette"` | `"tongs"` | …
    pub kind: String,
    pub label: String,
    /// `"bench"` | `"hand"` | `"held"` | …
    pub location: String,
    pub properties: ItemProperties,
}

/// UI-facing event from the last applied action(s).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneEvent {
    /// e.g. `"scooped"`, `"returned"`, `"poured"`, `"dissolved"`, `"did_not_dissolve"`.
    pub kind: String,
    pub message: String,
}

/// In-memory lab scene (domain model; wire conversion is the server's job).
#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    pub lab_id: String,
    pub version: u32,
    pub temperature_c: f64,
    pub items: Vec<SceneItem>,
    pub last_events: Vec<SceneEvent>,
    /// Server clock watermark (unix ms) for elapsed heat/evaporation.
    pub last_applied_unix_ms: Option<i64>,
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
    /// Return the tool to the bench holder. Spoon scoops restore to the dish they
    /// came from, or to the matching stock if scooped from a jar.
    PutAway { tool_item_id: String },
    /// Replace the scene with a fresh default bench (same `lab_id` / `version`).
    Reset,
    /// Idle click on the burner. Stays off when the dish has no liquid.
    ToggleBurner { burner_item_id: String },
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

/// Build the default bench scene for a lab.
pub fn initial_bench_scene(lab_id: impl Into<String>) -> Scene {
    Scene {
        lab_id: lab_id.into(),
        version: 0,
        temperature_c: 20.0,
        last_events: Vec::new(),
        last_applied_unix_ms: None,
        items: vec![
            SceneItem {
                id: "spoon-1".into(),
                kind: "spoon".into(),
                label: "Spoon".into(),
                location: "bench".into(),
                properties: ItemProperties::default(),
            },
            SceneItem {
                id: "beaker-h2o".into(),
                kind: "beaker".into(),
                label: "Distilled water".into(),
                location: "bench".into(),
                properties: ItemProperties {
                    volume_ml: Some(DISTILLED_WATER_CAPACITY_ML),
                    fill_ml: Some(DISTILLED_WATER_CAPACITY_ML),
                    transparent: Some(true),
                    colourless: Some(true),
                    temperature_c: Some(20.0),
                    composition: vec![CompositionEntry {
                        substance_id: "water".into(),
                        phase: "liquid".into(),
                        amount_ml: Some(DISTILLED_WATER_CAPACITY_ML),
                        amount_scoop: None,
                        amount_g: None,
                        amount_mol: None,
                    }],
                    holding: Vec::new(),
                    ..ItemProperties::default()
                },
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
                    temperature_c: Some(20.0),
                    composition: vec![CompositionEntry {
                        substance_id: "nacl".into(),
                        phase: "solid".into(),
                        amount_ml: None,
                        amount_scoop: Some(10),
                        amount_g: Some(10.0 * SPOON_SCOOP_MASS_G),
                        amount_mol: None,
                    }],
                    holding: Vec::new(),
                    ..ItemProperties::default()
                },
            },
            SceneItem {
                id: "beaker-cacl2".into(),
                kind: "beaker".into(),
                label: "Calcium chloride".into(),
                location: "bench".into(),
                properties: ItemProperties {
                    volume_ml: Some(250.0),
                    fill_ml: Some(100.0),
                    transparent: Some(true),
                    colourless: Some(true),
                    temperature_c: Some(20.0),
                    composition: vec![CompositionEntry {
                        substance_id: "cacl2".into(),
                        phase: "solid".into(),
                        amount_ml: None,
                        amount_scoop: Some(10),
                        amount_g: Some(10.0 * SPOON_SCOOP_MASS_G),
                        amount_mol: None,
                    }],
                    holding: Vec::new(),
                    ..ItemProperties::default()
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
                    temperature_c: Some(20.0),
                    composition: vec![CompositionEntry {
                        substance_id: "sand".into(),
                        phase: "solid".into(),
                        amount_ml: None,
                        amount_scoop: Some(10),
                        amount_g: Some(10.0 * SPOON_SCOOP_MASS_G),
                        amount_mol: None,
                    }],
                    holding: Vec::new(),
                    ..ItemProperties::default()
                },
            },
            SceneItem {
                id: "beaker-water".into(),
                kind: "beaker".into(),
                label: "Beaker".into(),
                location: "bench".into(),
                properties: ItemProperties {
                    volume_ml: Some(WATER_CAPACITY_ML),
                    fill_ml: Some(0.0),
                    transparent: Some(true),
                    colourless: Some(true),
                    temperature_c: Some(20.0),
                    ..ItemProperties::default()
                },
            },
            SceneItem {
                id: "tongs-1".into(),
                kind: "tongs".into(),
                label: "Tongs".into(),
                location: "bench".into(),
                properties: ItemProperties::default(),
            },
            SceneItem {
                id: "pipette-1".into(),
                kind: "pipette".into(),
                label: "Pipette".into(),
                location: "bench".into(),
                properties: ItemProperties {
                    volume_ml: Some(PIPETTE_VOLUME_ML),
                    ..ItemProperties::default()
                },
            },
            SceneItem {
                id: "beaker-filtrate".into(),
                kind: "beaker".into(),
                label: "Filtrate".into(),
                location: "bench".into(),
                properties: ItemProperties {
                    volume_ml: Some(FILTRATE_CAPACITY_ML),
                    fill_ml: Some(0.0),
                    transparent: Some(true),
                    colourless: Some(true),
                    temperature_c: Some(AMBIENT_TEMPERATURE_C),
                    ..ItemProperties::default()
                },
            },
            SceneItem {
                id: "filter-paper-1".into(),
                kind: "filter_paper".into(),
                label: "Filter paper".into(),
                location: "bench".into(),
                properties: ItemProperties::default(),
            },
            SceneItem {
                id: "dish-1".into(),
                kind: "evaporation_dish".into(),
                label: "Evaporation dish".into(),
                location: "bench".into(),
                properties: ItemProperties {
                    volume_ml: Some(DISH_CAPACITY_ML),
                    fill_ml: Some(0.0),
                    transparent: Some(true),
                    colourless: Some(true),
                    temperature_c: Some(AMBIENT_TEMPERATURE_C),
                    ..ItemProperties::default()
                },
            },
            SceneItem {
                id: "burner-1".into(),
                kind: "burner".into(),
                label: "Burner".into(),
                location: "bench".into(),
                properties: ItemProperties {
                    on: Some(false),
                    ..ItemProperties::default()
                },
            },
        ],
    }
}

/// Insert any default bench items missing from a persisted scene.
///
/// Labs saved before a catalog addition (e.g. `tongs-1`) keep their vessel
/// state; only absent ids are filled from [`initial_bench_scene`].
pub fn ensure_default_bench_items(scene: &mut Scene) {
    let defaults = initial_bench_scene(scene.lab_id.clone());
    for item in defaults.items {
        if !scene.items.iter().any(|existing| existing.id == item.id) {
            scene.items.push(item);
        }
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
        Action::PutAway { tool_item_id } => apply_put_away(scene, &tool_item_id),
        Action::Reset => {
            apply_reset(scene);
            Ok(())
        }
        Action::ToggleBurner { burner_item_id } => apply_toggle_burner(scene, &burner_item_id),
    }
}

fn apply_reset(scene: &mut Scene) {
    let lab_id = scene.lab_id.clone();
    let version = scene.version;
    let last_applied_unix_ms = scene.last_applied_unix_ms;
    *scene = initial_bench_scene(lab_id);
    scene.version = version;
    scene.last_applied_unix_ms = last_applied_unix_ms;
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
    if tool_kind == "pipette" {
        return apply_pipette_use(scene, tool_idx, target_idx);
    }
    if tool_kind == "tongs" {
        return apply_tongs_use(scene, tool_idx, target_idx);
    }
    if tool_kind != "spoon" {
        return Err(SceneError::InvalidAction);
    }

    if !scene.items[tool_idx].properties.holding.is_empty() {
        return apply_spoon_use_while_holding(scene, tool_idx, target_idx);
    }

    if scene.items[target_idx].id == "beaker-filtrate" {
        return Err(SceneError::InvalidAction);
    }

    if scene.items[target_idx].kind == "evaporation_dish"
        || scene.items[target_idx].kind == "filter_paper"
    {
        return apply_spoon_scoop_from_solids_vessel(scene, tool_idx, target_idx);
    }

    let solid_idx = scene.items[target_idx]
        .properties
        .composition
        .iter()
        .position(|c| c.phase == "solid" && is_stock_solid(&c.substance_id))
        .ok_or(SceneError::InvalidAction)?;

    let solid = &mut scene.items[target_idx].properties.composition[solid_idx];
    let scoops_available = solid.amount_scoop.unwrap_or(0);
    // Gate on remaining grams, not scoop count: dish/evaporation put-back adds
    // amount_g (often without amount_scoop), including leftovers shorter than 0.2 g.
    let mass_available = solid_amount_g(solid);
    if mass_available <= AMOUNT_EPS {
        return Err(SceneError::InvalidAction);
    }

    let take_all = mass_available + AMOUNT_EPS < SPOON_SCOOP_MASS_G;
    let take_g = if take_all {
        mass_available
    } else {
        SPOON_SCOOP_MASS_G
    };
    let remaining_g = mass_available - take_g;
    let emptied = remaining_g.abs() <= AMOUNT_EPS;

    let substance_id = solid.substance_id.clone();
    let taken_mol = solid.amount_mol.and_then(|moles| {
        let taken = if take_all {
            moles
        } else {
            moles * (take_g / mass_available)
        };
        (taken > AMOUNT_EPS).then_some(taken)
    });
    if emptied {
        solid.amount_mol = None;
    } else if let (Some(total), Some(taken)) = (solid.amount_mol, taken_mol) {
        let rest = total - taken;
        solid.amount_mol = (rest > AMOUNT_EPS).then_some(rest);
    }

    if emptied {
        solid.amount_scoop = Some(0);
    } else if !take_all && scoops_available >= 1 {
        solid.amount_scoop = Some(scoops_available - 1);
    }
    // Decrement grams by the taken mass so returned dish mass is not dropped
    // when scoops and grams disagree. Snap float dust to 0 so empty stock does
    // not keep a floor sliver in the beaker visual.
    solid.amount_g = Some(if emptied { 0.0 } else { remaining_g });

    let scoop = CompositionEntry {
        substance_id: substance_id.clone(),
        phase: "solid".into(),
        amount_ml: None,
        amount_scoop: if take_all { None } else { Some(1) },
        amount_g: Some(take_g),
        amount_mol: taken_mol,
    };

    let tool = &mut scene.items[tool_idx];
    tool.location = "hand".into();
    tool.properties.holding = vec![scoop];

    scene.last_events.push(SceneEvent {
        kind: "scooped".into(),
        message: format!("Scooped {substance_id} onto the spoon."),
    });
    Ok(())
}

fn holding_solid_species(holding: &[CompositionEntry]) -> Vec<String> {
    let mut ids = Vec::new();
    for entry in holding {
        if entry.phase == "solid" && !ids.iter().any(|id| id == &entry.substance_id) {
            ids.push(entry.substance_id.clone());
        }
    }
    ids
}

fn apply_spoon_use_while_holding(
    scene: &mut Scene,
    tool_idx: usize,
    target_idx: usize,
) -> Result<(), SceneError> {
    if scene.items[target_idx].id == "beaker-water" {
        let tool_id = scene.items[tool_idx].id.clone();
        let target_id = scene.items[target_idx].id.clone();
        return apply_pour(scene, &tool_id, &target_id);
    }

    if scene.items[target_idx].id == "beaker-filtrate"
        || scene.items[target_idx].kind == "filter_paper"
    {
        return Err(SceneError::InvalidAction);
    }

    let species = holding_solid_species(&scene.items[tool_idx].properties.holding);
    if species.len() != 1 {
        return Err(SceneError::InvalidAction);
    }
    apply_return_to_stock(scene, tool_idx, target_idx)
}

fn composition_has_liquid(item: &SceneItem) -> bool {
    item.properties
        .composition
        .iter()
        .any(|entry| entry.phase == "liquid" && entry.amount_ml.unwrap_or(0.0) > AMOUNT_EPS)
}

fn solid_amount_g(entry: &CompositionEntry) -> f64 {
    entry
        .amount_g
        .unwrap_or_else(|| entry.amount_scoop.unwrap_or(0) as f64 * SPOON_SCOOP_MASS_G)
}

fn total_solid_g(item: &SceneItem) -> f64 {
    item.properties
        .composition
        .iter()
        .filter(|entry| entry.phase == "solid")
        .map(solid_amount_g)
        .sum()
}

fn take_solids_by_mass(source: &mut SceneItem, take_g: f64) -> Vec<CompositionEntry> {
    let total = total_solid_g(source);
    if total <= AMOUNT_EPS || take_g <= AMOUNT_EPS {
        return Vec::new();
    }
    if take_g + AMOUNT_EPS >= total {
        return take_all_solids(source);
    }
    let frac = take_g / total;
    let original = std::mem::take(&mut source.properties.composition);
    let mut taken = Vec::new();
    let mut remaining = Vec::new();
    for entry in original {
        if entry.phase != "solid" {
            remaining.push(entry);
            continue;
        }
        if let Some(moved) = scale_entry(&entry, frac) {
            taken.push(moved);
        }
        if let Some(rest) = scale_entry(&entry, 1.0 - frac) {
            remaining.push(rest);
        }
    }
    source.properties.composition = remaining;
    crate::solubility::sync_fill_ml(source);
    taken
}

fn apply_spoon_scoop_from_solids_vessel(
    scene: &mut Scene,
    tool_idx: usize,
    target_idx: usize,
) -> Result<(), SceneError> {
    if scene.items[target_idx].location == "held" {
        return Err(SceneError::InvalidAction);
    }
    if composition_has_liquid(&scene.items[target_idx]) {
        return Err(SceneError::InvalidAction);
    }
    let total = total_solid_g(&scene.items[target_idx]);
    if total <= AMOUNT_EPS {
        return Err(SceneError::InvalidAction);
    }
    let take_g = total.min(SPOON_SCOOP_MASS_G);
    let source_id = scene.items[target_idx].id.clone();
    let source_kind = scene.items[target_idx].kind.clone();
    let taken = take_solids_by_mass(&mut scene.items[target_idx], take_g);
    if taken.is_empty() {
        return Err(SceneError::InvalidAction);
    }
    let tool = &mut scene.items[tool_idx];
    tool.location = "hand".into();
    tool.properties.holding = taken;
    tool.properties.source_item_id = Some(source_id);
    let place = if source_kind == "filter_paper" {
        "paper"
    } else {
        "dish"
    };
    scene.last_events.push(SceneEvent {
        kind: "scooped".into(),
        message: format!("Scooped solids from the {place}."),
    });
    Ok(())
}

fn apply_return_holding_to_solids_vessel(
    scene: &mut Scene,
    tool_idx: usize,
    dest_idx: usize,
) -> Result<(), SceneError> {
    let held = std::mem::take(&mut scene.items[tool_idx].properties.holding);
    if held.is_empty() {
        return Err(SceneError::InvalidAction);
    }
    for entry in held {
        if entry.phase != "solid" {
            continue;
        }
        add_or_increase_solid(
            &mut scene.items[dest_idx],
            &entry.substance_id,
            entry.amount_scoop,
            entry.amount_g,
        );
    }
    scene.items[tool_idx].properties.source_item_id = None;
    crate::solubility::sync_fill_ml(&mut scene.items[dest_idx]);
    let place = if scene.items[dest_idx].kind == "filter_paper" {
        "paper"
    } else {
        "dish"
    };
    scene.last_events.push(SceneEvent {
        kind: "returned".into(),
        message: format!("Returned solids to the {place}."),
    });
    Ok(())
}

/// Return held solid(s) of one species to its matching stock beaker.
fn apply_return_to_stock(
    scene: &mut Scene,
    tool_idx: usize,
    target_idx: usize,
) -> Result<(), SceneError> {
    let held_all = scene.items[tool_idx].properties.holding.clone();
    let species = holding_solid_species(&held_all);
    if species.len() != 1 {
        return Err(SceneError::InvalidAction);
    }
    let substance_id = species[0].clone();
    if !is_stock_solid(&substance_id) {
        return Err(SceneError::InvalidAction);
    }
    if scene.items[target_idx].kind != "beaker" {
        return Err(SceneError::InvalidAction);
    }
    if !scene.items[target_idx]
        .properties
        .composition
        .iter()
        .any(|c| c.phase == "solid" && c.substance_id == substance_id)
    {
        return Err(SceneError::InvalidAction);
    }

    for held in &held_all {
        if held.phase != "solid" || held.substance_id != substance_id {
            return Err(SceneError::InvalidAction);
        }
        add_or_increase_solid(
            &mut scene.items[target_idx],
            &held.substance_id,
            held.amount_scoop,
            held.amount_g,
        );
    }

    scene.items[tool_idx].properties.holding.clear();
    scene.items[tool_idx].properties.source_item_id = None;
    scene.last_events.push(SceneEvent {
        kind: "returned".into(),
        message: format!("Returned {substance_id} to the stock beaker."),
    });
    Ok(())
}

/// Put the spoon back on the bench. Dish scoops return to the dish; stock scoops
/// return to the matching stock beaker.
fn apply_put_away(scene: &mut Scene, tool_item_id: &str) -> Result<(), SceneError> {
    let tool_idx = find_item_index(scene, tool_item_id)?;
    let kind = scene.items[tool_idx].kind.as_str();
    if kind == "pipette" {
        return apply_pipette_put_away(scene, tool_idx);
    }
    if kind == "tongs" {
        return apply_tongs_put_away(scene, tool_idx);
    }
    if kind != "spoon" {
        return Err(SceneError::InvalidAction);
    }

    if !scene.items[tool_idx].properties.holding.is_empty() {
        let source_id = scene.items[tool_idx].properties.source_item_id.clone();
        if let Some(source_id) = source_id {
            let source_idx = find_item_index(scene, &source_id)?;
            if scene.items[source_idx].kind == "evaporation_dish"
                || scene.items[source_idx].kind == "filter_paper"
            {
                apply_return_holding_to_solids_vessel(scene, tool_idx, source_idx)?;
            } else {
                apply_return_to_stock(scene, tool_idx, source_idx)?;
            }
        } else {
            let substance_id = holding_solid_species(&scene.items[tool_idx].properties.holding)
                .into_iter()
                .next()
                .ok_or(SceneError::InvalidAction)?;
            let target_idx = find_matching_stock_index(scene, &substance_id)?;
            apply_return_to_stock(scene, tool_idx, target_idx)?;
        }
    }

    scene.items[tool_idx].location = "bench".into();
    Ok(())
}

fn find_matching_stock_index(scene: &Scene, substance_id: &str) -> Result<usize, SceneError> {
    scene
        .items
        .iter()
        .position(|item| {
            item.kind == "beaker"
                && item.properties.composition.iter().any(|c| {
                    c.phase == "solid"
                        && c.substance_id == substance_id
                        && is_stock_solid(substance_id)
                })
        })
        .ok_or(SceneError::InvalidAction)
}

fn apply_pour(
    scene: &mut Scene,
    source_item_id: &str,
    target_item_id: &str,
) -> Result<(), SceneError> {
    let source_idx = find_item_index(scene, source_item_id)?;
    let target_idx = find_item_index(scene, target_item_id)?;

    if scene.items[source_idx].kind == "pipette" {
        return apply_pipette_empty(scene, source_idx, target_idx);
    }

    if is_distilled_water_stock(&scene.items[target_idx])
        || is_filtrate_beaker(&scene.items[target_idx])
        || scene.items[target_idx].kind == "filter_paper"
    {
        return Err(SceneError::InvalidAction);
    }

    if scene.items[source_idx].properties.holding.is_empty() {
        return Err(SceneError::EmptyHolding);
    }

    let held_all = scene.items[source_idx].properties.holding.clone();
    if held_all.iter().any(|held| held.phase != "solid") {
        return Err(SceneError::InvalidAction);
    }

    let target = &scene.items[target_idx];
    let dest_is_main_beaker = target.id == "beaker-water";
    let has_water = target
        .properties
        .composition
        .iter()
        .any(|c| c.substance_id == "water" && c.phase == "liquid");
    if !has_water && !dest_is_main_beaker {
        return Err(SceneError::InvalidAction);
    }

    scene.items[source_idx].properties.holding.clear();
    scene.items[source_idx].properties.source_item_id = None;

    let poured_label = if held_all.len() == 1 {
        held_all[0].substance_id.clone()
    } else {
        "solids".into()
    };
    scene.last_events.push(SceneEvent {
        kind: "poured".into(),
        message: format!("Poured {poured_label} into the target."),
    });

    if dest_is_main_beaker && !has_water {
        for held in held_all {
            let scoops = held.amount_scoop.or(Some(1));
            let mass_g = held.amount_g.or(Some(SPOON_SCOOP_MASS_G));
            add_or_increase_solid(
                &mut scene.items[target_idx],
                &held.substance_id,
                scoops,
                mass_g,
            );
        }
        crate::solubility::enforce_saturation(&mut scene.items[target_idx]);
        return Ok(());
    }

    for held in held_all {
        let temperature_c = scene.items[target_idx]
            .properties
            .temperature_c
            .unwrap_or(scene.temperature_c);
        let outcome = dissolve(&held.substance_id, "water", temperature_c.round() as i32)?;
        mix_held_solid_into_water(
            &mut scene.items[target_idx],
            &held,
            temperature_c,
            outcome.dissolved,
        );
        scene.last_events.push(SceneEvent {
            kind: if outcome.dissolved {
                "dissolved".into()
            } else {
                "did_not_dissolve".into()
            },
            message: outcome.explanation.into(),
        });
    }

    crate::solubility::enforce_saturation(&mut scene.items[target_idx]);
    Ok(())
}

fn mix_held_solid_into_water(
    target: &mut SceneItem,
    held: &CompositionEntry,
    temperature_c: f64,
    dissolved: bool,
) {
    if dissolved {
        let mass_g = held.amount_g.unwrap_or(SPOON_SCOOP_MASS_G);
        if held.substance_id == "nacl" {
            let moles = mass_g / NACL_MOLAR_MASS_G_PER_MOL;
            add_or_increase_mol(target, "na+", "aqueous", moles);
            add_or_increase_mol(target, "cl-", "aqueous", moles);
            apply_dissolution_temperature_change(
                target,
                moles,
                NACL_DELTA_H_SOLUTION_J_PER_MOL,
                temperature_c,
            );
        } else if held.substance_id == "cacl2" {
            let moles = mass_g / CACL2_MOLAR_MASS_G_PER_MOL;
            add_or_increase_mol(target, "ca2+", "aqueous", moles);
            add_or_increase_mol(target, "cl-", "aqueous", 2.0 * moles);
            apply_dissolution_temperature_change(
                target,
                moles,
                CACL2_DELTA_H_SOLUTION_J_PER_MOL,
                temperature_c,
            );
        } else if let Some(existing) = target
            .properties
            .composition
            .iter_mut()
            .find(|c| c.substance_id == held.substance_id && c.phase == "aqueous")
        {
            existing.amount_scoop =
                Some(existing.amount_scoop.unwrap_or(0) + held.amount_scoop.unwrap_or(1));
            existing.amount_g = Some(existing.amount_g.unwrap_or(0.0) + mass_g);
        } else {
            target.properties.composition.push(CompositionEntry {
                substance_id: held.substance_id.clone(),
                phase: "aqueous".into(),
                amount_ml: None,
                amount_scoop: held.amount_scoop,
                amount_g: Some(mass_g),
                amount_mol: None,
            });
        }
    } else {
        let scoops = held.amount_scoop.or(Some(1));
        let mass_g = held.amount_g.or(Some(SPOON_SCOOP_MASS_G));
        add_or_increase_solid(target, &held.substance_id, scoops, mass_g);
    }
}

/// Apply dissolution heat to the solvent beaker: ΔT = −(n·ΔH_sol) / (m_water · c_p).
///
/// Water mass ≈ liquid `amount_ml` (density ≈ 1 g/ml). Endothermic ΔH cools; exothermic heats.
fn apply_dissolution_temperature_change(
    target: &mut SceneItem,
    moles: f64,
    delta_h_j_per_mol: f64,
    current_temperature_c: f64,
) {
    let water_ml = target
        .properties
        .composition
        .iter()
        .find(|c| c.substance_id == "water" && c.phase == "liquid")
        .and_then(|c| c.amount_ml)
        .unwrap_or(0.0);
    if water_ml <= 0.0 || moles <= 0.0 {
        return;
    }
    let heat_j = moles * delta_h_j_per_mol;
    let delta_t = -heat_j / (water_ml * WATER_SPECIFIC_HEAT_J_PER_G_K);
    target.properties.temperature_c = Some(current_temperature_c + delta_t);
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

fn is_distilled_water_stock(item: &SceneItem) -> bool {
    item.id == "beaker-h2o"
}

fn is_filtrate_beaker(item: &SceneItem) -> bool {
    item.id == "beaker-filtrate"
}

fn is_filter_unit_target(item: &SceneItem) -> bool {
    is_filtrate_beaker(item) || item.id == "filter-paper-1"
}

fn entry_has_amount(entry: &CompositionEntry) -> bool {
    entry.amount_ml.unwrap_or(0.0) > AMOUNT_EPS
        || entry.amount_g.unwrap_or(0.0) > AMOUNT_EPS
        || entry.amount_mol.unwrap_or(0.0) > AMOUNT_EPS
        || entry.amount_scoop.unwrap_or(0) > 0
}

/// Pure H2O: liquid water only, no ions or solids.
fn composition_is_pure_h2o(entries: &[CompositionEntry]) -> bool {
    let mut has_water = false;
    for entry in entries {
        if !entry_has_amount(entry) {
            continue;
        }
        if entry.substance_id == "water" && entry.phase == "liquid" {
            has_water = true;
            continue;
        }
        return false;
    }
    has_water
}

fn is_liquid_vessel(item: &SceneItem) -> bool {
    item.id == "beaker-water"
        || is_distilled_water_stock(item)
        || is_filtrate_beaker(item)
        || item.kind == "evaporation_dish"
}

fn pipette_holding_liquid_ml(pipette: &SceneItem) -> f64 {
    pipette
        .properties
        .holding
        .iter()
        .filter(|c| c.substance_id == "water" && c.phase == "liquid")
        .map(|c| c.amount_ml.unwrap_or(0.0))
        .sum()
}

fn apply_toggle_burner(scene: &mut Scene, burner_item_id: &str) -> Result<(), SceneError> {
    let burner_idx = find_item_index(scene, burner_item_id)?;
    if scene.items[burner_idx].kind != "burner" {
        return Err(SceneError::InvalidAction);
    }
    let dish_idx = scene
        .items
        .iter()
        .position(|item| item.kind == "evaporation_dish")
        .ok_or(SceneError::InvalidAction)?;
    let has_liquid = crate::solubility::dish_has_liquid(&scene.items[dish_idx]);
    let currently_on = scene.items[burner_idx].properties.on.unwrap_or(false);
    let next_on = if currently_on { false } else { has_liquid };
    scene.items[burner_idx].properties.on = Some(next_on);
    if next_on != currently_on {
        scene.last_events.push(SceneEvent {
            kind: "toggled".into(),
            message: if next_on {
                "Burner on.".into()
            } else {
                "Burner off.".into()
            },
        });
    }
    Ok(())
}

/// Apply burner heat / evaporation for `dt_s` seconds (no-op if the burner is off).
///
/// The HTTP layer clamps the clock delta (e.g. 0–2 s) before calling this.
pub fn apply_elapsed(scene: &mut Scene, dt_s: f64) {
    let dt = if dt_s.is_finite() { dt_s.max(0.0) } else { 0.0 };
    if dt <= 0.0 {
        return;
    }
    let Some(burner_idx) = scene.items.iter().position(|item| item.kind == "burner") else {
        return;
    };
    if scene.items[burner_idx].properties.on != Some(true) {
        return;
    }
    let Some(dish_idx) = scene
        .items
        .iter()
        .position(|item| item.kind == "evaporation_dish")
    else {
        return;
    };
    if scene.items[dish_idx].location != "bench" {
        scene.items[burner_idx].properties.on = Some(false);
        return;
    }
    if !crate::solubility::dish_has_liquid(&scene.items[dish_idx]) {
        scene.items[burner_idx].properties.on = Some(false);
        return;
    }
    let temperature = scene.items[dish_idx]
        .properties
        .temperature_c
        .unwrap_or(AMBIENT_TEMPERATURE_C);
    if temperature < BOILING_TEMPERATURE_C {
        scene.items[dish_idx].properties.temperature_c =
            Some((temperature + HEAT_K_PER_S * dt).min(BOILING_TEMPERATURE_C));
        crate::solubility::enforce_saturation(&mut scene.items[dish_idx]);
        return;
    }
    evaporate_water(&mut scene.items[dish_idx], dt);
    crate::solubility::enforce_saturation(&mut scene.items[dish_idx]);
    if !crate::solubility::dish_has_liquid(&scene.items[dish_idx]) {
        scene.items[burner_idx].properties.on = Some(false);
    }
}

fn evaporate_water(dish: &mut SceneItem, dt: f64) {
    let loss = EVAP_ML_PER_S * dt;
    let remain = (crate::solubility::liquid_water_ml(dish) - loss).max(0.0);
    if let Some(water) = dish
        .properties
        .composition
        .iter_mut()
        .find(|c| c.substance_id == "water" && c.phase == "liquid")
    {
        water.amount_ml = Some(remain);
    }
}

fn apply_pipette_use(
    scene: &mut Scene,
    tool_idx: usize,
    target_idx: usize,
) -> Result<(), SceneError> {
    if pipette_holding_liquid_ml(&scene.items[tool_idx]) > AMOUNT_EPS {
        return apply_pipette_empty(scene, tool_idx, target_idx);
    }
    apply_pipette_fill(scene, tool_idx, target_idx)
}

fn apply_pipette_fill(
    scene: &mut Scene,
    tool_idx: usize,
    target_idx: usize,
) -> Result<(), SceneError> {
    if !is_liquid_vessel(&scene.items[target_idx]) {
        return Err(SceneError::InvalidAction);
    }
    if is_filtrate_beaker(&scene.items[target_idx]) && scene.items[target_idx].location != "bench" {
        return Err(SceneError::InvalidAction);
    }
    if crate::solubility::liquid_water_ml(&scene.items[target_idx]) + AMOUNT_EPS < PIPETTE_VOLUME_ML
    {
        return Err(SceneError::InvalidAction);
    }
    let source_id = scene.items[target_idx].id.clone();
    let source_t = scene.items[target_idx]
        .properties
        .temperature_c
        .unwrap_or(scene.temperature_c);
    let aliquot = take_liquid_aliquot(&mut scene.items[target_idx], PIPETTE_VOLUME_ML)?;
    crate::solubility::enforce_saturation(&mut scene.items[target_idx]);

    let pipette = &mut scene.items[tool_idx];
    pipette.location = "hand".into();
    pipette.properties.holding = aliquot;
    pipette.properties.source_item_id = Some(source_id);
    pipette.properties.temperature_c = Some(source_t);
    pipette.properties.fill_ml = Some(PIPETTE_VOLUME_ML);
    pipette.properties.volume_ml = Some(PIPETTE_VOLUME_ML);

    scene.last_events.push(SceneEvent {
        kind: "pipetted".into(),
        message: "Filled the pipette with 1.00 ml of solution.".into(),
    });
    Ok(())
}

fn apply_pipette_empty(
    scene: &mut Scene,
    tool_idx: usize,
    target_idx: usize,
) -> Result<(), SceneError> {
    if pipette_holding_liquid_ml(&scene.items[tool_idx]) + AMOUNT_EPS < PIPETTE_VOLUME_ML {
        return Err(SceneError::EmptyHolding);
    }
    if !is_liquid_vessel(&scene.items[target_idx]) {
        return Err(SceneError::InvalidAction);
    }
    if scene.items[target_idx].kind == "evaporation_dish" {
        let current = crate::solubility::liquid_water_ml(&scene.items[target_idx]);
        if current + PIPETTE_VOLUME_ML > DISH_CAPACITY_ML + AMOUNT_EPS {
            return Err(SceneError::InvalidAction);
        }
    }
    if is_distilled_water_stock(&scene.items[target_idx]) {
        if !composition_is_pure_h2o(&scene.items[tool_idx].properties.holding) {
            return Err(SceneError::InvalidAction);
        }
        let current = crate::solubility::liquid_water_ml(&scene.items[target_idx]);
        let cap = scene.items[target_idx]
            .properties
            .volume_ml
            .unwrap_or(DISTILLED_WATER_CAPACITY_ML);
        if current + PIPETTE_VOLUME_ML > cap + AMOUNT_EPS {
            return Err(SceneError::InvalidAction);
        }
    }
    if is_filtrate_beaker(&scene.items[target_idx]) {
        if scene.items[target_idx].location != "bench" {
            return Err(SceneError::InvalidAction);
        }
        let current = crate::solubility::liquid_water_ml(&scene.items[target_idx]);
        if current + PIPETTE_VOLUME_ML > FILTRATE_CAPACITY_ML + AMOUNT_EPS {
            return Err(SceneError::InvalidAction);
        }
    }
    let aliquot = scene.items[tool_idx].properties.holding.clone();
    let aliquot_t = scene.items[tool_idx]
        .properties
        .temperature_c
        .unwrap_or(scene.temperature_c);
    mix_aliquot_into(&mut scene.items[target_idx], &aliquot, aliquot_t);
    crate::solubility::enforce_saturation(&mut scene.items[target_idx]);

    let pipette = &mut scene.items[tool_idx];
    pipette.properties.holding.clear();
    pipette.properties.source_item_id = None;
    pipette.properties.temperature_c = None;
    pipette.properties.fill_ml = Some(0.0);

    scene.last_events.push(SceneEvent {
        kind: "poured".into(),
        message: "Emptied the pipette into the vessel.".into(),
    });
    Ok(())
}

fn apply_pipette_put_away(scene: &mut Scene, tool_idx: usize) -> Result<(), SceneError> {
    if pipette_holding_liquid_ml(&scene.items[tool_idx]) > AMOUNT_EPS {
        let source_id = scene.items[tool_idx]
            .properties
            .source_item_id
            .clone()
            .ok_or(SceneError::InvalidAction)?;
        let target_idx = find_item_index(scene, &source_id)?;
        apply_pipette_empty(scene, tool_idx, target_idx)?;
    }
    scene.items[tool_idx].location = "bench".into();
    Ok(())
}

fn is_solid_stock_beaker(item: &SceneItem) -> bool {
    matches!(
        item.id.as_str(),
        "beaker-nacl" | "beaker-cacl2" | "beaker-sand"
    )
}

fn is_tongs_vessel(item: &SceneItem) -> bool {
    item.id == "beaker-water" || is_distilled_water_stock(item) || item.kind == "evaporation_dish"
}

fn is_tongs_pickup_target(item: &SceneItem) -> bool {
    is_tongs_vessel(item) || is_solid_stock_beaker(item)
}

fn vessel_liquid_capacity_ml(item: &SceneItem) -> Option<f64> {
    if !is_tongs_vessel(item) {
        return None;
    }
    item.properties.volume_ml
}

fn composition_has_solids(item: &SceneItem) -> bool {
    item.properties.composition.iter().any(|c| {
        c.phase == "solid"
            && (c.amount_g.unwrap_or(0.0) > AMOUNT_EPS
                || c.amount_scoop.unwrap_or(0) > 0
                || c.amount_mol.unwrap_or(0.0) > AMOUNT_EPS)
    })
}

fn apply_tongs_use(
    scene: &mut Scene,
    tool_idx: usize,
    target_idx: usize,
) -> Result<(), SceneError> {
    if let Some(held_id) = scene.items[tool_idx].properties.source_item_id.as_deref() {
        if let Ok(source_idx) = find_item_index(scene, held_id) {
            if is_solid_stock_beaker(&scene.items[source_idx])
                && scene.items[target_idx].id != "beaker-water"
            {
                return Err(SceneError::InvalidAction);
            }
        }
    }
    if is_filter_unit_target(&scene.items[target_idx]) {
        return apply_tongs_filter_unit(scene, tool_idx);
    }
    if !is_tongs_pickup_target(&scene.items[target_idx]) {
        return Err(SceneError::InvalidAction);
    }
    match scene.items[tool_idx].properties.source_item_id.clone() {
        None => apply_tongs_pick_up(scene, tool_idx, target_idx),
        Some(held_id) if held_id == scene.items[target_idx].id => Err(SceneError::InvalidAction),
        Some(_) => apply_tongs_pour(scene, tool_idx, target_idx),
    }
}

fn apply_tongs_filter_unit(scene: &mut Scene, tool_idx: usize) -> Result<(), SceneError> {
    match scene.items[tool_idx].properties.source_item_id.clone() {
        None => {
            let filtrate_idx = find_item_index(scene, "beaker-filtrate")?;
            if scene.items[filtrate_idx].location != "bench" {
                return Err(SceneError::InvalidAction);
            }
            apply_tongs_pick_up(scene, tool_idx, filtrate_idx)
        }
        Some(held_id) if held_id == "beaker-filtrate" => Err(SceneError::InvalidAction),
        Some(_) => apply_filter_pour(scene, tool_idx),
    }
}

fn apply_filter_pour(scene: &mut Scene, tool_idx: usize) -> Result<(), SceneError> {
    let source_id = scene.items[tool_idx]
        .properties
        .source_item_id
        .clone()
        .ok_or(SceneError::InvalidAction)?;
    let source_idx = find_item_index(scene, &source_id)?;
    let dest_idx = find_item_index(scene, "beaker-filtrate")?;
    let paper_idx = find_item_index(scene, "filter-paper-1")?;
    if source_idx == dest_idx {
        return Err(SceneError::InvalidAction);
    }
    if scene.items[dest_idx].location != "bench" {
        return Err(SceneError::InvalidAction);
    }

    let source_liquid = crate::solubility::liquid_water_ml(&scene.items[source_idx]);
    let dest_liquid = crate::solubility::liquid_water_ml(&scene.items[dest_idx]);
    let dest_room = (FILTRATE_CAPACITY_ML - dest_liquid).max(0.0);
    let has_solids = composition_has_solids(&scene.items[source_idx]);

    if source_liquid <= AMOUNT_EPS {
        if has_solids {
            return Err(SceneError::InvalidAction);
        }
        return Err(SceneError::EmptyHolding);
    }
    if dest_room <= AMOUNT_EPS {
        return Err(SceneError::InvalidAction);
    }

    let transferred = source_liquid.min(dest_room);
    let frac = transferred / source_liquid;
    let source_t = scene.items[source_idx]
        .properties
        .temperature_c
        .unwrap_or(scene.temperature_c);
    let taken = take_composition_fraction(&mut scene.items[source_idx], frac);
    let mut fluid = Vec::new();
    let mut solids = Vec::new();
    for entry in taken {
        if entry.phase == "solid" {
            solids.push(entry);
        } else {
            fluid.push(entry);
        }
    }
    mix_transfer_into(&mut scene.items[dest_idx], &fluid, Some(source_t));
    mix_transfer_into(&mut scene.items[paper_idx], &solids, None);
    crate::solubility::enforce_saturation(&mut scene.items[source_idx]);
    crate::solubility::enforce_saturation(&mut scene.items[dest_idx]);
    scene.last_events.push(SceneEvent {
        kind: "poured".into(),
        message: "Filtered into the filtrate beaker.".into(),
    });
    Ok(())
}

fn apply_tongs_pick_up(
    scene: &mut Scene,
    tool_idx: usize,
    target_idx: usize,
) -> Result<(), SceneError> {
    let vessel_id = scene.items[target_idx].id.clone();
    scene.items[target_idx].location = "held".into();
    scene.items[tool_idx].location = "hand".into();
    scene.items[tool_idx].properties.source_item_id = Some(vessel_id.clone());
    if scene.items[target_idx].kind == "evaporation_dish" {
        if let Some(burner) = scene.items.iter_mut().find(|item| item.kind == "burner") {
            burner.properties.on = Some(false);
        }
    }
    scene.last_events.push(SceneEvent {
        kind: "picked".into(),
        message: format!("Picked up {vessel_id} with the tongs."),
    });
    Ok(())
}

fn apply_tongs_pour(scene: &mut Scene, tool_idx: usize, dest_idx: usize) -> Result<(), SceneError> {
    let source_id = scene.items[tool_idx]
        .properties
        .source_item_id
        .clone()
        .ok_or(SceneError::InvalidAction)?;
    let source_idx = find_item_index(scene, &source_id)?;
    if source_idx == dest_idx {
        return Err(SceneError::InvalidAction);
    }
    if is_solid_stock_beaker(&scene.items[source_idx]) {
        if scene.items[dest_idx].id != "beaker-water" {
            return Err(SceneError::InvalidAction);
        }
    } else if !is_tongs_vessel(&scene.items[dest_idx]) {
        return Err(SceneError::InvalidAction);
    }

    let source_liquid = crate::solubility::liquid_water_ml(&scene.items[source_idx]);
    let dest_cap =
        vessel_liquid_capacity_ml(&scene.items[dest_idx]).ok_or(SceneError::InvalidAction)?;
    let dest_liquid = crate::solubility::liquid_water_ml(&scene.items[dest_idx]);
    let dest_room = (dest_cap - dest_liquid).max(0.0);
    let has_solids = composition_has_solids(&scene.items[source_idx]);

    if is_distilled_water_stock(&scene.items[dest_idx]) {
        if dest_room <= AMOUNT_EPS {
            return Err(SceneError::InvalidAction);
        }
        if !composition_is_pure_h2o(&scene.items[source_idx].properties.composition) {
            return Err(SceneError::InvalidAction);
        }
    }

    if source_liquid <= AMOUNT_EPS {
        if !has_solids {
            return Err(SceneError::EmptyHolding);
        }
        return dump_all_solids(scene, source_idx, dest_idx);
    }

    if dest_room <= AMOUNT_EPS {
        if has_solids {
            return dump_all_solids(scene, source_idx, dest_idx);
        }
        return Err(SceneError::InvalidAction);
    }

    let transferred = source_liquid.min(dest_room);
    let frac = transferred / source_liquid;
    let source_t = scene.items[source_idx]
        .properties
        .temperature_c
        .unwrap_or(scene.temperature_c);
    let taken = take_composition_fraction(&mut scene.items[source_idx], frac);
    mix_transfer_into(&mut scene.items[dest_idx], &taken, Some(source_t));
    crate::solubility::enforce_saturation(&mut scene.items[source_idx]);
    crate::solubility::enforce_saturation(&mut scene.items[dest_idx]);
    scene.last_events.push(SceneEvent {
        kind: "poured".into(),
        message: "Poured from the held vessel.".into(),
    });
    Ok(())
}

fn dump_all_solids(
    scene: &mut Scene,
    source_idx: usize,
    dest_idx: usize,
) -> Result<(), SceneError> {
    let taken = take_all_solids(&mut scene.items[source_idx]);
    mix_transfer_into(&mut scene.items[dest_idx], &taken, None);
    crate::solubility::enforce_saturation(&mut scene.items[source_idx]);
    crate::solubility::enforce_saturation(&mut scene.items[dest_idx]);
    scene.last_events.push(SceneEvent {
        kind: "poured".into(),
        message: "Poured solids into the vessel.".into(),
    });
    Ok(())
}

fn apply_tongs_put_away(scene: &mut Scene, tool_idx: usize) -> Result<(), SceneError> {
    if let Some(held_id) = scene.items[tool_idx].properties.source_item_id.clone() {
        let held_idx = find_item_index(scene, &held_id)?;
        scene.items[held_idx].location = "bench".into();
        scene.items[tool_idx].properties.source_item_id = None;
    }
    scene.items[tool_idx].location = "bench".into();
    Ok(())
}

fn scale_opt_f64(value: Option<f64>, frac: f64) -> Option<f64> {
    value.and_then(|amount| {
        let scaled = amount * frac;
        if scaled > AMOUNT_EPS {
            Some(scaled)
        } else {
            None
        }
    })
}

fn scale_scoop(value: Option<u32>, frac: f64) -> Option<u32> {
    let scoops = value?;
    let scaled = scoops as f64 * frac;
    if scaled <= AMOUNT_EPS {
        return None;
    }
    let rounded = scaled.round();
    if (scaled - rounded).abs() < 1e-9 {
        Some(rounded as u32)
    } else {
        None
    }
}

fn scale_entry(entry: &CompositionEntry, frac: f64) -> Option<CompositionEntry> {
    let scaled = CompositionEntry {
        substance_id: entry.substance_id.clone(),
        phase: entry.phase.clone(),
        amount_ml: scale_opt_f64(entry.amount_ml, frac),
        amount_scoop: scale_scoop(entry.amount_scoop, frac),
        amount_g: scale_opt_f64(entry.amount_g, frac),
        amount_mol: scale_opt_f64(entry.amount_mol, frac),
    };
    let has_amount = scaled.amount_ml.is_some()
        || scaled.amount_g.is_some()
        || scaled.amount_mol.is_some()
        || scaled.amount_scoop.is_some_and(|s| s > 0);
    has_amount.then_some(scaled)
}

fn take_composition_fraction(source: &mut SceneItem, frac: f64) -> Vec<CompositionEntry> {
    let frac = frac.clamp(0.0, 1.0);
    if frac <= AMOUNT_EPS {
        return Vec::new();
    }
    let original = std::mem::take(&mut source.properties.composition);
    let mut taken = Vec::new();
    let mut remaining = Vec::new();
    for entry in original {
        if let Some(moved) = scale_entry(&entry, frac) {
            taken.push(moved);
        }
        if frac < 1.0 - AMOUNT_EPS {
            if let Some(rest) = scale_entry(&entry, 1.0 - frac) {
                remaining.push(rest);
            }
        }
    }
    source.properties.composition = remaining;
    crate::solubility::sync_fill_ml(source);
    taken
}

fn take_all_solids(source: &mut SceneItem) -> Vec<CompositionEntry> {
    let mut taken = Vec::new();
    source.properties.composition.retain(|entry| {
        if entry.phase == "solid" {
            taken.push(entry.clone());
            false
        } else {
            true
        }
    });
    crate::solubility::sync_fill_ml(source);
    taken
}

fn mix_transfer_into(
    target: &mut SceneItem,
    transferred: &[CompositionEntry],
    source_t: Option<f64>,
) {
    let add_ml = transferred
        .iter()
        .filter(|c| c.substance_id == "water" && c.phase == "liquid")
        .map(|c| c.amount_ml.unwrap_or(0.0))
        .sum::<f64>();
    if let Some(aliquot_t) = source_t {
        let v0 = crate::solubility::liquid_water_ml(target);
        let t0 = target
            .properties
            .temperature_c
            .unwrap_or(AMBIENT_TEMPERATURE_C);
        if add_ml > AMOUNT_EPS {
            if v0 <= AMOUNT_EPS {
                target.properties.temperature_c = Some(aliquot_t);
            } else {
                target.properties.temperature_c =
                    Some((v0 * t0 + add_ml * aliquot_t) / (v0 + add_ml));
            }
        }
    }
    for entry in transferred {
        if entry.substance_id == "water" && entry.phase == "liquid" {
            add_or_increase_water(target, entry.amount_ml.unwrap_or(0.0));
        } else if entry.phase == "aqueous" {
            add_or_increase_mol(
                target,
                &entry.substance_id,
                "aqueous",
                entry.amount_mol.unwrap_or(0.0),
            );
        } else if entry.phase == "solid" {
            add_or_increase_solid(
                target,
                &entry.substance_id,
                entry.amount_scoop,
                entry.amount_g,
            );
        }
    }
    crate::solubility::sync_fill_ml(target);
}

fn mix_aliquot_into(target: &mut SceneItem, aliquot: &[CompositionEntry], aliquot_t: f64) {
    let add_ml = aliquot
        .iter()
        .filter(|c| c.substance_id == "water" && c.phase == "liquid")
        .map(|c| c.amount_ml.unwrap_or(0.0))
        .sum::<f64>();
    let v0 = crate::solubility::liquid_water_ml(target);
    let t0 = target
        .properties
        .temperature_c
        .unwrap_or(AMBIENT_TEMPERATURE_C);
    if v0 <= AMOUNT_EPS {
        target.properties.temperature_c = Some(aliquot_t);
    } else {
        target.properties.temperature_c = Some((v0 * t0 + add_ml * aliquot_t) / (v0 + add_ml));
    }
    for entry in aliquot {
        if entry.substance_id == "water" && entry.phase == "liquid" {
            add_or_increase_water(target, entry.amount_ml.unwrap_or(0.0));
        } else if entry.phase == "aqueous" {
            add_or_increase_mol(
                target,
                &entry.substance_id,
                "aqueous",
                entry.amount_mol.unwrap_or(0.0),
            );
        }
    }
    crate::solubility::sync_fill_ml(target);
}

fn add_or_increase_water(item: &mut SceneItem, ml: f64) {
    if ml <= AMOUNT_EPS {
        return;
    }
    if let Some(existing) = item
        .properties
        .composition
        .iter_mut()
        .find(|c| c.substance_id == "water" && c.phase == "liquid")
    {
        existing.amount_ml = Some(existing.amount_ml.unwrap_or(0.0) + ml);
        return;
    }
    item.properties.composition.push(CompositionEntry {
        substance_id: "water".into(),
        phase: "liquid".into(),
        amount_ml: Some(ml),
        amount_scoop: None,
        amount_g: None,
        amount_mol: None,
    });
}

fn take_liquid_aliquot(
    source: &mut SceneItem,
    volume_ml: f64,
) -> Result<Vec<CompositionEntry>, SceneError> {
    let liquid = crate::solubility::liquid_water_ml(source);
    if liquid + AMOUNT_EPS < volume_ml {
        return Err(SceneError::InvalidAction);
    }
    let frac = volume_ml / liquid;
    let mut aliquot = Vec::new();
    let mut remaining = Vec::new();
    for entry in source.properties.composition.drain(..) {
        if entry.phase == "solid" {
            remaining.push(entry);
            continue;
        }
        if entry.substance_id == "water" && entry.phase == "liquid" {
            aliquot.push(CompositionEntry {
                substance_id: "water".into(),
                phase: "liquid".into(),
                amount_ml: Some(volume_ml),
                amount_scoop: None,
                amount_g: None,
                amount_mol: None,
            });
            let rest = entry.amount_ml.unwrap_or(0.0) - volume_ml;
            if rest > AMOUNT_EPS {
                remaining.push(CompositionEntry {
                    amount_ml: Some(rest),
                    ..entry
                });
            }
            continue;
        }
        if entry.phase == "aqueous" {
            let mol = entry.amount_mol.unwrap_or(0.0);
            let taken = mol * frac;
            let rest = mol - taken;
            if taken > AMOUNT_EPS {
                aliquot.push(CompositionEntry {
                    amount_mol: Some(taken),
                    ..entry.clone()
                });
            }
            if rest > AMOUNT_EPS {
                remaining.push(CompositionEntry {
                    amount_mol: Some(rest),
                    ..entry
                });
            }
            continue;
        }
        remaining.push(entry);
    }
    source.properties.composition = remaining;
    crate::solubility::sync_fill_ml(source);
    Ok(aliquot)
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

    /// Tests that pour, pipette, or dissolve into the main beaker start from a filled vessel.
    const FILLED_MAIN_BEAKER_ML: f64 = 200.0;

    fn fill_main_beaker(scene: &mut Scene, amount_ml: f64) {
        let water = scene
            .items
            .iter_mut()
            .find(|item| item.id == "beaker-water")
            .expect("beaker-water");
        water.properties.fill_ml = Some(amount_ml);
        water.properties.composition = vec![CompositionEntry {
            substance_id: "water".into(),
            phase: "liquid".into(),
            amount_ml: Some(amount_ml),
            amount_scoop: None,
            amount_g: None,
            amount_mol: None,
        }];
    }

    fn bench_with_water(lab_id: &str) -> Scene {
        let mut scene = initial_bench_scene(lab_id);
        fill_main_beaker(&mut scene, FILLED_MAIN_BEAKER_ML);
        scene
    }

    #[test]
    fn initial_bench_scene_has_twelve_items_with_water_and_evaporation_bench() {
        let scene = initial_bench_scene("lab-test");
        assert_eq!(scene.lab_id, "lab-test");
        assert_eq!(scene.temperature_c, 20.0);
        assert_eq!(scene.version, 0);
        assert!(scene.last_events.is_empty());
        assert_eq!(scene.last_applied_unix_ms, None);
        assert_eq!(scene.items.len(), 12);

        let ids: Vec<_> = scene.items.iter().map(|i| i.id.as_str()).collect();
        assert!(ids.contains(&"spoon-1"));
        assert!(ids.contains(&"beaker-h2o"));
        assert!(ids.contains(&"beaker-nacl"));
        assert!(ids.contains(&"beaker-cacl2"));
        assert!(ids.contains(&"beaker-sand"));
        assert!(ids.contains(&"beaker-water"));
        assert!(ids.contains(&"pipette-1"));
        assert!(ids.contains(&"dish-1"));
        assert!(ids.contains(&"burner-1"));
        assert!(ids.contains(&"tongs-1"));
        assert!(ids.contains(&"beaker-filtrate"));
        assert!(ids.contains(&"filter-paper-1"));

        let distilled = item(&scene, "beaker-h2o");
        assert_eq!(distilled.kind, "beaker");
        assert_eq!(distilled.label, "Distilled water");
        assert_eq!(distilled.location, "bench");
        assert_eq!(distilled.properties.volume_ml, Some(100.0));
        assert_eq!(distilled.properties.fill_ml, Some(100.0));
        assert_eq!(distilled.properties.transparent, Some(true));
        assert_eq!(distilled.properties.colourless, Some(true));
        assert_eq!(distilled.properties.temperature_c, Some(20.0));
        assert_eq!(distilled.properties.composition.len(), 1);
        assert_eq!(distilled.properties.composition[0].substance_id, "water");
        assert_eq!(distilled.properties.composition[0].phase, "liquid");
        assert_eq!(distilled.properties.composition[0].amount_ml, Some(100.0));

        let water = item(&scene, "beaker-water");
        assert_eq!(water.kind, "beaker");
        assert_eq!(water.label, "Beaker");
        assert_eq!(water.location, "bench");
        assert_eq!(water.properties.volume_ml, Some(WATER_CAPACITY_ML));
        assert_eq!(water.properties.fill_ml, Some(0.0));
        assert_eq!(water.properties.transparent, Some(true));
        assert_eq!(water.properties.colourless, Some(true));
        assert_eq!(water.properties.temperature_c, Some(20.0));
        assert!(water.properties.composition.is_empty());

        let nacl = item(&scene, "beaker-nacl");
        assert!(nacl
            .properties
            .composition
            .iter()
            .any(|c| c.substance_id == "nacl" && c.phase == "solid"));

        let cacl2 = item(&scene, "beaker-cacl2");
        assert_eq!(cacl2.label, "Calcium chloride");
        assert!(cacl2
            .properties
            .composition
            .iter()
            .any(|c| c.substance_id == "cacl2" && c.phase == "solid" && c.amount_g == Some(2.0)));

        let sand = item(&scene, "beaker-sand");
        assert!(sand
            .properties
            .composition
            .iter()
            .any(|c| c.substance_id == "sand" && c.phase == "solid"));

        let spoon = item(&scene, "spoon-1");
        assert_eq!(spoon.kind, "spoon");
        assert!(spoon.properties.holding.is_empty());

        let pipette = item(&scene, "pipette-1");
        assert_eq!(pipette.kind, "pipette");
        assert_eq!(pipette.location, "bench");
        assert!(pipette.properties.holding.is_empty());
        assert_eq!(pipette.properties.source_item_id, None);

        let dish = item(&scene, "dish-1");
        assert_eq!(dish.kind, "evaporation_dish");
        assert_eq!(dish.properties.volume_ml, Some(DISH_CAPACITY_ML));
        assert_eq!(dish.properties.fill_ml, Some(0.0));
        assert_eq!(dish.properties.temperature_c, Some(AMBIENT_TEMPERATURE_C));
        assert!(dish.properties.composition.is_empty());

        let burner = item(&scene, "burner-1");
        assert_eq!(burner.kind, "burner");
        assert_eq!(burner.properties.on, Some(false));

        let tongs = item(&scene, "tongs-1");
        assert_eq!(tongs.kind, "tongs");
        assert_eq!(tongs.label, "Tongs");
        assert_eq!(tongs.location, "bench");
        assert_eq!(tongs.properties.source_item_id, None);
        assert!(tongs.properties.holding.is_empty());
    }

    #[test]
    fn ensure_default_bench_items_restores_beaker_h2o_without_resetting_vessels() {
        let mut scene = initial_bench_scene("lab-test");
        {
            let water = scene
                .items
                .iter_mut()
                .find(|item| item.id == "beaker-water")
                .unwrap();
            water.label = "Water".into();
            water.properties.fill_ml = Some(150.0);
            water.properties.composition = vec![CompositionEntry {
                substance_id: "water".into(),
                phase: "liquid".into(),
                amount_ml: Some(150.0),
                amount_scoop: None,
                amount_g: None,
                amount_mol: None,
            }];
        }
        scene.items.retain(|item| item.id != "beaker-h2o");

        ensure_default_bench_items(&mut scene);

        let distilled = item(&scene, "beaker-h2o");
        assert_eq!(distilled.kind, "beaker");
        assert_eq!(distilled.location, "bench");
        assert_eq!(distilled.properties.volume_ml, Some(100.0));
        assert_eq!(water_ml(distilled), 100.0);
        let water = item(&scene, "beaker-water");
        assert_eq!(water.label, "Water");
        assert_eq!(water.properties.fill_ml, Some(150.0));
        assert!((water_ml(water) - 150.0).abs() < 1e-9);
        assert_eq!(
            scene
                .items
                .iter()
                .filter(|item| item.id == "beaker-h2o")
                .count(),
            1
        );
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
        assert_eq!(
            spoon.properties.holding[0].amount_g,
            Some(SPOON_SCOOP_MASS_G)
        );
        assert_eq!(SPOON_SCOOP_MASS_G, 0.2);

        let nacl = item(&scene, "beaker-nacl");
        let stock = nacl
            .properties
            .composition
            .iter()
            .find(|c| c.substance_id == "nacl" && c.phase == "solid")
            .unwrap();
        assert_eq!(stock.amount_scoop, Some(9));
        assert_eq!(stock.amount_g, Some(9.0 * SPOON_SCOOP_MASS_G));

        assert!(scene.last_events.iter().any(|e| e.kind == "scooped"));
        // Scoop does not pour — the empty main beaker stays empty.
        let water = item(&scene, "beaker-water");
        assert!(water.properties.composition.is_empty());
    }

    #[test]
    fn use_tool_fails_when_stock_is_empty() {
        let mut scene = initial_bench_scene("lab-test");
        let nacl = scene
            .items
            .iter_mut()
            .find(|i| i.id == "beaker-nacl")
            .unwrap();
        let stock = nacl
            .properties
            .composition
            .iter_mut()
            .find(|c| c.substance_id == "nacl")
            .unwrap();
        stock.amount_scoop = Some(0);
        stock.amount_g = Some(0.0);

        let err = apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "beaker-nacl".into(),
            },
        )
        .unwrap_err();
        assert_eq!(err, SceneError::InvalidAction);
        let spoon = item(&scene, "spoon-1");
        assert!(spoon.properties.holding.is_empty());
    }

    #[test]
    fn scooping_stock_empty_leaves_zero_grams() {
        let mut scene = bench_with_water("lab-test");
        for _ in 0..10 {
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

        let nacl = item(&scene, "beaker-nacl");
        let stock = nacl
            .properties
            .composition
            .iter()
            .find(|c| c.substance_id == "nacl" && c.phase == "solid")
            .unwrap();
        assert_eq!(stock.amount_scoop, Some(0));
        assert_eq!(
            stock.amount_g,
            Some(0.0),
            "leftover grams {:?} would still draw a floor sliver",
            stock.amount_g
        );
    }

    #[test]
    fn use_tool_returns_held_nacl_to_matching_stock() {
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
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "beaker-nacl".into(),
            },
        )
        .unwrap();

        let spoon = item(&scene, "spoon-1");
        assert!(spoon.properties.holding.is_empty());
        let nacl = item(&scene, "beaker-nacl");
        let stock = nacl
            .properties
            .composition
            .iter()
            .find(|c| c.substance_id == "nacl" && c.phase == "solid")
            .unwrap();
        assert_eq!(stock.amount_scoop, Some(10));
        assert_eq!(stock.amount_g, Some(10.0 * SPOON_SCOOP_MASS_G));
        assert!(scene.last_events.iter().any(|e| e.kind == "returned"));
    }

    #[test]
    fn use_tool_rejects_returning_nacl_into_sand_stock() {
        let mut scene = initial_bench_scene("lab-test");
        apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "beaker-nacl".into(),
            },
        )
        .unwrap();
        let err = apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "beaker-sand".into(),
            },
        )
        .unwrap_err();
        assert_eq!(err, SceneError::InvalidAction);
        let spoon = item(&scene, "spoon-1");
        assert_eq!(spoon.properties.holding[0].substance_id, "nacl");
        let sand = item(&scene, "beaker-sand");
        let stock = sand
            .properties
            .composition
            .iter()
            .find(|c| c.substance_id == "sand")
            .unwrap();
        assert_eq!(stock.amount_scoop, Some(10));
        let nacl = item(&scene, "beaker-nacl");
        let nacl_stock = nacl
            .properties
            .composition
            .iter()
            .find(|c| c.substance_id == "nacl")
            .unwrap();
        assert_eq!(nacl_stock.amount_scoop, Some(9));
    }

    #[test]
    fn pour_nacl_into_water_dissolves_without_leftover_grains() {
        let mut scene = bench_with_water("lab-test");
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
    fn pour_nacl_into_water_cools_solution_endothermically() {
        let mut scene = bench_with_water("lab-test");
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

        let water = item(&scene, "beaker-water");
        let moles = SPOON_SCOOP_MASS_G / NACL_MOLAR_MASS_G_PER_MOL;
        let expected_delta_t =
            -(moles * NACL_DELTA_H_SOLUTION_J_PER_MOL) / (200.0 * WATER_SPECIFIC_HEAT_J_PER_G_K);
        let expected_t = 20.0 + expected_delta_t;
        assert!(
            expected_delta_t < 0.0,
            "NaCl dissolution must be endothermic (negative ΔT)"
        );
        let actual = water
            .properties
            .temperature_c
            .expect("water beaker should keep a temperature");
        assert!(
            (actual - expected_t).abs() < 1e-9,
            "expected {expected_t}, got {actual}"
        );
        // Ambient bench temperature is unchanged; only the solution cools.
        assert_eq!(scene.temperature_c, 20.0);
    }

    #[test]
    fn pour_cacl2_into_water_dissolves_with_ions_and_heats_exothermically() {
        let mut scene = bench_with_water("lab-test");
        apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "beaker-cacl2".into(),
            },
        )
        .unwrap();

        let stock = item(&scene, "beaker-cacl2")
            .properties
            .composition
            .iter()
            .find(|c| c.substance_id == "cacl2" && c.phase == "solid")
            .unwrap();
        assert_eq!(stock.amount_scoop, Some(9));
        assert_eq!(stock.amount_g, Some(9.0 * SPOON_SCOOP_MASS_G));

        apply_action(
            &mut scene,
            Action::Pour {
                source_item_id: "spoon-1".into(),
                target_item_id: "beaker-water".into(),
            },
        )
        .unwrap();

        let water = item(&scene, "beaker-water");
        let moles = SPOON_SCOOP_MASS_G / CACL2_MOLAR_MASS_G_PER_MOL;
        let ca = water
            .properties
            .composition
            .iter()
            .find(|c| c.substance_id == "ca2+" && c.phase == "aqueous")
            .unwrap();
        let cl = water
            .properties
            .composition
            .iter()
            .find(|c| c.substance_id == "cl-" && c.phase == "aqueous")
            .unwrap();
        assert!((ca.amount_mol.unwrap() - moles).abs() < 1e-12);
        assert!((cl.amount_mol.unwrap() - 2.0 * moles).abs() < 1e-12);
        assert!(!water
            .properties
            .composition
            .iter()
            .any(|c| c.substance_id == "cacl2"));

        let expected_delta_t =
            -(moles * CACL2_DELTA_H_SOLUTION_J_PER_MOL) / (200.0 * WATER_SPECIFIC_HEAT_J_PER_G_K);
        assert!(
            expected_delta_t > 0.0,
            "CaCl2 dissolution must be exothermic (positive ΔT)"
        );
        let actual = water
            .properties
            .temperature_c
            .expect("water beaker should keep a temperature");
        assert!((actual - (20.0 + expected_delta_t)).abs() < 1e-9);
        assert_eq!(scene.temperature_c, 20.0);
        assert!(scene.last_events.iter().any(|e| {
            e.kind == "dissolved"
                && e.message == "Calcium chloride (CaCl2) dissolves in water at bench temperature."
        }));
    }

    #[test]
    fn use_tool_returns_held_cacl2_to_matching_stock() {
        let mut scene = initial_bench_scene("lab-test");
        apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "beaker-cacl2".into(),
            },
        )
        .unwrap();
        apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "beaker-cacl2".into(),
            },
        )
        .unwrap();

        let spoon = item(&scene, "spoon-1");
        assert!(spoon.properties.holding.is_empty());
        let stock = item(&scene, "beaker-cacl2")
            .properties
            .composition
            .iter()
            .find(|c| c.substance_id == "cacl2" && c.phase == "solid")
            .unwrap();
        assert_eq!(stock.amount_scoop, Some(10));
        assert_eq!(stock.amount_g, Some(10.0 * SPOON_SCOOP_MASS_G));
        assert!(scene.last_events.iter().any(|e| e.kind == "returned"));
    }

    #[test]
    fn pour_sand_into_water_does_not_change_temperature() {
        let mut scene = bench_with_water("lab-test");
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

        let water = item(&scene, "beaker-water");
        assert_eq!(water.properties.temperature_c, Some(20.0));
    }

    #[test]
    fn second_nacl_pour_updates_existing_ion_moles_without_duplicate_lines() {
        let mut scene = bench_with_water("lab-test");
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
        let mut scene = bench_with_water("lab-test");
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
        let mut scene = bench_with_water("lab-test");
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
    fn reset_restores_empty_beaker_and_clears_holding_after_pour() {
        let mut scene = bench_with_water("lab-test");
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
        assert_eq!(water.label, "Beaker");
        assert_eq!(water.properties.fill_ml, Some(0.0));
        assert!(water.properties.composition.is_empty());
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
    fn pour_into_dry_beaker_returns_invalid_action() {
        let mut scene = initial_bench_scene("lab-test");
        apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "beaker-nacl".into(),
            },
        )
        .unwrap();

        let err = apply_action(
            &mut scene,
            Action::Pour {
                source_item_id: "spoon-1".into(),
                target_item_id: "beaker-sand".into(),
            },
        )
        .unwrap_err();
        assert_eq!(err, SceneError::InvalidAction);
        let spoon = item(&scene, "spoon-1");
        assert_eq!(spoon.properties.holding[0].substance_id, "nacl");
    }

    #[test]
    fn pour_sand_at_non_bench_temperature_leaves_undissolved_solid() {
        let mut scene = bench_with_water("lab-test");
        apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "beaker-sand".into(),
            },
        )
        .unwrap();

        let water = scene
            .items
            .iter_mut()
            .find(|i| i.id == "beaker-water")
            .unwrap();
        water.properties.temperature_c = Some(21.0);

        apply_action(
            &mut scene,
            Action::Pour {
                source_item_id: "spoon-1".into(),
                target_item_id: "beaker-water".into(),
            },
        )
        .unwrap();

        let water = item(&scene, "beaker-water");
        assert_eq!(water.properties.temperature_c, Some(21.0));
        assert!(water.properties.composition.iter().any(|c| {
            c.substance_id == "sand"
                && c.phase == "solid"
                && c.amount_scoop == Some(1)
                && (c.amount_g.unwrap() - SPOON_SCOOP_MASS_G).abs() < 1e-12
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
        assert!(item(&scene, "spoon-1").properties.holding.is_empty());
    }

    #[test]
    fn pour_sand_succeeds_after_cacl2_exothermic_heating() {
        let mut scene = bench_with_water("lab-test");

        // One scoop only raises T by ~0.175 °C (still rounds to 20). Pour until the
        // dissolve lookup sees a non-bench integer °C — the real warm-water bug path.
        let mut after_heat = 20.0;
        for _ in 0..4 {
            apply_action(
                &mut scene,
                Action::UseTool {
                    tool_item_id: "spoon-1".into(),
                    target_item_id: "beaker-cacl2".into(),
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
            after_heat = item(&scene, "beaker-water")
                .properties
                .temperature_c
                .expect("water beaker should keep a temperature");
        }
        assert!(
            after_heat.round() as i32 != 20,
            "CaCl2 heating must leave a non-bench lookup T, got {after_heat}"
        );

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

        let water = item(&scene, "beaker-water");
        assert_eq!(water.properties.temperature_c, Some(after_heat));
        assert!(water.properties.composition.iter().any(|c| {
            c.substance_id == "sand"
                && c.phase == "solid"
                && c.amount_scoop == Some(1)
                && (c.amount_g.unwrap() - SPOON_SCOOP_MASS_G).abs() < 1e-12
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
    fn pour_second_cacl2_scoop_succeeds_after_exothermic_heating() {
        let mut scene = bench_with_water("lab-test");

        apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "beaker-cacl2".into(),
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

        let after_first = item(&scene, "beaker-water")
            .properties
            .temperature_c
            .expect("water beaker should keep a temperature");
        assert!(
            after_first > 20.0,
            "first CaCl2 scoop must raise beaker T above 20 °C, got {after_first}"
        );

        apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "beaker-cacl2".into(),
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

        let water = item(&scene, "beaker-water");
        let moles = 2.0 * (SPOON_SCOOP_MASS_G / CACL2_MOLAR_MASS_G_PER_MOL);
        let ca = water
            .properties
            .composition
            .iter()
            .find(|c| c.substance_id == "ca2+" && c.phase == "aqueous")
            .unwrap();
        assert!((ca.amount_mol.unwrap() - moles).abs() < 1e-12);

        let after_second = water
            .properties
            .temperature_c
            .expect("water beaker should keep a temperature");
        assert!(
            after_second > after_first,
            "second CaCl2 scoop should heat further: {after_first} -> {after_second}"
        );
        assert!(scene.last_events.iter().any(|e| {
            e.kind == "dissolved"
                && e.message == "Calcium chloride (CaCl2) dissolves in water at bench temperature."
        }));
        assert!(item(&scene, "spoon-1").properties.holding.is_empty());
    }

    #[test]
    fn pour_nacl_succeeds_after_mild_heating_above_bench() {
        let mut scene = bench_with_water("lab-test");
        let water = scene
            .items
            .iter_mut()
            .find(|i| i.id == "beaker-water")
            .unwrap();
        water.properties.temperature_c = Some(21.5);

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

        let water = item(&scene, "beaker-water");
        let moles = SPOON_SCOOP_MASS_G / NACL_MOLAR_MASS_G_PER_MOL;
        let expected_t = 21.5
            - (moles * NACL_DELTA_H_SOLUTION_J_PER_MOL) / (200.0 * WATER_SPECIFIC_HEAT_J_PER_G_K);
        let actual = water
            .properties
            .temperature_c
            .expect("water beaker should keep a temperature");
        assert!((actual - expected_t).abs() < 1e-9);
        assert!(scene.last_events.iter().any(|e| {
            e.kind == "dissolved"
                && e.message == "Sodium chloride (NaCl) dissolves in water at bench temperature."
        }));
    }

    #[test]
    fn put_away_returns_held_nacl_to_stock_and_empties_spoon() {
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
            Action::PutAway {
                tool_item_id: "spoon-1".into(),
            },
        )
        .unwrap();

        let spoon = item(&scene, "spoon-1");
        assert!(spoon.properties.holding.is_empty());
        assert_eq!(spoon.location, "bench");
        let stock = item(&scene, "beaker-nacl")
            .properties
            .composition
            .iter()
            .find(|c| c.substance_id == "nacl" && c.phase == "solid")
            .unwrap();
        assert_eq!(stock.amount_scoop, Some(10));
        assert_eq!(stock.amount_g, Some(10.0 * SPOON_SCOOP_MASS_G));
        assert!(scene.last_events.iter().any(|e| e.kind == "returned"));
    }

    #[test]
    fn put_away_returns_held_sand_to_stock_and_empties_spoon() {
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
            Action::PutAway {
                tool_item_id: "spoon-1".into(),
            },
        )
        .unwrap();

        let spoon = item(&scene, "spoon-1");
        assert!(spoon.properties.holding.is_empty());
        assert_eq!(spoon.location, "bench");
        let stock = item(&scene, "beaker-sand")
            .properties
            .composition
            .iter()
            .find(|c| c.substance_id == "sand" && c.phase == "solid")
            .unwrap();
        assert_eq!(stock.amount_scoop, Some(10));
        assert_eq!(stock.amount_g, Some(10.0 * SPOON_SCOOP_MASS_G));
        assert!(scene.last_events.iter().any(|e| e.kind == "returned"));
    }

    #[test]
    fn put_away_returns_held_cacl2_to_stock_and_empties_spoon() {
        let mut scene = initial_bench_scene("lab-test");
        apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "beaker-cacl2".into(),
            },
        )
        .unwrap();
        apply_action(
            &mut scene,
            Action::PutAway {
                tool_item_id: "spoon-1".into(),
            },
        )
        .unwrap();

        let spoon = item(&scene, "spoon-1");
        assert!(spoon.properties.holding.is_empty());
        assert_eq!(spoon.location, "bench");
        let stock = item(&scene, "beaker-cacl2")
            .properties
            .composition
            .iter()
            .find(|c| c.substance_id == "cacl2" && c.phase == "solid")
            .unwrap();
        assert_eq!(stock.amount_scoop, Some(10));
        assert_eq!(stock.amount_g, Some(10.0 * SPOON_SCOOP_MASS_G));
        assert!(scene.last_events.iter().any(|e| e.kind == "returned"));
    }

    #[test]
    fn put_away_empty_spoon_is_noop() {
        let mut scene = initial_bench_scene("lab-test");
        let before = scene.clone();
        apply_action(
            &mut scene,
            Action::PutAway {
                tool_item_id: "spoon-1".into(),
            },
        )
        .unwrap();

        assert_eq!(item(&scene, "spoon-1").properties.holding, Vec::new());
        assert_eq!(item(&scene, "spoon-1").location, "bench");
        assert_eq!(
            item(&scene, "beaker-nacl").properties.composition,
            item(&before, "beaker-nacl").properties.composition
        );
        assert_eq!(
            item(&scene, "beaker-sand").properties.composition,
            item(&before, "beaker-sand").properties.composition
        );
        assert_eq!(
            item(&scene, "beaker-cacl2").properties.composition,
            item(&before, "beaker-cacl2").properties.composition
        );
        assert!(scene.last_events.is_empty());
    }

    fn aqueous_mol(item: &SceneItem, substance_id: &str) -> f64 {
        item.properties
            .composition
            .iter()
            .find(|c| c.substance_id == substance_id && c.phase == "aqueous")
            .and_then(|c| c.amount_mol)
            .unwrap_or(0.0)
    }

    fn water_ml(item: &SceneItem) -> f64 {
        crate::solubility::liquid_water_ml(item)
    }

    fn fill_pipette_from(scene: &mut Scene, target_id: &str) {
        apply_action(
            scene,
            Action::UseTool {
                tool_item_id: "pipette-1".into(),
                target_item_id: target_id.into(),
            },
        )
        .unwrap();
    }

    #[test]
    fn pipette_extracts_one_ml_from_water_scaling_aqueous_and_leaving_sand() {
        let mut scene = bench_with_water("lab-test");
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

        let water_before = item(&scene, "beaker-water");
        let na_before = aqueous_mol(water_before, "na+");
        let cl_before = aqueous_mol(water_before, "cl-");
        let sand_before = water_before
            .properties
            .composition
            .iter()
            .find(|c| c.substance_id == "sand" && c.phase == "solid")
            .unwrap()
            .amount_g
            .unwrap();
        let liquid_before = water_ml(water_before);
        assert!((liquid_before - 200.0).abs() < 1e-12);

        fill_pipette_from(&mut scene, "beaker-water");

        let water = item(&scene, "beaker-water");
        assert!((water_ml(water) - 199.0).abs() < 1e-9);
        let frac = 199.0 / 200.0;
        assert!((aqueous_mol(water, "na+") - na_before * frac).abs() < 1e-12);
        assert!((aqueous_mol(water, "cl-") - cl_before * frac).abs() < 1e-12);
        let sand_after = water
            .properties
            .composition
            .iter()
            .find(|c| c.substance_id == "sand" && c.phase == "solid")
            .unwrap()
            .amount_g
            .unwrap();
        assert!((sand_after - sand_before).abs() < 1e-12);

        let pipette = item(&scene, "pipette-1");
        assert_eq!(pipette.location, "hand");
        assert_eq!(
            pipette.properties.source_item_id.as_deref(),
            Some("beaker-water")
        );
        assert!((pipette_holding_liquid_ml(pipette) - PIPETTE_VOLUME_ML).abs() < 1e-12);
        assert!((aqueous_mol_holding(pipette, "na+") - na_before / 200.0).abs() < 1e-12);
        assert!(pipette
            .properties
            .holding
            .iter()
            .all(|c| c.phase != "solid"));
    }

    fn aqueous_mol_holding(item: &SceneItem, substance_id: &str) -> f64 {
        item.properties
            .holding
            .iter()
            .find(|c| c.substance_id == substance_id && c.phase == "aqueous")
            .and_then(|c| c.amount_mol)
            .unwrap_or(0.0)
    }

    #[test]
    fn pipette_fill_from_dish_and_pour_back_to_water_is_consistent() {
        let mut scene = bench_with_water("lab-test");
        fill_pipette_from(&mut scene, "beaker-water");
        apply_action(
            &mut scene,
            Action::Pour {
                source_item_id: "pipette-1".into(),
                target_item_id: "dish-1".into(),
            },
        )
        .unwrap();

        let dish = item(&scene, "dish-1");
        assert!((water_ml(dish) - 1.0).abs() < 1e-9);
        assert_eq!(dish.properties.temperature_c, Some(20.0));

        fill_pipette_from(&mut scene, "dish-1");
        apply_action(
            &mut scene,
            Action::Pour {
                source_item_id: "pipette-1".into(),
                target_item_id: "beaker-water".into(),
            },
        )
        .unwrap();

        let water = item(&scene, "beaker-water");
        assert!((water_ml(water) - 200.0).abs() < 1e-9);
        assert_eq!(water.properties.temperature_c, Some(20.0));
        let dish = item(&scene, "dish-1");
        assert!(water_ml(dish) < 1e-9);
        assert!(item(&scene, "pipette-1").properties.holding.is_empty());
    }

    #[test]
    fn dish_rejects_over_capacity_and_fill_requires_one_ml() {
        let mut scene = bench_with_water("lab-test");
        let dish = scene.items.iter_mut().find(|i| i.id == "dish-1").unwrap();
        dish.properties.composition.push(CompositionEntry {
            substance_id: "water".into(),
            phase: "liquid".into(),
            amount_ml: Some(24.5),
            amount_scoop: None,
            amount_g: None,
            amount_mol: None,
        });
        dish.properties.fill_ml = Some(24.5);

        fill_pipette_from(&mut scene, "beaker-water");
        let err = apply_action(
            &mut scene,
            Action::Pour {
                source_item_id: "pipette-1".into(),
                target_item_id: "dish-1".into(),
            },
        )
        .unwrap_err();
        assert_eq!(err, SceneError::InvalidAction);
        assert!((pipette_holding_liquid_ml(item(&scene, "pipette-1")) - 1.0).abs() < 1e-12);

        apply_action(
            &mut scene,
            Action::PutAway {
                tool_item_id: "pipette-1".into(),
            },
        )
        .unwrap();

        let dish = scene.items.iter_mut().find(|i| i.id == "dish-1").unwrap();
        dish.properties.composition.clear();
        dish.properties.composition.push(CompositionEntry {
            substance_id: "water".into(),
            phase: "liquid".into(),
            amount_ml: Some(0.5),
            amount_scoop: None,
            amount_g: None,
            amount_mol: None,
        });
        let err = apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "pipette-1".into(),
                target_item_id: "dish-1".into(),
            },
        )
        .unwrap_err();
        assert_eq!(err, SceneError::InvalidAction);
    }

    #[test]
    fn burner_heats_to_100_then_evaporates_precipitates_and_turns_off() {
        let mut scene = initial_bench_scene("lab-test");
        let dish = scene.items.iter_mut().find(|i| i.id == "dish-1").unwrap();
        dish.properties.temperature_c = Some(20.0);
        dish.properties.composition = vec![
            CompositionEntry {
                substance_id: "water".into(),
                phase: "liquid".into(),
                amount_ml: Some(2.0),
                amount_scoop: None,
                amount_g: None,
                amount_mol: None,
            },
            CompositionEntry {
                substance_id: "na+".into(),
                phase: "aqueous".into(),
                amount_ml: None,
                amount_scoop: None,
                amount_g: None,
                amount_mol: Some(0.02),
            },
            CompositionEntry {
                substance_id: "cl-".into(),
                phase: "aqueous".into(),
                amount_ml: None,
                amount_scoop: None,
                amount_g: None,
                amount_mol: Some(0.02),
            },
        ];
        crate::solubility::enforce_saturation(dish);

        apply_action(
            &mut scene,
            Action::ToggleBurner {
                burner_item_id: "burner-1".into(),
            },
        )
        .unwrap();
        assert_eq!(item(&scene, "burner-1").properties.on, Some(true));

        apply_elapsed(&mut scene, 8.0);
        let dish = item(&scene, "dish-1");
        assert!((dish.properties.temperature_c.unwrap() - 100.0).abs() < 1e-9);
        assert!(
            (water_ml(dish) - 2.0).abs() < 1e-9,
            "no evaporation below 100"
        );

        apply_elapsed(&mut scene, 2.0);
        let dish = item(&scene, "dish-1");
        assert!((water_ml(dish) - 1.0).abs() < 1e-9);
        assert_eq!(dish.properties.temperature_c, Some(100.0));
        assert!(
            dish.properties
                .composition
                .iter()
                .any(|c| c.substance_id == "nacl" && c.phase == "solid"),
            "1 ml cannot hold 0.02 mol NaCl; solid must appear"
        );
        let max_aq =
            crate::solubility::solubility_mol_per_l(crate::solubility::Salt::Nacl, 100.0) * 0.001;
        assert!((aqueous_mol(dish, "na+") - max_aq).abs() < 1e-9);

        apply_elapsed(&mut scene, 2.0);
        let dish = item(&scene, "dish-1");
        assert!(water_ml(dish) < 1e-9);
        assert_eq!(item(&scene, "burner-1").properties.on, Some(false));
        assert!(aqueous_mol(dish, "na+") < 1e-12);
        assert!(dish
            .properties
            .composition
            .iter()
            .any(|c| c.substance_id == "nacl" && c.phase == "solid"));
    }

    #[test]
    fn heating_below_boil_redissolves_nacl_as_solubility_rises() {
        let mut scene = initial_bench_scene("lab-test");
        let dish = scene.items.iter_mut().find(|i| i.id == "dish-1").unwrap();
        dish.properties.temperature_c = Some(20.0);
        dish.properties.composition = vec![
            CompositionEntry {
                substance_id: "water".into(),
                phase: "liquid".into(),
                amount_ml: Some(1.0),
                amount_scoop: None,
                amount_g: None,
                amount_mol: None,
            },
            CompositionEntry {
                substance_id: "na+".into(),
                phase: "aqueous".into(),
                amount_ml: None,
                amount_scoop: None,
                amount_g: None,
                amount_mol: Some(0.02),
            },
            CompositionEntry {
                substance_id: "cl-".into(),
                phase: "aqueous".into(),
                amount_ml: None,
                amount_scoop: None,
                amount_g: None,
                amount_mol: Some(0.02),
            },
        ];
        crate::solubility::enforce_saturation(dish);
        let dish = item(&scene, "dish-1");
        let na_before = aqueous_mol(dish, "na+");
        let solid_before = dish
            .properties
            .composition
            .iter()
            .find(|c| c.substance_id == "nacl" && c.phase == "solid")
            .and_then(|c| c.amount_mol)
            .unwrap_or(0.0);
        assert!(solid_before > 1e-6, "need leftover solid at 20 °C");

        apply_action(
            &mut scene,
            Action::ToggleBurner {
                burner_item_id: "burner-1".into(),
            },
        )
        .unwrap();
        apply_elapsed(&mut scene, 4.0);

        let dish = item(&scene, "dish-1");
        assert!((dish.properties.temperature_c.unwrap() - 60.0).abs() < 1e-9);
        assert!(
            (water_ml(dish) - 1.0).abs() < 1e-9,
            "no evaporation below 100"
        );
        let na_after = aqueous_mol(dish, "na+");
        let solid_after = dish
            .properties
            .composition
            .iter()
            .find(|c| c.substance_id == "nacl" && c.phase == "solid")
            .and_then(|c| c.amount_mol)
            .unwrap_or(0.0);
        assert!(
            na_after > na_before + 1e-6,
            "heating must redissolve NaCl as s(T) rises; {na_after} vs {na_before}"
        );
        assert!(solid_after < solid_before - 1e-6);
        let max_60 =
            crate::solubility::solubility_mol_per_l(crate::solubility::Salt::Nacl, 60.0) * 0.001;
        assert!((na_after - max_60).abs() < 1e-9);
    }

    #[test]
    fn evaporating_mixed_dish_precipitates_nacl_before_independent_caps() {
        let mut scene = initial_bench_scene("lab-test");
        let s_nacl = crate::solubility::solubility_mol_per_l(crate::solubility::Salt::Nacl, 100.0);
        let s_cacl2 =
            crate::solubility::solubility_mol_per_l(crate::solubility::Salt::Cacl2, 100.0);
        let n_nacl = s_nacl * 0.003;
        let n_cacl2 = s_cacl2 * 0.003;
        let dish = scene.items.iter_mut().find(|i| i.id == "dish-1").unwrap();
        dish.properties.temperature_c = Some(100.0);
        dish.properties.composition = vec![
            CompositionEntry {
                substance_id: "water".into(),
                phase: "liquid".into(),
                amount_ml: Some(15.0),
                amount_scoop: None,
                amount_g: None,
                amount_mol: None,
            },
            CompositionEntry {
                substance_id: "na+".into(),
                phase: "aqueous".into(),
                amount_ml: None,
                amount_scoop: None,
                amount_g: None,
                amount_mol: Some(n_nacl),
            },
            CompositionEntry {
                substance_id: "ca2+".into(),
                phase: "aqueous".into(),
                amount_ml: None,
                amount_scoop: None,
                amount_g: None,
                amount_mol: Some(n_cacl2),
            },
            CompositionEntry {
                substance_id: "cl-".into(),
                phase: "aqueous".into(),
                amount_ml: None,
                amount_scoop: None,
                amount_g: None,
                amount_mol: Some(n_nacl + 2.0 * n_cacl2),
            },
        ];
        crate::solubility::enforce_saturation(dish);
        assert!(!item(&scene, "dish-1")
            .properties
            .composition
            .iter()
            .any(|c| c.substance_id == "nacl" && c.phase == "solid"));

        apply_action(
            &mut scene,
            Action::ToggleBurner {
                burner_item_id: "burner-1".into(),
            },
        )
        .unwrap();
        apply_elapsed(&mut scene, 26.0);
        let dish = item(&scene, "dish-1");
        assert!((water_ml(dish) - 2.0).abs() < 1e-9);
        assert!(
            dish.properties
                .composition
                .iter()
                .any(|c| c.substance_id == "nacl" && c.phase == "solid"),
            "mixed evaporation must crash out NaCl"
        );
        let na = aqueous_mol(dish, "na+");
        let ca = aqueous_mol(dish, "ca2+");
        let ind_nacl = s_nacl * 0.002;
        let ind_cacl2 = s_cacl2 * 0.002;
        assert!(na < ind_nacl - 1e-6);
        assert!((na - ind_nacl).abs() > 1e-6 || (ca - ind_cacl2).abs() > 1e-6);
    }

    #[test]
    fn pipette_in_redissolves_solid_salt_up_to_solubility() {
        let mut scene = bench_with_water("lab-test");
        let dish = scene.items.iter_mut().find(|i| i.id == "dish-1").unwrap();
        dish.properties.composition = vec![CompositionEntry {
            substance_id: "nacl".into(),
            phase: "solid".into(),
            amount_ml: None,
            amount_scoop: None,
            amount_g: Some(0.02 * NACL_MOLAR_MASS_G_PER_MOL),
            amount_mol: Some(0.02),
        }];
        crate::solubility::enforce_saturation(dish);
        assert!(aqueous_mol(item(&scene, "dish-1"), "na+") < 1e-12);

        fill_pipette_from(&mut scene, "beaker-water");
        apply_action(
            &mut scene,
            Action::Pour {
                source_item_id: "pipette-1".into(),
                target_item_id: "dish-1".into(),
            },
        )
        .unwrap();

        let dish = item(&scene, "dish-1");
        let max_aq =
            crate::solubility::solubility_mol_per_l(crate::solubility::Salt::Nacl, 20.0) * 0.001;
        assert!((aqueous_mol(dish, "na+") - max_aq).abs() < 1e-9);
        assert!(
            dish.properties
                .composition
                .iter()
                .any(|c| c.substance_id == "nacl" && c.phase == "solid"),
            "leftover solid remains above 1 ml solubility"
        );
    }

    #[test]
    fn toggle_burner_stays_off_when_dish_has_no_liquid() {
        let mut scene = initial_bench_scene("lab-test");
        apply_action(
            &mut scene,
            Action::ToggleBurner {
                burner_item_id: "burner-1".into(),
            },
        )
        .unwrap();
        assert_eq!(item(&scene, "burner-1").properties.on, Some(false));
        assert!(scene.last_events.is_empty());
    }

    #[test]
    fn pipette_put_away_returns_aliquot_to_last_source() {
        let mut scene = bench_with_water("lab-test");
        fill_pipette_from(&mut scene, "beaker-water");
        apply_action(
            &mut scene,
            Action::PutAway {
                tool_item_id: "pipette-1".into(),
            },
        )
        .unwrap();
        assert!((water_ml(item(&scene, "beaker-water")) - 200.0).abs() < 1e-9);
        let pipette = item(&scene, "pipette-1");
        assert_eq!(pipette.location, "bench");
        assert!(pipette.properties.holding.is_empty());
    }

    #[test]
    fn reset_clears_dish_burner_and_pipette() {
        let mut scene = bench_with_water("lab-test");
        fill_pipette_from(&mut scene, "beaker-water");
        apply_action(
            &mut scene,
            Action::Pour {
                source_item_id: "pipette-1".into(),
                target_item_id: "dish-1".into(),
            },
        )
        .unwrap();
        apply_action(
            &mut scene,
            Action::ToggleBurner {
                burner_item_id: "burner-1".into(),
            },
        )
        .unwrap();
        apply_action(&mut scene, Action::Reset).unwrap();
        assert!(item(&scene, "dish-1").properties.composition.is_empty());
        assert_eq!(
            item(&scene, "dish-1").properties.temperature_c,
            Some(AMBIENT_TEMPERATURE_C)
        );
        assert_eq!(item(&scene, "burner-1").properties.on, Some(false));
        assert!(item(&scene, "pipette-1").properties.holding.is_empty());
        assert_eq!(item(&scene, "pipette-1").location, "bench");
        assert!((water_ml(item(&scene, "beaker-water")) - 0.0).abs() < 1e-9);
    }

    #[test]
    fn use_tool_full_pipette_empties_into_dish() {
        let mut scene = bench_with_water("lab-test");
        fill_pipette_from(&mut scene, "beaker-water");
        apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "pipette-1".into(),
                target_item_id: "dish-1".into(),
            },
        )
        .unwrap();
        assert!((water_ml(item(&scene, "dish-1")) - 1.0).abs() < 1e-9);
        assert!(item(&scene, "pipette-1").properties.holding.is_empty());
    }

    fn use_tongs(scene: &mut Scene, target_id: &str) -> Result<(), SceneError> {
        apply_action(
            scene,
            Action::UseTool {
                tool_item_id: "tongs-1".into(),
                target_item_id: target_id.into(),
            },
        )
    }

    fn put_tongs_away(scene: &mut Scene) -> Result<(), SceneError> {
        apply_action(
            scene,
            Action::PutAway {
                tool_item_id: "tongs-1".into(),
            },
        )
    }

    fn solid_g(item: &SceneItem, substance_id: &str) -> f64 {
        item.properties
            .composition
            .iter()
            .find(|c| c.substance_id == substance_id && c.phase == "solid")
            .and_then(|c| c.amount_g)
            .unwrap_or(0.0)
    }

    #[test]
    fn tongs_use_without_tongs_item_returns_unknown_item() {
        let mut scene = initial_bench_scene("lab-test");
        scene.items.retain(|item| item.id != "tongs-1");
        assert_eq!(
            use_tongs(&mut scene, "beaker-water").unwrap_err(),
            SceneError::UnknownItem
        );
    }

    #[test]
    fn ensure_default_bench_items_is_idempotent_when_catalog_is_complete() {
        let mut scene = initial_bench_scene("lab-test");
        let before = scene.items.len();
        ensure_default_bench_items(&mut scene);
        ensure_default_bench_items(&mut scene);
        assert_eq!(scene.items.len(), before);
        assert_eq!(
            scene
                .items
                .iter()
                .filter(|item| item.id == "tongs-1")
                .count(),
            1
        );
    }

    #[test]
    fn ensure_default_bench_items_restores_tongs_without_resetting_vessels() {
        let mut scene = initial_bench_scene("lab-test");
        scene
            .items
            .iter_mut()
            .find(|item| item.id == "beaker-water")
            .unwrap()
            .properties
            .fill_ml = Some(150.0);
        scene.items.retain(|item| item.id != "tongs-1");

        ensure_default_bench_items(&mut scene);

        let tongs = item(&scene, "tongs-1");
        assert_eq!(tongs.kind, "tongs");
        assert_eq!(tongs.location, "bench");
        assert_eq!(tongs.properties.source_item_id, None);
        assert_eq!(item(&scene, "beaker-water").properties.fill_ml, Some(150.0));

        use_tongs(&mut scene, "beaker-water").unwrap();
        assert_eq!(item(&scene, "beaker-water").location, "held");
        assert_eq!(
            item(&scene, "tongs-1").properties.source_item_id.as_deref(),
            Some("beaker-water")
        );
    }

    #[test]
    fn tongs_pick_up_water_sets_held_location_and_records_source() {
        let mut scene = initial_bench_scene("lab-test");
        use_tongs(&mut scene, "beaker-water").unwrap();

        let water = item(&scene, "beaker-water");
        assert_eq!(water.location, "held");
        let tongs = item(&scene, "tongs-1");
        assert_eq!(tongs.location, "hand");
        assert_eq!(
            tongs.properties.source_item_id.as_deref(),
            Some("beaker-water")
        );
        assert_eq!(item(&scene, "dish-1").location, "bench");
    }

    #[test]
    fn tongs_pick_up_dish_turns_burner_off() {
        let mut scene = bench_with_water("lab-test");
        fill_pipette_from(&mut scene, "beaker-water");
        apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "pipette-1".into(),
                target_item_id: "dish-1".into(),
            },
        )
        .unwrap();
        apply_action(
            &mut scene,
            Action::ToggleBurner {
                burner_item_id: "burner-1".into(),
            },
        )
        .unwrap();
        assert_eq!(item(&scene, "burner-1").properties.on, Some(true));

        use_tongs(&mut scene, "dish-1").unwrap();

        assert_eq!(item(&scene, "dish-1").location, "held");
        assert_eq!(item(&scene, "burner-1").properties.on, Some(false));
        assert_eq!(
            item(&scene, "tongs-1").properties.source_item_id.as_deref(),
            Some("dish-1")
        );
    }

    #[test]
    fn tongs_pour_water_into_dish_fills_to_capacity_and_scales_ions_and_solids() {
        let mut scene = bench_with_water("lab-test");
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

        let water_before = item(&scene, "beaker-water");
        let source_ml = water_ml(water_before);
        let na_before = aqueous_mol(water_before, "na+");
        let cl_before = aqueous_mol(water_before, "cl-");
        let sand_before = solid_g(water_before, "sand");
        let frac = DISH_CAPACITY_ML / source_ml;

        use_tongs(&mut scene, "beaker-water").unwrap();
        use_tongs(&mut scene, "dish-1").unwrap();

        let water = item(&scene, "beaker-water");
        let dish = item(&scene, "dish-1");
        assert_eq!(water.location, "held");
        assert!((water_ml(dish) - DISH_CAPACITY_ML).abs() < 1e-9);
        assert!((water_ml(water) - (source_ml - DISH_CAPACITY_ML)).abs() < 1e-9);
        assert!((aqueous_mol(dish, "na+") - na_before * frac).abs() < 1e-12);
        assert!((aqueous_mol(dish, "cl-") - cl_before * frac).abs() < 1e-12);
        assert!((solid_g(dish, "sand") - sand_before * frac).abs() < 1e-12);
        assert!((aqueous_mol(water, "na+") - na_before * (1.0 - frac)).abs() < 1e-12);
        assert!((solid_g(water, "sand") - sand_before * (1.0 - frac)).abs() < 1e-12);
        assert!(scene.last_events.iter().any(|e| e.kind == "poured"));
        assert_eq!(
            item(&scene, "tongs-1").properties.source_item_id.as_deref(),
            Some("beaker-water")
        );
    }

    #[test]
    fn tongs_pour_dish_into_water_moves_contents_and_keeps_holding() {
        let mut scene = bench_with_water("lab-test");
        fill_pipette_from(&mut scene, "beaker-water");
        apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "pipette-1".into(),
                target_item_id: "dish-1".into(),
            },
        )
        .unwrap();
        assert!((water_ml(item(&scene, "dish-1")) - 1.0).abs() < 1e-9);

        use_tongs(&mut scene, "dish-1").unwrap();
        use_tongs(&mut scene, "beaker-water").unwrap();

        assert!((water_ml(item(&scene, "dish-1")) - 0.0).abs() < 1e-9);
        assert!((water_ml(item(&scene, "beaker-water")) - 200.0).abs() < 1e-9);
        assert_eq!(item(&scene, "dish-1").location, "held");
        assert_eq!(
            item(&scene, "tongs-1").properties.source_item_id.as_deref(),
            Some("dish-1")
        );
    }

    #[test]
    fn tongs_pour_stops_when_destination_is_full() {
        let mut scene = bench_with_water("lab-test");
        use_tongs(&mut scene, "beaker-water").unwrap();
        use_tongs(&mut scene, "dish-1").unwrap();
        assert!((water_ml(item(&scene, "dish-1")) - DISH_CAPACITY_ML).abs() < 1e-9);

        let err = use_tongs(&mut scene, "dish-1").unwrap_err();
        assert_eq!(err, SceneError::InvalidAction);
        assert_eq!(item(&scene, "beaker-water").location, "held");
        assert!((water_ml(item(&scene, "dish-1")) - DISH_CAPACITY_ML).abs() < 1e-9);
        assert!((water_ml(item(&scene, "beaker-water")) - 175.0).abs() < 1e-9);
    }

    #[test]
    fn tongs_solids_only_dump_moves_all_solids_even_into_full_destination() {
        let mut scene = initial_bench_scene("lab-test");
        let water = scene
            .items
            .iter_mut()
            .find(|i| i.id == "beaker-water")
            .unwrap();
        water.properties.composition = vec![CompositionEntry {
            substance_id: "water".into(),
            phase: "liquid".into(),
            amount_ml: Some(250.0),
            amount_scoop: None,
            amount_g: None,
            amount_mol: None,
        }];
        crate::solubility::sync_fill_ml(water);

        let dish = scene.items.iter_mut().find(|i| i.id == "dish-1").unwrap();
        dish.properties.composition = vec![CompositionEntry {
            substance_id: "sand".into(),
            phase: "solid".into(),
            amount_ml: None,
            amount_scoop: Some(2),
            amount_g: Some(2.0 * SPOON_SCOOP_MASS_G),
            amount_mol: None,
        }];
        dish.properties.fill_ml = Some(0.0);

        use_tongs(&mut scene, "dish-1").unwrap();
        use_tongs(&mut scene, "beaker-water").unwrap();

        assert!((water_ml(item(&scene, "beaker-water")) - 250.0).abs() < 1e-9);
        assert!(
            (solid_g(item(&scene, "beaker-water"), "sand") - 2.0 * SPOON_SCOOP_MASS_G).abs()
                < 1e-12
        );
        assert_eq!(solid_g(item(&scene, "dish-1"), "sand"), 0.0);
        assert!(item(&scene, "dish-1")
            .properties
            .composition
            .iter()
            .all(|c| c.phase != "solid"));
        assert_eq!(item(&scene, "dish-1").location, "held");
    }

    #[test]
    fn tongs_full_dest_dumps_all_solids_from_wet_source_and_keeps_liquid() {
        let mut scene = initial_bench_scene("lab-test");
        let water = scene
            .items
            .iter_mut()
            .find(|i| i.id == "beaker-water")
            .unwrap();
        water.properties.composition = vec![CompositionEntry {
            substance_id: "water".into(),
            phase: "liquid".into(),
            amount_ml: Some(250.0),
            amount_scoop: None,
            amount_g: None,
            amount_mol: None,
        }];
        crate::solubility::sync_fill_ml(water);

        let dish = scene.items.iter_mut().find(|i| i.id == "dish-1").unwrap();
        dish.properties.composition = vec![
            CompositionEntry {
                substance_id: "water".into(),
                phase: "liquid".into(),
                amount_ml: Some(10.0),
                amount_scoop: None,
                amount_g: None,
                amount_mol: None,
            },
            CompositionEntry {
                substance_id: "sand".into(),
                phase: "solid".into(),
                amount_ml: None,
                amount_scoop: Some(2),
                amount_g: Some(2.0 * SPOON_SCOOP_MASS_G),
                amount_mol: None,
            },
        ];
        crate::solubility::sync_fill_ml(dish);

        use_tongs(&mut scene, "dish-1").unwrap();
        use_tongs(&mut scene, "beaker-water").unwrap();

        assert!((water_ml(item(&scene, "beaker-water")) - 250.0).abs() < 1e-9);
        assert!((water_ml(item(&scene, "dish-1")) - 10.0).abs() < 1e-9);
        assert!(
            (solid_g(item(&scene, "beaker-water"), "sand") - 2.0 * SPOON_SCOOP_MASS_G).abs()
                < 1e-12
        );
        assert_eq!(solid_g(item(&scene, "dish-1"), "sand"), 0.0);
        assert_eq!(item(&scene, "dish-1").location, "held");
        assert_eq!(
            item(&scene, "tongs-1").properties.source_item_id.as_deref(),
            Some("dish-1")
        );
    }

    #[test]
    fn tongs_put_away_returns_water_and_dish_home() {
        let mut scene = initial_bench_scene("lab-test");
        use_tongs(&mut scene, "beaker-water").unwrap();
        put_tongs_away(&mut scene).unwrap();
        assert_eq!(item(&scene, "beaker-water").location, "bench");
        assert_eq!(item(&scene, "tongs-1").location, "bench");
        assert_eq!(item(&scene, "tongs-1").properties.source_item_id, None);

        use_tongs(&mut scene, "dish-1").unwrap();
        put_tongs_away(&mut scene).unwrap();
        assert_eq!(item(&scene, "dish-1").location, "bench");
        assert_eq!(item(&scene, "tongs-1").location, "bench");
        assert_eq!(item(&scene, "tongs-1").properties.source_item_id, None);
    }

    #[test]
    fn tongs_pick_up_and_put_away_each_solid_stock() {
        for stock_id in ["beaker-nacl", "beaker-cacl2", "beaker-sand"] {
            let mut scene = initial_bench_scene("lab-test");
            use_tongs(&mut scene, stock_id).unwrap();
            assert_eq!(item(&scene, stock_id).location, "held");
            assert_eq!(item(&scene, "tongs-1").location, "hand");
            assert_eq!(
                item(&scene, "tongs-1").properties.source_item_id.as_deref(),
                Some(stock_id)
            );

            put_tongs_away(&mut scene).unwrap();
            assert_eq!(item(&scene, stock_id).location, "bench");
            assert_eq!(item(&scene, "tongs-1").location, "bench");
            assert_eq!(item(&scene, "tongs-1").properties.source_item_id, None);
        }
    }

    #[test]
    fn tongs_dump_all_nacl_into_empty_beaker() {
        let mut scene = initial_bench_scene("lab-test");
        let stock_g = solid_g(item(&scene, "beaker-nacl"), "nacl");
        assert!(stock_g > 0.0);

        use_tongs(&mut scene, "beaker-nacl").unwrap();
        use_tongs(&mut scene, "beaker-water").unwrap();

        assert_eq!(solid_g(item(&scene, "beaker-nacl"), "nacl"), 0.0);
        assert!((solid_g(item(&scene, "beaker-water"), "nacl") - stock_g).abs() < 1e-12);
        assert_eq!(aqueous_mol(item(&scene, "beaker-water"), "na+"), 0.0);
        assert_eq!(item(&scene, "beaker-nacl").location, "held");
        assert_eq!(
            item(&scene, "tongs-1").properties.source_item_id.as_deref(),
            Some("beaker-nacl")
        );
        assert!(scene.last_events.iter().any(|e| e.kind == "poured"));
        assert!(!scene.last_events.iter().any(|e| e.kind == "dissolved"));
    }

    #[test]
    fn tongs_dump_nacl_into_solids_only_beaker() {
        let mut scene = initial_bench_scene("lab-test");
        use_tongs(&mut scene, "beaker-sand").unwrap();
        use_tongs(&mut scene, "beaker-water").unwrap();
        put_tongs_away(&mut scene).unwrap();

        let sand_g = solid_g(item(&scene, "beaker-water"), "sand");
        let nacl_g = solid_g(item(&scene, "beaker-nacl"), "nacl");
        use_tongs(&mut scene, "beaker-nacl").unwrap();
        use_tongs(&mut scene, "beaker-water").unwrap();

        assert!((solid_g(item(&scene, "beaker-water"), "sand") - sand_g).abs() < 1e-12);
        assert!((solid_g(item(&scene, "beaker-water"), "nacl") - nacl_g).abs() < 1e-12);
        assert_eq!(aqueous_mol(item(&scene, "beaker-water"), "na+"), 0.0);
        assert_eq!(water_ml(item(&scene, "beaker-water")), 0.0);
        assert_eq!(item(&scene, "beaker-nacl").location, "held");
    }

    #[test]
    fn tongs_dump_nacl_into_filled_beaker_enforces_saturation() {
        let mut scene = bench_with_water("lab-test");
        let nacl_g = solid_g(item(&scene, "beaker-nacl"), "nacl");

        use_tongs(&mut scene, "beaker-nacl").unwrap();
        use_tongs(&mut scene, "beaker-water").unwrap();

        let expected = nacl_g / NACL_MOLAR_MASS_G_PER_MOL;
        assert!((aqueous_mol(item(&scene, "beaker-water"), "na+") - expected).abs() < 1e-9);
        assert!((aqueous_mol(item(&scene, "beaker-water"), "cl-") - expected).abs() < 1e-9);
        assert_eq!(solid_g(item(&scene, "beaker-water"), "nacl"), 0.0);
        assert_eq!(solid_g(item(&scene, "beaker-nacl"), "nacl"), 0.0);
        assert_eq!(item(&scene, "beaker-nacl").location, "held");
    }

    #[test]
    fn tongs_empty_stock_pour_is_empty_holding() {
        let mut scene = initial_bench_scene("lab-test");
        use_tongs(&mut scene, "beaker-nacl").unwrap();
        use_tongs(&mut scene, "beaker-water").unwrap();
        let err = use_tongs(&mut scene, "beaker-water").unwrap_err();
        assert_eq!(err, SceneError::EmptyHolding);
        assert_eq!(item(&scene, "beaker-nacl").location, "held");
        assert_eq!(
            item(&scene, "tongs-1").properties.source_item_id.as_deref(),
            Some("beaker-nacl")
        );
    }

    #[test]
    fn tongs_reject_held_solid_stock_into_dish_h2o_and_other_stock() {
        let mut scene = initial_bench_scene("lab-test");
        use_tongs(&mut scene, "beaker-nacl").unwrap();
        let stock_g = solid_g(item(&scene, "beaker-nacl"), "nacl");
        for dest in [
            "dish-1",
            "beaker-h2o",
            "beaker-cacl2",
            "burner-1",
            "beaker-filtrate",
            "filter-paper-1",
        ] {
            assert_eq!(
                use_tongs(&mut scene, dest).unwrap_err(),
                SceneError::InvalidAction
            );
        }
        assert_eq!(item(&scene, "beaker-nacl").location, "held");
        assert!((solid_g(item(&scene, "beaker-nacl"), "nacl") - stock_g).abs() < 1e-12);

        use_tongs(&mut scene, "beaker-water").unwrap();
        for dest in ["beaker-filtrate", "filter-paper-1"] {
            assert_eq!(
                use_tongs(&mut scene, dest).unwrap_err(),
                SceneError::InvalidAction
            );
        }
        assert_eq!(item(&scene, "beaker-nacl").location, "held");
        assert_eq!(solid_g(item(&scene, "beaker-nacl"), "nacl"), 0.0);
    }

    #[test]
    fn tongs_reject_pouring_into_sitting_solid_stock() {
        let mut scene = bench_with_water("lab-test");
        use_tongs(&mut scene, "beaker-water").unwrap();
        assert_eq!(
            use_tongs(&mut scene, "beaker-nacl").unwrap_err(),
            SceneError::InvalidAction
        );
        assert_eq!(item(&scene, "beaker-water").location, "held");
        assert!(solid_g(item(&scene, "beaker-nacl"), "nacl") > 0.0);
    }

    #[test]
    fn tongs_reject_burner_and_already_held_vessel() {
        let mut scene = initial_bench_scene("lab-test");
        assert_eq!(
            use_tongs(&mut scene, "burner-1").unwrap_err(),
            SceneError::InvalidAction
        );

        use_tongs(&mut scene, "beaker-water").unwrap();
        assert_eq!(
            use_tongs(&mut scene, "beaker-water").unwrap_err(),
            SceneError::InvalidAction
        );
        assert_eq!(item(&scene, "beaker-water").location, "held");
    }

    #[test]
    fn pour_scoop_into_empty_beaker_adds_undissolved_solid() {
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

        assert!(item(&scene, "spoon-1").properties.holding.is_empty());
        assert!((solid_g(item(&scene, "beaker-water"), "nacl") - SPOON_SCOOP_MASS_G).abs() < 1e-12);
        assert_eq!(aqueous_mol(item(&scene, "beaker-water"), "na+"), 0.0);
        assert!(scene.last_events.iter().any(|e| e.kind == "poured"));
        assert!(!scene.last_events.iter().any(|e| e.kind == "dissolved"));
    }

    #[test]
    fn tongs_empty_source_with_nothing_to_move_errors_and_stays_holding() {
        let mut scene = initial_bench_scene("lab-test");
        use_tongs(&mut scene, "dish-1").unwrap();
        let err = use_tongs(&mut scene, "beaker-water").unwrap_err();
        assert_eq!(err, SceneError::EmptyHolding);
        assert_eq!(item(&scene, "dish-1").location, "held");
        assert_eq!(
            item(&scene, "tongs-1").properties.source_item_id.as_deref(),
            Some("dish-1")
        );
    }

    #[test]
    fn tongs_pour_uses_volume_weighted_destination_temperature() {
        let mut scene = bench_with_water("lab-test");
        let dish = scene.items.iter_mut().find(|i| i.id == "dish-1").unwrap();
        dish.properties.temperature_c = Some(80.0);
        dish.properties.composition = vec![CompositionEntry {
            substance_id: "water".into(),
            phase: "liquid".into(),
            amount_ml: Some(10.0),
            amount_scoop: None,
            amount_g: None,
            amount_mol: None,
        }];
        crate::solubility::sync_fill_ml(dish);

        use_tongs(&mut scene, "beaker-water").unwrap();
        use_tongs(&mut scene, "dish-1").unwrap();

        let dish = item(&scene, "dish-1");
        assert!((water_ml(dish) - 25.0).abs() < 1e-9);
        let expected_t = (10.0 * 80.0 + 15.0 * 20.0) / 25.0;
        assert!((dish.properties.temperature_c.unwrap() - expected_t).abs() < 1e-9);
        assert_eq!(
            item(&scene, "beaker-water").properties.temperature_c,
            Some(20.0)
        );
    }

    fn solid(substance_id: &str, grams: f64) -> CompositionEntry {
        CompositionEntry {
            substance_id: substance_id.into(),
            phase: "solid".into(),
            amount_ml: None,
            amount_scoop: None,
            amount_g: Some(grams),
            amount_mol: None,
        }
    }

    fn set_dry_dish_solids(scene: &mut Scene, solids: Vec<CompositionEntry>) {
        let dish = scene.items.iter_mut().find(|i| i.id == "dish-1").unwrap();
        dish.properties.composition = solids;
        crate::solubility::sync_fill_ml(dish);
    }

    fn scoop_dish(scene: &mut Scene) -> Result<(), SceneError> {
        apply_action(
            scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "dish-1".into(),
            },
        )
    }

    fn empty_nacl_stock(scene: &mut Scene) {
        let nacl = scene
            .items
            .iter_mut()
            .find(|i| i.id == "beaker-nacl")
            .unwrap();
        let stock = nacl
            .properties
            .composition
            .iter_mut()
            .find(|c| c.substance_id == "nacl" && c.phase == "solid")
            .unwrap();
        stock.amount_scoop = Some(0);
        stock.amount_g = Some(0.0);
        stock.amount_mol = None;
    }

    fn scoop_nacl_stock(scene: &mut Scene) -> Result<(), SceneError> {
        apply_action(
            scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "beaker-nacl".into(),
            },
        )
    }

    fn holding_g(item: &SceneItem, substance_id: &str) -> f64 {
        item.properties
            .holding
            .iter()
            .find(|c| c.substance_id == substance_id && c.phase == "solid")
            .and_then(|c| c.amount_g)
            .unwrap_or(0.0)
    }

    #[test]
    fn use_tool_spoon_scoops_dish_solids_in_mass_ratio() {
        let mut scene = initial_bench_scene("lab-test");
        set_dry_dish_solids(&mut scene, vec![solid("nacl", 0.6), solid("cacl2", 0.4)]);
        scoop_dish(&mut scene).unwrap();

        let spoon = item(&scene, "spoon-1");
        assert_eq!(spoon.location, "hand");
        assert_eq!(spoon.properties.source_item_id.as_deref(), Some("dish-1"));
        assert!((holding_g(spoon, "nacl") - 0.12).abs() < 1e-12);
        assert!((holding_g(spoon, "cacl2") - 0.08).abs() < 1e-12);
        assert!(
            (holding_g(spoon, "nacl") + holding_g(spoon, "cacl2") - SPOON_SCOOP_MASS_G).abs()
                < 1e-12
        );
        assert!((solid_g(item(&scene, "dish-1"), "nacl") - 0.48).abs() < 1e-12);
        assert!((solid_g(item(&scene, "dish-1"), "cacl2") - 0.32).abs() < 1e-12);
        assert!(scene.last_events.iter().any(|e| e.kind == "scooped"));
        assert_eq!(solid_g(item(&scene, "beaker-nacl"), "nacl"), 2.0);
        assert_eq!(solid_g(item(&scene, "beaker-cacl2"), "cacl2"), 2.0);
    }

    #[test]
    fn use_tool_spoon_takes_all_when_dish_solids_below_scoop_mass() {
        let mut scene = initial_bench_scene("lab-test");
        set_dry_dish_solids(&mut scene, vec![solid("nacl", 0.05), solid("sand", 0.05)]);
        scoop_dish(&mut scene).unwrap();

        let spoon = item(&scene, "spoon-1");
        assert!((holding_g(spoon, "nacl") - 0.05).abs() < 1e-12);
        assert!((holding_g(spoon, "sand") - 0.05).abs() < 1e-12);
        assert!(item(&scene, "dish-1")
            .properties
            .composition
            .iter()
            .all(|c| c.phase != "solid"));
        assert_eq!(solid_g(item(&scene, "dish-1"), "nacl"), 0.0);
        assert_eq!(spoon.properties.source_item_id.as_deref(), Some("dish-1"));
    }

    #[test]
    fn pour_dumps_mixed_dish_scoop_into_water_and_clears_spoon() {
        let mut scene = bench_with_water("lab-test");
        set_dry_dish_solids(&mut scene, vec![solid("nacl", 0.6), solid("cacl2", 0.4)]);
        scoop_dish(&mut scene).unwrap();
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
        assert_eq!(spoon.properties.source_item_id, None);
        let water = item(&scene, "beaker-water");
        let expected_na = 0.12 / NACL_MOLAR_MASS_G_PER_MOL;
        let expected_ca = 0.08 / CACL2_MOLAR_MASS_G_PER_MOL;
        assert!((aqueous_mol(water, "na+") - expected_na).abs() < 1e-9);
        assert!((aqueous_mol(water, "ca2+") - expected_ca).abs() < 1e-9);
        assert!((aqueous_mol(water, "cl-") - (expected_na + 2.0 * expected_ca)).abs() < 1e-9);
        assert!(scene.last_events.iter().any(|e| e.kind == "dissolved"));
    }

    #[test]
    fn use_tool_dumps_dish_scoop_into_water() {
        let mut scene = bench_with_water("lab-test");
        set_dry_dish_solids(&mut scene, vec![solid("nacl", 0.5)]);
        scoop_dish(&mut scene).unwrap();
        apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "beaker-water".into(),
            },
        )
        .unwrap();

        let spoon = item(&scene, "spoon-1");
        assert!(spoon.properties.holding.is_empty());
        assert_eq!(spoon.properties.source_item_id, None);
        let expected_na = SPOON_SCOOP_MASS_G / NACL_MOLAR_MASS_G_PER_MOL;
        assert!((aqueous_mol(item(&scene, "beaker-water"), "na+") - expected_na).abs() < 1e-9);
        assert!((solid_g(item(&scene, "dish-1"), "nacl") - 0.3).abs() < 1e-12);
    }

    #[test]
    fn use_tool_returns_single_species_dish_scoop_to_matching_stock() {
        let mut scene = initial_bench_scene("lab-test");
        set_dry_dish_solids(&mut scene, vec![solid("nacl", 0.5)]);
        scoop_dish(&mut scene).unwrap();
        apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "beaker-nacl".into(),
            },
        )
        .unwrap();

        let spoon = item(&scene, "spoon-1");
        assert!(spoon.properties.holding.is_empty());
        assert_eq!(spoon.properties.source_item_id, None);
        assert!((solid_g(item(&scene, "beaker-nacl"), "nacl") - 2.2).abs() < 1e-12);
        assert!((solid_g(item(&scene, "dish-1"), "nacl") - 0.3).abs() < 1e-12);
        assert!(scene.last_events.iter().any(|e| e.kind == "returned"));
    }

    #[test]
    fn use_tool_returns_short_dish_scoop_mass_to_matching_stock() {
        let mut scene = initial_bench_scene("lab-test");
        set_dry_dish_solids(&mut scene, vec![solid("nacl", 0.1)]);
        scoop_dish(&mut scene).unwrap();
        apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "beaker-nacl".into(),
            },
        )
        .unwrap();

        assert!(item(&scene, "spoon-1").properties.holding.is_empty());
        assert!((solid_g(item(&scene, "beaker-nacl"), "nacl") - 2.1).abs() < 1e-12);
        assert_eq!(solid_g(item(&scene, "dish-1"), "nacl"), 0.0);
    }

    #[test]
    fn use_tool_rejects_mixed_dish_scoop_into_stock() {
        let mut scene = initial_bench_scene("lab-test");
        set_dry_dish_solids(&mut scene, vec![solid("nacl", 0.6), solid("cacl2", 0.4)]);
        scoop_dish(&mut scene).unwrap();
        let err = apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "beaker-nacl".into(),
            },
        )
        .unwrap_err();
        assert_eq!(err, SceneError::InvalidAction);
        let spoon = item(&scene, "spoon-1");
        assert!((holding_g(spoon, "nacl") - 0.12).abs() < 1e-12);
        assert!((holding_g(spoon, "cacl2") - 0.08).abs() < 1e-12);
        assert_eq!(solid_g(item(&scene, "beaker-nacl"), "nacl"), 2.0);
    }

    #[test]
    fn use_tool_rejects_wet_dish_and_stays_empty() {
        let mut scene = initial_bench_scene("lab-test");
        let dish = scene.items.iter_mut().find(|i| i.id == "dish-1").unwrap();
        dish.properties.composition = vec![
            CompositionEntry {
                substance_id: "water".into(),
                phase: "liquid".into(),
                amount_ml: Some(5.0),
                amount_scoop: None,
                amount_g: None,
                amount_mol: None,
            },
            solid("nacl", 0.5),
        ];
        crate::solubility::sync_fill_ml(dish);

        let err = scoop_dish(&mut scene).unwrap_err();
        assert_eq!(err, SceneError::InvalidAction);
        let spoon = item(&scene, "spoon-1");
        assert!(spoon.properties.holding.is_empty());
        assert_eq!(spoon.properties.source_item_id, None);
        assert!((solid_g(item(&scene, "dish-1"), "nacl") - 0.5).abs() < 1e-12);
        assert!((water_ml(item(&scene, "dish-1")) - 5.0).abs() < 1e-12);
    }

    #[test]
    fn use_tool_rejects_empty_dish_and_tongs_held_dish() {
        let mut scene = initial_bench_scene("lab-test");
        assert_eq!(
            scoop_dish(&mut scene).unwrap_err(),
            SceneError::InvalidAction
        );

        set_dry_dish_solids(&mut scene, vec![solid("sand", 0.4)]);
        scene
            .items
            .iter_mut()
            .find(|i| i.id == "dish-1")
            .unwrap()
            .location = "held".into();
        assert_eq!(
            scoop_dish(&mut scene).unwrap_err(),
            SceneError::InvalidAction
        );
        assert!(item(&scene, "spoon-1").properties.holding.is_empty());
        assert!((solid_g(item(&scene, "dish-1"), "sand") - 0.4).abs() < 1e-12);
    }

    #[test]
    fn put_away_returns_dish_scoop_to_dish_not_stock() {
        let mut scene = initial_bench_scene("lab-test");
        set_dry_dish_solids(&mut scene, vec![solid("nacl", 0.6), solid("cacl2", 0.4)]);
        scoop_dish(&mut scene).unwrap();
        apply_action(
            &mut scene,
            Action::PutAway {
                tool_item_id: "spoon-1".into(),
            },
        )
        .unwrap();

        let spoon = item(&scene, "spoon-1");
        assert!(spoon.properties.holding.is_empty());
        assert_eq!(spoon.location, "bench");
        assert_eq!(spoon.properties.source_item_id, None);
        assert!((solid_g(item(&scene, "dish-1"), "nacl") - 0.6).abs() < 1e-12);
        assert!((solid_g(item(&scene, "dish-1"), "cacl2") - 0.4).abs() < 1e-12);
        assert_eq!(solid_g(item(&scene, "beaker-nacl"), "nacl"), 2.0);
        assert_eq!(solid_g(item(&scene, "beaker-cacl2"), "cacl2"), 2.0);
    }

    #[test]
    fn returning_dish_scoop_then_stock_scoop_conserves_nacl_mass() {
        let mut scene = initial_bench_scene("lab-test");
        set_dry_dish_solids(&mut scene, vec![solid("nacl", 0.5)]);
        let start_total =
            solid_g(item(&scene, "beaker-nacl"), "nacl") + solid_g(item(&scene, "dish-1"), "nacl");
        scoop_dish(&mut scene).unwrap();
        apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "beaker-nacl".into(),
            },
        )
        .unwrap();
        assert!((solid_g(item(&scene, "beaker-nacl"), "nacl") - 2.2).abs() < 1e-12);

        apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "beaker-nacl".into(),
            },
        )
        .unwrap();

        let conserved = solid_g(item(&scene, "beaker-nacl"), "nacl")
            + holding_g(item(&scene, "spoon-1"), "nacl")
            + solid_g(item(&scene, "dish-1"), "nacl");
        assert!(
            (conserved - start_total).abs() < 1e-12,
            "nacl mass leaked: start {start_total}, after {conserved}"
        );
        assert!((solid_g(item(&scene, "beaker-nacl"), "nacl") - 2.0).abs() < 1e-12);
        assert!((holding_g(item(&scene, "spoon-1"), "nacl") - SPOON_SCOOP_MASS_G).abs() < 1e-12);
        assert!((solid_g(item(&scene, "dish-1"), "nacl") - 0.3).abs() < 1e-12);
    }

    #[test]
    fn returning_0_25g_nacl_to_empty_stock_is_scoopable() {
        let mut scene = bench_with_water("lab-test");
        empty_nacl_stock(&mut scene);
        set_dry_dish_solids(&mut scene, vec![solid("nacl", 0.25)]);

        scoop_dish(&mut scene).unwrap();
        scoop_nacl_stock(&mut scene).unwrap();
        scoop_dish(&mut scene).unwrap();
        scoop_nacl_stock(&mut scene).unwrap();

        let stock = item(&scene, "beaker-nacl")
            .properties
            .composition
            .iter()
            .find(|c| c.substance_id == "nacl" && c.phase == "solid")
            .unwrap();
        assert!((solid_g(item(&scene, "beaker-nacl"), "nacl") - 0.25).abs() < 1e-12);
        assert_eq!(stock.amount_scoop.unwrap_or(0), 0);
        assert!(item(&scene, "spoon-1").properties.holding.is_empty());

        scoop_nacl_stock(&mut scene).unwrap();
        assert!((holding_g(item(&scene, "spoon-1"), "nacl") - SPOON_SCOOP_MASS_G).abs() < 1e-12);
        assert!((solid_g(item(&scene, "beaker-nacl"), "nacl") - 0.05).abs() < 1e-12);

        apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "beaker-water".into(),
            },
        )
        .unwrap();

        scoop_nacl_stock(&mut scene).unwrap();
        assert!((holding_g(item(&scene, "spoon-1"), "nacl") - 0.05).abs() < 1e-12);
        assert_eq!(solid_g(item(&scene, "beaker-nacl"), "nacl"), 0.0);
        let emptied = item(&scene, "beaker-nacl")
            .properties
            .composition
            .iter()
            .find(|c| c.substance_id == "nacl" && c.phase == "solid")
            .unwrap();
        assert_eq!(emptied.amount_scoop, Some(0));
        assert_eq!(emptied.amount_g, Some(0.0));
    }

    #[test]
    fn use_tool_holding_stock_scoop_rejects_dish_even_with_matching_solid() {
        let mut scene = initial_bench_scene("lab-test");
        set_dry_dish_solids(&mut scene, vec![solid("nacl", 0.5)]);
        apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "beaker-nacl".into(),
            },
        )
        .unwrap();

        let err = apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "dish-1".into(),
            },
        )
        .unwrap_err();
        assert_eq!(err, SceneError::InvalidAction);

        let spoon = item(&scene, "spoon-1");
        assert_eq!(spoon.properties.holding.len(), 1);
        assert_eq!(spoon.properties.holding[0].substance_id, "nacl");
        assert!((holding_g(spoon, "nacl") - SPOON_SCOOP_MASS_G).abs() < 1e-12);
        assert!((solid_g(item(&scene, "dish-1"), "nacl") - 0.5).abs() < 1e-12);
        assert!((solid_g(item(&scene, "beaker-nacl"), "nacl") - 1.8).abs() < 1e-12);
        assert!(!scene.last_events.iter().any(|e| e.kind == "returned"));
    }

    #[test]
    fn tongs_pick_up_beaker_h2o_and_pour_into_dish_and_water() {
        let mut scene = initial_bench_scene("lab-test");
        use_tongs(&mut scene, "beaker-h2o").unwrap();
        assert_eq!(item(&scene, "beaker-h2o").location, "held");
        assert_eq!(
            item(&scene, "tongs-1").properties.source_item_id.as_deref(),
            Some("beaker-h2o")
        );

        use_tongs(&mut scene, "dish-1").unwrap();
        assert_eq!(item(&scene, "beaker-h2o").location, "held");
        assert!((water_ml(item(&scene, "dish-1")) - DISH_CAPACITY_ML).abs() < 1e-9);
        assert!((water_ml(item(&scene, "beaker-h2o")) - 75.0).abs() < 1e-9);

        use_tongs(&mut scene, "beaker-water").unwrap();
        assert!((water_ml(item(&scene, "beaker-water")) - 75.0).abs() < 1e-9);
        assert!((water_ml(item(&scene, "beaker-h2o")) - 0.0).abs() < 1e-9);
    }

    #[test]
    fn tongs_put_away_returns_beaker_h2o_home() {
        let mut scene = initial_bench_scene("lab-test");
        use_tongs(&mut scene, "beaker-h2o").unwrap();
        put_tongs_away(&mut scene).unwrap();
        assert_eq!(item(&scene, "beaker-h2o").location, "bench");
        assert_eq!(item(&scene, "tongs-1").location, "bench");
        assert_eq!(item(&scene, "tongs-1").properties.source_item_id, None);
    }

    #[test]
    fn pipette_extracts_one_ml_from_beaker_h2o_and_dumps_into_water() {
        let mut scene = initial_bench_scene("lab-test");
        fill_pipette_from(&mut scene, "beaker-h2o");
        assert!((water_ml(item(&scene, "beaker-h2o")) - 99.0).abs() < 1e-9);
        let pipette = item(&scene, "pipette-1");
        assert!((pipette_holding_liquid_ml(pipette) - PIPETTE_VOLUME_ML).abs() < 1e-12);
        assert!(pipette
            .properties
            .holding
            .iter()
            .all(|c| { c.substance_id == "water" && c.phase == "liquid" }));
        assert_eq!(
            pipette.properties.source_item_id.as_deref(),
            Some("beaker-h2o")
        );

        apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "pipette-1".into(),
                target_item_id: "beaker-water".into(),
            },
        )
        .unwrap();
        assert!((water_ml(item(&scene, "beaker-water")) - 1.0).abs() < 1e-9);
        assert!(item(&scene, "pipette-1").properties.holding.is_empty());
    }

    #[test]
    fn pipette_put_back_pure_water_into_beaker_h2o_up_to_capacity() {
        let mut scene = initial_bench_scene("lab-test");
        fill_pipette_from(&mut scene, "beaker-h2o");
        apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "pipette-1".into(),
                target_item_id: "beaker-h2o".into(),
            },
        )
        .unwrap();
        assert!((water_ml(item(&scene, "beaker-h2o")) - 100.0).abs() < 1e-9);
        assert!(item(&scene, "pipette-1").properties.holding.is_empty());
    }

    #[test]
    fn tongs_pour_pure_water_into_beaker_h2o_stops_at_capacity() {
        let mut scene = bench_with_water("lab-test");
        fill_pipette_from(&mut scene, "beaker-h2o");
        apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "pipette-1".into(),
                target_item_id: "dish-1".into(),
            },
        )
        .unwrap();
        assert!((water_ml(item(&scene, "beaker-h2o")) - 99.0).abs() < 1e-9);

        use_tongs(&mut scene, "beaker-water").unwrap();
        use_tongs(&mut scene, "beaker-h2o").unwrap();

        assert!((water_ml(item(&scene, "beaker-h2o")) - 100.0).abs() < 1e-9);
        assert!((water_ml(item(&scene, "beaker-water")) - 199.0).abs() < 1e-9);
        assert_eq!(item(&scene, "beaker-water").location, "held");
    }

    #[test]
    fn pipette_and_tongs_reject_put_back_when_beaker_h2o_is_full() {
        let mut scene = bench_with_water("lab-test");
        fill_pipette_from(&mut scene, "beaker-water");
        let err = apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "pipette-1".into(),
                target_item_id: "beaker-h2o".into(),
            },
        )
        .unwrap_err();
        assert_eq!(err, SceneError::InvalidAction);
        assert!((pipette_holding_liquid_ml(item(&scene, "pipette-1")) - 1.0).abs() < 1e-9);
        assert!((water_ml(item(&scene, "beaker-h2o")) - 100.0).abs() < 1e-9);

        apply_action(
            &mut scene,
            Action::PutAway {
                tool_item_id: "pipette-1".into(),
            },
        )
        .unwrap();

        use_tongs(&mut scene, "beaker-water").unwrap();
        let before_water = water_ml(item(&scene, "beaker-water"));
        let err = use_tongs(&mut scene, "beaker-h2o").unwrap_err();
        assert_eq!(err, SceneError::InvalidAction);
        assert!((water_ml(item(&scene, "beaker-h2o")) - 100.0).abs() < 1e-9);
        assert!((water_ml(item(&scene, "beaker-water")) - before_water).abs() < 1e-9);
        assert_eq!(item(&scene, "beaker-water").location, "held");
    }

    #[test]
    fn pipette_and_tongs_reject_impure_put_back_into_beaker_h2o() {
        let mut scene = bench_with_water("lab-test");
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
        fill_pipette_from(&mut scene, "beaker-h2o");
        apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "pipette-1".into(),
                target_item_id: "dish-1".into(),
            },
        )
        .unwrap();

        fill_pipette_from(&mut scene, "beaker-water");
        let err = apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "pipette-1".into(),
                target_item_id: "beaker-h2o".into(),
            },
        )
        .unwrap_err();
        assert_eq!(err, SceneError::InvalidAction);
        assert!(pipette_holding_liquid_ml(item(&scene, "pipette-1")) > AMOUNT_EPS);
        assert!((water_ml(item(&scene, "beaker-h2o")) - 99.0).abs() < 1e-9);

        apply_action(
            &mut scene,
            Action::PutAway {
                tool_item_id: "pipette-1".into(),
            },
        )
        .unwrap();

        use_tongs(&mut scene, "beaker-water").unwrap();
        let err = use_tongs(&mut scene, "beaker-h2o").unwrap_err();
        assert_eq!(err, SceneError::InvalidAction);
        assert!((water_ml(item(&scene, "beaker-h2o")) - 99.0).abs() < 1e-9);
        assert_eq!(item(&scene, "beaker-water").location, "held");
        assert!(aqueous_mol(item(&scene, "beaker-water"), "na+") > AMOUNT_EPS);
    }

    #[test]
    fn tongs_reject_solids_only_dump_into_beaker_h2o() {
        let mut scene = initial_bench_scene("lab-test");
        let water = scene
            .items
            .iter_mut()
            .find(|i| i.id == "beaker-water")
            .unwrap();
        water.properties.composition = vec![solid("sand", 0.4)];
        crate::solubility::sync_fill_ml(water);

        use_tongs(&mut scene, "beaker-water").unwrap();
        let err = use_tongs(&mut scene, "beaker-h2o").unwrap_err();
        assert_eq!(err, SceneError::InvalidAction);
        assert!((solid_g(item(&scene, "beaker-water"), "sand") - 0.4).abs() < 1e-12);
        assert_eq!(solid_g(item(&scene, "beaker-h2o"), "sand"), 0.0);
        assert_eq!(item(&scene, "beaker-water").location, "held");
    }

    #[test]
    fn use_tool_spoon_on_beaker_h2o_is_invalid() {
        let mut scene = initial_bench_scene("lab-test");
        let err = apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "beaker-h2o".into(),
            },
        )
        .unwrap_err();
        assert_eq!(err, SceneError::InvalidAction);
        assert!(item(&scene, "spoon-1").properties.holding.is_empty());
        assert!((water_ml(item(&scene, "beaker-h2o")) - 100.0).abs() < 1e-9);
    }

    #[test]
    fn pour_held_nacl_into_beaker_h2o_is_invalid() {
        let mut scene = initial_bench_scene("lab-test");
        apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "beaker-nacl".into(),
            },
        )
        .unwrap();

        let err = apply_action(
            &mut scene,
            Action::Pour {
                source_item_id: "spoon-1".into(),
                target_item_id: "beaker-h2o".into(),
            },
        )
        .unwrap_err();
        assert_eq!(err, SceneError::InvalidAction);

        let spoon = item(&scene, "spoon-1");
        assert_eq!(spoon.location, "hand");
        assert!((holding_g(spoon, "nacl") - SPOON_SCOOP_MASS_G).abs() < 1e-12);

        let h2o = item(&scene, "beaker-h2o");
        assert!((water_ml(h2o) - DISTILLED_WATER_CAPACITY_ML).abs() < 1e-9);
        assert!(h2o
            .properties
            .composition
            .iter()
            .all(|c| c.substance_id == "water" && c.phase == "liquid"));
        assert_eq!(aqueous_mol(h2o, "na+"), 0.0);
        assert_eq!(solid_g(item(&scene, "beaker-nacl"), "nacl"), 1.8);
    }

    #[test]
    fn use_tool_holding_spoon_on_beaker_h2o_is_invalid() {
        let mut scene = initial_bench_scene("lab-test");
        apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "beaker-nacl".into(),
            },
        )
        .unwrap();
        let before = scene.clone();

        let err = apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "beaker-h2o".into(),
            },
        )
        .unwrap_err();
        assert_eq!(err, SceneError::InvalidAction);
        assert_eq!(scene.items, before.items);
        assert_eq!(scene.temperature_c, before.temperature_c);
        assert_eq!(scene.version, before.version);
    }

    fn set_paper_solids(scene: &mut Scene, solids: Vec<CompositionEntry>) {
        let paper = scene
            .items
            .iter_mut()
            .find(|i| i.id == "filter-paper-1")
            .unwrap();
        paper.properties.composition = solids;
    }

    fn scoop_paper(scene: &mut Scene) -> Result<(), SceneError> {
        apply_action(
            scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "filter-paper-1".into(),
            },
        )
    }

    fn set_slurry_in_water(scene: &mut Scene) {
        fill_main_beaker(scene, FILLED_MAIN_BEAKER_ML);
        apply_action(
            scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "beaker-nacl".into(),
            },
        )
        .unwrap();
        apply_action(
            scene,
            Action::Pour {
                source_item_id: "spoon-1".into(),
                target_item_id: "beaker-water".into(),
            },
        )
        .unwrap();
        apply_action(
            scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "beaker-sand".into(),
            },
        )
        .unwrap();
        apply_action(
            scene,
            Action::Pour {
                source_item_id: "spoon-1".into(),
                target_item_id: "beaker-water".into(),
            },
        )
        .unwrap();
    }

    #[test]
    fn initial_bench_scene_spawns_empty_filtrate_beaker_and_filter_paper() {
        let scene = initial_bench_scene("lab-test");
        assert!(scene.items.iter().any(|i| i.id == "beaker-filtrate"));
        assert!(scene.items.iter().any(|i| i.id == "filter-paper-1"));

        let filtrate = item(&scene, "beaker-filtrate");
        assert_eq!(filtrate.kind, "beaker");
        assert_eq!(filtrate.label, "Filtrate");
        assert_eq!(filtrate.location, "bench");
        assert_eq!(filtrate.properties.volume_ml, Some(250.0));
        assert_eq!(filtrate.properties.fill_ml, Some(0.0));
        assert_eq!(filtrate.properties.transparent, Some(true));
        assert_eq!(filtrate.properties.colourless, Some(true));
        assert_eq!(
            filtrate.properties.temperature_c,
            Some(AMBIENT_TEMPERATURE_C)
        );
        assert!(filtrate.properties.composition.is_empty());

        let paper = item(&scene, "filter-paper-1");
        assert_eq!(paper.kind, "filter_paper");
        assert_eq!(paper.label, "Filter paper");
        assert_eq!(paper.location, "bench");
        assert!(paper.properties.composition.is_empty());
    }

    #[test]
    fn ensure_default_bench_items_restores_filtration_catalog_without_resetting_vessels() {
        let mut scene = initial_bench_scene("lab-test");
        scene
            .items
            .iter_mut()
            .find(|item| item.id == "beaker-water")
            .unwrap()
            .properties
            .fill_ml = Some(150.0);
        scene
            .items
            .retain(|item| item.id != "beaker-filtrate" && item.id != "filter-paper-1");

        ensure_default_bench_items(&mut scene);

        let filtrate = item(&scene, "beaker-filtrate");
        assert_eq!(filtrate.kind, "beaker");
        assert_eq!(filtrate.location, "bench");
        assert_eq!(filtrate.properties.volume_ml, Some(250.0));
        assert_eq!(filtrate.properties.fill_ml, Some(0.0));
        let paper = item(&scene, "filter-paper-1");
        assert_eq!(paper.kind, "filter_paper");
        assert!(paper.properties.composition.is_empty());
        assert_eq!(item(&scene, "beaker-water").properties.fill_ml, Some(150.0));
        assert_eq!(
            scene
                .items
                .iter()
                .filter(|item| item.id == "beaker-filtrate")
                .count(),
            1
        );
        assert_eq!(
            scene
                .items
                .iter()
                .filter(|item| item.id == "filter-paper-1")
                .count(),
            1
        );
    }

    #[test]
    fn tongs_empty_click_unit_picks_up_filtrate_beaker() {
        let mut scene = initial_bench_scene("lab-test");
        use_tongs(&mut scene, "filter-paper-1").unwrap();
        assert_eq!(item(&scene, "beaker-filtrate").location, "held");
        assert_eq!(item(&scene, "filter-paper-1").location, "bench");
        assert_eq!(
            item(&scene, "tongs-1").properties.source_item_id.as_deref(),
            Some("beaker-filtrate")
        );
        put_tongs_away(&mut scene).unwrap();
        assert_eq!(item(&scene, "beaker-filtrate").location, "bench");
        assert_eq!(item(&scene, "tongs-1").location, "bench");
        assert_eq!(item(&scene, "tongs-1").properties.source_item_id, None);

        use_tongs(&mut scene, "beaker-filtrate").unwrap();
        assert_eq!(item(&scene, "beaker-filtrate").location, "held");
        put_tongs_away(&mut scene).unwrap();
        assert_eq!(item(&scene, "beaker-filtrate").location, "bench");
    }

    #[test]
    fn filter_pour_splits_slurry_solids_on_paper_ions_in_filtrate() {
        let mut scene = initial_bench_scene("lab-test");
        set_slurry_in_water(&mut scene);
        let water_before = item(&scene, "beaker-water");
        let source_ml = water_ml(water_before);
        let na_before = aqueous_mol(water_before, "na+");
        let cl_before = aqueous_mol(water_before, "cl-");
        let sand_before = solid_g(water_before, "sand");

        use_tongs(&mut scene, "beaker-water").unwrap();
        use_tongs(&mut scene, "filter-paper-1").unwrap();

        let water = item(&scene, "beaker-water");
        let filtrate = item(&scene, "beaker-filtrate");
        let paper = item(&scene, "filter-paper-1");
        assert_eq!(water.location, "held");
        assert!((water_ml(filtrate) - source_ml).abs() < 1e-9);
        assert!((water_ml(water) - 0.0).abs() < 1e-9);
        assert!((aqueous_mol(filtrate, "na+") - na_before).abs() < 1e-12);
        assert!((aqueous_mol(filtrate, "cl-") - cl_before).abs() < 1e-12);
        assert_eq!(solid_g(filtrate, "sand"), 0.0);
        assert!((solid_g(paper, "sand") - sand_before).abs() < 1e-12);
        assert_eq!(solid_g(water, "sand"), 0.0);
        assert_eq!(aqueous_mol(paper, "na+"), 0.0);
        assert!(scene.last_events.iter().any(|e| e.kind == "poured"));
        assert_eq!(
            item(&scene, "tongs-1").properties.source_item_id.as_deref(),
            Some("beaker-water")
        );
    }

    #[test]
    fn filter_pour_caps_at_250_ml_and_moves_proportional_solids_to_paper() {
        let mut scene = initial_bench_scene("lab-test");
        set_slurry_in_water(&mut scene);
        let dest = scene
            .items
            .iter_mut()
            .find(|i| i.id == "beaker-filtrate")
            .unwrap();
        dest.properties.composition = vec![CompositionEntry {
            substance_id: "water".into(),
            phase: "liquid".into(),
            amount_ml: Some(100.0),
            amount_scoop: None,
            amount_g: None,
            amount_mol: None,
        }];
        dest.properties.temperature_c = Some(40.0);
        crate::solubility::sync_fill_ml(dest);

        let water_before = item(&scene, "beaker-water");
        let source_ml = water_ml(water_before);
        let na_before = aqueous_mol(water_before, "na+");
        let sand_before = solid_g(water_before, "sand");
        let source_t = water_before.properties.temperature_c.unwrap_or(20.0);
        let transferred = 250.0 - 100.0;
        let frac = transferred / source_ml;

        use_tongs(&mut scene, "beaker-water").unwrap();
        use_tongs(&mut scene, "beaker-filtrate").unwrap();

        let water = item(&scene, "beaker-water");
        let filtrate = item(&scene, "beaker-filtrate");
        let paper = item(&scene, "filter-paper-1");
        assert!((water_ml(filtrate) - 250.0).abs() < 1e-9);
        assert!((water_ml(water) - (source_ml - transferred)).abs() < 1e-9);
        assert!((aqueous_mol(filtrate, "na+") - na_before * frac).abs() < 1e-12);
        assert!((solid_g(paper, "sand") - sand_before * frac).abs() < 1e-12);
        assert_eq!(solid_g(filtrate, "sand"), 0.0);
        assert!((solid_g(water, "sand") - sand_before * (1.0 - frac)).abs() < 1e-12);
        let expected_t = (100.0 * 40.0 + transferred * source_t) / 250.0;
        assert!((filtrate.properties.temperature_c.unwrap() - expected_t).abs() < 1e-9);
        assert_eq!(water.location, "held");
    }

    #[test]
    fn filter_pour_rejects_when_filtrate_is_full_and_stays_holding() {
        let mut scene = initial_bench_scene("lab-test");
        set_slurry_in_water(&mut scene);
        let dest = scene
            .items
            .iter_mut()
            .find(|i| i.id == "beaker-filtrate")
            .unwrap();
        dest.properties.composition = vec![CompositionEntry {
            substance_id: "water".into(),
            phase: "liquid".into(),
            amount_ml: Some(250.0),
            amount_scoop: None,
            amount_g: None,
            amount_mol: None,
        }];
        crate::solubility::sync_fill_ml(dest);

        use_tongs(&mut scene, "beaker-water").unwrap();
        let sand_before = solid_g(item(&scene, "beaker-water"), "sand");
        let err = use_tongs(&mut scene, "filter-paper-1").unwrap_err();
        assert_eq!(err, SceneError::InvalidAction);
        assert_eq!(item(&scene, "beaker-water").location, "held");
        assert!((solid_g(item(&scene, "beaker-water"), "sand") - sand_before).abs() < 1e-12);
        assert_eq!(solid_g(item(&scene, "filter-paper-1"), "sand"), 0.0);
        assert!((water_ml(item(&scene, "beaker-filtrate")) - 250.0).abs() < 1e-9);
    }

    #[test]
    fn filter_pour_rejects_solids_only_source() {
        let mut scene = initial_bench_scene("lab-test");
        let water = scene
            .items
            .iter_mut()
            .find(|i| i.id == "beaker-water")
            .unwrap();
        water.properties.composition = vec![solid("sand", 0.4)];
        crate::solubility::sync_fill_ml(water);

        use_tongs(&mut scene, "beaker-water").unwrap();
        let err = use_tongs(&mut scene, "beaker-filtrate").unwrap_err();
        assert_eq!(err, SceneError::InvalidAction);
        assert_eq!(item(&scene, "beaker-water").location, "held");
        assert!((solid_g(item(&scene, "beaker-water"), "sand") - 0.4).abs() < 1e-12);
        assert_eq!(solid_g(item(&scene, "filter-paper-1"), "sand"), 0.0);
    }

    #[test]
    fn filter_pour_empty_source_errors_and_stays_holding() {
        let mut scene = initial_bench_scene("lab-test");
        use_tongs(&mut scene, "dish-1").unwrap();
        let err = use_tongs(&mut scene, "filter-paper-1").unwrap_err();
        assert_eq!(err, SceneError::EmptyHolding);
        assert_eq!(item(&scene, "dish-1").location, "held");
        assert_eq!(
            item(&scene, "tongs-1").properties.source_item_id.as_deref(),
            Some("dish-1")
        );
    }

    #[test]
    fn filter_pour_rejects_when_filtrate_beaker_is_not_home() {
        let mut scene = initial_bench_scene("lab-test");
        use_tongs(&mut scene, "beaker-filtrate").unwrap();
        put_tongs_away(&mut scene).unwrap();
        use_tongs(&mut scene, "beaker-filtrate").unwrap();
        let mut other = initial_bench_scene("lab-test");
        other.items = scene.items.clone();
        other
            .items
            .iter_mut()
            .find(|i| i.id == "beaker-water")
            .unwrap()
            .location = "held".into();
        other
            .items
            .iter_mut()
            .find(|i| i.id == "tongs-1")
            .unwrap()
            .properties
            .source_item_id = Some("beaker-water".into());
        other
            .items
            .iter_mut()
            .find(|i| i.id == "beaker-filtrate")
            .unwrap()
            .location = "held".into();

        let err = use_tongs(&mut other, "filter-paper-1").unwrap_err();
        assert_eq!(err, SceneError::InvalidAction);
        assert_eq!(item(&other, "beaker-filtrate").location, "held");
        assert_eq!(
            item(&other, "tongs-1").properties.source_item_id.as_deref(),
            Some("beaker-water")
        );
    }

    #[test]
    fn tongs_holding_filtrate_pours_into_water_dish_and_h2o() {
        let mut scene = initial_bench_scene("lab-test");
        let filtrate = scene
            .items
            .iter_mut()
            .find(|i| i.id == "beaker-filtrate")
            .unwrap();
        filtrate.properties.composition = vec![CompositionEntry {
            substance_id: "water".into(),
            phase: "liquid".into(),
            amount_ml: Some(30.0),
            amount_scoop: None,
            amount_g: None,
            amount_mol: None,
        }];
        crate::solubility::sync_fill_ml(filtrate);

        use_tongs(&mut scene, "beaker-filtrate").unwrap();
        use_tongs(&mut scene, "dish-1").unwrap();
        assert!((water_ml(item(&scene, "dish-1")) - 25.0).abs() < 1e-9);
        assert!((water_ml(item(&scene, "beaker-filtrate")) - 5.0).abs() < 1e-9);
        assert_eq!(item(&scene, "beaker-filtrate").location, "held");

        use_tongs(&mut scene, "beaker-water").unwrap();
        assert!((water_ml(item(&scene, "beaker-filtrate")) - 0.0).abs() < 1e-9);
        assert!((water_ml(item(&scene, "beaker-water")) - 5.0).abs() < 1e-9);

        put_tongs_away(&mut scene).unwrap();
        let filtrate = scene
            .items
            .iter_mut()
            .find(|i| i.id == "beaker-filtrate")
            .unwrap();
        filtrate.properties.composition = vec![CompositionEntry {
            substance_id: "water".into(),
            phase: "liquid".into(),
            amount_ml: Some(10.0),
            amount_scoop: None,
            amount_g: None,
            amount_mol: None,
        }];
        crate::solubility::sync_fill_ml(filtrate);
        scene
            .items
            .iter_mut()
            .find(|i| i.id == "beaker-h2o")
            .unwrap()
            .properties
            .composition[0]
            .amount_ml = Some(90.0);
        crate::solubility::sync_fill_ml(
            scene
                .items
                .iter_mut()
                .find(|i| i.id == "beaker-h2o")
                .unwrap(),
        );

        use_tongs(&mut scene, "beaker-filtrate").unwrap();
        use_tongs(&mut scene, "beaker-h2o").unwrap();
        assert!((water_ml(item(&scene, "beaker-h2o")) - 100.0).abs() < 1e-9);
        assert!((water_ml(item(&scene, "beaker-filtrate")) - 0.0).abs() < 1e-9);
    }

    #[test]
    fn pipette_extracts_one_ml_from_seated_filtrate() {
        let mut scene = initial_bench_scene("lab-test");
        let filtrate = scene
            .items
            .iter_mut()
            .find(|i| i.id == "beaker-filtrate")
            .unwrap();
        filtrate.properties.composition = vec![CompositionEntry {
            substance_id: "water".into(),
            phase: "liquid".into(),
            amount_ml: Some(10.0),
            amount_scoop: None,
            amount_g: None,
            amount_mol: None,
        }];
        crate::solubility::sync_fill_ml(filtrate);

        fill_pipette_from(&mut scene, "beaker-filtrate");
        assert!((water_ml(item(&scene, "beaker-filtrate")) - 9.0).abs() < 1e-9);
        assert!((pipette_holding_liquid_ml(item(&scene, "pipette-1")) - 1.0).abs() < 1e-9);
        assert_eq!(
            item(&scene, "pipette-1")
                .properties
                .source_item_id
                .as_deref(),
            Some("beaker-filtrate")
        );

        apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "pipette-1".into(),
                target_item_id: "beaker-water".into(),
            },
        )
        .unwrap();
        assert!((water_ml(item(&scene, "beaker-water")) - 1.0).abs() < 1e-9);
        assert!(item(&scene, "pipette-1").properties.holding.is_empty());
    }

    #[test]
    fn pipette_rejects_paper_and_held_filtrate() {
        let mut scene = initial_bench_scene("lab-test");
        let filtrate = scene
            .items
            .iter_mut()
            .find(|i| i.id == "beaker-filtrate")
            .unwrap();
        filtrate.properties.composition = vec![CompositionEntry {
            substance_id: "water".into(),
            phase: "liquid".into(),
            amount_ml: Some(10.0),
            amount_scoop: None,
            amount_g: None,
            amount_mol: None,
        }];
        crate::solubility::sync_fill_ml(filtrate);

        let err = apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "pipette-1".into(),
                target_item_id: "filter-paper-1".into(),
            },
        )
        .unwrap_err();
        assert_eq!(err, SceneError::InvalidAction);

        use_tongs(&mut scene, "beaker-filtrate").unwrap();
        let err = apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "pipette-1".into(),
                target_item_id: "beaker-filtrate".into(),
            },
        )
        .unwrap_err();
        assert_eq!(err, SceneError::InvalidAction);
        assert!((water_ml(item(&scene, "beaker-filtrate")) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn use_tool_spoon_scoops_paper_solids_in_mass_ratio() {
        let mut scene = initial_bench_scene("lab-test");
        set_paper_solids(&mut scene, vec![solid("nacl", 0.6), solid("cacl2", 0.4)]);
        scoop_paper(&mut scene).unwrap();

        let spoon = item(&scene, "spoon-1");
        assert_eq!(spoon.location, "hand");
        assert_eq!(
            spoon.properties.source_item_id.as_deref(),
            Some("filter-paper-1")
        );
        assert!((holding_g(spoon, "nacl") - 0.12).abs() < 1e-12);
        assert!((holding_g(spoon, "cacl2") - 0.08).abs() < 1e-12);
        assert!((solid_g(item(&scene, "filter-paper-1"), "nacl") - 0.48).abs() < 1e-12);
        assert!((solid_g(item(&scene, "filter-paper-1"), "cacl2") - 0.32).abs() < 1e-12);
        assert!(scene.last_events.iter().any(|e| e.kind == "scooped"));
    }

    #[test]
    fn use_tool_spoon_takes_all_when_paper_solids_below_scoop_mass() {
        let mut scene = initial_bench_scene("lab-test");
        set_paper_solids(&mut scene, vec![solid("sand", 0.1)]);
        scoop_paper(&mut scene).unwrap();
        assert!((holding_g(item(&scene, "spoon-1"), "sand") - 0.1).abs() < 1e-12);
        assert_eq!(solid_g(item(&scene, "filter-paper-1"), "sand"), 0.0);
    }

    #[test]
    fn put_away_returns_paper_scoop_to_paper_not_stock() {
        let mut scene = initial_bench_scene("lab-test");
        set_paper_solids(&mut scene, vec![solid("nacl", 0.5)]);
        scoop_paper(&mut scene).unwrap();
        apply_action(
            &mut scene,
            Action::PutAway {
                tool_item_id: "spoon-1".into(),
            },
        )
        .unwrap();

        let spoon = item(&scene, "spoon-1");
        assert!(spoon.properties.holding.is_empty());
        assert_eq!(spoon.location, "bench");
        assert_eq!(spoon.properties.source_item_id, None);
        assert!((solid_g(item(&scene, "filter-paper-1"), "nacl") - 0.5).abs() < 1e-12);
        assert_eq!(solid_g(item(&scene, "beaker-nacl"), "nacl"), 2.0);
        assert!(scene.last_events.iter().any(|e| e.kind == "returned"));
    }

    #[test]
    fn use_tool_returns_single_species_paper_scoop_to_matching_stock() {
        let mut scene = initial_bench_scene("lab-test");
        set_paper_solids(&mut scene, vec![solid("nacl", 0.5)]);
        scoop_paper(&mut scene).unwrap();
        apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "beaker-nacl".into(),
            },
        )
        .unwrap();
        assert!(item(&scene, "spoon-1").properties.holding.is_empty());
        assert!((solid_g(item(&scene, "beaker-nacl"), "nacl") - 2.2).abs() < 1e-12);
        assert!((solid_g(item(&scene, "filter-paper-1"), "nacl") - 0.3).abs() < 1e-12);
    }

    #[test]
    fn use_tool_rejects_empty_paper_and_spoon_on_filtrate() {
        let mut scene = initial_bench_scene("lab-test");
        assert_eq!(
            scoop_paper(&mut scene).unwrap_err(),
            SceneError::InvalidAction
        );

        let filtrate = scene
            .items
            .iter_mut()
            .find(|i| i.id == "beaker-filtrate")
            .unwrap();
        filtrate.properties.composition = vec![solid("nacl", 0.5)];
        let err = apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "beaker-filtrate".into(),
            },
        )
        .unwrap_err();
        assert_eq!(err, SceneError::InvalidAction);
        assert!(item(&scene, "spoon-1").properties.holding.is_empty());
        assert!((solid_g(item(&scene, "beaker-filtrate"), "nacl") - 0.5).abs() < 1e-12);
    }

    #[test]
    fn pour_held_nacl_into_beaker_filtrate_is_invalid() {
        let mut scene = initial_bench_scene("lab-test");
        let filtrate = scene
            .items
            .iter_mut()
            .find(|i| i.id == "beaker-filtrate")
            .unwrap();
        filtrate.properties.composition = vec![CompositionEntry {
            substance_id: "water".into(),
            phase: "liquid".into(),
            amount_ml: Some(10.0),
            amount_scoop: None,
            amount_g: None,
            amount_mol: None,
        }];
        crate::solubility::sync_fill_ml(filtrate);
        apply_action(
            &mut scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "beaker-nacl".into(),
            },
        )
        .unwrap();

        let err = apply_action(
            &mut scene,
            Action::Pour {
                source_item_id: "spoon-1".into(),
                target_item_id: "beaker-filtrate".into(),
            },
        )
        .unwrap_err();
        assert_eq!(err, SceneError::InvalidAction);
        assert!((holding_g(item(&scene, "spoon-1"), "nacl") - SPOON_SCOOP_MASS_G).abs() < 1e-12);
        assert_eq!(aqueous_mol(item(&scene, "beaker-filtrate"), "na+"), 0.0);
        assert_eq!(solid_g(item(&scene, "beaker-nacl"), "nacl"), 1.8);
    }
}
