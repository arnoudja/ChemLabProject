/** @vitest-environment jsdom */
import '@testing-library/jest-dom/vitest'
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { LabAction, LabScene } from '../generated/contracts'
import {
  LabBench,
  SPOON_SCOOP_MASS_G,
  STOCK_FULL_MASS_G,
  STOCK_FULL_SCOOPS,
  WATER_FULL_ML,
  DISH_CAPACITY_ML,
  PIPETTE_VOLUME_ML,
  dishFillRatio,
  stockFillRatio,
  waterFillRatio,
} from './LabBench'
import { clearCsrfTokenCache } from '../lib/api'
import { optionalArray } from '../lib/scene'

const NACL_EXPLANATION =
  'Sodium chloride (NaCl) dissolves in water at bench temperature.'
const SAND_EXPLANATION =
  'Sand (silica) does not dissolve in water at bench temperature.'

/** Mirror `chemlab-core::scene::NACL_DELTA_H_SOLUTION_J_PER_MOL` — keep in sync. */
const NACL_DELTA_H_SOLUTION_J_PER_MOL = 3880
/** Mirror `chemlab-core::scene::CACL2_DELTA_H_SOLUTION_J_PER_MOL` — keep in sync. */
const CACL2_DELTA_H_SOLUTION_J_PER_MOL = -81300
/** Mirror `chemlab-core::scene::WATER_SPECIFIC_HEAT_J_PER_G_K` — keep in sync. */
const WATER_SPECIFIC_HEAT_J_PER_G_K = 4.184
const NACL_MOLAR_MASS_G_PER_MOL = 58.44
const CACL2_MOLAR_MASS_G_PER_MOL = 110.98
const CACL2_EXPLANATION =
  'Calcium chloride (CaCl2) dissolves in water at bench temperature.'

function emptyProps() {
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

function initialScene(): LabScene {
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
        label: 'Water',
        location: 'bench',
        properties: {
          volume_ml: 250,
          fill_ml: 200,
          transparent: true,
          colourless: true,
          temperature_c: 20,
          composition: [
            { substance_id: 'water', phase: 'liquid', amount_ml: 200, amount_scoop: null, amount_g: null, amount_mol: null},
          ],
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
    ],
  }
}

function cloneScene(scene: LabScene): LabScene {
  return structuredClone(scene)
}

