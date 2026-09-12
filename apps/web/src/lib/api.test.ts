import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  clearCsrfTokenCache,
  fetchHealth,
  login,
  logout,
  register,
} from './api'

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

describe('api client', () => {
  beforeEach(() => {
    clearCsrfTokenCache()
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
        if (url === '/api/auth/csrf') {
          return jsonResponse({ csrf_token: 'tok-123' })
        }
        if (url === '/api/auth/register') {
          return jsonResponse(userPayload, 201)
        }
        if (url === '/api/auth/login') {
          return jsonResponse(userPayload)
        }
        if (url === '/api/auth/logout') {
          return new Response(null, { status: 204 })
        }
        return jsonResponse({ error: 'not found', code: 'not_found' }, 404)
      }),
    )
  })

  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('fetchHealth returns typed payload', async () => {
    const health = await fetchHealth()
    expect(health.status).toBe('ok')
    expect(health.service).toBe('chemlab-server')
    expect(fetch).toHaveBeenCalledWith('/api/health')
  })

  it('register fetches CSRF then posts X-CSRF-Token', async () => {
    await register({
      email: 'ada@chemlab.local',
      password: 'secret123',
      display_name: 'Ada',
    })
    expect(fetch).toHaveBeenCalledWith('/api/auth/csrf', {
      credentials: 'include',
    })
    expect(fetch).toHaveBeenCalledWith('/api/auth/register', {
      method: 'POST',
      credentials: 'include',
      headers: {
        'content-type': 'application/json',
        'X-CSRF-Token': 'tok-123',
      },
      body: JSON.stringify({
        email: 'ada@chemlab.local',
        password: 'secret123',
        display_name: 'Ada',
      }),
    })
  })

  it('login and logout reuse the cached CSRF token', async () => {
    await login({ email: 'ada@chemlab.local', password: 'secret123' })
    await logout()
    const csrfCalls = vi
      .mocked(fetch)
      .mock.calls.filter(([url]) => String(url) === '/api/auth/csrf')
    expect(csrfCalls).toHaveLength(1)
    expect(fetch).toHaveBeenCalledWith(
      '/api/auth/login',
      expect.objectContaining({
        method: 'POST',
        headers: expect.objectContaining({ 'X-CSRF-Token': 'tok-123' }),
      }),
    )
    expect(fetch).toHaveBeenCalledWith(
      '/api/auth/logout',
      expect.objectContaining({
        method: 'POST',
        headers: expect.objectContaining({ 'X-CSRF-Token': 'tok-123' }),
      }),
    )
  })
})
