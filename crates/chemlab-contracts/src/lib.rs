//! Wire-format DTOs shared between Axum and the Vite frontend (via ts-rs).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Liveness / readiness payload for `GET /api/health`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../../apps/web/src/generated/")]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub service: String,
}

impl HealthResponse {
    pub fn ok(version: impl Into<String>) -> Self {
        Self {
            status: "ok".into(),
            version: version.into(),
            service: "chemlab-server".into(),
        }
    }
}

/// Register a new account (v0.1 stub — email + password).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../../apps/web/src/generated/")]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub display_name: String,
}

/// Log in with email + password.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../../apps/web/src/generated/")]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Authenticated user as returned to the client.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../../apps/web/src/generated/")]
pub struct AuthUserResponse {
    pub id: String,
    pub email: String,
    pub display_name: String,
    #[ts(type = "string")]
    pub created_at: DateTime<Utc>,
}

/// Generic API error body.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../../apps/web/src/generated/")]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
}

/// Session probe response for `GET /api/auth/me`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../../apps/web/src/generated/")]
pub struct MeResponse {
    pub authenticated: bool,
    pub user: Option<AuthUserResponse>,
}

/// CSRF synchronizer token from `GET /api/auth/csrf` (cookie + `X-CSRF-Token` header).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../../apps/web/src/generated/")]
pub struct CsrfResponse {
    pub csrf_token: String,
}

/// Inputs for `POST /api/lab/dissolve`. Ids are matched as-is (no trim or case-fold).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../../apps/web/src/generated/")]
pub struct DissolveRequest {
    pub substance_id: String,
    pub solvent_id: String,
    pub temperature_c: i32,
}

/// Server-authoritative dissolve prediction (`dissolved` + UI `explanation`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../../apps/web/src/generated/")]
pub struct DissolveResponse {
    pub dissolved: bool,
    pub explanation: String,
}

// skip_serializing_if omits null/empty fields in JSON. ts-rs 11+ parses those serde
// attrs; with #[serde(default)] + skip_serializing_if on Vec, generated TS marks the
// field optional (`T[]?`), matching wire omission. Frontend must treat missing as [].
/// One substance entry in an item's composition or holding list.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "../../../apps/web/src/generated/")]
pub struct CompositionEntry {
    pub substance_id: String,
    /// `"solid"` | `"liquid"` | `"aqueous"` for this slice.
    pub phase: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount_ml: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount_scoop: Option<u32>,
    /// Mass in grams (solids). One spoon scoop is 0.2 g.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount_g: Option<f64>,
    /// Amount of substance in moles (aqueous species).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount_mol: Option<f64>,
}

/// Physical / chemical properties of a lab item (server-authored).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "../../../apps/web/src/generated/")]
pub struct ItemProperties {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_ml: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill_ml: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transparent: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colourless: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature_c: Option<f64>,
    /// Omitted from JSON when empty; treat as `[]` on the client.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub composition: Vec<CompositionEntry>,
    /// Omitted from JSON when empty; treat as `[]` on the client.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub holding: Vec<CompositionEntry>,
    /// Burner flame; omitted when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on: Option<bool>,
    /// Last vessel a pipette drew from, the vessel tongs currently hold, or the
    /// dish a spoon scoop came from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_item_id: Option<String>,
}

/// A single item in the lab scene (beaker, spoon, …).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "../../../apps/web/src/generated/")]
pub struct Item {
    pub id: String,
    /// `"beaker"` | `"spoon"` | `"pipette"` | `"tongs"` | `"evaporation_dish"` | `"burner"` | `"filter_paper"` | …
    pub kind: String,
    pub label: String,
    /// `"bench"` | `"hand"` | `"held"` | …
    pub location: String,
    pub properties: ItemProperties,
}

/// UI-facing event from the last applied action(s).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../../apps/web/src/generated/")]
pub struct LabEvent {
    /// e.g. `"scooped"`, `"poured"`, `"dissolved"`, `"did_not_dissolve"`.
    pub kind: String,
    pub message: String,
}

/// Full lab scene snapshot returned to the browser.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "../../../apps/web/src/generated/")]
pub struct LabScene {
    pub lab_id: String,
    pub version: u32,
    /// Bench ambient temperature; default 20.
    pub temperature_c: f64,
    pub items: Vec<Item>,
    /// Omitted from JSON when empty; treat as `[]` on the client.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub last_events: Vec<LabEvent>,
    /// Server clock watermark (unix ms) for elapsed heat/evaporation. Omitted when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_applied_unix_ms: Option<i64>,
}

