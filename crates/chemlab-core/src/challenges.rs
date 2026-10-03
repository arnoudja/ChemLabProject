//! Challenge catalog: the engine mirror of `docs/challenges.md`.
//!
//! A lab is always in exactly one mode — [`FREE_MODE`] or a challenge id. The catalog
//! describes a challenge purely as data (which stocks stay on the bench, which start
//! empty, what the main beaker is preloaded with, and when it is won), so a new
//! challenge is one more [`Challenge`] entry plus a section in the doc.

use crate::composition::aqueous_mol;
use crate::h2so4::liquid_h2so4_ml;
use crate::hcl::{ph_of_item, HclInventory};
use crate::scene::{
    solid_amount_g, Scene, SceneItem, AMBIENT_TEMPERATURE_C, CACL2_MOLAR_MASS_G_PER_MOL,
    CASO4_MOLAR_MASS_G_PER_MOL, NA2SO4_MOLAR_MASS_G_PER_MOL, NACL_MOLAR_MASS_G_PER_MOL,
    NAOH_MOLAR_MASS_G_PER_MOL,
};

/// Mode id of the default bench. Never "completed".
pub const FREE_MODE: &str = "free";

/// Tolerance when comparing a stock beaker against a win target, in grams.
///
/// Solids travel through mol ↔ gram conversions and fractional pours, so an exact
/// float compare would make a finished challenge unwinnable.
pub const CHALLENGE_MASS_TOLERANCE_G: f64 = 1e-6;

/// Aqueous "present" floor for win checks (mol).
const AQ_PRESENT_MOL: f64 = 1e-6;

/// How a [`StockTarget`] mass is compared against the stock's solid grams.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WinCompare {
    /// `|grams − amount_g| ≤ CHALLENGE_MASS_TOLERANCE_G`
    Exact,
    /// `grams + CHALLENGE_MASS_TOLERANCE_G ≥ amount_g`
    AtLeast,
}

/// One "this vessel holds about this much of this solid species" clause.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StockTarget {
    pub item_id: &'static str,
    pub substance_id: &'static str,
    pub amount_g: f64,
    pub compare: WinCompare,
}

/// Extra win predicates (pH, T, aqueous ions, leftover solid/liquid, HCl w/w).
///
/// Solid mass still uses [`StockTarget`] (any vessel, including filter paper).
/// `item_ids` clauses hold when **any** listed vessel satisfies the predicate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WinCheck {
    Ph {
        item_ids: &'static [&'static str],
        min: f64,
        max: f64,
    },
    TemperatureAtLeast {
        item_id: &'static str,
        min_c: f64,
    },
    AqueousAtLeast {
        item_ids: &'static [&'static str],
        substance_id: &'static str,
        amount_mol: f64,
    },
    SolidAtMost {
        item_id: &'static str,
        substance_id: &'static str,
        amount_g: f64,
    },
    /// Any listed vessel holds at least this much solid.
    SolidAtLeast {
        item_ids: &'static [&'static str],
        substance_id: &'static str,
        amount_g: f64,
    },
    LiquidAtMost {
        item_id: &'static str,
        substance_id: &'static str,
        amount_ml: f64,
    },
    HclWw {
        item_ids: &'static [&'static str],
        min: f64,
        max: f64,
    },
    /// Same vessel is acidic HCl: pH ≤ `max_ph` and aqueous `h+` and `cl-`.
    AcidicHcl {
        item_ids: &'static [&'static str],
        max_ph: f64,
    },
    /// Vessel was observed below 18% w/w HCl (VLE-rise latch).
    HclSeenLean {
        item_id: &'static str,
    },
    /// Vessel received liquid H₂SO₄ into water, and never water-onto-acid.
    AcidAddedIntoWater {
        item_id: &'static str,
    },
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
    /// Starting liquid ml in `beaker-h2o`. `None` keeps Free mode's full stock.
    pub distilled_water_ml: Option<f64>,
    /// Solid-mass clauses (stocks, filter paper, main beaker, …).
    pub win: &'static [StockTarget],
    /// Non-mass clauses; all must hold together with [`Challenge::win`].
    pub checks: &'static [WinCheck],
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
    distilled_water_ml: Some(10.0),
    win: &[
        StockTarget {
            item_id: "beaker-nacl",
            substance_id: "nacl",
            amount_g: 2.0,
            compare: WinCompare::Exact,
        },
        StockTarget {
            item_id: "beaker-sand",
            substance_id: "sand",
            amount_g: 2.0,
            compare: WinCompare::Exact,
        },
    ],
    checks: &[],
};

