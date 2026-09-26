//! Mixed NaCl / CaCl₂ saturation (common-ion Cl⁻) for every aqueous vessel.
//!
//! Pure-water solubilities are literature g / 100 g H₂O, converted with ρ_water = 1.000 g/mL
//! so concentrations are **mol per litre of liquid water** (the core’s solvent-volume basis,
//! numerically ≈ molality). In a mixture the salts share Cl⁻: ion activity products are
//! compared to T-fitted K(T) from the pure curves with a Davies activity correction.
//! SiO₂ / sand never enters the ion balance.
//!
//! **Activity model (Davies + soft-cap I):**
//! `log₁₀ γᵢ = −A zᵢ² [√I_eff/(1+√I_eff) − 0.3 I_eff]` with
//! `I_eff = I / (1 + I / I_STAR)`, `I_STAR = 3.0` mol/kg, and
//! `I = ½ Σ mᵢ zᵢ²` from Na⁺, Ca²⁺, H⁺, Cl⁻, and OH⁻
//! (Debye–Hückel A(T) from Malmberg–Maryott ε_r(water), ρ = 1.000 g/cm³).
//! Davies is designed for I ≲ 0.5 mol/kg; saturated NaCl/CaCl₂ are much higher.
//! The soft cap keeps γ continuous and finite at school brine strengths (not a
//! substitute for Pitzer). K(T) is fitted with the same `I_eff` so a **pure**
//! saturated solution still matches the literature curve; mixed SI is dominated
//! by the ion product (extra Cl⁻ suppresses NaCl in the correct direction).
//! Solvent basis for SI stays **water litres**, not solution volume.

use crate::scene::{
    CompositionEntry, SceneItem, CACL2_MOLAR_MASS_G_PER_MOL, NACL_MOLAR_MASS_G_PER_MOL,
};

/// NaCl g / 100 g water vs T. CRC Handbook (Haynes, ed.).
const NACL_G_PER_100G_WATER: &[(f64, f64)] = &[
    (20.0, 35.89),
    (40.0, 36.37),
    (60.0, 37.04),
    (80.0, 37.93),
    (100.0, 38.99),
];

/// Anhydrous CaCl₂ g / 100 g water vs T (hydrate solids collapsed to game `cacl2`).
/// Seidell / CRC mass % converted to g anhydrous per 100 g water.
const CACL2_G_PER_100G_WATER: &[(f64, f64)] = &[
    (20.0, 74.5),
    (40.0, 115.1),
    (60.0, 137.0),
    (80.0, 146.9),
    (100.0, 159.0),
];

const AMOUNT_EPS: f64 = 1e-12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Salt {
    Nacl,
    Cacl2,
}

/// Pure-water formula-unit solubility (mol per litre of water) at vessel T.
///
/// Piecewise-linear between literature table points; clamped to 20–100 °C.
pub fn solubility_mol_per_l(salt: Salt, temperature_c: f64) -> f64 {
    let (table, molar_mass) = match salt {
        Salt::Nacl => (NACL_G_PER_100G_WATER, NACL_MOLAR_MASS_G_PER_MOL),
        Salt::Cacl2 => (CACL2_G_PER_100G_WATER, CACL2_MOLAR_MASS_G_PER_MOL),
    };
    interpolate_g_per_100g(table, temperature_c) * 10.0 / molar_mass
}

fn interpolate_g_per_100g(table: &[(f64, f64)], temperature_c: f64) -> f64 {
    let first = table[0];
    let last = table[table.len() - 1];
    if temperature_c <= first.0 {
        return first.1;
    }
    if temperature_c >= last.0 {
        return last.1;
    }
    for pair in table.windows(2) {
        let (t0, y0) = pair[0];
        let (t1, y1) = pair[1];
        if temperature_c <= t1 {
            let frac = (temperature_c - t0) / (t1 - t0);
            return y0 + frac * (y1 - y0);
        }
    }
    last.1
}

pub fn liquid_water_ml(item: &SceneItem) -> f64 {
    item.properties
        .composition
        .iter()
        .find(|c| c.substance_id == "water" && c.phase == "liquid")
        .and_then(|c| c.amount_ml)
        .unwrap_or(0.0)
        .max(0.0)
}

pub fn dish_has_liquid(item: &SceneItem) -> bool {
    liquid_water_ml(item) > AMOUNT_EPS
}