/// Client → server lab action. Tagged JSON `type`: `use_tool` | `pour` | `put_away` | `reset` | `toggle_burner`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../../apps/web/src/generated/")]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LabAction {
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
    /// Rebuild the default bench scene (pure water, empty spoon, stock jars).
    Reset,
    /// Idle click on the burner. Stays off when the dish has no liquid.
    ToggleBurner { burner_item_id: String },
}

/// Response body for `POST /api/lab/action`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "../../../apps/web/src/generated/")]
pub struct LabActionResponse {
    pub scene: LabScene,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_response_round_trips() {
        let health = HealthResponse::ok("0.1.0");
        let json = serde_json::to_string(&health).unwrap();
        let back: HealthResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(health, back);
        assert_eq!(back.status, "ok");
    }

    #[test]
    fn register_request_deserializes() {
        let raw = r#"{"email":"a@b.co","password":"secret123","display_name":"Ada"}"#;
        let req: RegisterRequest = serde_json::from_str(raw).unwrap();
        assert_eq!(req.display_name, "Ada");
    }

    #[test]
    fn dissolve_request_round_trips() {
        let req = DissolveRequest {
            substance_id: "nacl".into(),
            solvent_id: "water".into(),
            temperature_c: 20,
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: DissolveRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req, back);
    }

