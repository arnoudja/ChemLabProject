/** @vitest-environment jsdom */
import '@testing-library/jest-dom/vitest'
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  LabBench,
} from './LabBench'
import { clearCsrfTokenCache } from '../lib/api'
import {
  filledScene,
  jsonResponse,
  stubLabFetch,
  lastActionInit,
  expectCsrfLabAction,
} from './labBenchTestHelpers'

describe('LabBench pipette / burner', () => {
  beforeEach(() => {
    clearCsrfTokenCache()
  })

  afterEach(() => {
    cleanup()
    vi.unstubAllGlobals()
    vi.useRealTimers()
  })

  it('pipette transfers 1 ml from the water beaker into the dish and back', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })

    fireEvent.click(screen.getByRole('button', { name: 'Pipette' }))
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'pipette')

    fireEvent.click(screen.getByRole('button', { name: 'Beaker' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'use_tool',
        tool_item_id: 'pipette-1',
        target_item_id: 'beaker-water',
      })
    })
    expect(document.querySelector('[data-water-fill]')).toHaveAttribute('data-water-fill', '0.99')

    fireEvent.click(screen.getByRole('button', { name: 'Evaporation dish' }))
    await waitFor(() => {
      expect(JSON.parse(String(lastActionInit(fetchMock)?.body))).toEqual({
        type: 'use_tool',
        tool_item_id: 'pipette-1',
        target_item_id: 'dish-1',
      })
    })
    expect(document.querySelector('[data-dish-fill]')).toHaveAttribute('data-dish-fill', '0.04')

    // The dish needs PIPETTE_MIN_SOURCE_ML before the pipette may draw from it again.
    for (let i = 0; i < 2; i++) {
      fireEvent.click(screen.getByRole('button', { name: 'Beaker' }))
      await waitFor(() => {
        expect(document.querySelector('[data-pipette-filled="true"]')).not.toBeNull()
      })
      fireEvent.click(screen.getByRole('button', { name: 'Evaporation dish' }))
      await waitFor(() => {
        expect(document.querySelector('[data-pipette-filled="true"]')).toBeNull()
      })
    }
    expect(document.querySelector('[data-dish-fill]')).toHaveAttribute('data-dish-fill', '0.12')

    fireEvent.click(screen.getByRole('button', { name: 'Evaporation dish' }))
    await waitFor(() => {
      expect(document.querySelector('[data-pipette-filled="true"]')).not.toBeNull()
    })
    expect(document.querySelector('[data-dish-fill]')).toHaveAttribute('data-dish-fill', '0.08')

    fireEvent.click(screen.getByRole('button', { name: 'Beaker' }))
    await waitFor(() => {
      expect(JSON.parse(String(lastActionInit(fetchMock)?.body))).toEqual({
        type: 'use_tool',
        tool_item_id: 'pipette-1',
        target_item_id: 'beaker-water',
      })
    })
    expect(document.querySelector('[data-water-fill]')).toHaveAttribute('data-water-fill', '0.99')
  })

  it('reports no fluid and not enough fluid when the pipette source is below 3 ml', async () => {
    const lowScene = filledScene()
    const fetchMock = stubLabFetch({ scene: lowScene })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })
    fireEvent.click(screen.getByRole('button', { name: 'Pipette' }))

    fireEvent.click(screen.getByRole('button', { name: 'Evaporation dish' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('No fluid available.')
    expect(document.querySelector('[data-pipette-filled="true"]')).toBeNull()

    // One aliquot in the dish is fluid, but still short of the 3.00 ml the pipette needs.
    fireEvent.click(screen.getByRole('button', { name: 'Beaker' }))
    await waitFor(() => {
      expect(document.querySelector('[data-pipette-filled="true"]')).not.toBeNull()
    })
    fireEvent.click(screen.getByRole('button', { name: 'Evaporation dish' }))
    await waitFor(() => {
      expect(document.querySelector('[data-pipette-filled="true"]')).toBeNull()
    })

    fireEvent.click(screen.getByRole('button', { name: 'Evaporation dish' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('Not enough fluid available.')
    expect(document.querySelector('[data-dish-fill]')).toHaveAttribute('data-dish-fill', '0.04')
    expect(document.querySelector('[data-pipette-filled="true"]')).toBeNull()
  })

  it('idle burner click toggles the burner after the dish has liquid', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Burner' })

    fireEvent.click(screen.getByRole('button', { name: 'Pipette' }))
    fireEvent.click(screen.getByRole('button', { name: 'Beaker' }))
    await waitFor(() => {
      expect(JSON.parse(String(lastActionInit(fetchMock)?.body)).type).toBe('use_tool')
    })
    fireEvent.click(screen.getByRole('button', { name: 'Evaporation dish' }))
    await waitFor(() => {
      expect(JSON.parse(String(lastActionInit(fetchMock)?.body)).target_item_id).toBe('dish-1')
    })
    fireEvent.click(screen.getByRole('button', { name: 'Pipette' }))
    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'none')
    })

    fireEvent.click(screen.getByRole('button', { name: 'Burner' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'toggle_burner',
        burner_item_id: 'burner-1',
      })
    })
    expect(screen.getByRole('button', { name: 'Burner' })).toHaveAttribute('aria-pressed', 'true')
    expect(document.querySelector('[data-burner-on]')).toHaveAttribute('data-burner-on', 'true')
  })

  it('does not toggle the burner while the pipette is selected', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })
    fireEvent.click(screen.getByRole('button', { name: 'Pipette' }))
    fireEvent.click(screen.getByRole('button', { name: 'Burner' }))

    expect(fetchMock).not.toHaveBeenCalledWith(
      '/api/lab/action',
      expect.objectContaining({
        body: JSON.stringify({ type: 'toggle_burner', burner_item_id: 'burner-1' }),
      }),
    )
  })

  it('polls the lab scene while the burner is on', async () => {
    vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval'] })
    const lit = filledScene()
    const dish = lit.items.find((item) => item.id === 'dish-1')!
    dish.properties.composition = [
      {
        substance_id: 'water',
        phase: 'liquid',
        amount_ml: 1,
        amount_scoop: null,
        amount_g: null,
        amount_mol: null,
      },
    ]
    dish.properties.fill_ml = 1
    lit.items.find((item) => item.id === 'burner-1')!.properties.on = true
    const fetchMock = stubLabFetch({ scene: lit })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Burner' })
    const getsBefore = fetchMock.mock.calls.filter(([url]) => String(url) === '/api/lab/scene').length

    await vi.advanceTimersByTimeAsync(900)
    const getsAfter = fetchMock.mock.calls.filter(([url]) => String(url) === '/api/lab/scene').length
    expect(getsAfter).toBeGreaterThan(getsBefore)
    vi.useRealTimers()
  })

  it('polls the lab scene while a vessel is off ambient after the burner is off', async () => {
    vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval'] })
    const cooling = filledScene()
    cooling.items.find((item) => item.id === 'burner-1')!.properties.on = false
    cooling.items.find((item) => item.id === 'dish-1')!.properties.temperature_c = 55
    const fetchMock = stubLabFetch({ scene: cooling })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Burner' })
    const getsBefore = fetchMock.mock.calls.filter(([url]) => String(url) === '/api/lab/scene').length

    await vi.advanceTimersByTimeAsync(900)
    const getsAfter = fetchMock.mock.calls.filter(([url]) => String(url) === '/api/lab/scene').length
    expect(getsAfter).toBeGreaterThan(getsBefore)
    vi.useRealTimers()
  })

  it('returns a filled pipette to the last source on put-away', async () => {
    const fetchMock = stubLabFetch()
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
    expect(document.querySelector('[data-water-fill]')).toHaveAttribute('data-water-fill', '0.99')

    fireEvent.click(screen.getByRole('button', { name: 'Pipette' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'put_away',
        tool_item_id: 'pipette-1',
      })
    })
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'none')
    expect(document.querySelector('[data-water-fill]')).toHaveAttribute('data-water-fill', '1.00')
  })

  it('shows a pipette transfer error from the server', async () => {
    vi.stubGlobal(
      'fetch',
      stubLabFetch({
        actionHandler: () => ({ error: 'Dish is full', code: 'invalid_action', status: 400 }),
      }),
    )

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })
    fireEvent.click(screen.getByRole('button', { name: 'Pipette' }))
    fireEvent.click(screen.getByRole('button', { name: 'Beaker' }))

    expect(await screen.findByRole('alert')).toHaveTextContent('Dish is full')
  })

  it('shows a burner toggle error from the server', async () => {
    vi.stubGlobal(
      'fetch',
      stubLabFetch({
        actionHandler: (action) => {
          if (action.type === 'toggle_burner') {
            return { error: 'Burner jammed', code: 'invalid_action', status: 400 }
          }
          return { error: 'unexpected', code: 'invalid_action', status: 400 }
        },
      }),
    )

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Burner' })
    fireEvent.click(screen.getByRole('button', { name: 'Burner' }))

    expect(await screen.findByRole('alert')).toHaveTextContent('Burner jammed')
  })

  it('does not surface poll errors while the burner is on', async () => {
    vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval'] })
    const lit = filledScene()
    const dish = lit.items.find((item) => item.id === 'dish-1')!
    dish.properties.composition = [
      {
        substance_id: 'water',
        phase: 'liquid',
        amount_ml: 1,
        amount_scoop: null,
        amount_g: null,
        amount_mol: null,
      },
    ]
    dish.properties.fill_ml = 1
    lit.items.find((item) => item.id === 'burner-1')!.properties.on = true

    let sceneGets = 0
    const fetchMock = vi.fn(async (input: RequestInfo | URL) => {
      const url = String(input)
      if (url === '/api/auth/csrf') {
        return jsonResponse({ csrf_token: 'tok-123' })
      }
      if (url === '/api/lab/scene') {
        sceneGets += 1
        if (sceneGets > 1) {
          return jsonResponse({ error: 'temporary', code: 'unavailable' }, 503)
        }
        return jsonResponse(lit)
      }
      return jsonResponse({ error: 'unexpected', code: 'invalid_action' }, 400)
    })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Burner' })
    await vi.advanceTimersByTimeAsync(900)
    expect(sceneGets).toBeGreaterThan(1)
    expect(screen.queryByRole('alert')).not.toBeInTheDocument()
    vi.useRealTimers()
  })

  it('pipette on distilled water posts use_tool for 1 ml', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })
    fireEvent.click(screen.getByRole('button', { name: 'Pipette' }))
    fireEvent.click(screen.getByRole('button', { name: 'Distilled water (H2O)' }))
    await waitFor(() => {
      expectCsrfLabAction(fetchMock, {
        type: 'use_tool',
        tool_item_id: 'pipette-1',
        target_item_id: 'beaker-h2o',
      })
    })
  })

})
