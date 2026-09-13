import { useEffect, useState, type MouseEvent } from 'react'
import type { Item, LabScene } from '../generated/contracts'
import { fetchLabScene, postLabAction } from '../lib/api'
import {
  CompositionInspectLine,
  formatTemperatureC,
  solventVolumeLitres,
  StockSubstanceLabel,
  stockSubstanceAriaLabel,
} from '../lib/compositionDisplay'
import { optionalArray } from '../lib/scene'
import {
  SPOON_SCOOP_MASS_G,
  STOCK_FULL_MASS_G,
  STOCK_FULL_SCOOPS,
  WATER_FULL_ML,
} from '../lib/benchAmounts'

export { SPOON_SCOOP_MASS_G, STOCK_FULL_MASS_G, STOCK_FULL_SCOOPS, WATER_FULL_ML }

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

function itemTemperatureC(scene: LabScene, item: Item): number {
  return item.properties.temperature_c ?? scene.temperature_c
}

/** Fill fraction 0..1 from server amount_ml relative to initial water volume. */
export function waterFillRatio(amountMl: number | null | undefined): number {
  if (amountMl == null || amountMl <= 0) return 0
  return Math.min(1, amountMl / WATER_FULL_ML)
}

function WaterBeakerSvg({
  leftoverSolid,
  busy,
  amountMl,
}: {
  leftoverSolid: 'nacl' | 'sand' | null
  busy: boolean
  amountMl?: number | null
}) {
  const fill = waterFillRatio(amountMl)
  // Full liquid top ~86; empty sits at the beaker floor (~148).
  const floorY = 148
  const fullHeight = 62
  const topY = floorY - fullHeight * fill
  const leftTop = 30 + 6 * fill
  const rightTop = 90 - 6 * fill
  return (
    <svg
      viewBox="0 0 120 168"
      className="h-40 w-28"
      aria-hidden
      data-water-fill={fill.toFixed(2)}
    >
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
      {fill > 0 ? (
        <path
          className={busy ? 'lab-water-busy' : undefined}
          d={`M${leftTop} ${topY} H${rightTop} L90 ${floorY - 10} C90 ${floorY - 4} 86 ${floorY} 80 ${floorY} H40 C34 ${floorY} 30 ${floorY - 4} 30 ${floorY - 10} Z`}
          fill="url(#bench-water)"
        />
      ) : null}
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

/** Fill fraction 0..1 from server amount_g relative to initial stock. */
export function stockFillRatio(amountG: number | null | undefined): number {
  if (amountG == null || amountG <= 0) return 0
  return Math.min(1, amountG / STOCK_FULL_MASS_G)
}

function SolidBeakerSvg({
  solid,
  amountG,
}: {
  solid: 'nacl' | 'sand'
  amountG?: number | null
}) {
  const pile = solid === 'nacl' ? '#F4FBFF' : '#C9A36A'
  const speck = solid === 'nacl' ? '#DDF7FF' : '#8C6A3A'
  const fill = stockFillRatio(amountG)
  // Full pile top ~72; empty sits near the beaker floor (~100).
  const topY = 100 - 28 * fill
  const floorY = 105
  const midY = topY + (floorY - topY) * 0.55
  return (
    <svg
      viewBox="0 0 80 118"
      className="h-28 w-20"
      aria-hidden
      data-stock-solid={solid}
      data-stock-fill={fill.toFixed(2)}
    >
      <path d="M22 10h36v8H22z" fill="#86A7DF" opacity="0.8" />
      <path
        d="M24 18h32l8 80c1 6-3 10-9 10H25c-6 0-10-4-9-10l8-80z"
        fill="#3E4058"
        fillOpacity="0.35"
        stroke="#C4D2ED"
        strokeWidth="2"
      />
      {fill > 0 ? (
        <>
          <path
            d={`M28 ${topY} H52 L56 ${floorY - 7} C56 ${floorY - 3} 53 ${floorY} 49 ${floorY} H31 C27 ${floorY} 24 ${floorY - 3} 24 ${floorY - 7} Z`}
            fill={pile}
          />
          {fill > 0.15 ? (
            <>
              <circle cx="34" cy={midY + 6} r="2" fill={speck} />
              <circle cx="46" cy={midY + 10} r="1.6" fill={speck} />
              <circle cx="40" cy={midY} r="1.4" fill={speck} opacity="0.7" />
            </>
          ) : null}
        </>
      ) : null}
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


function stockAmountG(scene: LabScene, itemId: string, substanceId: 'nacl' | 'sand'): number | null {
  const item = findItem(scene, itemId)
  const entry = optionalArray(item?.properties.composition).find(
    (c) => c.substance_id === substanceId && c.phase === 'solid',
  )
  return entry?.amount_g ?? null
}

function waterAmountMl(scene: LabScene): number | null {
  const item = findItem(scene, WATER_ID)
  const entry = optionalArray(item?.properties.composition).find(
    (c) => c.substance_id === 'water' && c.phase === 'liquid',
  )
  return entry?.amount_ml ?? null
}

function outcomeLabel(kind: string): string | null {
  if (kind === 'dissolved') return 'dissolved'
  if (kind === 'did_not_dissolve') return 'did not dissolve'
  if (kind === 'returned') return 'returned'
  if (kind === 'reset') return 'reset'
  return null
}

function BeakerInspectPanel({
  scene,
  item,
  onClose,
}: {
  scene: LabScene
  item: Item
  onClose: () => void
}) {
  const composition = optionalArray(item.properties.composition)
  const temperatureC = itemTemperatureC(scene, item)
  const solventL = solventVolumeLitres(composition)

  return (
    <aside
      className="lab-beaker-inspect mt-3 rounded-xl border border-[var(--border)] bg-[var(--input-bg)] p-3 text-sm"
      role="dialog"
      aria-label={`Contents of ${item.label}`}
      data-beaker-inspect
    >
      <div className="flex items-start justify-between gap-3">
        <div>
          <p className="font-medium text-[var(--ink)]">{item.label}</p>
          <p className="mt-1 text-[var(--ink-soft)]">Temperature: {formatTemperatureC(temperatureC)}°C</p>
        </div>
        <button
          type="button"
          className="rounded-md border border-[var(--border)] px-2 py-1 text-xs text-[var(--ink-soft)] hover:bg-[var(--surface-hover)]"
          onClick={onClose}
        >
          Close
        </button>
      </div>
      {composition.length > 0 ? (
        <ul className="mt-2 list-disc space-y-1 pl-5 text-[var(--ink)]">
          {composition.map((entry, index) => (
            <li key={`${entry.substance_id}-${entry.phase}-${index}`}>
              <CompositionInspectLine entry={entry} solventVolumeL={solventL} />
            </li>
          ))}
        </ul>
      ) : (
        <p className="mt-2 text-[var(--ink-soft)]">No composition reported by the server.</p>
      )}
    </aside>
  )
}

export function LabBench() {
  const [scene, setScene] = useState<LabScene | null>(null)
  const [selectedToolItemId, setSelectedToolItemId] = useState<string | null>(null)
  const [inspectItemId, setInspectItemId] = useState<string | null>(null)
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
    // Keep inspect open across tool pick-up / put-away; only Close dismisses it.
    setSelectedToolItemId((current) => (current === SPOON_ID ? null : SPOON_ID))
  }

  async function onSolid(targetItemId: string, event: MouseEvent<HTMLButtonElement>) {
    trackPointer(event)
    if (selectedToolItemId === SPOON_ID) {
      if (busy) return
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
      return
    }
    if (selectedToolItemId === null) {
      setInspectItemId(targetItemId)
    }
  }

  async function onWater(event: MouseEvent<HTMLButtonElement>) {
    trackPointer(event)
    if (!scene) return
    if (selectedToolItemId === SPOON_ID) {
      if (busy || !spoonHoldingSubstance(scene)) return
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
      return
    }
    if (selectedToolItemId === null) {
      setInspectItemId(WATER_ID)
    }
  }

  async function onReset() {
    if (busy) return
    setError(null)
    setBusy(true)
    try {
      const response = await postLabAction({ type: 'reset' })
      setScene(response.scene)
      setSelectedToolItemId(null)
      // Inspect stays open and rebinds to the reset scene item by id.
      setPointer(null)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Reset failed')
    } finally {
      setBusy(false)
    }
  }

  const holdingSelected = selectedToolItemId === SPOON_ID
  const spoonFill = scene ? spoonHoldingSubstance(scene) : null
  const leftoverSolid = scene ? undissolvedSolidInWater(scene) : null
  const toolUi: ToolUi = !holdingSelected ? 'none' : spoonFill ?? 'spoon'
  const water = scene ? findItem(scene, WATER_ID) : undefined
  const spoon = scene ? findItem(scene, SPOON_ID) : undefined
  const lastEvents = scene ? optionalArray(scene.last_events) : []
  const inspectItem = scene && inspectItemId ? findItem(scene, inspectItemId) : undefined

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
      <div className="mb-3 flex items-start justify-between gap-3">
        <p className="text-sm text-[var(--ink-soft)]">
          Pick up the spoon, scoop a solid, then click the water. With the spoon put away, click a
          beaker to inspect its contents.
        </p>
        <button
          type="button"
          className="lab-bench-reset shrink-0 rounded-md border border-[var(--border)] px-2.5 py-1 text-xs font-medium text-[var(--ink-soft)] hover:bg-[var(--surface-hover)] disabled:opacity-50"
          aria-label="Reset lab"
          disabled={busy || !scene}
          onClick={onReset}
        >
          Reset
        </button>
      </div>

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
            <WaterBeakerSvg
              leftoverSolid={leftoverSolid}
              busy={busy}
              amountMl={waterAmountMl(scene)}
            />
            <span className="lab-item-label">{water?.label ?? 'Water'}</span>
          </button>

          <button
            type="button"
            className="lab-item"
            aria-label={stockSubstanceAriaLabel('nacl')}
            disabled={busy}
            onClick={(event) => onSolid(NACL_ID, event)}
          >
            <SolidBeakerSvg solid="nacl" amountG={stockAmountG(scene, NACL_ID, 'nacl')} />
            <StockSubstanceLabel substanceId="nacl" />
          </button>

          <button
            type="button"
            className="lab-item"
            aria-label={stockSubstanceAriaLabel('sand')}
            disabled={busy}
            onClick={(event) => onSolid(SAND_ID, event)}
          >
            <SolidBeakerSvg solid="sand" amountG={stockAmountG(scene, SAND_ID, 'sand')} />
            <StockSubstanceLabel substanceId="sand" />
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

      {scene && inspectItem ? (
        <BeakerInspectPanel
          scene={scene}
          item={inspectItem}
          onClose={() => setInspectItemId(null)}
        />
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
