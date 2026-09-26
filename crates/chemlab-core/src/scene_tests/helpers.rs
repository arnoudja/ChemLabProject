use super::super::*;

pub(super) fn item<'a>(scene: &'a Scene, id: &str) -> &'a SceneItem {
    scene
        .items
        .iter()
        .find(|i| i.id == id)
        .unwrap_or_else(|| panic!("missing item {id}"))
}

/// Tests that pour, pipette, or dissolve into the main beaker start from a filled vessel.
pub(super) const FILLED_MAIN_BEAKER_ML: f64 = 200.0;

pub(super) fn fill_main_beaker(scene: &mut Scene, amount_ml: f64) {
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

pub(super) fn bench_with_water(lab_id: &str) -> Scene {
    let mut scene = initial_bench_scene(lab_id);
    fill_main_beaker(&mut scene, FILLED_MAIN_BEAKER_ML);
    scene
}

#[test]
fn initial_bench_scene_has_fourteen_items_with_water_hcl_naoh_and_evaporation_bench() {
    let scene = initial_bench_scene("lab-test");
    assert_eq!(scene.lab_id, "lab-test");
    assert_eq!(scene.temperature_c, 20.0);
    assert_eq!(scene.version, 0);
    assert!(scene.last_events.is_empty());
    assert_eq!(scene.last_applied_unix_ms, None);
    assert_eq!(scene.items.len(), 14);

    let ids: Vec<_> = scene.items.iter().map(|i| i.id.as_str()).collect();
    assert!(ids.contains(&"spoon-1"));
    assert!(ids.contains(&"beaker-h2o"));
    assert!(ids.contains(&"beaker-hcl"));
    assert!(ids.contains(&"beaker-nacl"));
    assert!(ids.contains(&"beaker-naoh"));
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

    let hcl = item(&scene, "beaker-hcl");
    assert_eq!(hcl.label, "Hydrochloric acid (30%)");
    assert_eq!(
        hcl.properties.volume_ml,
        Some(crate::hcl::HCL_STOCK_CAPACITY_ML)
    );
    assert!((crate::hcl::solution_volume_ml(hcl) - 10.0).abs() < 1e-6);
    assert!((water_ml(hcl) - crate::hcl::HCL_STOCK_WATER_MASS_G).abs() < 1e-9);
    assert!((aqueous_mol(hcl, "h+") - crate::hcl::HCL_STOCK_HCL_MOLES).abs() < 1e-12);
    assert!((aqueous_mol(hcl, "cl-") - crate::hcl::HCL_STOCK_HCL_MOLES).abs() < 1e-12);
    let ph = crate::hcl::ph_of_item(hcl).expect("stock pH");
    assert!(ph < 0.0);

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

    let naoh = item(&scene, "beaker-naoh");
    assert_eq!(naoh.label, "Sodium hydroxide");
    assert!(naoh.properties.composition.iter().any(|c| {
        c.substance_id == "naoh"
            && c.phase == "solid"
            && c.amount_g == Some(2.0)
            && c.amount_scoop == Some(10)
    }));

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

pub(super) fn aqueous_mol(item: &SceneItem, substance_id: &str) -> f64 {
    item.properties
        .composition
        .iter()
        .find(|c| c.substance_id == substance_id && c.phase == "aqueous")
        .and_then(|c| c.amount_mol)
        .unwrap_or(0.0)
}

pub(super) fn water_ml(item: &SceneItem) -> f64 {
    crate::solubility::liquid_water_ml(item)
}

pub(super) fn fill_pipette_from(scene: &mut Scene, target_id: &str) {
    apply_action(
        scene,
        Action::UseTool {
            tool_item_id: "pipette-1".into(),
            target_item_id: target_id.into(),
        },
    )
    .unwrap();
}

pub(super) fn aqueous_mol_holding(item: &SceneItem, substance_id: &str) -> f64 {
    item.properties
        .holding
        .iter()
        .find(|c| c.substance_id == substance_id && c.phase == "aqueous")
        .and_then(|c| c.amount_mol)
        .unwrap_or(0.0)
}

pub(super) fn use_tongs(scene: &mut Scene, target_id: &str) -> Result<(), SceneError> {
    apply_action(
        scene,
        Action::UseTool {
            tool_item_id: "tongs-1".into(),
            target_item_id: target_id.into(),
        },
    )
}

/// Pick up `source_id` with tongs (if not already held), pour into `target_id`, put tongs away.
pub(super) fn use_tongs_pour(scene: &mut Scene, source_id: &str, target_id: &str) {
    for target in [source_id, target_id] {
        apply_action(
            scene,
            Action::UseTool {
                tool_item_id: "tongs-1".into(),
                target_item_id: target.into(),
            },
        )
        .unwrap_or_else(|err| panic!("tongs use on {target}: {err:?}"));
    }
    apply_action(
        scene,
        Action::PutAway {
            tool_item_id: "tongs-1".into(),
        },
    )
    .unwrap();
}

pub(super) fn put_tongs_away(scene: &mut Scene) -> Result<(), SceneError> {
    apply_action(
        scene,
        Action::PutAway {
            tool_item_id: "tongs-1".into(),
        },
    )
}

pub(super) fn solid_g(item: &SceneItem, substance_id: &str) -> f64 {
    item.properties
        .composition
        .iter()
        .find(|c| c.substance_id == substance_id && c.phase == "solid")
        .and_then(|c| c.amount_g)
        .unwrap_or(0.0)
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

pub(super) fn solid(substance_id: &str, grams: f64) -> CompositionEntry {
    CompositionEntry {
        substance_id: substance_id.into(),
        phase: "solid".into(),
        amount_ml: None,
        amount_scoop: None,
        amount_g: Some(grams),
        amount_mol: None,
    }
}

pub(super) fn set_dry_dish_solids(scene: &mut Scene, solids: Vec<CompositionEntry>) {
    let dish = scene.items.iter_mut().find(|i| i.id == "dish-1").unwrap();
    dish.properties.composition = solids;
    crate::solubility::sync_fill_ml(dish);
}

pub(super) fn scoop_dish(scene: &mut Scene) -> Result<(), SceneError> {
    apply_action(
        scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "dish-1".into(),
        },
    )
}

pub(super) fn empty_nacl_stock(scene: &mut Scene) {
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

pub(super) fn scoop_nacl_stock(scene: &mut Scene) -> Result<(), SceneError> {
    apply_action(
        scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "beaker-nacl".into(),
        },
    )
}

pub(super) fn holding_g(item: &SceneItem, substance_id: &str) -> f64 {
    item.properties
        .holding
        .iter()
        .find(|c| c.substance_id == substance_id && c.phase == "solid")
        .and_then(|c| c.amount_g)
        .unwrap_or(0.0)
}

pub(super) fn set_paper_solids(scene: &mut Scene, solids: Vec<CompositionEntry>) {
    let paper = scene
        .items
        .iter_mut()
        .find(|i| i.id == "filter-paper-1")
        .unwrap();
    paper.properties.composition = solids;
}

pub(super) fn scoop_paper(scene: &mut Scene) -> Result<(), SceneError> {
    apply_action(
        scene,
        Action::UseTool {
            tool_item_id: "spoon-1".into(),
            target_item_id: "filter-paper-1".into(),
        },
    )
}

pub(super) fn set_slurry_in_water(scene: &mut Scene) {
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
