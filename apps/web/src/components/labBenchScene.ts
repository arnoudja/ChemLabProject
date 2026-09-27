import type { CompositionEntry, Item, LabScene } from '../generated/contracts'
import { optionalArray } from '../lib/scene'
import type { StockSolid } from './LabBenchIcons'

/** Apparent molar volumes (ml/mol) — mirrors chemlab-core `hcl::PHI_V_*`. */
export const HCL_STOCK_CAPACITY_ML = 10.0
export const HCL_STOCK_WATER_MASS_G = 8.043
export const HCL_STOCK_HCL_MOLES = 3.447 / 36.46
export const PHI_V_HCL_ML_PER_MOL =
  (HCL_STOCK_CAPACITY_ML - HCL_STOCK_WATER_MASS_G) / HCL_STOCK_HCL_MOLES
export const PHI_V_H2SO4_ML_PER_MOL = 40.0
export const PHI_V_NAHSO4_ML_PER_MOL = 30.0
export const PHI_V_NA2SO4_ML_PER_MOL = 20.0
export const PHI_V_CASO4_ML_PER_MOL = 15.0
export const PHI_V_NACL_ML_PER_MOL = 22.0
export const PHI_V_CACL2_ML_PER_MOL = 34.0
export const PHI_V_NAOH_ML_PER_MOL = 4.0
export const H2SO4_STOCK_CAPACITY_ML = 10.0

/**
 * Item-id / volume glossary (wire ids are stable — do not rename):
 *
 * - `WATER_ID` (`beaker-water`) — main reaction beaker. UI height via `waterAmountMl`
 *   = Φ_V solution volume (matches server `fill_ml`).
 * - `H2O_ID` (`beaker-h2o`) — distilled-water stock. `distilledWaterAmountMl` is
 *   liquid water ml only (solvent basis), not Φ_V.
 * - Substance id `"water"` — liquid H₂O rows in composition (`phase: "liquid"`).
 * - `fill_ml` — display fill; server writes Φ_V via `sync_fill_ml`.
 * - Liquid water `amount_ml` — SI / wash / solvent basis (not transfer capacity).
 */
export const SPOON_ID = 'spoon-1'
export const PIPETTE_ID = 'pipette-1'
export const TONGS_ID = 'tongs-1'
export const DISH_ID = 'dish-1'
export const BURNER_ID = 'burner-1'
export const FILTRATE_ID = 'beaker-filtrate'
export const PAPER_ID = 'filter-paper-1'
export const NACL_ID = 'beaker-nacl'
export const CACL2_ID = 'beaker-cacl2'
export const SAND_ID = 'beaker-sand'
export const NAOH_ID = 'beaker-naoh'
export const NA2SO4_ID = 'beaker-na2so4'
export const H2O_ID = 'beaker-h2o'
export const HCL_ID = 'beaker-hcl'
export const H2SO4_ID = 'beaker-h2so4'
export const WATER_ID = 'beaker-water'

export function isStockSolid(id: string): id is StockSolid {
  return id === 'nacl' || id === 'cacl2' || id === 'sand' || id === 'naoh' || id === 'na2so4'
}

export function findItem(scene: LabScene, id: string): Item | undefined {
  return scene.items.find((item) => item.id === id)
}

export function spoonHoldingSubstance(scene: LabScene): StockSolid | null {
  const spoon = findItem(scene, SPOON_ID)
  const held = optionalArray(spoon?.properties.holding)[0]
  if (!held || held.phase !== 'solid') return null
  return isStockSolid(held.substance_id) ? held.substance_id : null
}

export function spoonHoldingSpecies(scene: LabScene): string[] {
  const ids: string[] = []
  for (const held of optionalArray(findItem(scene, SPOON_ID)?.properties.holding)) {
    if (held.phase !== 'solid') continue
    if (!ids.includes(held.substance_id)) ids.push(held.substance_id)
  }
  return ids
}

