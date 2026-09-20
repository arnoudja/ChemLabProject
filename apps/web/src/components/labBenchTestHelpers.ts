import { fireEvent, screen } from '@testing-library/react'
import { expect, vi } from 'vitest'
import type { LabAction, LabScene } from '../generated/contracts'
import {
  SPOON_SCOOP_MASS_G,
  STOCK_FULL_MASS_G,
  STOCK_FULL_SCOOPS,
  WATER_FULL_ML,
  DISTILLED_WATER_CAPACITY_ML,
  DISH_CAPACITY_ML,
  PIPETTE_VOLUME_ML,
  FILTRATE_CAPACITY_ML,
} from './LabBench'
import { optionalArray } from '../lib/scene'

export const NACL_EXPLANATION =
  'Sodium chloride (NaCl) dissolves in water at bench temperature.'
export const SAND_EXPLANATION =
  'Sand (silica) does not dissolve in water at bench temperature.'

/** Mirror `chemlab-core::scene::NACL_DELTA_H_SOLUTION_J_PER_MOL` — keep in sync. */
export const NACL_DELTA_H_SOLUTION_J_PER_MOL = 3880
/** Mirror `chemlab-core::scene::CACL2_DELTA_H_SOLUTION_J_PER_MOL` — keep in sync. */
export const CACL2_DELTA_H_SOLUTION_J_PER_MOL = -81300
/** Mirror `chemlab-core::scene::WATER_SPECIFIC_HEAT_J_PER_G_K` — keep in sync. */
export const WATER_SPECIFIC_HEAT_J_PER_G_K = 4.184
export const NACL_MOLAR_MASS_G_PER_MOL = 58.44
export const CACL2_MOLAR_MASS_G_PER_MOL = 110.98
export const CACL2_EXPLANATION =
  'Calcium chloride (CaCl2) dissolves in water at bench temperature.'

export function emptyProps() {
  return {
    volume_ml: null,
    fill_ml: null,
    transparent: null,
    colourless: null,
    temperature_c: null,
    composition: [] as LabScene['items'][number]['properties']['composition'],
    holding: [] as LabScene['items'][number]['properties']['holding'],
  }
}

