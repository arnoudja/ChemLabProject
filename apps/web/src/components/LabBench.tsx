import { useEffect, useState, type MouseEvent } from 'react'
import type { Item, LabScene } from '../generated/contracts'
import { fetchLabScene, postLabAction } from '../lib/api'
import { optionalArray } from '../lib/scene'

const SPOON_ID = 'spoon-1'
const NACL_ID = 'beaker-nacl'
const SAND_ID = 'beaker-sand'
const WATER_ID = 'beaker-water'

type ToolUi = 'none' | 'spoon' | 'nacl' | 'sand'

function findItem(scene: LabScene, id: string): Item | undefined {
  return scene.items.find((item) => item.id === id)
}

function spoonHoldingSubstance(scene: LabScene): 'nacl' | 'sand' | null {
  const spoon = findItem(scene, SPOON_ID)
  const held = optionalArray(spoon?.properties.holding)[0]
  if (!held || held.phase !== 'solid') return null
  if (held.substance_id === 'nacl' || held.substance_id === 'sand') {
    return held.substance_id
  }
  return null
}

/** Undissolved solid grains come only from server composition on the water item. */
function undissolvedSolidInWater(scene: LabScene): 'nacl' | 'sand' | null {
  const water = findItem(scene, WATER_ID)
  const solid = optionalArray(water?.properties.composition).find((entry) => entry.phase === 'solid')
  if (!solid) return null
  if (solid.substance_id === 'nacl' || solid.substance_id === 'sand') {
    return solid.substance_id
  }
  return null
}

function WaterBeakerSvg({
  leftoverSolid,
  busy,
}: {
  leftoverSolid: 'nacl' | 'sand' | null
  busy: boolean
}) {
  return (
    <svg viewBox="0 0 120 168" className="h-40 w-28" aria-hidden>
      <defs>
        <linearGradient id="bench-glass" x1="20" y1="8" x2="100" y2="160" gradientUnits="userSpaceOnUse">
          <stop stopColor="#3E4058" stopOpacity="0.55" />
          <stop offset="1" stopColor="#151622" stopOpacity="0.25" />
        </linearGradient>
        <linearGradient id="bench-water" x1="40" y1="80" x2="80" y2="150" gradientUnits="userSpaceOnUse">
          <stop stopColor="#7CF8F7" stopOpacity="0.72" />
          <stop offset="1" stopColor="#85E1FB" stopOpacity="0.38" />
        </linearGradient>
      </defs>
      <path d="M28 18h64v10H28z" fill="#86A7DF" opacity="0.85" />
      <path
        d="M32 28h56l10 118c1 8-5 14-13 14H35c-8 0-14-6-13-14L32 28z"
        fill="url(#bench-glass)"
        stroke="#C4D2ED"
        strokeWidth="2.4"
      />
      <path
        className={busy ? 'lab-water-busy' : undefined}
        d="M36 86h48l6 62c0 6-4 10-10 10H40c-6 0-10-4-10-10l6-62z"
        fill="url(#bench-water)"
      />
      {leftoverSolid ? (
        <g fill={leftoverSolid === 'nacl' ? '#F4FBFF' : '#C9B48A'} opacity="0.9">
          <circle cx="48" cy="142" r="3.2" />
          <circle cx="62" cy="146" r="2.6" />
          <circle cx="74" cy="141" r="3" />
          <circle cx="56" cy="136" r="2.2" />
        </g>
      ) : null}
      <path d="M86 48c14-4 22 10 16 22" stroke="#DDF7FF" strokeWidth="1.6" strokeLinecap="round" opacity="0.45" />
    </svg>
  )
}