/// Neutralize HCl with NaOH, evaporate, and return solid NaCl to its empty stock.
pub const CREATE_TABLE_SALT: Challenge = Challenge {
    id: "create-table-salt",
    title: "Create table salt",
    prompt: "We're out of NaCl again, can you create some for us?",
    done: "Thank you again.",
    allowed_stock_item_ids: &["beaker-h2o", "beaker-hcl", "beaker-naoh", "beaker-nacl"],
    empty_stock_item_ids: &["beaker-nacl"],
    main_beaker_solids: &[],
    distilled_water_ml: None,
    win: &[StockTarget {
        item_id: "beaker-nacl",
        substance_id: "nacl",
        amount_g: 0.20,
        compare: WinCompare::AtLeast,
    }],
    checks: &[],
};

/// Neutralize H₂SO₄ with NaOH, evaporate, return solid Na₂SO₄ to its empty stock.
pub const CREATE_SODIUM_SULFATE: Challenge = Challenge {
    id: "create-sodium-sulfate",
    title: "Create sodium sulfate",
    prompt:
        "We're out of sodium sulfate, can you make some from sulfuric acid and sodium hydroxide?",
    done: "Thank you.",
    allowed_stock_item_ids: &["beaker-h2o", "beaker-h2so4", "beaker-naoh", "beaker-na2so4"],
    empty_stock_item_ids: &["beaker-na2so4"],
    main_beaker_solids: &[],
    distilled_water_ml: None,
    win: &[],
    checks: &[WinCheck::SolidAtLeast {
        item_ids: &["beaker-na2so4", "dish-1"],
        substance_id: "na2so4",
        amount_g: 0.20,
    }],
};

/// Mix CaCl₂ with a sulfate source and filter off gypsum.
pub const PRECIPITATE_GYPSUM: Challenge = Challenge {
    id: "precipitate-gypsum",
    title: "Precipitate gypsum",
    prompt: "We need gypsum. Mix calcium chloride with a sulfate and filter off the solid.",
    done: "Thank you.",
    allowed_stock_item_ids: &[
        "beaker-h2o",
        "beaker-cacl2",
        "beaker-na2so4",
        "beaker-h2so4",
    ],
    empty_stock_item_ids: &[],
    main_beaker_solids: &[],
    distilled_water_ml: None,
    win: &[StockTarget {
        item_id: "filter-paper-1",
        substance_id: "caso4",
        amount_g: 0.15,
        compare: WinCompare::AtLeast,
    }],
    checks: &[],
};

/// CaCl₂ + H₂SO₄ → gypsum + aqueous HCl; filter.
pub const MAKE_HCL_FROM_GYPSUM: Challenge = Challenge {
    id: "make-hcl-from-gypsum",
    title: "Make hydrochloric acid",
    prompt: "Make hydrochloric acid by mixing calcium chloride with sulfuric acid, then filter off the gypsum.",
    done: "Thank you.",
    allowed_stock_item_ids: &["beaker-h2o", "beaker-cacl2", "beaker-h2so4"],
    empty_stock_item_ids: &[],
    main_beaker_solids: &[],
    distilled_water_ml: None,
    win: &[StockTarget {
        item_id: "filter-paper-1",
        substance_id: "caso4",
        amount_g: 0.15,
        compare: WinCompare::AtLeast,
    }],
    checks: &[WinCheck::AcidicHcl {
        item_ids: &["beaker-filtrate", "dish-1"],
        max_ph: 3.0,
    }],
};

