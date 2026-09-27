//! Free-mode pure liquid H₂SO₄ stock: density, ionization / reformation,
//! dilution enthalpy, put-back.

use crate::composition::{aqueous_mol_entries, liquid_water_ml_entries};
use crate::scene::{CompositionEntry, SceneItem, WATER_MOLAR_MASS_G_PER_MOL};

const AMOUNT_EPS: f64 = 1e-12;

/// H₂SO₄ stock beaker capacity (ml of **liquid acid**).
pub const H2SO4_STOCK_CAPACITY_ML: f64 = 10.00;

/// Literature density of ~98% w/w H₂SO₄ at room temperature (g/ml).
pub const H2SO4_STOCK_DENSITY_G_PER_ML: f64 = 1.83;

/// Stock mass fraction (~98% w/w “oil of vitriol”).
pub const H2SO4_STOCK_W_W: f64 = 0.98;

/// Total mass of the filled Free-mode stock (g): 10.00 ml × 1.83 g/ml.
pub const H2SO4_STOCK_TOTAL_MASS_G: f64 = H2SO4_STOCK_CAPACITY_ML * H2SO4_STOCK_DENSITY_G_PER_ML;

/// H₂SO₄ mass in the filled stock (g): 98% w/w of [`H2SO4_STOCK_TOTAL_MASS_G`].
pub const H2SO4_STOCK_H2SO4_MASS_G: f64 = H2SO4_STOCK_TOTAL_MASS_G * H2SO4_STOCK_W_W;

/// Molar mass of H₂SO₄ (g/mol).
pub const H2SO4_MOLAR_MASS_G_PER_MOL: f64 = 98.079;

/// Moles of H₂SO₄ in the filled Free-mode stock: `(V·ρ·w)/M`.
pub const H2SO4_STOCK_H2SO4_MOLES: f64 = H2SO4_STOCK_H2SO4_MASS_G / H2SO4_MOLAR_MASS_G_PER_MOL;

/// Tolerance on put-back purity (relative mole / volume drift).
pub const H2SO4_STOCK_PUTBACK_EPS: f64 = 0.01;

/// Water:H₂SO₄ mole ratio at stock ~98% w/w.
///
/// At or below this ratio, free aqueous acid reforms to liquid `h2so4`, and
/// liquid acid does not ionize (reciprocal of the dilution gate).
pub const H2SO4_REFORM_WATER_PER_ACID: f64 = (H2SO4_MOLAR_MASS_G_PER_MOL * (1.0 - H2SO4_STOCK_W_W)
    / H2SO4_STOCK_W_W)
    / WATER_MOLAR_MASS_G_PER_MOL;

/// Liquid `h2so4` volume (ml) in a composition list.
pub fn liquid_h2so4_ml_entries(entries: &[CompositionEntry]) -> f64 {
    entries
        .iter()
        .find(|c| c.substance_id == "h2so4" && c.phase == "liquid")
        .map(|c| {
            c.amount_ml
                .or_else(|| {
                    c.amount_mol
                        .map(|n| n * H2SO4_MOLAR_MASS_G_PER_MOL / H2SO4_STOCK_DENSITY_G_PER_ML)
                })
                .unwrap_or(0.0)
                .max(0.0)
        })
        .unwrap_or(0.0)
}

pub fn liquid_h2so4_ml(item: &SceneItem) -> f64 {
    liquid_h2so4_ml_entries(&item.properties.composition)
}

/// Liquid H₂SO₄ moles from a composition list (stock / undissociated acid).
pub fn liquid_h2so4_mol_entries(entries: &[CompositionEntry]) -> f64 {
    entries
        .iter()
        .find(|c| c.substance_id == "h2so4" && c.phase == "liquid")
        .map(|c| {
            if let Some(n) = c.amount_mol {
                return n.max(0.0);
            }
            if let Some(g) = c.amount_g {
                return (g / H2SO4_MOLAR_MASS_G_PER_MOL).max(0.0);
            }
            if let Some(ml) = c.amount_ml {
                // Stock lock: volume at label density × w/w → moles.
                return (ml * H2SO4_STOCK_DENSITY_G_PER_ML * H2SO4_STOCK_W_W
                    / H2SO4_MOLAR_MASS_G_PER_MOL)
                    .max(0.0);
            }
            0.0
        })
        .unwrap_or(0.0)
}

