import { describe, expect, it } from 'vitest'
import type { CompositionEntry, Item, LabScene } from '../generated/contracts'
import { dishAmountMl, filtrateAmountMl, solutionVolumeMl } from './labBenchScene'

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

function sceneWith(items: Partial<Item>[]): LabScene {
  return {
    lab_id: 'lab-test',
    version: 1,
    temperature_c: 20,
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

describe('dishAmountMl / filtrateAmountMl', () => {
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
    ])

    expect(dishAmountMl(scene)).toBeCloseTo(expected, 5)
    expect(filtrateAmountMl(scene)).toBeCloseTo(expected, 5)
  })

  it('returns null when the vessel is missing', () => {
    const scene = sceneWith([])
    expect(dishAmountMl(scene)).toBeNull()
    expect(filtrateAmountMl(scene)).toBeNull()
  })
})