function SolidBeakerSvg({ solid }: { solid: 'nacl' | 'sand' }) {
  const pile = solid === 'nacl' ? '#F4FBFF' : '#C9A36A'
  const speck = solid === 'nacl' ? '#DDF7FF' : '#8C6A3A'
  return (
    <svg viewBox="0 0 80 118" className="h-28 w-20" aria-hidden>
      <path d="M22 10h36v8H22z" fill="#86A7DF" opacity="0.8" />
      <path
        d="M24 18h32l8 80c1 6-3 10-9 10H25c-6 0-10-4-9-10l8-80z"
        fill="#3E4058"
        fillOpacity="0.35"
        stroke="#C4D2ED"
        strokeWidth="2"
      />
      <path d="M28 72h24l4 26c0 4-3 7-7 7H31c-4 0-7-3-7-7l4-26z" fill={pile} />
      <circle cx="34" cy="92" r="2" fill={speck} />
      <circle cx="46" cy="96" r="1.6" fill={speck} />
      <circle cx="40" cy="86" r="1.4" fill={speck} opacity="0.7" />
    </svg>
  )
}

function SpoonSvg({ fill, floating }: { fill: 'nacl' | 'sand' | null; floating?: boolean }) {
  const bowl = fill === 'nacl' ? '#F4FBFF' : fill === 'sand' ? '#C9A36A' : '#C4D2ED'
  return (
    <svg
      viewBox="0 0 132 40"
      className={floating ? 'h-8 w-28' : 'h-10 w-32'}
      aria-hidden
    >
      <ellipse cx="24" cy="20" rx="20" ry="13" fill="#6A6E95" stroke="#DDF7FF" strokeWidth="2" />
      <ellipse cx="24" cy="20" rx="13" ry="8" fill={bowl} />
      <path d="M42 18h82c6 0 8 4 8 7s-2 7-8 7H42" fill="#6A6E95" stroke="#DDF7FF" strokeWidth="1.6" />
    </svg>
  )
}

function outcomeLabel(kind: string): string | null {
  if (kind === 'dissolved') return 'dissolved'
  if (kind === 'did_not_dissolve') return 'did not dissolve'
  return null
}