export function initialScene(): LabScene {
  return {
    lab_id: 'lab-1',
    version: 0,
    temperature_c: 20,
    last_events: [],
    items: [
      {
        id: 'spoon-1',
        kind: 'spoon',
        label: 'Spoon',
        location: 'bench',
        properties: emptyProps(),
      },
      {
        id: 'beaker-h2o',
        kind: 'beaker',
        label: 'Distilled water',
        location: 'bench',
        properties: {
          volume_ml: DISTILLED_WATER_CAPACITY_ML,
          fill_ml: DISTILLED_WATER_CAPACITY_ML,
          transparent: true,
          colourless: true,
          temperature_c: 20,
          composition: [
            { substance_id: 'water', phase: 'liquid', amount_ml: DISTILLED_WATER_CAPACITY_ML, amount_scoop: null, amount_g: null, amount_mol: null},
          ],
          holding: [],
        },
      },
      {
        id: 'beaker-nacl',
        kind: 'beaker',
        label: 'Sodium chloride',
        location: 'bench',
        properties: {
          volume_ml: 250,
          fill_ml: 100,
          transparent: true,
          colourless: true,
          temperature_c: 20,
          composition: [{ substance_id: 'nacl', phase: 'solid', amount_ml: null, amount_scoop: STOCK_FULL_SCOOPS, amount_g: STOCK_FULL_MASS_G, amount_mol: null}],
          holding: [],
        },
      },
      {
        id: 'beaker-cacl2',
        kind: 'beaker',
        label: 'Calcium chloride',
        location: 'bench',
        properties: {
          volume_ml: 250,
          fill_ml: 100,
          transparent: true,
          colourless: true,
          temperature_c: 20,
          composition: [{ substance_id: 'cacl2', phase: 'solid', amount_ml: null, amount_scoop: STOCK_FULL_SCOOPS, amount_g: STOCK_FULL_MASS_G, amount_mol: null}],
          holding: [],
        },
      },
      {
        id: 'beaker-sand',
        kind: 'beaker',
        label: 'Sand',
        location: 'bench',
        properties: {
          volume_ml: 250,
          fill_ml: 100,
          transparent: true,
          colourless: true,
          temperature_c: 20,
          composition: [{ substance_id: 'sand', phase: 'solid', amount_ml: null, amount_scoop: STOCK_FULL_SCOOPS, amount_g: STOCK_FULL_MASS_G, amount_mol: null}],
          holding: [],
        },
      },
      {
        id: 'beaker-water',
        kind: 'beaker',
        label: 'Beaker',
        location: 'bench',
        properties: {
          volume_ml: 250,
          fill_ml: 0,
          transparent: true,
          colourless: true,
          temperature_c: 20,
          composition: [],
          holding: [],
        },
      },
      {
        id: 'pipette-1',
        kind: 'pipette',
        label: 'Pipette',
        location: 'bench',
        properties: {
          volume_ml: PIPETTE_VOLUME_ML,
          fill_ml: 0,
          transparent: true,
          colourless: true,
          temperature_c: null,
          composition: [],
          holding: [],
          source_item_id: null,
        },
      },
      {
        id: 'tongs-1',
        kind: 'tongs',
        label: 'Tongs',
        location: 'bench',
        properties: {
          ...emptyProps(),
          source_item_id: null,
        },
      },
      {
        id: 'dish-1',
        kind: 'evaporation_dish',
        label: 'Evaporation dish',
        location: 'bench',
        properties: {
          volume_ml: 25,
          fill_ml: 0,
          transparent: true,
          colourless: true,
          temperature_c: 20,
          composition: [],
          holding: [],
        },
      },
      {
        id: 'burner-1',
        kind: 'burner',
        label: 'Burner',
        location: 'bench',
        properties: {
          volume_ml: null,
          fill_ml: null,
          transparent: null,
          colourless: null,
          temperature_c: null,
          composition: [],
          holding: [],
          on: false,
        },
      },
      {
        id: 'beaker-filtrate',
        kind: 'beaker',
        label: 'Filtrate',
        location: 'bench',
        properties: {
          volume_ml: FILTRATE_CAPACITY_ML,
          fill_ml: 0,
          transparent: true,
          colourless: true,
          temperature_c: 20,
          composition: [],
          holding: [],
        },
      },
      {
        id: 'filter-paper-1',
        kind: 'filter_paper',
        label: 'Filter paper',
        location: 'bench',
        properties: {
          ...emptyProps(),
        },
      },
    ],
  }
}

export function cloneScene(scene: LabScene): LabScene {
  return structuredClone(scene)
}

export function withFilledMainBeaker(scene: LabScene, amountMl = WATER_FULL_ML): LabScene {
  const next = cloneScene(scene)
  const water = next.items.find((item) => item.id === 'beaker-water')!
  water.properties.fill_ml = amountMl
  water.properties.composition = [
    {
      substance_id: 'water',
      phase: 'liquid',
      amount_ml: amountMl,
      amount_scoop: null,
      amount_g: null,
      amount_mol: null,
    },
  ]
  return next
}

export function filledScene(): LabScene {
  return withFilledMainBeaker(initialScene())
}

export function withDryDishSolids(
  scene: LabScene,
  solids: { substance_id: 'nacl' | 'cacl2' | 'sand'; amount_g: number }[],
): LabScene {
  const next = cloneScene(scene)
  const dish = next.items.find((item) => item.id === 'dish-1')!
  dish.location = 'bench'
  dish.properties.composition = solids.map((solid) => ({
    substance_id: solid.substance_id,
    phase: 'solid' as const,
    amount_ml: null,
    amount_scoop: null,
    amount_g: solid.amount_g,
    amount_mol: null,
  }))
  dish.properties.fill_ml = 0
  return next
}

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

export function liquidWaterEntry(item: LabScene['items'][number]) {
  return optionalArray(item.properties.composition).find(
    (entry) => entry.substance_id === 'water' && entry.phase === 'liquid',
  )
}

export function pipetteIsFull(scene: LabScene): boolean {
  const pipette = scene.items.find((item) => item.id === 'pipette-1')
  return optionalArray(pipette?.properties.holding).some(
    (entry) => entry.substance_id === 'water' && entry.phase === 'liquid' && (entry.amount_ml ?? 0) > 0,
  )
}

