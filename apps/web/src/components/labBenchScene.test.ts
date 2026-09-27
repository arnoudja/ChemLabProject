import { describe, expect, it } from 'vitest'
import type { CompositionEntry, Item, ItemProperties, LabScene } from '../generated/contracts'
import {
  dishAmountMl,
  distilledWaterAmountMl,
  filtrateAmountMl,
  HCL_STOCK_CAPACITY_ML,
  HCL_STOCK_HCL_MOLES,
  HCL_STOCK_WATER_MASS_G,
  H2SO4_STOCK_CAPACITY_ML,
  NA2SO4_ID,
  NAOH_ID,
  PHI_V_CACL2_ML_PER_MOL,
  PHI_V_CASO4_ML_PER_MOL,
  PHI_V_H2SO4_ML_PER_MOL,
  PHI_V_HCL_ML_PER_MOL,
  PHI_V_NA2SO4_ML_PER_MOL,
  PHI_V_NAHSO4_ML_PER_MOL,
  PHI_V_NACL_ML_PER_MOL,
  PHI_V_NAOH_ML_PER_MOL,
  sceneNeedsClockPoll,
  sceneNeedsDissolvePoll,
  sceneNeedsThermalPoll,
  solutionVolumeMl,
  tongsHeldVesselId,
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
    expect(PHI_V_H2SO4_ML_PER_MOL).toBe(40.0)
    expect(PHI_V_NAHSO4_ML_PER_MOL).toBe(30.0)
    expect(PHI_V_NA2SO4_ML_PER_MOL).toBe(20.0)
    expect(PHI_V_CASO4_ML_PER_MOL).toBe(15.0)
    expect(PHI_V_HCL_ML_PER_MOL).toBeCloseTo(20.7, 1)
    expect(HCL_STOCK_HCL_MOLES).toBeCloseTo(3.447 / 36.46, 12)
  })

  it('pure liquid H2SO4 stock volume is locked to 10.00 ml', () => {
    const stock = [
      entry({
        substance_id: 'h2so4',
        phase: 'liquid',
        amount_ml: H2SO4_STOCK_CAPACITY_ML,
      }),
    ]
    expect(solutionVolumeMl(stock)).toBeCloseTo(H2SO4_STOCK_CAPACITY_ML, 9)
  })

  it('sulfuric protons do not take Φ_V_HCl', () => {
    const n = 0.05
    const sulfuric = [
      entry({ substance_id: 'water', phase: 'liquid', amount_ml: 50 }),
      entry({ substance_id: 'h+', phase: 'aqueous', amount_mol: 2 * n }),
      entry({ substance_id: 'so4^2-', phase: 'aqueous', amount_mol: n }),
    ]
    expect(solutionVolumeMl(sulfuric)).toBeCloseTo(50 + n * PHI_V_H2SO4_ML_PER_MOL, 9)
  })

  it('NaHSO4-like bisulfate pairs as Φ_V_NaHSO4 not free H2SO4', () => {
    const nHso4 = 0.043
    const nSo4 = 0.007
    const nH = 0.007
    const nNa = 0.05
    const mix = [
      entry({ substance_id: 'water', phase: 'liquid', amount_ml: 100 }),
      entry({ substance_id: 'na+', phase: 'aqueous', amount_mol: nNa }),
      entry({ substance_id: 'hso4-', phase: 'aqueous', amount_mol: nHso4 }),
      entry({ substance_id: 'so4^2-', phase: 'aqueous', amount_mol: nSo4 }),
      entry({ substance_id: 'h+', phase: 'aqueous', amount_mol: nH }),
    ]
    // Bisulfate → NaHSO₄; Ka2 remnant Na⁺+H⁺+SO₄²⁻ → ½ Na₂SO₄ + ½ H₂SO₄.
    const nNahso4 = nHso4
    const nNa2so4 = Math.min((nNa - nNahso4) * 0.5, nSo4)
    const nH2so4 = Math.min(nH, nSo4 - nNa2so4)
    const expected =
      100 +
      nNahso4 * PHI_V_NAHSO4_ML_PER_MOL +
      nNa2so4 * PHI_V_NA2SO4_ML_PER_MOL +
      nH2so4 * PHI_V_H2SO4_ML_PER_MOL
    expect(solutionVolumeMl(mix)).toBeCloseTo(expected, 9)
    expect(nNahso4).toBeGreaterThan(0.04)
    expect(nH2so4).toBeLessThan(0.01)
  })

  it('dissolved gypsum uses Φ_V_CaSO4 not CaCl2', () => {
    const n = 0.01
    const gypsum = [
      entry({ substance_id: 'water', phase: 'liquid', amount_ml: 50 }),
      entry({ substance_id: 'ca2+', phase: 'aqueous', amount_mol: n }),
      entry({ substance_id: 'so4^2-', phase: 'aqueous', amount_mol: n }),
    ]
    expect(solutionVolumeMl(gypsum)).toBeCloseTo(50 + n * PHI_V_CASO4_ML_PER_MOL, 9)
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

describe('tongsHeldVesselId', () => {
  it('recognizes held solid Na2SO4 stock like NaOH', () => {
    for (const stockId of [NAOH_ID, NA2SO4_ID] as const) {
      const scene = sceneWith([
        {
          id: 'tongs-1',
          kind: 'tongs',
          properties: { source_item_id: stockId },
        },
        { id: stockId, kind: 'beaker', location: 'held' },
      ])
      expect(tongsHeldVesselId(scene)).toBe(stockId)
    }
  })
})

describe('clock poll gates', () => {
  it('needs dissolve poll for wet beaker with solid NaCl at ambient', () => {
    const scene = sceneWith([
      {
        id: 'beaker-water',
        kind: 'beaker',
        properties: {
          temperature_c: 20,
          composition: [
            entry({ substance_id: 'water', phase: 'liquid', amount_ml: 200 }),
            entry({ substance_id: 'nacl', phase: 'solid', amount_g: 1.6 }),
          ],
        },
      },
      {
        id: 'burner-1',
        kind: 'burner',
        properties: { on: false },
      },
    ])
    expect(sceneNeedsDissolvePoll(scene)).toBe(true)
    expect(sceneNeedsThermalPoll(scene)).toBe(false)
    expect(sceneNeedsClockPoll(scene)).toBe(true)
  })

  it('does not dissolve-poll dry solid stock or sand slurry', () => {
    const dryStock = sceneWith([
      {
        id: 'beaker-nacl',
        kind: 'beaker',
        properties: {
          composition: [entry({ substance_id: 'nacl', phase: 'solid', amount_g: 2 })],
        },
      },
    ])
    expect(sceneNeedsDissolvePoll(dryStock)).toBe(false)

    const sandOnly = sceneWith([
      {
        id: 'beaker-water',
        kind: 'beaker',
        properties: {
          composition: [
            entry({ substance_id: 'water', phase: 'liquid', amount_ml: 200 }),
            entry({ substance_id: 'sand', phase: 'solid', amount_g: 2 }),
          ],
        },
      },
    ])
    expect(sceneNeedsDissolvePoll(sandOnly)).toBe(false)
    expect(sceneNeedsClockPoll(sandOnly)).toBe(false)
  })
})
