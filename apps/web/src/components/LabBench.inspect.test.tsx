/** @vitest-environment jsdom */
import '@testing-library/jest-dom/vitest'
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  LabBench,
  SPOON_SCOOP_MASS_G,
  STOCK_FULL_MASS_G,
} from './LabBench'
import { clearCsrfTokenCache } from '../lib/api'
import {
  initialScene,
  filledScene,
  withScoop,
  afterNaclPour,
  afterSandPour,
  stubLabFetch,
  clickStock,
  clickSpoon,
} from './labBenchTestHelpers'

describe('LabBench inspect', () => {
  beforeEach(() => {
    clearCsrfTokenCache()
  })

  afterEach(() => {
    cleanup()
    vi.unstubAllGlobals()
    vi.useRealTimers()
  })

  it('idle water click shows beaker contents and temperature from the scene', async () => {
    const fetchMock = stubLabFetch({ scene: initialScene() })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Beaker' })

    fireEvent.click(screen.getByRole('button', { name: 'Beaker' }))

    const panel = await screen.findByRole('dialog', { name: 'Contents of Beaker' })
    expect(panel).toHaveTextContent('Beaker')
    expect(panel).toHaveTextContent('No composition reported by the server.')
    expect(panel).not.toHaveTextContent('H2O (l)')
    expect(panel).toHaveTextContent('Temperature: 20.00°C')
    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/action', expect.anything())
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'none')
  })

  it('idle salt inspect shows server stock mass', async () => {
    vi.stubGlobal('fetch', stubLabFetch())

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })

    clickStock('Sodium chloride (NaCl)')

    const panel = await screen.findByRole('dialog', { name: 'Contents of Sodium chloride' })
    expect(panel).toHaveTextContent('NaCl (s)')
    expect(panel).toHaveTextContent(`${STOCK_FULL_MASS_G.toFixed(2)} g`)
  })

  it('after scoop, salt inspect shows depleted server stock mass', async () => {
    const scooped = withScoop(initialScene(), 'nacl')
    const fetchMock = stubLabFetch({ scene: scooped })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })

    clickStock('Sodium chloride (NaCl)')

    const panel = await screen.findByRole('dialog', { name: 'Contents of Sodium chloride' })
    expect(panel).toHaveTextContent('NaCl (s)')
    expect(panel).toHaveTextContent('1.80 g')
  })

  it('idle inspect after dissolve shows aqueous ions from the server composition', async () => {
    const dissolved = afterNaclPour(withScoop(filledScene(), 'nacl'))
    dissolved.items.find((item) => item.id === 'spoon-1')!.properties.holding = []
    const fetchMock = stubLabFetch({ scene: dissolved })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Beaker' })

    fireEvent.click(screen.getByRole('button', { name: 'Beaker' }))

    const panel = await screen.findByRole('dialog', { name: 'Contents of Beaker' })
    expect(panel).toHaveTextContent('H2O (l)')
    expect(panel).toHaveTextContent('Na+ (aq)')
    expect(panel).toHaveTextContent('Cl− (aq)')
    expect(panel).toHaveTextContent('0.0171 M')
    expect(panel.querySelectorAll('sup')).toHaveLength(2)
    expect(panel).toHaveTextContent('Temperature: 19.98°C')
    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/action', expect.anything())
  })

  it('idle inspect after sand pour shows aggregated SiO2 mass not molarity', async () => {
    const leftover = afterSandPour(withScoop(filledScene(), 'sand'))
    leftover.items.find((item) => item.id === 'spoon-1')!.properties.holding = []
    const fetchMock = stubLabFetch({ scene: leftover })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Beaker' })

    fireEvent.click(screen.getByRole('button', { name: 'Beaker' }))

    const panel = await screen.findByRole('dialog', { name: 'Contents of Beaker' })
    expect(panel).toHaveTextContent('SiO2 (s)')
    expect(panel).toHaveTextContent(`${SPOON_SCOOP_MASS_G.toFixed(2)} g`)
    expect(panel.querySelector('sub')?.textContent).toBe('2')
    expect(panel).not.toHaveTextContent(' M')
    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/action', expect.anything())
  })

  it('with spoon selected, water click does not open inspect', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })

    clickSpoon()
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'spoon')

    fireEvent.click(screen.getByRole('button', { name: 'Beaker' }))

    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/action', expect.anything())
  })

  it('inspect stays open across scoop and refreshes stock mass', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })

    clickStock('Sodium chloride (NaCl)')
    const panel = await screen.findByRole('dialog', { name: 'Contents of Sodium chloride' })
    expect(panel).toHaveTextContent(`${STOCK_FULL_MASS_G.toFixed(2)} g`)

    clickSpoon()
    expect(screen.getByRole('dialog', { name: 'Contents of Sodium chloride' })).toBeInTheDocument()

    clickStock('Sodium chloride (NaCl)')
    await waitFor(() => {
      expect(screen.getByRole('dialog', { name: 'Contents of Sodium chloride' })).toHaveTextContent(
        `${(STOCK_FULL_MASS_G - SPOON_SCOOP_MASS_G).toFixed(2)} g`,
      )
    })
    expect(fetchMock).toHaveBeenCalledWith('/api/lab/action', expect.anything())
  })

  it('inspect stays open across pour/dissolve and refreshes composition and temperature', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Beaker' })

    fireEvent.click(screen.getByRole('button', { name: 'Beaker' }))
    const panel = await screen.findByRole('dialog', { name: 'Contents of Beaker' })
    expect(panel).toHaveTextContent('H2O (l)')
    expect(panel).toHaveTextContent('Temperature: 20.00°C')
    expect(panel).not.toHaveTextContent('Na+')

    clickSpoon()
    clickStock('Sodium chloride (NaCl)')
    await waitFor(() => {
      expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'nacl')
    })
    expect(screen.getByRole('dialog', { name: 'Contents of Beaker' })).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: 'Beaker' }))
    await waitFor(() => {
      const open = screen.getByRole('dialog', { name: 'Contents of Beaker' })
      expect(open).toHaveTextContent('Na+ (aq)')
      expect(open).toHaveTextContent('Cl− (aq)')
      expect(open).toHaveTextContent('0.0171 M')
      expect(open).toHaveTextContent('Temperature: 19.98°C')
    })
  })

  it('inspect stays open across put-back and only Close dismisses it', async () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Pipette' })

    clickStock('Sodium chloride (NaCl)')
    expect(await screen.findByRole('dialog', { name: 'Contents of Sodium chloride' })).toHaveTextContent(
      `${STOCK_FULL_MASS_G.toFixed(2)} g`,
    )

    clickSpoon()
    clickStock('Sodium chloride (NaCl)')
    await waitFor(() => {
      expect(screen.getByRole('dialog')).toHaveTextContent(
        `${(STOCK_FULL_MASS_G - SPOON_SCOOP_MASS_G).toFixed(2)} g`,
      )
    })

    clickStock('Sodium chloride (NaCl)')
    await waitFor(() => {
      expect(screen.getByRole('dialog')).toHaveTextContent(`${STOCK_FULL_MASS_G.toFixed(2)} g`)
    })
    expect(screen.getByRole('status')).toHaveTextContent('Returned nacl.')

    fireEvent.click(screen.getByRole('button', { name: 'Close' }))
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
  })


  it('idle dish click inspects contents including temperature', async () => {
    vi.stubGlobal('fetch', stubLabFetch())

    render(<LabBench />)
    await screen.findByRole('button', { name: 'Evaporation dish' })
    fireEvent.click(screen.getByRole('button', { name: 'Evaporation dish' }))

    const panel = await screen.findByRole('dialog', { name: 'Contents of Evaporation dish' })
    expect(panel).toHaveTextContent('Temperature: 20.00°C')
  })

  it('idle distilled water click inspects the 100 ml stock', async () => {
    vi.stubGlobal('fetch', stubLabFetch())

    render(<LabBench />)
    fireEvent.click(await screen.findByRole('button', { name: 'Distilled water (H2O)' }))

    const panel = await screen.findByRole('dialog', { name: 'Contents of Distilled water' })
    expect(panel).toHaveTextContent('H2O (l)')
    expect(panel).toHaveTextContent('100.00 ml')
  })

  it('idle click inspects paper versus filtrate', async () => {
    vi.stubGlobal('fetch', stubLabFetch())

    render(<LabBench />)
    fireEvent.click(await screen.findByRole('button', { name: 'Filter paper' }))
    expect(await screen.findByRole('dialog', { name: 'Contents of Filter paper' })).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Close' }))
    fireEvent.click(screen.getByRole('button', { name: 'Filtrate beaker' }))
    expect(await screen.findByRole('dialog', { name: 'Contents of Filtrate' })).toBeInTheDocument()
  })

})
