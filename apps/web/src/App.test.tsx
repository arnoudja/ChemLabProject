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

const NACL_SERVER = {
  dissolved: true,
  explanation: 'Sodium chloride (NaCl) dissolves in water at bench temperature.',
} as const

const SAND_SERVER = {
  dissolved: false,
  explanation: 'Sand (silica) does not dissolve in water at bench temperature.',
} as const

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
        composition: [{ substance_id: 'nacl', phase: 'solid', amount_ml: null, amount_scoop: 10 }],
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
        composition: [{ substance_id: 'sand', phase: 'solid', amount_ml: null, amount_scoop: 10 }],
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

function stubAppFetch(options?: {
  authenticated?: boolean
  dissolveBody?: unknown
  dissolveStatus?: number
  dissolveBySubstance?: Record<string, { body: unknown; status?: number }>
  dissolveNetworkError?: Error
  healthNetworkError?: Error
  registerBody?: unknown
  registerStatus?: number
  loginBody?: unknown
  loginStatus?: number
  logoutStatus?: number
}) {
  const authenticated = options?.authenticated ?? false
  return vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
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
    if (url === '/api/lab/dissolve') {
      if (options?.dissolveNetworkError) {
        throw options.dissolveNetworkError
      }
      const sent = init?.body ? JSON.parse(String(init.body)) : {}
      const bySubstance = options?.dissolveBySubstance?.[sent.substance_id]
      if (bySubstance) {
        return jsonResponse(bySubstance.body, bySubstance.status ?? 200)
      }
      return jsonResponse(
        options?.dissolveBody ?? NACL_SERVER,
        options?.dissolveStatus ?? 200,
      )
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

function lastDissolveInit(fetchMock: ReturnType<typeof vi.fn>) {
  const calls = fetchMock.mock.calls.filter(([url]) => String(url) === '/api/lab/dissolve')
  return calls.at(-1)?.[1] as RequestInit | undefined
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

  it('shows nacl then sand from the server payload only', async () => {
    const fetchMock = stubAppFetch({
      authenticated: true,
      dissolveBySubstance: {
        nacl: { body: NACL_SERVER },
        sand: { body: SAND_SERVER },
      },
    })
    vi.stubGlobal('fetch', fetchMock)

    render(<App />)
    expect(await screen.findByRole('button', { name: 'Dissolve' })).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: 'Dissolve' }))
    await waitFor(() => {
      expect(screen.getByRole('status')).toHaveTextContent(NACL_SERVER.explanation)
    })
    expect(screen.getByRole('status')).toHaveTextContent('Server outcome: dissolved')
    expect(screen.getByRole('status')).not.toHaveTextContent(SAND_SERVER.explanation)
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

    fireEvent.change(screen.getByLabelText('Solid'), { target: { value: 'sand' } })
    fireEvent.click(screen.getByRole('button', { name: 'Dissolve' }))
    await waitFor(() => {
      expect(screen.getByRole('status')).toHaveTextContent(SAND_SERVER.explanation)
    })
    expect(screen.getByRole('status')).toHaveTextContent('Server outcome: did not dissolve')
    expect(screen.getByRole('status')).not.toHaveTextContent(NACL_SERVER.explanation)
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

  it.each([
    {
      name: 'unknown substance',
      body: { error: 'unknown substance', code: 'unknown_substance' },
      status: 400,
    },
    {
      name: 'unsupported solvent',
      body: { error: 'unsupported solvent', code: 'unsupported_solvent' },
      status: 400,
    },
    {
      name: 'unsupported temperature',
      body: { error: 'unsupported temperature', code: 'unsupported_temperature' },
      status: 400,
    },
    {
      name: 'logged-out session',
      body: { error: 'Login required', code: 'unauthenticated' },
      status: 401,
    },
  ])('shows the server $name dissolve error and no outcome', async ({ body, status }) => {
    vi.stubGlobal(
      'fetch',
      stubAppFetch({
        authenticated: true,
        dissolveBody: body,
        dissolveStatus: status,
      }),
    )

    render(<App />)
    expect(await screen.findByRole('button', { name: 'Dissolve' })).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: 'Dissolve' }))

    expect(await screen.findByRole('alert')).toHaveTextContent(body.error)
    expect(screen.queryByRole('status')).not.toBeInTheDocument()
  })

  it('shows a dissolve network failure and clears a prior server outcome', async () => {
    const fetchMock = stubAppFetch({
      authenticated: true,
      dissolveBySubstance: { nacl: { body: NACL_SERVER } },
    })
    vi.stubGlobal('fetch', fetchMock)

    render(<App />)
    expect(await screen.findByRole('button', { name: 'Dissolve' })).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Dissolve' }))
    expect(await screen.findByRole('status')).toHaveTextContent(NACL_SERVER.explanation)

    fetchMock.mockImplementation(async (input: RequestInfo | URL, init?: RequestInit) => {
      if (String(input) === '/api/lab/dissolve') {
        throw new Error('Failed to fetch')
      }
      return stubAppFetch({ authenticated: true })(input, init)
    })
    fireEvent.click(screen.getByRole('button', { name: 'Dissolve' }))

    expect(await screen.findByRole('alert')).toHaveTextContent('Failed to fetch')
    expect(screen.queryByRole('status')).not.toBeInTheDocument()
  })

  it('creates an account and then shows the dissolve control', async () => {
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
    expect(screen.getByRole('button', { name: 'Dissolve' })).toBeInTheDocument()
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
    expect(screen.getByRole('button', { name: 'Dissolve' })).toBeInTheDocument()
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

  it('sign-out clears the dissolve result and returns the account form', async () => {
    vi.stubGlobal('fetch', stubAppFetch({ authenticated: true, dissolveBody: NACL_SERVER }))

    render(<App />)
    expect(await screen.findByRole('button', { name: 'Dissolve' })).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Dissolve' }))
    expect(await screen.findByRole('status')).toHaveTextContent(NACL_SERVER.explanation)

    fireEvent.click(screen.getByRole('button', { name: 'Sign out' }))

    expect(await screen.findByText('Display name')).toBeTruthy()
    expect(screen.queryByRole('status')).not.toBeInTheDocument()
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
