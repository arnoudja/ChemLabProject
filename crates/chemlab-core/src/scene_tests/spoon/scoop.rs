//! Spoon scoop / return / stock / put-away scene tests.

use super::super::super::*;
use super::super::helpers::*;

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

#[test]
fn use_tool_spoon_returns_evaporated_nacl_to_emptied_stock() {
    // Tongs can dump solids into an emptied exact-type stock; spoon must too after
    // evaporating aqueous NaCl back to dry solid (stock composition is empty).
    let mut scene = initial_bench_scene("lab-test");
    use_tongs(&mut scene, "beaker-nacl").unwrap();
    use_tongs(&mut scene, "dish-1").unwrap();
    put_tongs_away(&mut scene).unwrap();
    assert_eq!(solid_g(item(&scene, "beaker-nacl"), "nacl"), 0.0);
    assert!(item(&scene, "beaker-nacl")
        .properties
        .composition
        .iter()
        .all(|c| !(c.phase == "solid" && c.substance_id == "nacl")));

    let dish = scene.items.iter_mut().find(|i| i.id == "dish-1").unwrap();
    dish.properties.temperature_c = Some(20.0);
    dish.properties.composition.insert(
        0,
        CompositionEntry {
            substance_id: "water".into(),
            phase: "liquid".into(),
            amount_ml: Some(5.0),
            amount_scoop: None,
            amount_g: None,
            amount_mol: None,
        },
    );
    crate::solubility::enforce_saturation(dish);

    apply_action(
        &mut scene,
        Action::ToggleBurner {
            burner_item_id: "burner-1".into(),
        },
    )
    .unwrap();
    // Heat-limited boil is ~0.035 ml/s; loop until dry (salt elevates T_boil).
    for _ in 0..10_000 {
        if water_ml(item(&scene, "dish-1")) < 1e-9 {
            break;
        }
        apply_elapsed(&mut scene, 1.0);
    }

    assert!(water_ml(item(&scene, "dish-1")) < 1e-9);
    assert!((solid_g(item(&scene, "dish-1"), "nacl") - 2.0).abs() < 1e-9);

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
    assert!((solid_g(item(&scene, "beaker-nacl"), "nacl") - SPOON_SCOOP_MASS_G).abs() < 1e-12);
    assert!((solid_g(item(&scene, "dish-1"), "nacl") - (2.0 - SPOON_SCOOP_MASS_G)).abs() < 1e-9);
    assert!(scene.last_events.iter().any(|e| e.kind == "returned"));
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