export function applyTongsPickUp(scene: LabScene, targetId: string): LabScene {
  const next = cloneScene(scene)
  const tongs = next.items.find((item) => item.id === 'tongs-1')!
  const target = next.items.find((item) => item.id === targetId)!
  target.location = 'held'
  tongs.location = 'hand'
  tongs.properties.source_item_id = targetId
  if (targetId === 'dish-1') {
    const burner = next.items.find((item) => item.id === 'burner-1')!
    burner.properties.on = false
  }
  next.last_events = [{ kind: 'picked', message: `Picked up ${targetId} with the tongs.` }]
  next.version += 1
  return next
}

export function liquidCapacityMl(itemId: string): number {
  if (itemId === 'dish-1') return DISH_CAPACITY_ML
  if (itemId === 'beaker-h2o') return DISTILLED_WATER_CAPACITY_ML
  if (itemId === 'beaker-filtrate') return FILTRATE_CAPACITY_ML
  return 250
}

export function applyTongsPour(scene: LabScene, destId: string): LabScene | { error: string; code: string; status: number } {
  const next = cloneScene(scene)
  const tongs = next.items.find((item) => item.id === 'tongs-1')!
  const sourceId = tongs.properties.source_item_id
  if (!sourceId || sourceId === destId) {
    return { error: 'invalid action', code: 'invalid_action', status: 400 }
  }
  const source = next.items.find((item) => item.id === sourceId)!
  const dest = next.items.find((item) => item.id === destId)!
  const sourceWater = liquidWaterEntry(source)
  const sourceMl = sourceWater?.amount_ml ?? 0
  const destWater = liquidWaterEntry(dest)
  const destMl = destWater?.amount_ml ?? 0
  const room = Math.max(0, liquidCapacityMl(destId) - destMl)
  const solids = optionalArray(source.properties.composition).filter((entry) => entry.phase === 'solid')
  if (sourceMl <= 0) {
    if (solids.length === 0) {
      return { error: 'empty holding', code: 'empty_holding', status: 400 }
    }
    dest.properties.composition = [...optionalArray(dest.properties.composition), ...solids]
    source.properties.composition = optionalArray(source.properties.composition).filter(
      (entry) => entry.phase !== 'solid',
    )
    next.last_events = [{ kind: 'poured', message: 'Poured solids into the vessel.' }]
    next.version += 1
    return next
  }
  if (room <= 0) {
    return { error: 'invalid action', code: 'invalid_action', status: 400 }
  }
  const transferred = Math.min(sourceMl, room)
  if (sourceWater) {
    sourceWater.amount_ml = sourceMl - transferred
    source.properties.fill_ml = sourceWater.amount_ml
  }
  if (destWater) {
    destWater.amount_ml = destMl + transferred
    dest.properties.fill_ml = destWater.amount_ml
  } else {
    dest.properties.composition = [
      ...optionalArray(dest.properties.composition),
      {
        substance_id: 'water',
        phase: 'liquid',
        amount_ml: transferred,
        amount_scoop: null,
        amount_g: null,
        amount_mol: null,
      },
    ]
    dest.properties.fill_ml = transferred
  }
  next.last_events = [{ kind: 'poured', message: 'Poured from the held vessel.' }]
  next.version += 1
  return next
}