export function LabBench() {
  const [scene, setScene] = useState<LabScene | null>(null)
  const [selectedToolItemId, setSelectedToolItemId] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [pointer, setPointer] = useState<{ x: number; y: number } | null>(null)

  useEffect(() => {
    let cancelled = false
    ;(async () => {
      setError(null)
      try {
        const next = await fetchLabScene()
        if (!cancelled) setScene(next)
      } catch (err) {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : 'Could not load lab scene')
        }
      }
    })()
    return () => {
      cancelled = true
    }
  }, [])

  function trackPointer(event: MouseEvent<HTMLElement>) {
    const root = event.currentTarget.closest('[data-lab-bench]')
    if (!(root instanceof HTMLElement)) return
    const rect = root.getBoundingClientRect()
    setPointer({ x: event.clientX - rect.left, y: event.clientY - rect.top })
  }

  function onSpoon(event: MouseEvent<HTMLButtonElement>) {
    trackPointer(event)
    setSelectedToolItemId((current) => (current === SPOON_ID ? null : SPOON_ID))
  }

  async function onSolid(targetItemId: string, event: MouseEvent<HTMLButtonElement>) {
    trackPointer(event)
    if (selectedToolItemId !== SPOON_ID || busy) return
    setError(null)
    setBusy(true)
    try {
      const response = await postLabAction({
        type: 'use_tool',
        tool_item_id: SPOON_ID,
        target_item_id: targetItemId,
      })
      setScene(response.scene)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Action failed')
    } finally {
      setBusy(false)
    }
  }

  async function onWater(event: MouseEvent<HTMLButtonElement>) {
    trackPointer(event)
    if (!scene || selectedToolItemId !== SPOON_ID || busy) return
    if (!spoonHoldingSubstance(scene)) return
    setError(null)
    setBusy(true)
    try {
      const response = await postLabAction({
        type: 'pour',
        source_item_id: SPOON_ID,
        target_item_id: WATER_ID,
      })
      setScene(response.scene)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Action failed')
    } finally {
      setBusy(false)
    }
  }

  const holdingSelected = selectedToolItemId === SPOON_ID
  const spoonFill = scene ? spoonHoldingSubstance(scene) : null
  const leftoverSolid = scene ? undissolvedSolidInWater(scene) : null
  const toolUi: ToolUi = !holdingSelected ? 'none' : spoonFill ?? 'spoon'
  const nacl = scene ? findItem(scene, NACL_ID) : undefined
  const sand = scene ? findItem(scene, SAND_ID) : undefined
  const water = scene ? findItem(scene, WATER_ID) : undefined
  const spoon = scene ? findItem(scene, SPOON_ID) : undefined
  const lastEvents = scene ? optionalArray(scene.last_events) : []

  return (
    <section
      data-lab-bench
      data-tool={toolUi}
      aria-label="Lab bench"
      className={`lab-bench relative w-full overflow-hidden rounded-2xl border border-[var(--border)] bg-[var(--surface)] p-4 shadow-[var(--shadow)] backdrop-blur-md sm:p-5 ${
        holdingSelected ? 'lab-bench--holding' : ''
      }`}
      onMouseMove={holdingSelected ? trackPointer : undefined}
    >
      <p className="mb-3 text-sm text-[var(--ink-soft)]">
        Pick up the spoon, scoop a solid, then click the water. The server owns the scene and
        decides what happens.
      </p>

      {!scene && !error ? (
        <p className="text-sm text-[var(--ink-soft)]">Loading lab scene…</p>
      ) : null}

      {scene ? (
        <div className="lab-bench-surface flex flex-wrap items-end justify-center gap-6 rounded-xl px-4 pb-4 pt-8 sm:gap-10">
          <button
            type="button"
            className="lab-item"
            aria-label="Water beaker"
            disabled={busy}
            onClick={onWater}
          >
            <WaterBeakerSvg leftoverSolid={leftoverSolid} busy={busy} />
            <span className="lab-item-label">{water?.label ?? 'Water'}</span>
          </button>

          <button
            type="button"
            className="lab-item"
            aria-label="Sodium chloride (NaCl)"
            disabled={busy}
            onClick={(event) => onSolid(NACL_ID, event)}
          >
            <SolidBeakerSvg solid="nacl" />
            <span className="lab-item-label">{nacl?.label ?? 'NaCl'}</span>
          </button>

          <button
            type="button"
            className="lab-item"
            aria-label="Sand"
            disabled={busy}
            onClick={(event) => onSolid(SAND_ID, event)}
          >
            <SolidBeakerSvg solid="sand" />
            <span className="lab-item-label">{sand?.label ?? 'Sand'}</span>
          </button>

          <button
            type="button"
            className={`lab-item ${holdingSelected ? 'opacity-40' : ''}`}
            aria-label="Spoon"
            aria-pressed={holdingSelected}
            disabled={busy}
            onClick={onSpoon}
          >
            <SpoonSvg fill={null} />
            <span className="lab-item-label">{spoon?.label ?? 'Spoon'}</span>
          </button>
        </div>
      ) : null}

      {holdingSelected && pointer ? (
        <div
          className="lab-cursor-spoon pointer-events-none absolute z-20"
          style={{ left: pointer.x, top: pointer.y }}
          aria-hidden
        >
          <SpoonSvg fill={spoonFill} floating />
        </div>
      ) : null}

      {error ? (
        <p className="mt-3 text-sm text-[var(--danger)]" role="alert">
          {error}
        </p>
      ) : null}

      {lastEvents.length > 0 ? (
        <div className="mt-3 space-y-2 text-sm" role="status" aria-live="polite">
          {lastEvents.map((event, index) => {
            const outcome = outcomeLabel(event.kind)
            return (
              <div key={`${event.kind}-${index}`} className="space-y-1">
                {outcome ? (
                  <p className="font-medium text-[var(--ink)]">Server outcome: {outcome}</p>
                ) : null}
                <p className="text-[var(--ink-soft)]">{event.message}</p>
              </div>
            )
          })}
        </div>
      ) : null}
    </section>
  )
}
