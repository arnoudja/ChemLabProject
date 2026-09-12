/** @vitest-environment jsdom */
import '@testing-library/jest-dom/vitest'
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import App from './App'
import { clearCsrfTokenCache } from './lib/api'

const userPayload = {
  id: 'u1',
  email: 'ada@chemlab.local',
  display_name: 'Ada',
  created_at: '2020-01-01T00:00:00Z',
}

function jsonResponse(body: unknown, status = 200) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  })
}

function stubAppFetch(options?: {
  authenticated?: boolean
  dissolveBody?: unknown
  dissolveStatus?: number
}) {
  const authenticated = options?.authenticated ?? false
  return vi.fn(async (input: RequestInfo | URL, _init?: RequestInit) => {
    const url = String(input)
    if (url === '/api/health') {
      return jsonResponse({
        status: 'ok',
        version: '0.1.0',
        service: 'chemlab-server',
      })
    }
    if (url === '/api/auth/me') {
      return jsonResponse({
        authenticated,
        user: authenticated ? userPayload : null,
      })
    }
    if (url === '/api/auth/csrf') {
      return jsonResponse({ csrf_token: 'tok-123' })
    }
    if (url === '/api/lab/dissolve') {
      return jsonResponse(
        options?.dissolveBody ?? {
          dissolved: true,
          explanation:
            'Sodium chloride (NaCl) dissolves in water at bench temperature.',
        },
        options?.dissolveStatus ?? 200,
      )
    }
    return jsonResponse({ error: 'not found', code: 'not_found' }, 404)
  })
}

describe('App', () => {
  beforeEach(() => {
    clearCsrfTokenCache()
  })

  afterEach(() => {
    cleanup()
    vi.unstubAllGlobals()
  })

  it('shows the account form when there is no session', async () => {
    vi.stubGlobal('fetch', stubAppFetch({ authenticated: false }))

    render(<App />)

    expect(await screen.findByText('Display name')).toBeTruthy()
    expect(screen.getByText(/API 0.1.0/)).toBeTruthy()
  })

  it('welcomes a signed-in user', async () => {
    vi.stubGlobal('fetch', stubAppFetch({ authenticated: true }))

    render(<App />)

    expect(await screen.findByText('Welcome back, Ada')).toBeTruthy()
    expect(screen.getByRole('button', { name: 'Sign out' })).toBeTruthy()
  })

  it('replaces unlock-later copy with a dissolve control after login', async () => {
    vi.stubGlobal('fetch', stubAppFetch({ authenticated: true }))

    render(<App />)

    expect(await screen.findByRole('button', { name: 'Dissolve' })).toBeInTheDocument()
    expect(screen.queryByText(/benches unlock/i)).not.toBeInTheDocument()
    expect(screen.getByLabelText('Solid')).toBeInTheDocument()
    expect(screen.getByText(/water at 20/i)).toBeInTheDocument()
  })

  it('shows the server outcome and explanation without deciding locally', async () => {
    const fetchMock = stubAppFetch({
      authenticated: true,
      dissolveBody: {
        dissolved: false,
        explanation: 'Sand (silica) does not dissolve in water at bench temperature.',
      },
    })
    vi.stubGlobal('fetch', fetchMock)

    render(<App />)
    expect(await screen.findByRole('button', { name: 'Dissolve' })).toBeInTheDocument()

    fireEvent.change(screen.getByLabelText('Solid'), { target: { value: 'sand' } })
    fireEvent.click(screen.getByRole('button', { name: 'Dissolve' }))

    await waitFor(() => {
      expect(screen.getByRole('status')).toHaveTextContent(
        'Sand (silica) does not dissolve in water at bench temperature.',
      )
    })
    const dissolveCall = fetchMock.mock.calls.find(([url]) => String(url) === '/api/lab/dissolve')
    expect(dissolveCall?.[1]).toEqual(
      expect.objectContaining({
        method: 'POST',
        credentials: 'include',
        body: JSON.stringify({
          substance_id: 'sand',
          solvent_id: 'water',
          temperature_c: 20,
        }),
      }),
    )
    expect(screen.getByRole('status')).toHaveTextContent('did not dissolve')
  })

  it('renders a surprising server payload instead of inferring from the picker', async () => {
    vi.stubGlobal(
      'fetch',
      stubAppFetch({
        authenticated: true,
        dissolveBody: {
          dissolved: false,
          explanation: 'Unexpected server sentence for nacl.',
        },
      }),
    )

    render(<App />)
    expect(await screen.findByRole('button', { name: 'Dissolve' })).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: 'Dissolve' }))

    await waitFor(() => {
      expect(screen.getByRole('status')).toHaveTextContent(
        'Unexpected server sentence for nacl.',
      )
    })
    expect(screen.getByRole('status')).toHaveTextContent('did not dissolve')
    expect(screen.queryByText(/Sodium chloride \(NaCl\) dissolves/)).not.toBeInTheDocument()
  })
})
