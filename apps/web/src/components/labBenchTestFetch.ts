import { fireEvent, screen } from '@testing-library/react'
import { expect, vi } from 'vitest'
import type { LabAction, LabScene } from '../generated/contracts'
import { optionalArray } from '../lib/scene'
import { FREE_MODE } from '../lib/challenges'
import {
  challengeScene,
  cloneScene,
  initialScene,
  withFilledMainBeaker,
} from './labBenchTestFixtures'
import {
  afterCacl2Pour,
  afterNaclPour,
  afterSandPour,
  applySolidsDeposit,
  applySolidsPutAway,
  applySolidsScoop,
  withPutBack,
  withScoop,
} from './labBenchTestSpoon'
import {
  applyPipetteEmpty,
  applyPipettePutAway,
  applyPipetteUse,
  applyTongsPutAway,
  applyTongsUse,
  withBurnerToggle,
} from './labBenchTestTools'

/** Mirror of the server's mode-aware reset: back to the start scene of the current mode. */
function startSceneForMode(mode: string): LabScene {
  return mode === FREE_MODE ? initialScene() : challengeScene()
}

export function jsonResponse(body: unknown, status = 200) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  })
}

export function stubLabFetch(options?: {
  scene?: LabScene
  actionHandler?: (action: LabAction, scene: LabScene) => LabScene | { error: string; code: string; status: number }
  sceneError?: { error: string; code: string; status: number }
}) {
  let scene = cloneScene(options?.scene ?? withFilledMainBeaker(initialScene()))
  return vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = String(input)
    if (url === '/api/auth/csrf') {
      return jsonResponse({ csrf_token: 'tok-123' })
    }
    if (url === '/api/lab/scene') {
      if (options?.sceneError) {
        return jsonResponse(
          { error: options.sceneError.error, code: options.sceneError.code },
          options.sceneError.status,
        )
      }
      return jsonResponse(scene)
    }
    if (url === '/api/lab/action') {
      const action = init?.body ? (JSON.parse(String(init.body)) as LabAction) : null
      if (!action) {
        return jsonResponse({ error: 'bad request', code: 'invalid_input' }, 400)
      }
      if (options?.actionHandler) {
        const result = options.actionHandler(action, scene)
        if ('error' in result) {
          return jsonResponse({ error: result.error, code: result.code }, result.status)
        }
        scene = result
        return jsonResponse({ scene })
      }
      if (action.type === 'toggle_burner') {
        scene = withBurnerToggle(scene)
        return jsonResponse({ scene })
      }
      if (action.type === 'use_tool' && action.tool_item_id === 'pipette-1') {
        const result = applyPipetteUse(scene, action.target_item_id)
        if ('error' in result) {
          return jsonResponse({ error: result.error, code: result.code }, result.status)
        }
        scene = result
        return jsonResponse({ scene })
      }
      if (action.type === 'use_tool' && action.tool_item_id === 'tongs-1') {
        const result = applyTongsUse(scene, action.target_item_id)
        if ('error' in result) {
          return jsonResponse({ error: result.error, code: result.code }, result.status)
        }
        scene = result
        return jsonResponse({ scene })
      }
      if (
        action.type === 'use_tool' &&
        action.tool_item_id === 'spoon-1' &&
        (action.target_item_id === 'dish-1' ||
          action.target_item_id === 'filter-paper-1' ||
          action.target_item_id === 'beaker-filtrate' ||
          action.target_item_id === 'beaker-water')
      ) {
        const held = optionalArray(
          scene.items.find((item) => item.id === 'spoon-1')?.properties.holding,
        )
        if (held.length > 0) {
          scene = applySolidsDeposit(scene, action.target_item_id)
          return jsonResponse({ scene })
        }
        const result = applySolidsScoop(scene, action.target_item_id)
        if ('error' in result) {
          return jsonResponse({ error: result.error, code: result.code }, result.status)
        }
        scene = result
        return jsonResponse({ scene })
      }
      if (action.type === 'pour' && action.source_item_id === 'pipette-1') {
        scene = applyPipetteEmpty(scene, action.target_item_id)
        return jsonResponse({ scene })
      }
      if (action.type === 'put_away' && action.tool_item_id === 'pipette-1') {
        scene = applyPipettePutAway(scene)
        return jsonResponse({ scene })
      }
      if (action.type === 'put_away' && action.tool_item_id === 'tongs-1') {
        scene = applyTongsPutAway(scene)
        return jsonResponse({ scene })
      }
      if (action.type === 'use_tool' && (action.target_item_id === 'beaker-nacl' || action.target_item_id === 'beaker-cacl2' || action.target_item_id === 'beaker-sand')) {
        const targetSubstance =
          action.target_item_id === 'beaker-nacl'
            ? 'nacl'
            : action.target_item_id === 'beaker-cacl2'
              ? 'cacl2'
              : 'sand'
        const held = optionalArray(
          scene.items.find((item) => item.id === 'spoon-1')?.properties.holding,
        )
        const species = [...new Set(held.filter((entry) => entry.phase === 'solid').map((entry) => entry.substance_id))]
        if (species.length > 1) {
          return jsonResponse({ error: 'invalid action', code: 'invalid_action' }, 400)
        }
        const first = held[0]
        if (first) {
          if (first.substance_id === targetSubstance) {
            scene = withPutBack(scene, targetSubstance)
            return jsonResponse({ scene })
          }
          return jsonResponse({ error: 'invalid action', code: 'invalid_action' }, 400)
        }
        scene = withScoop(scene, targetSubstance)
        return jsonResponse({ scene })
      }
      if (action.type === 'put_away') {
        const spoon = scene.items.find((item) => item.id === 'spoon-1')
        if (spoon?.properties.source_item_id === 'dish-1') {
          scene = applySolidsPutAway(scene, 'dish-1')
          return jsonResponse({ scene })
        }
        if (spoon?.properties.source_item_id === 'filter-paper-1') {
          scene = applySolidsPutAway(scene, 'filter-paper-1')
          return jsonResponse({ scene })
        }
        const held = optionalArray(spoon?.properties.holding)[0]
        if (held && (held.substance_id === 'nacl' || held.substance_id === 'cacl2' || held.substance_id === 'sand')) {
          scene = withPutBack(scene, held.substance_id)
          return jsonResponse({ scene })
        }
        const next = cloneScene(scene)
        const nextSpoon = next.items.find((item) => item.id === 'spoon-1')!
        nextSpoon.location = 'bench'
        next.last_events = []
        next.version += 1
        scene = next
        return jsonResponse({ scene })
      }
      if (action.type === 'pour') {
        const held = optionalArray(
          scene.items.find((item) => item.id === 'spoon-1')?.properties.holding,
        )[0]
        if (held?.substance_id === 'nacl') {
          scene = afterNaclPour(scene)
          return jsonResponse({ scene })
        }
        if (held?.substance_id === 'cacl2') {
          scene = afterCacl2Pour(scene)
          return jsonResponse({ scene })
        }
        if (held?.substance_id === 'sand') {
          scene = afterSandPour(scene)
          return jsonResponse({ scene })
        }
      }
      if (action.type === 'reset') {
        const next = startSceneForMode(scene.mode)
        next.lab_id = scene.lab_id
        next.version = scene.version + 1
        next.last_events = [{ kind: 'reset', message: 'Lab reset to the starting bench.' }]
        scene = next
        return jsonResponse({ scene })
      }
      return jsonResponse({ error: 'invalid action', code: 'invalid_action' }, 400)
    }
    if (url === '/api/lab/dissolve') {
      throw new Error('LabBench must not call /api/lab/dissolve')
    }
    return jsonResponse({ error: 'not found', code: 'not_found' }, 404)
  })
}

