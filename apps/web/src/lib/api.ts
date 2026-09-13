import type {
  AuthUserResponse,
  CsrfResponse,
  DissolveRequest,
  DissolveResponse,
  ErrorResponse,
  HealthResponse,
  LabAction,
  LabActionResponse,
  LabScene,
  LoginRequest,
  MeResponse,
  RegisterRequest,
} from '../generated/contracts'

export const CSRF_HEADER = 'X-CSRF-Token'

let csrfTokenCache: string | null = null

export function clearCsrfTokenCache(): void {
  csrfTokenCache = null
}

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

export async function fetchCsrfToken(): Promise<string> {
  const response = await fetch('/api/auth/csrf', { credentials: 'include' })
  const data = await parseJson<CsrfResponse>(response)
  csrfTokenCache = data.csrf_token
  return csrfTokenCache
}

async function ensureCsrfToken(): Promise<string> {
  return csrfTokenCache ?? (await fetchCsrfToken())
}

/** Headers for mutating `/api/*` requests (auth + lab POSTs). */
export async function mutateHeaders(jsonBody = false): Promise<Record<string, string>> {
  const token = await ensureCsrfToken()
  const headers: Record<string, string> = { [CSRF_HEADER]: token }
  if (jsonBody) {
    headers['content-type'] = 'application/json'
  }
  return headers
}

export async function register(body: RegisterRequest): Promise<AuthUserResponse> {
  const response = await fetch('/api/auth/register', {
    method: 'POST',
    credentials: 'include',
    headers: await mutateHeaders(true),
    body: JSON.stringify(body),
  })
  return parseJson<AuthUserResponse>(response)
}

export async function login(body: LoginRequest): Promise<AuthUserResponse> {
  const response = await fetch('/api/auth/login', {
    method: 'POST',
    credentials: 'include',
    headers: await mutateHeaders(true),
    body: JSON.stringify(body),
  })
  return parseJson<AuthUserResponse>(response)
}

export async function logout(): Promise<void> {
  const response = await fetch('/api/auth/logout', {
    method: 'POST',
    credentials: 'include',
    headers: await mutateHeaders(),
  })
  if (!response.ok && response.status !== 204) {
    throw new Error('Could not log out')
  }
}

/** Predict-only dissolve endpoint (welcome picker). LabBench uses scene actions instead. */
export async function dissolve(body: DissolveRequest): Promise<DissolveResponse> {
  const response = await fetch('/api/lab/dissolve', {
    method: 'POST',
    credentials: 'include',
    headers: await mutateHeaders(true),
    body: JSON.stringify(body),
  })
  return parseJson<DissolveResponse>(response)
}

export async function fetchLabScene(): Promise<LabScene> {
  const response = await fetch('/api/lab/scene', { credentials: 'include' })
  return parseJson<LabScene>(response)
}

export async function postLabAction(action: LabAction): Promise<LabActionResponse> {
  const response = await fetch('/api/lab/action', {
    method: 'POST',
    credentials: 'include',
    headers: await mutateHeaders(true),
    body: JSON.stringify(action),
  })
  return parseJson<LabActionResponse>(response)
}