export function applyFilterPour(scene: LabScene): LabScene | { error: string; code: string; status: number } {
  const next = cloneScene(scene)
  const tongs = next.items.find((item) => item.id === 'tongs-1')!
  const sourceId = tongs.properties.source_item_id
  const dest = next.items.find((item) => item.id === 'beaker-filtrate')
  const paper = next.items.find((item) => item.id === 'filter-paper-1')
  if (!sourceId || !dest || !paper) {
    return { error: 'invalid action', code: 'invalid_action', status: 400 }
  }
  if (dest.location !== 'bench') {
    return { error: 'invalid action', code: 'invalid_action', status: 400 }
  }
  const source = next.items.find((item) => item.id === sourceId)!
  const sourceWater = liquidWaterEntry(source)
  const sourceMl = sourceWater?.amount_ml ?? 0
  const destMl = liquidWaterEntry(dest)?.amount_ml ?? 0
  const room = Math.max(0, FILTRATE_CAPACITY_ML - destMl)
  const solids = optionalArray(source.properties.composition).filter((entry) => entry.phase === 'solid')
  if (sourceMl <= 0) {
    if (solids.length === 0) {
      return { error: 'empty holding', code: 'empty_holding', status: 400 }
    }
    return { error: 'invalid action', code: 'invalid_action', status: 400 }
  }
  if (room <= 0) {
    return { error: 'invalid action', code: 'invalid_action', status: 400 }
  }
  const transferred = Math.min(sourceMl, room)
  const frac = transferred / sourceMl
  if (sourceWater) {
    sourceWater.amount_ml = sourceMl - transferred
    source.properties.fill_ml = sourceWater.amount_ml
  }
  const destWater = liquidWaterEntry(dest)
  if (destWater) {
    destWater.amount_ml = destMl + transferred
    dest.properties.fill_ml = destWater.amount_ml
  } else {
    dest.properties.composition = [
      ...optionalArray(dest.properties.composition),
      {
        substance_id: 'water',
        phase: 'liquid',
        amount_ml: transferred,
        amount_scoop: null,
        amount_g: null,
        amount_mol: null,
      },
    ]
    dest.properties.fill_ml = transferred
  }
  for (const solid of solids) {
    const moved = (solid.amount_g ?? 0) * frac
    solid.amount_g = (solid.amount_g ?? 0) - moved
    const existing = optionalArray(paper.properties.composition).find(
      (entry) => entry.substance_id === solid.substance_id && entry.phase === 'solid',
    )
    if (existing) {
      existing.amount_g = (existing.amount_g ?? 0) + moved
    } else if (moved > 0) {
      paper.properties.composition = [
        ...optionalArray(paper.properties.composition),
        { ...solid, amount_g: moved },
      ]
    }
  }
  source.properties.composition = optionalArray(source.properties.composition).filter(
    (entry) => entry.phase !== 'solid' || (entry.amount_g ?? 0) > 1e-12,
  )
  next.last_events = [{ kind: 'poured', message: 'Filtered into the filtrate beaker.' }]
  next.version += 1
  return next
}

export function applyTongsUse(
  scene: LabScene,
  targetId: string,
): LabScene | { error: string; code: string; status: number } {
  const held = scene.items.find((item) => item.id === 'tongs-1')?.properties.source_item_id
  if (targetId === 'filter-paper-1') {
    if (!held) return applyTongsPickUp(scene, 'filter-paper-1')
    if (held === 'beaker-filtrate') {
      return { error: 'invalid action', code: 'invalid_action', status: 400 }
    }
    const source = scene.items.find((item) => item.id === held)
    const sourceMl = liquidWaterEntry(source!)?.amount_ml ?? 0
    const solids = optionalArray(source?.properties.composition).filter((entry) => entry.phase === 'solid')
    if (sourceMl <= 0 && solids.length > 0) {
      return applyTongsPour(scene, 'filter-paper-1')
    }
    return applyFilterPour(scene)
  }
  if (targetId === 'beaker-filtrate') {
    if (!held) return applyTongsPickUp(scene, 'beaker-filtrate')
    if (held === 'beaker-filtrate') {
      return { error: 'invalid action', code: 'invalid_action', status: 400 }
    }
    return applyTongsPour(scene, 'beaker-filtrate')
  }
  if (!held) return applyTongsPickUp(scene, targetId)
  return applyTongsPour(scene, targetId)
}

export function applyTongsPutAway(scene: LabScene): LabScene {
  const next = cloneScene(scene)
  const tongs = next.items.find((item) => item.id === 'tongs-1')!
  const heldId = tongs.properties.source_item_id
  if (heldId) {
    const held = next.items.find((item) => item.id === heldId)
    if (held) held.location = 'bench'
    tongs.properties.source_item_id = null
  }
  tongs.location = 'bench'
  next.last_events = []
  next.version += 1
  return next
}

