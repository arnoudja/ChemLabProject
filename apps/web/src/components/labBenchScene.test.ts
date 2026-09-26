import { describe, expect, it } from 'vitest'
import type { CompositionEntry, Item, ItemProperties, LabScene } from '../generated/contracts'
import {
  dishAmountMl,
  distilledWaterAmountMl,
  filtrateAmountMl,
  HCL_STOCK_CAPACITY_ML,
  HCL_STOCK_HCL_MOLES,
  HCL_STOCK_WATER_MASS_G,
  PHI_V_CACL2_ML_PER_MOL,
  PHI_V_HCL_ML_PER_MOL,
  PHI_V_NACL_ML_PER_MOL,
  PHI_V_NAOH_ML_PER_MOL,
  solutionVolumeMl,
  waterAmountMl,
} from './labBenchScene'

function entry(
  partial: Partial<CompositionEntry> & Pick<CompositionEntry, 'substance_id' | 'phase'>,
): CompositionEntry {
  return {
    amount_ml: null,
    amount_scoop: null,
    amount_g: null,
    amount_mol: null,
    ...partial,
  }
}

type SceneItemInput = Partial<Omit<Item, 'properties'>> & {
  properties?: Partial<ItemProperties>
}

function sceneWith(items: SceneItemInput[]): LabScene {
  return {
    lab_id: 'lab-test',
    version: 1,
    temperature_c: 20,
    mode: 'free',
    challenge_completed: false,
    items: items.map((item) => ({
      id: item.id ?? 'item',
      kind: item.kind ?? 'beaker',
      label: item.label ?? 'Item',
      location: item.location ?? 'bench',
      properties: {
        volume_ml: null,
        fill_ml: null,
        transparent: null,
        colourless: null,
        temperature_c: null,
        composition: [],
        holding: [],
        on: null,
        source_item_id: null,
        ...item.properties,
      },
    })),
    last_events: [],
    last_applied_unix_ms: null,
  }
}

describe('Φ_V parity with chemlab-core (golden)', () => {
  it('matches Rust PHI_V constants and stock-derived Φ_HCl', () => {
    expect(PHI_V_NACL_ML_PER_MOL).toBe(22.0)
    expect(PHI_V_CACL2_ML_PER_MOL).toBe(34.0)
    expect(PHI_V_NAOH_ML_PER_MOL).toBe(4.0)
    expect(PHI_V_HCL_ML_PER_MOL).toBeCloseTo(20.7, 1)
    expect(HCL_STOCK_HCL_MOLES).toBeCloseTo(3.447 / 36.46, 12)
  })

  it('HCl stock composition volume is locked to 10.00 ml', () => {
    const stock = [
      entry({ substance_id: 'water', phase: 'liquid', amount_ml: HCL_STOCK_WATER_MASS_G }),
      entry({ substance_id: 'h+', phase: 'aqueous', amount_mol: HCL_STOCK_HCL_MOLES }),
      entry({ substance_id: 'cl-', phase: 'aqueous', amount_mol: HCL_STOCK_HCL_MOLES }),
    ]
    expect(solutionVolumeMl(stock)).toBeCloseTo(HCL_STOCK_CAPACITY_ML, 9)
    expect(
      HCL_STOCK_WATER_MASS_G + HCL_STOCK_HCL_MOLES * PHI_V_HCL_ML_PER_MOL,
    ).toBeCloseTo(HCL_STOCK_CAPACITY_ML, 9)
  })

  it('mixed electrolytes follow V = water + Σ n·Φ_V (Rust pairing)', () => {
    const nOh = 0.1
    const naohOnly = [
      entry({ substance_id: 'water', phase: 'liquid', amount_ml: 100 }),
      entry({ substance_id: 'na+', phase: 'aqueous', amount_mol: nOh }),
      entry({ substance_id: 'oh-', phase: 'aqueous', amount_mol: nOh }),
    ]
    expect(solutionVolumeMl(naohOnly)).toBeCloseTo(
      100 + nOh * PHI_V_NAOH_ML_PER_MOL,
      9,
    )

    const nH = 0.05
    const nNacl = 0.1
    const mixed = [
      entry({ substance_id: 'water', phase: 'liquid', amount_ml: 50 }),
      entry({ substance_id: 'h+', phase: 'aqueous', amount_mol: nH }),
      entry({ substance_id: 'na+', phase: 'aqueous', amount_mol: nNacl }),
      entry({ substance_id: 'cl-', phase: 'aqueous', amount_mol: nH + nNacl }),
    ]
    const expected = 50 + nH * PHI_V_HCL_ML_PER_MOL + nNacl * PHI_V_NACL_ML_PER_MOL
    expect(solutionVolumeMl(mixed)).toBeCloseTo(expected, 9)

    const nCacl2 = 0.02
    const cacl2Brine = [
      entry({ substance_id: 'water', phase: 'liquid', amount_ml: 10 }),
      entry({ substance_id: 'ca2+', phase: 'aqueous', amount_mol: nCacl2 }),
      entry({ substance_id: 'cl-', phase: 'aqueous', amount_mol: 2 * nCacl2 }),
    ]
    expect(solutionVolumeMl(cacl2Brine)).toBeCloseTo(
      10 + nCacl2 * PHI_V_CACL2_ML_PER_MOL,
      9,
    )
  })
})

