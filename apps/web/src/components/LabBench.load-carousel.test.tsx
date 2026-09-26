/** @vitest-environment jsdom */
import '@testing-library/jest-dom/vitest'
import type { LabScene } from '../generated/contracts'
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  LabBench,
  STOCK_FULL_MASS_G,
  STOCK_FULL_SCOOPS,
} from './LabBench'
import { clearCsrfTokenCache } from '../lib/api'
import {
  NACL_EXPLANATION,
  initialScene,
  stubLabFetch,
  lastActionInit,
  precedesInDocument,
  clickCarousel,
  clickStock,
  clickToolCarousel,
  clickSpoon,
} from './labBenchTestHelpers'

describe('LabBench load / carousel / layout', () => {
  beforeEach(() => {
    clearCsrfTokenCache()
  })

  afterEach(() => {
    cleanup()
    vi.unstubAllGlobals()
    vi.useRealTimers()
  })

  it('loads the server scene and does not call dissolve', async () => {
    const fetchMock = stubLabFetch({ scene: initialScene() })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)

    expect(await screen.findByRole('button', { name: 'Pipette' })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Spoon' })).not.toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Distilled water (H2O)' })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Sodium chloride (NaCl)' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Calcium chloride (CaCl2)' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Sand' })).not.toBeInTheDocument()
    expect(document.querySelector('[data-stock-solid="cacl2"]')).toBeNull()
    expect(document.querySelector('[data-stock-solid="nacl"]')).toBeNull()
    expect(document.querySelector('[data-stock-solid="sand"]')).toBeNull()
    expect(screen.getByRole('button', { name: 'Beaker' })).toBeInTheDocument()
    expect(document.querySelector('[data-water-fill]')).toHaveAttribute('data-water-fill', '0.00')
    const waterLabel = document.querySelector('[data-stock-label="water"]')
    expect(waterLabel?.textContent).toContain('H2O')
    expect(waterLabel?.textContent).toContain('(Water)')
    expect(waterLabel?.textContent).toContain('(distilled water)')
    expect(document.querySelector('[data-h2o-fill]')).toHaveAttribute('data-h2o-fill', '1.00')
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'none')
    expect(fetchMock).toHaveBeenCalledWith('/api/lab/scene', { credentials: 'include' })
    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/dissolve', expect.anything())
    expect(screen.queryByRole('status')).not.toBeInTheDocument()
  })

  it('wraps the ingredient carousel H2O → HCl → NaCl → CaCl2 → SiO2 and hides other stocks from the DOM', async () => {
    vi.stubGlobal('fetch', stubLabFetch())

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Distilled water (H2O)' })
    expect(document.querySelector('[data-h2o-fill]')).toHaveAttribute('data-h2o-fill', '1.00')

    clickCarousel('next')
    expect(screen.getByRole('button', { name: 'Hydrochloric acid (30%)' })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Distilled water (H2O)' })).not.toBeInTheDocument()
    expect(document.querySelector('[data-h2o-fill]')).toBeNull()
    expect(document.querySelector('[data-hcl-fill]')).toHaveAttribute('data-hcl-fill', '1.00')
    const hclLabel = document.querySelector('[data-stock-label="hcl"]')
    expect(hclLabel?.textContent).toContain('HCl')
    expect(hclLabel?.textContent).toContain('(Hydrochloric acid)')
    expect(hclLabel?.textContent).toContain('(30% w/w)')

    clickCarousel('next')
    expect(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Hydrochloric acid (30%)' })).not.toBeInTheDocument()
    expect(document.querySelector('[data-hcl-fill]')).toBeNull()
    const saltLabel = document.querySelector('[data-stock-label="nacl"]')
    expect(saltLabel?.textContent).toContain('NaCl')
    expect(saltLabel?.textContent).toContain('(Sodium chloride)')
    expect(saltLabel?.textContent).toContain('(Table salt)')

    clickCarousel('next')
    expect(screen.getByRole('button', { name: 'Calcium chloride (CaCl2)' })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Sodium chloride (NaCl)' })).not.toBeInTheDocument()
    expect(document.querySelector('[data-stock-solid="nacl"]')).toBeNull()
    const cacl2Label = document.querySelector('[data-stock-label="cacl2"]')
    expect(cacl2Label?.querySelector('sub')?.textContent).toBe('2')
    expect(cacl2Label?.textContent).toContain('(Calcium chloride)')
    expect(cacl2Label?.textContent).toContain('(De-icing salt)')
    expect(document.querySelector('[data-stock-solid="cacl2"]')).toHaveAttribute('data-stock-fill', '1.00')

    clickCarousel('next')
    expect(screen.getByRole('button', { name: 'Sand' })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Calcium chloride (CaCl2)' })).not.toBeInTheDocument()
    const sandLabel = document.querySelector('[data-stock-label="sand"]')
    expect(sandLabel?.querySelector('sub')?.textContent).toBe('2')
    expect(sandLabel?.textContent).toContain('(Silicon dioxide)')
    expect(sandLabel?.textContent).toContain('(Sand)')

    clickCarousel('next')
    expect(screen.getByRole('button', { name: 'Distilled water (H2O)' })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Sand' })).not.toBeInTheDocument()

    clickCarousel('previous')
    expect(screen.getByRole('button', { name: 'Sand' })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Distilled water (H2O)' })).not.toBeInTheDocument()
  })

  it('does not scoop or inspect when clicking carousel arrows', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })

    clickSpoon()
    clickCarousel('next')
    expect(screen.getByRole('button', { name: 'Hydrochloric acid (30%)' })).toBeInTheDocument()
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'spoon')
    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/action', expect.anything())
  })

  it('returns the ingredient carousel to distilled water after Reset', async () => {
    vi.stubGlobal('fetch', stubLabFetch())

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Distilled water (H2O)' })

    clickCarousel('next')
    expect(screen.getByRole('button', { name: 'Hydrochloric acid (30%)' })).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: 'Reset lab' }))
    await waitFor(() => {
      expect(screen.getByRole('button', { name: 'Distilled water (H2O)' })).toBeInTheDocument()
    })
    expect(screen.queryByRole('button', { name: 'Sodium chloride (NaCl)' })).not.toBeInTheDocument()
  })

  it('wraps the tool carousel pipette → spoon → tongs and hides the other tools from the DOM', async () => {
    vi.stubGlobal('fetch', stubLabFetch())

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })
    expect(screen.queryByRole('button', { name: 'Spoon' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Tongs' })).not.toBeInTheDocument()

    clickToolCarousel('next')
    expect(screen.getByRole('button', { name: 'Spoon' })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Pipette' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Tongs' })).not.toBeInTheDocument()

    clickToolCarousel('next')
    expect(screen.getByRole('button', { name: 'Tongs' })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Spoon' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Pipette' })).not.toBeInTheDocument()

    clickToolCarousel('next')
    expect(screen.getByRole('button', { name: 'Pipette' })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Tongs' })).not.toBeInTheDocument()

    clickToolCarousel('previous')
    expect(screen.getByRole('button', { name: 'Tongs' })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Pipette' })).not.toBeInTheDocument()
  })

  it('does not pick up or put away a tool when clicking tool carousel arrows', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })
    fireEvent.click(screen.getByRole('button', { name: 'Pipette' }))
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'pipette')

    clickToolCarousel('next')
    expect(screen.getByRole('button', { name: 'Spoon' })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Pipette' })).not.toBeInTheDocument()
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'pipette')
    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/action', expect.anything())

    clickToolCarousel('next')
    expect(screen.getByRole('button', { name: 'Tongs' })).toBeInTheDocument()
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'pipette')
    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/action', expect.anything())

    clickToolCarousel('next')
    expect(screen.getByRole('button', { name: 'Pipette' })).toHaveAttribute('aria-pressed', 'true')
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'pipette')
    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/action', expect.anything())
  })

  it('returns the tool carousel to pipette after Reset', async () => {
    vi.stubGlobal('fetch', stubLabFetch())

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })

    clickToolCarousel('next')
    expect(screen.getByRole('button', { name: 'Spoon' })).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: 'Reset lab' }))
    await waitFor(() => {
      expect(screen.getByRole('button', { name: 'Pipette' })).toBeInTheDocument()
    })
    expect(screen.queryByRole('button', { name: 'Spoon' })).not.toBeInTheDocument()
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
            composition: [{ substance_id: 'nacl', phase: 'solid', amount_ml: null, amount_scoop: STOCK_FULL_SCOOPS, amount_g: STOCK_FULL_MASS_G, amount_mol: null}],
          },
        },
        {
          id: 'beaker-cacl2',
          kind: 'beaker',
          label: 'Calcium chloride',
          location: 'bench',
          properties: {
            volume_ml: 250,
            fill_ml: 100,
            transparent: true,
            colourless: true,
            temperature_c: 20,
            composition: [{ substance_id: 'cacl2', phase: 'solid', amount_ml: null, amount_scoop: STOCK_FULL_SCOOPS, amount_g: STOCK_FULL_MASS_G, amount_mol: null}],
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
            composition: [{ substance_id: 'sand', phase: 'solid', amount_ml: null, amount_scoop: STOCK_FULL_SCOOPS, amount_g: STOCK_FULL_MASS_G, amount_mol: null}],
          },
        },
        {
          id: 'beaker-water',
          kind: 'beaker',
          label: 'Beaker',
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
    expect(await screen.findByRole('button', { name: 'Pipette' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Beaker' })).toBeInTheDocument()
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'none')
    expect(screen.queryByRole('status')).not.toBeInTheDocument()
    expect(screen.queryByRole('alert')).not.toBeInTheDocument()
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
    await screen.findByRole('button', { name: 'Pipette' })

    clickSpoon()
    clickStock('Sodium chloride (NaCl)')
    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'nacl')
    })
    fireEvent.click(screen.getByRole('button', { name: 'Beaker' }))
    await waitFor(() => {
      expect(screen.getByRole('status')).toHaveTextContent(NACL_EXPLANATION)
    })

    // Put the spoon away so idle inspect works, then confirm ions are present.
    clickSpoon()
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'none')
    fireEvent.click(screen.getByRole('button', { name: 'Beaker' }))
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
    // Reset refreshes inspect in place; it does not auto-dismiss.
    const panel = screen.getByRole('dialog', { name: 'Contents of Beaker' })
    expect(panel).toHaveTextContent('No composition reported by the server.')
    expect(panel).not.toHaveTextContent('H2O (l)')
    expect(panel).not.toHaveTextContent('Na+')
    expect(panel).not.toHaveTextContent('(aq)')
    expect(panel.querySelectorAll('sup')).toHaveLength(0)
  })

  it('keeps the filter stack left of the dish/burner stack and water beaker', async () => {
    vi.stubGlobal('fetch', stubLabFetch())

    render(<LabBench />)
    const water = await screen.findByRole('button', { name: 'Beaker' })
    const paper = screen.getByRole('button', { name: 'Filter paper' })
    const filtrate = screen.getByRole('button', { name: 'Filtrate beaker' })
    const dish = screen.getByRole('button', { name: 'Evaporation dish' })
    const burner = screen.getByRole('button', { name: 'Burner' })
    const filterStack = paper.closest('.lab-filter-stack')
    const evapStack = dish.closest('.lab-evap-stack')

    expect(filterStack).not.toBeNull()
    expect(filterStack).toContainElement(filtrate)
    expect(precedesInDocument(paper, filtrate)).toBe(true)
    expect(evapStack).not.toBeNull()
    expect(evapStack).toContainElement(burner)
    expect(precedesInDocument(dish, burner)).toBe(true)
    expect(precedesInDocument(filterStack as HTMLElement, evapStack as HTMLElement)).toBe(true)
    expect(precedesInDocument(evapStack as HTMLElement, water)).toBe(true)
  })

  it('folds the filter paper as a cone lining the inner funnel wall, not a disk underneath', async () => {
    vi.stubGlobal('fetch', stubLabFetch())

    render(<LabBench />)
    const svg = (await screen.findByRole('button', { name: 'Filter paper' })).querySelector('[data-funnel]')
    expect(svg).not.toBeNull()

    const paper = svg!.querySelector('[data-paper-cone]')
    const funnelCone = svg!.querySelector('[data-funnel-cone]')
    const stem = svg!.querySelector('[data-funnel-stem]')
    expect(paper).not.toBeNull()
    expect(funnelCone).not.toBeNull()
    expect(stem).not.toBeNull()
    expect(svg!.querySelector('ellipse')).toBeNull()

    const paperRimY = Number(paper!.getAttribute('data-rim-y'))
    const paperApexY = Number(paper!.getAttribute('data-apex-y'))
    const funnelRimY = Number(funnelCone!.getAttribute('data-rim-y'))
    const funnelApexY = Number(funnelCone!.getAttribute('data-apex-y'))
    const stemTopY = Number(stem!.getAttribute('data-top-y'))

    expect(paperApexY).toBeGreaterThan(paperRimY)
    expect(paperRimY).toBeGreaterThanOrEqual(funnelRimY)
    expect(paperRimY).toBeLessThanOrEqual(funnelRimY + 2)
    expect(paperApexY).toBeLessThanOrEqual(funnelApexY)
    expect(paperApexY).toBeLessThanOrEqual(stemTopY)
  })

  it('places the tool carousel after the stock slot with flanking arrows', async () => {
    vi.stubGlobal('fetch', stubLabFetch())

    render(<LabBench />)
    const pipette = await screen.findByRole('button', { name: 'Pipette' })
    const stock = screen.getByRole('button', { name: 'Distilled water (H2O)' })
    const carousel = pipette.closest('.lab-tool-carousel')
    const previous = screen.getByRole('button', { name: 'Previous tool' })
    const next = screen.getByRole('button', { name: 'Next tool' })

    expect(carousel).not.toBeNull()
    expect(carousel).toContainElement(previous)
    expect(carousel).toContainElement(next)
    expect(precedesInDocument(previous, pipette)).toBe(true)
    expect(precedesInDocument(pipette, next)).toBe(true)
    expect(precedesInDocument(stock, carousel as HTMLElement)).toBe(true)
    expect(screen.queryByRole('button', { name: 'Spoon' })).not.toBeInTheDocument()
  })

  it('reserves a fixed-width tool slot so pipette, spoon, and tongs do not shift the bench', async () => {
    vi.stubGlobal('fetch', stubLabFetch())

    render(<LabBench />)
    const pipette = await screen.findByRole('button', { name: 'Pipette' })
    expect(pipette.parentElement).toHaveClass('lab-tool-carousel-slot')

    clickToolCarousel('next')
    expect(screen.getByRole('button', { name: 'Spoon' }).parentElement).toHaveClass(
      'lab-tool-carousel-slot',
    )
    clickToolCarousel('next')
    expect(screen.getByRole('button', { name: 'Tongs' }).parentElement).toHaveClass(
      'lab-tool-carousel-slot',
    )
  })

  it('reserves a fixed-width ingredient slot so distilled water and salts do not shift the bench', async () => {
    vi.stubGlobal('fetch', stubLabFetch())

    render(<LabBench />)
    const water = await screen.findByRole('button', { name: 'Distilled water (H2O)' })
    expect(water.parentElement).toHaveClass('lab-stock-carousel-slot')

    clickCarousel('next')
    expect(screen.getByRole('button', { name: 'Hydrochloric acid (30%)' }).parentElement).toHaveClass(
      'lab-stock-carousel-slot',
    )
    clickCarousel('next')
    expect(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' }).parentElement).toHaveClass(
      'lab-stock-carousel-slot',
    )
    clickCarousel('next')
    expect(
      screen.getByRole('button', { name: 'Calcium chloride (CaCl2)' }).parentElement,
    ).toHaveClass('lab-stock-carousel-slot')
    clickCarousel('next')
    expect(screen.getByRole('button', { name: 'Sand' }).parentElement).toHaveClass(
      'lab-stock-carousel-slot',
    )
  })

})
