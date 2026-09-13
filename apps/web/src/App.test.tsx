/** @vitest-environment jsdom */
import '@testing-library/jest-dom/vitest'
import { cleanup, fireEvent, render, screen } from '@testing-library/react'
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

const EMPTY_LAB_SCENE = {
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
        composition: [{ substance_id: 'nacl', phase: 'solid', amount_ml: null, amount_scoop: 10, amount_g: 2, amount_mol: null}],
        holding: [],
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
        composition: [{ substance_id: 'sand', phase: 'solid', amount_ml: null, amount_scoop: 10, amount_g: 2, amount_mol: null}],
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
          { substance_id: 'water', phase: 'liquid', amount_ml: 200, amount_scoop: null, amount_g: null, amount_mol: null},
        ],
        holding: [],
      },
    },
  ],
}

function stubAppFetch(options?: {
  authenticated?: boolean
  healthNetworkError?: Error
  registerBody?: unknown
  registerStatus?: number
  loginBody?: unknown
  loginStatus?: number
  logoutStatus?: number
}) {
  const authenticated = options?.authenticated ?? false
  return vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
    void init
    const url = String(input)
    if (url === '/api/health') {
      if (options?.healthNetworkError) {
        throw options.healthNetworkError
      }
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
    if (url === '/api/auth/register') {
      return jsonResponse(
        options?.registerBody ?? userPayload,
        options?.registerStatus ?? 201,
      )
    }
    if (url === '/api/auth/login') {
      return jsonResponse(
        options?.loginBody ?? userPayload,
        options?.loginStatus ?? 200,
      )
    }
    if (url === '/api/auth/logout') {
      const status = options?.logoutStatus ?? 204
      if (status === 204) {
        return new Response(null, { status })
      }
      return jsonResponse({ error: 'Could not log out', code: 'internal' }, status)
    }
    if (url === '/api/lab/scene') {
      return jsonResponse(EMPTY_LAB_SCENE)
    }
    if (url === '/api/lab/action') {
      return jsonResponse({ scene: EMPTY_LAB_SCENE })
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
    expect(screen.queryByLabelText('Lab bench')).not.toBeInTheDocument()
  })

  it('welcomes a signed-in user and shows the lab bench', async () => {
    vi.stubGlobal('fetch', stubAppFetch({ authenticated: true }))

    render(<App />)

    expect(await screen.findByText('Welcome back, Ada')).toBeTruthy()
    expect(screen.getByRole('button', { name: 'Sign out' })).toBeTruthy()
    expect(await screen.findByLabelText('Lab bench')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Dissolve' })).not.toBeInTheDocument()
    expect(screen.queryByLabelText('Solid')).not.toBeInTheDocument()
  })

  it('signed-in copy points at the bench, not a text dissolve picker', async () => {
    vi.stubGlobal('fetch', stubAppFetch({ authenticated: true }))

    render(<App />)

    expect(await screen.findByText(/lab bench below/i)).toBeInTheDocument()
    expect(screen.queryByText(/benches unlock/i)).not.toBeInTheDocument()
    expect(screen.queryByText(/pick a solid/i)).not.toBeInTheDocument()
  })

  it('creates an account and then shows the lab bench', async () => {
    const fetchMock = stubAppFetch({ authenticated: false })
    vi.stubGlobal('fetch', fetchMock)

    render(<App />)
    expect(await screen.findByText('Display name')).toBeTruthy()

    fireEvent.change(screen.getByLabelText('Display name'), { target: { value: 'Ada' } })
    fireEvent.change(screen.getByLabelText('Email'), {
      target: { value: 'ada@chemlab.local' },
    })
    fireEvent.change(screen.getByLabelText('Password'), { target: { value: 'secret123' } })
    fireEvent.submit(screen.getByLabelText('Email').closest('form')!)

    expect(await screen.findByText('Welcome back, Ada')).toBeTruthy()
    expect(await screen.findByLabelText('Lab bench')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Dissolve' })).not.toBeInTheDocument()
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/auth/register',
      expect.objectContaining({
        method: 'POST',
        credentials: 'include',
        headers: expect.objectContaining({ 'X-CSRF-Token': 'tok-123' }),
        body: JSON.stringify({
          email: 'ada@chemlab.local',
          password: 'secret123',
          display_name: 'Ada',
        }),
      }),
    )
  })

  it('signs in from the login tab', async () => {
    const fetchMock = stubAppFetch({ authenticated: false })
    vi.stubGlobal('fetch', fetchMock)

    render(<App />)
    expect(await screen.findByText('Display name')).toBeTruthy()

    fireEvent.click(screen.getByRole('button', { name: 'Sign in' }))
    expect(screen.queryByLabelText('Display name')).not.toBeInTheDocument()

    fireEvent.change(screen.getByLabelText('Email'), {
      target: { value: 'ada@chemlab.local' },
    })
    fireEvent.change(screen.getByLabelText('Password'), { target: { value: 'secret123' } })
    fireEvent.submit(screen.getByLabelText('Email').closest('form')!)

    expect(await screen.findByText('Welcome back, Ada')).toBeTruthy()
    expect(await screen.findByLabelText('Lab bench')).toBeInTheDocument()
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/auth/login',
      expect.objectContaining({
        method: 'POST',
        credentials: 'include',
        headers: expect.objectContaining({ 'X-CSRF-Token': 'tok-123' }),
        body: JSON.stringify({
          email: 'ada@chemlab.local',
          password: 'secret123',
        }),
      }),
    )
  })

  it('signs in from the login tab and surfaces auth errors', async () => {
    const fetchMock = stubAppFetch({
      authenticated: false,
      loginBody: { error: 'Invalid email or password', code: 'invalid_credentials' },
      loginStatus: 401,
    })
    vi.stubGlobal('fetch', fetchMock)

    render(<App />)
    expect(await screen.findByText('Display name')).toBeTruthy()

    fireEvent.click(screen.getByRole('button', { name: 'Sign in' }))
    expect(screen.queryByLabelText('Display name')).not.toBeInTheDocument()

    fireEvent.change(screen.getByLabelText('Email'), {
      target: { value: 'ada@chemlab.local' },
    })
    fireEvent.change(screen.getByLabelText('Password'), { target: { value: 'secret123' } })
    fireEvent.submit(screen.getByLabelText('Email').closest('form')!)

    expect(await screen.findByRole('alert')).toHaveTextContent('Invalid email or password')
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/auth/login',
      expect.objectContaining({
        method: 'POST',
        credentials: 'include',
        headers: expect.objectContaining({ 'X-CSRF-Token': 'tok-123' }),
      }),
    )
  })

  it('sign-out hides the lab bench and returns the account form', async () => {
    vi.stubGlobal('fetch', stubAppFetch({ authenticated: true }))

    render(<App />)
    expect(await screen.findByLabelText('Lab bench')).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: 'Sign out' }))

    expect(await screen.findByText('Display name')).toBeTruthy()
    expect(screen.queryByLabelText('Lab bench')).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Dissolve' })).not.toBeInTheDocument()
  })

  it('marks the API offline when session bootstrap fails', async () => {
    vi.stubGlobal(
      'fetch',
      stubAppFetch({ healthNetworkError: new Error('Failed to fetch') }),
    )

    render(<App />)

    expect(await screen.findByText('API offline')).toBeTruthy()
    expect(await screen.findByText('Display name')).toBeTruthy()
  })
})
