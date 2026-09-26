import type { CompositionEntry, Item, LabScene } from '../generated/contracts'
import { optionalArray } from '../lib/scene'
import type { StockSolid } from './LabBenchIcons'

const AMOUNT_EPS = 1e-12

/** Apparent molar volumes (ml/mol) — mirrors chemlab-core `hcl::PHI_V_*`. */
const HCL_STOCK_CAPACITY_ML = 10.0
const HCL_STOCK_WATER_MASS_G = 8.043
const HCL_STOCK_HCL_MOLES = 3.447 / 36.46
const PHI_V_HCL_ML_PER_MOL =
  (HCL_STOCK_CAPACITY_ML - HCL_STOCK_WATER_MASS_G) / HCL_STOCK_HCL_MOLES
const PHI_V_NACL_ML_PER_MOL = 22.0
const PHI_V_CACL2_ML_PER_MOL = 34.0
const PHI_V_NAOH_ML_PER_MOL = 4.0

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
export const H2O_ID = 'beaker-h2o'
export const HCL_ID = 'beaker-hcl'
export const WATER_ID = 'beaker-water'

export function isStockSolid(id: string): id is StockSolid {
  return id === 'nacl' || id === 'cacl2' || id === 'sand' || id === 'naoh'
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

/** Density of aqueous HCl (g/ml) from HCl mass fraction — piecewise linear through lab stock table.
 * Kept for docs / continuity checks; solution volume uses Φ_V (see `solutionVolumeMl`). */
export function hclAqDensityGPerMl(wHcl: number): number {
  const w = Math.min(0.4, Math.max(0, wHcl))
  const table: [number, number][] = [
    [0, 1.0],
    [0.202, 1.098],
    [0.3, 1.149],
    [0.37, 1.184],
  ]
  for (let i = 0; i < table.length - 1; i += 1) {
    const [w0, d0] = table[i]!
    const [w1, d1] = table[i + 1]!
    if (w <= w1) {
      const frac = Math.abs(w1 - w0) <= AMOUNT_EPS ? 0 : (w - w0) / (w1 - w0)
      return d0 + frac * (d1 - d0)
    }
  }
  return table[table.length - 1]![1]
}

function aqueousMol(composition: CompositionEntry[], substanceId: string): number {
  const entry = composition.find((c) => c.substance_id === substanceId && c.phase === 'aqueous')
  return Math.max(0, entry?.amount_mol ?? 0)
}

/** Solution volume (ml) = water ml + Σ n · Φ_V (HCl → NaOH → NaCl → CaCl₂ pairing). */
export function solutionVolumeMl(composition: CompositionEntry[]): number {
  const waterEntry = composition.find(
    (c) => c.substance_id === 'water' && c.phase === 'liquid',
  )
  const waterMl = Math.max(0, waterEntry?.amount_ml ?? 0)
  const nH = aqueousMol(composition, 'h+')
  const nOh = aqueousMol(composition, 'oh-')
  const nNa = aqueousMol(composition, 'na+')
  const nCa = aqueousMol(composition, 'ca2+')
  const nHcl = nH
  const nNaoh = nOh
  const nNacl = Math.max(0, nNa - nOh)
  const nCacl2 = nCa
  return (
    waterMl +
    nHcl * PHI_V_HCL_ML_PER_MOL +
    nNaoh * PHI_V_NAOH_ML_PER_MOL +
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

export function waterAmountMl(scene: LabScene): number | null {
  const item = findItem(scene, WATER_ID)
  const entry = optionalArray(item?.properties.composition).find(
    (c) => c.substance_id === 'water' && c.phase === 'liquid',
  )
  return entry?.amount_ml ?? null
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

export function dishAmountMl(scene: LabScene): number | null {
  const item = findItem(scene, DISH_ID)
  const entry = optionalArray(item?.properties.composition).find(
    (c) => c.substance_id === 'water' && c.phase === 'liquid',
  )
  return entry?.amount_ml ?? null
}

export function filtrateAmountMl(scene: LabScene): number | null {
  const item = findItem(scene, FILTRATE_ID)
  const entry = optionalArray(item?.properties.composition).find(
    (c) => c.substance_id === 'water' && c.phase === 'liquid',
  )
  return entry?.amount_ml ?? item?.properties.fill_ml ?? null
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

export function tongsHeldVesselId(
  scene: LabScene,
):
  | typeof WATER_ID
  | typeof DISH_ID
  | typeof H2O_ID
  | typeof HCL_ID
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
