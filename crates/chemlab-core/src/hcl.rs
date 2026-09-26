//! Free-mode aqueous HCl stock helpers: density, solution volume, dilution enthalpy,
//! pH, and azeotrope-style evaporation.

use crate::scene::{CompositionEntry, SceneItem, WATER_MOLAR_MASS_G_PER_MOL};

const AMOUNT_EPS: f64 = 1e-12;

/// HCl stock beaker capacity (ml of **solution**).
pub const HCL_STOCK_CAPACITY_ML: f64 = 10.00;

/// Stock label density at 30% w/w HCl (g/ml).
pub const HCL_STOCK_DENSITY_G_PER_ML: f64 = 1.149;

/// Total mass of the filled Free-mode HCl stock (g): 10.00 ml × 1.149 g/ml.
pub const HCL_STOCK_TOTAL_MASS_G: f64 = 11.49;

/// HCl mass in the filled stock (g): 30% w/w of [`HCL_STOCK_TOTAL_MASS_G`].
pub const HCL_STOCK_HCL_MASS_G: f64 = 3.447;

/// Water mass in the filled stock (g / ml at ρ_water = 1 g/ml).
pub const HCL_STOCK_WATER_MASS_G: f64 = 8.043;

/// Molar mass of HCl (g/mol).
pub const HCL_MOLAR_MASS_G_PER_MOL: f64 = 36.46;

/// Moles of HCl (= H⁺ = Cl⁻) in the filled Free-mode stock.
pub const HCL_STOCK_HCL_MOLES: f64 = HCL_STOCK_HCL_MASS_G / HCL_MOLAR_MASS_G_PER_MOL;

/// Stock HCl mass fraction (30% w/w).
pub const HCL_STOCK_W_W: f64 = 0.30;

/// Tolerance on w/w when accepting put-back into the HCl stock.
pub const HCL_STOCK_W_W_EPS: f64 = 0.005;

/// HCl–water azeotrope mass fraction (~20.2% w/w HCl).
pub const HCL_AZEOTROPE_W_W: f64 = 0.202;

/// **Documented boil rule:** when aqueous HCl (`h+`) is present in the dish, the
/// dish boils at the HCl–water azeotrope temperature **108.6 °C** (not Raoult /
/// Antoine elevation alone). Without aqueous acid, boiling stays on the existing
/// water mole-fraction Antoine curve.
pub const HCL_AZEOTROPE_BOIL_C: f64 = 108.6;

/// How strongly vapor composition is biased toward the azeotrope relative to the
/// liquid (0 = vapor = liquid, 1 = vapor jumps fully past liquid toward the
/// qualitative azeotrope side).
const AZEOTROPE_VAPOR_BIAS: f64 = 0.55;

/// Density of aqueous HCl (g/ml) from HCl mass fraction.
///
/// Piecewise linear through (0, 1.000), (0.202, 1.098), (0.30, 1.149), (0.37, 1.184).
pub fn hcl_aq_density_g_per_ml(w_hcl: f64) -> f64 {
    let w = w_hcl.clamp(0.0, 0.40);
    const TABLE: [(f64, f64); 4] = [(0.0, 1.000), (0.202, 1.098), (0.30, 1.149), (0.37, 1.184)];
    for pair in TABLE.windows(2) {
        let (w0, d0) = pair[0];
        let (w1, d1) = pair[1];
        if w <= w1 {
            let frac = if (w1 - w0).abs() <= AMOUNT_EPS {
                0.0
            } else {
                (w - w0) / (w1 - w0)
            };
            return d0 + frac * (d1 - d0);
        }
    }
    TABLE[TABLE.len() - 1].1
}

fn aqueous_mol_entries(entries: &[CompositionEntry], substance_id: &str) -> f64 {
    entries
        .iter()
        .find(|c| c.substance_id == substance_id && c.phase == "aqueous")
        .and_then(|c| c.amount_mol)
        .unwrap_or(0.0)
        .max(0.0)
}

