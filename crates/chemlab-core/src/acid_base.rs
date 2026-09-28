//! Full aqueous acid–base speciation for the Free-mode ion set.
//!
//! Charge- and mass-balanced solver for species the engine authors:
//! `h+`, `oh-`, `cl-`, `na+`, `ca2+`, `so4^2-`, `hso4-`.
//!
//! Equilibria (school constants):
//! - Water: `K_w = [H⁺][OH⁻]`
//! - Sulfuric second step: `K_a2 = [H⁺][SO₄²⁻]/[HSO₄⁻]`
//!
//! Strong HCl / NaOH are fully dissociated spectators (`Cl⁻` / `Na⁺`); free
//! `[H⁺]` / `[OH⁻]` come from the solver. Neutralization heat is applied from
//! the H⁺+OH⁻ (and HSO₄⁻+OH⁻) extent implied by the pre-solve inventory.

use crate::composition::{aqueous_mol, liquid_water_ml, set_aqueous_mol};
use crate::scene::{
    CompositionEntry, SceneItem, H_OH_NEUTRALIZATION_J_PER_MOL, WATER_MOLAR_MASS_G_PER_MOL,
};

const AMOUNT_EPS: f64 = 1e-12;

/// School water ion-product at ~25 °C (mol²/L²). Mild T dependence is OOS.
pub const KW: f64 = 1.0e-14;

/// School HSO₄⁻ acid dissociation constant (mol/L).
pub const KA2_HSO4: f64 = 0.012;

/// Charge residual `f([H⁺]) = 0` for the aqueous speciation system.
///
/// Conserved: `n_Na`, `n_Ca`, `n_Cl`, `n_S = n_HSO4 + n_SO4`.
/// Solved: `n_H`, `n_OH`, `n_HSO4`, `n_SO4` via `K_w`, `K_a2`, and charge.
fn charge_residual(c_h: f64, v_l: f64, n_na: f64, n_ca: f64, n_cl: f64, n_s: f64) -> f64 {
    let c_h = c_h.max(1e-30);
    let n_h = c_h * v_l;
    let n_oh = (KW * v_l * v_l) / n_h;
    let (n_hso4, n_so4) = sulfate_split(n_s, c_h, v_l);
    // Cations − anions.
    n_h + n_na + 2.0 * n_ca - n_oh - n_cl - n_hso4 - 2.0 * n_so4
}

/// Split total sulfur moles into HSO₄⁻ / SO₄²⁻ at a given `[H⁺]`.
fn sulfate_split(n_s: f64, c_h: f64, v_l: f64) -> (f64, f64) {
    if n_s <= AMOUNT_EPS || v_l <= AMOUNT_EPS {
        return (0.0, 0.0);
    }
    let c_h = c_h.max(1e-30);
    // K_a2 = c_h * c_so4 / c_hso4, c_hso4 + c_so4 = c_s
    // ⇒ c_so4 = c_s * K_a2 / (K_a2 + c_h)
    let n_so4 = n_s * KA2_HSO4 / (KA2_HSO4 + c_h);
    let n_hso4 = (n_s - n_so4).max(0.0);
    (n_hso4, n_so4)
}

