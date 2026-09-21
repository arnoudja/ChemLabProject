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

    if is_spoon_solids_vessel(&scene.items[target_idx]) {
        return apply_spoon_scoop_from_solids_vessel(scene, tool_idx, target_idx);
    }

    if is_solid_stock_beaker(&scene.items[target_idx]) {
        if composition_has_liquid(&scene.items[target_idx]) {
            return Err(SceneError::InvalidAction);
        }
        return apply_spoon_scoop_from_stock(scene, tool_idx, target_idx);
    }

    Err(SceneError::InvalidAction)
}

fn is_spoon_solids_vessel(item: &SceneItem) -> bool {
    item.id == "beaker-water"
        || is_filtrate_beaker(item)
        || item.kind == "evaporation_dish"
        || item.kind == "filter_paper"
}

fn apply_spoon_scoop_from_stock(
    scene: &mut Scene,
    tool_idx: usize,
    target_idx: usize,
) -> Result<(), SceneError> {
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

    let target_id = scene.items[target_idx].id.clone();
    let tool = &mut scene.items[tool_idx];
    tool.location = "hand".into();
    tool.properties.holding = vec![scoop];
    tool.properties.source_item_id = Some(target_id);

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
    let target = &scene.items[target_idx];
    if is_distilled_water_stock(target) {
        return Err(SceneError::InvalidAction);
    }

    if is_solid_stock_beaker(target) {
        let species = holding_solid_species(&scene.items[tool_idx].properties.holding);
        if species.len() != 1 {
            return Err(SceneError::InvalidAction);
        }
        return apply_return_to_stock(scene, tool_idx, target_idx);
    }

    if target.id == "beaker-water"
        || is_filtrate_beaker(target)
        || target.kind == "evaporation_dish"
        || target.kind == "filter_paper"
    {
        if composition_has_liquid(target) {
            let tool_id = scene.items[tool_idx].id.clone();
            let target_id = scene.items[target_idx].id.clone();
            return apply_pour(scene, &tool_id, &target_id);
        }
        return apply_return_holding_to_solids_vessel(scene, tool_idx, target_idx);
    }

    Err(SceneError::InvalidAction)
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
    tool.properties.source_item_id = Some(source_id.clone());
    let place = match source_kind.as_str() {
        "filter_paper" => "paper",
        "evaporation_dish" => "dish",
        _ if source_id == "beaker-filtrate" => "filtrate beaker",
        _ => "beaker",
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
    let dest_id = scene.items[dest_idx].id.as_str();
    let place = match scene.items[dest_idx].kind.as_str() {
        "filter_paper" => "paper",
        "evaporation_dish" => "dish",
        _ if dest_id == "beaker-filtrate" => "filtrate beaker",
        _ => "beaker",
    };
    scene.last_events.push(SceneEvent {
        kind: "returned".into(),
        message: format!("Returned solids to the {place}."),
    });
    Ok(())
}

/// Return held solid(s) of one species to its matching stock beaker.
///
/// Match the destination by stock beaker id (same as tongs exact-type dump), not by
/// whether a solid composition line is already present — emptied stocks have none.
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
    let Some(expected) = stock_species_for_beaker(&scene.items[target_idx]) else {
        return Err(SceneError::InvalidAction);
    };
    if expected != substance_id {
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
            if is_spoon_solids_vessel(&scene.items[source_idx]) {
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
            stock_species_for_beaker(item).is_some_and(|species| species == substance_id)
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
    let dest_is_filtrate = is_filtrate_beaker(target);
    let dest_is_dish = target.kind == "evaporation_dish";
    if !(dest_is_main_beaker || dest_is_filtrate || dest_is_dish) {
        return Err(SceneError::InvalidAction);
    }
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

fn is_filter_paper(item: &SceneItem) -> bool {
    item.id == "filter-paper-1" || item.kind == "filter_paper"
}

fn is_tongs_vessel(item: &SceneItem) -> bool {
    item.id == "beaker-water"
        || is_distilled_water_stock(item)
        || is_filtrate_beaker(item)
        || item.kind == "evaporation_dish"
}

fn is_tongs_pickup_target(item: &SceneItem) -> bool {
    is_tongs_vessel(item) || is_solid_stock_beaker(item) || is_filter_paper(item)
}

fn vessel_liquid_capacity_ml(item: &SceneItem) -> Option<f64> {
    if is_filtrate_beaker(item) {
        return Some(item.properties.volume_ml.unwrap_or(FILTRATE_CAPACITY_ML));
    }
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

fn composition_is_only_solid_species(item: &SceneItem, species: &str) -> bool {
    let mut saw = false;
    for entry in &item.properties.composition {
        if !entry_has_amount(entry) {
            continue;
        }
        if entry.phase == "solid" && entry.substance_id == species {
            saw = true;
            continue;
        }
        return false;
    }
    saw
}

fn stock_species_for_beaker(item: &SceneItem) -> Option<&'static str> {
    match item.id.as_str() {
        "beaker-nacl" => Some("nacl"),
        "beaker-cacl2" => Some("cacl2"),
        "beaker-sand" => Some("sand"),
        _ => None,
    }
}

fn apply_tongs_use(
    scene: &mut Scene,
    tool_idx: usize,
    target_idx: usize,
) -> Result<(), SceneError> {
    if !is_tongs_pickup_target(&scene.items[target_idx]) {
        return Err(SceneError::InvalidAction);
    }
    match scene.items[tool_idx].properties.source_item_id.clone() {
        None => apply_tongs_pick_up(scene, tool_idx, target_idx),
        Some(held_id) if held_id == scene.items[target_idx].id => Err(SceneError::InvalidAction),
        Some(held_id) if is_filter_paper(&scene.items[target_idx]) => {
            apply_tongs_onto_filter_paper(scene, tool_idx, &held_id)
        }
        Some(_) => apply_tongs_pour(scene, tool_idx, target_idx),
    }
}

fn apply_tongs_onto_filter_paper(
    scene: &mut Scene,
    tool_idx: usize,
    held_id: &str,
) -> Result<(), SceneError> {
    if held_id == "beaker-filtrate" {
        return Err(SceneError::InvalidAction);
    }
    let source_idx = find_item_index(scene, held_id)?;
    let source_liquid = crate::solubility::liquid_water_ml(&scene.items[source_idx]);
    let has_solids = composition_has_solids(&scene.items[source_idx]);
    if source_liquid > AMOUNT_EPS {
        return apply_filter_pour(scene, tool_idx);
    }
    if has_solids {
        let paper_idx = find_item_index(scene, "filter-paper-1")?;
        return dump_all_solids(scene, source_idx, paper_idx);
    }
    let filtrate_idx = find_item_index(scene, "beaker-filtrate")?;
    if scene.items[filtrate_idx].location != "bench" {
        return Err(SceneError::InvalidAction);
    }
    Err(SceneError::EmptyHolding)
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

    let dest = &scene.items[dest_idx];
    if is_filter_paper(dest) {
        return Err(SceneError::InvalidAction);
    }
    if is_solid_stock_beaker(dest) {
        let Some(species) = stock_species_for_beaker(dest) else {
            return Err(SceneError::InvalidAction);
        };
        if !composition_is_only_solid_species(&scene.items[source_idx], species) {
            return Err(SceneError::InvalidAction);
        }
        return dump_all_solids(scene, source_idx, dest_idx);
    }
    if !is_tongs_vessel(dest) {
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

fn liquid_water_ml_in(entries: &[CompositionEntry]) -> f64 {
    entries
        .iter()
        .filter(|c| c.substance_id == "water" && c.phase == "liquid")
        .map(|c| c.amount_ml.unwrap_or(0.0))
        .sum()
}

fn blend_temperature_with_added_water(target: &mut SceneItem, add_ml: f64, source_t: f64) {
    let v0 = crate::solubility::liquid_water_ml(target);
    let t0 = target
        .properties
        .temperature_c
        .unwrap_or(AMBIENT_TEMPERATURE_C);
    if v0 <= AMOUNT_EPS {
        target.properties.temperature_c = Some(source_t);
    } else {
        target.properties.temperature_c = Some((v0 * t0 + add_ml * source_t) / (v0 + add_ml));
    }
}

fn merge_composition_into(
    target: &mut SceneItem,
    entries: &[CompositionEntry],
    include_solids: bool,
) {
    for entry in entries {
        if entry.substance_id == "water" && entry.phase == "liquid" {
            add_or_increase_water(target, entry.amount_ml.unwrap_or(0.0));
        } else if entry.phase == "aqueous" {
            add_or_increase_mol(
                target,
                &entry.substance_id,
                "aqueous",
                entry.amount_mol.unwrap_or(0.0),
            );
        } else if include_solids && entry.phase == "solid" {
            add_or_increase_solid(
                target,
                &entry.substance_id,
                entry.amount_scoop,
                entry.amount_g,
            );
        }
    }
}

fn mix_transfer_into(
    target: &mut SceneItem,
    transferred: &[CompositionEntry],
    source_t: Option<f64>,
) {
    let add_ml = liquid_water_ml_in(transferred);
    if let Some(source_t) = source_t {
        if add_ml > AMOUNT_EPS {
            blend_temperature_with_added_water(target, add_ml, source_t);
        }
    }
    merge_composition_into(target, transferred, true);
    crate::solubility::sync_fill_ml(target);
}

fn mix_aliquot_into(target: &mut SceneItem, aliquot: &[CompositionEntry], aliquot_t: f64) {
    let add_ml = liquid_water_ml_in(aliquot);
    // Pipette always blends with aliquot T (even when dest is empty / add_ml is tiny).
    blend_temperature_with_added_water(target, add_ml, aliquot_t);
    merge_composition_into(target, aliquot, false);
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
#[path = "scene_tests/mod.rs"]
mod tests;
