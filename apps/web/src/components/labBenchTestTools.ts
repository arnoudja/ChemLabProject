import type { CompositionEntry, LabScene } from '../generated/contracts'
import {
  DISH_CAPACITY_ML,
  DISTILLED_WATER_CAPACITY_ML,
  HCL_STOCK_CAPACITY_ML,
  FILTRATE_CAPACITY_ML,
  PIPETTE_MIN_SOURCE_ML,
  PIPETTE_VOLUME_ML,
} from './LabBench'
import { optionalArray } from '../lib/scene'
import { solutionVolumeMl } from './labBenchScene'
import { cloneScene } from './labBenchTestFixtures'

export function liquidWaterEntry(item: LabScene['items'][number]) {
  return optionalArray(item.properties.composition).find(
    (entry) => entry.substance_id === 'water' && entry.phase === 'liquid',
  )
}

/** Φ_V solution volume for room / fill_ml (matches server `transfer_volume_ml`). */
function solutionMlOf(item: LabScene['items'][number]): number {
  return solutionVolumeMl(optionalArray(item.properties.composition))
}

function syncFillMlFromPhiV(item: LabScene['items'][number]) {
  if (item.kind === 'beaker' || item.kind === 'evaporation_dish' || item.kind === 'pipette') {
    item.properties.fill_ml = solutionMlOf(item)
  }
}

/**
 * Scale non-solid composition by `frac` (Φ_V transfer). Solids stay on the source
 * unless the caller moves them (filter paper path).
 */
function takeFluidFraction(
  source: LabScene['items'][number],
  frac: number,
): CompositionEntry[] {
  const taken: CompositionEntry[] = []
  const remaining: CompositionEntry[] = []
  for (const entry of optionalArray(source.properties.composition)) {
    if (entry.phase === 'solid') {
      remaining.push(entry)
      continue
    }
    if (entry.substance_id === 'water' && entry.phase === 'liquid') {
      const solventWaterMl = entry.amount_ml ?? 0
      const move = solventWaterMl * frac
      const rest = solventWaterMl - move
      if (move > 1e-12) {
        taken.push({ ...entry, amount_ml: move })
      }
      if (rest > 1e-12) {
        remaining.push({ ...entry, amount_ml: rest })
      }
      continue
    }
    if (entry.phase === 'aqueous') {
      const mol = entry.amount_mol ?? 0
      const move = mol * frac
      const rest = mol - move
      if (move > 1e-12) {
        taken.push({ ...entry, amount_mol: move })
      }
      if (rest > 1e-12) {
        remaining.push({ ...entry, amount_mol: rest })
      }
      continue
    }
    remaining.push(entry)
  }
  source.properties.composition = remaining
  syncFillMlFromPhiV(source)
  return taken
}

function mergeFluidInto(dest: LabScene['items'][number], fluid: CompositionEntry[]) {
  const composition = [...optionalArray(dest.properties.composition)]
  for (const entry of fluid) {
    if (entry.substance_id === 'water' && entry.phase === 'liquid') {
      const existing = composition.find(
        (c) => c.substance_id === 'water' && c.phase === 'liquid',
      )
      if (existing) {
        existing.amount_ml = (existing.amount_ml ?? 0) + (entry.amount_ml ?? 0)
      } else {
        composition.push({ ...entry })
      }
      continue
    }
    if (entry.phase === 'aqueous') {
      const existing = composition.find(
        (c) => c.substance_id === entry.substance_id && c.phase === 'aqueous',
      )
      if (existing) {
        existing.amount_mol = (existing.amount_mol ?? 0) + (entry.amount_mol ?? 0)
      } else {
        composition.push({ ...entry })
      }
      continue
    }
    composition.push({ ...entry })
  }
  dest.properties.composition = composition
  syncFillMlFromPhiV(dest)
}

