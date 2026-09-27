//! Mixed NaCl / CaCl₂ / Na₂SO₄ / CaSO₄ saturation for every aqueous vessel.
//!
//! Pure-water solubilities are literature g / 100 g H₂O (chlorides, Na₂SO₄) or a
//! school gypsum solubility (CaSO₄), converted with ρ_water = 1.000 g/mL so
//! concentrations are **mol per litre of liquid water**. Chloride salts share Cl⁻;
//! gypsum uses Ca²⁺×SO₄²⁻. SiO₂ / sand never enters the ion balance.
//!
//! **Activity model (Davies + soft-cap I):**
//! `log₁₀ γᵢ = −A zᵢ² [√I_eff/(1+√I_eff) − 0.3 I_eff]` with
//! `I_eff = I / (1 + I / I_STAR)`, `I_STAR = 3.0` mol/kg, and
//! `I = m_Na_salt + 3 m_Ca + m_H_total + m_OH + m_SO4` (all aqueous H⁺, including
//! free sulfuric protons; SO₄²⁻ contributes as m_SO4 with z²=4 folded into the
//! coefficient choice above)
//! (Debye–Hückel A(T) from Malmberg–Maryott ε_r(water), ρ = 1.000 g/cm³).
//! **HCl common-ion** uses `min(n(h+), n_Cl_total)`, never bare `h+`, so sulfuric
//! acid cannot invent chloride. Chloride IAP uses partitioned salt Na/Ca + HCl
//! inventory only — sulfate Na is split out before the chloride SI pass so
//! `m_cl` is not understated by a `-2 m_SO4` term. Solvent basis for SI stays
//! **water litres**.