/// Aqueous H₂SO₄ formula units from **free acid** inventory (not salt bisulfate).
///
/// HCl inventory (`min(max(n_h − n_oh, 0), n_cl)`) is excluded first. Na⁺ needed
/// for leftover Cl⁻ is reserved, then remaining Na⁺ pairs with `hso4-` as NaHSO₄
/// (not free H₂SO₄). Leftover acid `hso4-` plus free `h+` paired with `so4^2-`
/// count as reformable / Φ_V H₂SO₄ (post-Kₐ₂).
pub fn aqueous_h2so4_moles_entries(entries: &[CompositionEntry]) -> f64 {
    let n_h = aqueous_mol_entries(entries, "h+");
    let n_oh = aqueous_mol_entries(entries, "oh-");
    let n_na = aqueous_mol_entries(entries, "na+");
    let n_cl = aqueous_mol_entries(entries, "cl-");
    let n_so4 = aqueous_mol_entries(entries, "so4^2-");
    let n_hso4 = aqueous_mol_entries(entries, "hso4-");
    let n_h_excess = (n_h - n_oh).max(0.0);
    let n_hcl = n_h_excess.min(n_cl).max(0.0);
    let n_h_acid = (n_h_excess - n_hcl).max(0.0);
    let n_cl_after_hcl = (n_cl - n_hcl).max(0.0);
    let n_naoh = if n_oh > n_h + 1e-9 {
        (n_oh - n_h).max(0.0)
    } else {
        0.0
    };
    let n_na_salt = (n_na - n_naoh).max(0.0);
    let n_na_for_cl = n_na_salt.min(n_cl_after_hcl).max(0.0);
    let n_na_sulfate_budget = (n_na_salt - n_na_for_cl).max(0.0);
    let n_nahso4 = n_na_sulfate_budget.min(n_hso4).max(0.0);
    let n_hso4_acid = (n_hso4 - n_nahso4).max(0.0);
    let n_na_after_nahso4 = (n_na_sulfate_budget - n_nahso4).max(0.0);
    let n_na2so4 = (n_na_after_nahso4 * 0.5).min(n_so4).max(0.0);
    let n_so4_after_salt = (n_so4 - n_na2so4).max(0.0);
    let n_so4_acid = (n_h_acid - n_hso4_acid).max(0.0).min(n_so4_after_salt);
    (n_hso4_acid + n_so4_acid).max(0.0)
}

/// Inventory of water + sulfuric acid (liquid + aqueous) for dilution enthalpy.
#[derive(Debug, Clone, Copy, Default)]
pub struct H2so4Inventory {
    pub water_ml: f64,
    /// Total H₂SO₄ formula units (liquid undissociated + aqueous paired).
    pub n_h2so4: f64,
}

impl H2so4Inventory {
    pub fn from_entries(entries: &[CompositionEntry]) -> Self {
        Self {
            water_ml: liquid_water_ml_entries(entries),
            n_h2so4: liquid_h2so4_mol_entries(entries) + aqueous_h2so4_moles_entries(entries),
        }
    }

    pub fn from_item(item: &SceneItem) -> Self {
        Self::from_entries(&item.properties.composition)
    }

    /// Water:H₂SO₄ mole ratio (∞ when no acid).
    pub fn water_per_h2so4(self) -> f64 {
        if self.n_h2so4 <= AMOUNT_EPS {
            return f64::INFINITY;
        }
        (self.water_ml / WATER_MOLAR_MASS_G_PER_MOL) / self.n_h2so4
    }

    /// Relative enthalpy vs infinite dilution (J). Positive Φ_L at finite r so
    /// dilution (Φ → 0) is exothermic.
    pub fn relative_enthalpy_j(self) -> f64 {
        if self.n_h2so4 <= AMOUNT_EPS {
            return 0.0;
        }
        self.n_h2so4 * h2so4_phi_l_j_per_mol(self.water_per_h2so4())
    }
}