fn liquid_water_ml_entries(entries: &[CompositionEntry]) -> f64 {
    entries
        .iter()
        .find(|c| c.substance_id == "water" && c.phase == "liquid")
        .and_then(|c| c.amount_ml)
        .unwrap_or(0.0)
        .max(0.0)
}

/// Inventory of water + aqueous HCl in a composition list.
#[derive(Debug, Clone, Copy, Default)]
pub struct HclInventory {
    pub water_ml: f64,
    pub n_h: f64,
}

impl HclInventory {
    pub fn from_entries(entries: &[CompositionEntry]) -> Self {
        Self {
            water_ml: liquid_water_ml_entries(entries),
            n_h: aqueous_mol_entries(entries, "h+"),
        }
    }

    pub fn from_item(item: &SceneItem) -> Self {
        Self::from_entries(&item.properties.composition)
    }

    pub fn hcl_mass_g(self) -> f64 {
        self.n_h * HCL_MOLAR_MASS_G_PER_MOL
    }

    pub fn water_mass_g(self) -> f64 {
        self.water_ml // ρ_water = 1 g/ml
    }

    pub fn total_mass_g(self) -> f64 {
        self.water_mass_g() + self.hcl_mass_g()
    }

    pub fn w_hcl(self) -> f64 {
        let m = self.total_mass_g();
        if m <= AMOUNT_EPS {
            return 0.0;
        }
        self.hcl_mass_g() / m
    }

    /// Water:HCl mole ratio (∞ when no HCl).
    pub fn water_per_hcl(self) -> f64 {
        if self.n_h <= AMOUNT_EPS {
            return f64::INFINITY;
        }
        (self.water_ml / WATER_MOLAR_MASS_G_PER_MOL) / self.n_h
    }

    /// Relative enthalpy of this HCl(aq) parcel vs infinite dilution (J).
    ///
    /// Simplified table of apparent relative molar enthalpy Φ_L (J/mol HCl) vs
    /// water:HCl mole ratio — dilution is exothermic when Φ_L rises toward 0.
    pub fn relative_enthalpy_j(self) -> f64 {
        if self.n_h <= AMOUNT_EPS {
            return 0.0;
        }
        self.n_h * hcl_phi_l_j_per_mol(self.water_per_hcl())
    }
}

/// Φ_L (J/mol HCl) vs r = n_water / n_HCl. Infinite dilution → 0.
///
/// Values are positive at finite concentration so dilution (Φ falling toward 0)
/// is exothermic: Q = n·Φ_after − n·Φ_before < 0.
fn hcl_phi_l_j_per_mol(r: f64) -> f64 {
    if !r.is_finite() || r >= 500.0 {
        return 0.0;
    }
    // Approximate CRC-style heat-of-dilution shape for HCl(aq).
    const TABLE: [(f64, f64); 7] = [
        (2.5, 42_000.0),
        (4.0, 32_000.0),
        (5.0, 27_000.0),
        (8.0, 18_000.0),
        (15.0, 10_000.0),
        (40.0, 3_500.0),
        (100.0, 800.0),
    ];
    let r = r.max(TABLE[0].0);
    if r >= TABLE[TABLE.len() - 1].0 {
        let (r0, y0) = TABLE[TABLE.len() - 1];
        // Soft approach to 0 beyond the last knot.
        return y0 * (r0 / r);
    }
    for pair in TABLE.windows(2) {
        let (r0, y0) = pair[0];
        let (r1, y1) = pair[1];
        if r <= r1 {
            let frac = (r - r0) / (r1 - r0);
            return y0 + frac * (y1 - y0);
        }
    }
    0.0
}

/// Solution volume (ml) for water + aqueous HCl using composition mass / density.
///
/// Pure water (no `h+`) stays at 1 g/ml (= water ml). Salt-only aqueous solutions
/// without HCl also keep the water-ml volume (existing bench behaviour).
pub fn solution_volume_ml_of_entries(entries: &[CompositionEntry]) -> f64 {
    let inv = HclInventory::from_entries(entries);
    if inv.n_h <= AMOUNT_EPS {
        return inv.water_ml;
    }
    let m = inv.total_mass_g();
    if m <= AMOUNT_EPS {
        return 0.0;
    }
    m / hcl_aq_density_g_per_ml(inv.w_hcl())
}

