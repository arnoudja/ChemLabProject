//! Lab scene engine: scoop with tools, pour into vessels, dissolve as a pour consequence.

use thiserror::Error;

use crate::challenges::{find_challenge, is_free_mode, Challenge, FREE_MODE};
use crate::dissolve::{dissolve, DissolveError};

/// Mass of one spoon scoop of solid, in grams.
pub const SPOON_SCOOP_MASS_G: f64 = 0.2;

/// Molar mass of NaCl used when converting scoop mass to aqueous ion moles.
pub(crate) const NACL_MOLAR_MASS_G_PER_MOL: f64 = 58.44;

/// Molar mass of NaOH (g/mol).
pub(crate) const NAOH_MOLAR_MASS_G_PER_MOL: f64 = 40.00;

/// Molar mass of anhydrous CaCl₂ (g/mol).
pub(crate) const CACL2_MOLAR_MASS_G_PER_MOL: f64 = 110.98;

/// Molar mass of anhydrous Na₂SO₄ (g/mol).
pub(crate) const NA2SO4_MOLAR_MASS_G_PER_MOL: f64 = 142.04;

/// Molar mass of anhydrous CaSO₄ (g/mol).
pub(crate) const CASO4_MOLAR_MASS_G_PER_MOL: f64 = 136.14;

/// Specific heat of liquid H₂SO₄ (order-of-magnitude), J/(g·K).
pub const CP_H2SO4_LIQUID: f64 = 1.4;

/// Pipette aliquot volume (ml).
pub const PIPETTE_VOLUME_ML: f64 = 1.00;

/// Liquid a vessel must hold before the pipette may draw from it (ml).
pub const PIPETTE_MIN_SOURCE_ML: f64 = 3.00;

/// Evaporation dish capacity (ml).
pub const DISH_CAPACITY_ML: f64 = 25.00;

/// Water beaker liquid capacity (ml).
pub const WATER_CAPACITY_ML: f64 = 250.00;

/// Wire id of the main reaction beaker (not the distilled-water stock).
pub const MAIN_BEAKER_ID: &str = "beaker-water";

/// Wire id of the distilled-water stock beaker.
pub const DISTILLED_WATER_ID: &str = "beaker-h2o";

/// Distilled-water stock beaker liquid capacity (ml).
pub const DISTILLED_WATER_CAPACITY_ML: f64 = 100.00;

/// Filtrate beaker liquid capacity (ml).
pub const FILTRATE_CAPACITY_ML: f64 = 250.00;

/// Ambient bench / reset temperature (°C).
pub const AMBIENT_TEMPERATURE_C: f64 = 20.0;

/// Lab air pressure (bar). Pure-water boil ≈ 100 °C at this pressure.
pub const ATM_PRESSURE_BAR: f64 = 1.00;

/// Fixed lab relative humidity (fraction).
pub const RELATIVE_HUMIDITY: f64 = 0.50;

/// Latent heat of vaporization of water (J/g ≈ J/ml).
pub const WATER_LATENT_HEAT_J_PER_G: f64 = 2257.0;

/// Molar mass of water (g/mol); 1 ml liquid water ≈ 1 g.
pub const WATER_MOLAR_MASS_G_PER_MOL: f64 = 18.01528;

/// Antoine A for water (`log10(P_sat / bar) = A − B / (T_C + C)`), ~1–100 °C.
pub const WATER_ANTOINE_A: f64 = 5.1962;

/// Antoine B for water (°C).
pub const WATER_ANTOINE_B: f64 = 1730.63;

/// Antoine C for water (°C).
pub const WATER_ANTOINE_C: f64 = 233.426;

/// Evaporating-dish free surface area (m²), ~70 mm diameter.
pub const DISH_EVAP_AREA_M2: f64 = 0.004;

/// Mass-transfer coefficient (ml/(s·m²)). With [`DISH_EVAP_AREA_M2`], pure water at
/// 20 °C / 50% RH loses ≈ 2 ml/h (`m_dot = k A max(0, p_w − p_air) / P_atm`).
pub const DISH_MASS_TRANSFER_COEFF_ML_PER_S_M2: f64 = 11.93;

/// Pure-water boiling temperature at [`ATM_PRESSURE_BAR`] (°C), from Antoine.
/// Prefer [`boiling_temperature_c`] for composition-aware boil.
pub const BOILING_TEMPERATURE_C: f64 = 99.63;

/// Burner heat power delivered to the evaporation dish (W).
pub const BURNER_POWER_W: f64 = 1100.0;

/// Glass beaker body heat capacity (J/K), including filtrate / distilled-water stocks.
pub const C_BEAKER: f64 = 150.0;

/// Evaporation dish body heat capacity (J/K).
pub const C_DISH: f64 = 80.0;

/// Overall heat-transfer coefficient × area for the dish (W/K).
pub const UA_DISH: f64 = 4.0;

/// Overall heat-transfer coefficient × area for beakers (W/K).
pub const UA_BEAKER: f64 = 3.0;

/// Specific heat of solid NaCl, J/(g·K).
pub const CP_NACL: f64 = 0.88;

/// Specific heat of solid CaCl₂, J/(g·K).
pub const CP_CACL2: f64 = 0.67;

/// Specific heat of solid sand (SiO₂), J/(g·K).
pub const CP_SAND: f64 = 0.74;

