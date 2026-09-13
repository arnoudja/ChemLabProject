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
    pub temperature_c: Option<i32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub composition: Vec<CompositionEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub holding: Vec<CompositionEntry>,
}

/// A single item in the lab scene (beaker, spoon, …).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "../../../apps/web/src/generated/")]
pub struct Item {
    pub id: String,
    /// `"beaker"` | `"spoon"` | …
    pub kind: String,
    pub label: String,
    /// `"bench"` | `"hand"` | …
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
    pub temperature_c: i32,
    pub items: Vec<Item>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub last_events: Vec<LabEvent>,
}

/// Client → server lab action. Tagged JSON `type`: `use_tool` | `pour`.
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
  "label": "Water",
  "location": "bench",
  "properties": {
    "volume_ml": 250,
    "fill_ml": 200,
    "transparent": true,
    "colourless": true,
    "temperature_c": 20,
    "composition": [
      { "substance_id": "water", "phase": "liquid", "amount_ml": 200 }
    ]
  }
}"#;
        let item: Item = serde_json::from_str(raw).unwrap();
        assert_eq!(item.id, "beaker-water");
        assert_eq!(item.kind, "beaker");
        assert_eq!(item.label, "Water");
        assert_eq!(item.location, "bench");
        assert_eq!(item.properties.volume_ml, Some(250.0));
        assert_eq!(item.properties.fill_ml, Some(200.0));
        assert_eq!(item.properties.transparent, Some(true));
        assert_eq!(item.properties.colourless, Some(true));
        assert_eq!(item.properties.temperature_c, Some(20));
        assert!(item.properties.holding.is_empty());
        assert_eq!(item.properties.composition.len(), 1);
        assert_eq!(item.properties.composition[0].substance_id, "water");
        assert_eq!(item.properties.composition[0].phase, "liquid");
        assert_eq!(item.properties.composition[0].amount_ml, Some(200.0));
        assert_eq!(item.properties.composition[0].amount_scoop, None);

        let scene = LabScene {
            lab_id: "lab-1".into(),
            version: 1,
            temperature_c: 20,
            items: vec![item.clone()],
            last_events: vec![],
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
                }],
            },
        };
        let scene = LabScene {
            lab_id: "lab-1".into(),
            version: 2,
            temperature_c: 20,
            items: vec![spoon],
            last_events: vec![LabEvent {
                kind: "scooped".into(),
                message: "Scooped nacl onto spoon.".into(),
            }],
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
        };
        let json = serde_json::to_string(&props).unwrap();
        assert!(!json.contains("composition"));
        assert!(!json.contains("holding"));
        assert!(!json.contains("fill_ml"));
        let back: ItemProperties = serde_json::from_str(&json).unwrap();
        assert_eq!(props, back);
    }
}