function withDryDishSolids(
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

function applyDishScoop(scene: LabScene): LabScene | { error: string; code: string; status: number } {
  const next = cloneScene(scene)
  const dish = next.items.find((item) => item.id === 'dish-1')!
  const spoon = next.items.find((item) => item.id === 'spoon-1')!
  if (dish.location === 'held') {
    return { error: 'invalid action', code: 'invalid_action', status: 400 }
  }
  const hasLiquid = optionalArray(dish.properties.composition).some(
    (entry) => entry.phase === 'liquid' && (entry.amount_ml ?? 0) > 0,
  )
  if (hasLiquid) {
    return { error: 'invalid action', code: 'invalid_action', status: 400 }
  }
  const solids = optionalArray(dish.properties.composition).filter((entry) => entry.phase === 'solid')
  const total = solids.reduce((sum, entry) => sum + (entry.amount_g ?? 0), 0)
  if (total <= 0) {
    return { error: 'invalid action', code: 'invalid_action', status: 400 }
  }
  const frac = Math.min(SPOON_SCOOP_MASS_G, total) / total
  spoon.location = 'hand'
  spoon.properties.source_item_id = 'dish-1'
  spoon.properties.holding = solids
    .map((entry) => ({
      ...entry,
      amount_g: (entry.amount_g ?? 0) * frac,
    }))
    .filter((entry) => (entry.amount_g ?? 0) > 0)
  dish.properties.composition = [
    ...optionalArray(dish.properties.composition).filter((entry) => entry.phase !== 'solid'),
    ...solids
      .map((entry) => ({
        ...entry,
        amount_g: (entry.amount_g ?? 0) * (1 - frac),
      }))
      .filter((entry) => (entry.amount_g ?? 0) > 1e-12),
  ]
  next.last_events = [{ kind: 'scooped', message: 'Scooped solids from the dish.' }]
  next.version += 1
  return next
}

function applyDishPutAway(scene: LabScene): LabScene {
  const next = cloneScene(scene)
  const spoon = next.items.find((item) => item.id === 'spoon-1')!
  const dish = next.items.find((item) => item.id === 'dish-1')!
  for (const held of optionalArray(spoon.properties.holding)) {
    if (held.phase !== 'solid') continue
    const existing = optionalArray(dish.properties.composition).find(
      (entry) => entry.substance_id === held.substance_id && entry.phase === 'solid',
    )
    if (existing) {
      existing.amount_g = (existing.amount_g ?? 0) + (held.amount_g ?? 0)
    } else {
      dish.properties.composition = [...optionalArray(dish.properties.composition), { ...held }]
    }
  }
  spoon.properties.holding = []
  spoon.properties.source_item_id = null
  spoon.location = 'bench'
  next.last_events = [{ kind: 'returned', message: 'Returned solids to the dish.' }]
  next.version += 1
  return next
}

function withScoop(scene: LabScene, substance: 'nacl' | 'cacl2' | 'sand'): LabScene {
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

function withPutBack(scene: LabScene, substance: 'nacl' | 'cacl2' | 'sand'): LabScene {
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

function afterNaclPour(scene: LabScene): LabScene {
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

function afterCacl2Pour(scene: LabScene): LabScene {
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

function afterSandPour(scene: LabScene): LabScene {
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

function liquidWaterEntry(item: LabScene['items'][number]) {
  return optionalArray(item.properties.composition).find(
    (entry) => entry.substance_id === 'water' && entry.phase === 'liquid',
  )
}

function pipetteIsFull(scene: LabScene): boolean {
  const pipette = scene.items.find((item) => item.id === 'pipette-1')
  return optionalArray(pipette?.properties.holding).some(
    (entry) => entry.substance_id === 'water' && entry.phase === 'liquid' && (entry.amount_ml ?? 0) > 0,
  )
}

function applyTongsPickUp(scene: LabScene, targetId: string): LabScene {
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

function liquidCapacityMl(itemId: string): number {
  return itemId === 'dish-1' ? DISH_CAPACITY_ML : 250
}

function applyTongsPour(scene: LabScene, destId: string): LabScene | { error: string; code: string; status: number } {
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

function applyTongsUse(
  scene: LabScene,
  targetId: string,
): LabScene | { error: string; code: string; status: number } {
  const held = scene.items.find((item) => item.id === 'tongs-1')?.properties.source_item_id
  if (!held) return applyTongsPickUp(scene, targetId)
  return applyTongsPour(scene, targetId)
}

function applyTongsPutAway(scene: LabScene): LabScene {
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

function applyPipetteFill(scene: LabScene, sourceId: string): LabScene {
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

function applyPipetteEmpty(scene: LabScene, targetId: string): LabScene {
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

function applyPipetteUse(scene: LabScene, targetId: string): LabScene {
  return pipetteIsFull(scene) ? applyPipetteEmpty(scene, targetId) : applyPipetteFill(scene, targetId)
}

function applyPipettePutAway(scene: LabScene): LabScene {
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

function withBurnerToggle(scene: LabScene): LabScene {
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

function jsonResponse(body: unknown, status = 200) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  })
}

function stubLabFetch(options?: {
  scene?: LabScene
  actionHandler?: (action: LabAction, scene: LabScene) => LabScene | { error: string; code: string; status: number }
  sceneError?: { error: string; code: string; status: number }
}) {
  let scene = cloneScene(options?.scene ?? initialScene())
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
      if (action.type === 'use_tool' && action.tool_item_id === 'spoon-1' && action.target_item_id === 'dish-1') {
        const result = applyDishScoop(scene)
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
          scene = applyDishPutAway(scene)
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

function lastActionInit(fetchMock: ReturnType<typeof vi.fn>) {
  const calls = fetchMock.mock.calls.filter(([url]) => String(url) === '/api/lab/action')
  return calls.at(-1)?.[1] as RequestInit | undefined
}

function expectCsrfLabAction(fetchMock: ReturnType<typeof vi.fn>, body: unknown) {
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

function precedesInDocument(earlier: HTMLElement, later: HTMLElement) {
  return Boolean(earlier.compareDocumentPosition(later) & Node.DOCUMENT_POSITION_FOLLOWING)
}

function clickCarousel(direction: 'next' | 'previous') {
  const name = direction === 'next' ? 'Next ingredient' : 'Previous ingredient'
  fireEvent.click(screen.getByRole('button', { name }))
}

function showStockInCarousel(name: string) {
  for (let i = 0; i < 3; i++) {
    if (screen.queryByRole('button', { name })) return
    clickCarousel('next')
  }
  throw new Error(`stock ${name} not visible after wrapping carousel`)
}

function clickToolCarousel(direction: 'next' | 'previous') {
  const name = direction === 'next' ? 'Next tool' : 'Previous tool'
  fireEvent.click(screen.getByRole('button', { name }))
}

function showToolInCarousel(name: string) {
  for (let i = 0; i < 3; i++) {
    if (screen.queryByRole('button', { name })) return
    clickToolCarousel('next')
  }
  throw new Error(`tool ${name} not visible after wrapping carousel`)
}

function clickSpoon() {
  showToolInCarousel('Spoon')
  fireEvent.click(screen.getByRole('button', { name: 'Spoon' }))
}

function clickTongs() {
  showToolInCarousel('Tongs')
  fireEvent.click(screen.getByRole('button', { name: 'Tongs' }))
}

describe('LabBench', () => {
  beforeEach(() => {
    clearCsrfTokenCache()
  })

  afterEach(() => {
    cleanup()
    vi.unstubAllGlobals()
    vi.useRealTimers()
  })

  it('loads the server scene and does not call dissolve', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)

    expect(await screen.findByRole('button', { name: 'Pipette' })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Spoon' })).not.toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Calcium chloride (CaCl2)' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Sand' })).not.toBeInTheDocument()
    expect(document.querySelector('[data-stock-solid="cacl2"]')).toBeNull()
    expect(document.querySelector('[data-stock-solid="sand"]')).toBeNull()
    expect(screen.getByRole('button', { name: 'Water beaker' })).toBeInTheDocument()
    const saltLabel = document.querySelector('[data-stock-label="nacl"]')
    expect(saltLabel?.textContent).toContain('NaCl')
    expect(saltLabel?.textContent).toContain('(Sodium chloride)')
    expect(saltLabel?.textContent).toContain('(Table salt)')
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'none')
    expect(fetchMock).toHaveBeenCalledWith('/api/lab/scene', { credentials: 'include' })
    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/dissolve', expect.anything())
    expect(screen.queryByRole('status')).not.toBeInTheDocument()
  })

  it('wraps the stock carousel NaCl → CaCl2 → SiO2 and hides other stocks from the DOM', async () => {
    vi.stubGlobal('fetch', stubLabFetch())

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Sodium chloride (NaCl)' })

    clickCarousel('next')
    expect(screen.getByRole('button', { name: 'Calcium chloride (CaCl2)' })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Sodium chloride (NaCl)' })).not.toBeInTheDocument()
    expect(document.querySelector('[data-stock-solid="nacl"]')).toBeNull()
    const cacl2Label = document.querySelector('[data-stock-label="cacl2"]')
    expect(cacl2Label?.querySelector('sub')?.textContent).toBe('2')
    expect(cacl2Label?.textContent).toContain('(Calcium chloride)')
    expect(cacl2Label?.textContent).toContain('(De-icing salt)')
    expect(document.querySelector('[data-stock-solid="cacl2"]')).toHaveAttribute('data-stock-fill', '1.00')

    clickCarousel('next')
    expect(screen.getByRole('button', { name: 'Sand' })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Calcium chloride (CaCl2)' })).not.toBeInTheDocument()
    const sandLabel = document.querySelector('[data-stock-label="sand"]')
    expect(sandLabel?.querySelector('sub')?.textContent).toBe('2')
    expect(sandLabel?.textContent).toContain('(Silicon dioxide)')
    expect(sandLabel?.textContent).toContain('(Sand)')

    clickCarousel('next')
    expect(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Sand' })).not.toBeInTheDocument()

    clickCarousel('previous')
    expect(screen.getByRole('button', { name: 'Sand' })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Sodium chloride (NaCl)' })).not.toBeInTheDocument()
  })

  it('does not scoop or inspect when clicking carousel arrows', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })

    clickSpoon()
    clickCarousel('next')

    expect(screen.getByRole('button', { name: 'Calcium chloride (CaCl2)' })).toBeInTheDocument()
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'spoon')
    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/action', expect.anything())
  })

  it('returns the stock carousel to NaCl after Reset', async () => {
    vi.stubGlobal('fetch', stubLabFetch())

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Sodium chloride (NaCl)' })

    clickCarousel('next')
    expect(screen.getByRole('button', { name: 'Calcium chloride (CaCl2)' })).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: 'Reset lab' }))
    await waitFor(() => {
      expect(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' })).toBeInTheDocument()
    })
    expect(screen.queryByRole('button', { name: 'Calcium chloride (CaCl2)' })).not.toBeInTheDocument()
  })

  it('wraps the tool carousel pipette → spoon → tongs and hides the other tools from the DOM', async () => {
    vi.stubGlobal('fetch', stubLabFetch())

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })
    expect(screen.queryByRole('button', { name: 'Spoon' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Tongs' })).not.toBeInTheDocument()

    clickToolCarousel('next')
    expect(screen.getByRole('button', { name: 'Spoon' })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Pipette' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Tongs' })).not.toBeInTheDocument()

    clickToolCarousel('next')
    expect(screen.getByRole('button', { name: 'Tongs' })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Spoon' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Pipette' })).not.toBeInTheDocument()

    clickToolCarousel('next')
    expect(screen.getByRole('button', { name: 'Pipette' })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Tongs' })).not.toBeInTheDocument()

    clickToolCarousel('previous')
    expect(screen.getByRole('button', { name: 'Tongs' })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Pipette' })).not.toBeInTheDocument()
  })

  it('does not pick up or put away a tool when clicking tool carousel arrows', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })
    fireEvent.click(screen.getByRole('button', { name: 'Pipette' }))
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'pipette')

    clickToolCarousel('next')
    expect(screen.getByRole('button', { name: 'Spoon' })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Pipette' })).not.toBeInTheDocument()
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'pipette')
    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/action', expect.anything())

    clickToolCarousel('next')
    expect(screen.getByRole('button', { name: 'Tongs' })).toBeInTheDocument()
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'pipette')
    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/action', expect.anything())

    clickToolCarousel('next')
    expect(screen.getByRole('button', { name: 'Pipette' })).toHaveAttribute('aria-pressed', 'true')
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'pipette')
    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/action', expect.anything())
  })

  it('returns the tool carousel to pipette after Reset', async () => {
    vi.stubGlobal('fetch', stubLabFetch())

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })

    clickToolCarousel('next')
    expect(screen.getByRole('button', { name: 'Spoon' })).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: 'Reset lab' }))
    await waitFor(() => {
      expect(screen.getByRole('button', { name: 'Pipette' })).toBeInTheDocument()
    })
    expect(screen.queryByRole('button', { name: 'Spoon' })).not.toBeInTheDocument()
  })

  it('renders when server omits empty holding/composition/last_events (serde skip)', async () => {
    // Wire JSON from skip_serializing_if = Vec::is_empty — no holding/composition/last_events keys.
    const wireScene = {
      lab_id: 'lab-1',
      version: 0,
      temperature_c: 20,
      items: [
        {
          id: 'spoon-1',
          kind: 'spoon',
          label: 'Spoon',
          location: 'bench',
          properties: {},
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
          },
        },
        {
          id: 'beaker-water',
          kind: 'beaker',
          label: 'Water',
          location: 'bench',
          properties: {
            volume_ml: 250,
            fill_ml: 200,
            transparent: true,
            colourless: true,
            temperature_c: 20,
            composition: [
              { substance_id: 'water', phase: 'liquid', amount_ml: 200, amount_scoop: null, amount_g: null, amount_mol: null},
            ],
          },
        },
      ],
    }
    expect(JSON.stringify(wireScene)).not.toContain('"holding"')
    expect(JSON.stringify(wireScene)).not.toContain('"last_events"')

    vi.stubGlobal(
      'fetch',
      stubLabFetch({ scene: wireScene as LabScene }),
    )

    expect(() => render(<LabBench />)).not.toThrow()
    expect(await screen.findByRole('button', { name: 'Pipette' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Water beaker' })).toBeInTheDocument()
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'none')
    expect(screen.queryByRole('status')).not.toBeInTheDocument()
    expect(screen.queryByRole('alert')).not.toBeInTheDocument()
  })

  it('clicking the spoon holds it as the cursor tool', async () => {
    vi.stubGlobal('fetch', stubLabFetch())

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })

    clickSpoon()

    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'spoon')
    expect(screen.getByRole('button', { name: 'Spoon' })).toHaveAttribute('aria-pressed', 'true')
  })

  it('does not scoop or post an action without a held spoon', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })

    fireEvent.click(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' }))
    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))

    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'none')
    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/action', expect.anything())
  })

  it('idle water click shows beaker contents and temperature from the scene', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Water beaker' })

    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))

    const panel = await screen.findByRole('dialog', { name: 'Contents of Water' })
    expect(panel).toHaveTextContent('H2O (l)')
    expect(panel).toHaveTextContent('200.00 ml')
    expect(panel).toHaveTextContent('Temperature: 20.00°C')
    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/action', expect.anything())
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'none')
  })

  it('idle salt inspect shows server stock mass', async () => {
    vi.stubGlobal('fetch', stubLabFetch())

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Sodium chloride (NaCl)' })

    fireEvent.click(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' }))

    const panel = await screen.findByRole('dialog', { name: 'Contents of Sodium chloride' })
    expect(panel).toHaveTextContent('NaCl (s)')
    expect(panel).toHaveTextContent(`${STOCK_FULL_MASS_G.toFixed(2)} g`)
  })

  it('after scoop, salt inspect shows depleted server stock mass', async () => {
    const scooped = withScoop(initialScene(), 'nacl')
    const fetchMock = stubLabFetch({ scene: scooped })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Sodium chloride (NaCl)' })

    fireEvent.click(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' }))

    const panel = await screen.findByRole('dialog', { name: 'Contents of Sodium chloride' })
    expect(panel).toHaveTextContent('NaCl (s)')
    expect(panel).toHaveTextContent('1.80 g')
  })

  it('idle inspect after dissolve shows aqueous ions from the server composition', async () => {
    const dissolved = afterNaclPour(withScoop(initialScene(), 'nacl'))
    dissolved.items.find((item) => item.id === 'spoon-1')!.properties.holding = []
    const fetchMock = stubLabFetch({ scene: dissolved })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Water beaker' })

    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))

    const panel = await screen.findByRole('dialog', { name: 'Contents of Water' })
    expect(panel).toHaveTextContent('H2O (l)')
    expect(panel).toHaveTextContent('Na+ (aq)')
    expect(panel).toHaveTextContent('Cl− (aq)')
    expect(panel).toHaveTextContent('0.0171 M')
    expect(panel.querySelectorAll('sup')).toHaveLength(2)
    expect(panel).toHaveTextContent('Temperature: 19.98°C')
    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/action', expect.anything())
  })

  it('idle inspect after sand pour shows aggregated SiO2 mass not molarity', async () => {
    const leftover = afterSandPour(withScoop(initialScene(), 'sand'))
    leftover.items.find((item) => item.id === 'spoon-1')!.properties.holding = []
    const fetchMock = stubLabFetch({ scene: leftover })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Water beaker' })

    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))

    const panel = await screen.findByRole('dialog', { name: 'Contents of Water' })
    expect(panel).toHaveTextContent('SiO2 (s)')
    expect(panel).toHaveTextContent(`${SPOON_SCOOP_MASS_G.toFixed(2)} g`)
    expect(panel.querySelector('sub')?.textContent).toBe('2')
    expect(panel).not.toHaveTextContent(' M')
    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/action', expect.anything())
  })

  it('with spoon selected, water click does not open inspect', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })

    clickSpoon()
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'spoon')

    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))

    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/action', expect.anything())
  })

  it('inspect stays open across scoop and refreshes stock mass', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Sodium chloride (NaCl)' })

    fireEvent.click(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' }))
    const panel = await screen.findByRole('dialog', { name: 'Contents of Sodium chloride' })
    expect(panel).toHaveTextContent(`${STOCK_FULL_MASS_G.toFixed(2)} g`)

    clickSpoon()
    expect(screen.getByRole('dialog', { name: 'Contents of Sodium chloride' })).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' }))
    await waitFor(() => {
      expect(screen.getByRole('dialog', { name: 'Contents of Sodium chloride' })).toHaveTextContent(
        `${(STOCK_FULL_MASS_G - SPOON_SCOOP_MASS_G).toFixed(2)} g`,
      )
    })
    expect(fetchMock).toHaveBeenCalledWith('/api/lab/action', expect.anything())
  })

  it('inspect stays open across pour/dissolve and refreshes composition and temperature', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Water beaker' })

    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))
    const panel = await screen.findByRole('dialog', { name: 'Contents of Water' })
    expect(panel).toHaveTextContent('H2O (l)')
    expect(panel).toHaveTextContent('Temperature: 20.00°C')
    expect(panel).not.toHaveTextContent('Na+')

    clickSpoon()
    fireEvent.click(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' }))
    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'nacl')
    })
    expect(screen.getByRole('dialog', { name: 'Contents of Water' })).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))
    await waitFor(() => {
      const open = screen.getByRole('dialog', { name: 'Contents of Water' })
      expect(open).toHaveTextContent('Na+ (aq)')
      expect(open).toHaveTextContent('Cl− (aq)')
      expect(open).toHaveTextContent('0.0171 M')
      expect(open).toHaveTextContent('Temperature: 19.98°C')
    })
  })

  it('inspect stays open across put-back and only Close dismisses it', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Sodium chloride (NaCl)' })

    fireEvent.click(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' }))
    expect(await screen.findByRole('dialog', { name: 'Contents of Sodium chloride' })).toHaveTextContent(
      `${STOCK_FULL_MASS_G.toFixed(2)} g`,
    )

    clickSpoon()
    fireEvent.click(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' }))
    await waitFor(() => {
      expect(screen.getByRole('dialog')).toHaveTextContent(
        `${(STOCK_FULL_MASS_G - SPOON_SCOOP_MASS_G).toFixed(2)} g`,
      )
    })

    fireEvent.click(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' }))
    await waitFor(() => {
      expect(screen.getByRole('dialog')).toHaveTextContent(`${STOCK_FULL_MASS_G.toFixed(2)} g`)
    })
    expect(screen.getByRole('status')).toHaveTextContent('Returned nacl.')

    fireEvent.click(screen.getByRole('button', { name: 'Close' }))
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
  })


  it('shows full stock fill from server amount_g and lowers after scoop', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })

    expect(document.querySelector('[data-stock-solid="nacl"]')).toHaveAttribute('data-stock-fill', '1.00')
    expect(stockFillRatio(STOCK_FULL_MASS_G)).toBe(1)
    expect(stockFillRatio(STOCK_FULL_MASS_G - SPOON_SCOOP_MASS_G)).toBeCloseTo(0.9)

    showStockInCarousel('Sand')
    expect(document.querySelector('[data-stock-solid="sand"]')).toHaveAttribute('data-stock-fill', '1.00')
    showStockInCarousel('Sodium chloride (NaCl)')

    clickSpoon()
    fireEvent.click(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' }))

    await waitFor(() => {
      expect(document.querySelector('[data-stock-solid="nacl"]')).toHaveAttribute('data-stock-fill', '0.90')
    })
    showStockInCarousel('Sand')
    expect(document.querySelector('[data-stock-solid="sand"]')).toHaveAttribute('data-stock-fill', '1.00')
  })

  it('shows water fill from server amount_ml and updates after pour and reset', async () => {
    const fetchMock = stubLabFetch({
      actionHandler: (action, scene) => {
        if (action.type === 'use_tool' && action.target_item_id === 'beaker-nacl') {
          return withScoop(scene, 'nacl')
        }
        if (action.type === 'pour') {
          const next = afterNaclPour(scene)
          const water = next.items.find((item) => item.id === 'beaker-water')!
          const liquid = optionalArray(water.properties.composition).find(
            (entry) => entry.substance_id === 'water' && entry.phase === 'liquid',
          )
          // Exercise fill tracking when the server reports a different volume after pour.
          if (liquid) liquid.amount_ml = WATER_FULL_ML / 2
          return next
        }
        if (action.type === 'reset') {
          const next = initialScene()
          next.lab_id = scene.lab_id
          next.version = scene.version + 1
          next.last_events = [{ kind: 'reset', message: 'Lab reset to the starting bench.' }]
          return next
        }
        return { error: 'invalid action', code: 'invalid_action', status: 400 }
      },
    })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })

    expect(document.querySelector('[data-water-fill]')).toHaveAttribute('data-water-fill', '1.00')
    expect(waterFillRatio(WATER_FULL_ML)).toBe(1)
    expect(waterFillRatio(WATER_FULL_ML / 2)).toBeCloseTo(0.5)
    expect(waterFillRatio(0)).toBe(0)
    expect(waterFillRatio(null)).toBe(0)
    expect(dishFillRatio(DISH_CAPACITY_ML)).toBe(1)
    expect(dishFillRatio(DISH_CAPACITY_ML + 10)).toBe(1)
    expect(dishFillRatio(1)).toBeCloseTo(0.04)
    expect(dishFillRatio(0)).toBe(0)
    expect(dishFillRatio(null)).toBe(0)

    clickSpoon()
    fireEvent.click(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' }))
    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'nacl')
    })
    // Scoop does not change water volume — fill stays full from server amount_ml.
    expect(document.querySelector('[data-water-fill]')).toHaveAttribute('data-water-fill', '1.00')

    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))
    await waitFor(() => {
      expect(document.querySelector('[data-water-fill]')).toHaveAttribute('data-water-fill', '0.50')
    })
    expect(screen.getByRole('status')).toHaveTextContent(NACL_EXPLANATION)

    fireEvent.click(screen.getByRole('button', { name: 'Reset lab' }))
    await waitFor(() => {
      expect(document.querySelector('[data-water-fill]')).toHaveAttribute('data-water-fill', '1.00')
    })
  })

  it('renders half-full water when the initial scene reports half amount_ml', async () => {
    const scene = initialScene()
    const water = scene.items.find((item) => item.id === 'beaker-water')!
    const liquid = optionalArray(water.properties.composition).find(
      (entry) => entry.substance_id === 'water' && entry.phase === 'liquid',
    )!
    liquid.amount_ml = WATER_FULL_ML / 2
    vi.stubGlobal('fetch', stubLabFetch({ scene }))

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Water beaker' })

    expect(document.querySelector('[data-water-fill]')).toHaveAttribute('data-water-fill', '0.50')
  })

  it('puts salt back into the salt stock and restores fill from the server scene', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })

    clickSpoon()
    fireEvent.click(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' }))
    await waitFor(() => {
      expect(document.querySelector('[data-stock-solid="nacl"]')).toHaveAttribute('data-stock-fill', '0.90')
    })

    fireEvent.click(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' }))
    await waitFor(() => {
      expect(document.querySelector('[data-stock-solid="nacl"]')).toHaveAttribute('data-stock-fill', '1.00')
    })
    expect(screen.getByRole('status')).toHaveTextContent('Returned')
    // Spoon holding cleared — cursor tool stays spoon without solid fill.
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'spoon')
  })

  it('rejects putting salt into the sand stock beaker', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })

    clickSpoon()
    fireEvent.click(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' }))
    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'nacl')
    })

    showStockInCarousel('Sand')
    fireEvent.click(screen.getByRole('button', { name: 'Sand' }))
    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument()
    })
    expect(document.querySelector('[data-stock-solid="sand"]')).toHaveAttribute('data-stock-fill', '1.00')
    showStockInCarousel('Sodium chloride (NaCl)')
    expect(document.querySelector('[data-stock-solid="nacl"]')).toHaveAttribute('data-stock-fill', '0.90')
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'nacl')
  })

  it('spoon then nacl then water posts use_tool then pour with CSRF and shows server events', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })

    clickSpoon()
    fireEvent.click(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' }))

    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'nacl')
    })
    expect(lastActionInit(fetchMock)).toEqual(
      expect.objectContaining({
        method: 'POST',
        credentials: 'include',
        headers: expect.objectContaining({
          'content-type': 'application/json',
          'X-CSRF-Token': 'tok-123',
        }),
        body: JSON.stringify({
          type: 'use_tool',
          tool_item_id: 'spoon-1',
          target_item_id: 'beaker-nacl',
        }),
      }),
    )

    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))

    await waitFor(() => {
      expect(screen.getByRole('status')).toHaveTextContent(NACL_EXPLANATION)
    })
    expect(screen.getByRole('status')).toHaveTextContent('Dissolved')
    expect(document.querySelector('[data-bench-status="dissolved"]')).toHaveTextContent(NACL_EXPLANATION)
    expect(document.querySelector('[data-dissolve-cue="dissolved"]')).toBeTruthy()
    expect(document.querySelector('[data-water-aqueous="true"]')).toBeTruthy()
    expect(lastActionInit(fetchMock)).toEqual(
      expect.objectContaining({
        method: 'POST',
        credentials: 'include',
        headers: expect.objectContaining({
          'content-type': 'application/json',
          'X-CSRF-Token': 'tok-123',
        }),
        body: JSON.stringify({
          type: 'pour',
          source_item_id: 'spoon-1',
          target_item_id: 'beaker-water',
        }),
      }),
    )
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'spoon')
    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/dissolve', expect.anything())
  })

  it('spoon then cacl2 then water dissolves with server ions, heating, and molarity on inspect', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })

    showStockInCarousel('Calcium chloride (CaCl2)')
    expect(document.querySelector('[data-stock-solid="cacl2"]')).toHaveAttribute('data-stock-fill', '1.00')

    clickSpoon()
    fireEvent.click(screen.getByRole('button', { name: 'Calcium chloride (CaCl2)' }))
    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'cacl2')
    })
    expect(document.querySelector('[data-stock-solid="cacl2"]')).toHaveAttribute('data-stock-fill', '0.90')

    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))
    await waitFor(() => {
      expect(screen.getByRole('status')).toHaveTextContent(CACL2_EXPLANATION)
    })
    expect(screen.getByRole('status')).toHaveTextContent('Dissolved')
    expect(document.querySelector('[data-water-aqueous="true"]')).toBeTruthy()

    clickSpoon()
    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))
    const panel = await screen.findByRole('dialog', { name: 'Contents of Water' })
    expect(panel).toHaveTextContent('Ca')
    expect(panel).toHaveTextContent('2+')
    expect(panel).toHaveTextContent('(aq)')
    const moles = SPOON_SCOOP_MASS_G / CACL2_MOLAR_MASS_G_PER_MOL
    const expectedCaM = moles / 0.2
    const expectedClM = (2 * moles) / 0.2
    expect(panel).toHaveTextContent(`${expectedCaM.toPrecision(3)} M`)
    expect(panel).toHaveTextContent('Cl')
    expect(panel).toHaveTextContent(`${expectedClM.toPrecision(3)} M`)
    const expectedT =
      20 -
      (moles * CACL2_DELTA_H_SOLUTION_J_PER_MOL) / (200 * WATER_SPECIFIC_HEAT_J_PER_G_K)
    expect(panel).toHaveTextContent(`Temperature: ${expectedT.toFixed(2)}°C`)
    expect(expectedT).toBeGreaterThan(20)
  })

  it('renders a surprising server sand event without client solubility branching', async () => {
    const fetchMock = stubLabFetch({
      actionHandler: (action, scene) => {
        if (action.type === 'use_tool') return withScoop(scene, 'sand')
        const next = cloneScene(scene)
        const spoon = next.items.find((item) => item.id === 'spoon-1')!
        spoon.properties.holding = []
        next.last_events = [
          { kind: 'dissolved', message: 'Unexpected server sentence for sand.' },
        ]
        next.version += 1
        return next
      },
    })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })

    showStockInCarousel('Sand')
    clickSpoon()
    fireEvent.click(screen.getByRole('button', { name: 'Sand' }))
    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'sand')
    })
    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))

    await waitFor(() => {
      expect(screen.getByRole('status')).toHaveTextContent('Unexpected server sentence for sand.')
    })
    expect(screen.getByRole('status')).toHaveTextContent('Dissolved')
    expect(document.querySelector('[data-bench-status="dissolved"]')).toHaveTextContent(
      'Unexpected server sentence for sand.',
    )
    expect(screen.queryByText(SAND_EXPLANATION)).not.toBeInTheDocument()
    // No undissolved solid in the surprising payload → no leftover grains invented.
    expect(screen.getByRole('button', { name: 'Water beaker' }).querySelectorAll('circle')).toHaveLength(0)
  })

  it('sand pour shows the server did-not-dissolve sentence and leftover grains from the scene', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })

    showStockInCarousel('Sand')
    clickSpoon()
    fireEvent.click(screen.getByRole('button', { name: 'Sand' }))
    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'sand')
    })
    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))

    await waitFor(() => {
      expect(screen.getByRole('status')).toHaveTextContent(SAND_EXPLANATION)
    })
    expect(screen.getByRole('status')).toHaveTextContent('Did not dissolve')
    expect(document.querySelector('[data-bench-status="did_not_dissolve"]')).toHaveTextContent(
      SAND_EXPLANATION,
    )
    expect(document.querySelector('[data-dissolve-cue="did_not_dissolve"]')).toBeTruthy()
    expect(document.querySelector('[data-water-aqueous="false"]')).toBeTruthy()
    // Leftover grains are SVG circles rendered only because the server put solid sand in water.
    expect(
      screen.getByRole('button', { name: 'Water beaker' }).querySelectorAll('circle').length,
    ).toBeGreaterThan(0)
  })

  it('empty spoon on water does not post a pour action', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })

    clickSpoon()
    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))

    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/action', expect.anything())
    expect(screen.queryByRole('status')).not.toBeInTheDocument()
  })

  it.each([
    ['nacl', 'Sodium chloride (NaCl)'] as const,
    ['sand', 'Sand'] as const,
    ['cacl2', 'Calcium chloride (CaCl2)'] as const,
  ])(
    'scoop %s then put spoon away restores stock and empties the spoon',
    async (substance, label) => {
      const fetchMock = stubLabFetch()
      vi.stubGlobal('fetch', fetchMock)

      render(<LabBench />)
      await screen.findByRole('button', { name: 'Pipette' })

      showStockInCarousel(label)
      clickSpoon()
      fireEvent.click(screen.getByRole('button', { name: label }))
      await waitFor(() => {
        expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute(
          'data-tool',
          substance,
        )
      })
      expect(document.querySelector(`[data-stock-solid="${substance}"]`)).toHaveAttribute(
        'data-stock-fill',
        '0.90',
      )

      clickSpoon()

      await waitFor(() => {
        expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'none')
      })
      expect(document.querySelector(`[data-stock-solid="${substance}"]`)).toHaveAttribute(
        'data-stock-fill',
        '1.00',
      )
      expect(screen.getByRole('status')).toHaveTextContent(`Returned ${substance}.`)
      expect(JSON.parse(String(lastActionInit(fetchMock)?.body))).toEqual({
        type: 'put_away',
        tool_item_id: 'spoon-1',
      })
    },
  )

  it('empty spoon put-away is a no-op with no server action', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })

    clickSpoon()
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'spoon')

    clickSpoon()
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'none')
    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/action', expect.anything())
  })

  it('shows the server action error and no outcome after a pour', async () => {
    vi.stubGlobal(
      'fetch',
      stubLabFetch({
        actionHandler: (action, scene) => {
          if (action.type === 'use_tool') return withScoop(scene, 'nacl')
          return { error: 'Login required', code: 'unauthenticated', status: 401 }
        },
      }),
    )

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })

    clickSpoon()
    fireEvent.click(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' }))
    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'nacl')
    })
    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))

    expect(await screen.findByRole('alert')).toHaveTextContent('Login required')
    expect(screen.queryByText(/Server outcome:/)).not.toBeInTheDocument()
    expect(screen.queryByText(NACL_EXPLANATION)).not.toBeInTheDocument()
  })

  it('shows a scene load error from the server', async () => {
    vi.stubGlobal(
      'fetch',
      stubLabFetch({
        sceneError: { error: 'Login required', code: 'unauthenticated', status: 401 },
      }),
    )

    render(<LabBench />)

    expect(await screen.findByRole('alert')).toHaveTextContent('Login required')
    expect(screen.queryByRole('button', { name: 'Spoon' })).not.toBeInTheDocument()
  })

  it('reset posts CSRF reset action and restores pure water from the server scene', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })

    clickSpoon()
    fireEvent.click(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' }))
    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'nacl')
    })
    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))
    await waitFor(() => {
      expect(screen.getByRole('status')).toHaveTextContent(NACL_EXPLANATION)
    })

    // Put the spoon away so idle inspect works, then confirm ions are present.
    clickSpoon()
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'none')
    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))
    expect(await screen.findByRole('dialog')).toHaveTextContent('Na+ (aq)')

    fireEvent.click(screen.getByRole('button', { name: 'Reset lab' }))

    await waitFor(() => {
      expect(screen.getByRole('status')).toHaveTextContent('Lab reset to the starting bench.')
    })
    expect(lastActionInit(fetchMock)).toEqual(
      expect.objectContaining({
        method: 'POST',
        credentials: 'include',
        headers: expect.objectContaining({
          'content-type': 'application/json',
          'X-CSRF-Token': 'tok-123',
        }),
        body: JSON.stringify({ type: 'reset' }),
      }),
    )
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'none')
    // Reset refreshes inspect in place; it does not auto-dismiss.
    const panel = screen.getByRole('dialog', { name: 'Contents of Water' })
    expect(panel).toHaveTextContent('H2O (l)')
    expect(panel).not.toHaveTextContent('Na+')
    expect(panel).not.toHaveTextContent('(aq)')
    expect(panel.querySelectorAll('sup')).toHaveLength(0)
  })

  it('pipette transfers 1 ml from the water beaker into the dish and back', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })

    fireEvent.click(screen.getByRole('button', { name: 'Pipette' }))
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'pipette')

    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'use_tool',
        tool_item_id: 'pipette-1',
        target_item_id: 'beaker-water',
      })
    })
    expect(document.querySelector('[data-water-fill]')).toHaveAttribute('data-water-fill', '0.99')

    fireEvent.click(screen.getByRole('button', { name: 'Evaporation dish' }))
    await waitFor(() => {
      expect(JSON.parse(String(lastActionInit(fetchMock)?.body))).toEqual({
        type: 'use_tool',
        tool_item_id: 'pipette-1',
        target_item_id: 'dish-1',
      })
    })
    expect(document.querySelector('[data-dish-fill]')).toHaveAttribute('data-dish-fill', '0.04')

    fireEvent.click(screen.getByRole('button', { name: 'Evaporation dish' }))
    await waitFor(() => {
      expect(JSON.parse(String(lastActionInit(fetchMock)?.body))).toEqual({
        type: 'use_tool',
        tool_item_id: 'pipette-1',
        target_item_id: 'dish-1',
      })
    })
    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))
    await waitFor(() => {
      expect(JSON.parse(String(lastActionInit(fetchMock)?.body))).toEqual({
        type: 'use_tool',
        tool_item_id: 'pipette-1',
        target_item_id: 'beaker-water',
      })
    })
    expect(document.querySelector('[data-dish-fill]')).toHaveAttribute('data-dish-fill', '0.00')
  })

  it('idle burner click toggles the burner after the dish has liquid', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Burner' })

    fireEvent.click(screen.getByRole('button', { name: 'Pipette' }))
    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))
    await waitFor(() => {
      expect(JSON.parse(String(lastActionInit(fetchMock)?.body)).type).toBe('use_tool')
    })
    fireEvent.click(screen.getByRole('button', { name: 'Evaporation dish' }))
    await waitFor(() => {
      expect(JSON.parse(String(lastActionInit(fetchMock)?.body)).target_item_id).toBe('dish-1')
    })
    fireEvent.click(screen.getByRole('button', { name: 'Pipette' }))
    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'none')
    })

    fireEvent.click(screen.getByRole('button', { name: 'Burner' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'toggle_burner',
        burner_item_id: 'burner-1',
      })
    })
    expect(screen.getByRole('button', { name: 'Burner' })).toHaveAttribute('aria-pressed', 'true')
    expect(document.querySelector('[data-burner-on]')).toHaveAttribute('data-burner-on', 'true')
  })

  it('does not toggle the burner while the pipette is selected', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })
    fireEvent.click(screen.getByRole('button', { name: 'Pipette' }))
    fireEvent.click(screen.getByRole('button', { name: 'Burner' }))

    expect(fetchMock).not.toHaveBeenCalledWith(
      '/api/lab/action',
      expect.objectContaining({
        body: JSON.stringify({ type: 'toggle_burner', burner_item_id: 'burner-1' }),
      }),
    )
  })

  it('idle dish click inspects contents including temperature', async () => {
    vi.stubGlobal('fetch', stubLabFetch())

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Evaporation dish' })
    fireEvent.click(screen.getByRole('button', { name: 'Evaporation dish' }))

    const panel = await screen.findByRole('dialog', { name: 'Contents of Evaporation dish' })
    expect(panel).toHaveTextContent('Temperature: 20.00°C')
  })

  it('polls the lab scene while the burner is on', async () => {
    vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval'] })
    const lit = initialScene()
    const dish = lit.items.find((item) => item.id === 'dish-1')!
    dish.properties.composition = [
      {
        substance_id: 'water',
        phase: 'liquid',
        amount_ml: 1,
        amount_scoop: null,
        amount_g: null,
        amount_mol: null,
      },
    ]
    dish.properties.fill_ml = 1
    lit.items.find((item) => item.id === 'burner-1')!.properties.on = true
    const fetchMock = stubLabFetch({ scene: lit })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Burner' })
    const getsBefore = fetchMock.mock.calls.filter(([url]) => String(url) === '/api/lab/scene').length

    await vi.advanceTimersByTimeAsync(900)
    const getsAfter = fetchMock.mock.calls.filter(([url]) => String(url) === '/api/lab/scene').length
    expect(getsAfter).toBeGreaterThan(getsBefore)
    vi.useRealTimers()
  })

  it('returns a filled pipette to the last source on put-away', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })
    fireEvent.click(screen.getByRole('button', { name: 'Pipette' }))
    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'use_tool',
        tool_item_id: 'pipette-1',
        target_item_id: 'beaker-water',
      })
    })
    expect(document.querySelector('[data-water-fill]')).toHaveAttribute('data-water-fill', '0.99')

    fireEvent.click(screen.getByRole('button', { name: 'Pipette' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'put_away',
        tool_item_id: 'pipette-1',
      })
    })
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'none')
    expect(document.querySelector('[data-water-fill]')).toHaveAttribute('data-water-fill', '1.00')
  })

  it('shows a pipette transfer error from the server', async () => {
    vi.stubGlobal(
      'fetch',
      stubLabFetch({
        actionHandler: () => ({ error: 'Dish is full', code: 'invalid_action', status: 400 }),
      }),
    )

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })
    fireEvent.click(screen.getByRole('button', { name: 'Pipette' }))
    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))

    expect(await screen.findByRole('alert')).toHaveTextContent('Dish is full')
  })

  it('shows a burner toggle error from the server', async () => {
    vi.stubGlobal(
      'fetch',
      stubLabFetch({
        actionHandler: (action) => {
          if (action.type === 'toggle_burner') {
            return { error: 'Burner jammed', code: 'invalid_action', status: 400 }
          }
          return { error: 'unexpected', code: 'invalid_action', status: 400 }
        },
      }),
    )

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Burner' })
    fireEvent.click(screen.getByRole('button', { name: 'Burner' }))

    expect(await screen.findByRole('alert')).toHaveTextContent('Burner jammed')
  })

  it('does not surface poll errors while the burner is on', async () => {
    vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval'] })
    const lit = initialScene()
    const dish = lit.items.find((item) => item.id === 'dish-1')!
    dish.properties.composition = [
      {
        substance_id: 'water',
        phase: 'liquid',
        amount_ml: 1,
        amount_scoop: null,
        amount_g: null,
        amount_mol: null,
      },
    ]
    dish.properties.fill_ml = 1
    lit.items.find((item) => item.id === 'burner-1')!.properties.on = true

    let sceneGets = 0
    const fetchMock = vi.fn(async (input: RequestInfo | URL) => {
      const url = String(input)
      if (url === '/api/auth/csrf') {
        return jsonResponse({ csrf_token: 'tok-123' })
      }
      if (url === '/api/lab/scene') {
        sceneGets += 1
        if (sceneGets > 1) {
          return jsonResponse({ error: 'temporary', code: 'unavailable' }, 503)
        }
        return jsonResponse(lit)
      }
      return jsonResponse({ error: 'unexpected', code: 'invalid_action' }, 400)
    })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Burner' })
    await vi.advanceTimersByTimeAsync(900)
    expect(sceneGets).toBeGreaterThan(1)
    expect(screen.queryByRole('alert')).not.toBeInTheDocument()
    vi.useRealTimers()
  })

  it('selecting the spoon puts an empty pipette away without a server call', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })
    fireEvent.click(screen.getByRole('button', { name: 'Pipette' }))
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'pipette')

    clickSpoon()
    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'spoon')
    })
    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/action', expect.anything())
  })

  it('keeps the dish stacked on the burner to the left of the water beaker', async () => {
    vi.stubGlobal('fetch', stubLabFetch())

    render(<LabBench />)
    const water = await screen.findByRole('button', { name: 'Water beaker' })
    const dish = screen.getByRole('button', { name: 'Evaporation dish' })
    const burner = screen.getByRole('button', { name: 'Burner' })
    const stack = dish.closest('.lab-evap-stack')

    expect(stack).not.toBeNull()
    expect(stack).toContainElement(burner)
    expect(precedesInDocument(dish, burner)).toBe(true)
    expect(precedesInDocument(stack as HTMLElement, water)).toBe(true)
  })

  it('places the tool carousel after the stock slot with flanking arrows', async () => {
    vi.stubGlobal('fetch', stubLabFetch())

    render(<LabBench />)
    const pipette = await screen.findByRole('button', { name: 'Pipette' })
    const stock = screen.getByRole('button', { name: 'Sodium chloride (NaCl)' })
    const carousel = pipette.closest('.lab-tool-carousel')
    const previous = screen.getByRole('button', { name: 'Previous tool' })
    const next = screen.getByRole('button', { name: 'Next tool' })

    expect(carousel).not.toBeNull()
    expect(carousel).toContainElement(previous)
    expect(carousel).toContainElement(next)
    expect(precedesInDocument(previous, pipette)).toBe(true)
    expect(precedesInDocument(pipette, next)).toBe(true)
    expect(precedesInDocument(stock, carousel as HTMLElement)).toBe(true)
    expect(screen.queryByRole('button', { name: 'Spoon' })).not.toBeInTheDocument()
  })

  it('reserves a fixed-width tool slot so pipette, spoon, and tongs do not shift the bench', async () => {
    vi.stubGlobal('fetch', stubLabFetch())

    render(<LabBench />)
    const pipette = await screen.findByRole('button', { name: 'Pipette' })
    expect(pipette.parentElement).toHaveClass('lab-tool-carousel-slot')

    clickToolCarousel('next')
    expect(screen.getByRole('button', { name: 'Spoon' }).parentElement).toHaveClass(
      'lab-tool-carousel-slot',
    )
    clickToolCarousel('next')
    expect(screen.getByRole('button', { name: 'Tongs' }).parentElement).toHaveClass(
      'lab-tool-carousel-slot',
    )
  })

  it('picks up the water beaker with tongs, hides the bench vessel, and pours into the dish', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Water beaker' })
    clickTongs()
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'tongs')

    const water = screen.getByRole('button', { name: 'Water beaker' })
    fireEvent.mouseMove(screen.getByRole('region', { name: 'Lab bench' }), { clientX: 40, clientY: 40 })
    fireEvent.click(water)
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'use_tool',
        tool_item_id: 'tongs-1',
        target_item_id: 'beaker-water',
      })
    })
    expect(water.querySelector('[data-water-fill]')).toBeNull()
    expect(document.querySelector('.lab-cursor-vessel [data-water-fill]')).not.toBeNull()

    fireEvent.click(screen.getByRole('button', { name: 'Evaporation dish' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'use_tool',
        tool_item_id: 'tongs-1',
        target_item_id: 'dish-1',
      })
    })
    expect(document.querySelector('[data-dish-fill]')).toHaveAttribute('data-dish-fill', '1.00')
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'tongs')
  })

  it('picks up the dish with tongs, turns the burner off, and pours back into water', async () => {
    const lit = initialScene()
    const dish = lit.items.find((item) => item.id === 'dish-1')!
    dish.properties.composition = [
      {
        substance_id: 'water',
        phase: 'liquid',
        amount_ml: DISH_CAPACITY_ML,
        amount_scoop: null,
        amount_g: null,
        amount_mol: null,
      },
    ]
    dish.properties.fill_ml = DISH_CAPACITY_ML
    lit.items.find((item) => item.id === 'burner-1')!.properties.on = true
    const fetchMock = stubLabFetch({ scene: lit })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Burner' })
    expect(screen.getByRole('button', { name: 'Burner' })).toHaveAttribute('aria-pressed', 'true')

    clickTongs()
    fireEvent.click(screen.getByRole('button', { name: 'Evaporation dish' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'use_tool',
        tool_item_id: 'tongs-1',
        target_item_id: 'dish-1',
      })
    })
    expect(screen.getByRole('button', { name: 'Burner' })).toHaveAttribute('aria-pressed', 'false')
    expect(screen.getByRole('button', { name: 'Evaporation dish' }).querySelector('[data-dish-fill]')).toBeNull()
    expect(document.querySelector('.lab-cursor-vessel [data-dish-fill]')).not.toBeNull()

    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'use_tool',
        tool_item_id: 'tongs-1',
        target_item_id: 'beaker-water',
      })
    })
    expect(document.querySelector('[data-water-fill]')).toHaveAttribute('data-water-fill', '1.00')
  })

  it('put-away and clicking the empty water slot return the held beaker home', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Water beaker' })
    clickTongs()
    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'use_tool',
        tool_item_id: 'tongs-1',
        target_item_id: 'beaker-water',
      })
    })
    expect(screen.getByRole('button', { name: 'Water beaker' }).querySelector('[data-water-fill]')).toBeNull()

    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'put_away',
        tool_item_id: 'tongs-1',
      })
    })
    expect(screen.getByRole('button', { name: 'Water beaker' }).querySelector('[data-water-fill]')).not.toBeNull()
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'none')
  })

  it('does not use tongs on stock or the burner', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Sodium chloride (NaCl)' })
    clickTongs()
    fireEvent.click(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' }))
    fireEvent.click(screen.getByRole('button', { name: 'Burner' }))
    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/action', expect.anything())
  })

  it('clicking the empty dish slot or the tongs put-away returns the held dish home', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Evaporation dish' })
    clickTongs()
    fireEvent.click(screen.getByRole('button', { name: 'Evaporation dish' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'use_tool',
        tool_item_id: 'tongs-1',
        target_item_id: 'dish-1',
      })
    })

    fireEvent.click(screen.getByRole('button', { name: 'Evaporation dish' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'put_away',
        tool_item_id: 'tongs-1',
      })
    })
    expect(screen.getByRole('button', { name: 'Evaporation dish' }).querySelector('[data-dish-fill]')).not.toBeNull()
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'none')
  })

  it('switching tools puts tongs away first, including a held vessel', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Water beaker' })
    clickTongs()
    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'use_tool',
        tool_item_id: 'tongs-1',
        target_item_id: 'beaker-water',
      })
    })

    clickSpoon()
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'put_away',
        tool_item_id: 'tongs-1',
      })
    })
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'spoon')
    expect(screen.getByRole('button', { name: 'Water beaker' }).querySelector('[data-water-fill]')).not.toBeNull()
  })

  it('selecting the pipette puts empty tongs away without a server call', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })
    clickTongs()
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'tongs')

    showToolInCarousel('Pipette')
    fireEvent.click(screen.getByRole('button', { name: 'Pipette' }))
    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'pipette')
    })
    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/action', expect.anything())
  })

  it('picks up water with tongs when the loaded scene predates tongs-1', async () => {
    const stale = initialScene()
    stale.items = stale.items.filter((item) => item.id !== 'tongs-1')
    const fetchMock = stubLabFetch({
      scene: stale,
      actionHandler: (action, scene) => {
        if (action.type !== 'use_tool' || action.tool_item_id !== 'tongs-1') {
          return { error: 'unexpected', code: 'invalid_action', status: 400 }
        }
        const next = cloneScene(scene)
        if (!next.items.some((item) => item.id === 'tongs-1')) {
          next.items.push({
            id: 'tongs-1',
            kind: 'tongs',
            label: 'Tongs',
            location: 'bench',
            properties: { ...emptyProps(), source_item_id: null },
          })
        }
        return applyTongsPickUp(next, action.target_item_id)
      },
    })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Water beaker' })
    clickTongs()
    fireEvent.mouseMove(screen.getByRole('region', { name: 'Lab bench' }), { clientX: 40, clientY: 40 })
    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'use_tool',
        tool_item_id: 'tongs-1',
        target_item_id: 'beaker-water',
      })
    })
    expect(screen.getByRole('button', { name: 'Water beaker' }).querySelector('[data-water-fill]')).toBeNull()
    expect(document.querySelector('.lab-cursor-vessel [data-water-fill]')).not.toBeNull()
  })

  it('shows a tongs pour error from the server and stays holding', async () => {
    vi.stubGlobal(
      'fetch',
      stubLabFetch({
        actionHandler: (action, scene) => {
          if (action.type === 'use_tool' && action.tool_item_id === 'tongs-1') {
            const held = scene.items.find((item) => item.id === 'tongs-1')?.properties.source_item_id
            if (!held) return applyTongsPickUp(scene, action.target_item_id)
            return { error: 'Nothing to pour', code: 'empty_holding', status: 400 }
          }
          return { error: 'unexpected', code: 'invalid_action', status: 400 }
        },
      }),
    )

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Water beaker' })
    clickTongs()
    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))
    await screen.findByRole('status')
    fireEvent.click(screen.getByRole('button', { name: 'Evaporation dish' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('Nothing to pour')
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'tongs')
  })

  it('empty spoon on a dry solids dish posts use_tool and holds the scoop', async () => {
    const fetchMock = stubLabFetch({
      scene: withDryDishSolids(initialScene(), [
        { substance_id: 'nacl', amount_g: 0.6 },
        { substance_id: 'cacl2', amount_g: 0.4 },
      ]),
    })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Evaporation dish' })

    clickSpoon()
    fireEvent.click(screen.getByRole('button', { name: 'Evaporation dish' }))

    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'use_tool',
        tool_item_id: 'spoon-1',
        target_item_id: 'dish-1',
      })
    })
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'nacl')
    expect(screen.getByRole('status')).toHaveTextContent('Scooped solids from the dish.')
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
  })

  it('water click dumps dish solids with pour and CSRF', async () => {
    const fetchMock = stubLabFetch({
      scene: withDryDishSolids(initialScene(), [{ substance_id: 'nacl', amount_g: 0.5 }]),
    })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Evaporation dish' })

    clickSpoon()
    fireEvent.click(screen.getByRole('button', { name: 'Evaporation dish' }))
    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'nacl')
    })

    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'pour',
        source_item_id: 'spoon-1',
        target_item_id: 'beaker-water',
      })
    })
    expect(screen.getByRole('status')).toHaveTextContent(NACL_EXPLANATION)
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'spoon')
  })

  it('matching stock is a target for a single-species dish scoop', async () => {
    const fetchMock = stubLabFetch({
      scene: withDryDishSolids(initialScene(), [{ substance_id: 'nacl', amount_g: 0.5 }]),
    })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Evaporation dish' })

    clickSpoon()
    fireEvent.click(screen.getByRole('button', { name: 'Evaporation dish' }))
    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'nacl')
    })

    fireEvent.click(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'use_tool',
        tool_item_id: 'spoon-1',
        target_item_id: 'beaker-nacl',
      })
    })
    expect(screen.getByRole('status')).toHaveTextContent('Returned')
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'spoon')
  })

  it('mixed dish scoop does not post to a stock', async () => {
    const fetchMock = stubLabFetch({
      scene: withDryDishSolids(initialScene(), [
        { substance_id: 'nacl', amount_g: 0.6 },
        { substance_id: 'cacl2', amount_g: 0.4 },
      ]),
    })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Evaporation dish' })

    clickSpoon()
    fireEvent.click(screen.getByRole('button', { name: 'Evaporation dish' }))
    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'nacl')
    })
    const callsAfterScoop = fetchMock.mock.calls.filter(([url]) => String(url) === '/api/lab/action').length

    fireEvent.click(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' }))
    expect(fetchMock.mock.calls.filter(([url]) => String(url) === '/api/lab/action')).toHaveLength(
      callsAfterScoop,
    )
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'nacl')
  })

  it('water is a pour target when holding mixed dish solids', async () => {
    const fetchMock = stubLabFetch({
      scene: withDryDishSolids(initialScene(), [
        { substance_id: 'nacl', amount_g: 0.6 },
        { substance_id: 'cacl2', amount_g: 0.4 },
      ]),
    })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Evaporation dish' })

    clickSpoon()
    fireEvent.click(screen.getByRole('button', { name: 'Evaporation dish' }))
    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'nacl')
    })

    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'pour',
        source_item_id: 'spoon-1',
        target_item_id: 'beaker-water',
      })
    })
  })

  it('put-away returns a dish scoop to the dish', async () => {
    const fetchMock = stubLabFetch({
      scene: withDryDishSolids(initialScene(), [
        { substance_id: 'nacl', amount_g: 0.6 },
        { substance_id: 'cacl2', amount_g: 0.4 },
      ]),
    })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Evaporation dish' })

    clickSpoon()
    fireEvent.click(screen.getByRole('button', { name: 'Evaporation dish' }))
    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'nacl')
    })

    clickSpoon()
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'put_away',
        tool_item_id: 'spoon-1',
      })
    })
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'none')
    expect(screen.getByRole('status')).toHaveTextContent('Returned solids to the dish.')
    expect(document.querySelector('[data-stock-solid="nacl"]')).toHaveAttribute('data-stock-fill', '1.00')
  })
})
