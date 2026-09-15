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
  DISH_CAPACITY_ML,
  PIPETTE_VOLUME_ML,
} from '../lib/benchAmounts'

export {
  SPOON_SCOOP_MASS_G,
  STOCK_FULL_MASS_G,
  STOCK_FULL_SCOOPS,
  WATER_FULL_ML,
  DISH_CAPACITY_ML,
  PIPETTE_VOLUME_ML,
}

const SPOON_ID = 'spoon-1'
const PIPETTE_ID = 'pipette-1'
const DISH_ID = 'dish-1'
const BURNER_ID = 'burner-1'
const NACL_ID = 'beaker-nacl'
const CACL2_ID = 'beaker-cacl2'
const SAND_ID = 'beaker-sand'
const WATER_ID = 'beaker-water'
const BURNER_POLL_MS = 300

type StockSolid = 'nacl' | 'cacl2' | 'sand'

const STOCK_CAROUSEL: { itemId: string; solid: StockSolid }[] = [
  { itemId: NACL_ID, solid: 'nacl' },
  { itemId: CACL2_ID, solid: 'cacl2' },
  { itemId: SAND_ID, solid: 'sand' },
]

const TOOL_CAROUSEL: { itemId: string; kind: 'pipette' | 'spoon' }[] = [
  { itemId: PIPETTE_ID, kind: 'pipette' },
  { itemId: SPOON_ID, kind: 'spoon' },
]

type ToolUi = 'none' | 'spoon' | 'pipette' | StockSolid

function isStockSolid(id: string): id is StockSolid {
  return id === 'nacl' || id === 'cacl2' || id === 'sand'
}

function findItem(scene: LabScene, id: string): Item | undefined {
  return scene.items.find((item) => item.id === id)
}

function spoonHoldingSubstance(scene: LabScene): StockSolid | null {
  const spoon = findItem(scene, SPOON_ID)
  const held = optionalArray(spoon?.properties.holding)[0]
  if (!held || held.phase !== 'solid') return null
  return isStockSolid(held.substance_id) ? held.substance_id : null
}

