/** @vitest-environment jsdom */
import '@testing-library/jest-dom/vitest'
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { LabBench } from './LabBench'
import { clearCsrfTokenCache } from '../lib/api'
import {
  SEPARATE_CHALLENGE,
  challengeScene,
  cloneScene,
  expectCsrfLabAction,
  clickCarousel,
  stubLabFetch,
} from './labBenchTestHelpers'

describe('LabBench challenge mode', () => {
  beforeEach(() => {
    clearCsrfTokenCache()
  })

  afterEach(() => {
    cleanup()
    vi.unstubAllGlobals()
  })

  it('shows the challenge prompt instead of the Free-mode tool blurb', async () => {
    vi.stubGlobal('fetch', stubLabFetch({ scene: challengeScene() }))

    render(<LabBench />)

    expect(await screen.findByText(SEPARATE_CHALLENGE.prompt)).toBeInTheDocument()
    expect(screen.queryByText(/Pick up the spoon/i)).not.toBeInTheDocument()
    expect(screen.queryByText(SEPARATE_CHALLENGE.done)).not.toBeInTheDocument()
  })

  it('thanks the player once the server reports the challenge completed', async () => {
    const won = cloneScene(challengeScene())
    won.challenge_completed = true
    vi.stubGlobal('fetch', stubLabFetch({ scene: won }))

    render(<LabBench />)

    expect(await screen.findByText(SEPARATE_CHALLENGE.done)).toBeInTheDocument()
    expect(screen.queryByText(SEPARATE_CHALLENGE.prompt)).not.toBeInTheDocument()
  })

  it('keeps the Free-mode blurb and every stock in Free mode', async () => {
    vi.stubGlobal('fetch', stubLabFetch())

    render(<LabBench />)

    expect(await screen.findByText(/Pick up the spoon/i)).toBeInTheDocument()
    const stocks = [
      'Distilled water (H2O)',
      'Hydrochloric acid (30%)',
      'Sodium chloride (NaCl)',
      'Sodium hydroxide (NaOH)',
      'Calcium chloride (CaCl2)',
      'Sand',
    ]
    for (const stock of stocks) {
      expect(screen.getByRole('button', { name: stock })).toBeInTheDocument()
      clickCarousel('next')
    }
  })

  it('drops stocks the challenge removed from the carousel', async () => {
    vi.stubGlobal('fetch', stubLabFetch({ scene: challengeScene() }))

    render(<LabBench />)
    expect(await screen.findByRole('button', { name: 'Distilled water (H2O)' })).toBeInTheDocument()

    const seen: string[] = []
    for (let i = 0; i < 6; i += 1) {
      for (const stock of [
        'Distilled water (H2O)',
        'Hydrochloric acid (30%)',
        'Sodium chloride (NaCl)',
        'Sodium hydroxide (NaOH)',
        'Calcium chloride (CaCl2)',
        'Sand',
      ]) {
        if (screen.queryByRole('button', { name: stock })) seen.push(stock)
      }
      clickCarousel('next')
    }

    expect(new Set(seen)).toEqual(new Set(['Distilled water (H2O)', 'Sodium chloride (NaCl)', 'Sand']))
    expect(seen).not.toContain('Calcium chloride (CaCl2)')
    expect(seen).not.toContain('Sodium hydroxide (NaOH)')
    expect(seen).not.toContain('Hydrochloric acid (30%)')
  })

  it('starts the challenge distilled-water stock at 10 ml', () => {
    const h2o = challengeScene().items.find((item) => item.id === 'beaker-h2o')!
    expect(h2o.properties.volume_ml).toBe(100)
    expect(h2o.properties.fill_ml).toBe(10)
    expect(h2o.properties.composition?.[0]?.amount_ml).toBe(10)
  })

  it('reset inside a challenge stays in the challenge', async () => {
    const fetchMock = stubLabFetch({ scene: challengeScene() })
    vi.stubGlobal('fetch', fetchMock)
    const onModeChange = vi.fn()

    render(<LabBench onModeChange={onModeChange} />)
    await screen.findByText(SEPARATE_CHALLENGE.prompt)
    expect(onModeChange).toHaveBeenCalledWith(SEPARATE_CHALLENGE.id)

    fireEvent.click(screen.getByRole('button', { name: 'Reset lab' }))

    await waitFor(() => {
      expectCsrfLabAction(fetchMock, { type: 'reset' })
    })
    expect(screen.getByText(SEPARATE_CHALLENGE.prompt)).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Calcium chloride (CaCl2)' })).not.toBeInTheDocument()
    expect(onModeChange).not.toHaveBeenCalledWith('free')
  })
})
