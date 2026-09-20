/** @vitest-environment jsdom */
import '@testing-library/jest-dom/vitest'
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  LabBench,
  SPOON_SCOOP_MASS_G,
  STOCK_FULL_MASS_G,
  WATER_FULL_ML,
  DISTILLED_WATER_CAPACITY_ML,
  DISH_CAPACITY_ML,
  FILTRATE_CAPACITY_ML,
  dishFillRatio,
  distilledWaterFillRatio,
  filtrateFillRatio,
  stockFillRatio,
  waterFillRatio,
} from './LabBench'
import { clearCsrfTokenCache } from '../lib/api'
import { optionalArray } from '../lib/scene'
import {
  NACL_EXPLANATION,
  SAND_EXPLANATION,
  CACL2_DELTA_H_SOLUTION_J_PER_MOL,
  WATER_SPECIFIC_HEAT_J_PER_G_K,
  CACL2_MOLAR_MASS_G_PER_MOL,
  CACL2_EXPLANATION,
  initialScene,
  cloneScene,
  filledScene,
  withDryDishSolids,
  withScoop,
  afterNaclPour,
  pipetteIsFull,
  applyTongsPickUp,
  applyTongsPour,
  applyPipetteUse,
  applyPipettePutAway,
  stubLabFetch,
  lastActionInit,
  expectCsrfLabAction,
  showStockInCarousel,
  clickStock,
  clickSpoon,
  clickTongs,
} from './labBenchTestHelpers'