use crate::composition::{aqueous_mol, set_aqueous_mol};
use crate::scene::{
    CompositionEntry, SceneItem, CACL2_MOLAR_MASS_G_PER_MOL, CASO4_MOLAR_MASS_G_PER_MOL,
    NA2SO4_MOLAR_MASS_G_PER_MOL, NACL_MOLAR_MASS_G_PER_MOL,
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

/// Anhydrous Na₂SO₄ g / 100 g water vs T (school / CRC-style; hydrate collapse).
const NA2SO4_G_PER_100G_WATER: &[(f64, f64)] = &[
    (20.0, 19.5),
    (30.0, 40.8),
    (40.0, 48.8),
    (60.0, 45.3),
    (80.0, 43.7),
    (100.0, 42.5),
];

/// Gypsum-style CaSO₄ solubility (mol per litre water) vs T — school table.
const CASO4_MOL_PER_L: &[(f64, f64)] = &[
    (20.0, 0.015),
    (40.0, 0.014),
    (60.0, 0.013),
    (80.0, 0.012),
    (100.0, 0.011),
];

const AMOUNT_EPS: f64 = 1e-12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Salt {
    Nacl,
    Cacl2,
    Na2so4,
    Caso4,
}

/// Pure-water formula-unit solubility (mol per litre of water) at vessel T.
///
/// Piecewise-linear between literature table points; clamped to 20–100 °C.
pub fn solubility_mol_per_l(salt: Salt, temperature_c: f64) -> f64 {
    match salt {
        Salt::Nacl => {
            interpolate_g_per_100g(NACL_G_PER_100G_WATER, temperature_c) * 10.0
                / NACL_MOLAR_MASS_G_PER_MOL
        }
        Salt::Cacl2 => {
            interpolate_g_per_100g(CACL2_G_PER_100G_WATER, temperature_c) * 10.0
                / CACL2_MOLAR_MASS_G_PER_MOL
        }
        Salt::Na2so4 => {
            interpolate_g_per_100g(NA2SO4_G_PER_100G_WATER, temperature_c) * 10.0
                / NA2SO4_MOLAR_MASS_G_PER_MOL
        }
        Salt::Caso4 => interpolate_mol_per_l(CASO4_MOL_PER_L, temperature_c),
    }
}

fn interpolate_mol_per_l(table: &[(f64, f64)], temperature_c: f64) -> f64 {
    interpolate_g_per_100g(table, temperature_c)
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

pub use crate::composition::{liquid_water_ml, sync_fill_ml};

pub fn dish_has_liquid(item: &SceneItem) -> bool {
    liquid_water_ml(item) > AMOUNT_EPS || crate::h2so4::liquid_h2so4_ml(item) > AMOUNT_EPS
}

/// Liquid water (H₂O) in the dish — burner heat gate. Liquid molecular H₂SO₄ alone
/// does not count; the burner auto-offs (and stays off) when water is gone.
pub fn dish_has_liquid_water(item: &SceneItem) -> bool {
    liquid_water_ml(item) > AMOUNT_EPS
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
/// inventory at `temperature_c` before mixed SI = 1 (0 if already saturated).
///
/// HCl common-ion is `min(max(n_h_aq − n_oh_aq, 0), n_cl_aq)` — same free-H⁺ gate as
/// VLE / Φ_V inventory. Bare sulfuric `h+` / `hso4-` never invents Cl⁻; pass free
/// `h+` (not collapsed bisulfate) as `n_h_aq`. `n_h_aq` and `n_so4_aq` still
/// contribute to ionic strength. `n_oh_aq` contributes to I only (not to NaCl/CaCl₂
/// IAP). Holds the other cation fixed (no precipitation sidelight during the probe)
/// so filter wash capacity matches common-ion suppression.
#[allow(clippy::too_many_arguments)]
pub(crate) fn unsaturated_capacity_g(
    salt: Salt,
    water_ml: f64,
    n_na_aq: f64,
    n_ca_aq: f64,
    n_h_aq: f64,
    n_cl_aq: f64,
    n_oh_aq: f64,
    n_so4_aq: f64,
    temperature_c: f64,
) -> f64 {
    let litres = water_ml / 1000.0;
    if litres <= AMOUNT_EPS {
        return 0.0;
    }
    let n_h = n_h_aq.max(0.0);
    let n_oh = n_oh_aq.max(0.0);
    let n_hcl = (n_h - n_oh).max(0.0).min(n_cl_aq.max(0.0));
    let n_so4 = n_so4_aq.max(0.0);
    match salt {
        Salt::Nacl => {
            let pure_max = solubility_mol_per_l(Salt::Nacl, temperature_c) * litres;
            let probe = n_na_aq.max(0.0) + pure_max * 2.0 + 1.0;
            let max_aq = dissolved_nacl_at_si1(
                probe,
                n_ca_aq.max(0.0),
                n_so4,
                n_hcl,
                n_h,
                n_oh,
                litres,
                temperature_c,
            );
            (max_aq - n_na_aq).max(0.0) * NACL_MOLAR_MASS_G_PER_MOL
        }
        Salt::Cacl2 => {
            let pure_max = solubility_mol_per_l(Salt::Cacl2, temperature_c) * litres;
            let probe = n_ca_aq.max(0.0) + pure_max * 2.0 + 1.0;
            let max_aq = dissolved_cacl2_at_si1(
                probe,
                n_na_aq.max(0.0),
                n_so4,
                n_hcl,
                n_h,
                n_oh,
                litres,
                temperature_c,
            );
            (max_aq - n_ca_aq).max(0.0) * CACL2_MOLAR_MASS_G_PER_MOL
        }
        // Filter wash does not currently dissolve sulfate solids.
        Salt::Na2so4 | Salt::Caso4 => 0.0,
    }
}

/// Convert aqueous ↔ solid so saturation indices are ≤ 1 at the item's current T.
///
/// Mixed NaCl/CaCl₂/Na₂SO₄/CaSO₄ equilibrium on every aqueous vessel. Dry stock solids
/// (no water, no aqueous ions, not an evaporation dish) are left untouched.
///
/// **Charge-safe Cl⁻:** total chloride is conserved from aq Cl⁻ + solid NaCl/CaCl₂.
/// HCl common-ion for SI is `min(n(h+), total_cl)` — bare sulfuric `h+` never invents Cl⁻.
/// Sulfate is conserved across aq SO₄²⁻ + solid Na₂SO₄/CaSO₄.
///
/// **NaOH / OH⁻:** aqueous Na⁺ paired 1:1 with excess OH⁻ (above K_w residual) is
/// **not** counted as NaCl/Na₂SO₄ inventory. Callers must run acid–base speciation
/// before this when acid and base are present.
///
/// **HSO₄⁻:** collapsed into free `h+` + `so4^2-` for the SI mass balance so gypsum
/// / Na₂SO₄ can pull total sulfur; a post-SI speciate restores Kₐ₂.
pub fn enforce_saturation(item: &mut SceneItem) {
    let temperature_c = item.properties.temperature_c.unwrap_or(20.0);
    let water_ml = liquid_water_ml(item);
    let litres = water_ml / 1000.0;

    if litres <= AMOUNT_EPS && item.kind != "evaporation_dish" && !has_aqueous_ions(item) {
        return;
    }

    let n_oh = aqueous_mol(item, "oh-");
    let n_h_free = aqueous_mol(item, "h+");
    let n_hso4 = aqueous_mol(item, "hso4-");
    // Collapse bisulfate so SI can consume total sulfur; bound H becomes free H⁺.
    let n_h = n_h_free + n_hso4;
    let s_nacl = solid_mol(item, "nacl", NACL_MOLAR_MASS_G_PER_MOL);
    let s_cacl2 = solid_mol(item, "cacl2", CACL2_MOLAR_MASS_G_PER_MOL);
    let s_na2so4 = solid_mol(item, "na2so4", NA2SO4_MOLAR_MASS_G_PER_MOL);
    let s_caso4 = solid_mol(item, "caso4", CASO4_MOLAR_MASS_G_PER_MOL);

    let total_cl = aqueous_mol(item, "cl-") + s_nacl + 2.0 * s_cacl2;
    let total_so4 = aqueous_mol(item, "so4^2-") + n_hso4 + s_na2so4 + s_caso4;
    let total_ca = aqueous_mol(item, "ca2+") + s_cacl2 + s_caso4;
    let n_na_total = aqueous_mol(item, "na+");
    // Excess OH above free H marks strong-base NaOH (post-K_w both are present).
    let n_naoh = if n_oh > n_h_free + 1e-9 {
        (n_oh - n_h_free).max(0.0)
    } else {
        0.0
    };
    let total_na_salt = (n_na_total - n_naoh).max(0.0) + s_nacl + 2.0 * s_na2so4;
    // HCl common-ion matches VLE inventory: free excess H⁺ only (not hso4-).
    let n_h_excess = (n_h_free - n_oh).max(0.0);
    let n_hcl = n_h_excess.min(total_cl).max(0.0);

    let (aq_na_salt, aq_ca, aq_so4, solid_nacl, solid_cacl2, solid_na2so4, solid_caso4) =
        if litres <= AMOUNT_EPS {
            dry_sulfate_chloride_split(total_na_salt, total_ca, total_so4, total_cl)
        } else {
            mixed_sulfate_chloride_equilibrium(
                total_na_salt,
                total_ca,
                total_so4,
                total_cl,
                n_hcl,
                n_h,
                n_oh,
                litres,
                temperature_c,
            )
        };

    let aq_cl = (total_cl - solid_nacl - 2.0 * solid_cacl2).max(0.0);

    set_aqueous_mol(item, "na+", aq_na_salt + n_naoh);
    set_aqueous_mol(item, "ca2+", aq_ca);
    set_aqueous_mol(item, "so4^2-", aq_so4);
    set_aqueous_mol(item, "hso4-", 0.0); // restored by post-SI speciate
    set_aqueous_mol(item, "cl-", aq_cl);
    set_aqueous_mol(item, "h+", n_h);
    set_aqueous_mol(item, "oh-", n_oh);
    set_salt_solid(item, "nacl", solid_nacl, NACL_MOLAR_MASS_G_PER_MOL);
    set_salt_solid(item, "cacl2", solid_cacl2, CACL2_MOLAR_MASS_G_PER_MOL);
    set_salt_solid(item, "na2so4", solid_na2so4, NA2SO4_MOLAR_MASS_G_PER_MOL);
    set_salt_solid(item, "caso4", solid_caso4, CASO4_MOLAR_MASS_G_PER_MOL);
    remove_near_zero_water(item);
    sync_fill_ml(item);
}

/// Dry-out split: prefer CaSO₄, then Na₂SO₄, then chloride solids.
///
/// Leftover free SO₄²⁻ (e.g. non-volatile H₂SO₄ with no Ca/Na partner) stays as
/// aqueous inventory — same conservation rule as leftover aq HCl ions when dry.
fn dry_sulfate_chloride_split(
    total_na_salt: f64,
    total_ca: f64,
    total_so4: f64,
    total_cl: f64,
) -> (f64, f64, f64, f64, f64, f64, f64) {
    let solid_caso4 = total_ca.min(total_so4).max(0.0);
    let ca_left = (total_ca - solid_caso4).max(0.0);
    let so4_left = (total_so4 - solid_caso4).max(0.0);
    let solid_na2so4 = (total_na_salt * 0.5).min(so4_left).max(0.0);
    let na_left = (total_na_salt - 2.0 * solid_na2so4).max(0.0);
    let so4_aq = (so4_left - solid_na2so4).max(0.0);
    // Remaining Ca / Na with Cl → chloride solids.
    let solid_cacl2 = ca_left.min(total_cl * 0.5).max(0.0);
    let cl_after_ca = (total_cl - 2.0 * solid_cacl2).max(0.0);
    let solid_nacl = na_left.min(cl_after_ca).max(0.0);
    (
        0.0,
        0.0,
        so4_aq,
        solid_nacl,
        solid_cacl2,
        solid_na2so4,
        solid_caso4,
    )
}

/// log₁₀(SI) tolerance ≈ SI within 10⁻⁸ of 1.
const LOG10_SI_TOL: f64 = 1e-8;
const MIXED_OUTER_ITERS: usize = 64;
const MIXED_BISECT_ITERS: usize = 80;

#[derive(Clone, Copy)]
struct Mixture {
    m_na: f64,
    m_ca: f64,
    /// Molality of H⁺ counted as HCl common-ion (`min(max(n_h−n_oh,0), n_cl)`) for `m_cl`.
    m_hcl: f64,
    /// Molality of all aqueous H⁺ (HCl + free sulfuric protons) for ionic strength.
    m_h: f64,
    /// Molality of OH⁻ (contributes to I only; not to chloride IAP / `m_cl`).
    m_oh: f64,
    m_so4: f64,
}

impl Mixture {
    fn from_moles(
        n_na: f64,
        n_ca: f64,
        n_so4: f64,
        n_hcl: f64,
        n_h_total: f64,
        n_oh: f64,
        litres: f64,
    ) -> Self {
        let n_hcl = n_hcl.max(0.0);
        let n_h = n_h_total.max(n_hcl);
        Self {
            m_na: (n_na / litres).max(0.0),
            m_ca: (n_ca / litres).max(0.0),
            m_hcl: (n_hcl / litres).max(0.0),
            m_h: (n_h / litres).max(0.0),
            m_oh: (n_oh / litres).max(0.0),
            m_so4: (n_so4 / litres).max(0.0),
        }
    }

    fn m_cl(self) -> f64 {
        // Chloride molality from Cl-bearing inventory only. Callers partition
        // sulfate-paired Na out of `m_na` before chloride SI, so do **not**
        // subtract `2·m_so4` here (that understates Cl when SO₄ Na is excluded).
        (self.m_na + 2.0 * self.m_ca + self.m_hcl).max(0.0)
    }

    fn ionic_strength(self) -> f64 {
        // I = m_na + 3 m_ca + m_h_total + m_oh + m_so4 (see module docs).
        self.m_na + 3.0 * self.m_ca + self.m_h + self.m_oh + self.m_so4
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
            m_hcl: 0.0,
            m_h: 0.0,
            m_oh: 0.0,
            m_so4: 0.0,
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
            m_hcl: 0.0,
            m_h: 0.0,
            m_oh: 0.0,
            m_so4: 0.0,
        },
        a_dh,
    )
    .expect("pure CaCl2 solubility is positive")
}

fn log10_iap_na2so4(mix: Mixture, a_dh: f64) -> Option<f64> {
    if mix.m_na <= 0.0 || mix.m_so4 <= 0.0 {
        return None;
    }
    let i = mix.ionic_strength();
    // IAP = (γ_Na m_Na)² (γ_SO4 m_SO4)
    Some(
        2.0 * log10_gamma(1, i, a_dh)
            + 2.0 * mix.m_na.log10()
            + log10_gamma(2, i, a_dh)
            + mix.m_so4.log10(),
    )
}

fn log10_iap_caso4(mix: Mixture, a_dh: f64) -> Option<f64> {
    if mix.m_ca <= 0.0 || mix.m_so4 <= 0.0 {
        return None;
    }
    let i = mix.ionic_strength();
    Some(log10_gamma(2, i, a_dh) + mix.m_ca.log10() + log10_gamma(2, i, a_dh) + mix.m_so4.log10())
}

fn log10_k_na2so4(temperature_c: f64) -> f64 {
    let s = solubility_mol_per_l(Salt::Na2so4, temperature_c);
    let a_dh = debye_huckel_a(temperature_c);
    log10_iap_na2so4(
        Mixture {
            m_na: 2.0 * s,
            m_ca: 0.0,
            m_hcl: 0.0,
            m_h: 0.0,
            m_oh: 0.0,
            m_so4: s,
        },
        a_dh,
    )
    .expect("pure Na2SO4 solubility is positive")
}

fn log10_k_caso4(temperature_c: f64) -> f64 {
    let s = solubility_mol_per_l(Salt::Caso4, temperature_c);
    let a_dh = debye_huckel_a(temperature_c);
    log10_iap_caso4(
        Mixture {
            m_na: 0.0,
            m_ca: s,
            m_hcl: 0.0,
            m_h: 0.0,
            m_oh: 0.0,
            m_so4: s,
        },
        a_dh,
    )
    .expect("pure CaSO4 solubility is positive")
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

#[allow(dead_code)]
fn log10_si_na2so4(mix: Mixture, temperature_c: f64) -> f64 {
    match log10_iap_na2so4(mix, debye_huckel_a(temperature_c)) {
        None => f64::NEG_INFINITY,
        Some(log_iap) => log_iap - log10_k_na2so4(temperature_c),
    }
}

#[allow(dead_code)]
fn log10_si_caso4(mix: Mixture, temperature_c: f64) -> f64 {
    match log10_iap_caso4(mix, debye_huckel_a(temperature_c)) {
        None => f64::NEG_INFINITY,
        Some(log_iap) => log_iap - log10_k_caso4(temperature_c),
    }
}

#[allow(clippy::too_many_arguments)]
fn dissolved_nacl_at_si1(
    n_na_tot: f64,
    n_ca: f64,
    n_so4: f64,
    n_hcl: f64,
    n_h_total: f64,
    n_oh: f64,
    litres: f64,
    temperature_c: f64,
) -> f64 {
    let a_dh = debye_huckel_a(temperature_c);
    let log_k = log10_k_nacl(temperature_c);
    let log_si = |n_na: f64| {
        log10_iap_nacl(
            Mixture::from_moles(n_na, n_ca, n_so4, n_hcl, n_h_total, n_oh, litres),
            a_dh,
        )
        .map(|log_iap| log_iap - log_k)
        .unwrap_or(f64::NEG_INFINITY)
    };
    bisect_dissolved(n_na_tot, log_si)
}

#[allow(clippy::too_many_arguments)]
fn dissolved_cacl2_at_si1(
    n_ca_tot: f64,
    n_na: f64,
    n_so4: f64,
    n_hcl: f64,
    n_h_total: f64,
    n_oh: f64,
    litres: f64,
    temperature_c: f64,
) -> f64 {
    let a_dh = debye_huckel_a(temperature_c);
    let log_k = log10_k_cacl2(temperature_c);
    let log_si = |n_ca: f64| {
        log10_iap_cacl2(
            Mixture::from_moles(n_na, n_ca, n_so4, n_hcl, n_h_total, n_oh, litres),
            a_dh,
        )
        .map(|log_iap| log_iap - log_k)
        .unwrap_or(f64::NEG_INFINITY)
    };
    bisect_dissolved(n_ca_tot, log_si)
}

#[allow(clippy::too_many_arguments)]
fn dissolved_na2so4_at_si1(
    n_so4_tot: f64,
    n_na_budget: f64,
    n_ca: f64,
    n_hcl: f64,
    n_h_total: f64,
    n_oh: f64,
    litres: f64,
    temperature_c: f64,
) -> f64 {
    // n_so4_tot is the SO₄ pool available for Na₂SO₄; each unit needs 2 Na⁺.
    // IAP uses the full Na budget (incl. coexisting NaCl Na) for common-ion.
    let max_units = n_so4_tot.min(n_na_budget * 0.5).max(0.0);
    let a_dh = debye_huckel_a(temperature_c);
    let log_k = log10_k_na2so4(temperature_c);
    let log_si = |n_so4: f64| {
        log10_iap_na2so4(
            Mixture::from_moles(n_na_budget, n_ca, n_so4, n_hcl, n_h_total, n_oh, litres),
            a_dh,
        )
        .map(|log_iap| log_iap - log_k)
        .unwrap_or(f64::NEG_INFINITY)
    };
    bisect_dissolved(max_units, log_si)
}

/// Max aqueous CaSO₄ formula-unit pairs at SI ≈ 1 (common-ion aware).
#[allow(clippy::too_many_arguments)]
fn dissolved_caso4_pair_at_si1(
    n_ca_tot: f64,
    n_so4_tot: f64,
    n_na: f64,
    n_hcl: f64,
    n_h_total: f64,
    n_oh: f64,
    litres: f64,
    temperature_c: f64,
) -> f64 {
    let max_pair = n_ca_tot.min(n_so4_tot).max(0.0);
    let excess_ca = (n_ca_tot - n_so4_tot).max(0.0);
    let excess_so4 = (n_so4_tot - n_ca_tot).max(0.0);
    let a_dh = debye_huckel_a(temperature_c);
    let log_k = log10_k_caso4(temperature_c);
    let log_si_aq = |aq_pair: f64| {
        let n_ca = aq_pair + excess_ca;
        let n_so4 = aq_pair + excess_so4;
        log10_iap_caso4(
            Mixture::from_moles(n_na, n_ca, n_so4, n_hcl, n_h_total, n_oh, litres),
            a_dh,
        )
        .map(|log_iap| log_iap - log_k)
        .unwrap_or(f64::NEG_INFINITY)
    };
    bisect_dissolved(max_pair, log_si_aq)
}

fn bisect_dissolved(n_tot: f64, log_si: impl Fn(f64) -> f64) -> f64 {
    if n_tot <= AMOUNT_EPS {
        return 0.0;
    }
    if log_si(n_tot) <= LOG10_SI_TOL {
        return n_tot;
    }
    let mut lo = 0.0;
    let mut hi = n_tot;
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

/// Returns `(aq_na_salt, aq_ca, aq_so4, solid_nacl, solid_cacl2, solid_na2so4, solid_caso4)`.
#[allow(clippy::too_many_arguments)]
fn mixed_sulfate_chloride_equilibrium(
    total_na_salt: f64,
    total_ca: f64,
    total_so4: f64,
    total_cl: f64,
    n_hcl: f64,
    n_h_total: f64,
    n_oh: f64,
    litres: f64,
    temperature_c: f64,
) -> (f64, f64, f64, f64, f64, f64, f64) {
    let total_na_salt = total_na_salt.max(0.0);
    let total_ca = total_ca.max(0.0);
    let total_so4 = total_so4.max(0.0);
    let total_cl = total_cl.max(0.0);
    let n_hcl = n_hcl.min(total_cl).max(0.0);
    let n_h_total = n_h_total.max(n_hcl);
    let n_oh = n_oh.max(0.0);

    // 1) Gypsum: precipitate excess Ca×SO₄ beyond SI ≈ 1.
    let aq_caso4_pair = dissolved_caso4_pair_at_si1(
        total_ca,
        total_so4,
        0.0,
        n_hcl,
        n_h_total,
        n_oh,
        litres,
        temperature_c,
    );
    let max_pair = total_ca.min(total_so4);
    let solid_caso4 = (max_pair - aq_caso4_pair).max(0.0);
    let excess_ca = (total_ca - total_so4).max(0.0);
    let excess_so4 = (total_so4 - total_ca).max(0.0);
    let mut aq_ca = aq_caso4_pair + excess_ca;
    let mut aq_so4 = aq_caso4_pair + excess_so4;
    // Numerical clamp to totals.
    aq_ca = aq_ca.min(total_ca - solid_caso4).max(0.0);
    aq_so4 = aq_so4.min(total_so4 - solid_caso4).max(0.0);

    // 2) Na₂SO₄ SI on remaining SO₄ and Na budget.
    let aq_na2so4_cap = dissolved_na2so4_at_si1(
        aq_so4,
        total_na_salt,
        aq_ca,
        n_hcl,
        n_h_total,
        n_oh,
        litres,
        temperature_c,
    );
    let max_na2so4 = aq_so4.min(total_na_salt * 0.5);
    let solid_na2so4 = (max_na2so4 - aq_na2so4_cap).max(0.0);
    aq_so4 = (aq_so4 - solid_na2so4).max(0.0);
    let na_after_solid = (total_na_salt - 2.0 * solid_na2so4).max(0.0);
    let aq_na2so4_units = aq_so4.min(na_after_solid * 0.5).min(aq_na2so4_cap);
    let n_na_for_chloride = (na_after_solid - 2.0 * aq_na2so4_units).max(0.0);
    let n_ca_for_chloride = aq_ca;

    // 3) Chloride SI — Cl reserved for HCl inventory first.
    // `m_na` here is chloride-budget Na only; SO₄ still contributes to I via m_so4,
    // while `m_cl` no longer subtracts 2·m_so4 (Na already partitioned).
    let cl_for_salts = (total_cl - n_hcl).max(0.0);
    let total_nacl = n_na_for_chloride.min(cl_for_salts);
    let cl_after_nacl_cap = (cl_for_salts - total_nacl).max(0.0);
    let total_cacl2 = n_ca_for_chloride.min(cl_after_nacl_cap * 0.5);

    let mut n_na = total_nacl;
    let mut n_ca = total_cacl2;
    for _ in 0..MIXED_OUTER_ITERS {
        let mix = Mixture::from_moles(n_na, n_ca, aq_so4, n_hcl, n_h_total, n_oh, litres);
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
                n_na = dissolved_nacl_at_si1(
                    total_nacl,
                    n_ca,
                    aq_so4,
                    n_hcl,
                    n_h_total,
                    n_oh,
                    litres,
                    temperature_c,
                );
            } else {
                n_ca = dissolved_cacl2_at_si1(
                    total_cacl2,
                    n_na,
                    aq_so4,
                    n_hcl,
                    n_h_total,
                    n_oh,
                    litres,
                    temperature_c,
                );
            }
        } else if over_n || under_n {
            n_na = dissolved_nacl_at_si1(
                total_nacl,
                n_ca,
                aq_so4,
                n_hcl,
                n_h_total,
                n_oh,
                litres,
                temperature_c,
            );
        } else {
            n_ca = dissolved_cacl2_at_si1(
                total_cacl2,
                n_na,
                aq_so4,
                n_hcl,
                n_h_total,
                n_oh,
                litres,
                temperature_c,
            );
        }
    }
    n_na = n_na.clamp(0.0, total_nacl);
    n_ca = n_ca.clamp(0.0, total_cacl2);
    let solid_nacl = (total_nacl - n_na).max(0.0);
    let solid_cacl2 = (total_cacl2 - n_ca).max(0.0);

    let aq_ca_out = (n_ca + (n_ca_for_chloride - total_cacl2).max(0.0)).min(aq_ca);
    let aq_na_salt = n_na + 2.0 * aq_na2so4_units;

    (
        aq_na_salt,
        aq_ca_out,
        aq_so4,
        solid_nacl,
        solid_cacl2,
        solid_na2so4,
        solid_caso4,
    )
}

#[cfg(test)]
#[path = "solubility_tests.rs"]
mod tests;
