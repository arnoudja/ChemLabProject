//! Burner heat, dish evaporation, ambient cooling, and heat-capacity helpers.
//!
//! Extracted from [`super`] for a smaller review surface. Public entry points are
//! re-exported from `scene` / the crate root so the API stays stable.

use super::*;

/// Apply burner heat / dish evaporation and ambient Newton cooling for `dt_s` seconds.
///
/// The HTTP layer clamps the clock delta (e.g. 0–2 s) before calling this.
/// Dish water can leave with the burner on or off (ambient mass transfer); beakers
/// do not evaporate.
pub fn apply_elapsed(scene: &mut Scene, dt_s: f64) {
    let dt = if dt_s.is_finite() { dt_s.max(0.0) } else { 0.0 };
    if dt <= 0.0 {
        return;
    }

    let burner_idx = scene.items.iter().position(|item| item.kind == "burner");
    let dish_idx = scene
        .items
        .iter()
        .position(|item| item.kind == "evaporation_dish");

    let mut heating_dish = false;
    if let (Some(burner_idx), Some(dish_idx)) = (burner_idx, dish_idx) {
        if scene.items[burner_idx].properties.on == Some(true) {
            // Heat only while liquid water remains. Leftover liquid H₂SO₄ after
            // dry-out must not keep the burner on (T would stall with no water).
            if scene.items[dish_idx].location != "bench"
                || !crate::solubility::dish_has_liquid_water(&scene.items[dish_idx])
            {
                scene.items[burner_idx].properties.on = Some(false);
            } else {
                heating_dish = true;
            }
        }
    }

    let mut finalized_dish = false;
    if let Some(dish_idx) = dish_idx {
        if scene.items[dish_idx].location == "bench"
            && crate::solubility::dish_has_liquid(&scene.items[dish_idx])
        {
            apply_dish_evaporation(&mut scene.items[dish_idx], dt, heating_dish);
            finalize_aqueous_vessel(&mut scene.items[dish_idx], dt);
            finalized_dish = true;
            if let Some(burner_idx) = burner_idx {
                if scene.items[burner_idx].properties.on == Some(true)
                    && !crate::solubility::dish_has_liquid_water(&scene.items[dish_idx])
                {
                    scene.items[burner_idx].properties.on = Some(false);
                }
            }
        }
    }

    let dish_id = dish_idx.map(|idx| scene.items[idx].id.clone());
    for idx in 0..scene.items.len() {
        if heating_dish && dish_id.as_deref() == Some(scene.items[idx].id.as_str()) {
            continue;
        }
        apply_ambient_cool(&mut scene.items[idx], dt);
    }

    // Kinetic dissolve on wet vessels holding soluble solids (beakers, etc.).
    for idx in 0..scene.items.len() {
        if finalized_dish && dish_id.as_deref() == Some(scene.items[idx].id.as_str()) {
            continue;
        }
        if !matches!(
            scene.items[idx].kind.as_str(),
            "beaker" | "evaporation_dish"
        ) {
            continue;
        }
        if crate::dissolve_kinetics::vessel_needs_kinetic_dissolve(&scene.items[idx]) {
            finalize_aqueous_vessel(&mut scene.items[idx], dt);
        }
    }
}

/// Saturation vapor pressure of pure water (bar) via Antoine, `T` in °C.
pub fn water_vapor_pressure_bar(temperature_c: f64) -> f64 {
    if !temperature_c.is_finite() {
        return 0.0;
    }
    10f64.powf(WATER_ANTOINE_A - WATER_ANTOINE_B / (temperature_c + WATER_ANTOINE_C))
}

/// Liquid-water mole fraction among water + aqueous ions (solids / sand excluded).
pub fn water_mole_fraction(item: &SceneItem) -> f64 {
    let n_water = crate::solubility::liquid_water_ml(item) / WATER_MOLAR_MASS_G_PER_MOL;
    let n_ions: f64 = item
        .properties
        .composition
        .iter()
        .filter(|c| c.phase == "aqueous")
        .map(|c| c.amount_mol.unwrap_or(0.0).max(0.0))
        .sum();
    let n_tot = n_water + n_ions;
    if n_tot <= AMOUNT_EPS {
        return 0.0;
    }
    (n_water / n_tot).clamp(0.0, 1.0)
}