export function applyPipetteFill(scene: LabScene, sourceId: string): LabScene {
  const next = cloneScene(scene)
  const source = next.items.find((item) => item.id === sourceId)!
  const pipette = next.items.find((item) => item.id === 'pipette-1')!
  const water = liquidWaterEntry(source)
  if (!water || (water.amount_ml ?? 0) < PIPETTE_VOLUME_ML) return scene
  water.amount_ml = (water.amount_ml ?? 0) - PIPETTE_VOLUME_ML
  source.properties.fill_ml = water.amount_ml
  pipette.location = 'hand'
  pipette.properties.holding = [
    {
      substance_id: 'water',
      phase: 'liquid',
      amount_ml: PIPETTE_VOLUME_ML,
      amount_scoop: null,
      amount_g: null,
      amount_mol: null,
    },
  ]
  pipette.properties.source_item_id = sourceId
  pipette.properties.temperature_c = source.properties.temperature_c
  pipette.properties.fill_ml = PIPETTE_VOLUME_ML
  next.last_events = [
    {
      kind: 'pipetted',
      message: `Filled the pipette with ${PIPETTE_VOLUME_ML.toFixed(2)} ml of solution.`,
    },
  ]
  next.version += 1
  return next
}

export function applyPipetteEmpty(scene: LabScene, targetId: string): LabScene {
  const next = cloneScene(scene)
  const target = next.items.find((item) => item.id === targetId)!
  const pipette = next.items.find((item) => item.id === 'pipette-1')!
  if (!pipetteIsFull(next)) return scene
  const existing = liquidWaterEntry(target)
  if (existing) {
    existing.amount_ml = (existing.amount_ml ?? 0) + PIPETTE_VOLUME_ML
    target.properties.fill_ml = existing.amount_ml
  } else {
    target.properties.composition = [
      ...optionalArray(target.properties.composition),
      {
        substance_id: 'water',
        phase: 'liquid',
        amount_ml: PIPETTE_VOLUME_ML,
        amount_scoop: null,
        amount_g: null,
        amount_mol: null,
      },
    ]
    target.properties.fill_ml = PIPETTE_VOLUME_ML
  }
  pipette.properties.holding = []
  pipette.properties.source_item_id = null
  pipette.properties.fill_ml = 0
  pipette.properties.temperature_c = null
  next.last_events = [{ kind: 'poured', message: 'Emptied the pipette into the vessel.' }]
  next.version += 1
  return next
}

export function applyPipetteUse(scene: LabScene, targetId: string): LabScene {
  return pipetteIsFull(scene) ? applyPipetteEmpty(scene, targetId) : applyPipetteFill(scene, targetId)
}

export function applyPipettePutAway(scene: LabScene): LabScene {
  const pipette = scene.items.find((item) => item.id === 'pipette-1')!
  const sourceId = pipette.properties.source_item_id
  let next = cloneScene(scene)
  if (pipetteIsFull(next) && sourceId) {
    next = applyPipetteEmpty(next, sourceId)
  }
  const tool = next.items.find((item) => item.id === 'pipette-1')!
  tool.location = 'bench'
  return next
}

export function withBurnerToggle(scene: LabScene): LabScene {
  const next = cloneScene(scene)
  const burner = next.items.find((item) => item.id === 'burner-1')!
  const dish = next.items.find((item) => item.id === 'dish-1')!
  const hasLiquid = (liquidWaterEntry(dish)?.amount_ml ?? 0) > 0
  const currentlyOn = burner.properties.on === true
  const nextOn = currentlyOn ? false : hasLiquid
  burner.properties.on = nextOn
  if (nextOn !== currentlyOn) {
    next.last_events = [{ kind: 'toggled', message: nextOn ? 'Burner on.' : 'Burner off.' }]
  }
  next.version += 1
  return next
}

export function jsonResponse(body: unknown, status = 200) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  })
}