export function pipetteIsFull(scene: LabScene): boolean {
  const pipette = scene.items.find((item) => item.id === 'pipette-1')
  return solutionVolumeMl(optionalArray(pipette?.properties.holding)) > 0
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
  if (itemId === 'beaker-hcl') return HCL_STOCK_CAPACITY_ML
  if (itemId === 'beaker-h2so4') return 10
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
  const sourceSolutionMl = solutionMlOf(source)
  const destSolutionMl = solutionMlOf(dest)
  const room = Math.max(0, liquidCapacityMl(destId) - destSolutionMl)
  const solids = optionalArray(source.properties.composition).filter((entry) => entry.phase === 'solid')
  if (sourceSolutionMl <= 0) {
    if (solids.length === 0) {
      return { error: 'empty holding', code: 'empty_holding', status: 400 }
    }
    dest.properties.composition = [...optionalArray(dest.properties.composition), ...solids]
    source.properties.composition = optionalArray(source.properties.composition).filter(
      (entry) => entry.phase !== 'solid',
    )
    syncFillMlFromPhiV(source)
    syncFillMlFromPhiV(dest)
    next.last_events = [{ kind: 'poured', message: 'Poured solids into the vessel.' }]
    next.version += 1
    return next
  }
  if (room <= 0) {
    return { error: 'invalid action', code: 'invalid_action', status: 400 }
  }
  const transferred = Math.min(sourceSolutionMl, room)
  const frac = transferred / sourceSolutionMl
  const fluid = takeFluidFraction(source, frac)
  // Stub: move a matching solid fraction with the pour (layout tests only).
  for (const solid of solids) {
    const moved = (solid.amount_g ?? 0) * frac
    solid.amount_g = (solid.amount_g ?? 0) - moved
    if (moved > 1e-12) {
      fluid.push({ ...solid, amount_g: moved })
    }
  }
  source.properties.composition = optionalArray(source.properties.composition).filter(
    (entry) => entry.phase !== 'solid' || (entry.amount_g ?? 0) > 1e-12,
  )
  mergeFluidInto(dest, fluid)
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
  const sourceSolutionMl = solutionMlOf(source)
  const destSolutionMl = solutionMlOf(dest)
  const room = Math.max(0, FILTRATE_CAPACITY_ML - destSolutionMl)
  const solids = optionalArray(source.properties.composition).filter((entry) => entry.phase === 'solid')
  if (sourceSolutionMl <= 0) {
    if (solids.length === 0) {
      return { error: 'empty holding', code: 'empty_holding', status: 400 }
    }
    return { error: 'invalid action', code: 'invalid_action', status: 400 }
  }
  if (room <= 0) {
    return { error: 'invalid action', code: 'invalid_action', status: 400 }
  }
  const transferred = Math.min(sourceSolutionMl, room)
  const frac = transferred / sourceSolutionMl
  const fluid = takeFluidFraction(source, frac)
  mergeFluidInto(dest, fluid)
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
    const sourceSolutionMl = solutionMlOf(source!)
    const solids = optionalArray(source?.properties.composition).filter((entry) => entry.phase === 'solid')
    if (sourceSolutionMl <= 0 && solids.length > 0) {
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
  const availableMl = solutionMlOf(source)
  if (availableMl <= 0) {
    return { error: 'No fluid available.', code: 'no_fluid_available', status: 400 }
  }
  if (availableMl < PIPETTE_MIN_SOURCE_ML) {
    return {
      error: 'Not enough fluid available.',
      code: 'not_enough_fluid_available',
      status: 400,
    }
  }
  const frac = PIPETTE_VOLUME_ML / availableMl
  const fluid = takeFluidFraction(source, frac)
  pipette.location = 'hand'
  pipette.properties.holding = fluid
  pipette.properties.source_item_id = sourceId
  pipette.properties.temperature_c = source.properties.temperature_c
  pipette.properties.fill_ml = solutionVolumeMl(fluid)
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
  const holding = optionalArray(pipette.properties.holding)
  mergeFluidInto(target, holding)
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
  const hasLiquid = solutionMlOf(dish) > 0
  const currentlyOn = burner.properties.on === true
  const nextOn = currentlyOn ? false : hasLiquid
  burner.properties.on = nextOn
  if (nextOn !== currentlyOn) {
    next.last_events = [{ kind: 'toggled', message: nextOn ? 'Burner on.' : 'Burner off.' }]
  }
  next.version += 1
  return next
}