/// Dissolve CaCl₂ and inspect the exothermic temperature rise.
///
/// Vessel heat capacity caps the rise well below a calorimeter 30 °C, so the
/// judged bar is ≥ 25 °C after dissolving the 2 g stock into a small water volume.
pub const HOT_PACK_CACL2: Challenge = Challenge {
    id: "hot-pack-cacl2",
    title: "Hot pack",
    prompt: "Dissolve the calcium chloride in a little distilled water and check that the beaker warms up.",
    done: "Thank you.",
    allowed_stock_item_ids: &["beaker-h2o", "beaker-cacl2"],
    empty_stock_item_ids: &[],
    main_beaker_solids: &[],
    distilled_water_ml: Some(10.0),
    win: &[],
    checks: &[
        WinCheck::TemperatureAtLeast {
            item_id: "beaker-water",
            min_c: 25.0,
        },
        WinCheck::AqueousAtLeast {
            item_ids: &["beaker-water"],
            substance_id: "ca2+",
            amount_mol: AQ_PRESENT_MOL,
        },
        WinCheck::AqueousAtLeast {
            item_ids: &["beaker-water"],
            substance_id: "cl-",
            amount_mol: AQ_PRESENT_MOL,
        },
        WinCheck::SolidAtMost {
            item_id: "beaker-water",
            substance_id: "cacl2",
            amount_g: 1e-4,
        },
    ],
};

/// Near-saturated NaCl, then 30% HCl until extra solid salt appears.
pub const COMMON_ION_NACL: Challenge = Challenge {
    id: "common-ion-nacl",
    title: "Crash salt with acid",
    prompt: "Make a near-saturated salt solution, add a little hydrochloric acid, and heat until extra salt appears.",
    done: "Thank you.",
    allowed_stock_item_ids: &["beaker-h2o", "beaker-hcl", "beaker-nacl"],
    empty_stock_item_ids: &[],
    main_beaker_solids: &[],
    distilled_water_ml: Some(5.6),
    win: &[],
    checks: &[
        WinCheck::SolidAtLeast {
            item_ids: &["beaker-water", "dish-1"],
            substance_id: "nacl",
            amount_g: 0.05,
        },
        WinCheck::AqueousAtLeast {
            item_ids: &["beaker-water", "dish-1"],
            substance_id: "h+",
            amount_mol: AQ_PRESENT_MOL,
        },
        WinCheck::AqueousAtLeast {
            item_ids: &["beaker-water", "dish-1"],
            substance_id: "na+",
            amount_mol: 1e-4,
        },
    ],
};

/// NaOH + HCl; inspect pH is playable with 1 ml / 0.2 g steps (not a titre to 7).
pub const NEUTRALIZE_TO_PH7: Challenge = Challenge {
    id: "neutralize-to-ph7",
    title: "Neutralise to pH 7",
    prompt: "Neutralise sodium hydroxide with hydrochloric acid and check the inspect pH — a 1 ml pipette cannot land on 7.",
    done: "Thank you.",
    allowed_stock_item_ids: &["beaker-h2o", "beaker-hcl", "beaker-naoh"],
    empty_stock_item_ids: &[],
    main_beaker_solids: &[],
    distilled_water_ml: None,
    win: &[],
    checks: &[
        WinCheck::Ph {
            item_ids: &["beaker-water"],
            min: 0.0,
            max: 13.0,
        },
        WinCheck::AqueousAtLeast {
            item_ids: &["beaker-water"],
            substance_id: "na+",
            amount_mol: AQ_PRESENT_MOL,
        },
        WinCheck::AqueousAtLeast {
            item_ids: &["beaker-water"],
            substance_id: "cl-",
            amount_mol: AQ_PRESENT_MOL,
        },
        WinCheck::SolidAtMost {
            item_id: "beaker-water",
            substance_id: "naoh",
            amount_g: 1e-4,
        },
    ],
};

