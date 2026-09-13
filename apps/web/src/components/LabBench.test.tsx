/** @vitest-environment jsdom */
import '@testing-library/jest-dom/vitest'
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { LabAction, LabScene } from '../generated/contracts'
import { LabBench, stockFillRatio } from './LabBench'
import { clearCsrfTokenCache } from '../lib/api'
import { optionalArray } from '../lib/scene'

const NACL_EXPLANATION =
  'Sodium chloride (NaCl) dissolves in water at bench temperature.'
const SAND_EXPLANATION =
  'Sand (silica) does not dissolve in water at bench temperature.'

/** Mirror `chemlab-core::scene::NACL_DELTA_H_SOLUTION_J_PER_MOL` — keep in sync. */
const NACL_DELTA_H_SOLUTION_J_PER_MOL = 3880
/** Mirror `chemlab-core::scene::WATER_SPECIFIC_HEAT_J_PER_G_K` — keep in sync. */
const WATER_SPECIFIC_HEAT_J_PER_G_K = 4.184

function emptyProps() {
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

function initialScene(): LabScene {
  return {
    lab_id: 'lab-1',
    version: 0,
    temperature_c: 20,
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
          composition: [{ substance_id: 'nacl', phase: 'solid', amount_ml: null, amount_scoop: 10, amount_g: 2, amount_mol: null}],
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
          composition: [{ substance_id: 'sand', phase: 'solid', amount_ml: null, amount_scoop: 10, amount_g: 2, amount_mol: null}],
          holding: [],
        },
      },
      {
        id: 'beaker-water',
        kind: 'beaker',
        label: 'Water',
        location: 'bench',
        properties: {
          volume_ml: 250,
          fill_ml: 200,
          transparent: true,
          colourless: true,
          temperature_c: 20,
          composition: [
            { substance_id: 'water', phase: 'liquid', amount_ml: 200, amount_scoop: null, amount_g: null, amount_mol: null},
          ],
          holding: [],
        },
      },
    ],
  }
}

function cloneScene(scene: LabScene): LabScene {
  return structuredClone(scene)
}

function withScoop(scene: LabScene, substance: 'nacl' | 'sand'): LabScene {
  const next = cloneScene(scene)
  const spoon = next.items.find((item) => item.id === 'spoon-1')!
  const stockId = substance === 'nacl' ? 'beaker-nacl' : 'beaker-sand'
  const stock = next.items.find((item) => item.id === stockId)!
  const solid = optionalArray(stock.properties.composition).find(
    (entry) => entry.substance_id === substance && entry.phase === 'solid',
  )
  if (solid) {
    const scoops = (solid.amount_scoop ?? 0) - 1
    solid.amount_scoop = scoops
    solid.amount_g = scoops * 0.2
  }
  spoon.location = 'hand'
  spoon.properties.holding = [
    {
      substance_id: substance,
      phase: 'solid',
      amount_ml: null,
      amount_scoop: 1,
      amount_g: 0.2,
      amount_mol: null,
    },
  ]
  next.last_events = [{ kind: 'scooped', message: `Scooped ${substance}.` }]
  next.version += 1
  return next
}

function withPutBack(scene: LabScene, substance: 'nacl' | 'sand'): LabScene {
  const next = cloneScene(scene)
  const spoon = next.items.find((item) => item.id === 'spoon-1')!
  const stockId = substance === 'nacl' ? 'beaker-nacl' : 'beaker-sand'
  const stock = next.items.find((item) => item.id === stockId)!
  const solid = optionalArray(stock.properties.composition).find(
    (entry) => entry.substance_id === substance && entry.phase === 'solid',
  )
  const held = optionalArray(spoon.properties.holding)[0]
  const scoopsAdd = held?.amount_scoop ?? 1
  if (solid) {
    const scoops = (solid.amount_scoop ?? 0) + scoopsAdd
    solid.amount_scoop = scoops
    solid.amount_g = scoops * 0.2
  }
  spoon.properties.holding = []
  next.last_events = [{ kind: 'returned', message: `Returned ${substance}.` }]
  next.version += 1
  return next
}