describe('LabBench spoon', () => {
  beforeEach(() => {
    clearCsrfTokenCache()
  })

  afterEach(() => {
    cleanup()
    vi.unstubAllGlobals()
    vi.useRealTimers()
  })

  it('clicking the spoon holds it as the cursor tool', async () => {
    vi.stubGlobal('fetch', stubLabFetch())

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })

    clickSpoon()

    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'spoon')
    expect(screen.getByRole('button', { name: 'Spoon' })).toHaveAttribute('aria-pressed', 'true')
  })

  it('does not scoop or post an action without a held spoon', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })

    clickStock('Sodium chloride (NaCl)')
    fireEvent.click(screen.getByRole('button', { name: 'Beaker' }))

    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'none')
    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/action', expect.anything())
  })

  it('shows full stock fill from server amount_g and lowers after scoop', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })

    showStockInCarousel('Sodium chloride (NaCl)')
    expect(document.querySelector('[data-stock-solid="nacl"]')).toHaveAttribute('data-stock-fill', '1.00')
    expect(stockFillRatio(STOCK_FULL_MASS_G)).toBe(1)
    expect(stockFillRatio(STOCK_FULL_MASS_G - SPOON_SCOOP_MASS_G)).toBeCloseTo(0.9)

    showStockInCarousel('Sand')
    expect(document.querySelector('[data-stock-solid="sand"]')).toHaveAttribute('data-stock-fill', '1.00')
    showStockInCarousel('Sodium chloride (NaCl)')

    clickSpoon()
    clickStock('Sodium chloride (NaCl)')

    await waitFor(() => {
      expect(document.querySelector('[data-stock-solid="nacl"]')).toHaveAttribute('data-stock-fill', '0.90')
    })
    showStockInCarousel('Sand')
    expect(document.querySelector('[data-stock-solid="sand"]')).toHaveAttribute('data-stock-fill', '1.00')
  })

  it('draws no salt pile when leftover stock grams display as empty', async () => {
    const leftoverG = 2.7755575615628914e-16
    const scene = initialScene()
    const nacl = scene.items.find((item) => item.id === 'beaker-nacl')!
    const solid = optionalArray(nacl.properties.composition).find(
      (entry) => entry.substance_id === 'nacl' && entry.phase === 'solid',
    )!
    solid.amount_scoop = 0
    solid.amount_g = leftoverG

    const fetchMock = stubLabFetch({ scene })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })
    showStockInCarousel('Sodium chloride (NaCl)')

    expect(leftoverG.toFixed(2)).toBe('0.00')
    expect(stockFillRatio(leftoverG)).toBe(0)
    const svg = document.querySelector('[data-stock-solid="nacl"]')
    expect(svg).toHaveAttribute('data-stock-fill', '0.00')
    expect(svg?.querySelector('path[fill="#F4FBFF"]')).not.toBeInTheDocument()
  })

  it('shows water fill from server amount_ml and updates after pour and reset', async () => {
    const fetchMock = stubLabFetch({
      actionHandler: (action, scene) => {
        if (action.type === 'use_tool' && action.target_item_id === 'beaker-nacl') {
          return withScoop(scene, 'nacl')
        }
        if (action.type === 'pour') {
          const next = afterNaclPour(scene)
          const water = next.items.find((item) => item.id === 'beaker-water')!
          const liquid = optionalArray(water.properties.composition).find(
            (entry) => entry.substance_id === 'water' && entry.phase === 'liquid',
          )
          // Exercise fill tracking when the server reports a different volume after pour.
          if (liquid) liquid.amount_ml = WATER_FULL_ML / 2
          return next
        }
        if (action.type === 'reset') {
          const next = initialScene()
          next.lab_id = scene.lab_id
          next.version = scene.version + 1
          next.last_events = [{ kind: 'reset', message: 'Lab reset to the starting bench.' }]
          return next
        }
        return { error: 'invalid action', code: 'invalid_action', status: 400 }
      },
    })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })

    expect(document.querySelector('[data-water-fill]')).toHaveAttribute('data-water-fill', '1.00')
    expect(waterFillRatio(WATER_FULL_ML)).toBe(1)
    expect(waterFillRatio(WATER_FULL_ML / 2)).toBeCloseTo(0.5)
    expect(waterFillRatio(0)).toBe(0)
    expect(waterFillRatio(null)).toBe(0)
    expect(distilledWaterFillRatio(DISTILLED_WATER_CAPACITY_ML)).toBe(1)
    expect(distilledWaterFillRatio(50)).toBeCloseTo(0.5)
    expect(distilledWaterFillRatio(0)).toBe(0)
    expect(distilledWaterFillRatio(null)).toBe(0)
    expect(dishFillRatio(DISH_CAPACITY_ML)).toBe(1)
    expect(dishFillRatio(DISH_CAPACITY_ML + 10)).toBe(1)
    expect(dishFillRatio(1)).toBeCloseTo(0.04)
    expect(dishFillRatio(0)).toBe(0)
    expect(dishFillRatio(null)).toBe(0)
    expect(filtrateFillRatio(FILTRATE_CAPACITY_ML)).toBe(1)
    expect(filtrateFillRatio(125)).toBeCloseTo(0.5)
    expect(filtrateFillRatio(0)).toBe(0)
    expect(filtrateFillRatio(null)).toBe(0)

    clickSpoon()
    clickStock('Sodium chloride (NaCl)')
    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'nacl')
    })
    // Scoop does not change water volume — fill stays full from server amount_ml.
    expect(document.querySelector('[data-water-fill]')).toHaveAttribute('data-water-fill', '1.00')

    fireEvent.click(screen.getByRole('button', { name: 'Beaker' }))
    await waitFor(() => {
      expect(document.querySelector('[data-water-fill]')).toHaveAttribute('data-water-fill', '0.50')
    })
    expect(screen.getByRole('status')).toHaveTextContent(NACL_EXPLANATION)

    fireEvent.click(screen.getByRole('button', { name: 'Reset lab' }))
    await waitFor(() => {
      expect(document.querySelector('[data-water-fill]')).toHaveAttribute('data-water-fill', '0.00')
    })
  })

  it('renders half-full water when the initial scene reports half amount_ml', async () => {
    const scene = filledScene()
    const water = scene.items.find((item) => item.id === 'beaker-water')!
    const liquid = optionalArray(water.properties.composition).find(
      (entry) => entry.substance_id === 'water' && entry.phase === 'liquid',
    )!
    liquid.amount_ml = WATER_FULL_ML / 2
    vi.stubGlobal('fetch', stubLabFetch({ scene }))

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Beaker' })

    expect(document.querySelector('[data-water-fill]')).toHaveAttribute('data-water-fill', '0.50')
  })

  it('puts salt back into the salt stock and restores fill from the server scene', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })

    clickSpoon()
    clickStock('Sodium chloride (NaCl)')
    await waitFor(() => {
      expect(document.querySelector('[data-stock-solid="nacl"]')).toHaveAttribute('data-stock-fill', '0.90')
    })

    clickStock('Sodium chloride (NaCl)')
    await waitFor(() => {
      expect(document.querySelector('[data-stock-solid="nacl"]')).toHaveAttribute('data-stock-fill', '1.00')
    })
    expect(screen.getByRole('status')).toHaveTextContent('Returned')
    // Spoon holding cleared — cursor tool stays spoon without solid fill.
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'spoon')
  })

  it('rejects putting salt into the sand stock beaker', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })

    clickSpoon()
    clickStock('Sodium chloride (NaCl)')
    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'nacl')
    })

    showStockInCarousel('Sand')
    fireEvent.click(screen.getByRole('button', { name: 'Sand' }))
    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument()
    })
    expect(document.querySelector('[data-stock-solid="sand"]')).toHaveAttribute('data-stock-fill', '1.00')
    showStockInCarousel('Sodium chloride (NaCl)')
    expect(document.querySelector('[data-stock-solid="nacl"]')).toHaveAttribute('data-stock-fill', '0.90')
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'nacl')
  })

  it('spoon then nacl then water posts use_tool then pour with CSRF and shows server events', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })

    clickSpoon()
    clickStock('Sodium chloride (NaCl)')

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

    fireEvent.click(screen.getByRole('button', { name: 'Beaker' }))

    await waitFor(() => {
      expect(screen.getByRole('status')).toHaveTextContent(NACL_EXPLANATION)
    })
    expect(screen.getByRole('status')).toHaveTextContent('Dissolved')
    expect(document.querySelector('[data-bench-status="dissolved"]')).toHaveTextContent(NACL_EXPLANATION)
    expect(document.querySelector('[data-dissolve-cue="dissolved"]')).toBeTruthy()
    expect(document.querySelector('[data-water-aqueous="true"]')).toBeTruthy()
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

  it('spoon then cacl2 then water dissolves with server ions, heating, and molarity on inspect', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })

    showStockInCarousel('Calcium chloride (CaCl2)')
    expect(document.querySelector('[data-stock-solid="cacl2"]')).toHaveAttribute('data-stock-fill', '1.00')

    clickSpoon()
    fireEvent.click(screen.getByRole('button', { name: 'Calcium chloride (CaCl2)' }))
    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'cacl2')
    })
    expect(document.querySelector('[data-stock-solid="cacl2"]')).toHaveAttribute('data-stock-fill', '0.90')

    fireEvent.click(screen.getByRole('button', { name: 'Beaker' }))
    await waitFor(() => {
      expect(screen.getByRole('status')).toHaveTextContent(CACL2_EXPLANATION)
    })
    expect(screen.getByRole('status')).toHaveTextContent('Dissolved')
    expect(document.querySelector('[data-water-aqueous="true"]')).toBeTruthy()

    clickSpoon()
    fireEvent.click(screen.getByRole('button', { name: 'Beaker' }))
    const panel = await screen.findByRole('dialog', { name: 'Contents of Beaker' })
    expect(panel).toHaveTextContent('Ca')
    expect(panel).toHaveTextContent('2+')
    expect(panel).toHaveTextContent('(aq)')
    const moles = SPOON_SCOOP_MASS_G / CACL2_MOLAR_MASS_G_PER_MOL
    const expectedCaM = moles / 0.2
    const expectedClM = (2 * moles) / 0.2
    expect(panel).toHaveTextContent(`${expectedCaM.toPrecision(3)} M`)
    expect(panel).toHaveTextContent('Cl')
    expect(panel).toHaveTextContent(`${expectedClM.toPrecision(3)} M`)
    const expectedT =
      20 -
      (moles * CACL2_DELTA_H_SOLUTION_J_PER_MOL) / (200 * WATER_SPECIFIC_HEAT_J_PER_G_K)
    expect(panel).toHaveTextContent(`Temperature: ${expectedT.toFixed(2)}°C`)
    expect(expectedT).toBeGreaterThan(20)
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
    await screen.findByRole('button', { name: 'Pipette' })

    showStockInCarousel('Sand')
    clickSpoon()
    fireEvent.click(screen.getByRole('button', { name: 'Sand' }))
    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'sand')
    })
    fireEvent.click(screen.getByRole('button', { name: 'Beaker' }))

    await waitFor(() => {
      expect(screen.getByRole('status')).toHaveTextContent('Unexpected server sentence for sand.')
    })
    expect(screen.getByRole('status')).toHaveTextContent('Dissolved')
    expect(document.querySelector('[data-bench-status="dissolved"]')).toHaveTextContent(
      'Unexpected server sentence for sand.',
    )
    expect(screen.queryByText(SAND_EXPLANATION)).not.toBeInTheDocument()
    // No undissolved solid in the surprising payload → no leftover grains invented.
    expect(screen.getByRole('button', { name: 'Beaker' }).querySelectorAll('circle')).toHaveLength(0)
  })

  it('sand pour shows the server did-not-dissolve sentence and leftover grains from the scene', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })

    showStockInCarousel('Sand')
    clickSpoon()
    fireEvent.click(screen.getByRole('button', { name: 'Sand' }))
    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'sand')
    })
    fireEvent.click(screen.getByRole('button', { name: 'Beaker' }))

    await waitFor(() => {
      expect(screen.getByRole('status')).toHaveTextContent(SAND_EXPLANATION)
    })
    expect(screen.getByRole('status')).toHaveTextContent('Did not dissolve')
    expect(document.querySelector('[data-bench-status="did_not_dissolve"]')).toHaveTextContent(
      SAND_EXPLANATION,
    )
    expect(document.querySelector('[data-dissolve-cue="did_not_dissolve"]')).toBeTruthy()
    expect(document.querySelector('[data-water-aqueous="false"]')).toBeTruthy()
    // Leftover grains are SVG circles rendered only because the server put solid sand in water.
    expect(
      screen.getByRole('button', { name: 'Beaker' }).querySelectorAll('circle').length,
    ).toBeGreaterThan(0)
  })

  it('empty spoon on water does not post a pour action', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })

    clickSpoon()
    fireEvent.click(screen.getByRole('button', { name: 'Beaker' }))

    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/action', expect.anything())
    expect(screen.queryByRole('status')).not.toBeInTheDocument()
  })

  it.each([
    ['nacl', 'Sodium chloride (NaCl)'] as const,
    ['sand', 'Sand'] as const,
    ['cacl2', 'Calcium chloride (CaCl2)'] as const,
  ])(
    'scoop %s then put spoon away restores stock and empties the spoon',
    async (substance, label) => {
      const fetchMock = stubLabFetch()
      vi.stubGlobal('fetch', fetchMock)

      render(<LabBench />)
      await screen.findByRole('button', { name: 'Pipette' })

      showStockInCarousel(label)
      clickSpoon()
      fireEvent.click(screen.getByRole('button', { name: label }))
      await waitFor(() => {
        expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute(
          'data-tool',
          substance,
        )
      })
      expect(document.querySelector(`[data-stock-solid="${substance}"]`)).toHaveAttribute(
        'data-stock-fill',
        '0.90',
      )

      clickSpoon()

      await waitFor(() => {
        expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'none')
      })
      expect(document.querySelector(`[data-stock-solid="${substance}"]`)).toHaveAttribute(
        'data-stock-fill',
        '1.00',
      )
      expect(screen.getByRole('status')).toHaveTextContent(`Returned ${substance}.`)
      expect(JSON.parse(String(lastActionInit(fetchMock)?.body))).toEqual({
        type: 'put_away',
        tool_item_id: 'spoon-1',
      })
    },
  )

  it('empty spoon put-away is a no-op with no server action', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })

    clickSpoon()
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'spoon')

    clickSpoon()
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'none')
    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/action', expect.anything())
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
    await screen.findByRole('button', { name: 'Pipette' })

    clickSpoon()
    clickStock('Sodium chloride (NaCl)')
    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'nacl')
    })
    fireEvent.click(screen.getByRole('button', { name: 'Beaker' }))

    expect(await screen.findByRole('alert')).toHaveTextContent('Login required')
    expect(screen.queryByText(/Server outcome:/)).not.toBeInTheDocument()
    expect(screen.queryByText(NACL_EXPLANATION)).not.toBeInTheDocument()
  })

  it('selecting the spoon puts an empty pipette away without a server call', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })
    fireEvent.click(screen.getByRole('button', { name: 'Pipette' }))
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'pipette')

    clickSpoon()
    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'spoon')
    })
    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/action', expect.anything())
  })

  it('empty spoon on a dry solids dish posts use_tool and holds the scoop', async () => {
    const fetchMock = stubLabFetch({
      scene: withDryDishSolids(initialScene(), [
        { substance_id: 'nacl', amount_g: 0.6 },
        { substance_id: 'cacl2', amount_g: 0.4 },
      ]),
    })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Evaporation dish' })

    clickSpoon()
    fireEvent.click(screen.getByRole('button', { name: 'Evaporation dish' }))

    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'use_tool',
        tool_item_id: 'spoon-1',
        target_item_id: 'dish-1',
      })
    })
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'nacl')
    expect(screen.getByRole('status')).toHaveTextContent('Scooped solids from the dish.')
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
  })

  it('water click dumps dish solids with pour and CSRF', async () => {
    const fetchMock = stubLabFetch({
      scene: withDryDishSolids(initialScene(), [{ substance_id: 'nacl', amount_g: 0.5 }]),
    })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Evaporation dish' })

    clickSpoon()
    fireEvent.click(screen.getByRole('button', { name: 'Evaporation dish' }))
    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'nacl')
    })

    fireEvent.click(screen.getByRole('button', { name: 'Beaker' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'pour',
        source_item_id: 'spoon-1',
        target_item_id: 'beaker-water',
      })
    })
    expect(screen.getByRole('status')).toHaveTextContent(NACL_EXPLANATION)
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'spoon')
  })

  it('matching stock is a target for a single-species dish scoop', async () => {
    const fetchMock = stubLabFetch({
      scene: withDryDishSolids(initialScene(), [{ substance_id: 'nacl', amount_g: 0.5 }]),
    })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Evaporation dish' })

    clickSpoon()
    fireEvent.click(screen.getByRole('button', { name: 'Evaporation dish' }))
    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'nacl')
    })

    clickStock('Sodium chloride (NaCl)')
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'use_tool',
        tool_item_id: 'spoon-1',
        target_item_id: 'beaker-nacl',
      })
    })
    expect(screen.getByRole('status')).toHaveTextContent('Returned')
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'spoon')
  })

  it('mixed dish scoop does not post to a stock', async () => {
    const fetchMock = stubLabFetch({
      scene: withDryDishSolids(initialScene(), [
        { substance_id: 'nacl', amount_g: 0.6 },
        { substance_id: 'cacl2', amount_g: 0.4 },
      ]),
    })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Evaporation dish' })

    clickSpoon()
    fireEvent.click(screen.getByRole('button', { name: 'Evaporation dish' }))
    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'nacl')
    })
    const callsAfterScoop = fetchMock.mock.calls.filter(([url]) => String(url) === '/api/lab/action').length

    clickStock('Sodium chloride (NaCl)')
    expect(fetchMock.mock.calls.filter(([url]) => String(url) === '/api/lab/action')).toHaveLength(
      callsAfterScoop,
    )
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'nacl')
  })

  it('water is a pour target when holding mixed dish solids', async () => {
    const fetchMock = stubLabFetch({
      scene: withDryDishSolids(initialScene(), [
        { substance_id: 'nacl', amount_g: 0.6 },
        { substance_id: 'cacl2', amount_g: 0.4 },
      ]),
    })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Evaporation dish' })

    clickSpoon()
    fireEvent.click(screen.getByRole('button', { name: 'Evaporation dish' }))
    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'nacl')
    })

    fireEvent.click(screen.getByRole('button', { name: 'Beaker' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'pour',
        source_item_id: 'spoon-1',
        target_item_id: 'beaker-water',
      })
    })
  })

  it('put-away returns a dish scoop to the dish', async () => {
    const fetchMock = stubLabFetch({
      scene: withDryDishSolids(initialScene(), [
        { substance_id: 'nacl', amount_g: 0.6 },
        { substance_id: 'cacl2', amount_g: 0.4 },
      ]),
    })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Evaporation dish' })

    clickSpoon()
    fireEvent.click(screen.getByRole('button', { name: 'Evaporation dish' }))
    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'nacl')
    })

    clickSpoon()
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'put_away',
        tool_item_id: 'spoon-1',
      })
    })
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'none')
    expect(screen.getByRole('status')).toHaveTextContent('Returned solids to the dish.')
    showStockInCarousel('Sodium chloride (NaCl)')
    expect(document.querySelector('[data-stock-solid="nacl"]')).toHaveAttribute('data-stock-fill', '1.00')
  })

  it('spoon on distilled water does not post', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })
    clickSpoon()
    fireEvent.click(screen.getByRole('button', { name: 'Distilled water (H2O)' }))
    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/action', expect.anything())
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'spoon')
  })

  it('shows a server error and stays as-is when put-back into distilled water is full or impure', async () => {
    const fetchMock = stubLabFetch({
      actionHandler: (action, scene) => {
        if (action.type === 'put_away' && action.tool_item_id === 'pipette-1') {
          return applyPipettePutAway(scene)
        }
        if (action.type === 'use_tool' && action.tool_item_id === 'pipette-1') {
          if (action.target_item_id === 'beaker-h2o' && pipetteIsFull(scene)) {
            return { error: 'Stock is full', code: 'invalid_action', status: 400 }
          }
          return applyPipetteUse(scene, action.target_item_id)
        }
        if (action.type === 'use_tool' && action.tool_item_id === 'tongs-1') {
          const held = scene.items.find((item) => item.id === 'tongs-1')?.properties.source_item_id
          if (!held) return applyTongsPickUp(scene, action.target_item_id)
          if (action.target_item_id === 'beaker-h2o') {
            return { error: 'Only pure water', code: 'invalid_action', status: 400 }
          }
          return applyTongsPour(scene, action.target_item_id)
        }
        return { error: 'unexpected', code: 'invalid_action', status: 400 }
      },
    })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })
    fireEvent.click(screen.getByRole('button', { name: 'Pipette' }))
    fireEvent.click(screen.getByRole('button', { name: 'Beaker' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'use_tool',
        tool_item_id: 'pipette-1',
        target_item_id: 'beaker-water',
      })
    })
    fireEvent.click(screen.getByRole('button', { name: 'Distilled water (H2O)' }))
    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent('Stock is full')
    })
    expect(document.querySelector('[data-pipette-filled="true"]')).not.toBeNull()
    expect(document.querySelector('[data-h2o-fill]')).toHaveAttribute('data-h2o-fill', '1.00')

    fireEvent.click(screen.getByRole('button', { name: 'Pipette' }))
    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'none')
    })

    clickTongs()
    fireEvent.click(screen.getByRole('button', { name: 'Beaker' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'use_tool',
        tool_item_id: 'tongs-1',
        target_item_id: 'beaker-water',
      })
    })
    fireEvent.click(screen.getByRole('button', { name: 'Distilled water (H2O)' }))
    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent('Only pure water')
    })
    expect(screen.getByRole('button', { name: 'Beaker' }).querySelector('[data-water-fill]')).toBeNull()
    expect(document.querySelector('[data-h2o-fill]')).toHaveAttribute('data-h2o-fill', '1.00')
  })

})