/// Solve for `[H⁺]` by bisection on `log₁₀ c_h`.
fn solve_c_h(v_l: f64, n_na: f64, n_ca: f64, n_cl: f64, n_s: f64) -> f64 {
    if v_l <= AMOUNT_EPS {
        return KW.sqrt();
    }
    // Bracket: very basic → very acidic (covers stock HCl ~10 M).
    let mut lo = -16.0_f64;
    let mut hi = 2.0_f64;
    let mut f_lo = charge_residual(10f64.powf(lo), v_l, n_na, n_ca, n_cl, n_s);
    let f_hi = charge_residual(10f64.powf(hi), v_l, n_na, n_ca, n_cl, n_s);
    if f_lo * f_hi > 0.0 {
        // Fall back: pure-water guess adjusted by strong-acid excess.
        let excess = n_cl + n_s - n_na - 2.0 * n_ca; // rough strong-acid meq
        if excess > AMOUNT_EPS {
            return (excess / v_l).max(KW.sqrt());
        }
        if excess < -AMOUNT_EPS {
            let c_oh = ((-excess) / v_l).max(KW.sqrt());
            return KW / c_oh;
        }
        return KW.sqrt();
    }
    for _ in 0..80 {
        let mid = 0.5 * (lo + hi);
        let f_mid = charge_residual(10f64.powf(mid), v_l, n_na, n_ca, n_cl, n_s);
        if f_lo.is_sign_positive() == f_mid.is_sign_positive() {
            lo = mid;
            f_lo = f_mid;
        } else {
            hi = mid;
        }
        if (hi - lo).abs() < 1e-10 {
            break;
        }
    }
    10f64.powf(0.5 * (lo + hi))
}

fn add_water_ml(item: &mut SceneItem, ml: f64) {
    if ml <= AMOUNT_EPS {
        return;
    }
    let next = liquid_water_ml(item) + ml;
    if let Some(existing) = item
        .properties
        .composition
        .iter_mut()
        .find(|c| c.substance_id == "water" && c.phase == "liquid")
    {
        existing.amount_ml = Some(next);
        return;
    }
    item.properties.composition.push(CompositionEntry {
        substance_id: "water".into(),
        phase: "liquid".into(),
        amount_ml: Some(next),
        amount_scoop: None,
        amount_g: None,
        amount_mol: None,
    });
}

/// Speciate aqueous acid/base in `item`: write equilibrated `h+`/`oh-`/`hso4-`/`so4^2-`.
///
/// When `apply_heat` is true, also forms water and applies neutralization ΔH from the
/// H⁺+OH⁻ / HSO₄⁻+OH⁻ extent: `0.5 · Δ(n_h + n_oh + n_hso4)` across the solve
/// (subsumes the old instant neutralization step). Bound bisulfate must be in the
/// sum — otherwise HSO₄⁻+OH⁻ → SO₄²⁻+H₂O undercounts heat by ~2×. Re-equilibrating
/// already-solved Kw water does not invent water or heat.
///
/// Concentrations use **liquid water** ml as the solvent basis (not Φ_V solution
/// volume, which can include non-aqueous liquid H₂SO₄). No-ops when dry.
///
/// Returns `true` when neutralization heat discarded spit mass.
///
/// `allow_spit_mass` is ignored when `apply_heat` is false; on heat, it gates
/// spray discard (false on clock ticks).
pub fn speciate_aqueous_acid_base(
    item: &mut SceneItem,
    apply_heat: bool,
    allow_spit_mass: bool,
) -> bool {
    let water_ml = liquid_water_ml(item);
    // Dry or empty: leave ion piles for reform / SI; strip solvent-only Kw ions.
    if water_ml <= AMOUNT_EPS {
        let n_cl = aqueous_mol(item, "cl-");
        let n_s = aqueous_mol(item, "so4^2-") + aqueous_mol(item, "hso4-");
        set_aqueous_mol(item, "oh-", 0.0);
        if n_cl <= AMOUNT_EPS && n_s <= AMOUNT_EPS {
            set_aqueous_mol(item, "h+", 0.0);
        }
        return false;
    }
    let v_l = water_ml / 1000.0;

    let n_h_before = aqueous_mol(item, "h+");
    let n_oh_before = aqueous_mol(item, "oh-");
    let n_hso4_before = aqueous_mol(item, "hso4-");

    let n_na = aqueous_mol(item, "na+");
    let n_ca = aqueous_mol(item, "ca2+");
    let n_cl = aqueous_mol(item, "cl-");
    let n_s = aqueous_mol(item, "so4^2-") + aqueous_mol(item, "hso4-");

    let c_h = solve_c_h(v_l, n_na, n_ca, n_cl, n_s);
    let n_h = c_h * v_l;
    let n_oh = (KW * v_l * v_l) / n_h.max(1e-30);
    let (n_hso4, n_so4) = sulfate_split(n_s, c_h, v_l);

    set_aqueous_mol(item, "h+", n_h);
    set_aqueous_mol(item, "oh-", n_oh);
    set_aqueous_mol(item, "hso4-", n_hso4);
    set_aqueous_mol(item, "so4^2-", n_so4);

    if apply_heat {
        // Each water formed (H⁺+OH⁻ or HSO₄⁻+OH⁻) drops (n_h + n_oh + n_hso4) by 2.
        let acid_base_before = n_h_before + n_oh_before + n_hso4_before;
        let acid_base_after = n_h + n_oh + n_hso4;
        let n_rxn = 0.5 * (acid_base_before - acid_base_after).max(0.0);
        if n_rxn > AMOUNT_EPS {
            add_water_ml(item, n_rxn * WATER_MOLAR_MASS_G_PER_MOL);
            let q = n_rxn * H_OH_NEUTRALIZATION_J_PER_MOL;
            return crate::aqueous_pipeline::apply_chemical_heat(item, q, allow_spit_mass);
        }
    }
    false
}