function afterNaclPour(scene: LabScene): LabScene {
  const next = cloneScene(scene)
  const spoon = next.items.find((item) => item.id === 'spoon-1')!
  const water = next.items.find((item) => item.id === 'beaker-water')!
  spoon.properties.holding = []
  // Mirror server-authored aqueous ions after NaCl dissolve (not client dissociation).
  // 0.2 g NaCl / 58.44 g·mol⁻¹ ≈ 0.003422 mol
  const moles = 0.2 / 58.44
  water.properties.composition = [
    ...optionalArray(water.properties.composition),
    {
      substance_id: 'na+',
      phase: 'aqueous',
      amount_ml: null,
      amount_scoop: null,
      amount_g: null,
      amount_mol: moles,
    },
    {
      substance_id: 'cl-',
      phase: 'aqueous',
      amount_ml: null,
      amount_scoop: null,
      amount_g: null,
      amount_mol: moles,
    },
  ]
  // Mirror server endothermic cooling (water mass ≈ liquid amount_ml at 1 g/ml).
  const waterMassG =
    optionalArray(water.properties.composition).find(
      (entry) => entry.substance_id === 'water' && entry.phase === 'liquid',
    )?.amount_ml ?? 0
  const heatJ = moles * NACL_DELTA_H_SOLUTION_J_PER_MOL
  const currentT = water.properties.temperature_c ?? next.temperature_c
  water.properties.temperature_c =
    currentT - heatJ / (waterMassG * WATER_SPECIFIC_HEAT_J_PER_G_K)
  next.last_events = [
    { kind: 'poured', message: 'Poured onto water.' },
    { kind: 'dissolved', message: NACL_EXPLANATION },
  ]
  next.version += 1
  return next
}

function afterSandPour(scene: LabScene): LabScene {
  const next = cloneScene(scene)
  const spoon = next.items.find((item) => item.id === 'spoon-1')!
  const water = next.items.find((item) => item.id === 'beaker-water')!
  spoon.properties.holding = []
  water.properties.composition = [
    ...optionalArray(water.properties.composition),
    {
      substance_id: 'sand',
      phase: 'solid',
      amount_ml: null,
      amount_scoop: 1,
      amount_g: 0.2,
      amount_mol: null,
    },
  ]
  next.last_events = [
    { kind: 'poured', message: 'Poured onto water.' },
    { kind: 'did_not_dissolve', message: SAND_EXPLANATION },
  ]
  next.version += 1
  return next
}

function jsonResponse(body: unknown, status = 200) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  })
}

function stubLabFetch(options?: {
  scene?: LabScene
  actionHandler?: (action: LabAction, scene: LabScene) => LabScene | { error: string; code: string; status: number }
  sceneError?: { error: string; code: string; status: number }
}) {
  let scene = cloneScene(options?.scene ?? initialScene())
  return vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = String(input)
    if (url === '/api/auth/csrf') {
      return jsonResponse({ csrf_token: 'tok-123' })
    }
    if (url === '/api/lab/scene') {
      if (options?.sceneError) {
        return jsonResponse(
          { error: options.sceneError.error, code: options.sceneError.code },
          options.sceneError.status,
        )
      }
      return jsonResponse(scene)
    }
    if (url === '/api/lab/action') {
      const action = init?.body ? (JSON.parse(String(init.body)) as LabAction) : null
      if (!action) {
        return jsonResponse({ error: 'bad request', code: 'invalid_input' }, 400)
      }
      if (options?.actionHandler) {
        const result = options.actionHandler(action, scene)
        if ('error' in result) {
          return jsonResponse({ error: result.error, code: result.code }, result.status)
        }
        scene = result
        return jsonResponse({ scene })
      }
      if (action.type === 'use_tool' && (action.target_item_id === 'beaker-nacl' || action.target_item_id === 'beaker-sand')) {
        const targetSubstance = action.target_item_id === 'beaker-nacl' ? 'nacl' : 'sand'
        const held = optionalArray(
          scene.items.find((item) => item.id === 'spoon-1')?.properties.holding,
        )[0]
        if (held) {
          if (held.substance_id === targetSubstance) {
            scene = withPutBack(scene, targetSubstance)
            return jsonResponse({ scene })
          }
          return jsonResponse({ error: 'invalid action', code: 'invalid_action' }, 400)
        }
        scene = withScoop(scene, targetSubstance)
        return jsonResponse({ scene })
      }
      if (action.type === 'pour') {
        const held = optionalArray(
          scene.items.find((item) => item.id === 'spoon-1')?.properties.holding,
        )[0]
        if (held?.substance_id === 'nacl') {
          scene = afterNaclPour(scene)
          return jsonResponse({ scene })
        }
        if (held?.substance_id === 'sand') {
          scene = afterSandPour(scene)
          return jsonResponse({ scene })
        }
      }
      if (action.type === 'reset') {
        const next = initialScene()
        next.lab_id = scene.lab_id
        next.version = scene.version + 1
        next.last_events = [{ kind: 'reset', message: 'Lab reset to the starting bench.' }]
        scene = next
        return jsonResponse({ scene })
      }
      return jsonResponse({ error: 'invalid action', code: 'invalid_action' }, 400)
    }
    if (url === '/api/lab/dissolve') {
      throw new Error('LabBench must not call /api/lab/dissolve')
    }
    return jsonResponse({ error: 'not found', code: 'not_found' }, 404)
  })
}