/// Add concentrated H₂SO₄ into water (never the reverse).
pub const DILUTE_SULFURIC_SAFE: Challenge = Challenge {
    id: "dilute-sulfuric-safe",
    title: "Dilute sulfuric acid safely",
    prompt: "Dilute the concentrated sulfuric acid the safe way: add the acid into water, not water onto the acid.",
    done: "Thank you.",
    allowed_stock_item_ids: &["beaker-h2o", "beaker-h2so4"],
    empty_stock_item_ids: &[],
    main_beaker_solids: &[],
    distilled_water_ml: None,
    win: &[],
    checks: &[
        WinCheck::AcidAddedIntoWater {
            item_id: "beaker-water",
        },
        WinCheck::LiquidAtMost {
            item_id: "beaker-water",
            substance_id: "h2so4",
            amount_ml: 1e-6,
        },
        WinCheck::AqueousAtLeast {
            item_ids: &["beaker-water"],
            substance_id: "h+",
            amount_mol: AQ_PRESENT_MOL,
        },
        WinCheck::TemperatureAtLeast {
            item_id: "beaker-water",
            min_c: AMBIENT_TEMPERATURE_C + 0.5,
        },
        WinCheck::Ph {
            item_ids: &["beaker-water"],
            min: f64::NEG_INFINITY,
            max: 2.0,
        },
    ],
};

/// Boil dish HCl toward the ~20% w/w azeotrope.
pub const CONCENTRATE_HCL_AZEOTROPE: Challenge = Challenge {
    id: "concentrate-hcl-azeotrope",
    title: "Concentrate hydrochloric acid",
    prompt: "Dilute the hydrochloric acid, heat it in the dish, and stop near the azeotrope — you cannot boil it to pure HCl.",
    done: "Thank you.",
    allowed_stock_item_ids: &["beaker-h2o", "beaker-hcl"],
    empty_stock_item_ids: &[],
    main_beaker_solids: &[],
    distilled_water_ml: None,
    win: &[],
    checks: &[
        WinCheck::HclSeenLean {
            item_id: "dish-1",
        },
        WinCheck::HclWw {
            item_ids: &["dish-1"],
            min: 0.18,
            max: 0.22,
        },
        WinCheck::TemperatureAtLeast {
            item_id: "dish-1",
            min_c: 99.0,
        },
    ],
};

/// Every challenge, in picker order (Free mode is not a challenge).
pub const CHALLENGES: &[Challenge] = &[
    SEPARATE_NACL_SIO2,
    CREATE_TABLE_SALT,
    CREATE_SODIUM_SULFATE,
    PRECIPITATE_GYPSUM,
    MAKE_HCL_FROM_GYPSUM,
    HOT_PACK_CACL2,
    COMMON_ION_NACL,
    NEUTRALIZE_TO_PH7,
    DILUTE_SULFURIC_SAFE,
    CONCENTRATE_HCL_AZEOTROPE,
];

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
        && challenge
            .checks
            .iter()
            .all(|check| check_holds(scene, check))
}

fn stock_holds_target(scene: &Scene, target: &StockTarget) -> bool {
    let Some(item) = scene.items.iter().find(|item| item.id == target.item_id) else {
        return false;
    };
    let grams = solid_grams(item, target.substance_id);
    match target.compare {
        WinCompare::Exact => (grams - target.amount_g).abs() <= CHALLENGE_MASS_TOLERANCE_G,
        WinCompare::AtLeast => grams + CHALLENGE_MASS_TOLERANCE_G >= target.amount_g,
    }
}

