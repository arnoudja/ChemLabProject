import { describe, expect, it } from 'vitest'
import type { LabScene } from '../generated/contracts'
import { normalizeLabScene, optionalArray } from './scene'

describe('optionalArray', () => {
  it('returns the same array when present', () => {
    const values = [1, 2]
    expect(optionalArray(values)).toBe(values)
  })

  it('treats undefined and null as empty', () => {
    expect(optionalArray(undefined)).toEqual([])
    expect(optionalArray(null)).toEqual([])
  })
})

describe('normalizeLabScene', () => {
  it('fills omitted holding, composition, and last_events', () => {
    const wire = {
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
      ],
    } as LabScene

    const scene = normalizeLabScene(wire)
    expect(scene.last_events).toEqual([])
    expect(scene.items[0]?.properties.holding).toEqual([])
    expect(scene.items[0]?.properties.composition).toEqual([])
  })
})