pub fn sync_fill_ml(item: &mut SceneItem) {
    if item.kind == "beaker" || item.kind == "evaporation_dish" || item.kind == "pipette" {
        item.properties.fill_ml = Some(crate::hcl::solution_volume_ml(item));
    }
}

fn aqueous_mol(item: &SceneItem, substance_id: &str) -> f64 {
    item.properties
        .composition
        .iter()
        .find(|c| c.substance_id == substance_id && c.phase == "aqueous")
        .and_then(|c| c.amount_mol)
        .unwrap_or(0.0)
        .max(0.0)
}

fn solid_mol(item: &SceneItem, substance_id: &str, molar_mass: f64) -> f64 {
    item.properties
        .composition
        .iter()
        .find(|c| c.substance_id == substance_id && c.phase == "solid")
        .and_then(|c| c.amount_g)
        .map(|g| (g.max(0.0)) / molar_mass)
        .unwrap_or(0.0)
}

fn set_aqueous_mol(item: &mut SceneItem, substance_id: &str, moles: f64) {
    if moles <= AMOUNT_EPS {
        item.properties
            .composition
            .retain(|c| !(c.substance_id == substance_id && c.phase == "aqueous"));
        return;
    }
    if let Some(existing) = item
        .properties
        .composition
        .iter_mut()
        .find(|c| c.substance_id == substance_id && c.phase == "aqueous")
    {
        existing.amount_mol = Some(moles);
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

fn set_salt_solid(item: &mut SceneItem, substance_id: &str, moles: f64, molar_mass: f64) {
    let grams = moles * molar_mass;
    if grams <= AMOUNT_EPS {
        item.properties
            .composition
            .retain(|c| !(c.substance_id == substance_id && c.phase == "solid"));
        return;
    }
    if let Some(existing) = item
        .properties
        .composition
        .iter_mut()
        .find(|c| c.substance_id == substance_id && c.phase == "solid")
    {
        existing.amount_g = Some(grams);
        existing.amount_mol = Some(moles);
        existing.amount_scoop = None;
        return;
    }
    item.properties.composition.push(CompositionEntry {
        substance_id: substance_id.into(),
        phase: "solid".into(),
        amount_ml: None,
        amount_scoop: None,
        amount_g: Some(grams),
        amount_mol: Some(moles),
    });
}

fn remove_near_zero_water(item: &mut SceneItem) {
    if liquid_water_ml(item) <= AMOUNT_EPS {
        item.properties
            .composition
            .retain(|c| !(c.substance_id == "water" && c.phase == "liquid"));
    }
}

fn has_aqueous_ions(item: &SceneItem) -> bool {
    item.properties
        .composition
        .iter()
        .any(|c| c.phase == "aqueous")
}

/// Additional grams of `salt` that could dissolve into water with the given aqueous
/// Na⁺ / Ca²⁺ / H⁺ / OH⁻ inventory at `temperature_c` before mixed SI = 1 (0 if already saturated).
///
/// `n_h_aq` contributes Cl⁻ common-ion (and ionic strength) from aqueous HCl without
/// mutating the fluid. `n_oh_aq` contributes to ionic strength only (not to NaCl/CaCl₂ IAP).
/// Holds the other cation fixed (no precipitation sidelight during the probe) so filter
/// wash capacity matches common-ion suppression.
pub(crate) fn unsaturated_capacity_g(
    salt: Salt,
    water_ml: f64,
    n_na_aq: f64,
    n_ca_aq: f64,
    n_h_aq: f64,
    n_oh_aq: f64,
    temperature_c: f64,
) -> f64 {
    let litres = water_ml / 1000.0;
    if litres <= AMOUNT_EPS {
        return 0.0;
    }
    let n_h = n_h_aq.max(0.0);
    let n_oh = n_oh_aq.max(0.0);
    match salt {
        Salt::Nacl => {
            let pure_max = solubility_mol_per_l(Salt::Nacl, temperature_c) * litres;
            let probe = n_na_aq.max(0.0) + pure_max * 2.0 + 1.0;
            let max_aq =
                dissolved_nacl_at_si1(probe, n_ca_aq.max(0.0), n_h, n_oh, litres, temperature_c);
            (max_aq - n_na_aq).max(0.0) * NACL_MOLAR_MASS_G_PER_MOL
        }
        Salt::Cacl2 => {
            let pure_max = solubility_mol_per_l(Salt::Cacl2, temperature_c) * litres;
            let probe = n_ca_aq.max(0.0) + pure_max * 2.0 + 1.0;
            let max_aq =
                dissolved_cacl2_at_si1(probe, n_na_aq.max(0.0), n_h, n_oh, litres, temperature_c);
            (max_aq - n_ca_aq).max(0.0) * CACL2_MOLAR_MASS_G_PER_MOL
        }
    }
}

/// Convert aqueous ↔ solid so both saturation indices are ≤ 1 at the item's current T.
///
/// Same mixed NaCl/CaCl₂ equilibrium on every aqueous vessel. Dry stock solids (no water,
/// no aqueous ions, not an evaporation dish) are left untouched. Precipitating NaCl
/// removes 1 Na⁺ + 1 Cl⁻; CaCl₂ removes 1 Ca²⁺ + 2 Cl⁻. Aqueous H⁺ from HCl is preserved
/// and contributes its Cl⁻ (common ion) to the SI solver.
///
/// **NaOH / OH⁻:** aqueous Na⁺ paired 1:1 with OH⁻ is **not** counted as NaCl inventory
/// (otherwise SI would invent matching Cl⁻ and can precipitate solid `nacl`). Callers must
/// run H⁺+OH⁻ neutralization before this when both are present. Excess OH⁻ is preserved.
pub fn enforce_saturation(item: &mut SceneItem) {
    let temperature_c = item.properties.temperature_c.unwrap_or(20.0);
    let water_ml = liquid_water_ml(item);
    let litres = water_ml / 1000.0;

    if litres <= AMOUNT_EPS && item.kind != "evaporation_dish" && !has_aqueous_ions(item) {
        return;
    }

    let n_oh = aqueous_mol(item, "oh-");
    let n_na_total = aqueous_mol(item, "na+");
    // Na⁺ beyond OH⁻-paired inventory is the NaCl (and free Na⁺) pool for SI.
    let n_na_salt = (n_na_total - n_oh).max(0.0);
    let total_nacl = n_na_salt + solid_mol(item, "nacl", NACL_MOLAR_MASS_G_PER_MOL);
    let total_cacl2 =
        aqueous_mol(item, "ca2+") + solid_mol(item, "cacl2", CACL2_MOLAR_MASS_G_PER_MOL);
    let n_h = aqueous_mol(item, "h+");

    let (aq_nacl, solid_nacl, aq_cacl2, solid_cacl2) = if litres <= AMOUNT_EPS {
        (0.0, total_nacl, 0.0, total_cacl2)
    } else {
        mixed_equilibrium(total_nacl, total_cacl2, n_h, n_oh, litres, temperature_c)
    };

    set_aqueous_mol(item, "na+", aq_nacl + n_oh);
    set_aqueous_mol(item, "ca2+", aq_cacl2);
    set_aqueous_mol(item, "cl-", aq_nacl + 2.0 * aq_cacl2 + n_h);
    // Preserve H⁺ and OH⁻ (set explicitly so a wiped Cl⁻ line does not imply lost acid/base).
    set_aqueous_mol(item, "h+", n_h);
    set_aqueous_mol(item, "oh-", n_oh);
    set_salt_solid(item, "nacl", solid_nacl, NACL_MOLAR_MASS_G_PER_MOL);
    set_salt_solid(item, "cacl2", solid_cacl2, CACL2_MOLAR_MASS_G_PER_MOL);
    remove_near_zero_water(item);
    sync_fill_ml(item);
}

/// log₁₀(SI) tolerance ≈ SI within 10⁻⁸ of 1.
const LOG10_SI_TOL: f64 = 1e-8;
const MIXED_OUTER_ITERS: usize = 64;
const MIXED_BISECT_ITERS: usize = 80;

#[derive(Clone, Copy)]
struct Mixture {
    m_na: f64,
    m_ca: f64,
    /// Molality of H⁺ from aqueous HCl (paired Cl⁻ included in `m_cl`).
    m_h: f64,
    /// Molality of OH⁻ (contributes to I only; not to NaCl/CaCl₂ IAP / `m_cl`).
    m_oh: f64,
}

impl Mixture {
    fn from_moles(n_na: f64, n_ca: f64, n_h: f64, n_oh: f64, litres: f64) -> Self {
        Self {
            m_na: (n_na / litres).max(0.0),
            m_ca: (n_ca / litres).max(0.0),
            m_h: (n_h / litres).max(0.0),
            m_oh: (n_oh / litres).max(0.0),
        }
    }

    fn m_cl(self) -> f64 {
        self.m_na + 2.0 * self.m_ca + self.m_h
    }

    fn ionic_strength(self) -> f64 {
        // I = ½ (m_Na_total + 4 m_Ca + m_H + m_Cl + m_OH) with
        // m_Na_total = m_na (salt) + m_oh and m_Cl = m_na + 2 m_Ca + m_H
        // → m_na + 3 m_Ca + m_h + m_oh.
        self.m_na + 3.0 * self.m_ca + self.m_h + self.m_oh
    }
}

/// Debye–Hückel A (molal⁻½). Malmberg & Maryott, J. Res. Natl. Bur. Stand. 56 (1956) 1.
fn debye_huckel_a(temperature_c: f64) -> f64 {
    let t_c = temperature_c.clamp(0.0, 100.0);
    let t_k = t_c + 273.15;
    let eps = 87.740 - 0.4008 * t_c + 9.398e-4 * t_c * t_c - 1.410e-6 * t_c * t_c * t_c;
    let rho: f64 = 1.0;
    1.8246e6 * rho.sqrt() / (eps * t_k).powf(1.5)
}

/// Soft-cap scale for effective ionic strength in Davies γ (mol/kg).
const DAVIES_I_STAR: f64 = 3.0;

fn effective_ionic_strength(ionic_strength: f64) -> f64 {
    let i = ionic_strength.max(0.0);
    i / (1.0 + i / DAVIES_I_STAR)
}

fn log10_gamma(z: i32, ionic_strength: f64, a_dh: f64) -> f64 {
    let i = effective_ionic_strength(ionic_strength);
    let sqrt_i = i.sqrt();
    -a_dh * f64::from(z * z) * (sqrt_i / (1.0 + sqrt_i) - 0.3 * i)
}

fn log10_iap_nacl(mix: Mixture, a_dh: f64) -> Option<f64> {
    let m_cl = mix.m_cl();
    if mix.m_na <= 0.0 || m_cl <= 0.0 {
        return None;
    }
    let i = mix.ionic_strength();
    Some(2.0 * log10_gamma(1, i, a_dh) + mix.m_na.log10() + m_cl.log10())
}

fn log10_iap_cacl2(mix: Mixture, a_dh: f64) -> Option<f64> {
    let m_cl = mix.m_cl();
    if mix.m_ca <= 0.0 || m_cl <= 0.0 {
        return None;
    }
    let i = mix.ionic_strength();
    Some(
        log10_gamma(2, i, a_dh)
            + mix.m_ca.log10()
            + 2.0 * log10_gamma(1, i, a_dh)
            + 2.0 * m_cl.log10(),
    )
}

fn log10_k_nacl(temperature_c: f64) -> f64 {
    let s = solubility_mol_per_l(Salt::Nacl, temperature_c);
    let a_dh = debye_huckel_a(temperature_c);
    log10_iap_nacl(
        Mixture {
            m_na: s,
            m_ca: 0.0,
            m_h: 0.0,
            m_oh: 0.0,
        },
        a_dh,
    )
    .expect("pure NaCl solubility is positive")
}

fn log10_k_cacl2(temperature_c: f64) -> f64 {
    let s = solubility_mol_per_l(Salt::Cacl2, temperature_c);
    let a_dh = debye_huckel_a(temperature_c);
    log10_iap_cacl2(
        Mixture {
            m_na: 0.0,
            m_ca: s,
            m_h: 0.0,
            m_oh: 0.0,
        },
        a_dh,
    )
    .expect("pure CaCl2 solubility is positive")
}

fn log10_si_nacl(mix: Mixture, temperature_c: f64) -> f64 {
    match log10_iap_nacl(mix, debye_huckel_a(temperature_c)) {
        None => f64::NEG_INFINITY,
        Some(log_iap) => log_iap - log10_k_nacl(temperature_c),
    }
}

fn log10_si_cacl2(mix: Mixture, temperature_c: f64) -> f64 {
    match log10_iap_cacl2(mix, debye_huckel_a(temperature_c)) {
        None => f64::NEG_INFINITY,
        Some(log_iap) => log_iap - log10_k_cacl2(temperature_c),
    }
}

fn dissolved_nacl_at_si1(
    n_na_tot: f64,
    n_ca: f64,
    n_h: f64,
    n_oh: f64,
    litres: f64,
    temperature_c: f64,
) -> f64 {
    let a_dh = debye_huckel_a(temperature_c);
    let log_k = log10_k_nacl(temperature_c);
    let log_si = |n_na: f64| {
        log10_iap_nacl(Mixture::from_moles(n_na, n_ca, n_h, n_oh, litres), a_dh)
            .map(|log_iap| log_iap - log_k)
            .unwrap_or(f64::NEG_INFINITY)
    };
    if log_si(n_na_tot) <= LOG10_SI_TOL {
        return n_na_tot;
    }
    let mut lo = 0.0;
    let mut hi = n_na_tot;
    for _ in 0..MIXED_BISECT_ITERS {
        let mid = 0.5 * (lo + hi);
        if log_si(mid) > 0.0 {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    0.5 * (lo + hi)
}

fn dissolved_cacl2_at_si1(
    n_ca_tot: f64,
    n_na: f64,
    n_h: f64,
    n_oh: f64,
    litres: f64,
    temperature_c: f64,
) -> f64 {
    let a_dh = debye_huckel_a(temperature_c);
    let log_k = log10_k_cacl2(temperature_c);
    let log_si = |n_ca: f64| {
        log10_iap_cacl2(Mixture::from_moles(n_na, n_ca, n_h, n_oh, litres), a_dh)
            .map(|log_iap| log_iap - log_k)
            .unwrap_or(f64::NEG_INFINITY)
    };
    if log_si(n_ca_tot) <= LOG10_SI_TOL {
        return n_ca_tot;
    }
    let mut lo = 0.0;
    let mut hi = n_ca_tot;
    for _ in 0..MIXED_BISECT_ITERS {
        let mid = 0.5 * (lo + hi);
        if log_si(mid) > 0.0 {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    0.5 * (lo + hi)
}

fn mixed_equilibrium(
    total_nacl: f64,
    total_cacl2: f64,
    n_h: f64,
    n_oh: f64,
    litres: f64,
    temperature_c: f64,
) -> (f64, f64, f64, f64) {
    let mut n_na = total_nacl.max(0.0);
    let mut n_ca = total_cacl2.max(0.0);
    let n_h = n_h.max(0.0);
    let n_oh = n_oh.max(0.0);
    for _ in 0..MIXED_OUTER_ITERS {
        let mix = Mixture::from_moles(n_na, n_ca, n_h, n_oh, litres);
        let log_si_n = log10_si_nacl(mix, temperature_c);
        let log_si_c = log10_si_cacl2(mix, temperature_c);
        let over_n = log_si_n > LOG10_SI_TOL;
        let over_c = log_si_c > LOG10_SI_TOL;
        let under_n = log_si_n < -LOG10_SI_TOL && (total_nacl - n_na) > AMOUNT_EPS;
        let under_c = log_si_c < -LOG10_SI_TOL && (total_cacl2 - n_ca) > AMOUNT_EPS;
        if !over_n && !over_c && !under_n && !under_c {
            break;
        }
        if over_n && over_c {
            if log_si_n >= log_si_c {
                n_na = dissolved_nacl_at_si1(total_nacl, n_ca, n_h, n_oh, litres, temperature_c);
            } else {
                n_ca = dissolved_cacl2_at_si1(total_cacl2, n_na, n_h, n_oh, litres, temperature_c);
            }
        } else if over_n || under_n {
            n_na = dissolved_nacl_at_si1(total_nacl, n_ca, n_h, n_oh, litres, temperature_c);
        } else {
            n_ca = dissolved_cacl2_at_si1(total_cacl2, n_na, n_h, n_oh, litres, temperature_c);
        }
    }
    n_na = n_na.clamp(0.0, total_nacl);
    n_ca = n_ca.clamp(0.0, total_cacl2);
    (
        n_na,
        (total_nacl - n_na).max(0.0),
        n_ca,
        (total_cacl2 - n_ca).max(0.0),
    )
}

#[cfg(test)]
mod tests {
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
}