/// Net charge (mol e⁻) of aqueous ions — should be ~0 after speciation.
#[cfg(test)]
pub fn aqueous_charge_mol(item: &SceneItem) -> f64 {
    aqueous_mol(item, "h+") + aqueous_mol(item, "na+") + 2.0 * aqueous_mol(item, "ca2+")
        - aqueous_mol(item, "oh-")
        - aqueous_mol(item, "cl-")
        - aqueous_mol(item, "hso4-")
        - 2.0 * aqueous_mol(item, "so4^2-")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hcl::{ph_of_entries, solution_volume_ml};
    use crate::scene::{effective_heat_capacity, CompositionEntry, ItemProperties};

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

    fn vessel(entries: Vec<CompositionEntry>) -> SceneItem {
        SceneItem {
            id: "vessel".into(),
            kind: "beaker".into(),
            label: "test".into(),
            location: "bench".into(),
            properties: ItemProperties {
                volume_ml: Some(250.0),
                fill_ml: Some(100.0),
                transparent: Some(true),
                colourless: Some(true),
                temperature_c: Some(20.0),
                composition: entries,
                ..ItemProperties::default()
            },
        }
    }

    #[test]
    fn pure_water_speciates_near_ph_7() {
        let mut item = vessel(vec![water(100.0)]);
        speciate_aqueous_acid_base(&mut item, false, false);
        let ph = ph_of_entries(&item.properties.composition).expect("pH");
        assert!((ph - 7.0).abs() < 0.05, "got pH {ph}");
        assert!(aqueous_charge_mol(&item).abs() < 1e-12);
    }

    #[test]
    fn dilute_h2so4_ka2_regime_not_fully_dissociated() {
        // 0.1 M H₂SO₄ in 1 L → c = 0.1; school [H+] ≈ 0.11 not 0.2.
        let c = 0.1;
        let mut item = vessel(vec![water(1000.0), aq("h+", c), aq("hso4-", c)]);
        speciate_aqueous_acid_base(&mut item, false, false);
        let v_l = solution_volume_ml(&item) / 1000.0;
        let c_h = aqueous_mol(&item, "h+") / v_l;
        assert!(
            c_h > 0.10 && c_h < 0.14,
            "expected Ka2 regime [H+]~0.11, got {c_h}"
        );
        assert!(aqueous_mol(&item, "hso4-") > aqueous_mol(&item, "so4^2-"));
        assert!(aqueous_charge_mol(&item).abs() < 1e-9);
        let ph = ph_of_entries(&item.properties.composition).unwrap();
        assert!((ph - (-c_h.log10())).abs() < 1e-9);
    }

    #[test]
    fn strong_hcl_and_naoh_extremes() {
        let mut acid = vessel(vec![water(100.0), aq("h+", 0.01), aq("cl-", 0.01)]);
        speciate_aqueous_acid_base(&mut acid, false, false);
        let ph_a = ph_of_entries(&acid.properties.composition).unwrap();
        assert!((ph_a - 1.0).abs() < 0.05, "0.1 M HCl → pH~1, got {ph_a}");

        let mut base = vessel(vec![water(100.0), aq("na+", 0.01), aq("oh-", 0.01)]);
        speciate_aqueous_acid_base(&mut base, false, false);
        let ph_b = ph_of_entries(&base.properties.composition).unwrap();
        assert!(ph_b > 11.0 && ph_b < 13.0, "got {ph_b}");
    }

    #[test]
    fn neutralization_heat_and_salt_near_neutral() {
        let n = 0.01;
        let mut item = vessel(vec![
            water(100.0),
            aq("h+", n),
            aq("cl-", n),
            aq("na+", n),
            aq("oh-", n),
        ]);
        let t0 = item.properties.temperature_c.unwrap();
        speciate_aqueous_acid_base(&mut item, true, true);
        let t1 = item.properties.temperature_c.unwrap();
        assert!(t1 > t0 + 0.5, "expected neutralization heat {t0} → {t1}");
        let ph = ph_of_entries(&item.properties.composition).unwrap();
        assert!((ph - 7.0).abs() < 0.1, "got pH {ph}");
        assert!(aqueous_charge_mol(&item).abs() < 1e-9);
    }

    #[test]
    fn bisulfate_plus_oh_neutralization_heat_matches_stoich() {
        // NaHSO₄ + NaOH → Na₂SO₄ + H₂O: extent must be ~n, not ~n/2.
        // (Needs 2 Na⁺ so the post-solve inventory is neutral sulfate salt.)
        let n = 0.01;
        let mut item = vessel(vec![
            water(100.0),
            aq("hso4-", n),
            aq("na+", 2.0 * n),
            aq("oh-", n),
        ]);
        let t0 = item.properties.temperature_c.unwrap();
        let c0 = effective_heat_capacity(&item);
        speciate_aqueous_acid_base(&mut item, true, true);
        let t1 = item.properties.temperature_c.unwrap();
        let dt = t1 - t0;
        let dt_expected = -n * H_OH_NEUTRALIZATION_J_PER_MOL / c0;
        assert!(
            (dt - dt_expected).abs() < 0.15 * dt_expected.abs(),
            "HSO₄⁻+OH⁻ heat extent: ΔT={dt} expected~{dt_expected}"
        );
        assert!(aqueous_mol(&item, "so4^2-") > n * 0.9);
        assert!(aqueous_charge_mol(&item).abs() < 1e-9);
    }

    #[test]
    fn h2so4_two_eq_naoh_neutralization_heat_matches_stoich() {
        // H⁺ + HSO₄⁻ + 2 OH⁻ → 2 H₂O + SO₄²⁻: extent ~2n (not 1.5n).
        let n = 0.01;
        let mut item = vessel(vec![
            water(100.0),
            aq("h+", n),
            aq("hso4-", n),
            aq("na+", 2.0 * n),
            aq("oh-", 2.0 * n),
        ]);
        let t0 = item.properties.temperature_c.unwrap();
        let c0 = effective_heat_capacity(&item);
        speciate_aqueous_acid_base(&mut item, true, true);
        let t1 = item.properties.temperature_c.unwrap();
        let dt = t1 - t0;
        let dt_expected = -2.0 * n * H_OH_NEUTRALIZATION_J_PER_MOL / c0;
        assert!(
            (dt - dt_expected).abs() < 0.15 * dt_expected.abs(),
            "2-eq H₂SO₄ heat: ΔT={dt} expected~{dt_expected}"
        );
        assert!(aqueous_charge_mol(&item).abs() < 1e-9);
    }
}