describe('solutionVolumeMl', () => {
  it('adds NaCl apparent molar volume above water ml', () => {
    const composition = [
      entry({ substance_id: 'water', phase: 'liquid', amount_ml: 100 }),
      entry({ substance_id: 'na+', phase: 'aqueous', amount_mol: 0.5 }),
      entry({ substance_id: 'cl-', phase: 'aqueous', amount_mol: 0.5 }),
    ]
    expect(solutionVolumeMl(composition)).toBeCloseTo(111, 5)
  })
})

describe('SVG amount helpers (dish / filtrate / water)', () => {
  it('uses solution volume for brine fills (not water-only ml)', () => {
    const brine = [
      entry({ substance_id: 'water', phase: 'liquid', amount_ml: 20 }),
      entry({ substance_id: 'na+', phase: 'aqueous', amount_mol: 0.1 }),
      entry({ substance_id: 'cl-', phase: 'aqueous', amount_mol: 0.1 }),
    ]
    const expected = solutionVolumeMl(brine)
    expect(expected).toBeGreaterThan(20)

    const scene = sceneWith([
      {
        id: 'dish-1',
        kind: 'evaporation_dish',
        properties: { composition: brine, fill_ml: expected },
      },
      {
        id: 'beaker-filtrate',
        kind: 'beaker',
        properties: { composition: brine, fill_ml: expected },
      },
      {
        id: 'beaker-water',
        kind: 'beaker',
        properties: { composition: brine, fill_ml: expected },
      },
    ])

    expect(dishAmountMl(scene)).toBeCloseTo(expected, 5)
    expect(filtrateAmountMl(scene)).toBeCloseTo(expected, 5)
    expect(waterAmountMl(scene)).toBeCloseTo(expected, 5)
  })

  it('keeps distilled stock on water ml only', () => {
    const scene = sceneWith([
      {
        id: 'beaker-h2o',
        kind: 'beaker',
        properties: {
          composition: [entry({ substance_id: 'water', phase: 'liquid', amount_ml: 100 })],
          fill_ml: 100,
        },
      },
    ])
    expect(distilledWaterAmountMl(scene)).toBe(100)
  })

  it('returns null when the vessel is missing', () => {
    const scene = sceneWith([])
    expect(dishAmountMl(scene)).toBeNull()
    expect(filtrateAmountMl(scene)).toBeNull()
    expect(waterAmountMl(scene)).toBeNull()
  })
})