fn check_holds(scene: &Scene, check: &WinCheck) -> bool {
    match *check {
        WinCheck::Ph { item_ids, min, max } => item_ids.iter().any(|id| {
            scene
                .items
                .iter()
                .find(|item| item.id == *id)
                .and_then(ph_of_item)
                .is_some_and(|ph| ph >= min && ph <= max)
        }),
        WinCheck::TemperatureAtLeast { item_id, min_c } => scene
            .items
            .iter()
            .find(|item| item.id == item_id)
            .and_then(|item| item.properties.temperature_c)
            .is_some_and(|t| t + CHALLENGE_MASS_TOLERANCE_G >= min_c),
        WinCheck::AqueousAtLeast {
            item_ids,
            substance_id,
            amount_mol,
        } => item_ids.iter().any(|id| {
            scene
                .items
                .iter()
                .find(|item| item.id == *id)
                .is_some_and(|item| aqueous_mol(item, substance_id) + 1e-12 >= amount_mol)
        }),
        WinCheck::SolidAtMost {
            item_id,
            substance_id,
            amount_g,
        } => scene
            .items
            .iter()
            .find(|item| item.id == item_id)
            .is_some_and(|item| {
                solid_grams(item, substance_id) <= amount_g + CHALLENGE_MASS_TOLERANCE_G
            }),
        WinCheck::SolidAtLeast {
            item_ids,
            substance_id,
            amount_g,
        } => item_ids.iter().any(|id| {
            scene
                .items
                .iter()
                .find(|item| item.id == *id)
                .is_some_and(|item| {
                    solid_grams(item, substance_id) + CHALLENGE_MASS_TOLERANCE_G >= amount_g
                })
        }),
        WinCheck::LiquidAtMost {
            item_id,
            substance_id,
            amount_ml,
        } => scene
            .items
            .iter()
            .find(|item| item.id == item_id)
            .is_some_and(|item| {
                liquid_ml(item, substance_id) <= amount_ml + CHALLENGE_MASS_TOLERANCE_G
            }),
        WinCheck::HclWw { item_ids, min, max } => item_ids.iter().any(|id| {
            scene
                .items
                .iter()
                .find(|item| item.id == *id)
                .is_some_and(|item| {
                    let inv = HclInventory::from_item(item);
                    inv.total_mass_g() > 1e-6 && {
                        let w = inv.w_hcl();
                        w >= min && w <= max
                    }
                })
        }),
        WinCheck::AcidicHcl { item_ids, max_ph } => item_ids.iter().any(|id| {
            scene
                .items
                .iter()
                .find(|item| item.id == *id)
                .is_some_and(|item| {
                    ph_of_item(item).is_some_and(|ph| ph <= max_ph)
                        && aqueous_mol(item, "h+") + 1e-12 >= AQ_PRESENT_MOL
                        && aqueous_mol(item, "cl-") + 1e-12 >= AQ_PRESENT_MOL
                })
        }),
        WinCheck::HclSeenLean { item_id } => scene
            .items
            .iter()
            .find(|item| item.id == item_id)
            .is_some_and(|item| item.properties.hcl_seen_lean == Some(true)),
        WinCheck::AcidAddedIntoWater { item_id } => scene
            .items
            .iter()
            .find(|item| item.id == item_id)
            .is_some_and(|item| item.properties.h2so4_dilution_into_water == Some(true)),
    }
}

fn solid_grams(item: &SceneItem, substance_id: &str) -> f64 {
    item.properties
        .composition
        .iter()
        .filter(|entry| entry.phase == "solid" && entry.substance_id == substance_id)
        .map(|entry| {
            if let Some(g) = entry.amount_g {
                return g;
            }
            if let Some(n) = entry.amount_mol {
                return n * molar_mass_g_per_mol(substance_id);
            }
            solid_amount_g(entry)
        })
        .sum()
}

fn molar_mass_g_per_mol(substance_id: &str) -> f64 {
    match substance_id {
        "nacl" => NACL_MOLAR_MASS_G_PER_MOL,
        "cacl2" => CACL2_MOLAR_MASS_G_PER_MOL,
        "naoh" => NAOH_MOLAR_MASS_G_PER_MOL,
        "na2so4" => NA2SO4_MOLAR_MASS_G_PER_MOL,
        "caso4" => CASO4_MOLAR_MASS_G_PER_MOL,
        _ => 0.0,
    }
}

fn liquid_ml(item: &SceneItem, substance_id: &str) -> f64 {
    if substance_id == "h2so4" {
        return liquid_h2so4_ml(item);
    }
    item.properties
        .composition
        .iter()
        .filter(|entry| entry.phase == "liquid" && entry.substance_id == substance_id)
        .filter_map(|entry| entry.amount_ml)
        .sum()
}