/// Φ_L (J/mol H₂SO₄) vs r = n_water / n_H₂SO₄. Infinite dilution → 0.
///
/// School-magnitude table: concentrated acid dilution is strongly exothermic
/// (larger than HCl). Not a CRC fit — documented approximation.
fn h2so4_phi_l_j_per_mol(r: f64) -> f64 {
    if !r.is_finite() || r >= 1000.0 {
        return 0.0;
    }
    // r ≈ 0 (anhydrous / pure liquid) carries a large relative enthalpy.
    const TABLE: [(f64, f64); 8] = [
        (0.0, 95_000.0),
        (1.0, 75_000.0),
        (2.0, 55_000.0),
        (4.0, 35_000.0),
        (8.0, 20_000.0),
        (20.0, 8_000.0),
        (50.0, 2_500.0),
        (200.0, 500.0),
    ];
    let r = r.max(TABLE[0].0);
    if r >= TABLE[TABLE.len() - 1].0 {
        let (r0, y0) = TABLE[TABLE.len() - 1];
        return y0 * (r0 / r);
    }
    for pair in TABLE.windows(2) {
        let (r0, y0) = pair[0];
        let (r1, y1) = pair[1];
        if r <= r1 {
            let frac = if (r1 - r0).abs() <= AMOUNT_EPS {
                0.0
            } else {
                (r - r0) / (r1 - r0)
            };
            return y0 + frac * (y1 - y0);
        }
    }
    0.0
}

/// Heat absorbed when combining `dest` + `added` into `after` (J). Negative ⇒ exothermic.
pub fn h2so4_dilution_heat_j(
    dest: H2so4Inventory,
    added: H2so4Inventory,
    after: H2so4Inventory,
) -> f64 {
    after.relative_enthalpy_j() - dest.relative_enthalpy_j() - added.relative_enthalpy_j()
}

/// True when entries are only pure liquid H₂SO₄ (stock put-back gate).
pub fn composition_is_stock_h2so4(entries: &[CompositionEntry]) -> bool {
    let mut n_mol = 0.0;
    let mut ml = 0.0;
    let mut saw = false;
    for entry in entries {
        let has_amt = entry.amount_ml.unwrap_or(0.0) > AMOUNT_EPS
            || entry.amount_g.unwrap_or(0.0) > AMOUNT_EPS
            || entry.amount_mol.unwrap_or(0.0) > AMOUNT_EPS
            || entry.amount_scoop.unwrap_or(0) > 0;
        if !has_amt {
            continue;
        }
        if entry.substance_id == "h2so4" && entry.phase == "liquid" {
            saw = true;
            n_mol += liquid_h2so4_mol_entries(std::slice::from_ref(entry));
            ml += liquid_h2so4_ml_entries(std::slice::from_ref(entry));
            continue;
        }
        return false;
    }
    if !saw || n_mol <= AMOUNT_EPS || ml <= AMOUNT_EPS {
        return false;
    }
    // Concentration lock: moles per ml match stock (±ε relative).
    let stock_m_per_ml = H2SO4_STOCK_H2SO4_MOLES / H2SO4_STOCK_CAPACITY_ML;
    let m_per_ml = n_mol / ml;
    (m_per_ml - stock_m_per_ml).abs() / stock_m_per_ml <= H2SO4_STOCK_PUTBACK_EPS
}

/// Liquid `h2so4` volume (ml) authored at the stock density × w/w lock for `n` moles.
pub fn liquid_h2so4_ml_from_moles(n: f64) -> f64 {
    if n <= AMOUNT_EPS {
        return 0.0;
    }
    n * H2SO4_MOLAR_MASS_G_PER_MOL / (H2SO4_STOCK_DENSITY_G_PER_ML * H2SO4_STOCK_W_W)
}

