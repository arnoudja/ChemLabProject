import type { LabScene } from '../generated/contracts'
import { findChallenge, FREE_MODE } from '../lib/challenges'
import { optionalArray } from '../lib/scene'
import {
  STOCK_FULL_MASS_G,
  STOCK_FULL_SCOOPS,
  WATER_FULL_ML,
  DISTILLED_WATER_CAPACITY_ML,
  HCL_STOCK_CAPACITY_ML,
  HCL_STOCK_WATER_ML,
  HCL_STOCK_HCL_MOLES,
  PIPETTE_VOLUME_ML,
  FILTRATE_CAPACITY_ML,
} from './LabBench'
import {
  H2SO4_STOCK_CAPACITY_ML,
  H2SO4_STOCK_H2SO4_MOLES,
  H2SO4_STOCK_DENSITY_G_PER_ML,
  H2SO4_STOCK_W_W,
} from '../lib/benchAmounts'

export const NACL_EXPLANATION =
  'Sodium chloride (NaCl) dissolves in water at bench temperature.'
export const SAND_EXPLANATION =
  'Sand (silica) does not dissolve in water at bench temperature.'

/** Mirror `chemlab-core::scene::NACL_DELTA_H_SOLUTION_J_PER_MOL` — keep in sync. */
export const NACL_DELTA_H_SOLUTION_J_PER_MOL = 3880
/** Mirror `chemlab-core::scene::CACL2_DELTA_H_SOLUTION_J_PER_MOL` — keep in sync. */
export const CACL2_DELTA_H_SOLUTION_J_PER_MOL = -81300
/** Mirror `chemlab-core::scene::NAOH_DELTA_H_SOLUTION_J_PER_MOL` — keep in sync. */
export const NAOH_DELTA_H_SOLUTION_J_PER_MOL = -44500
/** Mirror `chemlab-core::scene::WATER_SPECIFIC_HEAT_J_PER_G_K` — keep in sync. */
export const WATER_SPECIFIC_HEAT_J_PER_G_K = 4.184
/** Mirror `chemlab-core::scene::C_BEAKER` — keep in sync. */
export const C_BEAKER = 150
export const NACL_MOLAR_MASS_G_PER_MOL = 58.44
export const CACL2_MOLAR_MASS_G_PER_MOL = 110.98
export const NAOH_MOLAR_MASS_G_PER_MOL = 40.0
export const CACL2_EXPLANATION =
  'Calcium chloride (CaCl2) dissolves in water at bench temperature.'
