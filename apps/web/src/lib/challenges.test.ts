import { readFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'
import { CHALLENGES, FREE_MODE, LAB_MODE_OPTIONS, findChallenge, type Challenge } from './challenges'

// apps/web/src/lib → repo root
const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../../..')

type CatalogCopy = Pick<Challenge, 'id' | 'title' | 'prompt' | 'done'>

/** Pull `id`/`title`/`prompt`/`done` from each `Challenge { ... }` const in the Rust catalog. */
function catalogCopyFromRust(source: string): CatalogCopy[] {
  const blocks = [
    ...source.matchAll(
      /pub const \w+: Challenge = Challenge \{([\s\S]*?)\n\};/g,
    ),
  ]
  return blocks.map((match) => {
    const body = match[1]
    const field = (name: string): string => {
      const m = body.match(new RegExp(`${name}:\\s*"((?:\\\\.|[^"\\\\])*)"`))
      if (!m) throw new Error(`missing Challenge field ${name} in Rust catalog`)
      return m[1].replace(/\\"/g, '"').replace(/\\n/g, '\n')
    }
    return {
      id: field('id'),
      title: field('title'),
      prompt: field('prompt'),
      done: field('done'),
    }
  })
}

/** Pull Id/Title/Prompt/Done rows from each challenge section table in docs/challenges.md. */
function catalogCopyFromDocs(markdown: string): CatalogCopy[] {
  const sections = markdown.split(/\n## `/).slice(1)
  return sections.map((section) => {
    const field = (label: string): string => {
      const m = section.match(new RegExp(`\\| ${label} \\| (.+?) \\|`))
      if (!m) throw new Error(`missing docs table row ${label}`)
      return m[1].replace(/^`|`$/g, '').trim()
    }
    return {
      id: field('Id'),
      title: field('Title'),
      prompt: field('Prompt'),
      done: field('Done'),
    }
  })
}

describe('challenge catalog FE↔Rust/docs parity', () => {
  const rustSource = readFileSync(
    path.join(repoRoot, 'crates/chemlab-core/src/challenges.rs'),
    'utf8',
  )
  const docsSource = readFileSync(path.join(repoRoot, 'docs/challenges.md'), 'utf8')
  const rustCatalog = catalogCopyFromRust(rustSource)
  const docsCatalog = catalogCopyFromDocs(docsSource)

  it('matches Rust challenge copy (id/title/prompt/done)', () => {
    expect(rustCatalog.length).toBeGreaterThan(0)
    expect(CHALLENGES).toEqual(rustCatalog)
  })

  it('matches docs/challenges.md copy tables', () => {
    expect(docsCatalog.length).toBeGreaterThan(0)
    expect(CHALLENGES).toEqual(docsCatalog)
  })

  it('exposes Free mode first, then catalog titles', () => {
    expect(LAB_MODE_OPTIONS).toEqual([
      { id: FREE_MODE, title: 'Free mode' },
      ...CHALLENGES.map(({ id, title }) => ({ id, title })),
    ])
  })

  it('findChallenge resolves catalog ids and ignores free', () => {
    expect(findChallenge(FREE_MODE)).toBeNull()
    expect(findChallenge('separate-nacl-sio2')?.title).toBe('Separate salt from sand')
    expect(findChallenge('create-table-salt')?.done).toBe('Thank you again.')
    expect(findChallenge('missing')).toBeNull()
  })
})
