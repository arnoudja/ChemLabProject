use super::super::*;
use super::helpers::{item, solid_g};
use crate::challenges::{is_completed, CHALLENGES, SEPARATE_NACL_SIO2};

const SEPARATE: &str = "separate-nacl-sio2";

fn separate_scene() -> Scene {
    initial_scene_for_mode("lab-test", SEPARATE).expect("challenge scene")
}

fn select_mode(scene: &mut Scene, mode: &str) -> Result<(), SceneError> {
    apply_action(
        scene,
        Action::SelectMode {
            mode: mode.to_string(),
        },
    )
}

fn fill_stock(scene: &mut Scene, item_id: &str, substance_id: &str, grams: f64) {
    let stock = scene
        .items
        .iter_mut()
        .find(|item| item.id == item_id)
        .expect("stock beaker");
    stock.properties.composition = vec![CompositionEntry {
        substance_id: substance_id.into(),
        phase: "solid".into(),
        amount_ml: None,
        amount_scoop: None,
        amount_g: Some(grams),
        amount_mol: None,
    }];
}

#[test]
fn free_is_the_default_mode_and_is_never_completed() {
    let scene = initial_bench_scene("lab-test");
    assert_eq!(scene.mode, FREE_MODE);
    assert!(is_free_mode(&scene.mode));
    assert!(!is_completed(&scene));
}

#[test]
fn catalog_entry_matches_the_docs_copy() {
    assert_eq!(CHALLENGES.len(), 1);
    let challenge = find_challenge(SEPARATE).expect("catalog entry");
    assert_eq!(challenge, &SEPARATE_NACL_SIO2);
    assert_eq!(challenge.title, "Separate salt from sand");
    assert_eq!(
        challenge.prompt,
        "The previous student accidentally put all the salt and sand in the main beaker, can you please separate them and put them back in their containers?"
    );
    assert_eq!(challenge.done, "Thank you.");
    assert_eq!(find_challenge("nope"), None);
    assert_eq!(find_challenge(FREE_MODE), None);
}

#[test]
fn free_mode_scene_is_the_default_bench() {
    let free = initial_scene_for_mode("lab-test", FREE_MODE).expect("free scene");
    assert_eq!(free, initial_bench_scene("lab-test"));
    assert_eq!(initial_scene_for_mode("lab-test", "nope"), None);
}

#[test]
fn separate_challenge_starts_with_mixed_beaker_and_empty_stocks() {
    let scene = separate_scene();
    assert_eq!(scene.mode, SEPARATE);

    let ids: Vec<_> = scene.items.iter().map(|i| i.id.as_str()).collect();
    assert!(!ids.contains(&"beaker-cacl2"));
    assert!(ids.contains(&"beaker-h2o"));
    assert!(ids.contains(&"beaker-nacl"));
    assert!(ids.contains(&"beaker-sand"));

    let beaker = item(&scene, "beaker-water");
    assert_eq!(solid_g(beaker, "nacl"), 2.0);
    assert_eq!(solid_g(beaker, "sand"), 2.0);
    assert!(!beaker
        .properties
        .composition
        .iter()
        .any(|entry| entry.phase == "liquid"));

    for (stock_id, substance_id) in [("beaker-nacl", "nacl"), ("beaker-sand", "sand")] {
        let stock = item(&scene, stock_id);
        assert_eq!(solid_g(stock, substance_id), 0.0);
        let entry = stock
            .properties
            .composition
            .iter()
            .find(|entry| entry.substance_id == substance_id)
            .expect("stock keeps its species line");
        assert_eq!(entry.amount_scoop, Some(0));
    }

    let distilled = item(&scene, "beaker-h2o");
    assert_eq!(distilled.properties.fill_ml, Some(100.0));

    // Full tool set stays available.
    for tool_id in [
        "spoon-1",
        "pipette-1",
        "tongs-1",
        "filter-paper-1",
        "dish-1",
        "burner-1",
    ] {
        assert!(ids.contains(&tool_id), "missing {tool_id}");
    }
}

#[test]
fn select_mode_switches_into_and_out_of_a_challenge() {
    let mut scene = initial_bench_scene("lab-test");
    scene.version = 7;
    scene.last_applied_unix_ms = Some(1_700_000_000_000);

    select_mode(&mut scene, SEPARATE).unwrap();
    assert_eq!(scene.mode, SEPARATE);
    assert_eq!(scene.version, 7);
    assert_eq!(scene.last_applied_unix_ms, Some(1_700_000_000_000));
    assert_eq!(scene.last_events.len(), 1);
    assert_eq!(scene.last_events[0].kind, "mode_selected");
    assert_eq!(
        scene.last_events[0].message,
        "Started challenge: Separate salt from sand."
    );
    assert!(scene.items.iter().all(|item| item.id != "beaker-cacl2"));

    select_mode(&mut scene, FREE_MODE).unwrap();
    assert_eq!(scene.mode, FREE_MODE);
    assert_eq!(scene.last_events[0].message, "Switched to Free mode.");
    let free = initial_bench_scene("lab-test");
    assert_eq!(scene.items, free.items);
}

#[test]
fn select_mode_rejects_an_unknown_challenge_id() {
    let mut scene = initial_bench_scene("lab-test");
    assert_eq!(
        select_mode(&mut scene, "no-such-challenge"),
        Err(SceneError::UnknownMode)
    );
    assert_eq!(scene.mode, FREE_MODE);
    assert_eq!(scene.items, initial_bench_scene("lab-test").items);
}

