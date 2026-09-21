//! Challenge catalog: the engine mirror of `docs/challenges.md`.
//!
//! A lab is always in exactly one mode — [`FREE_MODE`] or a challenge id. The catalog
//! describes a challenge purely as data (which stocks stay on the bench, which start
//! empty, what the main beaker is preloaded with, and when it is won), so a new
//! challenge is one more [`Challenge`] entry plus a section in the doc.

use crate::scene::{solid_amount_g, Scene};

/// Mode id of the default bench. Never "completed".
pub const FREE_MODE: &str = "free";

/// Tolerance when comparing a stock beaker against a win target, in grams.
///
/// Solids travel through mol ↔ gram conversions and fractional pours, so an exact
/// float compare would make a finished challenge unwinnable.
pub const CHALLENGE_MASS_TOLERANCE_G: f64 = 1e-6;

/// Ingredient stock beakers on the Free bench. A challenge keeps a subset.
pub(crate) const STOCK_ITEM_IDS: &[&str] =
    &["beaker-h2o", "beaker-nacl", "beaker-cacl2", "beaker-sand"];

/// One "this stock holds about this much of this species" clause of a win condition.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StockTarget {
    pub item_id: &'static str,
    pub substance_id: &'static str,
    pub amount_g: f64,
}

/// A challenge as both product copy and engine rules.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Challenge {
    pub id: &'static str,
    /// Picker label.
    pub title: &'static str,
    /// Bench header copy while the challenge is unsolved.
    pub prompt: &'static str,
    /// Bench header copy once the win condition holds.
    pub done: &'static str,
    /// Ingredient stocks kept on the bench; the rest are removed from the Free layout.
    pub allowed_stock_item_ids: &'static [&'static str],
    /// Kept stocks that start with none of their solid.
    pub empty_stock_item_ids: &'static [&'static str],
    /// Dry solids (`substance_id`, grams) preloaded into `beaker-water`.
    pub main_beaker_solids: &'static [(&'static str, f64)],
    /// All clauses must hold for the challenge to be complete.
    pub win: &'static [StockTarget],
}

/// Separate a mixed 2 g NaCl / 2 g SiO₂ solid back into its stock beakers.
pub const SEPARATE_NACL_SIO2: Challenge = Challenge {
    id: "separate-nacl-sio2",
    title: "Separate salt from sand",
    prompt: "The previous student accidentally put all the salt and sand in the main beaker, can you please separate them and put them back in their containers?",
    done: "Thank you.",
    allowed_stock_item_ids: &["beaker-h2o", "beaker-nacl", "beaker-sand"],
    empty_stock_item_ids: &["beaker-nacl", "beaker-sand"],
    main_beaker_solids: &[("nacl", 2.0), ("sand", 2.0)],
    win: &[
        StockTarget {
            item_id: "beaker-nacl",
            substance_id: "nacl",
            amount_g: 2.0,
        },
        StockTarget {
            item_id: "beaker-sand",
            substance_id: "sand",
            amount_g: 2.0,
        },
    ],
};

/// Every challenge, in picker order (Free mode is not a challenge).
pub const CHALLENGES: &[Challenge] = &[SEPARATE_NACL_SIO2];

/// Whether `mode` is the default bench rather than a challenge.
pub fn is_free_mode(mode: &str) -> bool {
    mode == FREE_MODE
}

/// Look up a challenge by mode id.
pub fn find_challenge(id: &str) -> Option<&'static Challenge> {
    CHALLENGES.iter().find(|challenge| challenge.id == id)
}

/// Whether the scene's challenge win condition currently holds. Free mode is never complete.
pub fn is_completed(scene: &Scene) -> bool {
    let Some(challenge) = find_challenge(&scene.mode) else {
        return false;
    };
    challenge
        .win
        .iter()
        .all(|target| stock_holds_target(scene, target))
}

fn stock_holds_target(scene: &Scene, target: &StockTarget) -> bool {
    let Some(item) = scene.items.iter().find(|item| item.id == target.item_id) else {
        return false;
    };
    let grams: f64 = item
        .properties
        .composition
        .iter()
        .filter(|entry| entry.phase == "solid" && entry.substance_id == target.substance_id)
        .map(solid_amount_g)
        .sum();
    (grams - target.amount_g).abs() <= CHALLENGE_MASS_TOLERANCE_G
}