export function stubLabFetch(options?: {
  scene?: LabScene
  actionHandler?: (action: LabAction, scene: LabScene) => LabScene | { error: string; code: string; status: number }
  sceneError?: { error: string; code: string; status: number }
}) {
  let scene = cloneScene(options?.scene ?? withFilledMainBeaker(initialScene()))
  return vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = String(input)
    if (url === '/api/auth/csrf') {
      return jsonResponse({ csrf_token: 'tok-123' })
    }
    if (url === '/api/lab/scene') {
      if (options?.sceneError) {
        return jsonResponse(
          { error: options.sceneError.error, code: options.sceneError.code },
          options.sceneError.status,
        )
      }
      return jsonResponse(scene)
    }
    if (url === '/api/lab/action') {
      const action = init?.body ? (JSON.parse(String(init.body)) as LabAction) : null
      if (!action) {
        return jsonResponse({ error: 'bad request', code: 'invalid_input' }, 400)
      }
      if (options?.actionHandler) {
        const result = options.actionHandler(action, scene)
        if ('error' in result) {
          return jsonResponse({ error: result.error, code: result.code }, result.status)
        }
        scene = result
        return jsonResponse({ scene })
      }
      if (action.type === 'toggle_burner') {
        scene = withBurnerToggle(scene)
        return jsonResponse({ scene })
      }
      if (action.type === 'use_tool' && action.tool_item_id === 'pipette-1') {
        scene = applyPipetteUse(scene, action.target_item_id)
        return jsonResponse({ scene })
      }
      if (action.type === 'use_tool' && action.tool_item_id === 'tongs-1') {
        const result = applyTongsUse(scene, action.target_item_id)
        if ('error' in result) {
          return jsonResponse({ error: result.error, code: result.code }, result.status)
        }
        scene = result
        return jsonResponse({ scene })
      }
      if (
        action.type === 'use_tool' &&
        action.tool_item_id === 'spoon-1' &&
        (action.target_item_id === 'dish-1' ||
          action.target_item_id === 'filter-paper-1' ||
          action.target_item_id === 'beaker-filtrate' ||
          action.target_item_id === 'beaker-water')
      ) {
        const held = optionalArray(
          scene.items.find((item) => item.id === 'spoon-1')?.properties.holding,
        )
        if (held.length > 0) {
          scene = applySolidsDeposit(scene, action.target_item_id)
          return jsonResponse({ scene })
        }
        const result = applySolidsScoop(scene, action.target_item_id)
        if ('error' in result) {
          return jsonResponse({ error: result.error, code: result.code }, result.status)
        }
        scene = result
        return jsonResponse({ scene })
      }
      if (action.type === 'pour' && action.source_item_id === 'pipette-1') {
        scene = applyPipetteEmpty(scene, action.target_item_id)
        return jsonResponse({ scene })
      }
      if (action.type === 'put_away' && action.tool_item_id === 'pipette-1') {
        scene = applyPipettePutAway(scene)
        return jsonResponse({ scene })
      }
      if (action.type === 'put_away' && action.tool_item_id === 'tongs-1') {
        scene = applyTongsPutAway(scene)
        return jsonResponse({ scene })
      }
      if (action.type === 'use_tool' && (action.target_item_id === 'beaker-nacl' || action.target_item_id === 'beaker-cacl2' || action.target_item_id === 'beaker-sand')) {
        const targetSubstance =
          action.target_item_id === 'beaker-nacl'
            ? 'nacl'
            : action.target_item_id === 'beaker-cacl2'
              ? 'cacl2'
              : 'sand'
        const held = optionalArray(
          scene.items.find((item) => item.id === 'spoon-1')?.properties.holding,
        )
        const species = [...new Set(held.filter((entry) => entry.phase === 'solid').map((entry) => entry.substance_id))]
        if (species.length > 1) {
          return jsonResponse({ error: 'invalid action', code: 'invalid_action' }, 400)
        }
        const first = held[0]
        if (first) {
          if (first.substance_id === targetSubstance) {
            scene = withPutBack(scene, targetSubstance)
            return jsonResponse({ scene })
          }
          return jsonResponse({ error: 'invalid action', code: 'invalid_action' }, 400)
        }
        scene = withScoop(scene, targetSubstance)
        return jsonResponse({ scene })
      }
      if (action.type === 'put_away') {
        const spoon = scene.items.find((item) => item.id === 'spoon-1')
        if (spoon?.properties.source_item_id === 'dish-1') {
          scene = applySolidsPutAway(scene, 'dish-1')
          return jsonResponse({ scene })
        }
        if (spoon?.properties.source_item_id === 'filter-paper-1') {
          scene = applySolidsPutAway(scene, 'filter-paper-1')
          return jsonResponse({ scene })
        }
        const held = optionalArray(spoon?.properties.holding)[0]
        if (held && (held.substance_id === 'nacl' || held.substance_id === 'cacl2' || held.substance_id === 'sand')) {
          scene = withPutBack(scene, held.substance_id)
          return jsonResponse({ scene })
        }
        const next = cloneScene(scene)
        const nextSpoon = next.items.find((item) => item.id === 'spoon-1')!
        nextSpoon.location = 'bench'
        next.last_events = []
        next.version += 1
        scene = next
        return jsonResponse({ scene })
      }
      if (action.type === 'pour') {
        const held = optionalArray(
          scene.items.find((item) => item.id === 'spoon-1')?.properties.holding,
        )[0]
        if (held?.substance_id === 'nacl') {
          scene = afterNaclPour(scene)
          return jsonResponse({ scene })
        }
        if (held?.substance_id === 'cacl2') {
          scene = afterCacl2Pour(scene)
          return jsonResponse({ scene })
        }
        if (held?.substance_id === 'sand') {
          scene = afterSandPour(scene)
          return jsonResponse({ scene })
        }
      }
      if (action.type === 'reset') {
        const next = initialScene()
        next.lab_id = scene.lab_id
        next.version = scene.version + 1
        next.last_events = [{ kind: 'reset', message: 'Lab reset to the starting bench.' }]
        scene = next
        return jsonResponse({ scene })
      }
      return jsonResponse({ error: 'invalid action', code: 'invalid_action' }, 400)
    }
    if (url === '/api/lab/dissolve') {
      throw new Error('LabBench must not call /api/lab/dissolve')
    }
    return jsonResponse({ error: 'not found', code: 'not_found' }, 404)
  })
}

