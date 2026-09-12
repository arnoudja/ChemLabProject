import type {
  AuthUserResponse,
  ErrorResponse,
  HealthResponse,
  LoginRequest,
  MeResponse,
  RegisterRequest,
} from '../generated/contracts'

async function parseJson<T>(response: Response): Promise<T> {
  const data = (await response.json()) as T | ErrorResponse
  if (!response.ok) {
    const err = data as ErrorResponse
    throw new Error(err.error || `Request failed (${response.status})`)
  }
  return data as T
}

export async function fetchHealth(): Promise<HealthResponse> {
  const response = await fetch('/api/health')
  return parseJson<HealthResponse>(response)
}

export async function fetchMe(): Promise<MeResponse> {
  const response = await fetch('/api/auth/me', { credentials: 'include' })
  return parseJson<MeResponse>(response)
}

export async function register(body: RegisterRequest): Promise<AuthUserResponse> {
  const response = await fetch('/api/auth/register', {
    method: 'POST',
    credentials: 'include',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(body),
  })
  return parseJson<AuthUserResponse>(response)
}

export async function login(body: LoginRequest): Promise<AuthUserResponse> {
  const response = await fetch('/api/auth/login', {
    method: 'POST',
    credentials: 'include',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(body),
  })
  return parseJson<AuthUserResponse>(response)
}

export async function logout(): Promise<void> {
  const response = await fetch('/api/auth/logout', {
    method: 'POST',
    credentials: 'include',
  })
  if (!response.ok && response.status !== 204) {
    throw new Error('Could not log out')
  }
}