export function lastActionInit(fetchMock: ReturnType<typeof vi.fn>) {
  const calls = fetchMock.mock.calls.filter(([url]) => String(url) === '/api/lab/action')
  return calls.at(-1)?.[1] as RequestInit | undefined
}

export function expectCsrfLabAction(fetchMock: ReturnType<typeof vi.fn>, body: unknown) {
  expect(lastActionInit(fetchMock)).toEqual(
    expect.objectContaining({
      method: 'POST',
      credentials: 'include',
      headers: expect.objectContaining({
        'content-type': 'application/json',
        'X-CSRF-Token': 'tok-123',
      }),
      body: JSON.stringify(body),
    }),
  )
}

export function precedesInDocument(earlier: HTMLElement, later: HTMLElement) {
  return Boolean(earlier.compareDocumentPosition(later) & Node.DOCUMENT_POSITION_FOLLOWING)
}

export function clickCarousel(direction: 'next' | 'previous') {
  const name = direction === 'next' ? 'Next ingredient' : 'Previous ingredient'
  fireEvent.click(screen.getByRole('button', { name }))
}

export function showStockInCarousel(name: string) {
  for (let i = 0; i < 4; i++) {
    if (screen.queryByRole('button', { name })) return
    clickCarousel('next')
  }
  throw new Error(`stock ${name} not visible after wrapping carousel`)
}

export function clickStock(name: string) {
  showStockInCarousel(name)
  fireEvent.click(screen.getByRole('button', { name }))
}

export function clickToolCarousel(direction: 'next' | 'previous') {
  const name = direction === 'next' ? 'Next tool' : 'Previous tool'
  fireEvent.click(screen.getByRole('button', { name }))
}

export function showToolInCarousel(name: string) {
  for (let i = 0; i < 3; i++) {
    if (screen.queryByRole('button', { name })) return
    clickToolCarousel('next')
  }
  throw new Error(`tool ${name} not visible after wrapping carousel`)
}

export function clickSpoon() {
  showToolInCarousel('Spoon')
  fireEvent.click(screen.getByRole('button', { name: 'Spoon' }))
}

export function clickTongs() {
  showToolInCarousel('Tongs')
  fireEvent.click(screen.getByRole('button', { name: 'Tongs' }))
}