fn author_liquid_h2so4_moles(item: &mut SceneItem, n: f64) {
    if n <= AMOUNT_EPS {
        return;
    }
    let add_ml = liquid_h2so4_ml_from_moles(n);
    let add_g = n * H2SO4_MOLAR_MASS_G_PER_MOL;
    if let Some(existing) = item
        .properties
        .composition
        .iter_mut()
        .find(|c| c.substance_id == "h2so4" && c.phase == "liquid")
    {
        existing.amount_ml = Some(existing.amount_ml.unwrap_or(0.0) + add_ml);
        existing.amount_mol = Some(existing.amount_mol.unwrap_or(0.0) + n);
        existing.amount_g = Some(existing.amount_g.unwrap_or(0.0) + add_g);
        return;
    }
    item.properties.composition.push(CompositionEntry {
        substance_id: "h2so4".into(),
        phase: "liquid".into(),
        amount_ml: Some(add_ml),
        amount_scoop: None,
        amount_g: Some(add_g),
        amount_mol: Some(n),
    });
}

/// Convert all liquid `h2so4` to aqueous `H⁺ + HSO₄⁻` when the water:acid mole
/// ratio is **above** [`H2SO4_REFORM_WATER_PER_ACID`] (~98% w/w stock).
///
/// The strong first proton is authored here; the weak second step
/// (`HSO₄⁻ ⇌ H⁺ + SO₄²⁻`, school Kₐ₂) is solved by
/// [`crate::acid_base::speciate_aqueous_acid_base`].
///
/// Returns moles of H₂SO₄ ionized (for dilution-heat callers that already mixed).
pub fn ionize_liquid_h2so4_in_water(item: &mut SceneItem) -> f64 {
    let n = liquid_h2so4_mol_entries(&item.properties.composition);
    if n <= AMOUNT_EPS {
        return 0.0;
    }
    let inv = H2so4Inventory::from_item(item);
    if inv.water_per_h2so4() <= H2SO4_REFORM_WATER_PER_ACID {
        return 0.0;
    }
    item.properties
        .composition
        .retain(|c| !(c.substance_id == "h2so4" && c.phase == "liquid"));
    crate::composition::set_aqueous_mol(
        item,
        "h+",
        crate::composition::aqueous_mol(item, "h+") + n,
    );
    crate::composition::set_aqueous_mol(
        item,
        "hso4-",
        crate::composition::aqueous_mol(item, "hso4-") + n,
    );
    n
}

/// Inverse of [`ionize_liquid_h2so4_in_water`]: reform free aqueous sulfuric
/// (HSO₄⁻ + acid SO₄²⁻) to liquid `h2so4` when the water:acid mole ratio is
/// at or below [`H2SO4_REFORM_WATER_PER_ACID`].
///
/// HCl inventory (`min(max(n_h − n_oh, 0), n_cl)`) is left aqueous. Call **after** SI so salt
/// sulfates claim SO₄ first.
///
/// Returns moles of H₂SO₄ reformed.
pub fn reform_aqueous_h2so4_to_liquid(item: &mut SceneItem) -> f64 {
    let n = aqueous_h2so4_moles_entries(&item.properties.composition);
    if n <= AMOUNT_EPS {
        return 0.0;
    }
    let inv = H2so4Inventory::from_item(item);
    if inv.water_per_h2so4() > H2SO4_REFORM_WATER_PER_ACID {
        return 0.0;
    }
    let n_h = crate::composition::aqueous_mol(item, "h+");
    let n_hso4 = crate::composition::aqueous_mol(item, "hso4-");
    let n_so4 = crate::composition::aqueous_mol(item, "so4^2-");
    let take_hso4 = n_hso4.min(n);
    let take_so4 = (n - take_hso4).min(n_so4);
    // Each formula unit removes two acidic hydrogens (bound HSO₄⁻ + free H⁺).
    let h_from_bound = take_hso4;
    let h_from_free = (2.0 * n - h_from_bound).max(0.0);
    crate::composition::set_aqueous_mol(item, "h+", (n_h - h_from_free).max(0.0));
    crate::composition::set_aqueous_mol(item, "hso4-", (n_hso4 - take_hso4).max(0.0));
    crate::composition::set_aqueous_mol(item, "so4^2-", (n_so4 - take_so4).max(0.0));
    author_liquid_h2so4_moles(item, n);
    n
}