export const NAOH_EXPLANATION =
  'Sodium hydroxide (NaOH) dissolves in water at bench temperature.'

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
        id: 'beaker-hcl',
        kind: 'beaker',
        label: 'Hydrochloric acid (30%)',
        location: 'bench',
        properties: {
          volume_ml: HCL_STOCK_CAPACITY_ML,
          fill_ml: HCL_STOCK_CAPACITY_ML,
          transparent: true,
          colourless: true,
          temperature_c: 20,
          composition: [
            {
              substance_id: 'water',
              phase: 'liquid',
              amount_ml: HCL_STOCK_WATER_ML,
              amount_scoop: null,
              amount_g: null,
              amount_mol: null,
            },
            {
              substance_id: 'h+',
              phase: 'aqueous',
              amount_ml: null,
              amount_scoop: null,
              amount_g: null,
              amount_mol: HCL_STOCK_HCL_MOLES,
            },
            {
              substance_id: 'cl-',
              phase: 'aqueous',
              amount_ml: null,
              amount_scoop: null,
              amount_g: null,
              amount_mol: HCL_STOCK_HCL_MOLES,
            },
          ],
          holding: [],
        },
      },
      {
        id: 'beaker-h2so4',
        kind: 'beaker',
        label: 'Sulfuric acid',
        location: 'bench',
        properties: {
          volume_ml: H2SO4_STOCK_CAPACITY_ML,
          fill_ml: H2SO4_STOCK_CAPACITY_ML,
          transparent: true,
          colourless: true,
          temperature_c: 20,
          composition: [
            {
              substance_id: 'h2so4',
              phase: 'liquid',
              amount_ml: H2SO4_STOCK_CAPACITY_ML,
              amount_scoop: null,
              amount_g: H2SO4_STOCK_CAPACITY_ML * H2SO4_STOCK_DENSITY_G_PER_ML * H2SO4_STOCK_W_W,
              amount_mol: H2SO4_STOCK_H2SO4_MOLES,
            },
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
        id: 'beaker-naoh',
        kind: 'beaker',
        label: 'Sodium hydroxide',
        location: 'bench',
        properties: {
          volume_ml: 250,
          fill_ml: 100,
          transparent: true,
          colourless: true,
          temperature_c: 20,
          composition: [{ substance_id: 'naoh', phase: 'solid', amount_ml: null, amount_scoop: STOCK_FULL_SCOOPS, amount_g: STOCK_FULL_MASS_G, amount_mol: null}],
          holding: [],
        },
      },
      {
        id: 'beaker-na2so4',
        kind: 'beaker',
        label: 'Sodium sulfate',
        location: 'bench',
        properties: {
          volume_ml: 250,
          fill_ml: 100,
          transparent: true,
          colourless: true,
          temperature_c: 20,
          composition: [{ substance_id: 'na2so4', phase: 'solid', amount_ml: null, amount_scoop: STOCK_FULL_SCOOPS, amount_g: STOCK_FULL_MASS_G, amount_mol: null}],
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
export const CREATE_TABLE_SALT_CHALLENGE = findChallenge('create-table-salt')!

/**
 * Layout params mirroring `chemlab-core::challenges::Challenge` fields used by
 * `challenge_scene` / `initial_scene_for_mode`. Prefer this over new hand-cloned
 * start-scene functions when adding a challenge.
 */
export type ChallengeStartLayout = {
  mode: string
  /** Ingredient stock ids kept on the bench (others removed from Free layout). */
  allowedStockItemIds: readonly string[]
  /** Kept stocks that start with none of their solid. */
  emptyStockItemIds: readonly string[]
  /** Dry solids preloaded into `beaker-water`. */
  mainBeakerSolids: readonly { substance_id: string; amount_g: number }[]
  /** Starting liquid ml in `beaker-h2o`. `null` keeps Free mode's full stock. */
  distilledWaterMl: number | null
}

/** Free-mode stock ids that challenges may filter (mirrors server stock catalog). */
const FREE_STOCK_ITEM_IDS = [
  'beaker-h2o',
  'beaker-hcl',
  'beaker-h2so4',
  'beaker-nacl',
  'beaker-naoh',
  'beaker-na2so4',
  'beaker-cacl2',
  'beaker-sand',
] as const

/**
 * One parameterized FE start-scene builder driven by the same allowed/empty/preload
 * lists the server uses. Prefer precomputed snapshots in tests when possible.
 */
export function challengeStartScene(layout: ChallengeStartLayout): LabScene {
  const next = initialScene()
  next.mode = layout.mode
  const allowed = new Set(layout.allowedStockItemIds)
  next.items = next.items.filter(
    (item) => !FREE_STOCK_ITEM_IDS.includes(item.id as (typeof FREE_STOCK_ITEM_IDS)[number]) || allowed.has(item.id),
  )
  for (const stockId of layout.emptyStockItemIds) {
    const stock = next.items.find((item) => item.id === stockId)
    if (!stock) continue
    stock.properties.composition = optionalArray(stock.properties.composition).map((entry) =>
      entry.phase === 'solid'
        ? { ...entry, amount_scoop: 0, amount_g: 0 }
        : entry,
    )
  }
  if (layout.distilledWaterMl != null) {
    const h2o = next.items.find((item) => item.id === 'beaker-h2o')!
    h2o.properties.fill_ml = layout.distilledWaterMl
    h2o.properties.composition = optionalArray(h2o.properties.composition).map((entry) =>
      entry.substance_id === 'water' && entry.phase === 'liquid'
        ? { ...entry, amount_ml: layout.distilledWaterMl }
        : entry,
    )
  }
  const beaker = next.items.find((item) => item.id === 'beaker-water')!
  beaker.properties.composition = layout.mainBeakerSolids.map((solid) => ({
    substance_id: solid.substance_id,
    phase: 'solid' as const,
    amount_ml: null,
    amount_scoop: null,
    amount_g: solid.amount_g,
    amount_mol: null,
  }))
  beaker.properties.fill_ml = 0
  return next
}

/** Layout goldens mirroring Rust `SEPARATE_NACL_SIO2` / `CREATE_TABLE_SALT`. */
export const SEPARATE_NACL_SIO2_LAYOUT: ChallengeStartLayout = {
  mode: SEPARATE_CHALLENGE.id,
  allowedStockItemIds: ['beaker-h2o', 'beaker-nacl', 'beaker-sand'],
  emptyStockItemIds: ['beaker-nacl', 'beaker-sand'],
  mainBeakerSolids: [
    { substance_id: 'nacl', amount_g: 2 },
    { substance_id: 'sand', amount_g: 2 },
  ],
  distilledWaterMl: 10,
}

export const CREATE_TABLE_SALT_LAYOUT: ChallengeStartLayout = {
  mode: CREATE_TABLE_SALT_CHALLENGE.id,
  allowedStockItemIds: ['beaker-h2o', 'beaker-hcl', 'beaker-naoh', 'beaker-nacl'],
  emptyStockItemIds: ['beaker-nacl'],
  mainBeakerSolids: [],
  distilledWaterMl: null,
}

export const CREATE_SODIUM_SULFATE_LAYOUT: ChallengeStartLayout = {
  mode: 'create-sodium-sulfate',
  allowedStockItemIds: ['beaker-h2o', 'beaker-h2so4', 'beaker-naoh', 'beaker-na2so4'],
  emptyStockItemIds: ['beaker-na2so4'],
  mainBeakerSolids: [],
  distilledWaterMl: null,
}

export const PRECIPITATE_GYPSUM_LAYOUT: ChallengeStartLayout = {
  mode: 'precipitate-gypsum',
  allowedStockItemIds: ['beaker-h2o', 'beaker-cacl2', 'beaker-na2so4', 'beaker-h2so4'],
  emptyStockItemIds: [],
  mainBeakerSolids: [],
  distilledWaterMl: null,
}

export const MAKE_HCL_FROM_GYPSUM_LAYOUT: ChallengeStartLayout = {
  mode: 'make-hcl-from-gypsum',
  allowedStockItemIds: ['beaker-h2o', 'beaker-cacl2', 'beaker-h2so4'],
  emptyStockItemIds: [],
  mainBeakerSolids: [],
  distilledWaterMl: null,
}

export const HOT_PACK_CACL2_LAYOUT: ChallengeStartLayout = {
  mode: 'hot-pack-cacl2',
  allowedStockItemIds: ['beaker-h2o', 'beaker-cacl2'],
  emptyStockItemIds: [],
  mainBeakerSolids: [],
  distilledWaterMl: 10,
}

export const COMMON_ION_NACL_LAYOUT: ChallengeStartLayout = {
  mode: 'common-ion-nacl',
  allowedStockItemIds: ['beaker-h2o', 'beaker-hcl', 'beaker-nacl'],
  emptyStockItemIds: [],
  mainBeakerSolids: [],
  distilledWaterMl: 10,
}

export const NEUTRALIZE_TO_PH7_LAYOUT: ChallengeStartLayout = {
  mode: 'neutralize-to-ph7',
  allowedStockItemIds: ['beaker-h2o', 'beaker-hcl', 'beaker-naoh'],
  emptyStockItemIds: [],
  mainBeakerSolids: [],
  distilledWaterMl: null,
}

export const DILUTE_SULFURIC_SAFE_LAYOUT: ChallengeStartLayout = {
  mode: 'dilute-sulfuric-safe',
  allowedStockItemIds: ['beaker-h2o', 'beaker-h2so4'],
  emptyStockItemIds: [],
  mainBeakerSolids: [],
  distilledWaterMl: null,
}

export const CONCENTRATE_HCL_AZEOTROPE_LAYOUT: ChallengeStartLayout = {
  mode: 'concentrate-hcl-azeotrope',
  allowedStockItemIds: ['beaker-h2o', 'beaker-hcl'],
  emptyStockItemIds: [],
  mainBeakerSolids: [],
  distilledWaterMl: null,
}

const CHALLENGE_START_LAYOUTS: ChallengeStartLayout[] = [
  SEPARATE_NACL_SIO2_LAYOUT,
  CREATE_TABLE_SALT_LAYOUT,
  CREATE_SODIUM_SULFATE_LAYOUT,
  PRECIPITATE_GYPSUM_LAYOUT,
  MAKE_HCL_FROM_GYPSUM_LAYOUT,
  HOT_PACK_CACL2_LAYOUT,
  COMMON_ION_NACL_LAYOUT,
  NEUTRALIZE_TO_PH7_LAYOUT,
  DILUTE_SULFURIC_SAFE_LAYOUT,
  CONCENTRATE_HCL_AZEOTROPE_LAYOUT,
]

export function layoutForMode(mode: string): ChallengeStartLayout | null {
  return CHALLENGE_START_LAYOUTS.find((layout) => layout.mode === mode) ?? null
}

/** @deprecated Prefer `challengeStartScene(SEPARATE_NACL_SIO2_LAYOUT)` or a snapshot. */
export function challengeScene(): LabScene {
  return challengeStartScene(SEPARATE_NACL_SIO2_LAYOUT)
}

/** @deprecated Prefer `challengeStartScene(CREATE_TABLE_SALT_LAYOUT)` or a snapshot. */
export function createTableSaltScene(): LabScene {
  return challengeStartScene(CREATE_TABLE_SALT_LAYOUT)
}

export function withDryDishSolids(
  scene: LabScene,
  solids: { substance_id: 'nacl' | 'cacl2' | 'sand' | 'naoh' | 'na2so4'; amount_g: number }[],
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