/** Undissolved solid grains come only from server composition on the water item. */
function undissolvedSolidInWater(scene: LabScene): StockSolid | null {
  const water = findItem(scene, WATER_ID)
  const solid = optionalArray(water?.properties.composition).find((entry) => entry.phase === 'solid')
  if (!solid) return null
  return isStockSolid(solid.substance_id) ? solid.substance_id : null
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
  hasAqueous,
  dissolveCue,
}: {
  leftoverSolid: StockSolid | null
  busy: boolean
  amountMl?: number | null
  /** Server-authored aqueous ions in the water beaker (not a client dissolve decision). */
  hasAqueous?: boolean
  /** Latest dissolve-related kind from scene `last_events`. */
  dissolveCue?: 'dissolved' | 'did_not_dissolve' | null
}) {
  const fill = waterFillRatio(amountMl)
  // Full liquid top ~86; empty sits at the beaker floor (~148).
  const floorY = 148
  const fullHeight = 62
  const topY = floorY - fullHeight * fill
  const leftTop = 30 + 6 * fill
  const rightTop = 90 - 6 * fill
  const liquidClass = [
    busy ? 'lab-water-busy' : null,
    dissolveCue === 'dissolved' ? 'lab-water-dissolved-cue' : null,
    hasAqueous ? 'lab-water-aqueous' : null,
  ]
    .filter(Boolean)
    .join(' ')
  return (
    <svg
      viewBox="0 0 120 168"
      className="h-40 w-28"
      aria-hidden
      data-water-fill={fill.toFixed(2)}
      data-water-aqueous={hasAqueous ? 'true' : 'false'}
      data-dissolve-cue={dissolveCue ?? 'none'}
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
        <linearGradient id="bench-water-aqueous" x1="40" y1="80" x2="80" y2="150" gradientUnits="userSpaceOnUse">
          <stop stopColor="#A4FFEC" stopOpacity="0.78" />
          <stop offset="1" stopColor="#7CF8F7" stopOpacity="0.48" />
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
          className={liquidClass || undefined}
          d={`M${leftTop} ${topY} H${rightTop} L90 ${floorY - 10} C90 ${floorY - 4} 86 ${floorY} 80 ${floorY} H40 C34 ${floorY} 30 ${floorY - 4} 30 ${floorY - 10} Z`}
          fill={hasAqueous ? 'url(#bench-water-aqueous)' : 'url(#bench-water)'}
        />
      ) : null}
      {leftoverSolid ? (
        <g
          fill={
            leftoverSolid === 'sand' ? '#C9B48A' : leftoverSolid === 'cacl2' ? '#F2F7FF' : '#F4FBFF'
          }
          opacity="0.9"
        >
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
  solid: StockSolid
  amountG?: number | null
}) {
  const pile = solid === 'sand' ? '#C9A36A' : solid === 'cacl2' ? '#EEF4FF' : '#F4FBFF'
  const speck = solid === 'sand' ? '#8C6A3A' : solid === 'cacl2' ? '#C4D2ED' : '#DDF7FF'
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

function SpoonSvg({ fill, floating }: { fill: StockSolid | null; floating?: boolean }) {
  const bowl =
    fill === 'nacl' ? '#F4FBFF' : fill === 'cacl2' ? '#EEF4FF' : fill === 'sand' ? '#C9A36A' : '#C4D2ED'
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

export function dishFillRatio(amountMl: number | null | undefined): number {
  if (amountMl == null || amountMl <= 0) return 0
  return Math.min(1, amountMl / DISH_CAPACITY_ML)
}

function PipetteSvg({ filled, floating }: { filled: boolean; floating?: boolean }) {
  return (
    <svg
      viewBox="0 0 36 120"
      className={floating ? 'h-16 w-5' : 'h-24 w-8'}
      aria-hidden
      data-pipette-filled={filled ? 'true' : 'false'}
    >
      <path d="M14 8h8v14l4 8v70c0 6-3 12-8 12s-8-6-8-12V30l4-8z" fill="#3E4058" fillOpacity="0.35" stroke="#C4D2ED" strokeWidth="2" />
      <rect x="14" y="4" width="8" height="8" rx="2" fill="#86A7DF" />
      {filled ? (
        <path d="M16 48h4v42c0 4-1 8-4 8s-4-4-4-8V48z" fill="#7CF8F7" fillOpacity="0.7" />
      ) : null}
    </svg>
  )
}

function EvaporationDishSvg({ amountMl }: { amountMl?: number | null }) {
  const fill = dishFillRatio(amountMl)
  const floorY = 42
  const height = 16 * fill
  const topY = floorY - height
  return (
    <svg
      viewBox="0 0 120 56"
      className="h-14 w-28"
      aria-hidden
      data-dish-fill={fill.toFixed(2)}
    >
      <path
        d="M18 16h84l8 28c1 6-4 10-10 10H20c-6 0-11-4-10-10l8-28z"
        fill="#3E4058"
        fillOpacity="0.35"
        stroke="#C4D2ED"
        strokeWidth="2.2"
      />
      {fill > 0 ? (
        <path
          d={`M24 ${topY} H96 L100 ${floorY} H20 Z`}
          fill="#7CF8F7"
          fillOpacity="0.55"
        />
      ) : null}
    </svg>
  )
}

function BurnerSvg({ on }: { on: boolean }) {
  return (
    <svg
      viewBox="0 0 80 48"
      className="h-12 w-20"
      aria-hidden
      data-burner-on={on ? 'true' : 'false'}
    >
      <rect x="8" y="28" width="64" height="14" rx="3" fill="#6A6E95" stroke="#DDF7FF" strokeWidth="1.6" />
      <circle cx="40" cy="28" r="10" fill="#3E4058" stroke="#C4D2ED" strokeWidth="1.6" />
      {on ? (
        <path d="M40 6c6 8 10 12 10 18 0 6-4 10-10 10S30 30 30 24c0-6 4-10 10-18z" fill="#FFB14A" />
      ) : null}
    </svg>
  )
}


function stockAmountG(scene: LabScene, itemId: string, substanceId: StockSolid): number | null {
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

/** True when the water beaker composition includes server-authored aqueous ions. */
function waterHasAqueous(scene: LabScene): boolean {
  const item = findItem(scene, WATER_ID)
  return optionalArray(item?.properties.composition).some((c) => c.phase === 'aqueous')
}

/** Latest dissolve-related cue from server `last_events` (no client chemistry). */
function dissolveCueFromEvents(
  events: { kind: string; message: string }[],
): 'dissolved' | 'did_not_dissolve' | null {
  for (let i = events.length - 1; i >= 0; i -= 1) {
    const kind = events[i]?.kind
    if (kind === 'dissolved' || kind === 'did_not_dissolve') return kind
  }
  return null
}

function dishAmountMl(scene: LabScene): number | null {
  const item = findItem(scene, DISH_ID)
  const entry = optionalArray(item?.properties.composition).find(
    (c) => c.substance_id === 'water' && c.phase === 'liquid',
  )
  return entry?.amount_ml ?? null
}

function pipetteIsFilled(scene: LabScene): boolean {
  const pipette = findItem(scene, PIPETTE_ID)
  return optionalArray(pipette?.properties.holding).some(
    (entry) => entry.phase === 'liquid' && (entry.amount_ml ?? 0) > 0,
  )
}

function burnerIsOn(scene: LabScene): boolean {
  return findItem(scene, BURNER_ID)?.properties.on === true
}

function outcomeLabel(kind: string): string | null {
  if (kind === 'dissolved') return 'Dissolved'
  if (kind === 'did_not_dissolve') return 'Did not dissolve'
  if (kind === 'returned') return 'Returned'
  if (kind === 'reset') return 'Reset'
  if (kind === 'scooped') return 'Scooped'
  if (kind === 'poured') return 'Poured'
  if (kind === 'pipetted') return 'Pipetted'
  if (kind === 'toggled') return 'Toggled'
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
  const [stockCarouselIndex, setStockCarouselIndex] = useState(0)
  const [toolCarouselIndex, setToolCarouselIndex] = useState(0)

  const burnerOn = scene ? burnerIsOn(scene) : false

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

  useEffect(() => {
    if (!burnerOn) return
    let cancelled = false
    const id = window.setInterval(() => {
      void fetchLabScene()
        .then((next) => {
          if (!cancelled) setScene(next)
        })
        .catch(() => {
          /* Poll errors stay quiet so a transient GET failure does not clear the bench. */
        })
    }, BURNER_POLL_MS)
    return () => {
      cancelled = true
      window.clearInterval(id)
    }
  }, [burnerOn])

  function trackPointer(event: MouseEvent<HTMLElement>) {
    const root = event.currentTarget.closest('[data-lab-bench]')
    if (!(root instanceof HTMLElement)) return
    const rect = root.getBoundingClientRect()
    setPointer({ x: event.clientX - rect.left, y: event.clientY - rect.top })
  }

  async function onSpoon(event: MouseEvent<HTMLButtonElement>) {
    trackPointer(event)
    // Keep inspect open across tool pick-up / put-away; only Close dismisses it.
    if (selectedToolItemId === SPOON_ID) {
      await putSpoonAway()
      return
    }
    if (selectedToolItemId === PIPETTE_ID) {
      const putAway = await putPipetteAway()
      if (!putAway) return
    }
    setSelectedToolItemId(SPOON_ID)
  }

  async function onPipette(event: MouseEvent<HTMLButtonElement>) {
    trackPointer(event)
    if (selectedToolItemId === PIPETTE_ID) {
      await putPipetteAway()
      return
    }
    if (selectedToolItemId === SPOON_ID) {
      const putAway = await putSpoonAway()
      if (!putAway) return
    }
    setSelectedToolItemId(PIPETTE_ID)
  }

  async function putSpoonAway(): Promise<boolean> {
    if (!scene) {
      setSelectedToolItemId(null)
      return true
    }
    // Empty spoon put-away is a client no-op (no server round-trip).
    if (!spoonHoldingSubstance(scene)) {
      setSelectedToolItemId(null)
      return true
    }
    if (busy) return false
    setError(null)
    setBusy(true)
    try {
      const response = await postLabAction({
        type: 'put_away',
        tool_item_id: SPOON_ID,
      })
      setScene(response.scene)
      setSelectedToolItemId(null)
      return true
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Action failed')
      return false
    } finally {
      setBusy(false)
    }
  }

  async function putPipetteAway(): Promise<boolean> {
    if (!scene) {
      setSelectedToolItemId(null)
      return true
    }
    if (!pipetteIsFilled(scene)) {
      setSelectedToolItemId(null)
      return true
    }
    if (busy) return false
    setError(null)
    setBusy(true)
    try {
      const response = await postLabAction({
        type: 'put_away',
        tool_item_id: PIPETTE_ID,
      })
      setScene(response.scene)
      setSelectedToolItemId(null)
      return true
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Action failed')
      return false
    } finally {
      setBusy(false)
    }
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

  async function applyPipetteTo(targetItemId: string) {
    if (busy) return
    setError(null)
    setBusy(true)
    try {
      const response = await postLabAction({
        type: 'use_tool',
        tool_item_id: PIPETTE_ID,
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
    if (!scene) return
    if (selectedToolItemId === PIPETTE_ID) {
      await applyPipetteTo(WATER_ID)
      return
    }
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

  async function onDish(event: MouseEvent<HTMLButtonElement>) {
    trackPointer(event)
    if (!scene) return
    if (selectedToolItemId === PIPETTE_ID) {
      await applyPipetteTo(DISH_ID)
      return
    }
    if (selectedToolItemId === null) {
      setInspectItemId(DISH_ID)
    }
  }

  async function onBurner(event: MouseEvent<HTMLButtonElement>) {
    trackPointer(event)
    if (selectedToolItemId !== null) return
    if (busy || !scene) return
    setError(null)
    setBusy(true)
    try {
      const response = await postLabAction({
        type: 'toggle_burner',
        burner_item_id: BURNER_ID,
      })
      setScene(response.scene)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Action failed')
    } finally {
      setBusy(false)
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
      setStockCarouselIndex(0)
      setToolCarouselIndex(0)
      // Inspect stays open and rebinds to the reset scene item by id.
      setPointer(null)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Reset failed')
    } finally {
      setBusy(false)
    }
  }

  const spoonSelected = selectedToolItemId === SPOON_ID
  const pipetteSelected = selectedToolItemId === PIPETTE_ID
  const holdingSelected = spoonSelected || pipetteSelected
  const spoonFill = scene ? spoonHoldingSubstance(scene) : null
  const leftoverSolid = scene ? undissolvedSolidInWater(scene) : null
  const toolUi: ToolUi = pipetteSelected
    ? 'pipette'
    : spoonSelected
      ? (spoonFill ?? 'spoon')
      : 'none'
  const water = scene ? findItem(scene, WATER_ID) : undefined
  const spoon = scene ? findItem(scene, SPOON_ID) : undefined
  const pipette = scene ? findItem(scene, PIPETTE_ID) : undefined
  const dish = scene ? findItem(scene, DISH_ID) : undefined
  const burner = scene ? findItem(scene, BURNER_ID) : undefined
  const lastEvents = scene ? optionalArray(scene.last_events) : []
  const dissolveCue = dissolveCueFromEvents(lastEvents)
  const hasAqueous = scene ? waterHasAqueous(scene) : false
  const inspectItem = scene && inspectItemId ? findItem(scene, inspectItemId) : undefined
  const pipetteFilled = scene ? pipetteIsFilled(scene) : false
  const visibleStock = STOCK_CAROUSEL[stockCarouselIndex] ?? STOCK_CAROUSEL[0]
  const visibleTool = TOOL_CAROUSEL[toolCarouselIndex] ?? TOOL_CAROUSEL[0]

  function stepStockCarousel(delta: number, event: MouseEvent<HTMLButtonElement>) {
    event.stopPropagation()
    setStockCarouselIndex(
      (index) => (index + delta + STOCK_CAROUSEL.length) % STOCK_CAROUSEL.length,
    )
  }

  function stepToolCarousel(delta: number, event: MouseEvent<HTMLButtonElement>) {
    event.stopPropagation()
    setToolCarouselIndex(
      (index) => (index + delta + TOOL_CAROUSEL.length) % TOOL_CAROUSEL.length,
    )
  }

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
          Pick up the spoon to scoop solids, or the pipette to move {PIPETTE_VOLUME_ML.toFixed(2)} ml
          of solution between the water beaker and the dish. With no tool selected, click a vessel to
          inspect it, or the burner to heat the dish.
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
          {dish || burner ? (
            <div className="lab-evap-stack">
              {dish ? (
                <button
                  type="button"
                  className="lab-item"
                  aria-label="Evaporation dish"
                  disabled={busy}
                  onClick={onDish}
                >
                  <EvaporationDishSvg amountMl={dishAmountMl(scene)} />
                  <span className="lab-item-label">{dish.label}</span>
                </button>
              ) : null}
              {burner ? (
                <button
                  type="button"
                  className="lab-item"
                  aria-label="Burner"
                  aria-pressed={burnerOn}
                  disabled={busy}
                  onClick={onBurner}
                >
                  <BurnerSvg on={burnerOn} />
                  <span className="lab-item-label">{burner.label}</span>
                </button>
              ) : null}
            </div>
          ) : null}

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
              hasAqueous={hasAqueous}
              dissolveCue={dissolveCue}
            />
            <span className="lab-item-label">{water?.label ?? 'Water'}</span>
          </button>

          <div className="lab-stock-carousel">
            <button
              type="button"
              className="lab-stock-carousel-arrow"
              aria-label="Previous ingredient"
              onClick={(event) => stepStockCarousel(-1, event)}
            >
              ‹
            </button>
            <button
              type="button"
              className="lab-item"
              aria-label={stockSubstanceAriaLabel(visibleStock.solid)}
              disabled={busy}
              onClick={(event) => onSolid(visibleStock.itemId, event)}
            >
              <SolidBeakerSvg
                solid={visibleStock.solid}
                amountG={stockAmountG(scene, visibleStock.itemId, visibleStock.solid)}
              />
              <StockSubstanceLabel substanceId={visibleStock.solid} />
            </button>
            <button
              type="button"
              className="lab-stock-carousel-arrow"
              aria-label="Next ingredient"
              onClick={(event) => stepStockCarousel(1, event)}
            >
              ›
            </button>
          </div>

          <div className="lab-tool-carousel">
            <button
              type="button"
              className="lab-tool-carousel-arrow"
              aria-label="Previous tool"
              onClick={(event) => stepToolCarousel(-1, event)}
            >
              ‹
            </button>
            <div className="lab-tool-carousel-slot">
              {visibleTool.kind === 'pipette' ? (
                <button
                  type="button"
                  className={`lab-item ${pipetteSelected ? 'opacity-40' : ''}`}
                  aria-label="Pipette"
                  aria-pressed={pipetteSelected}
                  disabled={busy}
                  onClick={onPipette}
                >
                  <PipetteSvg filled={pipetteFilled} />
                  <span className="lab-item-label">{pipette?.label ?? 'Pipette'}</span>
                </button>
              ) : (
                <button
                  type="button"
                  className={`lab-item ${spoonSelected ? 'opacity-40' : ''}`}
                  aria-label="Spoon"
                  aria-pressed={spoonSelected}
                  disabled={busy}
                  onClick={onSpoon}
                >
                  <SpoonSvg fill={null} />
                  <span className="lab-item-label">{spoon?.label ?? 'Spoon'}</span>
                </button>
              )}
            </div>
            <button
              type="button"
              className="lab-tool-carousel-arrow"
              aria-label="Next tool"
              onClick={(event) => stepToolCarousel(1, event)}
            >
              ›
            </button>
          </div>
        </div>
      ) : null}

      {holdingSelected && pointer ? (
        <div
          className={`pointer-events-none absolute z-20 ${
            pipetteSelected ? 'lab-cursor-pipette' : 'lab-cursor-spoon'
          }`}
          style={{ left: pointer.x, top: pointer.y }}
          aria-hidden
        >
          {pipetteSelected ? (
            <PipetteSvg filled={pipetteFilled} floating />
          ) : (
            <SpoonSvg fill={spoonFill} floating />
          )}
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
        <div className="lab-bench-events mt-3 space-y-2 text-sm" role="status" aria-live="polite">
          {dissolveCue ? (
            <p
              className="lab-bench-status text-[var(--ink)]"
              data-bench-status={dissolveCue}
            >
              <span className="font-medium text-[var(--hackerman-bright-aqua)]">
                {outcomeLabel(dissolveCue)}
              </span>
              {': '}
              <span className="text-[var(--ink-soft)]">
                {dissolveCue === 'dissolved'
                  ? lastEvents.find((event) => event.kind === 'dissolved')?.message
                  : lastEvents.find((event) => event.kind === 'did_not_dissolve')?.message}
              </span>
            </p>
          ) : (
            lastEvents.map((event, index) => {
              const outcome = outcomeLabel(event.kind)
              return (
                <div key={`${event.kind}-${index}`} className="space-y-1" data-event-kind={event.kind}>
                  {outcome ? (
                    <p className="font-medium text-[var(--ink)]">{outcome}</p>
                  ) : null}
                  <p className="text-[var(--ink-soft)]">{event.message}</p>
                </div>
              )
            })
          )}
        </div>
      ) : null}
    </section>
  )
}
