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

/// Azeotrope boiling temperature (°C) — the **peak** knot of
/// [`hcl_boil_temperature_c`], not a universal dish plateau.
pub const HCL_AZEOTROPE_BOIL_C: f64 = 108.6;

/// Max relative pull of vapor vs liquid far from the azeotrope (0 = y = w_l).
/// Strength is scaled by distance from [`HCL_AZEOTROPE_W_W`] so `|y − w_l|`
/// shrinks continuously as the liquid approaches the azeotrope.
const AZEOTROPE_VAPOR_BIAS: f64 = 0.55;

/// School-grade HCl–water boiling curve (°C) vs HCl mass fraction.
///
/// Piecewise linear through dilute (near pure-water Antoine), azeotrope peak
/// [`HCL_AZEOTROPE_BOIL_C`] at [`HCL_AZEOTROPE_W_W`], and rich-side fall
/// (stock ~30% w/w → ~105 °C).
pub fn hcl_boil_temperature_c(w_hcl: f64) -> f64 {
    use crate::scene::BOILING_TEMPERATURE_C;
    let w = w_hcl.clamp(0.0, 0.40);
    // Knots: (w, T_boil °C). w=0 matches pure-water Antoine boil.
    const TABLE: [(f64, f64); 8] = [
        (0.0, BOILING_TEMPERATURE_C),
        (0.05, 101.0),
        (0.10, 103.0),
        (0.15, 106.0),
        (0.202, HCL_AZEOTROPE_BOIL_C),
        (0.25, 107.0),
        (0.30, 105.0),
        (0.37, 98.0),
    ];
    for pair in TABLE.windows(2) {
        let (w0, t0) = pair[0];
        let (w1, t1) = pair[1];
        if w <= w1 {
            let frac = if (w1 - w0).abs() <= AMOUNT_EPS {
                0.0
            } else {
                (w - w0) / (w1 - w0)
            };
            return t0 + frac * (t1 - t0);
        }
    }
    TABLE[TABLE.len() - 1].1
}

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

/// Apparent molar volume of aqueous HCl (ml/mol).
///
/// Documented school value ≈ **20.70**; derived from locked Free stock so
/// `8.043 + n_HCl·Φ_V = 10.00` exactly (avoids float drift on put-back capacity).
pub const PHI_V_HCL_ML_PER_MOL: f64 =
    (HCL_STOCK_CAPACITY_ML - HCL_STOCK_WATER_MASS_G) / HCL_STOCK_HCL_MOLES;
/// Apparent molar volume of aqueous NaCl (ml/mol) at school brine strengths.
pub const PHI_V_NACL_ML_PER_MOL: f64 = 22.0;
/// Apparent molar volume of aqueous CaCl₂ (ml/mol).
pub const PHI_V_CACL2_ML_PER_MOL: f64 = 34.0;
/// Apparent molar volume of aqueous NaOH (ml/mol); mid-range compromise (Φ_V° is negative).
pub const PHI_V_NAOH_ML_PER_MOL: f64 = 4.0;

/// Pair aqueous ions into electrolyte formula units for additive Φ_V volume.
///
/// Order matches neutralization / SI inventory: HCl → NaOH → NaCl → CaCl₂.
#[derive(Debug, Clone, Copy, Default)]
pub struct ElectrolyteMoles {
    pub n_hcl: f64,
    pub n_naoh: f64,
    pub n_nacl: f64,
    pub n_cacl2: f64,
}

impl ElectrolyteMoles {
    pub fn from_entries(entries: &[CompositionEntry]) -> Self {
        let n_h = aqueous_mol_entries(entries, "h+");
        let n_oh = aqueous_mol_entries(entries, "oh-");
        let n_na = aqueous_mol_entries(entries, "na+");
        let n_ca = aqueous_mol_entries(entries, "ca2+");
        Self {
            n_hcl: n_h,
            n_naoh: n_oh,
            n_nacl: (n_na - n_oh).max(0.0),
            n_cacl2: n_ca,
        }
    }
}

/// Solution volume (ml) = liquid water ml + Σ nᵢ · Φ_V,ᵢ for paired electrolytes.
///
/// Φ_V values are T-independent school constants (see module constants). SI / wash
/// kinetics keep using liquid-water ml; this volume drives `fill_ml`, pipette frac,
/// tongs capacity, and inspect molarity / pH.
pub fn solution_volume_ml_of_entries(entries: &[CompositionEntry]) -> f64 {
    let water_ml = liquid_water_ml_entries(entries);
    let el = ElectrolyteMoles::from_entries(entries);
    water_ml
        + el.n_hcl * PHI_V_HCL_ML_PER_MOL
        + el.n_naoh * PHI_V_NAOH_ML_PER_MOL
        + el.n_nacl * PHI_V_NACL_ML_PER_MOL
        + el.n_cacl2 * PHI_V_CACL2_ML_PER_MOL
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
///
/// Pull strength is scaled by relative distance from the azeotrope so
/// `|y − w_l|` shrinks continuously as `w_l → w_az`.
pub fn azeotrope_vapor_w_hcl(w_liquid: f64) -> f64 {
    let w_l = w_liquid.clamp(0.0, 1.0);
    let w_az = HCL_AZEOTROPE_W_W;
    if (w_l - w_az).abs() < 1e-4 {
        return w_az;
    }
    if w_l < w_az {
        // Bias vapor toward water; pull → 0 as w_l → w_az.
        let rel = ((w_az - w_l) / w_az).clamp(0.0, 1.0);
        let pull = AZEOTROPE_VAPOR_BIAS * rel;
        let w_v = w_l * (1.0 - pull);
        w_v.clamp(0.0, w_l)
    } else {
        // Bias vapor toward richer HCl; pull → 0 as w_l → w_az.
        let rel = ((w_l - w_az) / (1.0 - w_az)).clamp(0.0, 1.0);
        let pull = AZEOTROPE_VAPOR_BIAS * rel;
        let w_v = w_l + pull * (1.0 - w_l);
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
}
