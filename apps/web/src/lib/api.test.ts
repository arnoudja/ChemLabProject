import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest'
import { fetchHealth } from './api'

describe('api client', () => {
  beforeEach(() => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () =>
        new Response(JSON.stringify({ status: 'ok', version: '0.1.0', service: 'chemlab-server' }), {
          status: 200,
          headers: { 'content-type': 'application/json' },
        }),
      ),
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
})