pub fn solution_volume_ml(item: &SceneItem) -> f64 {
    solution_volume_ml_of_entries(&item.properties.composition)
}

/// Strong-acid / strong-base approximate pH from composition.
///
/// - Acid (`h+` present, no `oh-`): pH = −log₁₀([H⁺]) with [H⁺] = n_h+ / V_solution_L.
/// - Base (`oh-` present, no `h+`): pH ≈ 14 + log₁₀([OH⁻]).
/// - Both present: returns `None` (callers should neutralize first).
/// - Neither / empty volume: `None`.
pub fn ph_of_entries(entries: &[CompositionEntry]) -> Option<f64> {
    let n_h = aqueous_mol_entries(entries, "h+");
    let n_oh = aqueous_mol_entries(entries, "oh-");
    let v_l = solution_volume_ml_of_entries(entries) / 1000.0;
    if v_l <= AMOUNT_EPS {
        return None;
    }
    if n_h > AMOUNT_EPS && n_oh > AMOUNT_EPS {
        return None;
    }
    if n_h > AMOUNT_EPS {
        let conc = n_h / v_l;
        if conc <= AMOUNT_EPS {
            return None;
        }
        return Some(-conc.log10());
    }
    if n_oh > AMOUNT_EPS {
        let conc = n_oh / v_l;
        if conc <= AMOUNT_EPS {
            return None;
        }
        return Some(14.0 + conc.log10());
    }
    None
}

pub fn ph_of_item(item: &SceneItem) -> Option<f64> {
    ph_of_entries(&item.properties.composition)
}

/// True when entries are only water + stoichiometric H⁺/Cl⁻ at stock 30% w/w (±ε).
pub fn composition_is_stock_hcl(entries: &[CompositionEntry]) -> bool {
    let mut has_water = false;
    let mut n_h = 0.0;
    let mut n_cl = 0.0;
    let mut water_ml = 0.0;
    for entry in entries {
        let has_amt = entry.amount_ml.unwrap_or(0.0) > AMOUNT_EPS
            || entry.amount_g.unwrap_or(0.0) > AMOUNT_EPS
            || entry.amount_mol.unwrap_or(0.0) > AMOUNT_EPS
            || entry.amount_scoop.unwrap_or(0) > 0;
        if !has_amt {
            continue;
        }
        if entry.substance_id == "water" && entry.phase == "liquid" {
            has_water = true;
            water_ml += entry.amount_ml.unwrap_or(0.0).max(0.0);
            continue;
        }
        if entry.substance_id == "h+" && entry.phase == "aqueous" {
            n_h += entry.amount_mol.unwrap_or(0.0).max(0.0);
            continue;
        }
        if entry.substance_id == "cl-" && entry.phase == "aqueous" {
            n_cl += entry.amount_mol.unwrap_or(0.0).max(0.0);
            continue;
        }
        return false;
    }
    if !has_water || n_h <= AMOUNT_EPS {
        return false;
    }
    if (n_h - n_cl).abs() > 1e-6 {
        return false;
    }
    let inv = HclInventory { water_ml, n_h };
    (inv.w_hcl() - HCL_STOCK_W_W).abs() <= HCL_STOCK_W_W_EPS
}

/// Heat absorbed by the chemical change when combining `dest` + `added` into `after` (J).
///
/// Negative ⇒ exothermic dilution. Apply `ΔT = −Q / C_eff` on the destination.
pub fn hcl_dilution_heat_j(dest: HclInventory, added: HclInventory, after: HclInventory) -> f64 {
    after.relative_enthalpy_j() - dest.relative_enthalpy_j() - added.relative_enthalpy_j()
}

