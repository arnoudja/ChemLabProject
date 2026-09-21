import type { LabScene } from '../generated/contracts'
import { SPOON_SCOOP_MASS_G } from './LabBench'
import { optionalArray } from '../lib/scene'
import {
  CACL2_DELTA_H_SOLUTION_J_PER_MOL,
  CACL2_EXPLANATION,
  CACL2_MOLAR_MASS_G_PER_MOL,
  cloneScene,
  NACL_DELTA_H_SOLUTION_J_PER_MOL,
  NACL_EXPLANATION,
  NACL_MOLAR_MASS_G_PER_MOL,
  SAND_EXPLANATION,
  WATER_SPECIFIC_HEAT_J_PER_G_K,
} from './labBenchTestFixtures'

export function applySolidsScoop(
  scene: LabScene,
  sourceId: string,
): LabScene | { error: string; code: string; status: number } {
  const next = cloneScene(scene)
  const source = next.items.find((item) => item.id === sourceId)!
  const spoon = next.items.find((item) => item.id === 'spoon-1')!
  if (source.location === 'held') {
    return { error: 'invalid action', code: 'invalid_action', status: 400 }
  }
  const hasLiquid = optionalArray(source.properties.composition).some(
    (entry) => entry.phase === 'liquid' && (entry.amount_ml ?? 0) > 0,
  )
  if (hasLiquid) {
    return { error: 'invalid action', code: 'invalid_action', status: 400 }
  }
  const solids = optionalArray(source.properties.composition).filter((entry) => entry.phase === 'solid')
  const total = solids.reduce((sum, entry) => sum + (entry.amount_g ?? 0), 0)
  if (total <= 0) {
    return { error: 'invalid action', code: 'invalid_action', status: 400 }
  }
  const frac = Math.min(SPOON_SCOOP_MASS_G, total) / total
  spoon.location = 'hand'
  spoon.properties.source_item_id = sourceId
  spoon.properties.holding = solids
    .map((entry) => ({
      ...entry,
      amount_g: (entry.amount_g ?? 0) * frac,
    }))
    .filter((entry) => (entry.amount_g ?? 0) > 0)
  source.properties.composition = [
    ...optionalArray(source.properties.composition).filter((entry) => entry.phase !== 'solid'),
    ...solids
      .map((entry) => ({
        ...entry,
        amount_g: (entry.amount_g ?? 0) * (1 - frac),
      }))
      .filter((entry) => (entry.amount_g ?? 0) > 1e-12),
  ]
  const place =
    sourceId === 'filter-paper-1'
      ? 'paper'
      : sourceId === 'beaker-filtrate'
        ? 'filtrate beaker'
        : sourceId === 'beaker-water'
          ? 'beaker'
          : 'dish'
  next.last_events = [
    {
      kind: 'scooped',
      message: `Scooped solids from the ${place}.`,
    },
  ]
  next.version += 1
  return next
}

export function applySolidsDeposit(scene: LabScene, destId: string): LabScene {
  const next = cloneScene(scene)
  const spoon = next.items.find((item) => item.id === 'spoon-1')!
  const dest = next.items.find((item) => item.id === destId)!
  for (const held of optionalArray(spoon.properties.holding)) {
    if (held.phase !== 'solid') continue
    const existing = optionalArray(dest.properties.composition).find(
      (entry) => entry.substance_id === held.substance_id && entry.phase === 'solid',
    )
    if (existing) {
      existing.amount_g = (existing.amount_g ?? 0) + (held.amount_g ?? 0)
    } else {
      dest.properties.composition = [...optionalArray(dest.properties.composition), { ...held }]
    }
  }
  spoon.properties.holding = []
  spoon.properties.source_item_id = null
  const place =
    destId === 'filter-paper-1'
      ? 'paper'
      : destId === 'beaker-filtrate'
        ? 'filtrate beaker'
        : destId === 'beaker-water'
          ? 'beaker'
          : 'dish'
  next.last_events = [{ kind: 'returned', message: `Returned solids to the ${place}.` }]
  next.version += 1
  return next
}

export function applySolidsPutAway(scene: LabScene, sourceId: string): LabScene {
  const next = applySolidsDeposit(scene, sourceId)
  const spoon = next.items.find((item) => item.id === 'spoon-1')!
  spoon.location = 'bench'
  return next
}

export function withScoop(scene: LabScene, substance: 'nacl' | 'cacl2' | 'sand'): LabScene {
  const next = cloneScene(scene)
  const spoon = next.items.find((item) => item.id === 'spoon-1')!
  const stockId =
    substance === 'nacl' ? 'beaker-nacl' : substance === 'cacl2' ? 'beaker-cacl2' : 'beaker-sand'
  const stock = next.items.find((item) => item.id === stockId)!
  const solid = optionalArray(stock.properties.composition).find(
    (entry) => entry.substance_id === substance && entry.phase === 'solid',
  )
  if (solid) {
    const scoops = (solid.amount_scoop ?? 0) - 1
    solid.amount_scoop = scoops
    solid.amount_g = scoops * SPOON_SCOOP_MASS_G
  }
  spoon.location = 'hand'
  spoon.properties.holding = [
    {
      substance_id: substance,
      phase: 'solid',
      amount_ml: null,
      amount_scoop: 1,
      amount_g: SPOON_SCOOP_MASS_G,
      amount_mol: null,
    },
  ]
  next.last_events = [{ kind: 'scooped', message: `Scooped ${substance}.` }]
  next.version += 1
  return next
}

