/** @vitest-environment jsdom */
import '@testing-library/jest-dom/vitest'
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  LabBench,
  DISH_CAPACITY_ML,
} from './LabBench'
import { clearCsrfTokenCache } from '../lib/api'
import {
  emptyProps,
  initialScene,
  cloneScene,
  filledScene,
  applyTongsPickUp,
  stubLabFetch,
  expectCsrfLabAction,
  clickStock,
  showToolInCarousel,
  clickSpoon,
  clickTongs,
} from './labBenchTestHelpers'

describe('LabBench tongs', () => {
  beforeEach(() => {
    clearCsrfTokenCache()
  })

  afterEach(() => {
    cleanup()
    vi.unstubAllGlobals()
    vi.useRealTimers()
  })

  it('picks up the water beaker with tongs, hides the bench vessel, and pours into the dish', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Beaker' })
    clickTongs()
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'tongs')

    const water = screen.getByRole('button', { name: 'Beaker' })
    fireEvent.mouseMove(screen.getByRole('region', { name: 'Lab bench' }), { clientX: 40, clientY: 40 })
    fireEvent.click(water)
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'use_tool',
        tool_item_id: 'tongs-1',
        target_item_id: 'beaker-water',
      })
    })
    expect(water.querySelector('[data-water-fill]')).toBeNull()
    expect(document.querySelector('.lab-cursor-vessel [data-water-fill]')).not.toBeNull()

    fireEvent.click(screen.getByRole('button', { name: 'Evaporation dish' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'use_tool',
        tool_item_id: 'tongs-1',
        target_item_id: 'dish-1',
      })
    })
    expect(document.querySelector('[data-dish-fill]')).toHaveAttribute('data-dish-fill', '1.00')
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'tongs')
  })

  it('picks up the dish with tongs, turns the burner off, and pours back into water', async () => {
    const lit = filledScene()
    const dish = lit.items.find((item) => item.id === 'dish-1')!
    dish.properties.composition = [
      {
        substance_id: 'water',
        phase: 'liquid',
        amount_ml: DISH_CAPACITY_ML,
        amount_scoop: null,
        amount_g: null,
        amount_mol: null,
      },
    ]
    dish.properties.fill_ml = DISH_CAPACITY_ML
    lit.items.find((item) => item.id === 'burner-1')!.properties.on = true
    const fetchMock = stubLabFetch({ scene: lit })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Burner' })
    expect(screen.getByRole('button', { name: 'Burner' })).toHaveAttribute('aria-pressed', 'true')

    clickTongs()
    fireEvent.click(screen.getByRole('button', { name: 'Evaporation dish' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'use_tool',
        tool_item_id: 'tongs-1',
        target_item_id: 'dish-1',
      })
    })
    expect(screen.getByRole('button', { name: 'Burner' })).toHaveAttribute('aria-pressed', 'false')
    expect(screen.getByRole('button', { name: 'Evaporation dish' }).querySelector('[data-dish-fill]')).toBeNull()
    expect(document.querySelector('.lab-cursor-vessel [data-dish-fill]')).not.toBeNull()

    fireEvent.click(screen.getByRole('button', { name: 'Beaker' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'use_tool',
        tool_item_id: 'tongs-1',
        target_item_id: 'beaker-water',
      })
    })
    expect(document.querySelector('[data-water-fill]')).toHaveAttribute('data-water-fill', '1.00')
  })

  it('put-away and clicking the empty water slot return the held beaker home', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Beaker' })
    clickTongs()
    fireEvent.click(screen.getByRole('button', { name: 'Beaker' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'use_tool',
        tool_item_id: 'tongs-1',
        target_item_id: 'beaker-water',
      })
    })
    expect(screen.getByRole('button', { name: 'Beaker' }).querySelector('[data-water-fill]')).toBeNull()

    fireEvent.click(screen.getByRole('button', { name: 'Beaker' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'put_away',
        tool_item_id: 'tongs-1',
      })
    })
    expect(screen.getByRole('button', { name: 'Beaker' }).querySelector('[data-water-fill]')).not.toBeNull()
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'none')
  })

  it('does not use tongs on the burner', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })
    clickTongs()
    fireEvent.click(screen.getByRole('button', { name: 'Burner' }))
    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/action', expect.anything())
  })

  it('picks up the visible solid stock with tongs, dumps into the Beaker, and puts it away', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Beaker' })
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveTextContent(
      /tongs to lift vessels, filter paper, or solid ingredients/,
    )

    clickTongs()
    fireEvent.mouseMove(screen.getByRole('region', { name: 'Lab bench' }), { clientX: 40, clientY: 40 })
    clickStock('Sodium chloride (NaCl)')
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'use_tool',
        tool_item_id: 'tongs-1',
        target_item_id: 'beaker-nacl',
      })
    })
    expect(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' }).querySelector('[data-stock-solid]')).toBeNull()
    expect(document.querySelector('.lab-cursor-vessel [data-stock-solid="nacl"]')).not.toBeNull()
    expect(screen.queryByRole('button', { name: 'Distilled water (H2O)' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Calcium chloride (CaCl2)' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Sand' })).not.toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: 'Beaker' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'use_tool',
        tool_item_id: 'tongs-1',
        target_item_id: 'beaker-water',
      })
    })
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'tongs')
    expect(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' }).querySelector('[data-stock-solid]')).toBeNull()

    fireEvent.click(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'put_away',
        tool_item_id: 'tongs-1',
      })
    })
    expect(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' }).querySelector('[data-stock-solid="nacl"]')).not.toBeNull()
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'none')
  })

  it('clicking the empty dish slot or the tongs put-away returns the held dish home', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Evaporation dish' })
    clickTongs()
    fireEvent.click(screen.getByRole('button', { name: 'Evaporation dish' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'use_tool',
        tool_item_id: 'tongs-1',
        target_item_id: 'dish-1',
      })
    })

    fireEvent.click(screen.getByRole('button', { name: 'Evaporation dish' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'put_away',
        tool_item_id: 'tongs-1',
      })
    })
    expect(screen.getByRole('button', { name: 'Evaporation dish' }).querySelector('[data-dish-fill]')).not.toBeNull()
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'none')
  })

  it('switching tools puts tongs away first, including a held vessel', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Beaker' })
    clickTongs()
    fireEvent.click(screen.getByRole('button', { name: 'Beaker' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'use_tool',
        tool_item_id: 'tongs-1',
        target_item_id: 'beaker-water',
      })
    })

    clickSpoon()
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'put_away',
        tool_item_id: 'tongs-1',
      })
    })
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'spoon')
    expect(screen.getByRole('button', { name: 'Beaker' }).querySelector('[data-water-fill]')).not.toBeNull()
  })

  it('selecting the pipette puts empty tongs away without a server call', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })
    clickTongs()
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'tongs')

    showToolInCarousel('Pipette')
    fireEvent.click(screen.getByRole('button', { name: 'Pipette' }))
    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'pipette')
    })
    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/action', expect.anything())
  })

  it('picks up water with tongs when the loaded scene predates tongs-1', async () => {
    const stale = initialScene()
    stale.items = stale.items.filter((item) => item.id !== 'tongs-1')
    const fetchMock = stubLabFetch({
      scene: stale,
      actionHandler: (action, scene) => {
        if (action.type !== 'use_tool' || action.tool_item_id !== 'tongs-1') {
          return { error: 'unexpected', code: 'invalid_action', status: 400 }
        }
        const next = cloneScene(scene)
        if (!next.items.some((item) => item.id === 'tongs-1')) {
          next.items.push({
            id: 'tongs-1',
            kind: 'tongs',
            label: 'Tongs',
            location: 'bench',
            properties: { ...emptyProps(), source_item_id: null },
          })
        }
        return applyTongsPickUp(next, action.target_item_id)
      },
    })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Beaker' })
    clickTongs()
    fireEvent.mouseMove(screen.getByRole('region', { name: 'Lab bench' }), { clientX: 40, clientY: 40 })
    fireEvent.click(screen.getByRole('button', { name: 'Beaker' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'use_tool',
        tool_item_id: 'tongs-1',
        target_item_id: 'beaker-water',
      })
    })
    expect(screen.getByRole('button', { name: 'Beaker' }).querySelector('[data-water-fill]')).toBeNull()
    expect(document.querySelector('.lab-cursor-vessel [data-water-fill]')).not.toBeNull()
  })

  it('shows a tongs pour error from the server and stays holding', async () => {
    vi.stubGlobal(
      'fetch',
      stubLabFetch({
        actionHandler: (action, scene) => {
          if (action.type === 'use_tool' && action.tool_item_id === 'tongs-1') {
            const held = scene.items.find((item) => item.id === 'tongs-1')?.properties.source_item_id
            if (!held) return applyTongsPickUp(scene, action.target_item_id)
            return { error: 'Nothing to pour', code: 'empty_holding', status: 400 }
          }
          return { error: 'unexpected', code: 'invalid_action', status: 400 }
        },
      }),
    )

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Beaker' })
    clickTongs()
    fireEvent.click(screen.getByRole('button', { name: 'Beaker' }))
    await screen.findByRole('status')
    fireEvent.click(screen.getByRole('button', { name: 'Evaporation dish' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('Nothing to pour')
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'tongs')
  })

  it('picks up distilled water with tongs, hides the home slot, and pours into the dish', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Distilled water (H2O)' })
    clickTongs()
    fireEvent.mouseMove(screen.getByRole('region', { name: 'Lab bench' }), { clientX: 40, clientY: 40 })
    fireEvent.click(screen.getByRole('button', { name: 'Distilled water (H2O)' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'use_tool',
        tool_item_id: 'tongs-1',
        target_item_id: 'beaker-h2o',
      })
    })
    expect(screen.getByRole('button', { name: 'Distilled water (H2O)' }).querySelector('[data-h2o-fill]')).toBeNull()
    expect(document.querySelector('.lab-cursor-vessel [data-h2o-fill]')).not.toBeNull()

    fireEvent.click(screen.getByRole('button', { name: 'Evaporation dish' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'use_tool',
        tool_item_id: 'tongs-1',
        target_item_id: 'dish-1',
      })
    })
    expect(screen.getByRole('button', { name: 'Distilled water (H2O)' }).querySelector('[data-h2o-fill]')).toBeNull()
  })

  it('clicking the empty distilled-water home slot puts tongs away', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Distilled water (H2O)' })
    clickTongs()
    fireEvent.click(screen.getByRole('button', { name: 'Distilled water (H2O)' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'use_tool',
        tool_item_id: 'tongs-1',
        target_item_id: 'beaker-h2o',
      })
    })

    fireEvent.click(screen.getByRole('button', { name: 'Distilled water (H2O)' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'put_away',
        tool_item_id: 'tongs-1',
      })
    })
    expect(screen.getByRole('button', { name: 'Distilled water (H2O)' }).querySelector('[data-h2o-fill]')).not.toBeNull()
  })

})
