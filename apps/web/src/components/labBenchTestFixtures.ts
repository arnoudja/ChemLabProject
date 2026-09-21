import type { LabScene } from '../generated/contracts'
import { findChallenge, FREE_MODE } from '../lib/challenges'
import {
  STOCK_FULL_MASS_G,
  STOCK_FULL_SCOOPS,
  WATER_FULL_ML,
  DISTILLED_WATER_CAPACITY_ML,
  PIPETTE_VOLUME_ML,
  FILTRATE_CAPACITY_ML,
} from './LabBench'

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
    mode: FREE_MODE,
    challenge_completed: false,
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

export const SEPARATE_CHALLENGE = findChallenge('separate-nacl-sio2')!

/** Mirror of the server start scene for `separate-nacl-sio2`. */
export function challengeScene(): LabScene {
  const next = initialScene()
  next.mode = SEPARATE_CHALLENGE.id
  next.items = next.items.filter((item) => item.id !== 'beaker-cacl2')
  for (const stockId of ['beaker-nacl', 'beaker-sand']) {
    const stock = next.items.find((item) => item.id === stockId)!
    stock.properties.composition = stock.properties.composition!.map((entry) => ({
      ...entry,
      amount_scoop: 0,
      amount_g: 0,
    }))
  }
  const beaker = next.items.find((item) => item.id === 'beaker-water')!
  beaker.properties.composition = (['nacl', 'sand'] as const).map((substance_id) => ({
    substance_id,
    phase: 'solid' as const,
    amount_ml: null,
    amount_scoop: null,
    amount_g: 2,
    amount_mol: null,
  }))
  return next
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
