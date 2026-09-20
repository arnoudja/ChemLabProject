/** @vitest-environment jsdom */
import '@testing-library/jest-dom/vitest'
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  LabBench,
} from './LabBench'
import { clearCsrfTokenCache } from '../lib/api'
import {
  initialScene,
  stubLabFetch,
  expectCsrfLabAction,
  clickSpoon,
  clickTongs,
} from './labBenchTestHelpers'

describe('LabBench filter', () => {
  beforeEach(() => {
    clearCsrfTokenCache()
  })

  afterEach(() => {
    cleanup()
    vi.unstubAllGlobals()
    vi.useRealTimers()
  })

  it('picks up filter paper and filtrate separately with empty tongs', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    const filtrate = await screen.findByRole('button', { name: 'Filtrate beaker' })
    expect(filtrate.querySelector('[data-filtrate-fill]')).toHaveAttribute('data-filtrate-fill', '0.00')
    clickTongs()
    fireEvent.mouseMove(screen.getByRole('region', { name: 'Lab bench' }), { clientX: 40, clientY: 40 })
    fireEvent.click(screen.getByRole('button', { name: 'Filter paper' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'use_tool',
        tool_item_id: 'tongs-1',
        target_item_id: 'filter-paper-1',
      })
    })
    expect(screen.getByRole('button', { name: 'Filter paper' }).querySelector('[data-funnel]')).toBeNull()
    expect(document.querySelector('.lab-cursor-vessel [data-funnel]')).not.toBeNull()
    expect(filtrate.querySelector('[data-filtrate-fill]')).not.toBeNull()

    fireEvent.click(screen.getByRole('button', { name: 'Filter paper' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'put_away',
        tool_item_id: 'tongs-1',
      })
    })
    expect(screen.getByRole('button', { name: 'Filter paper' }).querySelector('[data-funnel]')).not.toBeNull()

    clickTongs()
    fireEvent.click(filtrate)
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'use_tool',
        tool_item_id: 'tongs-1',
        target_item_id: 'beaker-filtrate',
      })
    })
    expect(filtrate.querySelector('[data-filtrate-fill]')).toBeNull()
    expect(document.querySelector('.lab-cursor-vessel [data-filtrate-fill]')).not.toBeNull()

    fireEvent.click(filtrate)
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'put_away',
        tool_item_id: 'tongs-1',
      })
    })
    expect(screen.getByRole('button', { name: 'Filtrate beaker' }).querySelector('[data-filtrate-fill]')).not.toBeNull()
  })

  it('filter-pours a held water beaker through the paper into the seated filtrate', async () => {
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
    fireEvent.click(screen.getByRole('button', { name: 'Filter paper' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'use_tool',
        tool_item_id: 'tongs-1',
        target_item_id: 'filter-paper-1',
      })
    })
    expect(document.querySelector('[data-filtrate-fill]')).toHaveAttribute('data-filtrate-fill', '0.80')
    expect(screen.getByRole('button', { name: 'Beaker' }).querySelector('[data-water-fill]')).toBeNull()
  })

  it('pours a held vessel into filtrate without filtering through paper', async () => {
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
    fireEvent.click(screen.getByRole('button', { name: 'Filtrate beaker' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'use_tool',
        tool_item_id: 'tongs-1',
        target_item_id: 'beaker-filtrate',
      })
    })
    expect(document.querySelector('[data-filtrate-fill]')).toHaveAttribute('data-filtrate-fill', '0.80')
  })

  it('scoops paper solids with the spoon and put-away returns them to the paper', async () => {
    const seeded = initialScene()
    const paper = seeded.items.find((item) => item.id === 'filter-paper-1')!
    paper.properties.composition = [
      {
        substance_id: 'sand',
        phase: 'solid',
        amount_ml: null,
        amount_scoop: null,
        amount_g: 0.5,
        amount_mol: null,
      },
    ]
    const fetchMock = stubLabFetch({ scene: seeded })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Filter paper' })
    const paperSvg = document.querySelector('[data-paper-residue="true"]')
    expect(paperSvg).not.toBeNull()
    const apexY = Number(paperSvg!.querySelector('[data-paper-cone]')?.getAttribute('data-apex-y'))
    const stemTopY = Number(paperSvg!.querySelector('[data-funnel-stem]')?.getAttribute('data-top-y'))
    for (const dot of paperSvg!.querySelectorAll('circle')) {
      const cy = Number(dot.getAttribute('cy'))
      expect(cy).toBeLessThanOrEqual(apexY)
      expect(cy).toBeLessThanOrEqual(stemTopY)
    }
    clickSpoon()
    fireEvent.click(screen.getByRole('button', { name: 'Filter paper' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'use_tool',
        tool_item_id: 'spoon-1',
        target_item_id: 'filter-paper-1',
      })
    })
    fireEvent.click(screen.getByRole('button', { name: 'Spoon' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'put_away',
        tool_item_id: 'spoon-1',
      })
    })
  })

  it('pipettes 1 ml from the seated filtrate beaker and does not pipette the paper', async () => {
    const seeded = initialScene()
    const filtrate = seeded.items.find((item) => item.id === 'beaker-filtrate')!
    filtrate.properties.composition = [
      {
        substance_id: 'water',
        phase: 'liquid',
        amount_ml: 10,
        amount_scoop: null,
        amount_g: null,
        amount_mol: null,
      },
    ]
    filtrate.properties.fill_ml = 10
    const fetchMock = stubLabFetch({ scene: seeded })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })
    fireEvent.click(screen.getByRole('button', { name: 'Pipette' }))
    fireEvent.click(screen.getByRole('button', { name: 'Filter paper' }))
    expect(fetchMock.mock.calls.filter(([url]) => String(url) === '/api/lab/action')).toHaveLength(0)

    fireEvent.click(screen.getByRole('button', { name: 'Filtrate beaker' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'use_tool',
        tool_item_id: 'pipette-1',
        target_item_id: 'beaker-filtrate',
      })
    })
    expect(document.querySelector('[data-filtrate-fill]')).toHaveAttribute('data-filtrate-fill', '0.04')
  })

  it('pipettes from filtrate, dumps back into filtrate, and rejects paper', async () => {
    const seeded = initialScene()
    const filtrate = seeded.items.find((item) => item.id === 'beaker-filtrate')!
    filtrate.properties.composition = [
      {
        substance_id: 'water',
        phase: 'liquid',
        amount_ml: 10,
        amount_scoop: null,
        amount_g: null,
        amount_mol: null,
      },
    ]
    filtrate.properties.fill_ml = 10
    const fetchMock = stubLabFetch({ scene: seeded })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })
    fireEvent.click(screen.getByRole('button', { name: 'Pipette' }))
    fireEvent.click(screen.getByRole('button', { name: 'Filtrate beaker' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'use_tool',
        tool_item_id: 'pipette-1',
        target_item_id: 'beaker-filtrate',
      })
    })
    const actionsAfterFill = fetchMock.mock.calls.filter(([url]) => String(url) === '/api/lab/action').length
    fireEvent.click(screen.getByRole('button', { name: 'Filter paper' }))
    expect(fetchMock.mock.calls.filter(([url]) => String(url) === '/api/lab/action')).toHaveLength(
      actionsAfterFill,
    )
    fireEvent.click(screen.getByRole('button', { name: 'Filtrate beaker' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'use_tool',
        tool_item_id: 'pipette-1',
        target_item_id: 'beaker-filtrate',
      })
    })
  })

  it('spoon scoops and deposits dry filtrate solids over CSRF', async () => {
    const seeded = initialScene()
    const filtrate = seeded.items.find((item) => item.id === 'beaker-filtrate')!
    filtrate.properties.composition = [
      {
        substance_id: 'sand',
        phase: 'solid',
        amount_ml: null,
        amount_scoop: null,
        amount_g: 0.5,
        amount_mol: null,
      },
    ]
    const fetchMock = stubLabFetch({ scene: seeded })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Filtrate beaker' })
    clickSpoon()
    fireEvent.click(screen.getByRole('button', { name: 'Filtrate beaker' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'use_tool',
        tool_item_id: 'spoon-1',
        target_item_id: 'beaker-filtrate',
      })
    })
    fireEvent.click(screen.getByRole('button', { name: 'Filter paper' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'use_tool',
        tool_item_id: 'spoon-1',
        target_item_id: 'filter-paper-1',
      })
    })
  })

  it('pours a held filtrate beaker into water with tongs', async () => {
    const seeded = initialScene()
    const filtrate = seeded.items.find((item) => item.id === 'beaker-filtrate')!
    filtrate.properties.composition = [
      {
        substance_id: 'water',
        phase: 'liquid',
        amount_ml: 30,
        amount_scoop: null,
        amount_g: null,
        amount_mol: null,
      },
    ]
    filtrate.properties.fill_ml = 30
    const fetchMock = stubLabFetch({ scene: seeded })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Filtrate beaker' })
    clickTongs()
    fireEvent.click(screen.getByRole('button', { name: 'Filtrate beaker' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'use_tool',
        tool_item_id: 'tongs-1',
        target_item_id: 'beaker-filtrate',
      })
    })
    const actionsAfterPickup = fetchMock.mock.calls.filter(([url]) => String(url) === '/api/lab/action').length
    fireEvent.click(screen.getByRole('button', { name: 'Filter paper' }))
    expect(fetchMock.mock.calls.filter(([url]) => String(url) === '/api/lab/action')).toHaveLength(
      actionsAfterPickup,
    )
    fireEvent.click(screen.getByRole('button', { name: 'Beaker' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'use_tool',
        tool_item_id: 'tongs-1',
        target_item_id: 'beaker-water',
      })
    })
  })
})
