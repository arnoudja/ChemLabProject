import type { LabScene } from '../generated/contracts'
import {
  DISH_CAPACITY_ML,
  DISTILLED_WATER_CAPACITY_ML,
  FILTRATE_CAPACITY_ML,
  PIPETTE_MIN_SOURCE_ML,
  PIPETTE_VOLUME_ML,
} from './LabBench'
import { optionalArray } from '../lib/scene'
import { cloneScene } from './labBenchTestFixtures'

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
  // Phase-split only for FE layout tests. Wash dissolve of paper salts into the
  // filtrate fluid is owned by chemlab-core `apply_filter_pour` (partial / τ∝V).
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

export function applyPipetteFill(
  scene: LabScene,
  sourceId: string,
): LabScene | { error: string; code: string; status: number } {
  const next = cloneScene(scene)
  const source = next.items.find((item) => item.id === sourceId)!
  const pipette = next.items.find((item) => item.id === 'pipette-1')!
  const water = liquidWaterEntry(source)
  const availableMl = water?.amount_ml ?? 0
  if (!water || availableMl <= 0) {
    return { error: 'No fluid available.', code: 'no_fluid_available', status: 400 }
  }
  if (availableMl < PIPETTE_MIN_SOURCE_ML) {
    return {
      error: 'Not enough fluid available.',
      code: 'not_enough_fluid_available',
      status: 400,
    }
  }
  water.amount_ml = availableMl - PIPETTE_VOLUME_ML
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

export function applyPipetteUse(
  scene: LabScene,
  targetId: string,
): LabScene | { error: string; code: string; status: number } {
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
