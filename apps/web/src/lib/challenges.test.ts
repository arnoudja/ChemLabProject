import { describe, expect, it } from 'vitest'
import { CHALLENGES, FREE_MODE, LAB_MODE_OPTIONS, findChallenge } from './challenges'

/**
 * Goldens matching `chemlab-core::challenges` (`SEPARATE_NACL_SIO2`, `CREATE_TABLE_SALT`)
 * and `docs/challenges.md`. Keep in sync when adding a challenge.
 */
const RUST_CATALOG_GOLDENS = [
  {
    id: 'separate-nacl-sio2',
    title: 'Separate salt from sand',
    prompt:
      'The previous student accidentally put all the salt and sand in the main beaker, can you please separate them and put them back in their containers?',
    done: 'Thank you.',
  },
  {
    id: 'create-table-salt',
    title: 'Create table salt',
    prompt: "We're out of NaCl again, can you create some for us?",
    done: 'Thank you again.',
  },
] as const

describe('challenge catalog FE↔Rust parity', () => {
  it('matches Rust challenge copy and mode ids', () => {
    expect(CHALLENGES).toHaveLength(RUST_CATALOG_GOLDENS.length)
    expect(CHALLENGES).toEqual([...RUST_CATALOG_GOLDENS])
  })

  it('exposes Free mode first, then catalog titles', () => {
    expect(LAB_MODE_OPTIONS).toEqual([
      { id: FREE_MODE, title: 'Free mode' },
      ...RUST_CATALOG_GOLDENS.map(({ id, title }) => ({ id, title })),
    ])
  })

  it('findChallenge resolves catalog ids and ignores free', () => {
    expect(findChallenge(FREE_MODE)).toBeNull()
    expect(findChallenge('separate-nacl-sio2')?.title).toBe('Separate salt from sand')
    expect(findChallenge('create-table-salt')?.done).toBe('Thank you again.')
    expect(findChallenge('missing')).toBeNull()
  })
})