#[test]
fn select_mode_hard_resets_work_in_progress() {
    let mut scene = separate_scene();
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();
    assert!(!item(&scene, "spoon-1").properties.holding.is_empty());

    select_mode(&mut scene, SEPARATE).unwrap();
    assert!(item(&scene, "spoon-1").properties.holding.is_empty());
    assert_eq!(solid_g(item(&scene, "beaker-water"), "nacl"), 2.0);
}

#[test]
fn reset_rebuilds_the_current_mode_not_free() {
    let mut scene = separate_scene();
    scene.version = 3;
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-water".into(),
        },
    )
    .unwrap();

    apply_action(&mut scene, Action::Reset).unwrap();

    assert_eq!(scene.mode, SEPARATE);
    assert_eq!(scene.version, 3);
    assert!(scene.items.iter().all(|item| item.id != "beaker-cacl2"));
    assert_eq!(solid_g(item(&scene, "beaker-water"), "nacl"), 2.0);
    assert_eq!(solid_g(item(&scene, "beaker-nacl"), "nacl"), 0.0);
    assert_eq!(scene.last_events[0].kind, "reset");
}

#[test]
fn reset_from_an_unknown_persisted_mode_falls_back_to_free() {
    let mut scene = initial_bench_scene("lab-test");
    scene.mode = "retired-challenge".into();

    apply_action(&mut scene, Action::Reset).unwrap();

    assert_eq!(scene.mode, FREE_MODE);
    assert!(scene.items.iter().any(|item| item.id == "beaker-cacl2"));
}

#[test]
fn ensure_default_bench_items_keeps_a_challenge_layout() {
    let mut scene = separate_scene();
    scene.items.retain(|item| item.id != "tongs-1");

    ensure_default_bench_items(&mut scene);

    assert!(scene.items.iter().any(|item| item.id == "tongs-1"));
    assert!(scene.items.iter().all(|item| item.id != "beaker-cacl2"));
    assert_eq!(solid_g(item(&scene, "beaker-nacl"), "nacl"), 0.0);
}

#[test]
fn challenge_completes_only_when_both_stocks_hold_their_species() {
    let mut scene = separate_scene();
    assert!(!is_completed(&scene));

    fill_stock(&mut scene, "beaker-nacl", "nacl", 2.0);
    assert!(!is_completed(&scene), "sand is still missing");

    fill_stock(&mut scene, "beaker-sand", "sand", 1.8);
    assert!(!is_completed(&scene), "0.2 g of sand short");

    fill_stock(&mut scene, "beaker-sand", "sand", 2.0);
    assert!(is_completed(&scene));
}

#[test]
fn challenge_ignores_the_wrong_species_and_tolerates_float_dust() {
    let mut scene = separate_scene();
    fill_stock(&mut scene, "beaker-nacl", "nacl", 2.0);
    fill_stock(&mut scene, "beaker-sand", "nacl", 2.0);
    assert!(!is_completed(&scene));

    fill_stock(&mut scene, "beaker-sand", "sand", 10.0 * 0.2);
    assert!(is_completed(&scene));

    // Completion is derived, so taking a scoop back out un-wins the challenge.
    apply_action(
        &mut scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-sand".into(),
        },
    )
    .unwrap();
    assert!(!is_completed(&scene));
}

#[test]
fn solved_by_playing_the_challenge_through() {
    let mut scene = separate_scene();

    // Wet the mixture with the distilled-water stock, then filter off the sand.
    use_tongs_pour(&mut scene, "beaker-h2o", "beaker-water");
    use_tongs_pour(&mut scene, "beaker-water", "filter-paper-1");

    // Boil the filtrate dry in the dish, one dish-full at a time.
    while liquid_ml(&scene, "beaker-filtrate") > 1e-9 {
        use_tongs_pour(&mut scene, "beaker-filtrate", "dish-1");
        boil_dish_dry(&mut scene);
    }

    move_all_solids(&mut scene, "dish-1", "beaker-nacl");
    move_all_solids(&mut scene, "filter-paper-1", "beaker-sand");

    assert!(
        (solid_g(item(&scene, "beaker-nacl"), "nacl") - 2.0).abs() < 1e-6,
        "recovered NaCl {}",
        solid_g(item(&scene, "beaker-nacl"), "nacl")
    );
    assert!(is_completed(&scene));
}

fn liquid_ml(scene: &Scene, item_id: &str) -> f64 {
    crate::solubility::liquid_water_ml(item(scene, item_id))
}

fn use_tongs_pour(scene: &mut Scene, source_id: &str, target_id: &str) {
    for target in [source_id, target_id] {
        apply_action(
            scene,
            Action::UseTool {
                tool_item_id: "tongs-1".into(),
                target_item_id: target.into(),
            },
        )
        .unwrap();
    }
    apply_action(
        scene,
        Action::PutAway {
            tool_item_id: "tongs-1".into(),
        },
    )
    .unwrap();
}

fn boil_dish_dry(scene: &mut Scene) {
    apply_action(
        scene,
        Action::ToggleBurner {
            burner_item_id: "burner-1".into(),
        },
    )
    .unwrap();
    for _ in 0..1000 {
        apply_elapsed(scene, 1.0);
        if liquid_ml(scene, "dish-1") <= 0.0 {
            return;
        }
    }
    panic!("dish never boiled dry");
}

/// Spoon every gram of solid out of `source_id` and into its stock beaker.
fn move_all_solids(scene: &mut Scene, source_id: &str, stock_id: &str) {
    for _ in 0..200 {
        if apply_action(
            scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: source_id.into(),
            },
        )
        .is_err()
        {
            return;
        }
        apply_action(
            scene,
            Action::UseTool {
                tool_item_id: "spoon-1".into(),
                target_item_id: stock_id.into(),
            },
        )
        .unwrap();
    }
    panic!("solids never ran out in {source_id}");
}