/// Specific heat of solid NaOH (crystalline, order-of-magnitude), J/(g·K).
pub const CP_NAOH: f64 = 1.49;

/// Snap item temperature to ambient when closer than this (°C).
/// Kept below a typical NaCl scoop ΔT (~0.014 °C with vessel C) so dissolve
/// cooling is not erased on the next clock tick.
const TEMPERATURE_SNAP_EPS_C: f64 = 0.005;

pub(crate) const AMOUNT_EPS: f64 = 1e-12;

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

/// Enthalpy of neutralization H⁺ + OH⁻ → H₂O (exothermic), J/mol.
pub const H_OH_NEUTRALIZATION_J_PER_MOL: f64 = -55800.0;

/// Specific heat capacity of liquid water, J/(g·K). Mass of water ≈ volume in ml.
pub const WATER_SPECIFIC_HEAT_J_PER_G_K: f64 = 4.184;

fn is_stock_solid(substance_id: &str) -> bool {
    matches!(substance_id, "nacl" | "cacl2" | "sand" | "naoh" | "na2so4")
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
    /// [`FREE_MODE`] or a challenge id from [`crate::challenges`].
    pub mode: String,
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
    /// Replace the scene with a fresh start scene for the scene's current mode
    /// (same `lab_id` / `version`).
    Reset,
    /// Idle click on the burner. Stays off when the dish has no liquid water.
    ToggleBurner { burner_item_id: String },
    /// Switch to Free mode or a challenge; always a hard reset into that mode's start scene.
    SelectMode { mode: String },
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
    #[error("No fluid available.")]
    NoFluidAvailable,
    #[error("Not enough fluid available.")]
    NotEnoughFluidAvailable,
    #[error("unknown mode")]
    UnknownMode,
    #[error(transparent)]
    Dissolve(#[from] DissolveError),
}

fn solid_stock_beaker(
    id: impl Into<String>,
    label: impl Into<String>,
    substance_id: impl Into<String>,
) -> SceneItem {
    SceneItem {
        id: id.into(),
        kind: "beaker".into(),
        label: label.into(),
        location: "bench".into(),
        properties: ItemProperties {
            volume_ml: Some(250.0),
            fill_ml: Some(100.0),
            transparent: Some(true),
            colourless: Some(true),
            temperature_c: Some(20.0),
            composition: vec![CompositionEntry {
                substance_id: substance_id.into(),
                phase: "solid".into(),
                amount_ml: None,
                amount_scoop: Some(10),
                amount_g: Some(10.0 * SPOON_SCOOP_MASS_G),
                amount_mol: None,
            }],
            holding: Vec::new(),
            ..ItemProperties::default()
        },
    }
}

fn distilled_water_stock_beaker() -> SceneItem {
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
    }
}

fn hcl_stock_beaker() -> SceneItem {
    SceneItem {
        id: "beaker-hcl".into(),
        kind: "beaker".into(),
        label: "Hydrochloric acid (30%)".into(),
        location: "bench".into(),
        properties: ItemProperties {
            volume_ml: Some(crate::hcl::HCL_STOCK_CAPACITY_ML),
            fill_ml: Some(crate::hcl::HCL_STOCK_CAPACITY_ML),
            transparent: Some(true),
            colourless: Some(true),
            temperature_c: Some(20.0),
            composition: vec![
                CompositionEntry {
                    substance_id: "water".into(),
                    phase: "liquid".into(),
                    amount_ml: Some(crate::hcl::HCL_STOCK_WATER_MASS_G),
                    amount_scoop: None,
                    amount_g: None,
                    amount_mol: None,
                },
                CompositionEntry {
                    substance_id: "h+".into(),
                    phase: "aqueous".into(),
                    amount_ml: None,
                    amount_scoop: None,
                    amount_g: None,
                    amount_mol: Some(crate::hcl::HCL_STOCK_HCL_MOLES),
                },
                CompositionEntry {
                    substance_id: "cl-".into(),
                    phase: "aqueous".into(),
                    amount_ml: None,
                    amount_scoop: None,
                    amount_g: None,
                    amount_mol: Some(crate::hcl::HCL_STOCK_HCL_MOLES),
                },
            ],
            holding: Vec::new(),
            ..ItemProperties::default()
        },
    }
}

fn h2so4_stock_beaker() -> SceneItem {
    SceneItem {
        id: "beaker-h2so4".into(),
        kind: "beaker".into(),
        label: "Sulfuric acid".into(),
        location: "bench".into(),
        properties: ItemProperties {
            volume_ml: Some(crate::h2so4::H2SO4_STOCK_CAPACITY_ML),
            fill_ml: Some(crate::h2so4::H2SO4_STOCK_CAPACITY_ML),
            transparent: Some(true),
            colourless: Some(true),
            temperature_c: Some(20.0),
            composition: crate::h2so4::stock_h2so4_composition(),
            holding: Vec::new(),
            ..ItemProperties::default()
        },
    }
}

fn empty_bench_beaker(
    id: impl Into<String>,
    label: impl Into<String>,
    volume_ml: f64,
    temperature_c: f64,
) -> SceneItem {
    SceneItem {
        id: id.into(),
        kind: "beaker".into(),
        label: label.into(),
        location: "bench".into(),
        properties: ItemProperties {
            volume_ml: Some(volume_ml),
            fill_ml: Some(0.0),
            transparent: Some(true),
            colourless: Some(true),
            temperature_c: Some(temperature_c),
            ..ItemProperties::default()
        },
    }
}

