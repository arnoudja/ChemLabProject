/** @vitest-environment jsdom */
import { render, screen } from '@testing-library/react'
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

describe('App', () => {
  beforeEach(() => {
    clearCsrfTokenCache()
  })

  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('shows the account form when there is no session', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async (input: RequestInfo | URL) => {
        const url = String(input)
        if (url === '/api/health') {
          return jsonResponse({
            status: 'ok',
            version: '0.1.0',
            service: 'chemlab-server',
          })
        }
        if (url === '/api/auth/me') {
          return jsonResponse({ authenticated: false, user: null })
        }
        return jsonResponse({ error: 'not found', code: 'not_found' }, 404)
      }),
    )

    render(<App />)

    expect(await screen.findByText('Display name')).toBeTruthy()
    expect(screen.getByText(/API 0.1.0/)).toBeTruthy()
  })

  it('welcomes a signed-in user', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async (input: RequestInfo | URL) => {
        const url = String(input)
        if (url === '/api/health') {
          return jsonResponse({
            status: 'ok',
            version: '0.1.0',
            service: 'chemlab-server',
          })
        }
        if (url === '/api/auth/me') {
          return jsonResponse({ authenticated: true, user: userPayload })
        }
        return jsonResponse({ error: 'not found', code: 'not_found' }, 404)
      }),
    )

    render(<App />)

    expect(await screen.findByText('Welcome back, Ada')).toBeTruthy()
    expect(screen.getByRole('button', { name: 'Sign out' })).toBeTruthy()
  })
})
