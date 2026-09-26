    use super::*;
    use crate::scene::ItemProperties;

    fn dish_at(temperature_c: f64, water_ml: f64) -> SceneItem {
        SceneItem {
            id: "dish-1".into(),
            kind: "evaporation_dish".into(),
            label: "Evaporation dish".into(),
            location: "bench".into(),
            properties: ItemProperties {
                volume_ml: Some(25.0),
                fill_ml: Some(water_ml),
                transparent: Some(true),
                colourless: Some(true),
                temperature_c: Some(temperature_c),
                composition: if water_ml > 0.0 {
                    vec![CompositionEntry {
                        substance_id: "water".into(),
                        phase: "liquid".into(),
                        amount_ml: Some(water_ml),
                        amount_scoop: None,
                        amount_g: None,
                        amount_mol: None,
                    }]
                } else {
                    vec![]
                },
                ..ItemProperties::default()
            },
        }
    }

    fn nacl_s(t: f64) -> f64 {
        solubility_mol_per_l(Salt::Nacl, t)
    }

    fn cacl2_s(t: f64) -> f64 {
        solubility_mol_per_l(Salt::Cacl2, t)
    }

    #[test]
    fn nacl_solubility_interpolates_between_table_points() {
        let s20 = nacl_s(20.0);
        let s40 = nacl_s(40.0);
        let s60 = nacl_s(60.0);
        let s100 = nacl_s(100.0);
        assert!((s20 - 35.89 * 10.0 / NACL_MOLAR_MASS_G_PER_MOL).abs() < 1e-12);
        assert!((s100 - 38.99 * 10.0 / NACL_MOLAR_MASS_G_PER_MOL).abs() < 1e-12);
        assert!((s60 - 37.04 * 10.0 / NACL_MOLAR_MASS_G_PER_MOL).abs() < 1e-12);
        let s50 = nacl_s(50.0);
        assert!((s50 - 0.5 * (s40 + s60)).abs() < 1e-12);
        assert!(s100 > s20);
    }

    #[test]
    fn unsaturated_capacity_is_zero_when_already_at_si1_and_positive_in_pure_water() {
        let water_ml = 10.0;
        let litres = water_ml / 1000.0;
        let t = 20.0;
        let pure_cap = unsaturated_capacity_g(Salt::Nacl, water_ml, 0.0, 0.0, 0.0, 0.0, t);
        let expected = solubility_mol_per_l(Salt::Nacl, t) * litres * NACL_MOLAR_MASS_G_PER_MOL;
        assert!((pure_cap - expected).abs() < 1e-6);
        let sat_mol = solubility_mol_per_l(Salt::Nacl, t) * litres;
        let sat_cap = unsaturated_capacity_g(Salt::Nacl, water_ml, sat_mol, 0.0, 0.0, 0.0, t);
        assert!(sat_cap < 1e-6);
        assert!(
            unsaturated_capacity_g(Salt::Nacl, water_ml, 0.0, 0.0, 0.0, 0.0, 80.0)
                > pure_cap + 1e-4
        );
    }

    #[test]
    fn cacl2_solubility_is_higher_at_100c_than_20c() {
        assert!(solubility_mol_per_l(Salt::Cacl2, 100.0) > solubility_mol_per_l(Salt::Cacl2, 20.0));
    }

    #[test]
    fn excess_nacl_precipitates_and_aqueous_caps_at_saturation() {
        let mut dish = dish_at(20.0, 1.0);
        let max = solubility_mol_per_l(Salt::Nacl, 20.0) * 0.001;
        dish.properties.composition.push(CompositionEntry {
            substance_id: "na+".into(),
            phase: "aqueous".into(),
            amount_ml: None,
            amount_scoop: None,
            amount_g: None,
            amount_mol: Some(max + 0.01),
        });
        dish.properties.composition.push(CompositionEntry {
            substance_id: "cl-".into(),
            phase: "aqueous".into(),
            amount_ml: None,
            amount_scoop: None,
            amount_g: None,
            amount_mol: Some(max + 0.01),
        });
        enforce_saturation(&mut dish);
        let na = aqueous_mol(&dish, "na+");
        assert!((na - max).abs() < 1e-9, "aqueous na+ {na} vs cap {max}");
        let solid = dish
            .properties
            .composition
            .iter()
            .find(|c| c.substance_id == "nacl" && c.phase == "solid")
            .expect("solid nacl");
        assert!((solid.amount_mol.unwrap() - 0.01).abs() < 1e-9);
    }

    #[test]
    fn adding_water_redissolves_solid_nacl_up_to_solubility() {
        let mut dish = dish_at(20.0, 1.0);
        let max_1ml = solubility_mol_per_l(Salt::Nacl, 20.0) * 0.001;
        dish.properties.composition.push(CompositionEntry {
            substance_id: "nacl".into(),
            phase: "solid".into(),
            amount_ml: None,
            amount_scoop: None,
            amount_g: Some(0.02 * NACL_MOLAR_MASS_G_PER_MOL),
            amount_mol: Some(0.02),
        });
        enforce_saturation(&mut dish);
        assert!((aqueous_mol(&dish, "na+") - max_1ml).abs() < 1e-9);

        if let Some(water) = dish
            .properties
            .composition
            .iter_mut()
            .find(|c| c.substance_id == "water")
        {
            water.amount_ml = Some(10.0);
        }
        enforce_saturation(&mut dish);
        let max_10ml = solubility_mol_per_l(Salt::Nacl, 20.0) * 0.010;
        let expected_aq = 0.02_f64.min(max_10ml);
        assert!((aqueous_mol(&dish, "na+") - expected_aq).abs() < 1e-9);
        assert!(
            dish.properties
                .composition
                .iter()
                .all(|c| !(c.substance_id == "nacl" && c.phase == "solid")),
            "10 ml should dissolve 0.02 mol NaCl at 20 °C"
        );
    }

    fn push_aq(item: &mut SceneItem, substance_id: &str, moles: f64) {
        if let Some(existing) = item
            .properties
            .composition
            .iter_mut()
            .find(|c| c.substance_id == substance_id && c.phase == "aqueous")
        {
            existing.amount_mol = Some(existing.amount_mol.unwrap_or(0.0) + moles);
            return;
        }
        item.properties.composition.push(CompositionEntry {
            substance_id: substance_id.into(),
            phase: "aqueous".into(),
            amount_ml: None,
            amount_scoop: None,
            amount_g: None,
            amount_mol: Some(moles),
        });
    }

    fn atom_totals(item: &SceneItem) -> (f64, f64, f64) {
        let na = aqueous_mol(item, "na+") + solid_mol(item, "nacl", NACL_MOLAR_MASS_G_PER_MOL);
        let ca = aqueous_mol(item, "ca2+") + solid_mol(item, "cacl2", CACL2_MOLAR_MASS_G_PER_MOL);
        let cl = aqueous_mol(item, "cl-")
            + solid_mol(item, "nacl", NACL_MOLAR_MASS_G_PER_MOL)
            + 2.0 * solid_mol(item, "cacl2", CACL2_MOLAR_MASS_G_PER_MOL);
        (na, ca, cl)
    }

    fn has_solid(item: &SceneItem, substance_id: &str) -> bool {
        item.properties
            .composition
            .iter()
            .any(|c| c.substance_id == substance_id && c.phase == "solid")
    }

    #[test]
    fn pure_nacl_and_cacl2_match_literature_curves_at_20_and_100() {
        for (salt, t, grams_per_100g, mm) in [
            (Salt::Nacl, 20.0, 35.89, NACL_MOLAR_MASS_G_PER_MOL),
            (Salt::Nacl, 100.0, 38.99, NACL_MOLAR_MASS_G_PER_MOL),
            (Salt::Cacl2, 20.0, 74.5, CACL2_MOLAR_MASS_G_PER_MOL),
            (Salt::Cacl2, 100.0, 159.0, CACL2_MOLAR_MASS_G_PER_MOL),
        ] {
            let expected = grams_per_100g * 10.0 / mm;
            assert!(
                (solubility_mol_per_l(salt, t) - expected).abs() < 1e-12,
                "{salt:?} at {t} °C"
            );
        }

        let mut nacl_only = dish_at(20.0, 1000.0);
        let s_nacl = solubility_mol_per_l(Salt::Nacl, 20.0);
        push_aq(&mut nacl_only, "na+", s_nacl + 0.5);
        push_aq(&mut nacl_only, "cl-", s_nacl + 0.5);
        enforce_saturation(&mut nacl_only);
        assert!((aqueous_mol(&nacl_only, "na+") - s_nacl).abs() < 1e-8);
        assert!((aqueous_mol(&nacl_only, "cl-") - s_nacl).abs() < 1e-8);
        assert!(aqueous_mol(&nacl_only, "ca2+") < 1e-12);

        let mut cacl2_only = dish_at(100.0, 1000.0);
        let s_cacl2 = solubility_mol_per_l(Salt::Cacl2, 100.0);
        push_aq(&mut cacl2_only, "ca2+", s_cacl2 + 0.25);
        push_aq(&mut cacl2_only, "cl-", 2.0 * (s_cacl2 + 0.25));
        enforce_saturation(&mut cacl2_only);
        assert!((aqueous_mol(&cacl2_only, "ca2+") - s_cacl2).abs() < 1e-7);
        assert!((aqueous_mol(&cacl2_only, "cl-") - 2.0 * s_cacl2).abs() < 1e-7);
        assert!(aqueous_mol(&cacl2_only, "na+") < 1e-12);
    }

    #[test]
    fn adding_cacl2_to_near_saturated_nacl_precipitates_extra_nacl() {
        let mut dish = dish_at(20.0, 1000.0);
        let s_nacl = solubility_mol_per_l(Salt::Nacl, 20.0);
        let n_na = 0.98 * s_nacl;
        push_aq(&mut dish, "na+", n_na);
        push_aq(&mut dish, "cl-", n_na);
        enforce_saturation(&mut dish);
        assert!(!has_solid(&dish, "nacl"));
        let na_before = aqueous_mol(&dish, "na+");
        let (na_tot, ca_tot, cl_tot) = atom_totals(&dish);

        push_aq(&mut dish, "ca2+", 1.0);
        push_aq(&mut dish, "cl-", 2.0);
        let (na_tot, ca_tot, cl_tot) = (na_tot, ca_tot + 1.0, cl_tot + 2.0);
        enforce_saturation(&mut dish);

        let na_after = aqueous_mol(&dish, "na+");
        assert!(
            has_solid(&dish, "nacl"),
            "common-ion Cl- from CaCl2 must precipitate extra NaCl"
        );
        assert!(
            na_after < na_before - 0.05,
            "aqueous Na+ {na_after} should drop well below {na_before} (independent model keeps it)"
        );
        assert!(
            na_after < s_nacl - 0.05,
            "mixed NaCl solubility must be below the pure-water cap {s_nacl}, got {na_after}"
        );
        let (na2, ca2, cl2) = atom_totals(&dish);
        assert!((na2 - na_tot).abs() < 1e-9, "Na atoms");
        assert!((ca2 - ca_tot).abs() < 1e-9, "Ca atoms");
        assert!((cl2 - cl_tot).abs() < 1e-9, "Cl atoms");
        assert!(!has_solid(&dish, "sand"));
    }

    #[test]
    fn adding_water_redissolves_mixed_solids_toward_mixed_limit() {
        let mut dish = dish_at(20.0, 2.0);
        let s_nacl = solubility_mol_per_l(Salt::Nacl, 20.0);
        push_aq(&mut dish, "na+", 0.04);
        push_aq(&mut dish, "cl-", 0.10);
        push_aq(&mut dish, "ca2+", 0.03);
        enforce_saturation(&mut dish);
        assert!(has_solid(&dish, "nacl"));
        let solid_before = solid_mol(&dish, "nacl", NACL_MOLAR_MASS_G_PER_MOL);
        let aq_na_before = aqueous_mol(&dish, "na+");
        let (na_tot, ca_tot, cl_tot) = atom_totals(&dish);

        if let Some(water) = dish
            .properties
            .composition
            .iter_mut()
            .find(|c| c.substance_id == "water")
        {
            water.amount_ml = Some(5.0);
        }
        enforce_saturation(&mut dish);

        let solid_after = solid_mol(&dish, "nacl", NACL_MOLAR_MASS_G_PER_MOL);
        let aq_na_after = aqueous_mol(&dish, "na+");
        assert!(
            aq_na_after > aq_na_before + 1e-6,
            "dilution must redissolve toward the mixed limit"
        );
        assert!(solid_after < solid_before - 1e-6);
        let independent_cap_5ml = s_nacl * 0.005;
        assert!(
            has_solid(&dish, "nacl"),
            "5 ml should not dissolve all mixed NaCl"
        );
        assert!(
            aq_na_after < independent_cap_5ml - 1e-6,
            "mixed redissolve limit {aq_na_after} must stay below independent cap {independent_cap_5ml}"
        );
        let (na2, ca2, cl2) = atom_totals(&dish);
        assert!((na2 - na_tot).abs() < 1e-9);
        assert!((ca2 - ca_tot).abs() < 1e-9);
        assert!((cl2 - cl_tot).abs() < 1e-9);
    }

    #[test]
    fn evaporating_mixed_dish_precipitates_nacl_not_both_independent_caps() {
        let t = 100.0;
        let mut dish = dish_at(t, 15.0);
        let s_nacl = solubility_mol_per_l(Salt::Nacl, t);
        let s_cacl2 = solubility_mol_per_l(Salt::Cacl2, t);
        let n_nacl = s_nacl * 0.003;
        let n_cacl2 = s_cacl2 * 0.003;
        push_aq(&mut dish, "na+", n_nacl);
        push_aq(&mut dish, "ca2+", n_cacl2);
        push_aq(&mut dish, "cl-", n_nacl + 2.0 * n_cacl2);
        enforce_saturation(&mut dish);
        assert!(!has_solid(&dish, "nacl"));
        assert!(!has_solid(&dish, "cacl2"));

        if let Some(water) = dish
            .properties
            .composition
            .iter_mut()
            .find(|c| c.substance_id == "water")
        {
            water.amount_ml = Some(2.0);
        }
        let (na_tot, ca_tot, cl_tot) = atom_totals(&dish);
        enforce_saturation(&mut dish);

        let na = aqueous_mol(&dish, "na+");
        let ca = aqueous_mol(&dish, "ca2+");
        let ind_nacl = s_nacl * 0.002;
        let ind_cacl2 = s_cacl2 * 0.002;
        assert!(
            has_solid(&dish, "nacl"),
            "NaCl must crash out on evaporation"
        );
        assert!(
            na < ind_nacl - 1e-6,
            "common ion must keep aqueous Na+ {na} below the independent NaCl cap {ind_nacl}"
        );
        let both_at_independent_caps =
            (na - ind_nacl).abs() < 1e-6 && (ca - ind_cacl2).abs() < 1e-6;
        assert!(
            !both_at_independent_caps,
            "must not park both salts at their independent T caps"
        );
        let (na2, ca2, cl2) = atom_totals(&dish);
        assert!((na2 - na_tot).abs() < 1e-9);
        assert!((ca2 - ca_tot).abs() < 1e-9);
        assert!((cl2 - cl_tot).abs() < 1e-9);
    }

    #[test]
    fn sio2_stays_out_of_the_ion_balance() {
        let mut dish = dish_at(20.0, 1.0);
        dish.properties.composition.push(CompositionEntry {
            substance_id: "sand".into(),
            phase: "solid".into(),
            amount_ml: None,
            amount_scoop: Some(1),
            amount_g: Some(0.2),
            amount_mol: None,
        });
        push_aq(&mut dish, "na+", 0.02);
        push_aq(&mut dish, "cl-", 0.02);
        enforce_saturation(&mut dish);
        let sand = dish
            .properties
            .composition
            .iter()
            .find(|c| c.substance_id == "sand" && c.phase == "solid")
            .expect("SiO2 remains");
        assert!((sand.amount_g.unwrap() - 0.2).abs() < 1e-12);
        assert_eq!(sand.amount_scoop, Some(1));
        assert!(aqueous_mol(&dish, "sand") < 1e-12);
    }

    #[test]
    fn mixed_equilibrium_runs_on_a_water_beaker() {
        let mut beaker = dish_at(20.0, 1000.0);
        beaker.id = "beaker-water".into();
        beaker.kind = "beaker".into();
        beaker.label = "Water".into();
        let s_nacl = solubility_mol_per_l(Salt::Nacl, 20.0);
        push_aq(&mut beaker, "na+", 0.98 * s_nacl);
        push_aq(&mut beaker, "cl-", 0.98 * s_nacl);
        push_aq(&mut beaker, "ca2+", 1.0);
        push_aq(&mut beaker, "cl-", 2.0);
        enforce_saturation(&mut beaker);
        assert!(has_solid(&beaker, "nacl"));
        assert!(aqueous_mol(&beaker, "na+") < 0.98 * s_nacl - 0.05);
    }

    #[test]
    fn solubility_clamps_outside_table_range() {
        assert!((nacl_s(0.0) - nacl_s(20.0)).abs() < 1e-12);
        assert!((nacl_s(120.0) - nacl_s(100.0)).abs() < 1e-12);
        assert!((cacl2_s(-10.0) - cacl2_s(20.0)).abs() < 1e-12);
    }

    #[test]
    fn dry_dish_dumps_ions_to_solids_and_stock_solids_stay_put() {
        let mut dish = dish_at(20.0, 0.0);
        push_aq(&mut dish, "na+", 0.01);
        push_aq(&mut dish, "cl-", 0.01);
        enforce_saturation(&mut dish);
        assert!(aqueous_mol(&dish, "na+") < 1e-12);
        assert!(has_solid(&dish, "nacl"));
        assert!((solid_mol(&dish, "nacl", NACL_MOLAR_MASS_G_PER_MOL) - 0.01).abs() < 1e-12);

        let mut stock = SceneItem {
            id: "beaker-nacl".into(),
            kind: "beaker".into(),
            label: "Sodium chloride".into(),
            location: "bench".into(),
            properties: ItemProperties {
                composition: vec![CompositionEntry {
                    substance_id: "nacl".into(),
                    phase: "solid".into(),
                    amount_ml: None,
                    amount_scoop: Some(9),
                    amount_g: Some(1.8),
                    amount_mol: None,
                }],
                ..ItemProperties::default()
            },
        };
        enforce_saturation(&mut stock);
        let solid = stock
            .properties
            .composition
            .iter()
            .find(|c| c.substance_id == "nacl")
            .expect("stock solid");
        assert_eq!(solid.amount_scoop, Some(9));
        assert!((solid.amount_g.unwrap() - 1.8).abs() < 1e-12);
    }

    #[test]
    fn naoh_na_plus_does_not_invent_chloride() {
        let mut beaker = dish_at(20.0, 100.0);
        beaker.id = "beaker-water".into();
        beaker.kind = "beaker".into();
        // Dissolved NaOH: na+ + oh- only — must not precipitate NaCl / invent cl-.
        push_aq(&mut beaker, "na+", 0.05);
        push_aq(&mut beaker, "oh-", 0.05);
        enforce_saturation(&mut beaker);
        assert!((aqueous_mol(&beaker, "na+") - 0.05).abs() < 1e-12);
        assert!((aqueous_mol(&beaker, "oh-") - 0.05).abs() < 1e-12);
        assert!(aqueous_mol(&beaker, "cl-") < 1e-12);
        assert!(!has_solid(&beaker, "nacl"));
    }

    #[test]
    fn soft_cap_ionic_strength_stays_below_i_star_and_varies_at_high_i() {
        assert!((effective_ionic_strength(6.0) - 2.0).abs() < 1e-12);
        assert!((effective_ionic_strength(15.0) - 2.5).abs() < 1e-12);
        let a = debye_huckel_a(20.0);
        let g1 = log10_gamma(1, 1.0, a);
        let g6 = log10_gamma(1, 6.0, a);
        let g15 = log10_gamma(1, 15.0, a);
        assert!(g1.is_finite() && g6.is_finite() && g15.is_finite());
        // Soft-cap keeps γ moving past I=1 (hard cap would freeze g6 == g1).
        assert!((g6 - g1).abs() > 1e-6);
        assert!((g15 - g6).abs() > 1e-6);
    }

    #[test]
    fn oh_contributes_to_ionic_strength_without_inventing_nacl() {
        let mix_salt = Mixture::from_moles(0.5, 0.0, 0.0, 0.0, 1.0);
        let mix_with_oh = Mixture::from_moles(0.5, 0.0, 0.0, 0.2, 1.0);
        assert!((mix_salt.ionic_strength() - 0.5).abs() < 1e-12);
        assert!((mix_with_oh.ionic_strength() - 0.7).abs() < 1e-12);

        let mut beaker = dish_at(20.0, 1000.0);
        let s_nacl = solubility_mol_per_l(Salt::Nacl, 20.0);
        // Near-sat NaCl plus NaOH: OH raises I but must not invent Cl⁻ / solid.
        push_aq(&mut beaker, "na+", 0.5 * s_nacl + 0.2);
        push_aq(&mut beaker, "cl-", 0.5 * s_nacl);
        push_aq(&mut beaker, "oh-", 0.2);
        enforce_saturation(&mut beaker);
        assert!(!has_solid(&beaker, "nacl"));
        assert!((aqueous_mol(&beaker, "oh-") - 0.2).abs() < 1e-12);
        assert!((aqueous_mol(&beaker, "cl-") - 0.5 * s_nacl).abs() < 1e-8);
        assert!((aqueous_mol(&beaker, "na+") - (0.5 * s_nacl + 0.2)).abs() < 1e-8);
    }