fn empty_evaporation_dish() -> SceneItem {
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
    }
}

/// Build the Free-mode bench scene for a lab.
pub fn initial_bench_scene(lab_id: impl Into<String>) -> Scene {
    Scene {
        lab_id: lab_id.into(),
        version: 0,
        temperature_c: 20.0,
        last_events: Vec::new(),
        last_applied_unix_ms: None,
        mode: FREE_MODE.into(),
        items: vec![
            SceneItem {
                id: "spoon-1".into(),
                kind: "spoon".into(),
                label: "Spoon".into(),
                location: "bench".into(),
                properties: ItemProperties::default(),
            },
            distilled_water_stock_beaker(),
            hcl_stock_beaker(),
            h2so4_stock_beaker(),
            solid_stock_beaker("beaker-nacl", "Sodium chloride", "nacl"),
            solid_stock_beaker("beaker-naoh", "Sodium hydroxide", "naoh"),
            solid_stock_beaker("beaker-na2so4", "Sodium sulfate", "na2so4"),
            solid_stock_beaker("beaker-cacl2", "Calcium chloride", "cacl2"),
            solid_stock_beaker("beaker-sand", "Sand", "sand"),
            empty_bench_beaker("beaker-water", "Beaker", WATER_CAPACITY_ML, 20.0),
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
            empty_bench_beaker(
                "beaker-filtrate",
                "Filtrate",
                FILTRATE_CAPACITY_ML,
                AMBIENT_TEMPERATURE_C,
            ),
            SceneItem {
                id: "filter-paper-1".into(),
                kind: "filter_paper".into(),
                label: "Filter paper".into(),
                location: "bench".into(),
                properties: ItemProperties::default(),
            },
            empty_evaporation_dish(),
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

/// Build the start scene of `mode`: Free, or a challenge from the catalog.
///
/// Returns `None` for an unknown challenge id.
pub fn initial_scene_for_mode(lab_id: impl Into<String>, mode: &str) -> Option<Scene> {
    let lab_id = lab_id.into();
    if is_free_mode(mode) {
        return Some(initial_bench_scene(lab_id));
    }
    let challenge = find_challenge(mode)?;
    Some(challenge_scene(lab_id, challenge))
}

/// The Free bench with the challenge's edits: allowed stocks only, listed stocks
/// emptied, main beaker preloaded with the challenge's dry solids, optional
/// distilled-water start volume.
fn challenge_scene(lab_id: String, challenge: &Challenge) -> Scene {
    let mut scene = initial_bench_scene(lab_id);
    scene.mode = challenge.id.into();
    scene.items.retain(|item| {
        !is_ingredient_stock(item) || challenge.allowed_stock_item_ids.contains(&item.id.as_str())
    });
    for stock_id in challenge.empty_stock_item_ids {
        if let Some(stock) = scene.items.iter_mut().find(|item| item.id == *stock_id) {
            empty_stock_solids(stock);
        }
    }
    if let Some(ml) = challenge.distilled_water_ml {
        if let Some(h2o) = scene.items.iter_mut().find(|item| item.id == "beaker-h2o") {
            set_distilled_water_amount(h2o, ml);
        }
    }
    if let Some(beaker) = scene
        .items
        .iter_mut()
        .find(|item| item.id == "beaker-water")
    {
        for (substance_id, grams) in challenge.main_beaker_solids {
            add_or_increase_solid(beaker, substance_id, None, Some(*grams));
        }
    }
    scene
}

/// Set the distilled-water stock's liquid fill (capacity stays Free-mode size).
fn set_distilled_water_amount(h2o: &mut SceneItem, ml: f64) {
    h2o.properties.fill_ml = Some(ml);
    for entry in &mut h2o.properties.composition {
        if entry.substance_id == "water" && entry.phase == "liquid" {
            entry.amount_ml = Some(ml);
        }
    }
}

/// Zero a stock beaker's own solid without dropping the composition line, so the
/// UI still renders an (empty) stock of that species.
fn empty_stock_solids(stock: &mut SceneItem) {
    for entry in &mut stock.properties.composition {
        if entry.phase != "solid" {
            continue;
        }
        entry.amount_scoop = Some(0);
        entry.amount_g = Some(0.0);
        entry.amount_mol = None;
    }
}

/// Insert any start-scene items missing from a persisted scene.
///
/// Labs saved before a catalog addition (e.g. `tongs-1`) keep their vessel state;
/// only absent ids are filled, from the start scene of the lab's own mode — a
/// challenge must not gain back a stock its layout removed.
pub fn ensure_default_bench_items(scene: &mut Scene) {
    let defaults = initial_scene_for_mode(scene.lab_id.clone(), &scene.mode)
        .unwrap_or_else(|| initial_bench_scene(scene.lab_id.clone()));
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
        Action::SelectMode { mode } => apply_select_mode(scene, &mode),
    }
}

/// Rebuild the start scene of the mode the lab is already in.
fn apply_reset(scene: &mut Scene) {
    let lab_id = scene.lab_id.clone();
    let start = initial_scene_for_mode(lab_id.clone(), &scene.mode)
        .unwrap_or_else(|| initial_bench_scene(lab_id));
    restart_into(scene, start);
    scene.last_events.push(SceneEvent {
        kind: "reset".into(),
        message: "Lab reset to the starting bench.".into(),
    });
}

/// Switch modes. Always a hard reset into the new mode's start scene.
fn apply_select_mode(scene: &mut Scene, mode: &str) -> Result<(), SceneError> {
    let start =
        initial_scene_for_mode(scene.lab_id.clone(), mode).ok_or(SceneError::UnknownMode)?;
    restart_into(scene, start);
    let message = match find_challenge(mode) {
        Some(challenge) => format!("Started challenge: {}.", challenge.title),
        None => "Switched to Free mode.".into(),
    };
    scene.last_events.push(SceneEvent {
        kind: "mode_selected".into(),
        message,
    });
    Ok(())
}

/// Replace the scene with `start`, keeping the lab's version and clock watermark.
fn restart_into(scene: &mut Scene, start: Scene) {
    let version = scene.version;
    let last_applied_unix_ms = scene.last_applied_unix_ms;
    *scene = start;
    scene.version = version;
    scene.last_applied_unix_ms = last_applied_unix_ms;
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
    let source_t = scene.items[target_idx]
        .properties
        .temperature_c
        .unwrap_or(scene.temperature_c);
    let tool = &mut scene.items[tool_idx];
    tool.location = "hand".into();
    tool.properties.holding = vec![scoop];
    tool.properties.source_item_id = Some(target_id);
    tool.properties.temperature_c = Some(source_t);

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
    if is_distilled_water_stock(target) || is_hcl_stock(target) || is_h2so4_stock(target) {
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

pub(crate) fn solid_amount_g(entry: &CompositionEntry) -> f64 {
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
    let source_t = scene.items[target_idx]
        .properties
        .temperature_c
        .unwrap_or(scene.temperature_c);
    let taken = take_solids_by_mass(&mut scene.items[target_idx], take_g);
    if taken.is_empty() {
        return Err(SceneError::InvalidAction);
    }
    let tool = &mut scene.items[tool_idx];
    tool.location = "hand".into();
    tool.properties.holding = taken;
    tool.properties.source_item_id = Some(source_id.clone());
    tool.properties.temperature_c = Some(source_t);
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
    scene.items[tool_idx].properties.temperature_c = None;
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
    scene.items[tool_idx].properties.temperature_c = None;
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

    scene.items[tool_idx].properties.temperature_c = None;
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
        || is_hcl_stock(&scene.items[target_idx])
        || is_h2so4_stock(&scene.items[target_idx])
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
    let spoon_t = scene.items[source_idx]
        .properties
        .temperature_c
        .unwrap_or(scene.temperature_c);
    scene.items[source_idx].properties.temperature_c = None;

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
        for held in &held_all {
            blend_temperature_capacity(
                &mut scene.items[target_idx],
                heat_capacity_of_entries(std::slice::from_ref(held)),
                spoon_t,
            );
            let scoops = held.amount_scoop.or(Some(1));
            let mass_g = held.amount_g.or(Some(SPOON_SCOOP_MASS_G));
            add_or_increase_solid(
                &mut scene.items[target_idx],
                &held.substance_id,
                scoops,
                mass_g,
            );
        }
        finalize_aqueous_vessel(
            &mut scene.items[target_idx],
            crate::dissolve_kinetics::POUR_CONTACT_TAU_S,
        );
        return Ok(());
    }

    for held in held_all {
        blend_temperature_capacity(
            &mut scene.items[target_idx],
            heat_capacity_of_entries(std::slice::from_ref(&held)),
            spoon_t,
        );
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

    finalize_aqueous_vessel(
        &mut scene.items[target_idx],
        crate::dissolve_kinetics::POUR_CONTACT_TAU_S,
    );
    Ok(())
}

fn mix_held_solid_into_water(
    target: &mut SceneItem,
    held: &CompositionEntry,
    _temperature_c: f64,
    _dissolved: bool,
) {
    // Always deposit solid; kinetic dissolve in finalize authors ions over τ.
    // Qualitative `dissolved` only drives the scene event, not ion authorship.
    let scoops = held.amount_scoop.or(Some(1));
    let mass_g = held.amount_g.or(Some(SPOON_SCOOP_MASS_G));
    add_or_increase_solid(target, &held.substance_id, scoops, mass_g);
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
pub(crate) fn finalize_aqueous_vessel(item: &mut SceneItem, dissolve_tau_s: f64) {
    crate::h2so4::ionize_liquid_h2so4_in_water(item);
    crate::dissolve_kinetics::apply_kinetic_dissolve(item, dissolve_tau_s);
    crate::acid_base::speciate_aqueous_acid_base(item, true);
    crate::solubility::enforce_saturation(item);
    // SI can consume SO₄²⁻ (and collapsed HSO₄⁻); re-equilibrate without re-heating.
    crate::acid_base::speciate_aqueous_acid_base(item, false);
    crate::h2so4::reform_aqueous_h2so4_to_liquid(item);
    // Reform removes acid protons; restore K_w / K_a2 if solvent remains.
    crate::acid_base::speciate_aqueous_acid_base(item, false);
    crate::solubility::sync_fill_ml(item);
}

/// Apply dissolution heat to the solvent vessel: ΔT = −(n·ΔH_sol) / C_eff.
///
/// `C_eff` is [`effective_heat_capacity`] of the target (vessel + water + solids).
/// Endothermic ΔH cools; exothermic heats.
pub(crate) fn apply_dissolution_temperature_change(
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
    let c_eff = effective_heat_capacity(target);
    if c_eff <= AMOUNT_EPS {
        return;
    }
    let heat_j = moles * delta_h_j_per_mol;
    let delta_t = -heat_j / c_eff;
    target.properties.temperature_c = Some(current_temperature_c + delta_t);
}

fn add_or_increase_mol(target: &mut SceneItem, substance_id: &str, phase: &str, moles: f64) {
    add_or_increase_mol_in(
        &mut target.properties.composition,
        substance_id,
        phase,
        moles,
    );
}

fn add_or_increase_mol_in(
    composition: &mut Vec<CompositionEntry>,
    substance_id: &str,
    phase: &str,
    moles: f64,
) {
    if let Some(existing) = composition
        .iter_mut()
        .find(|c| c.substance_id == substance_id && c.phase == phase)
    {
        existing.amount_mol = Some(existing.amount_mol.unwrap_or(0.0) + moles);
        return;
    }
    composition.push(CompositionEntry {
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

fn is_hcl_stock(item: &SceneItem) -> bool {
    item.id == "beaker-hcl"
}

fn is_h2so4_stock(item: &SceneItem) -> bool {
    item.id == "beaker-h2so4"
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
        || is_hcl_stock(item)
        || is_h2so4_stock(item)
        || is_filtrate_beaker(item)
        || item.kind == "evaporation_dish"
}

fn pipette_holding_liquid_ml(pipette: &SceneItem) -> f64 {
    crate::hcl::transfer_volume_ml_of_entries(&pipette.properties.holding)
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
    // Toggle-on requires liquid water; liquid H₂SO₄ alone is not enough heat fuel.
    let has_water = crate::solubility::dish_has_liquid_water(&scene.items[dish_idx]);
    let currently_on = scene.items[burner_idx].properties.on.unwrap_or(false);
    let next_on = if currently_on { false } else { has_water };
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

#[path = "thermal.rs"]
mod thermal;

pub use thermal::{
    apply_elapsed, boiling_temperature_c, effective_heat_capacity, water_mole_fraction,
    water_vapor_pressure_bar,
};

use thermal::blend_temperature_capacity;
pub(crate) use thermal::heat_capacity_of_entries;

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
    let available_ml = crate::hcl::transfer_volume_ml(&scene.items[target_idx]);
    if available_ml <= AMOUNT_EPS {
        return Err(SceneError::NoFluidAvailable);
    }
    if available_ml + AMOUNT_EPS < PIPETTE_MIN_SOURCE_ML {
        return Err(SceneError::NotEnoughFluidAvailable);
    }
    let source_id = scene.items[target_idx].id.clone();
    let source_t = scene.items[target_idx]
        .properties
        .temperature_c
        .unwrap_or(scene.temperature_c);
    let aliquot = take_liquid_aliquot(&mut scene.items[target_idx], PIPETTE_VOLUME_ML)?;
    // Concentration may precipitate; no dissolve contact on the source draw.
    finalize_aqueous_vessel(&mut scene.items[target_idx], 0.0);

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
        let current = crate::hcl::transfer_volume_ml(&scene.items[target_idx]);
        if current + PIPETTE_VOLUME_ML > DISH_CAPACITY_ML + AMOUNT_EPS {
            return Err(SceneError::InvalidAction);
        }
    }
    if is_distilled_water_stock(&scene.items[target_idx]) {
        if !composition_is_pure_h2o(&scene.items[tool_idx].properties.holding) {
            return Err(SceneError::InvalidAction);
        }
        let current = crate::hcl::transfer_volume_ml(&scene.items[target_idx]);
        let cap = scene.items[target_idx]
            .properties
            .volume_ml
            .unwrap_or(DISTILLED_WATER_CAPACITY_ML);
        if current + PIPETTE_VOLUME_ML > cap + AMOUNT_EPS {
            return Err(SceneError::InvalidAction);
        }
    }
    if is_hcl_stock(&scene.items[target_idx]) {
        if !crate::hcl::composition_is_stock_hcl(&scene.items[tool_idx].properties.holding) {
            return Err(SceneError::InvalidAction);
        }
        let current = crate::hcl::transfer_volume_ml(&scene.items[target_idx]);
        let cap = scene.items[target_idx]
            .properties
            .volume_ml
            .unwrap_or(crate::hcl::HCL_STOCK_CAPACITY_ML);
        if current + PIPETTE_VOLUME_ML > cap + AMOUNT_EPS {
            return Err(SceneError::InvalidAction);
        }
    }
    if is_h2so4_stock(&scene.items[target_idx]) {
        if !crate::h2so4::composition_is_stock_h2so4(&scene.items[tool_idx].properties.holding) {
            return Err(SceneError::InvalidAction);
        }
        let current = crate::hcl::transfer_volume_ml(&scene.items[target_idx]);
        let cap = scene.items[target_idx]
            .properties
            .volume_ml
            .unwrap_or(crate::h2so4::H2SO4_STOCK_CAPACITY_ML);
        if current + PIPETTE_VOLUME_ML > cap + AMOUNT_EPS {
            return Err(SceneError::InvalidAction);
        }
    }
    if is_filtrate_beaker(&scene.items[target_idx]) {
        if scene.items[target_idx].location != "bench" {
            return Err(SceneError::InvalidAction);
        }
        let current = crate::hcl::transfer_volume_ml(&scene.items[target_idx]);
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
    finalize_aqueous_vessel(
        &mut scene.items[target_idx],
        crate::dissolve_kinetics::POUR_CONTACT_TAU_S,
    );

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
        "beaker-nacl" | "beaker-cacl2" | "beaker-sand" | "beaker-naoh" | "beaker-na2so4"
    )
}

/// Any ingredient stock on the Free bench, so a challenge layout that does not list
/// a stock drops it — including stocks added to the bench later.
fn is_ingredient_stock(item: &SceneItem) -> bool {
    is_distilled_water_stock(item)
        || is_hcl_stock(item)
        || is_h2so4_stock(item)
        || is_solid_stock_beaker(item)
}

fn is_filter_paper(item: &SceneItem) -> bool {
    item.id == "filter-paper-1" || item.kind == "filter_paper"
}

fn is_tongs_vessel(item: &SceneItem) -> bool {
    item.id == "beaker-water"
        || is_distilled_water_stock(item)
        || is_hcl_stock(item)
        || is_h2so4_stock(item)
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
        "beaker-naoh" => Some("naoh"),
        "beaker-na2so4" => Some("na2so4"),
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
    let source_liquid = crate::hcl::transfer_volume_ml(&scene.items[source_idx]);
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

/// Take a Φ_V-limited liquid fraction from `source` toward a vessel with `dest_cap`.
///
/// Callers own empty-source / stock-guard / solids-only branches. Filter wash stays
/// outside; this only computes room, `take_composition_fraction`, and source T.
fn transfer_liquid_fraction(
    scene: &mut Scene,
    source_idx: usize,
    dest_idx: usize,
    dest_cap: f64,
) -> Result<(Vec<CompositionEntry>, f64), SceneError> {
    let source_liquid = crate::hcl::transfer_volume_ml(&scene.items[source_idx]);
    let dest_liquid = crate::hcl::transfer_volume_ml(&scene.items[dest_idx]);
    let dest_room = (dest_cap - dest_liquid).max(0.0);
    if source_liquid <= AMOUNT_EPS || dest_room <= AMOUNT_EPS {
        return Err(SceneError::InvalidAction);
    }
    let transferred = source_liquid.min(dest_room);
    let frac = transferred / source_liquid;
    let source_t = scene.items[source_idx]
        .properties
        .temperature_c
        .unwrap_or(scene.temperature_c);
    let taken = take_composition_fraction(&mut scene.items[source_idx], frac);
    Ok((taken, source_t))
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

    let source_liquid = crate::hcl::transfer_volume_ml(&scene.items[source_idx]);
    let has_solids = composition_has_solids(&scene.items[source_idx]);

    if source_liquid <= AMOUNT_EPS {
        if has_solids {
            return Err(SceneError::InvalidAction);
        }
        return Err(SceneError::EmptyHolding);
    }

    let (taken, source_t) =
        transfer_liquid_fraction(scene, source_idx, dest_idx, FILTRATE_CAPACITY_ML)?;
    let mut fluid = Vec::new();
    let mut solids = Vec::new();
    for entry in taken {
        if entry.phase == "solid" {
            solids.push(entry);
        } else {
            fluid.push(entry);
        }
    }
    // Deposit source solids onto paper first so this pour's soluble fraction is
    // eligible for wash with the fluid still about to enter the filtrate.
    mix_transfer_into(&mut scene.items[paper_idx], &solids, None);
    let mut fluid_t = source_t;
    crate::dissolve_kinetics::wash_paper_solids_into_fluid(
        &mut scene.items[paper_idx],
        &mut fluid,
        &mut fluid_t,
    );
    mix_transfer_into(&mut scene.items[dest_idx], &fluid, Some(fluid_t));
    // Wash already applied contact-time kinetics; finalize is SI/speciate only.
    finalize_aqueous_vessel(&mut scene.items[source_idx], 0.0);
    finalize_aqueous_vessel(&mut scene.items[dest_idx], 0.0);
    scene.last_events.push(SceneEvent {
        kind: "poured".into(),
        message: "Filtered into the filtrate beaker.".into(),
    });
    Ok(())
}

pub(crate) fn remove_solid_mass(item: &mut SceneItem, substance_id: &str, mass_g: f64) {
    if mass_g <= AMOUNT_EPS {
        return;
    }
    let Some(existing) = item
        .properties
        .composition
        .iter_mut()
        .find(|c| c.substance_id == substance_id && c.phase == "solid")
    else {
        return;
    };
    let remain = (solid_amount_g(existing) - mass_g).max(0.0);
    if remain <= AMOUNT_EPS {
        item.properties
            .composition
            .retain(|c| !(c.substance_id == substance_id && c.phase == "solid"));
        return;
    }
    existing.amount_g = Some(remain);
    if substance_id == "nacl" {
        existing.amount_mol = Some(remain / NACL_MOLAR_MASS_G_PER_MOL);
    } else if substance_id == "cacl2" {
        existing.amount_mol = Some(remain / CACL2_MOLAR_MASS_G_PER_MOL);
    } else if substance_id == "naoh" {
        existing.amount_mol = Some(remain / NAOH_MOLAR_MASS_G_PER_MOL);
    } else if substance_id == "na2so4" {
        existing.amount_mol = Some(remain / NA2SO4_MOLAR_MASS_G_PER_MOL);
    } else if substance_id == "caso4" {
        existing.amount_mol = Some(remain / CASO4_MOLAR_MASS_G_PER_MOL);
    }
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

    let source_liquid = crate::hcl::transfer_volume_ml(&scene.items[source_idx]);
    let dest_cap =
        vessel_liquid_capacity_ml(&scene.items[dest_idx]).ok_or(SceneError::InvalidAction)?;
    let dest_liquid = crate::hcl::transfer_volume_ml(&scene.items[dest_idx]);
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
    if is_hcl_stock(&scene.items[dest_idx]) {
        if dest_room <= AMOUNT_EPS {
            return Err(SceneError::InvalidAction);
        }
        if !crate::hcl::composition_is_stock_hcl(&scene.items[source_idx].properties.composition) {
            return Err(SceneError::InvalidAction);
        }
    }
    if is_h2so4_stock(&scene.items[dest_idx]) {
        if dest_room <= AMOUNT_EPS {
            return Err(SceneError::InvalidAction);
        }
        if !crate::h2so4::composition_is_stock_h2so4(
            &scene.items[source_idx].properties.composition,
        ) {
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

    let (taken, source_t) = transfer_liquid_fraction(scene, source_idx, dest_idx, dest_cap)?;
    mix_transfer_into(&mut scene.items[dest_idx], &taken, Some(source_t));
    finalize_aqueous_vessel(&mut scene.items[source_idx], 0.0);
    finalize_aqueous_vessel(
        &mut scene.items[dest_idx],
        crate::dissolve_kinetics::POUR_CONTACT_TAU_S,
    );
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
    let source_t = scene.items[source_idx]
        .properties
        .temperature_c
        .unwrap_or(scene.temperature_c);
    let taken = take_all_solids(&mut scene.items[source_idx]);
    mix_transfer_into(&mut scene.items[dest_idx], &taken, Some(source_t));
    finalize_aqueous_vessel(&mut scene.items[source_idx], 0.0);
    finalize_aqueous_vessel(
        &mut scene.items[dest_idx],
        crate::dissolve_kinetics::POUR_CONTACT_TAU_S,
    );
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

fn merge_composition_into(
    target: &mut SceneItem,
    entries: &[CompositionEntry],
    include_solids: bool,
) {
    for entry in entries {
        if entry.substance_id == "water" && entry.phase == "liquid" {
            add_or_increase_water(target, entry.amount_ml.unwrap_or(0.0));
        } else if entry.substance_id == "h2so4" && entry.phase == "liquid" {
            add_or_increase_liquid_h2so4(target, entry);
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

fn add_or_increase_liquid_h2so4(item: &mut SceneItem, entry: &CompositionEntry) {
    let add_ml = entry.amount_ml.unwrap_or(0.0).max(0.0);
    let add_mol = entry
        .amount_mol
        .unwrap_or_else(|| crate::h2so4::liquid_h2so4_mol_entries(std::slice::from_ref(entry)));
    let add_g = entry
        .amount_g
        .unwrap_or(add_mol * crate::h2so4::H2SO4_MOLAR_MASS_G_PER_MOL);
    if add_ml <= AMOUNT_EPS && add_mol <= AMOUNT_EPS && add_g <= AMOUNT_EPS {
        return;
    }
    if let Some(existing) = item
        .properties
        .composition
        .iter_mut()
        .find(|c| c.substance_id == "h2so4" && c.phase == "liquid")
    {
        existing.amount_ml = Some(existing.amount_ml.unwrap_or(0.0) + add_ml);
        existing.amount_mol = Some(existing.amount_mol.unwrap_or(0.0) + add_mol);
        existing.amount_g = Some(existing.amount_g.unwrap_or(0.0) + add_g);
        return;
    }
    item.properties.composition.push(CompositionEntry {
        substance_id: "h2so4".into(),
        phase: "liquid".into(),
        amount_ml: Some(add_ml).filter(|&v| v > AMOUNT_EPS),
        amount_scoop: None,
        amount_g: Some(add_g).filter(|&v| v > AMOUNT_EPS),
        amount_mol: Some(add_mol).filter(|&v| v > AMOUNT_EPS),
    });
}

fn mix_transfer_into(
    target: &mut SceneItem,
    transferred: &[CompositionEntry],
    source_t: Option<f64>,
) {
    let dest_hcl_before = crate::hcl::HclInventory::from_item(target);
    let added_hcl = crate::hcl::HclInventory::from_entries(transferred);
    let dest_h2so4_before = crate::h2so4::H2so4Inventory::from_item(target);
    let added_h2so4 = crate::h2so4::H2so4Inventory::from_entries(transferred);
    if let Some(source_t) = source_t {
        let c_add = heat_capacity_of_entries(transferred);
        if c_add > AMOUNT_EPS {
            blend_temperature_capacity(target, c_add, source_t);
        }
    }
    merge_composition_into(target, transferred, true);
    crate::h2so4::ionize_liquid_h2so4_in_water(target);
    crate::solubility::sync_fill_ml(target);
    apply_hcl_dilution_temperature(target, dest_hcl_before, added_hcl);
    apply_h2so4_dilution_temperature(target, dest_h2so4_before, added_h2so4);
}

fn mix_aliquot_into(target: &mut SceneItem, aliquot: &[CompositionEntry], aliquot_t: f64) {
    let dest_hcl_before = crate::hcl::HclInventory::from_item(target);
    let added_hcl = crate::hcl::HclInventory::from_entries(aliquot);
    let dest_h2so4_before = crate::h2so4::H2so4Inventory::from_item(target);
    let added_h2so4 = crate::h2so4::H2so4Inventory::from_entries(aliquot);
    let c_add = heat_capacity_of_entries(aliquot);
    // Pipette always blends with aliquot T (even when dest is empty / add_ml is tiny).
    if c_add > AMOUNT_EPS {
        blend_temperature_capacity(target, c_add, aliquot_t);
    } else if effective_heat_capacity(target) <= AMOUNT_EPS {
        target.properties.temperature_c = Some(aliquot_t);
    }
    merge_composition_into(target, aliquot, false);
    crate::h2so4::ionize_liquid_h2so4_in_water(target);
    crate::solubility::sync_fill_ml(target);
    apply_hcl_dilution_temperature(target, dest_hcl_before, added_hcl);
    apply_h2so4_dilution_temperature(target, dest_h2so4_before, added_h2so4);
}

fn apply_hcl_dilution_temperature(
    target: &mut SceneItem,
    dest_before: crate::hcl::HclInventory,
    added: crate::hcl::HclInventory,
) {
    // Gated on HCl inventory (`min(max(n_h − n_oh, 0), n_cl)`), not bare protons.
    if dest_before.n_h <= AMOUNT_EPS && added.n_h <= AMOUNT_EPS {
        return;
    }
    let after = crate::hcl::HclInventory::from_item(target);
    let q = crate::hcl::hcl_dilution_heat_j(dest_before, added, after);
    apply_chemical_heat(target, q);
}

fn apply_h2so4_dilution_temperature(
    target: &mut SceneItem,
    dest_before: crate::h2so4::H2so4Inventory,
    added: crate::h2so4::H2so4Inventory,
) {
    if dest_before.n_h2so4 <= AMOUNT_EPS && added.n_h2so4 <= AMOUNT_EPS {
        return;
    }
    let after = crate::h2so4::H2so4Inventory::from_item(target);
    let q = crate::h2so4::h2so4_dilution_heat_j(dest_before, added, after);
    apply_chemical_heat(target, q);
}

fn apply_chemical_heat(target: &mut SceneItem, q_j: f64) {
    if q_j.abs() <= AMOUNT_EPS {
        return;
    }
    let c_eff = effective_heat_capacity(target);
    if c_eff <= AMOUNT_EPS {
        return;
    }
    let t = target
        .properties
        .temperature_c
        .unwrap_or(AMBIENT_TEMPERATURE_C);
    target.properties.temperature_c = Some(t - q_j / c_eff);
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
    let liquid = crate::hcl::transfer_volume_ml(source);
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
            let water_ml = entry.amount_ml.unwrap_or(0.0).max(0.0);
            let taken_water = water_ml * frac;
            let rest = water_ml - taken_water;
            if taken_water > AMOUNT_EPS {
                aliquot.push(CompositionEntry {
                    substance_id: "water".into(),
                    phase: "liquid".into(),
                    amount_ml: Some(taken_water),
                    amount_scoop: None,
                    amount_g: None,
                    amount_mol: None,
                });
            }
            if rest > AMOUNT_EPS {
                remaining.push(CompositionEntry {
                    amount_ml: Some(rest),
                    ..entry
                });
            }
            continue;
        }
        if entry.substance_id == "h2so4" && entry.phase == "liquid" {
            let ml = entry.amount_ml.unwrap_or(0.0).max(0.0);
            let mol = entry.amount_mol.unwrap_or(0.0).max(0.0);
            let g = entry.amount_g.unwrap_or(0.0).max(0.0);
            let taken_ml = ml * frac;
            let taken_mol = mol * frac;
            let taken_g = g * frac;
            if taken_ml > AMOUNT_EPS || taken_mol > AMOUNT_EPS || taken_g > AMOUNT_EPS {
                aliquot.push(CompositionEntry {
                    substance_id: "h2so4".into(),
                    phase: "liquid".into(),
                    amount_ml: Some(taken_ml).filter(|&v| v > AMOUNT_EPS),
                    amount_scoop: None,
                    amount_g: Some(taken_g).filter(|&v| v > AMOUNT_EPS),
                    amount_mol: Some(taken_mol).filter(|&v| v > AMOUNT_EPS),
                });
            }
            let rest_ml = ml - taken_ml;
            let rest_mol = mol - taken_mol;
            let rest_g = g - taken_g;
            if rest_ml > AMOUNT_EPS || rest_mol > AMOUNT_EPS || rest_g > AMOUNT_EPS {
                remaining.push(CompositionEntry {
                    amount_ml: Some(rest_ml).filter(|&v| v > AMOUNT_EPS),
                    amount_g: Some(rest_g).filter(|&v| v > AMOUNT_EPS),
                    amount_mol: Some(rest_mol).filter(|&v| v > AMOUNT_EPS),
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