/// Mass fraction of HCl in vapor for azeotrope-style boil / MT.
///
/// Maximum-boiling azeotrope qualitative split:
/// - liquid leaner than az (`w_l < w_az`): vapor **leaner** in HCl than liquid
///   (richer in water) so the liquid concentrates toward the azeotrope;
/// - liquid richer than az: vapor **richer** in HCl than liquid so the liquid
///   leans toward the azeotrope.
pub fn azeotrope_vapor_w_hcl(w_liquid: f64) -> f64 {
    let w_l = w_liquid.clamp(0.0, 1.0);
    let w_az = HCL_AZEOTROPE_W_W;
    if (w_l - w_az).abs() < 1e-4 {
        return w_az;
    }
    if w_l < w_az {
        // Bias vapor toward water (lower HCl fraction than liquid).
        let w_v = w_l * (1.0 - AZEOTROPE_VAPOR_BIAS);
        w_v.clamp(0.0, w_l)
    } else {
        // Bias vapor toward richer HCl than liquid.
        let w_v = w_l + AZEOTROPE_VAPOR_BIAS * (1.0 - w_l);
        w_v.clamp(w_l, 1.0)
    }
}

/// Remove `loss_mass_g` of vapor from a dish with aqueous HCl, splitting water and
/// HCl toward the azeotrope. Evaporated HCl removes `n_loss = m_hcl_loss / M_HCl`
/// moles from **both** `h+` and `cl-` (clamp ≥ 0) so salt Cl⁻ is not scaled away.
/// Latent heat is still charged as water ΔH_vap × mass (documented approximation).
pub fn remove_hcl_water_evap_mass(dish: &mut SceneItem, loss_mass_g: f64) {
    if loss_mass_g <= AMOUNT_EPS {
        return;
    }
    let inv = HclInventory::from_item(dish);
    if inv.n_h <= AMOUNT_EPS {
        // Caller should use water-only removal when no acid.
        return;
    }
    let m_tot = inv.total_mass_g();
    if m_tot <= AMOUNT_EPS {
        return;
    }
    let loss = loss_mass_g.min(m_tot);
    let w_v = azeotrope_vapor_w_hcl(inv.w_hcl());
    let m_hcl_loss = (loss * w_v).min(inv.hcl_mass_g());
    let m_water_loss = (loss - m_hcl_loss).min(inv.water_mass_g()).max(0.0);

    if let Some(water) = dish
        .properties
        .composition
        .iter_mut()
        .find(|c| c.substance_id == "water" && c.phase == "liquid")
    {
        let remain = (water.amount_ml.unwrap_or(0.0) - m_water_loss).max(0.0);
        water.amount_ml = Some(remain);
    }

    if m_hcl_loss > AMOUNT_EPS {
        let n_loss = m_hcl_loss / HCL_MOLAR_MASS_G_PER_MOL;
        subtract_aqueous(dish, "h+", n_loss);
        subtract_aqueous(dish, "cl-", n_loss);
    }
}

fn subtract_aqueous(item: &mut SceneItem, substance_id: &str, n_loss: f64) {
    if n_loss <= AMOUNT_EPS {
        return;
    }
    if let Some(entry) = item
        .properties
        .composition
        .iter_mut()
        .find(|c| c.substance_id == substance_id && c.phase == "aqueous")
    {
        let next = (entry.amount_mol.unwrap_or(0.0).max(0.0) - n_loss).max(0.0);
        entry.amount_mol = Some(if next <= AMOUNT_EPS { 0.0 } else { next });
    }
    item.properties.composition.retain(|c| {
        if c.phase == "aqueous" && c.substance_id == substance_id {
            c.amount_mol.unwrap_or(0.0) > AMOUNT_EPS
        } else {
            true
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stock_moles_and_volume_match_locked_numbers() {
        assert!((HCL_STOCK_HCL_MOLES - 3.447 / 36.46).abs() < 1e-12);
        assert!((HCL_STOCK_TOTAL_MASS_G / HCL_STOCK_DENSITY_G_PER_ML - 10.0).abs() < 1e-9);
        let dens = hcl_aq_density_g_per_ml(0.30);
        assert!((dens - 1.149).abs() < 1e-9);
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
}
