/** @vitest-environment jsdom */
import '@testing-library/jest-dom/vitest'
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { LabBench } from './LabBench'
import { clearCsrfTokenCache } from '../lib/api'

const NACL_SERVER = {
  dissolved: true,
  explanation: 'Sodium chloride (NaCl) dissolves in water at bench temperature.',
} as const

const SAND_SERVER = {
  dissolved: false,
  explanation: 'Sand (silica) does not dissolve in water at bench temperature.',
} as const

function jsonResponse(body: unknown, status = 200) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  })
}

function stubLabFetch(options?: {
  dissolveBySubstance?: Record<string, { body: unknown; status?: number }>
  dissolveBody?: unknown
  dissolveStatus?: number
}) {
  return vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = String(input)
    if (url === '/api/auth/csrf') {
      return jsonResponse({ csrf_token: 'tok-123' })
    }
    if (url === '/api/lab/dissolve') {
      const sent = init?.body ? JSON.parse(String(init.body)) : {}
      const bySubstance = options?.dissolveBySubstance?.[sent.substance_id]
      if (bySubstance) {
        return jsonResponse(bySubstance.body, bySubstance.status ?? 200)
      }
      return jsonResponse(options?.dissolveBody ?? NACL_SERVER, options?.dissolveStatus ?? 200)
    }
    return jsonResponse({ error: 'not found', code: 'not_found' }, 404)
  })
}

function lastDissolveInit(fetchMock: ReturnType<typeof vi.fn>) {
  const calls = fetchMock.mock.calls.filter(([url]) => String(url) === '/api/lab/dissolve')
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

  it('starts with the spoon on the table and no dissolve request', () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)

    const bench = screen.getByRole('region', { name: 'Lab bench' })
    expect(bench).toHaveAttribute('data-tool', 'none')
    expect(screen.getByRole('button', { name: 'Spoon' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Sand' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Water beaker' })).toBeInTheDocument()
    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/dissolve', expect.anything())
    expect(screen.queryByRole('status')).not.toBeInTheDocument()
  })

  it('clicking the spoon holds it as the cursor tool', () => {
    vi.stubGlobal('fetch', stubLabFetch())

    render(<LabBench />)

    fireEvent.click(screen.getByRole('button', { name: 'Spoon' }))

    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'spoon')
    expect(screen.getByRole('button', { name: 'Spoon' })).toHaveAttribute('aria-pressed', 'true')
  })

  it('does not scoop or call the API without a held spoon', () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)

    fireEvent.click(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' }))
    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))

    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'none')
    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/dissolve', expect.anything())
  })

  it('spoon then nacl then water posts nacl/water/20 with CSRF and shows the server sentence', async () => {
    const fetchMock = stubLabFetch({
      dissolveBySubstance: { nacl: { body: NACL_SERVER } },
    })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)

    fireEvent.click(screen.getByRole('button', { name: 'Spoon' }))
    fireEvent.click(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' }))
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'nacl')
    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/dissolve', expect.anything())

    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))

    await waitFor(() => {
      expect(screen.getByRole('status')).toHaveTextContent(NACL_SERVER.explanation)
    })
    expect(screen.getByRole('status')).toHaveTextContent('Server outcome: dissolved')
    expect(lastDissolveInit(fetchMock)).toEqual(
      expect.objectContaining({
        method: 'POST',
        credentials: 'include',
        headers: expect.objectContaining({
          'content-type': 'application/json',
          'X-CSRF-Token': 'tok-123',
        }),
        body: JSON.stringify({
          substance_id: 'nacl',
          solvent_id: 'water',
          temperature_c: 20,
        }),
      }),
    )
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'spoon')
  })

  it('spoon then sand then water posts sand/water/20 and renders a surprising server payload', async () => {
    const fetchMock = stubLabFetch({
      dissolveBody: {
        dissolved: true,
        explanation: 'Unexpected server sentence for sand.',
      },
    })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)

    fireEvent.click(screen.getByRole('button', { name: 'Spoon' }))
    fireEvent.click(screen.getByRole('button', { name: 'Sand' }))
    expect(screen.getByRole('region', { name: 'Lab bench' })).toHaveAttribute('data-tool', 'sand')
    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))

    await waitFor(() => {
      expect(screen.getByRole('status')).toHaveTextContent('Unexpected server sentence for sand.')
    })
    expect(screen.getByRole('status')).toHaveTextContent('Server outcome: dissolved')
    expect(screen.queryByText(SAND_SERVER.explanation)).not.toBeInTheDocument()
    expect(lastDissolveInit(fetchMock)).toEqual(
      expect.objectContaining({
        credentials: 'include',
        headers: expect.objectContaining({ 'X-CSRF-Token': 'tok-123' }),
        body: JSON.stringify({
          substance_id: 'sand',
          solvent_id: 'water',
          temperature_c: 20,
        }),
      }),
    )
  })

  it('sand pour shows the server did-not-dissolve sentence', async () => {
    const fetchMock = stubLabFetch({
      dissolveBySubstance: { sand: { body: SAND_SERVER } },
    })
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)

    fireEvent.click(screen.getByRole('button', { name: 'Spoon' }))
    fireEvent.click(screen.getByRole('button', { name: 'Sand' }))
    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))

    await waitFor(() => {
      expect(screen.getByRole('status')).toHaveTextContent(SAND_SERVER.explanation)
    })
    expect(screen.getByRole('status')).toHaveTextContent('Server outcome: did not dissolve')
    expect(lastDissolveInit(fetchMock)).toEqual(
      expect.objectContaining({
        body: JSON.stringify({
          substance_id: 'sand',
          solvent_id: 'water',
          temperature_c: 20,
        }),
      }),
    )
  })

  it('empty spoon on water does not call dissolve', () => {
    const fetchMock = stubLabFetch()
    vi.stubGlobal('fetch', fetchMock)

    render(<LabBench />)

    fireEvent.click(screen.getByRole('button', { name: 'Spoon' }))
    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))

    expect(fetchMock).not.toHaveBeenCalledWith('/api/lab/dissolve', expect.anything())
    expect(screen.queryByRole('status')).not.toBeInTheDocument()
  })

  it('shows the server dissolve error and no outcome after a pour', async () => {
    vi.stubGlobal(
      'fetch',
      stubLabFetch({
        dissolveBody: { error: 'Login required', code: 'unauthenticated' },
        dissolveStatus: 401,
      }),
    )

    render(<LabBench />)

    fireEvent.click(screen.getByRole('button', { name: 'Spoon' }))
    fireEvent.click(screen.getByRole('button', { name: 'Sodium chloride (NaCl)' }))
    fireEvent.click(screen.getByRole('button', { name: 'Water beaker' }))

    expect(await screen.findByRole('alert')).toHaveTextContent('Login required')
    expect(screen.queryByRole('status')).not.toBeInTheDocument()
  })
})
