import { useState, type MouseEvent } from 'react'
import type { DissolveResponse } from '../generated/contracts'
import { dissolve } from '../lib/api'

type Tool = 'none' | 'spoon' | 'nacl' | 'sand'
type SolidId = 'nacl' | 'sand'

function isSolid(tool: Tool): tool is SolidId {
  return tool === 'nacl' || tool === 'sand'
}

function WaterBeakerSvg({
  leftover,
  leftoverSolid,
  busy,
}: {
  leftover: boolean
  leftoverSolid: SolidId | null
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
      {leftover ? (
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

function SolidBeakerSvg({ solid }: { solid: SolidId }) {
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

function SpoonSvg({ fill, floating }: { fill: SolidId | null; floating?: boolean }) {
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

export function LabBench() {
  const [tool, setTool] = useState<Tool>('none')
  const [result, setResult] = useState<DissolveResponse | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [pointer, setPointer] = useState<{ x: number; y: number } | null>(null)
  const [poured, setPoured] = useState<SolidId | null>(null)

  function trackPointer(event: MouseEvent<HTMLElement>) {
    const root = event.currentTarget.closest('[data-lab-bench]')
    if (!(root instanceof HTMLElement)) return
    const rect = root.getBoundingClientRect()
    setPointer({ x: event.clientX - rect.left, y: event.clientY - rect.top })
  }

  function onSpoon(event: MouseEvent<HTMLButtonElement>) {
    trackPointer(event)
    setTool((current) => (current === 'none' ? 'spoon' : 'none'))
  }

  function onSolid(id: SolidId, event: MouseEvent<HTMLButtonElement>) {
    trackPointer(event)
    if (tool === 'none') return
    setTool(id)
  }

  async function onWater(event: MouseEvent<HTMLButtonElement>) {
    trackPointer(event)
    if (!isSolid(tool) || busy) return
    const substanceId = tool
    setError(null)
    setBusy(true)
    try {
      const outcome = await dissolve({
        substance_id: substanceId,
        solvent_id: 'water',
        temperature_c: 20,
      })
      setPoured(substanceId)
      setResult(outcome)
      setTool('spoon')
    } catch (err) {
      setResult(null)
      setError(err instanceof Error ? err.message : 'Dissolve failed')
      setTool('spoon')
    } finally {
      setBusy(false)
    }
  }

  const holding = tool !== 'none'
  const spoonFill = isSolid(tool) ? tool : null
  const leftover = result?.dissolved === false

  return (
    <section
      data-lab-bench
      data-tool={tool}
      aria-label="Lab bench"
      className={`lab-bench relative w-full overflow-hidden rounded-2xl border border-[var(--border)] bg-[var(--surface)] p-4 shadow-[var(--shadow)] backdrop-blur-md sm:p-5 ${
        holding ? 'lab-bench--holding' : ''
      }`}
      onMouseMove={holding ? trackPointer : undefined}
    >
      <p className="mb-3 text-sm text-[var(--ink-soft)]">
        Pick up the spoon, scoop a solid, then click the water. The server decides what happens.
      </p>

      <div className="lab-bench-surface flex flex-wrap items-end justify-center gap-6 rounded-xl px-4 pb-4 pt-8 sm:gap-10">
        <button
          type="button"
          className="lab-item"
          aria-label="Water beaker"
          disabled={busy}
          onClick={onWater}
        >
          <WaterBeakerSvg leftover={leftover} leftoverSolid={poured} busy={busy} />
          <span className="lab-item-label">Water</span>
        </button>

        <button
          type="button"
          className="lab-item"
          aria-label="Sodium chloride (NaCl)"
          onClick={(event) => onSolid('nacl', event)}
        >
          <SolidBeakerSvg solid="nacl" />
          <span className="lab-item-label">NaCl</span>
        </button>

        <button
          type="button"
          className="lab-item"
          aria-label="Sand"
          onClick={(event) => onSolid('sand', event)}
        >
          <SolidBeakerSvg solid="sand" />
          <span className="lab-item-label">Sand</span>
        </button>

        <button
          type="button"
          className={`lab-item ${holding ? 'opacity-40' : ''}`}
          aria-label="Spoon"
          aria-pressed={holding}
          onClick={onSpoon}
        >
          <SpoonSvg fill={null} />
          <span className="lab-item-label">Spoon</span>
        </button>
      </div>

      {holding && pointer ? (
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

      {result ? (
        <div className="mt-3 space-y-1 text-sm" role="status" aria-live="polite">
          <p className="font-medium text-[var(--ink)]">
            Server outcome: {result.dissolved ? 'dissolved' : 'did not dissolve'}
          </p>
          <p className="text-[var(--ink-soft)]">{result.explanation}</p>
        </div>
      ) : null}
    </section>
  )
}