export function lastActionInit(fetchMock: ReturnType<typeof vi.fn>) {
  const calls = fetchMock.mock.calls.filter(([url]) => String(url) === '/api/lab/action')
  return calls.at(-1)?.[1] as RequestInit | undefined
}

export function expectCsrfLabAction(fetchMock: ReturnType<typeof vi.fn>, body: unknown) {
  expect(lastActionInit(fetchMock)).toEqual(
    expect.objectContaining({
      method: 'POST',
      credentials: 'include',
      headers: expect.objectContaining({
        'content-type': 'application/json',
        'X-CSRF-Token': 'tok-123',
      }),
      body: JSON.stringify(body),
    }),
  )
}

export function precedesInDocument(earlier: HTMLElement, later: HTMLElement) {
  return Boolean(earlier.compareDocumentPosition(later) & Node.DOCUMENT_POSITION_FOLLOWING)
}

export function clickCarousel(direction: 'next' | 'previous') {
  const name = direction === 'next' ? 'Next ingredient' : 'Previous ingredient'
  fireEvent.click(screen.getByRole('button', { name }))
}

export function showStockInCarousel(name: string) {
  for (let i = 0; i < 4; i++) {
    if (screen.queryByRole('button', { name })) return
    clickCarousel('next')
  }
  throw new Error(`stock ${name} not visible after wrapping carousel`)
}

export function clickStock(name: string) {
  showStockInCarousel(name)
  fireEvent.click(screen.getByRole('button', { name }))
}

export function clickToolCarousel(direction: 'next' | 'previous') {
  const name = direction === 'next' ? 'Next tool' : 'Previous tool'
  fireEvent.click(screen.getByRole('button', { name }))
}

export function showToolInCarousel(name: string) {
  for (let i = 0; i < 3; i++) {
    if (screen.queryByRole('button', { name })) return
    clickToolCarousel('next')
  }
  throw new Error(`tool ${name} not visible after wrapping carousel`)
}

export function clickSpoon() {
  showToolInCarousel('Spoon')
  fireEvent.click(screen.getByRole('button', { name: 'Spoon' }))
}

export function clickTongs() {
  showToolInCarousel('Tongs')
  fireEvent.click(screen.getByRole('button', { name: 'Tongs' }))
}