function lastActionInit(fetchMock: ReturnType<typeof vi.fn>) {
  const calls = fetchMock.mock.calls.filter(([url]) => String(url) === '/api/lab/action')
  return calls.at(-1)?.[1] as RequestInit | undefined
}

describe('LabBench', () => {
  beforeEach(() => {
    clearCsrfTokenCache()
  })

  afterEach(() => {
    cleanup()
    vi.unstubAllGlobals()
  })

  it('loads the server scene and does not call dissolve', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)

    expect(await screen.findByRole('button', { name: 'Spoon' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Sand' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Water beaker' })).toBeInTheDocument()
    const saltLabel = document.querySelector('[data-stock-label="nacl"]')
    expect(saltLabel?.textContent).toContain('NaCl')
    expect(saltLabel?.textContent).toContain('(Sodium chloride)')
    expect(saltLabel?.textContent).toContain('(Table salt)')
    const sandLabel = document.querySelector('[data-stock-label="sand"]')
    expect(sandLabel?.querySelector('sub')?.textContent).toBe('2')
    expect(sandLabel?.textContent).toContain('(Silicon dioxide)')
    expect(sandLabel?.textContent).toContain('(Sand)')
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'none')
    expect(fetchMock).toHaveBeenCalledWith('/api/lab/scene', { credentials: 'include' })
    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/dissolve', expect.anything())
    expect(screen.queryByRole('status')).not.toBeInTheDocument()
  })

  it('renders when server omits empty holding/composition/last_events (serde skip)', async () => {
    // Wire JSON from skip_serializing_if = Vec::is_empty — no holding/composition/last_events keys.
    const wireScene = {
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
            composition: [{ substance_id: 'nacl', phase: 'solid', amount_ml: null, amount_scoop: 10, amount_g: 2, amount_mol: null}],
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
            composition: [{ substance_id: 'sand', phase: 'solid', amount_ml: null, amount_scoop: 10, amount_g: 2, amount_mol: null}],
          },
        },
        {
          id: 'beaker-water',
          kind: 'beaker',
          label: 'Water',
          location: 'bench',
          properties: {
            volume_ml: 250,
            fill_ml: 200,
            transparent: true,
            colourless: true,
            temperature_c: 20,
            composition: [
              { substance_id: 'water', phase: 'liquid', amount_ml: 200, amount_scoop: null, amount_g: null, amount_mol: null},
            ],
          },
        },
      ],
    }
    expect(JSON.stringify(wireScene)).not.toContain('"holding"')
    expect(JSON.stringify(wireScene)).not.toContain('"last_events"')

    vi.stubGlobal(
      'fetch',
      stubLabFetch({ scene: wireScene as LabScene }),
    )

    expect(() => render(<LabBench />)).not.toThrow()
    expect(await screen.findByRole('button', { name: 'Spoon' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Water beaker' })).toBeInTheDocument()
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'none')
    expect(screen.queryByRole('status')).not.toBeInTheDocument()
    expect(screen.queryByRole('alert')).not.toBeInTheDocument()
  })

  it('clicking the spoon holds it as the cursor tool', async () => {
    vi.stubGlobal('fetch', stubLabFetch())

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Spoon' })

    fireEvent.click(screen.getByRole('button', { name: 'Spoon' }))

    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'spoon')
    expect(screen.getByRole('button', { name: 'Spoon' })).toHaveAttribute('aria-pressed', 'true')
  })

  it('does not scoop or post an action without a held spoon', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Spoon' })

    fireEvent.click(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' }))
    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))

    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'none')
    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/action', expect.anything())
  })

  it('idle water click shows beaker contents and temperature from the scene', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Water beaker' })

    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))

    const panel = await screen.findByRole('dialog', { name: 'Contents of Water' })
    expect(panel).toHaveTextContent('H2O (l)')
    expect(panel).toHaveTextContent('200 ml')
    expect(panel).toHaveTextContent('Temperature: 20.00°C')
    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/action', expect.anything())
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'none')
  })

  it('idle salt inspect shows server stock mass', async () => {
    vi.stubGlobal('fetch', stubLabFetch())

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Sodium chloride (NaCl)' })

    fireEvent.click(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' }))

    const panel = await screen.findByRole('dialog', { name: 'Contents of Sodium chloride' })
    expect(panel).toHaveTextContent('NaCl (s)')
    expect(panel).toHaveTextContent('2 g')
  })

  it('after scoop, salt inspect shows depleted server stock mass', async () => {
    const scooped = withScoop(initialScene(), 'nacl')
    const fetchMock = stubLabFetch({ scene: scooped })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Sodium chloride (NaCl)' })

    fireEvent.click(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' }))

    const panel = await screen.findByRole('dialog', { name: 'Contents of Sodium chloride' })
    expect(panel).toHaveTextContent('NaCl (s)')
    expect(panel).toHaveTextContent('1.8 g')
  })

  it('idle inspect after dissolve shows aqueous ions from the server composition', async () => {
    const dissolved = afterNaclPour(withScoop(initialScene(), 'nacl'))
    dissolved.items.find((item) => item.id === 'spoon-1')!.properties.holding = []
    const fetchMock = stubLabFetch({ scene: dissolved })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Water beaker' })

    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))

    const panel = await screen.findByRole('dialog', { name: 'Contents of Water' })
    expect(panel).toHaveTextContent('H2O (l)')
    expect(panel).toHaveTextContent('Na+ (aq)')
    expect(panel).toHaveTextContent('Cl− (aq)')
    expect(panel).toHaveTextContent('0.017 M')
    expect(panel.querySelectorAll('sup')).toHaveLength(2)
    expect(panel).toHaveTextContent('Temperature: 19.98°C')
    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/action', expect.anything())
  })

  it('idle inspect after sand pour shows aggregated SiO2 mass not molarity', async () => {
    const leftover = afterSandPour(withScoop(initialScene(), 'sand'))
    leftover.items.find((item) => item.id === 'spoon-1')!.properties.holding = []
    const fetchMock = stubLabFetch({ scene: leftover })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Water beaker' })

    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))

    const panel = await screen.findByRole('dialog', { name: 'Contents of Water' })
    expect(panel).toHaveTextContent('SiO2 (s)')
    expect(panel).toHaveTextContent('0.2 g')
    expect(panel.querySelector('sub')?.textContent).toBe('2')
    expect(panel).not.toHaveTextContent(' M')
    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/action', expect.anything())
  })

  it('with spoon selected, water click does not open inspect', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Spoon' })

    fireEvent.click(screen.getByRole('button', { name: 'Spoon' }))
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'spoon')

    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))

    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/action', expect.anything())
  })


  it('shows full stock fill from server amount_g and lowers after scoop', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Spoon' })

    expect(document.querySelector('[data-stock-solid="nacl"]')).toHaveAttribute('data-stock-fill', '1.00')
    expect(document.querySelector('[data-stock-solid="sand"]')).toHaveAttribute('data-stock-fill', '1.00')
    expect(stockFillRatio(2)).toBe(1)
    expect(stockFillRatio(1.8)).toBeCloseTo(0.9)

    fireEvent.click(screen.getByRole('button', { name: 'Spoon' }))
    fireEvent.click(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' }))

    await waitFor(() => {
      expect(document.querySelector('[data-stock-solid="nacl"]')).toHaveAttribute('data-stock-fill', '0.90')
    })
    expect(document.querySelector('[data-stock-solid="sand"]')).toHaveAttribute('data-stock-fill', '1.00')
  })

  it('puts salt back into the salt stock and restores fill from the server scene', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Spoon' })

    fireEvent.click(screen.getByRole('button', { name: 'Spoon' }))
    fireEvent.click(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' }))
    await waitFor(() => {
      expect(document.querySelector('[data-stock-solid="nacl"]')).toHaveAttribute('data-stock-fill', '0.90')
    })

    fireEvent.click(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' }))
    await waitFor(() => {
      expect(document.querySelector('[data-stock-solid="nacl"]')).toHaveAttribute('data-stock-fill', '1.00')
    })
    expect(screen.getByRole('status')).toHaveTextContent('returned')
    // Spoon holding cleared — cursor tool stays spoon without solid fill.
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'spoon')
  })

  it('rejects putting salt into the sand stock beaker', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Spoon' })

    fireEvent.click(screen.getByRole('button', { name: 'Spoon' }))
    fireEvent.click(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' }))
    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'nacl')
    })

    fireEvent.click(screen.getByRole('button', { name: 'Sand' }))
    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument()
    })
    expect(document.querySelector('[data-stock-solid="sand"]')).toHaveAttribute('data-stock-fill', '1.00')
    expect(document.querySelector('[data-stock-solid="nacl"]')).toHaveAttribute('data-stock-fill', '0.90')
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'nacl')
  })

  it('spoon then nacl then water posts use_tool then pour with CSRF and shows server events', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Spoon' })

    fireEvent.click(screen.getByRole('button', { name: 'Spoon' }))
    fireEvent.click(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' }))

    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'nacl')
    })
    expect(lastActionInit(fetchMock)).toEqual(
      expect.objectContaining({
        method: 'POST',
        credentials: 'include',
        headers: expect.objectContaining({
          'content-type': 'application/json',
          'X-CSRF-Token': 'tok-123',
        }),
        body: JSON.stringify({
          type: 'use_tool',
          tool_item_id: 'spoon-1',
          target_item_id: 'beaker-nacl',
        }),
      }),
    )

    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))

    await waitFor(() => {
      expect(screen.getByRole('status')).toHaveTextContent(NACL_EXPLANATION)
    })
    expect(screen.getByRole('status')).toHaveTextContent('Server outcome: dissolved')
    expect(lastActionInit(fetchMock)).toEqual(
      expect.objectContaining({
        method: 'POST',
        credentials: 'include',
        headers: expect.objectContaining({
          'content-type': 'application/json',
          'X-CSRF-Token': 'tok-123',
        }),
        body: JSON.stringify({
          type: 'pour',
          source_item_id: 'spoon-1',
          target_item_id: 'beaker-water',
        }),
      }),
    )
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'spoon')
    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/dissolve', expect.anything())
  })

  it('renders a surprising server sand event without client solubility branching', async () => {
    const fetchMock = stubLabFetch({
      actionHandler: (action, scene) => {
        if (action.type === 'use_tool') return withScoop(scene, 'sand')
        const next = cloneScene(scene)
        const spoon = next.items.find((item) => item.id === 'spoon-1')!
        spoon.properties.holding = []
        next.last_events = [
          { kind: 'dissolved', message: 'Unexpected server sentence for sand.' },
        ]
        next.version += 1
        return next
      },
    })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Spoon' })

    fireEvent.click(screen.getByRole('button', { name: 'Spoon' }))
    fireEvent.click(screen.getByRole('button', { name: 'Sand' }))
    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'sand')
    })
    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))

    await waitFor(() => {
      expect(screen.getByRole('status')).toHaveTextContent('Unexpected server sentence for sand.')
    })
    expect(screen.getByRole('status')).toHaveTextContent('Server outcome: dissolved')
    expect(screen.queryByText(SAND_EXPLANATION)).not.toBeInTheDocument()
    // No undissolved solid in the surprising payload → no leftover grains invented.
    expect(screen.getByRole('button', { name: 'Water beaker' }).querySelectorAll('circle')).toHaveLength(0)
  })

  it('sand pour shows the server did-not-dissolve sentence and leftover grains from the scene', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Spoon' })

    fireEvent.click(screen.getByRole('button', { name: 'Spoon' }))
    fireEvent.click(screen.getByRole('button', { name: 'Sand' }))
    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'sand')
    })
    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))

    await waitFor(() => {
      expect(screen.getByRole('status')).toHaveTextContent(SAND_EXPLANATION)
    })
    expect(screen.getByRole('status')).toHaveTextContent('Server outcome: did not dissolve')
    // Leftover grains are SVG circles rendered only because the server put solid sand in water.
    expect(
      screen.getByRole('button', { name: 'Water beaker' }).querySelectorAll('circle').length,
    ).toBeGreaterThan(0)
  })

  it('empty spoon on water does not post a pour action', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Spoon' })

    fireEvent.click(screen.getByRole('button', { name: 'Spoon' }))
    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))

    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/action', expect.anything())
    expect(screen.queryByRole('status')).not.toBeInTheDocument()
  })

  it('shows the server action error and no outcome after a pour', async () => {
    vi.stubGlobal(
      'fetch',
      stubLabFetch({
        actionHandler: (action, scene) => {
          if (action.type === 'use_tool') return withScoop(scene, 'nacl')
          return { error: 'Login required', code: 'unauthenticated', status: 401 }
        },
      }),
    )

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Spoon' })

    fireEvent.click(screen.getByRole('button', { name: 'Spoon' }))
    fireEvent.click(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' }))
    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'nacl')
    })
    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))

    expect(await screen.findByRole('alert')).toHaveTextContent('Login required')
    expect(screen.queryByText(/Server outcome:/)).not.toBeInTheDocument()
    expect(screen.queryByText(NACL_EXPLANATION)).not.toBeInTheDocument()
  })

  it('shows a scene load error from the server', async () => {
    vi.stubGlobal(
      'fetch',
      stubLabFetch({
        sceneError: { error: 'Login required', code: 'unauthenticated', status: 401 },
      }),
    )

    render(<LabBench />)

    expect(await screen.findByRole('alert')).toHaveTextContent('Login required')
    expect(screen.queryByRole('button', { name: 'Spoon' })).not.toBeInTheDocument()
  })

  it('reset posts CSRF reset action and restores pure water from the server scene', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Spoon' })

    fireEvent.click(screen.getByRole('button', { name: 'Spoon' }))
    fireEvent.click(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' }))
    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'nacl')
    })
    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))
    await waitFor(() => {
      expect(screen.getByRole('status')).toHaveTextContent(NACL_EXPLANATION)
    })

    // Put the spoon away so idle inspect works, then confirm ions are present.
    fireEvent.click(screen.getByRole('button', { name: 'Spoon' }))
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'none')
    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))
    expect(await screen.findByRole('dialog')).toHaveTextContent('Na+ (aq)')

    fireEvent.click(screen.getByRole('button', { name: 'Reset lab' }))

    await waitFor(() => {
      expect(screen.getByRole('status')).toHaveTextContent('Lab reset to the starting bench.')
    })
    expect(lastActionInit(fetchMock)).toEqual(
      expect.objectContaining({
        method: 'POST',
        credentials: 'include',
        headers: expect.objectContaining({
          'content-type': 'application/json',
          'X-CSRF-Token': 'tok-123',
        }),
        body: JSON.stringify({ type: 'reset' }),
      }),
    )
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'none')
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))
    const panel = await screen.findByRole('dialog', { name: 'Contents of Water' })
    expect(panel).toHaveTextContent('H2O (l)')
    expect(panel).not.toHaveTextContent('Na+')
    expect(panel).not.toHaveTextContent('(aq)')
    expect(panel.querySelectorAll('sup')).toHaveLength(0)
  })
})
