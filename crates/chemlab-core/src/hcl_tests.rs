    use super::*;

    fn aq(substance_id: &str, amount_mol: f64) -> CompositionEntry {
        CompositionEntry {
            substance_id: substance_id.into(),
            phase: "aqueous".into(),
            amount_ml: None,
            amount_scoop: None,
            amount_g: None,
            amount_mol: Some(amount_mol),
        }
    }

    fn water(amount_ml: f64) -> CompositionEntry {
        CompositionEntry {
            substance_id: "water".into(),
            phase: "liquid".into(),
            amount_ml: Some(amount_ml),
            amount_scoop: None,
            amount_g: None,
            amount_mol: None,
        }
    }

    #[test]
    fn stock_moles_and_volume_match_locked_numbers() {
        assert!((HCL_STOCK_HCL_MOLES - 3.447 / 36.46).abs() < 1e-12);
        assert!((HCL_STOCK_TOTAL_MASS_G / HCL_STOCK_DENSITY_G_PER_ML - 10.0).abs() < 1e-9);
        let dens = hcl_aq_density_g_per_ml(0.30);
        assert!((dens - 1.149).abs() < 1e-9);
        let stock = vec![
            water(HCL_STOCK_WATER_MASS_G),
            aq("h+", HCL_STOCK_HCL_MOLES),
            aq("cl-", HCL_STOCK_HCL_MOLES),
        ];
        let v = solution_volume_ml_of_entries(&stock);
        assert!(
            (v - HCL_STOCK_CAPACITY_ML).abs() < 1e-9,
            "Φ_V stock volume must stay 10.00 ml, got {v}"
        );
        assert!(
            (PHI_V_HCL_ML_PER_MOL - 20.70).abs() < 0.01,
            "Φ_HCl should be ≈20.70 ml/mol, got {PHI_V_HCL_ML_PER_MOL}"
        );
        // Continuity: Φ_V stock matches the old ρ path within 1e-3 ml.
        let rho_v = HCL_STOCK_TOTAL_MASS_G / dens;
        assert!((v - rho_v).abs() < 1e-3);
    }

    #[test]
    fn saturated_nacl_adds_apparent_molar_volume() {
        // ~35.89 g / 100 g H₂O at 20 °C → V ≈ 113.5 ml for 100 ml water.
        let n_nacl = 35.89 / 58.44;
        let entries = vec![water(100.0), aq("na+", n_nacl), aq("cl-", n_nacl)];
        let v = solution_volume_ml_of_entries(&entries);
        assert!(
            (v - 113.5).abs() < 1.0,
            "sat NaCl V expected ≈113.5 ml, got {v}"
        );
    }

    #[test]
    fn naoh_and_mixed_electrolytes_add_phi_v() {
        let n_oh = 0.1;
        let naoh_only = vec![water(100.0), aq("na+", n_oh), aq("oh-", n_oh)];
        let v_naoh = solution_volume_ml_of_entries(&naoh_only);
        assert!(
            v_naoh > 100.0 + 1e-9,
            "NaOH must increase solution volume above water ml"
        );
        assert!((v_naoh - (100.0 + n_oh * PHI_V_NAOH_ML_PER_MOL)).abs() < 1e-9);

        let n_h = 0.05;
        let n_nacl = 0.1;
        let mixed = vec![
            water(50.0),
            aq("h+", n_h),
            aq("na+", n_nacl),
            aq("cl-", n_h + n_nacl),
        ];
        let expected = 50.0 + n_h * PHI_V_HCL_ML_PER_MOL + n_nacl * PHI_V_NACL_ML_PER_MOL;
        assert!((solution_volume_ml_of_entries(&mixed) - expected).abs() < 1e-9);
    }

    #[test]
    fn dilution_of_stock_into_water_is_exothermic() {
        let dest = HclInventory {
            water_ml: 10.0,
            n_h: 0.0,
        };
        let added = HclInventory {
            water_ml: HCL_STOCK_WATER_MASS_G / 10.0,
            n_h: HCL_STOCK_HCL_MOLES / 10.0,
        };
        let after = HclInventory {
            water_ml: dest.water_ml + added.water_ml,
            n_h: dest.n_h + added.n_h,
        };
        let q = hcl_dilution_heat_j(dest, added, after);
        assert!(q < 0.0, "expected exothermic dilution, Q={q}");
    }

    #[test]
    fn vapor_bias_drives_liquid_toward_azeotrope() {
        let lean = azeotrope_vapor_w_hcl(0.10);
        assert!(lean < 0.10);
        let rich = azeotrope_vapor_w_hcl(0.30);
        assert!(rich > 0.30);
        let at = azeotrope_vapor_w_hcl(HCL_AZEOTROPE_W_W);
        assert!((at - HCL_AZEOTROPE_W_W).abs() < 1e-6);
    }

    #[test]
    fn vapor_bias_shrinks_toward_azeotrope() {
        let far_lean = (azeotrope_vapor_w_hcl(0.05) - 0.05).abs();
        let near_lean = (azeotrope_vapor_w_hcl(0.18) - 0.18).abs();
        assert!(
            near_lean < far_lean,
            "lean |y-w| should shrink near az: far={far_lean} near={near_lean}"
        );
        let far_rich = (azeotrope_vapor_w_hcl(0.35) - 0.35).abs();
        let near_rich = (azeotrope_vapor_w_hcl(0.22) - 0.22).abs();
        assert!(
            near_rich < far_rich,
            "rich |y-w| should shrink near az: far={far_rich} near={near_rich}"
        );
    }

    #[test]
    fn hcl_boil_temperature_peaks_at_azeotrope() {
        use crate::scene::BOILING_TEMPERATURE_C;
        let t0 = hcl_boil_temperature_c(0.0);
        assert!(
            (t0 - BOILING_TEMPERATURE_C).abs() < 0.05,
            "w=0 should match pure-water Antoine boil: {t0} vs {BOILING_TEMPERATURE_C}"
        );
        let t_az = hcl_boil_temperature_c(HCL_AZEOTROPE_W_W);
        assert!(
            (t_az - HCL_AZEOTROPE_BOIL_C).abs() < 1e-9,
            "azeotrope knot must be {HCL_AZEOTROPE_BOIL_C}, got {t_az}"
        );
        let t_stock = hcl_boil_temperature_c(0.30);
        assert!(
            (t_stock - 105.0).abs() < 0.05,
            "stock ~30% w/w should boil near 105 °C, got {t_stock}"
        );
        let t_dilute = hcl_boil_temperature_c(0.05);
        assert!(
            (100.0..=102.0).contains(&t_dilute),
            "dilute ~5% should boil ~100–102 °C, got {t_dilute}"
        );
        assert!(t_az > t_stock && t_az > t_dilute && t_az > t0);
    }

    #[test]
    fn ph_of_stock_is_strongly_acidic() {
        let entries = vec![
            CompositionEntry {
                substance_id: "water".into(),
                phase: "liquid".into(),
                amount_ml: Some(HCL_STOCK_WATER_MASS_G),
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
                amount_mol: Some(HCL_STOCK_HCL_MOLES),
            },
            CompositionEntry {
                substance_id: "cl-".into(),
                phase: "aqueous".into(),
                amount_ml: None,
                amount_scoop: None,
                amount_g: None,
                amount_mol: Some(HCL_STOCK_HCL_MOLES),
            },
        ];
        let ph = ph_of_entries(&entries).expect("stock has pH");
        // ~9.45 M → pH ≈ -0.97
        assert!(ph < 0.0);
        assert!(ph > -1.5);
    }