/** Undissolved solid grains come only from server composition on the water item. */
export function undissolvedSolidInWater(scene: LabScene): StockSolid | null {
  const water = findItem(scene, WATER_ID)
  const solid = optionalArray(water?.properties.composition).find((entry) => entry.phase === 'solid')
  if (!solid) return null
  return isStockSolid(solid.substance_id) ? solid.substance_id : null
}

export function itemTemperatureC(scene: LabScene, item: Item): number {
  return item.properties.temperature_c ?? scene.temperature_c
}

export function stockAmountG(scene: LabScene, itemId: string, substanceId: StockSolid): number | null {
  const item = findItem(scene, itemId)
  const entry = optionalArray(item?.properties.composition).find(
    (c) => c.substance_id === substanceId && c.phase === 'solid',
  )
  return entry?.amount_g ?? null
}

export function distilledWaterAmountMl(scene: LabScene): number | null {
  const item = findItem(scene, H2O_ID)
  const entry = optionalArray(item?.properties.composition).find(
    (c) => c.substance_id === 'water' && c.phase === 'liquid',
  )
  return entry?.amount_ml ?? null
}

function aqueousMol(composition: CompositionEntry[], substanceId: string): number {
  const entry = composition.find((c) => c.substance_id === substanceId && c.phase === 'aqueous')
  return Math.max(0, entry?.amount_mol ?? 0)
}

/** Solution volume (ml) = water ml + liquid H₂SO₄ ml + Σ n · Φ_V. */
export function solutionVolumeMl(composition: CompositionEntry[]): number {
  const waterEntry = composition.find(
    (c) => c.substance_id === 'water' && c.phase === 'liquid',
  )
  const waterMl = Math.max(0, waterEntry?.amount_ml ?? 0)
  const h2so4Liquid = composition.find(
    (c) => c.substance_id === 'h2so4' && c.phase === 'liquid',
  )
  const h2so4Ml = Math.max(0, h2so4Liquid?.amount_ml ?? 0)
  const nH = aqueousMol(composition, 'h+')
  const nOh = aqueousMol(composition, 'oh-')
  const nNa = aqueousMol(composition, 'na+')
  const nCa = aqueousMol(composition, 'ca2+')
  const nCl = aqueousMol(composition, 'cl-')
  const nSo4 = aqueousMol(composition, 'so4^2-')
  const nHso4 = aqueousMol(composition, 'hso4-')
  const nHExcess = Math.max(0, nH - nOh)
  const nHcl = Math.min(nHExcess, nCl)
  const nHAfterHcl = Math.max(0, nHExcess - nHcl)
  const nClAfterHcl = Math.max(0, nCl - nHcl)
  // Excess OH above free H marks strong-base NaOH (Kw leaves both present).
  const nNaoh = nOh > nH + 1e-9 ? Math.max(0, nOh - nH) : 0
  const nNaSalt = Math.max(0, nNa - nNaoh)
  // Reserve Na for chloride salts before NaHSO₄ / Na₂SO₄ pairing.
  const nNaForCl = Math.min(nNaSalt, nClAfterHcl)
  const nNaSulfateBudget = Math.max(0, nNaSalt - nNaForCl)
  const nNahso4 = Math.min(nNaSulfateBudget, nHso4)
  const nHso4Acid = Math.max(0, nHso4 - nNahso4)
  const nNaAfterNahso4 = Math.max(0, nNaSulfateBudget - nNahso4)
  // Salt SO₄ before free-acid SO₄ (Ka2 remnant → ½ Na₂SO₄ + ½ H₂SO₄).
  const nNa2so4 = Math.min(nNaAfterNahso4 * 0.5, nSo4)
  const nSo4AfterNa2so4 = Math.max(0, nSo4 - nNa2so4)
  const nSo4Acid = Math.min(Math.max(0, nHAfterHcl - nHso4Acid), nSo4AfterNa2so4)
  const nH2so4 = nHso4Acid + nSo4Acid
  const nSo4AfterAcid = Math.max(0, nSo4AfterNa2so4 - nSo4Acid)
  const nCaso4 = Math.min(nCa, nSo4AfterAcid)
  const nCaAfterSulfate = Math.max(0, nCa - nCaso4)
  const nNacl = nNaForCl
  const nClAfterNacl = Math.max(0, nClAfterHcl - nNacl)
  const nCacl2 = Math.min(nCaAfterSulfate, nClAfterNacl * 0.5)
  return (
    waterMl +
    h2so4Ml +
    nHcl * PHI_V_HCL_ML_PER_MOL +
    nH2so4 * PHI_V_H2SO4_ML_PER_MOL +
    nNahso4 * PHI_V_NAHSO4_ML_PER_MOL +
    nNaoh * PHI_V_NAOH_ML_PER_MOL +
    nNa2so4 * PHI_V_NA2SO4_ML_PER_MOL +
    nCaso4 * PHI_V_CASO4_ML_PER_MOL +
    nNacl * PHI_V_NACL_ML_PER_MOL +
    nCacl2 * PHI_V_CACL2_ML_PER_MOL
  )
}