/// Boiling temperature (°C) where `x_w · P_sat(T) = P_atm` (Raoult + Antoine).
pub fn boiling_temperature_c(x_w: f64) -> f64 {
    let x = x_w.clamp(AMOUNT_EPS, 1.0);
    // Pure water at 1.00 bar is ~99.61 °C; salts raise T. Bracket generously.
    let mut lo = 50.0;
    let mut hi = 200.0;
    for _ in 0..48 {
        let mid = 0.5 * (lo + hi);
        let p_w = x * water_vapor_pressure_bar(mid);
        if p_w < ATM_PRESSURE_BAR {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

fn air_water_partial_pressure_bar() -> f64 {
    RELATIVE_HUMIDITY * water_vapor_pressure_bar(AMBIENT_TEMPERATURE_C)
}

fn remove_liquid_water_ml(dish: &mut SceneItem, loss_ml: f64) {
    if loss_ml <= AMOUNT_EPS {
        return;
    }
    let remain = (crate::solubility::liquid_water_ml(dish) - loss_ml).max(0.0);
    if let Some(water) = dish
        .properties
        .composition
        .iter_mut()
        .find(|c| c.substance_id == "water" && c.phase == "liquid")
    {
        water.amount_ml = Some(remain);
    }
}

/// Remove evaporated mass from the dish. With aqueous HCl, use azeotrope-style
/// HCl+water split; otherwise water-only (1 g ≈ 1 ml).
fn remove_evaporated_mass(dish: &mut SceneItem, loss_mass_g: f64) {
    if loss_mass_g <= AMOUNT_EPS {
        return;
    }
    let inv = crate::hcl::HclInventory::from_item(dish);
    if inv.n_h > AMOUNT_EPS {
        crate::hcl::remove_hcl_water_evap_mass(dish, loss_mass_g);
    } else {
        remove_liquid_water_ml(dish, loss_mass_g);
    }
}

/// Vessel boil temperature (beaker or dish).
///
/// With HCl inventory: tabulated `T_hcl(w)`. When non-HCl solutes are also
/// present (salt metals / salt Cl⁻ / sulfate concentrates), take
/// `max(T_hcl(w), T_raoult(x_w))` so elevation is not ignored (no more acid-wins).
/// Concentrated HCl alone stays on the acid table — Raoult on the acid-depleted
/// `x_w` would falsely soar. Without HCl inventory: Raoult + Antoine from water
/// mole fraction (sulfuric-only / water path). Shared by burner dish boil and
/// instant chemical-heat spit gating.
pub fn vessel_boil_temperature_c(item: &SceneItem) -> f64 {
    let inv = crate::hcl::HclInventory::from_item(item);
    let t_water = boiling_temperature_c(water_mole_fraction(item));
    if inv.n_h > AMOUNT_EPS {
        let t_hcl = crate::hcl::hcl_boil_temperature_c(inv.w_hcl());
        if dish_has_non_hcl_solute(item) {
            return t_hcl.max(t_water);
        }
        return t_hcl;
    }
    t_water
}

fn dish_boil_temperature_c(item: &SceneItem) -> f64 {
    vessel_boil_temperature_c(item)
}

/// True when aqueous non-HCl solutes depress `x_w` beyond gated HCl alone:
/// salt Na⁺/Ca²⁺, salt Cl⁻, or sulfate / bisulfate concentrates.
fn dish_has_non_hcl_solute(item: &SceneItem) -> bool {
    let n_na = crate::composition::aqueous_mol(item, "na+");
    let n_ca = crate::composition::aqueous_mol(item, "ca2+");
    let n_cl = crate::composition::aqueous_mol(item, "cl-");
    let n_h = crate::composition::aqueous_mol(item, "h+");
    let n_oh = crate::composition::aqueous_mol(item, "oh-");
    let n_so4 = crate::composition::aqueous_mol(item, "so4^2-")
        + crate::composition::aqueous_mol(item, "hso4-");
    let n_naoh = if n_oh > n_h + 1e-9 {
        (n_oh - n_h).max(0.0)
    } else {
        0.0
    };
    let n_na_salt = (n_na - n_naoh).max(0.0);
    // Cl⁻ not charge-balanced by free H⁺ is salt chloride (ignore Kw-scale noise).
    let n_cl_salt = (n_cl - n_h).max(0.0);
    const SALT_EPS: f64 = 1e-9;
    n_na_salt > SALT_EPS || n_ca > SALT_EPS || n_cl_salt > SALT_EPS || n_so4 > SALT_EPS
}

/// Latent heat (J/g) for the vapor leaving `dish` this step.
fn dish_vapor_latent_heat_j_per_g(dish: &SceneItem) -> f64 {
    let inv = crate::hcl::HclInventory::from_item(dish);
    if inv.n_h <= AMOUNT_EPS {
        return WATER_LATENT_HEAT_J_PER_G;
    }
    let y = crate::hcl::azeotrope_vapor_w_hcl(inv.w_hcl());
    crate::hcl::hcl_latent_heat_j_per_g(y)
}

fn apply_latent_cool_with_h_vap(dish: &mut SceneItem, mass_g: f64, c_eff_before: f64, h_vap: f64) {
    if mass_g <= AMOUNT_EPS || c_eff_before <= AMOUNT_EPS {
        return;
    }
    let t = dish
        .properties
        .temperature_c
        .unwrap_or(AMBIENT_TEMPERATURE_C);
    dish.properties.temperature_c = Some(t - mass_g * h_vap / c_eff_before);
}

/// Sub-boil mass transfer: `m_dot = k A max(0, p_w − p_air) / P_atm` (ml/s ≈ g/s), then latent cool.
fn apply_sub_boil_mass_transfer(dish: &mut SceneItem, dt: f64) {
    if dt <= AMOUNT_EPS || !crate::solubility::dish_has_liquid(dish) {
        return;
    }
    let t = dish
        .properties
        .temperature_c
        .unwrap_or(AMBIENT_TEMPERATURE_C);
    let x_w = water_mole_fraction(dish);
    if x_w <= AMOUNT_EPS {
        return;
    }
    let p_w = x_w * water_vapor_pressure_bar(t);
    let driving = (p_w - air_water_partial_pressure_bar()).max(0.0) / ATM_PRESSURE_BAR;
    let m_dot = DISH_MASS_TRANSFER_COEFF_ML_PER_S_M2 * DISH_EVAP_AREA_M2 * driving;
    let loss = m_dot * dt;
    if loss <= AMOUNT_EPS {
        return;
    }
    let c_eff = effective_heat_capacity(dish).max(AMOUNT_EPS);
    let h_vap = dish_vapor_latent_heat_j_per_g(dish);
    remove_evaporated_mass(dish, loss);
    apply_latent_cool_with_h_vap(dish, loss, c_eff, h_vap);
}

fn apply_heat_limited_boil(dish: &mut SceneItem, dt: f64) {
    let t_boil = dish_boil_temperature_c(dish);
    // Plateau at T_boil while heat-limited boiling; do not also apply latent ΔT
    // (Q_net already pays for vaporization).
    dish.properties.temperature_c = Some(t_boil);
    // Caller only invokes this while the burner is on. Newton cool is skipped for
    // the dish while heating (see apply_elapsed), so Q_net = burner power only.
    // Subtracting UA here would double-count loss that is not applied while
    // heating (and historically with BURNER_POWER_W=80 / UA_DISH=4 would make
    // Q_net negative near 100 °C). Burner-off paths never call this (Q_net ≤ 0).
    let h_vap = dish_vapor_latent_heat_j_per_g(dish).max(AMOUNT_EPS);
    let m_dot = (BURNER_POWER_W / h_vap).max(0.0);
    remove_evaporated_mass(dish, m_dot * dt);
}

fn dish_is_boiling(temperature_c: f64, item: &SceneItem) -> bool {
    let inv = crate::hcl::HclInventory::from_item(item);
    if inv.n_h > AMOUNT_EPS {
        // Concentration-dependent acid boil: T vs tabulated T_boil(w_HCl).
        return temperature_c + 1e-3 >= dish_boil_temperature_c(item);
    }
    // Vapor-pressure gate only. Do not use T >= T_boil alone: as the dish
    // concentrates, T_boil can run away and a T comparison falsely trips.
    const BOIL_P_EPS_BAR: f64 = 1e-4;
    let x_w = water_mole_fraction(item);
    x_w * water_vapor_pressure_bar(temperature_c) + BOIL_P_EPS_BAR >= ATM_PRESSURE_BAR
}

fn apply_dish_evaporation(dish: &mut SceneItem, dt: f64, heating: bool) {
    let mut remaining = dt;
    if remaining <= AMOUNT_EPS || !crate::solubility::dish_has_liquid(dish) {
        return;
    }

    let mut t = dish
        .properties
        .temperature_c
        .unwrap_or(AMBIENT_TEMPERATURE_C);
    let x_w = water_mole_fraction(dish);
    if x_w <= AMOUNT_EPS && crate::hcl::HclInventory::from_item(dish).n_h <= AMOUNT_EPS {
        return;
    }
    let t_boil = dish_boil_temperature_c(dish);

    if heating {
        // While the burner heats: sensible heat until boil, then heat-limited
        // evaporation. Skip sub-boil MT here — with Antoine-scaled driving force
        // it would dry / concentrate the dish before p_w reaches P_atm and block
        // the boil plateau. Ambient MT runs when the burner is off (below).
        if dish_is_boiling(t, dish) {
            apply_heat_limited_boil(dish, remaining);
            return;
        }
        let c_eff = effective_heat_capacity(dish).max(AMOUNT_EPS);
        let time_to_boil = ((t_boil - t).max(0.0) * c_eff / BURNER_POWER_W).max(0.0);
        if time_to_boil >= remaining {
            t = (t + (BURNER_POWER_W / c_eff) * remaining).min(t_boil);
            dish.properties.temperature_c = Some(t);
            return;
        }
        dish.properties.temperature_c = Some(t_boil);
        remaining -= time_to_boil;
        if remaining <= AMOUNT_EPS {
            return;
        }
        apply_heat_limited_boil(dish, remaining);
        return;
    }

    // Burner off: slow ambient mass transfer (and latent cool). No heat-limited
    // boil when Q_net ≤ 0.
    if !dish_is_boiling(t, dish) {
        apply_sub_boil_mass_transfer(dish, remaining);
    }
}

fn solid_specific_heat(substance_id: &str) -> Option<f64> {
    match substance_id {
        "nacl" => Some(CP_NACL),
        "cacl2" => Some(CP_CACL2),
        "sand" => Some(CP_SAND),
        "naoh" => Some(CP_NAOH),
        _ => None,
    }
}

fn vessel_heat_capacity(item: &SceneItem) -> f64 {
    match item.kind.as_str() {
        "evaporation_dish" => C_DISH,
        "beaker" => C_BEAKER,
        _ => 0.0,
    }
}

fn vessel_ua(item: &SceneItem) -> Option<f64> {
    match item.kind.as_str() {
        "evaporation_dish" => Some(UA_DISH),
        "beaker" => Some(UA_BEAKER),
        "pipette" => Some(UA_BEAKER),
        _ => None,
    }
}

pub(crate) fn heat_capacity_of_entries(entries: &[CompositionEntry]) -> f64 {
    let mut c = 0.0;
    for entry in entries {
        if entry.substance_id == "water" && entry.phase == "liquid" {
            c += entry.amount_ml.unwrap_or(0.0) * WATER_SPECIFIC_HEAT_J_PER_G_K;
        } else if entry.substance_id == "h2so4" && entry.phase == "liquid" {
            let mass_g = entry.amount_g.unwrap_or_else(|| {
                entry
                    .amount_mol
                    .map(|n| n * crate::h2so4::H2SO4_MOLAR_MASS_G_PER_MOL)
                    .unwrap_or_else(|| {
                        entry.amount_ml.unwrap_or(0.0)
                            * crate::h2so4::H2SO4_STOCK_DENSITY_G_PER_ML
                            * crate::h2so4::H2SO4_STOCK_W_W
                    })
            });
            c += mass_g.max(0.0) * CP_H2SO4_LIQUID;
        } else if entry.phase == "solid" {
            if let Some(cp) = solid_specific_heat(&entry.substance_id) {
                c += solid_amount_g(entry) * cp;
            }
        }
        // Aqueous ions: thermal mass counted with the water solvent (plan).
    }
    c
}

/// Effective heat capacity of an item: vessel body + Σ contents m·c_p.
///
/// Tools / burner / filter paper contribute no vessel term. Pipettes use liquid
/// holding only (no glass).
pub fn effective_heat_capacity(item: &SceneItem) -> f64 {
    if item.kind == "pipette" {
        return heat_capacity_of_entries(&item.properties.holding);
    }
    if matches!(
        item.kind.as_str(),
        "spoon" | "tongs" | "burner" | "filter_paper"
    ) {
        return 0.0;
    }
    vessel_heat_capacity(item) + heat_capacity_of_entries(&item.properties.composition)
}

fn is_thermal_item(item: &SceneItem) -> bool {
    match item.kind.as_str() {
        "beaker" | "evaporation_dish" => item.properties.temperature_c.is_some(),
        "pipette" => {
            item.properties.temperature_c.is_some() && pipette_holding_liquid_ml(item) > AMOUNT_EPS
        }
        _ => false,
    }
}

fn apply_ambient_cool(item: &mut SceneItem, dt: f64) {
    if !is_thermal_item(item) {
        return;
    }
    let Some(ua) = vessel_ua(item) else {
        return;
    };
    let Some(temperature) = item.properties.temperature_c else {
        return;
    };
    if (temperature - AMBIENT_TEMPERATURE_C).abs() < TEMPERATURE_SNAP_EPS_C {
        item.properties.temperature_c = Some(AMBIENT_TEMPERATURE_C);
        return;
    }
    let c_eff = effective_heat_capacity(item);
    if c_eff <= AMOUNT_EPS {
        item.properties.temperature_c = Some(AMBIENT_TEMPERATURE_C);
        return;
    }
    let delta = -(ua / c_eff) * (temperature - AMBIENT_TEMPERATURE_C) * dt;
    let mut next = temperature + delta;
    // Do not cross ambient in one tick.
    if (temperature - AMBIENT_TEMPERATURE_C).signum() != (next - AMBIENT_TEMPERATURE_C).signum()
        && (next - AMBIENT_TEMPERATURE_C).abs() > AMOUNT_EPS
    {
        next = AMBIENT_TEMPERATURE_C;
    }
    if (next - AMBIENT_TEMPERATURE_C).abs() < TEMPERATURE_SNAP_EPS_C {
        next = AMBIENT_TEMPERATURE_C;
    }
    item.properties.temperature_c = Some(next);
}

pub(super) fn blend_temperature_capacity(target: &mut SceneItem, c_add: f64, source_t: f64) {
    if c_add <= AMOUNT_EPS {
        return;
    }
    let c_dest = effective_heat_capacity(target);
    let t0 = target
        .properties
        .temperature_c
        .unwrap_or(AMBIENT_TEMPERATURE_C);
    if (source_t - t0).abs() <= AMOUNT_EPS {
        return;
    }
    if c_dest <= AMOUNT_EPS {
        target.properties.temperature_c = Some(source_t);
    } else {
        target.properties.temperature_c = Some((c_dest * t0 + c_add * source_t) / (c_dest + c_add));
    }
}