/// Composition row for a filled pure-liquid H₂SO₄ stock.
pub fn stock_h2so4_composition() -> Vec<CompositionEntry> {
    vec![CompositionEntry {
        substance_id: "h2so4".into(),
        phase: "liquid".into(),
        amount_ml: Some(H2SO4_STOCK_CAPACITY_ML),
        amount_scoop: None,
        amount_g: Some(H2SO4_STOCK_H2SO4_MASS_G),
        amount_mol: Some(H2SO4_STOCK_H2SO4_MOLES),
    }]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hcl::{hcl_inventory_moles_entries, solution_volume_ml_of_entries, HclInventory};

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
    fn stock_moles_match_locked_density_ww() {
        let expected = (10.0 * 1.83 * 0.98) / 98.079;
        assert!((H2SO4_STOCK_H2SO4_MOLES - expected).abs() < 1e-12);
        let stock = stock_h2so4_composition();
        assert!((solution_volume_ml_of_entries(&stock) - 10.0).abs() < 1e-9);
        assert!(composition_is_stock_h2so4(&stock));
    }

    #[test]
    fn sulfuric_only_is_not_hcl_inventory() {
        let n = 0.05;
        let entries = vec![water(50.0), aq("h+", 2.0 * n), aq("so4^2-", n)];
        assert!(hcl_inventory_moles_entries(&entries) < 1e-15);
        let inv = HclInventory::from_entries(&entries);
        assert!(inv.n_h < 1e-15);
        assert!(inv.relative_enthalpy_j().abs() < 1e-12);
        assert!((aqueous_h2so4_moles_entries(&entries) - n).abs() < 1e-12);
    }

    #[test]
    fn dilution_of_pure_acid_into_water_is_exothermic() {
        let dest = H2so4Inventory {
            water_ml: 20.0,
            n_h2so4: 0.0,
        };
        let added = H2so4Inventory {
            water_ml: 0.0,
            n_h2so4: H2SO4_STOCK_H2SO4_MOLES / 10.0,
        };
        let after = H2so4Inventory {
            water_ml: 20.0,
            n_h2so4: added.n_h2so4,
        };
        let q = h2so4_dilution_heat_j(dest, added, after);
        assert!(q < 0.0, "expected exothermic dilution, Q={q}");
    }

    fn vessel(entries: Vec<CompositionEntry>) -> SceneItem {
        use crate::scene::ItemProperties;
        SceneItem {
            id: "vessel".into(),
            kind: "evaporation_dish".into(),
            label: "test".into(),
            location: "bench".into(),
            properties: ItemProperties {
                volume_ml: Some(25.0),
                fill_ml: Some(10.0),
                transparent: Some(true),
                colourless: Some(true),
                temperature_c: Some(20.0),
                composition: entries,
                ..ItemProperties::default()
            },
        }
    }

    fn water_ml_for_ratio(n_acid: f64, ratio: f64) -> f64 {
        ratio * n_acid * WATER_MOLAR_MASS_G_PER_MOL
    }

    #[test]
    fn reform_threshold_matches_stock_ww() {
        let expected = (H2SO4_MOLAR_MASS_G_PER_MOL * (1.0 - H2SO4_STOCK_W_W) / H2SO4_STOCK_W_W)
            / WATER_MOLAR_MASS_G_PER_MOL;
        assert!((H2SO4_REFORM_WATER_PER_ACID - expected).abs() < 1e-12);
        // Sanity: ~98% w/w is a small water:acid mole ratio (order 0.1).
        let r = H2SO4_REFORM_WATER_PER_ACID;
        assert!((0.05..0.2).contains(&r), "unexpected reform threshold {r}");
    }

    #[test]
    fn dilute_aqueous_acid_does_not_reform() {
        let n = 0.1;
        let mut item = vessel(vec![water(50.0), aq("h+", 2.0 * n), aq("so4^2-", n)]);
        assert_eq!(reform_aqueous_h2so4_to_liquid(&mut item), 0.0);
        assert!((aqueous_h2so4_moles_entries(&item.properties.composition) - n).abs() < 1e-12);
        assert!(liquid_h2so4_mol_entries(&item.properties.composition) < 1e-15);
    }

    #[test]
    fn concentrate_at_threshold_reforms_to_liquid() {
        let n = 0.1;
        let w = water_ml_for_ratio(n, H2SO4_REFORM_WATER_PER_ACID);
        let mut item = vessel(vec![water(w), aq("h+", 2.0 * n), aq("so4^2-", n)]);
        let reformed = reform_aqueous_h2so4_to_liquid(&mut item);
        assert!((reformed - n).abs() < 1e-12);
        assert!((liquid_h2so4_mol_entries(&item.properties.composition) - n).abs() < 1e-12);
        assert!(crate::composition::aqueous_mol(&item, "h+") < 1e-12);
        assert!(crate::composition::aqueous_mol(&item, "so4^2-") < 1e-12);
        // Residual water stays; ionize must not immediately undo reform.
        assert_eq!(ionize_liquid_h2so4_in_water(&mut item), 0.0);
        assert!((liquid_h2so4_mol_entries(&item.properties.composition) - n).abs() < 1e-12);
    }

    #[test]
    fn dry_out_reforms_free_acid_to_liquid() {
        let n = 0.1;
        let mut item = vessel(vec![aq("h+", 2.0 * n), aq("so4^2-", n)]);
        assert!((reform_aqueous_h2so4_to_liquid(&mut item) - n).abs() < 1e-12);
        assert!((liquid_h2so4_mol_entries(&item.properties.composition) - n).abs() < 1e-12);
        assert!(composition_is_stock_h2so4(&item.properties.composition));
    }

    #[test]
    fn reform_leaves_hcl_inventory_aqueous() {
        let n_h2so4 = 0.1;
        let n_hcl = 0.05;
        // Ratio uses total H₂SO₄ only (HCl is not in H2so4Inventory).
        let w = water_ml_for_ratio(n_h2so4, H2SO4_REFORM_WATER_PER_ACID * 0.5);
        let mut item = vessel(vec![
            water(w),
            aq("h+", 2.0 * n_h2so4 + n_hcl),
            aq("so4^2-", n_h2so4),
            aq("cl-", n_hcl),
        ]);
        let reformed = reform_aqueous_h2so4_to_liquid(&mut item);
        assert!((reformed - n_h2so4).abs() < 1e-12);
        assert!((liquid_h2so4_mol_entries(&item.properties.composition) - n_h2so4).abs() < 1e-12);
        assert!((crate::composition::aqueous_mol(&item, "h+") - n_hcl).abs() < 1e-12);
        assert!((crate::composition::aqueous_mol(&item, "cl-") - n_hcl).abs() < 1e-12);
        assert!(crate::composition::aqueous_mol(&item, "so4^2-") < 1e-12);
        assert!((hcl_inventory_moles_entries(&item.properties.composition) - n_hcl).abs() < 1e-12);
    }

    #[test]
    fn re_dilute_above_threshold_ionizes_liquid() {
        let n = 0.05;
        let mut item = vessel(stock_h2so4_composition());
        // Trim stock to `n` moles.
        let ml = liquid_h2so4_ml_from_moles(n);
        item.properties.composition = vec![CompositionEntry {
            substance_id: "h2so4".into(),
            phase: "liquid".into(),
            amount_ml: Some(ml),
            amount_scoop: None,
            amount_g: Some(n * H2SO4_MOLAR_MASS_G_PER_MOL),
            amount_mol: Some(n),
        }];
        // Add water well above threshold.
        item.properties.composition.push(water(20.0));
        let ionized = ionize_liquid_h2so4_in_water(&mut item);
        assert!((ionized - n).abs() < 1e-12);
        assert!(liquid_h2so4_mol_entries(&item.properties.composition) < 1e-15);
        assert!((crate::composition::aqueous_mol(&item, "hso4-") - n).abs() < 1e-12);
        assert!((crate::composition::aqueous_mol(&item, "h+") - n).abs() < 1e-12);
        assert!(crate::composition::aqueous_mol(&item, "so4^2-") < 1e-15);
    }
}