export function withPutBack(scene: LabScene, substance: 'nacl' | 'cacl2' | 'sand'): LabScene {
  const next = cloneScene(scene)
  const spoon = next.items.find((item) => item.id === 'spoon-1')!
  const stockId =
    substance === 'nacl' ? 'beaker-nacl' : substance === 'cacl2' ? 'beaker-cacl2' : 'beaker-sand'
  const stock = next.items.find((item) => item.id === stockId)!
  const solid = optionalArray(stock.properties.composition).find(
    (entry) => entry.substance_id === substance && entry.phase === 'solid',
  )
  const held = optionalArray(spoon.properties.holding)[0]
  const scoopsAdd = held?.amount_scoop ?? 1
  if (solid) {
    const scoops = (solid.amount_scoop ?? 0) + scoopsAdd
    solid.amount_scoop = scoops
    solid.amount_g = scoops * SPOON_SCOOP_MASS_G
  }
  spoon.properties.holding = []
  spoon.location = 'bench'
  next.last_events = [{ kind: 'returned', message: `Returned ${substance}.` }]
  next.version += 1
  return next
}

export function afterNaclPour(scene: LabScene): LabScene {
  const next = cloneScene(scene)
  const spoon = next.items.find((item) => item.id === 'spoon-1')!
  const water = next.items.find((item) => item.id === 'beaker-water')!
  spoon.properties.holding = []
  // Mirror server-authored aqueous ions after NaCl dissolve (not client dissociation).
  // SPOON_SCOOP_MASS_G NaCl / 58.44 g·mol⁻¹
  const moles = SPOON_SCOOP_MASS_G / NACL_MOLAR_MASS_G_PER_MOL
  water.properties.composition = [
    ...optionalArray(water.properties.composition),
    {
      substance_id: 'na+',
      phase: 'aqueous',
      amount_ml: null,
      amount_scoop: null,
      amount_g: null,
      amount_mol: moles,
    },
    {
      substance_id: 'cl-',
      phase: 'aqueous',
      amount_ml: null,
      amount_scoop: null,
      amount_g: null,
      amount_mol: moles,
    },
  ]
  // Mirror server endothermic cooling (water mass ≈ liquid amount_ml at 1 g/ml).
  const waterMassG =
    optionalArray(water.properties.composition).find(
      (entry) => entry.substance_id === 'water' && entry.phase === 'liquid',
    )?.amount_ml ?? 0
  const heatJ = moles * NACL_DELTA_H_SOLUTION_J_PER_MOL
  const currentT = water.properties.temperature_c ?? next.temperature_c
  water.properties.temperature_c =
    currentT - heatJ / (waterMassG * WATER_SPECIFIC_HEAT_J_PER_G_K)
  next.last_events = [
    { kind: 'poured', message: 'Poured onto water.' },
    { kind: 'dissolved', message: NACL_EXPLANATION },
  ]
  next.version += 1
  return next
}

export function afterCacl2Pour(scene: LabScene): LabScene {
  const next = cloneScene(scene)
  const spoon = next.items.find((item) => item.id === 'spoon-1')!
  const water = next.items.find((item) => item.id === 'beaker-water')!
  spoon.properties.holding = []
  const moles = SPOON_SCOOP_MASS_G / CACL2_MOLAR_MASS_G_PER_MOL
  water.properties.composition = [
    ...optionalArray(water.properties.composition),
    {
      substance_id: 'ca2+',
      phase: 'aqueous',
      amount_ml: null,
      amount_scoop: null,
      amount_g: null,
      amount_mol: moles,
    },
    {
      substance_id: 'cl-',
      phase: 'aqueous',
      amount_ml: null,
      amount_scoop: null,
      amount_g: null,
      amount_mol: 2 * moles,
    },
  ]
  const waterMassG =
    optionalArray(water.properties.composition).find(
      (entry) => entry.substance_id === 'water' && entry.phase === 'liquid',
    )?.amount_ml ?? 0
  const heatJ = moles * CACL2_DELTA_H_SOLUTION_J_PER_MOL
  const currentT = water.properties.temperature_c ?? next.temperature_c
  water.properties.temperature_c =
    currentT - heatJ / (waterMassG * WATER_SPECIFIC_HEAT_J_PER_G_K)
  next.last_events = [
    { kind: 'poured', message: 'Poured onto water.' },
    { kind: 'dissolved', message: CACL2_EXPLANATION },
  ]
  next.version += 1
  return next
}

export function afterSandPour(scene: LabScene): LabScene {
  const next = cloneScene(scene)
  const spoon = next.items.find((item) => item.id === 'spoon-1')!
  const water = next.items.find((item) => item.id === 'beaker-water')!
  spoon.properties.holding = []
  water.properties.composition = [
    ...optionalArray(water.properties.composition),
    {
      substance_id: 'sand',
      phase: 'solid',
      amount_ml: null,
      amount_scoop: 1,
      amount_g: SPOON_SCOOP_MASS_G,
      amount_mol: null,
    },
  ]
  next.last_events = [
    { kind: 'poured', message: 'Poured onto water.' },
    { kind: 'did_not_dissolve', message: SAND_EXPLANATION },
  ]
  next.version += 1
  return next
}
