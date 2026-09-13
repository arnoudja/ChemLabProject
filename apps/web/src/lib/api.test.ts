import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  clearCsrfTokenCache,
  dissolve,
  fetchHealth,
  fetchLabScene,
  fetchMe,
  login,
  logout,
  postLabAction,
  register,
} from './api'

const initialScene = {
  lab_id: 'lab-1',
  version: 0,
  temperature_c: 20,
  last_events: [] as { kind: string; message: string }[],
  items: [
    {
      id: 'spoon-1',
      kind: 'spoon',
      label: 'Spoon',
      location: 'bench',
      properties: {
        volume_ml: null,
        fill_ml: null,
        transparent: null,
        colourless: null,
        temperature_c: null,
        composition: [],
        holding: [],
      },
    },
    {
      id: 'beaker-water',
      kind: 'beaker',
      label: 'Water',
      location: 'bench',
      properties: {
        volume_ml: 250,
        fill_ml: 200,
        transparent: true,
        colourless: true,
        temperature_c: 20,
        composition: [
          { substance_id: 'water', phase: 'liquid', amount_ml: 200, amount_scoop: null },
        ],
        holding: [],
      },
    },
  ],
}

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
        if (url === '/api/auth/me') {
          return jsonResponse({ authenticated: true, user: userPayload })
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
        if (url === '/api/lab/dissolve') {
          return jsonResponse({
            dissolved: true,
            explanation:
              'Sodium chloride (NaCl) dissolves in water at bench temperature.',
          })
        }
        if (url === '/api/lab/scene') {
          return jsonResponse(initialScene)
        }
        if (url === '/api/lab/action') {
          return jsonResponse({
            scene: {
              ...initialScene,
              version: 1,
              last_events: [{ kind: 'scooped', message: 'Scooped nacl.' }],
              items: initialScene.items.map((item) =>
                item.id === 'spoon-1'
                  ? {
                      ...item,
                      location: 'hand',
                      properties: {
                        ...item.properties,
                        holding: [
                          {
                            substance_id: 'nacl',
                            phase: 'solid',
                            amount_ml: null,
                            amount_scoop: 1,
                          },
                        ],
                      },
                    }
                  : item,
              ),
            },
          })
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

  it('dissolve posts CSRF JSON and returns the server payload as-is', async () => {
    const outcome = await dissolve({
      substance_id: 'nacl',
      solvent_id: 'water',
      temperature_c: 20,
    })

    expect(fetch).toHaveBeenCalledWith('/api/auth/csrf', {
      credentials: 'include',
    })
    expect(fetch).toHaveBeenCalledWith('/api/lab/dissolve', {
      method: 'POST',
      credentials: 'include',
      headers: {
        'content-type': 'application/json',
        'X-CSRF-Token': 'tok-123',
      },
      body: JSON.stringify({
        substance_id: 'nacl',
        solvent_id: 'water',
        temperature_c: 20,
      }),
    })
    expect(outcome).toEqual({
      dissolved: true,
      explanation: 'Sodium chloride (NaCl) dissolves in water at bench temperature.',
    })
  })

  it('dissolve forwards ids as-is and does not decide dissolved locally', async () => {
    vi.mocked(fetch).mockImplementation(async (input: RequestInfo | URL) => {
      const url = String(input)
      if (url === '/api/auth/csrf') {
        return jsonResponse({ csrf_token: 'tok-123' })
      }
      if (url === '/api/lab/dissolve') {
        return jsonResponse({
          dissolved: false,
          explanation: 'Sand (silica) does not dissolve in water at bench temperature.',
        })
      }
      return jsonResponse({ error: 'not found', code: 'not_found' }, 404)
    })

    const outcome = await dissolve({
      substance_id: 'sand',
      solvent_id: 'water',
      temperature_c: 20,
    })

    const dissolveCall = vi
      .mocked(fetch)
      .mock.calls.find(([url]) => String(url) === '/api/lab/dissolve')
    expect(dissolveCall?.[1]).toEqual(
      expect.objectContaining({
        body: JSON.stringify({
          substance_id: 'sand',
          solvent_id: 'water',
          temperature_c: 20,
        }),
      }),
    )
    expect(outcome.dissolved).toBe(false)
    expect(outcome.explanation).toBe(
      'Sand (silica) does not dissolve in water at bench temperature.',
    )
  })

  it('fetchMe includes the session cookie', async () => {
    const me = await fetchMe()
    expect(me).toEqual({ authenticated: true, user: userPayload })
    expect(fetch).toHaveBeenCalledWith('/api/auth/me', { credentials: 'include' })
  })

  it.each([
    {
      error: 'unknown substance',
      code: 'unknown_substance',
      status: 400,
    },
    {
      error: 'unsupported solvent',
      code: 'unsupported_solvent',
      status: 400,
    },
    {
      error: 'unsupported temperature',
      code: 'unsupported_temperature',
      status: 400,
    },
    {
      error: 'Login required',
      code: 'unauthenticated',
      status: 401,
    },
  ])('dissolve surfaces $code from the server body', async ({ error, code, status }) => {
    vi.mocked(fetch).mockImplementation(async (input: RequestInfo | URL) => {
      const url = String(input)
      if (url === '/api/auth/csrf') {
        return jsonResponse({ csrf_token: 'tok-123' })
      }
      if (url === '/api/lab/dissolve') {
        return jsonResponse({ error, code }, status)
      }
      return jsonResponse({ error: 'not found', code: 'not_found' }, 404)
    })

    await expect(
      dissolve({
        substance_id: 'nacl',
        solvent_id: 'water',
        temperature_c: 20,
      }),
    ).rejects.toThrow(error)
  })

  it('parseJson falls back to the HTTP status when the error body has no message', async () => {
    vi.mocked(fetch).mockImplementation(async (input: RequestInfo | URL) => {
      const url = String(input)
      if (url === '/api/auth/csrf') {
        return jsonResponse({ csrf_token: 'tok-123' })
      }
      if (url === '/api/lab/dissolve') {
        return jsonResponse({}, 503)
      }
      return jsonResponse({ error: 'not found', code: 'not_found' }, 404)
    })

    await expect(
      dissolve({
        substance_id: 'nacl',
        solvent_id: 'water',
        temperature_c: 20,
      }),
    ).rejects.toThrow('Request failed (503)')
  })

  it('logout throws when the server rejects the session teardown', async () => {
    vi.mocked(fetch).mockImplementation(async (input: RequestInfo | URL) => {
      const url = String(input)
      if (url === '/api/auth/csrf') {
        return jsonResponse({ csrf_token: 'tok-123' })
      }
      if (url === '/api/auth/logout') {
        return jsonResponse({ error: 'Database error', code: 'internal' }, 500)
      }
      return jsonResponse({ error: 'not found', code: 'not_found' }, 404)
    })

    await expect(logout()).rejects.toThrow('Could not log out')
  })

  it('fetchLabScene includes credentials and returns the server snapshot', async () => {
    const scene = await fetchLabScene()
    expect(scene.lab_id).toBe('lab-1')
    expect(scene.items.some((item) => item.id === 'beaker-water')).toBe(true)
    expect(fetch).toHaveBeenCalledWith('/api/lab/scene', { credentials: 'include' })
  })

  it('postLabAction posts CSRF JSON and returns the updated scene as-is', async () => {
    const response = await postLabAction({
      type: 'use_tool',
      tool_item_id: 'spoon-1',
      target_item_id: 'beaker-nacl',
    })

    expect(fetch).toHaveBeenCalledWith('/api/auth/csrf', {
      credentials: 'include',
    })
    expect(fetch).toHaveBeenCalledWith('/api/lab/action', {
      method: 'POST',
      credentials: 'include',
      headers: {
        'content-type': 'application/json',
        'X-CSRF-Token': 'tok-123',
      },
      body: JSON.stringify({
        type: 'use_tool',
        tool_item_id: 'spoon-1',
        target_item_id: 'beaker-nacl',
      }),
    })
    expect(response.scene.version).toBe(1)
    expect(response.scene.last_events?.[0]?.message).toBe('Scooped nacl.')
    const spoon = response.scene.items.find((item) => item.id === 'spoon-1')
    expect(spoon?.properties.holding?.[0]?.substance_id).toBe('nacl')
  })

  it('fetchLabScene fills omitted empty holding/composition/last_events', async () => {
    vi.mocked(fetch).mockImplementation(async (input: RequestInfo | URL) => {
      const url = String(input)
      if (url === '/api/lab/scene') {
        return jsonResponse({
          lab_id: 'lab-omit',
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
          ],
        })
      }
      return jsonResponse({ error: 'not found', code: 'not_found' }, 404)
    })

    const scene = await fetchLabScene()
    expect(scene.last_events).toEqual([])
    expect(scene.items[0]?.properties.holding).toEqual([])
    expect(scene.items[0]?.properties.composition).toEqual([])
  })

  it('postLabAction surfaces unauthenticated errors from the server body', async () => {
    vi.mocked(fetch).mockImplementation(async (input: RequestInfo | URL) => {
      const url = String(input)
      if (url === '/api/auth/csrf') {
        return jsonResponse({ csrf_token: 'tok-123' })
      }
      if (url === '/api/lab/action') {
        return jsonResponse({ error: 'Login required', code: 'unauthenticated' }, 401)
      }
      return jsonResponse({ error: 'not found', code: 'not_found' }, 404)
    })

    await expect(
      postLabAction({
        type: 'pour',
        source_item_id: 'spoon-1',
        target_item_id: 'beaker-water',
      }),
    ).rejects.toThrow('Login required')
  })
})