export function hclStockAmountMl(scene: LabScene): number | null {
  const item = findItem(scene, HCL_ID)
  if (!item) return null
  const composition = optionalArray(item.properties.composition)
  if (composition.length === 0) return null
  return solutionVolumeMl(composition)
}

export function h2so4StockAmountMl(scene: LabScene): number | null {
  const item = findItem(scene, H2SO4_ID)
  if (!item) return null
  const composition = optionalArray(item.properties.composition)
  if (composition.length === 0) return null
  return solutionVolumeMl(composition)
}

/** Main water beaker liquid height — Φ_V solution volume (matches server `fill_ml`).
 * Distilled stock (`distilledWaterAmountMl`) stays water-ml only. */
export function waterAmountMl(scene: LabScene): number | null {
  const item = findItem(scene, WATER_ID)
  if (!item) return null
  const composition = optionalArray(item.properties.composition)
  if (composition.length === 0) {
    return item.properties.fill_ml ?? null
  }
  return solutionVolumeMl(composition)
}

/** True when the water beaker composition includes server-authored aqueous ions. */
export function waterHasAqueous(scene: LabScene): boolean {
  const item = findItem(scene, WATER_ID)
  return optionalArray(item?.properties.composition).some((c) => c.phase === 'aqueous')
}

/** Latest dissolve-related cue from server `last_events` (no client chemistry). */
export function dissolveCueFromEvents(
  events: { kind: string; message: string }[],
): 'dissolved' | 'did_not_dissolve' | null {
  for (let i = events.length - 1; i >= 0; i -= 1) {
    const kind = events[i]?.kind
    if (kind === 'dissolved' || kind === 'did_not_dissolve') return kind
  }
  return null
}

/** Dish liquid height — Φ_V solution volume (matches server `fill_ml`), not water-only ml. */
export function dishAmountMl(scene: LabScene): number | null {
  const item = findItem(scene, DISH_ID)
  if (!item) return null
  const composition = optionalArray(item.properties.composition)
  if (composition.length === 0) {
    return item.properties.fill_ml ?? null
  }
  return solutionVolumeMl(composition)
}

/** Filtrate liquid height — Φ_V solution volume (matches server `fill_ml`), not water-only ml. */
export function filtrateAmountMl(scene: LabScene): number | null {
  const item = findItem(scene, FILTRATE_ID)
  if (!item) return null
  const composition = optionalArray(item.properties.composition)
  if (composition.length === 0) {
    return item.properties.fill_ml ?? null
  }
  return solutionVolumeMl(composition)
}

export function paperHasResidue(scene: LabScene): boolean {
  return optionalArray(findItem(scene, PAPER_ID)?.properties.composition).some(
    (entry) => entry.phase === 'solid' && (entry.amount_g ?? 0) > 0,
  )
}

export function pipetteIsFilled(scene: LabScene): boolean {
  const pipette = findItem(scene, PIPETTE_ID)
  return optionalArray(pipette?.properties.holding).some(
    (entry) => entry.phase === 'liquid' && (entry.amount_ml ?? 0) > 0,
  )
}

export function burnerIsOn(scene: LabScene): boolean {
  return findItem(scene, BURNER_ID)?.properties.on === true
}