    #[test]
    fn dissolve_response_round_trips() {
        let resp = DissolveResponse {
            dissolved: false,
            explanation: "Sand (silica) does not dissolve in water at bench temperature.".into(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let back: DissolveResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(resp, back);
        assert!(!back.dissolved);
    }

    #[test]
    fn water_beaker_item_deserializes_from_spec_fragment() {
        let raw = r#"{
  "id": "beaker-water",
  "kind": "beaker",
  "label": "Beaker",
  "location": "bench",
  "properties": {
    "volume_ml": 250,
    "fill_ml": 0,
    "transparent": true,
    "colourless": true,
    "temperature_c": 20,
    "composition": []
  }
}"#;
        let item: Item = serde_json::from_str(raw).unwrap();
        assert_eq!(item.id, "beaker-water");
        assert_eq!(item.kind, "beaker");
        assert_eq!(item.label, "Beaker");
        assert_eq!(item.location, "bench");
        assert_eq!(item.properties.volume_ml, Some(250.0));
        assert_eq!(item.properties.fill_ml, Some(0.0));
        assert_eq!(item.properties.transparent, Some(true));
        assert_eq!(item.properties.colourless, Some(true));
        assert_eq!(item.properties.temperature_c, Some(20.0));
        assert!(item.properties.holding.is_empty());
        assert!(item.properties.composition.is_empty());

        let scene = LabScene {
            lab_id: "lab-1".into(),
            version: 1,
            temperature_c: 20.0,
            items: vec![item.clone()],
            last_events: vec![],
            last_applied_unix_ms: None,
        };
        let json = serde_json::to_string(&scene).unwrap();
        let back: LabScene = serde_json::from_str(&json).unwrap();
        assert_eq!(scene, back);
        assert_eq!(back.items[0], item);
    }

    #[test]
    fn use_tool_action_deserializes_from_spec_json() {
        let raw = r#"{
  "type": "use_tool",
  "tool_item_id": "spoon-1",
  "target_item_id": "beaker-nacl"
}"#;
        let action: LabAction = serde_json::from_str(raw).unwrap();
        assert_eq!(
            action,
            LabAction::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: "beaker-nacl".into(),
            }
        );
        let json = serde_json::to_string(&action).unwrap();
        let back: LabAction = serde_json::from_str(&json).unwrap();
        assert_eq!(action, back);
    }

    #[test]
    fn put_away_action_deserializes_from_json() {
        let raw = r#"{ "type": "put_away", "tool_item_id": "spoon-1" }"#;
        let action: LabAction = serde_json::from_str(raw).unwrap();
        assert_eq!(
            action,
            LabAction::PutAway {
                tool_item_id: "spoon-1".into(),
            }
        );
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains(r#""type":"put_away""#));
        let back: LabAction = serde_json::from_str(&json).unwrap();
        assert_eq!(action, back);
    }

    #[test]
    fn reset_action_deserializes_from_json() {
        let raw = r#"{ "type": "reset" }"#;
        let action: LabAction = serde_json::from_str(raw).unwrap();
        assert_eq!(action, LabAction::Reset);
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains(r#""type":"reset""#));
        let back: LabAction = serde_json::from_str(&json).unwrap();
        assert_eq!(action, back);
    }

    #[test]
    fn pour_action_and_lab_action_response_round_trip() {
        let action = LabAction::Pour {
            source_item_id: "spoon-1".into(),
            target_item_id: "beaker-water".into(),
        };
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains(r#""type":"pour""#));
        let back: LabAction = serde_json::from_str(&json).unwrap();
        assert_eq!(action, back);

        let spoon = Item {
            id: "spoon-1".into(),
            kind: "spoon".into(),
            label: "Spoon".into(),
            location: "hand".into(),
            properties: ItemProperties {
                volume_ml: None,
                fill_ml: None,
                transparent: None,
                colourless: None,
                temperature_c: None,
                composition: vec![],
                holding: vec![CompositionEntry {
                    substance_id: "nacl".into(),
                    phase: "solid".into(),
                    amount_ml: None,
                    amount_scoop: Some(1),
                    amount_g: Some(0.2),
                    amount_mol: None,
                }],
                on: None,
                source_item_id: None,
            },
        };
        let scene = LabScene {
            lab_id: "lab-1".into(),
            version: 2,
            temperature_c: 20.0,
            items: vec![spoon],
            last_events: vec![LabEvent {
                kind: "scooped".into(),
                message: "Scooped nacl onto spoon.".into(),
            }],
            last_applied_unix_ms: None,
        };
        let response = LabActionResponse {
            scene: scene.clone(),
        };
        let resp_json = serde_json::to_string(&response).unwrap();
        let resp_back: LabActionResponse = serde_json::from_str(&resp_json).unwrap();
        assert_eq!(response, resp_back);
        assert_eq!(
            resp_back.scene.items[0].properties.holding[0].substance_id,
            "nacl"
        );
    }

    #[test]
    fn item_properties_omit_empty_lists_when_serializing() {
        let props = ItemProperties {
            volume_ml: Some(100.0),
            fill_ml: None,
            transparent: None,
            colourless: None,
            temperature_c: None,
            composition: vec![],
            holding: vec![],
            on: None,
            source_item_id: None,
        };
        let json = serde_json::to_string(&props).unwrap();
        assert!(!json.contains("composition"));
        assert!(!json.contains("holding"));
        assert!(!json.contains("fill_ml"));
        assert!(!json.contains("on"));
        assert!(!json.contains("source_item_id"));
        let back: ItemProperties = serde_json::from_str(&json).unwrap();
        assert_eq!(props, back);
    }

    #[test]
    fn toggle_burner_action_deserializes_from_json() {
        let raw = r#"{ "type": "toggle_burner", "burner_item_id": "burner-1" }"#;
        let action: LabAction = serde_json::from_str(raw).unwrap();
        assert_eq!(
            action,
            LabAction::ToggleBurner {
                burner_item_id: "burner-1".into(),
            }
        );
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains(r#""type":"toggle_burner""#));
        let back: LabAction = serde_json::from_str(&json).unwrap();
        assert_eq!(action, back);
    }

    #[test]
    fn pipette_dish_and_burner_items_round_trip() {
        let pipette = Item {
            id: "pipette-1".into(),
            kind: "pipette".into(),
            label: "Pipette".into(),
            location: "hand".into(),
            properties: ItemProperties {
                volume_ml: Some(1.0),
                fill_ml: Some(1.0),
                transparent: None,
                colourless: None,
                temperature_c: Some(20.0),
                composition: vec![],
                holding: vec![CompositionEntry {
                    substance_id: "water".into(),
                    phase: "liquid".into(),
                    amount_ml: Some(1.0),
                    amount_scoop: None,
                    amount_g: None,
                    amount_mol: None,
                }],
                on: None,
                source_item_id: Some("beaker-water".into()),
            },
        };
        let dish = Item {
            id: "dish-1".into(),
            kind: "evaporation_dish".into(),
            label: "Evaporation dish".into(),
            location: "bench".into(),
            properties: ItemProperties {
                volume_ml: Some(25.0),
                fill_ml: Some(0.0),
                transparent: Some(true),
                colourless: Some(true),
                temperature_c: Some(20.0),
                composition: vec![],
                holding: vec![],
                on: None,
                source_item_id: None,
            },
        };
        let burner = Item {
            id: "burner-1".into(),
            kind: "burner".into(),
            label: "Burner".into(),
            location: "bench".into(),
            properties: ItemProperties {
                volume_ml: None,
                fill_ml: None,
                transparent: None,
                colourless: None,
                temperature_c: None,
                composition: vec![],
                holding: vec![],
                on: Some(false),
                source_item_id: None,
            },
        };
        let scene = LabScene {
            lab_id: "lab-1".into(),
            version: 0,
            temperature_c: 20.0,
            items: vec![pipette, dish, burner],
            last_events: vec![],
            last_applied_unix_ms: Some(1_700_000_000_000),
        };
        let json = serde_json::to_string(&scene).unwrap();
        assert!(json.contains(r#""kind":"pipette""#));
        assert!(json.contains(r#""kind":"evaporation_dish""#));
        assert!(json.contains(r#""kind":"burner""#));
        assert!(json.contains(r#""source_item_id":"beaker-water""#));
        assert!(json.contains(r#""on":false"#));
        assert!(json.contains(r#""last_applied_unix_ms":1700000000000"#));
        let back: LabScene = serde_json::from_str(&json).unwrap();
        assert_eq!(scene, back);
        assert_eq!(
            back.items[0].properties.source_item_id.as_deref(),
            Some("beaker-water")
        );
        assert_eq!(back.items[2].properties.on, Some(false));
        assert_eq!(back.last_applied_unix_ms, Some(1_700_000_000_000));
    }

    #[test]
    fn distilled_water_beaker_item_round_trips() {
        let item = Item {
            id: "beaker-h2o".into(),
            kind: "beaker".into(),
            label: "Distilled water".into(),
            location: "bench".into(),
            properties: ItemProperties {
                volume_ml: Some(100.0),
                fill_ml: Some(100.0),
                transparent: Some(true),
                colourless: Some(true),
                temperature_c: Some(20.0),
                composition: vec![CompositionEntry {
                    substance_id: "water".into(),
                    phase: "liquid".into(),
                    amount_ml: Some(100.0),
                    amount_scoop: None,
                    amount_g: None,
                    amount_mol: None,
                }],
                holding: vec![],
                on: None,
                source_item_id: None,
            },
        };
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains(r#""id":"beaker-h2o""#));
        assert!(json.contains(r#""kind":"beaker""#));
        assert!(json.contains(r#""label":"Distilled water""#));
        assert!(json.contains(r#""amount_ml":100.0"#) || json.contains(r#""amount_ml":100"#));
        let back: Item = serde_json::from_str(&json).unwrap();
        assert_eq!(item, back);
        assert_eq!(back.properties.volume_ml, Some(100.0));
        assert_eq!(back.properties.composition[0].substance_id, "water");
        assert_eq!(back.properties.composition[0].phase, "liquid");
        assert_eq!(back.properties.composition[0].amount_ml, Some(100.0));
    }

    #[test]
    fn tongs_item_round_trips_held_vessel() {
        let tongs = Item {
            id: "tongs-1".into(),
            kind: "tongs".into(),
            label: "Tongs".into(),
            location: "hand".into(),
            properties: ItemProperties {
                volume_ml: None,
                fill_ml: None,
                transparent: None,
                colourless: None,
                temperature_c: None,
                composition: vec![],
                holding: vec![],
                on: None,
                source_item_id: Some("beaker-water".into()),
            },
        };
        let water = Item {
            id: "beaker-water".into(),
            kind: "beaker".into(),
            label: "Beaker".into(),
            location: "held".into(),
            properties: ItemProperties {
                volume_ml: Some(250.0),
                fill_ml: Some(200.0),
                transparent: Some(true),
                colourless: Some(true),
                temperature_c: Some(20.0),
                composition: vec![CompositionEntry {
                    substance_id: "water".into(),
                    phase: "liquid".into(),
                    amount_ml: Some(200.0),
                    amount_scoop: None,
                    amount_g: None,
                    amount_mol: None,
                }],
                holding: vec![],
                on: None,
                source_item_id: None,
            },
        };
        let json = serde_json::to_string(&tongs).unwrap();
        assert!(json.contains(r#""kind":"tongs""#));
        assert!(json.contains(r#""source_item_id":"beaker-water""#));
        let back: Item = serde_json::from_str(&json).unwrap();
        assert_eq!(tongs, back);
        let water_json = serde_json::to_string(&water).unwrap();
        assert!(water_json.contains(r#""location":"held""#));
        let water_back: Item = serde_json::from_str(&water_json).unwrap();
        assert_eq!(water, water_back);
    }

    #[test]
    fn spoon_dish_scoop_round_trips_source_and_mixed_holding() {
        let spoon = Item {
            id: "spoon-1".into(),
            kind: "spoon".into(),
            label: "Spoon".into(),
            location: "hand".into(),
            properties: ItemProperties {
                volume_ml: None,
                fill_ml: None,
                transparent: None,
                colourless: None,
                temperature_c: None,
                composition: vec![],
                holding: vec![
                    CompositionEntry {
                        substance_id: "nacl".into(),
                        phase: "solid".into(),
                        amount_ml: None,
                        amount_scoop: None,
                        amount_g: Some(0.12),
                        amount_mol: None,
                    },
                    CompositionEntry {
                        substance_id: "cacl2".into(),
                        phase: "solid".into(),
                        amount_ml: None,
                        amount_scoop: None,
                        amount_g: Some(0.08),
                        amount_mol: None,
                    },
                ],
                on: None,
                source_item_id: Some("dish-1".into()),
            },
        };
        let json = serde_json::to_string(&spoon).unwrap();
        assert!(json.contains(r#""source_item_id":"dish-1""#));
        assert!(json.contains(r#""amount_g":0.12"#));
        assert!(json.contains(r#""amount_g":0.08"#));
        let back: Item = serde_json::from_str(&json).unwrap();
        assert_eq!(spoon, back);
        assert_eq!(back.properties.holding.len(), 2);
        assert_eq!(back.properties.source_item_id.as_deref(), Some("dish-1"));
    }

    #[test]
    fn filtrate_beaker_item_round_trips() {
        let item = Item {
            id: "beaker-filtrate".into(),
            kind: "beaker".into(),
            label: "Filtrate".into(),
            location: "bench".into(),
            properties: ItemProperties {
                volume_ml: Some(250.0),
                fill_ml: Some(0.0),
                transparent: Some(true),
                colourless: Some(true),
                temperature_c: Some(20.0),
                composition: vec![],
                holding: vec![],
                on: None,
                source_item_id: None,
            },
        };
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains(r#""id":"beaker-filtrate""#));
        assert!(json.contains(r#""kind":"beaker""#));
        assert!(json.contains(r#""label":"Filtrate""#));
        let back: Item = serde_json::from_str(&json).unwrap();
        assert_eq!(item, back);
        assert_eq!(back.properties.volume_ml, Some(250.0));
        assert!(back.properties.composition.is_empty());
    }

    #[test]
    fn filter_paper_item_round_trips_solids() {
        let item = Item {
            id: "filter-paper-1".into(),
            kind: "filter_paper".into(),
            label: "Filter paper".into(),
            location: "bench".into(),
            properties: ItemProperties {
                volume_ml: None,
                fill_ml: None,
                transparent: None,
                colourless: None,
                temperature_c: None,
                composition: vec![CompositionEntry {
                    substance_id: "sand".into(),
                    phase: "solid".into(),
                    amount_ml: None,
                    amount_scoop: None,
                    amount_g: Some(0.2),
                    amount_mol: None,
                }],
                holding: vec![],
                on: None,
                source_item_id: None,
            },
        };
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains(r#""id":"filter-paper-1""#));
        assert!(json.contains(r#""kind":"filter_paper""#));
        assert!(json.contains(r#""label":"Filter paper""#));
        assert!(json.contains(r#""amount_g":0.2"#) || json.contains(r#""amount_g":0.20"#));
        let back: Item = serde_json::from_str(&json).unwrap();
        assert_eq!(item, back);
        assert_eq!(back.kind, "filter_paper");
        assert_eq!(back.properties.composition[0].substance_id, "sand");
        assert_eq!(back.properties.composition[0].amount_g, Some(0.2));
    }
}
