//! `From` / `Into` between wire DTOs and `chemlab-core` domain types.
//!
//! `LabScene.challenge_completed` is wire-only and stays derived at the server
//! boundary — there is no `From<chemlab_core::Scene> for LabScene`. Outbound
//! mapping (e.g. server `scene_to_contract`) sets `challenge_completed` after
//! `is_completed`.

use crate::{CompositionEntry, Item, ItemProperties, LabAction, LabEvent, LabScene};

impl From<chemlab_core::CompositionEntry> for CompositionEntry {
    fn from(entry: chemlab_core::CompositionEntry) -> Self {
        Self {
            substance_id: entry.substance_id,
            phase: entry.phase,
            amount_ml: entry.amount_ml,
            amount_scoop: entry.amount_scoop,
            amount_g: entry.amount_g,
            amount_mol: entry.amount_mol,
        }
    }
}

impl From<CompositionEntry> for chemlab_core::CompositionEntry {
    fn from(entry: CompositionEntry) -> Self {
        Self {
            substance_id: entry.substance_id,
            phase: entry.phase,
            amount_ml: entry.amount_ml,
            amount_scoop: entry.amount_scoop,
            amount_g: entry.amount_g,
            amount_mol: entry.amount_mol,
        }
    }
}

impl From<chemlab_core::ItemProperties> for ItemProperties {
    fn from(props: chemlab_core::ItemProperties) -> Self {
        Self {
            volume_ml: props.volume_ml,
            fill_ml: props.fill_ml,
            transparent: props.transparent,
            colourless: props.colourless,
            temperature_c: props.temperature_c,
            composition: props.composition.into_iter().map(Into::into).collect(),
            holding: props.holding.into_iter().map(Into::into).collect(),
            on: props.on,
            source_item_id: props.source_item_id,
            h2so4_dilution_into_water: props.h2so4_dilution_into_water,
        }
    }
}

impl From<ItemProperties> for chemlab_core::ItemProperties {
    fn from(props: ItemProperties) -> Self {
        Self {
            volume_ml: props.volume_ml,
            fill_ml: props.fill_ml,
            transparent: props.transparent,
            colourless: props.colourless,
            temperature_c: props.temperature_c,
            composition: props.composition.into_iter().map(Into::into).collect(),
            holding: props.holding.into_iter().map(Into::into).collect(),
            on: props.on,
            source_item_id: props.source_item_id,
            h2so4_dilution_into_water: props.h2so4_dilution_into_water,
            hcl_seen_lean: None,
            nacl_seen_clear_brine: None,
            nacl_solid_g_at_acid: None,
        }
    }
}

impl From<chemlab_core::SceneItem> for Item {
    fn from(item: chemlab_core::SceneItem) -> Self {
        Self {
            id: item.id,
            kind: item.kind,
            label: item.label,
            location: item.location,
            properties: item.properties.into(),
        }
    }
}

impl From<Item> for chemlab_core::SceneItem {
    fn from(item: Item) -> Self {
        Self {
            id: item.id,
            kind: item.kind,
            label: item.label,
            location: item.location,
            properties: item.properties.into(),
        }
    }
}

impl From<chemlab_core::SceneEvent> for LabEvent {
    fn from(event: chemlab_core::SceneEvent) -> Self {
        Self {
            kind: event.kind,
            message: event.message,
        }
    }
}

impl From<LabEvent> for chemlab_core::SceneEvent {
    fn from(event: LabEvent) -> Self {
        Self {
            kind: event.kind,
            message: event.message,
        }
    }
}

impl From<LabScene> for chemlab_core::Scene {
    fn from(scene: LabScene) -> Self {
        // `challenge_completed` is derived on the way out; drop it inbound.
        Self {
            mode: scene.mode,
            lab_id: scene.lab_id,
            version: scene.version,
            temperature_c: scene.temperature_c,
            items: scene.items.into_iter().map(Into::into).collect(),
            last_events: scene.last_events.into_iter().map(Into::into).collect(),
            last_applied_unix_ms: scene.last_applied_unix_ms,
        }
    }
}

impl From<LabAction> for chemlab_core::Action {
    fn from(action: LabAction) -> Self {
        match action {
            LabAction::UseTool {
                tool_item_id,
                target_item_id,
            } => Self::UseTool {
                tool_item_id,
                target_item_id,
            },
            LabAction::Pour {
                source_item_id,
                target_item_id,
            } => Self::Pour {
                source_item_id,
                target_item_id,
            },
            LabAction::PutAway { tool_item_id } => Self::PutAway { tool_item_id },
            LabAction::Reset => Self::Reset,
            LabAction::ToggleBurner { burner_item_id } => Self::ToggleBurner { burner_item_id },
            LabAction::SelectMode { mode } => Self::SelectMode { mode },
        }
    }
}

impl From<chemlab_core::Action> for LabAction {
    fn from(action: chemlab_core::Action) -> Self {
        match action {
            chemlab_core::Action::UseTool {
                tool_item_id,
                target_item_id,
            } => Self::UseTool {
                tool_item_id,
                target_item_id,
            },
            chemlab_core::Action::Pour {
                source_item_id,
                target_item_id,
            } => Self::Pour {
                source_item_id,
                target_item_id,
            },
            chemlab_core::Action::PutAway { tool_item_id } => Self::PutAway { tool_item_id },
            chemlab_core::Action::Reset => Self::Reset,
            chemlab_core::Action::ToggleBurner { burner_item_id } => {
                Self::ToggleBurner { burner_item_id }
            }
            chemlab_core::Action::SelectMode { mode } => Self::SelectMode { mode },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composition_entry_round_trips() {
        let core = chemlab_core::CompositionEntry {
            substance_id: "nacl".into(),
            phase: "solid".into(),
            amount_ml: None,
            amount_scoop: Some(1),
            amount_g: Some(0.2),
            amount_mol: None,
        };
        let wire: CompositionEntry = core.clone().into();
        let back: chemlab_core::CompositionEntry = wire.into();
        assert_eq!(back, core);
    }
}