/** Ambient bench temperature mirrored from `chemlab-core::AMBIENT_TEMPERATURE_C`. */
export const AMBIENT_TEMPERATURE_C = 20

/** Poll while any vessel is meaningfully off ambient (cool-down / residual heat). */
export const THERMAL_POLL_DELTA_C = 0.5

/** Soluble solids that advance via server kinetic dissolve (mirrors `dissolve_kinetics`). */
const KINETIC_SOLUBLE_SOLIDS = new Set(['nacl', 'cacl2', 'naoh', 'na2so4', 'caso4'])

/** True when the bench should poll for ongoing heat / ambient cool-down. */
export function sceneNeedsThermalPoll(scene: LabScene): boolean {
  if (burnerIsOn(scene)) return true
  for (const item of scene.items) {
    const t = item.properties.temperature_c
    if (t == null) continue
    if (Math.abs(t - AMBIENT_TEMPERATURE_C) >= THERMAL_POLL_DELTA_C) return true
  }
  return false
}

/**
 * True when a wet beaker/dish still holds kinetically soluble solid.
 * NaCl dissolve ΔT is often ≪ {@link THERMAL_POLL_DELTA_C}, so thermal poll alone
 * leaves inspect amounts stale until the next user action.
 */
export function sceneNeedsDissolvePoll(scene: LabScene): boolean {
  for (const item of scene.items) {
    if (item.kind !== 'beaker' && item.kind !== 'evaporation_dish') continue
    const composition = optionalArray(item.properties.composition)
    const hasWater = composition.some(
      (entry) =>
        entry.substance_id === 'water' &&
        entry.phase === 'liquid' &&
        (entry.amount_ml ?? 0) > 0,
    )
    if (!hasWater) continue
    const hasSolubleSolid = composition.some(
      (entry) =>
        entry.phase === 'solid' &&
        KINETIC_SOLUBLE_SOLIDS.has(entry.substance_id) &&
        (entry.amount_g ?? 0) > 0,
    )
    if (hasSolubleSolid) return true
  }
  return false
}

/** True when GET /api/lab/scene must keep ticking the server clock. */
export function sceneNeedsClockPoll(scene: LabScene): boolean {
  return sceneNeedsThermalPoll(scene) || sceneNeedsDissolvePoll(scene)
}

export function tongsHeldVesselId(
  scene: LabScene,
):
  | typeof WATER_ID
  | typeof DISH_ID
  | typeof H2O_ID
  | typeof HCL_ID
  | typeof H2SO4_ID
  | typeof FILTRATE_ID
  | typeof PAPER_ID
  | typeof NACL_ID
  | typeof CACL2_ID
  | typeof SAND_ID
  | typeof NAOH_ID
  | null {
  const held = findItem(scene, TONGS_ID)?.properties.source_item_id
  if (
    held === WATER_ID ||
    held === DISH_ID ||
    held === H2O_ID ||
    held === HCL_ID ||
    held === H2SO4_ID ||
    held === FILTRATE_ID ||
    held === PAPER_ID ||
    held === NACL_ID ||
    held === CACL2_ID ||
    held === SAND_ID ||
    held === NAOH_ID
  ) {
    return held
  }
  return null
}

export function solidStockKind(itemId: string): StockSolid | null {
  if (itemId === NACL_ID) return 'nacl'
  if (itemId === CACL2_ID) return 'cacl2'
  if (itemId === SAND_ID) return 'sand'
  if (itemId === NAOH_ID) return 'naoh'
  if (itemId === NA2SO4_ID) return 'na2so4'
  return null
}

export function outcomeLabel(kind: string): string | null {
  if (kind === 'dissolved') return 'Dissolved'
  if (kind === 'did_not_dissolve') return 'Did not dissolve'
  if (kind === 'returned') return 'Returned'
  if (kind === 'reset') return 'Reset'
  if (kind === 'scooped') return 'Scooped'
  if (kind === 'poured') return 'Poured'
  if (kind === 'pipetted') return 'Pipetted'
  if (kind === 'toggled') return 'Toggled'
  return null
}
