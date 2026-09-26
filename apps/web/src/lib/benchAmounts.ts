/** Mass of one spoon scoop of solid (grams) — mirrors `chemlab_core::SPOON_SCOOP_MASS_G`. */
export const SPOON_SCOOP_MASS_G = 0.2

/** Initial scoop count in salt/sand stock beakers. */
export const STOCK_FULL_SCOOPS = 10

/** Initial salt/sand stock mass on the server bench. */
export const STOCK_FULL_MASS_G = STOCK_FULL_SCOOPS * SPOON_SCOOP_MASS_G

/** Main bench beaker fill-height scale (ml). Capacity is 250 ml; 200 ml reads as full. */
export const WATER_FULL_ML = 200

/** Distilled-water stock beaker capacity (ml). */
export const DISTILLED_WATER_CAPACITY_ML = 100

/** HCl stock beaker capacity (ml of solution) — mirrors `chemlab_core::hcl::HCL_STOCK_CAPACITY_ML`. */
export const HCL_STOCK_CAPACITY_ML = 10

/** Water mass (g / ml) in the filled Free-mode HCl stock. */
export const HCL_STOCK_WATER_ML = 8.043

/** Solution density at 30% w/w HCl (g/ml). */
export const HCL_STOCK_DENSITY_G_PER_ML = 1.149

/** Moles of HCl (= H⁺ = Cl⁻) in the filled Free-mode stock. */
export const HCL_STOCK_HCL_MOLES = 3.447 / 36.46

/** Pipette aliquot volume (ml). */
export const PIPETTE_VOLUME_ML = 1

/** Liquid a vessel must hold before the pipette may draw from it (ml). */
export const PIPETTE_MIN_SOURCE_ML = 3

/** Evaporation dish capacity (ml). */
export const DISH_CAPACITY_ML = 25

/** Filtrate beaker liquid capacity (ml). */
export const FILTRATE_CAPACITY_ML = 250
