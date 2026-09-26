import {
  STOCK_FULL_MASS_G,
  WATER_FULL_ML,
  DISTILLED_WATER_CAPACITY_ML,
  HCL_STOCK_CAPACITY_ML,
  DISH_CAPACITY_ML,
  FILTRATE_CAPACITY_ML,
} from '../lib/benchAmounts'

export type StockSolid = 'nacl' | 'cacl2' | 'sand' | 'naoh'

/** Fill fraction 0..1 from server amount_ml relative to initial water volume. */
export function waterFillRatio(amountMl: number | null | undefined): number {
  if (amountMl == null || amountMl <= 0) return 0
  return Math.min(1, amountMl / WATER_FULL_ML)
}

export function WaterBeakerSvg({
  leftoverSolid,
  busy,
  amountMl,
  hasAqueous,
  dissolveCue,
  floating,
}: {
  leftoverSolid: StockSolid | null
  busy: boolean
  amountMl?: number | null
  /** Server-authored aqueous ions in the water beaker (not a client dissolve decision). */
  hasAqueous?: boolean
  /** Latest dissolve-related kind from scene `last_events`. */
  dissolveCue?: 'dissolved' | 'did_not_dissolve' | null
  floating?: boolean
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
      className={floating ? 'h-24 w-16' : 'h-40 w-28'}
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
            leftoverSolid === 'sand'
              ? '#C9B48A'
              : leftoverSolid === 'cacl2'
                ? '#F2F7FF'
                : leftoverSolid === 'naoh'
                  ? '#E8F5E9'
                  : '#F4FBFF'
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
  // Inspect shows two decimals; leftover float that displays as 0.00 g is empty.
  if (Number(amountG.toFixed(2)) === 0) return 0
  return Math.min(1, amountG / STOCK_FULL_MASS_G)
}

export function SolidBeakerSvg({
  solid,
  amountG,
  floating,
}: {
  solid: StockSolid
  amountG?: number | null
  floating?: boolean
}) {
  const pile =
    solid === 'sand'
      ? '#C9A36A'
      : solid === 'cacl2'
        ? '#EEF4FF'
        : solid === 'naoh'
          ? '#C8E6C9'
          : '#F4FBFF'
  const speck =
    solid === 'sand'
      ? '#8C6A3A'
      : solid === 'cacl2'
        ? '#C4D2ED'
        : solid === 'naoh'
          ? '#81C784'
          : '#DDF7FF'
  const fill = stockFillRatio(amountG)
  // Full pile top ~72; empty sits near the beaker floor (~100).
  const topY = 100 - 28 * fill
  const floorY = 105
  const midY = topY + (floorY - topY) * 0.55
  return (
    <svg
      viewBox="0 0 80 118"
      className={floating ? 'h-16 w-12' : 'h-28 w-20'}
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

/** Fill fraction 0..1 from server amount_ml relative to the distilled-water stock. */
export function distilledWaterFillRatio(amountMl: number | null | undefined): number {
  if (amountMl == null || amountMl <= 0) return 0
  return Math.min(1, amountMl / DISTILLED_WATER_CAPACITY_ML)
}

export function DistilledWaterBeakerSvg({
  amountMl,
  floating,
}: {
  amountMl?: number | null
  floating?: boolean
}) {
  const fill = distilledWaterFillRatio(amountMl)
  const floorY = 105
  const topY = 100 - 28 * fill
  return (
    <svg
      viewBox="0 0 80 118"
      className={floating ? 'h-16 w-12' : 'h-28 w-20'}
      aria-hidden
      data-h2o-fill={fill.toFixed(2)}
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
        <path
          d={`M28 ${topY} H52 L56 ${floorY - 7} C56 ${floorY - 3} 53 ${floorY} 49 ${floorY} H31 C27 ${floorY} 24 ${floorY - 3} 24 ${floorY - 7} Z`}
          fill="#7CF8F7"
          fillOpacity="0.62"
        />
      ) : null}
    </svg>
  )
}

/** Fill fraction 0..1 from solution volume relative to the HCl stock beaker. */
export function hclFillRatio(amountMl: number | null | undefined): number {
  if (amountMl == null || amountMl <= 0) return 0
  return Math.min(1, amountMl / HCL_STOCK_CAPACITY_ML)
}

export function HclBeakerSvg({
  amountMl,
  floating,
}: {
  amountMl?: number | null
  floating?: boolean
}) {
  const fill = hclFillRatio(amountMl)
  const floorY = 105
  const topY = 100 - 28 * fill
  return (
    <svg
      viewBox="0 0 80 118"
      className={floating ? 'h-16 w-12' : 'h-28 w-20'}
      aria-hidden
      data-hcl-fill={fill.toFixed(2)}
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
        <path
          d={`M28 ${topY} H52 L56 ${floorY - 7} C56 ${floorY - 3} 53 ${floorY} 49 ${floorY} H31 C27 ${floorY} 24 ${floorY - 3} 24 ${floorY - 7} Z`}
          fill="#D4E8A8"
          fillOpacity="0.72"
        />
      ) : null}
    </svg>
  )
}

export function SpoonSvg({ fill, floating }: { fill: StockSolid | null; floating?: boolean }) {
  const bowl =
    fill === 'nacl'
      ? '#F4FBFF'
      : fill === 'cacl2'
        ? '#EEF4FF'
        : fill === 'sand'
          ? '#C9A36A'
          : fill === 'naoh'
            ? '#C8E6C9'
            : '#C4D2ED'
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

export function PipetteSvg({ filled, floating }: { filled: boolean; floating?: boolean }) {
  return (
    <svg
      viewBox="0 0 36 120"
      className={floating ? 'h-16 w-5' : 'h-24 w-8'}
      aria-hidden
      data-pipette-filled={filled ? 'true' : 'false'}
    >
      <path
        d="M18 4c6 0 10 5 10 12 0 6-3 10-6 12v5h-8v-5c-3-2-6-6-6-12 0-7 4-12 10-12z"
        fill="#86A7DF"
        stroke="#DDF7FF"
        strokeWidth="1.6"
        strokeLinejoin="round"
      />
      <rect x="12" y="32" width="12" height="6" rx="2" fill="#6A6E95" stroke="#C4D2ED" strokeWidth="1.2" />
      <path
        d="M14 38H22V92L19 114c-.3 1.3-1.7 1.3-2 0L14 92z"
        fill="#3E4058"
        fillOpacity="0.32"
        stroke="#C4D2ED"
        strokeWidth="1.8"
        strokeLinejoin="round"
      />
      {filled ? (
        <path d="M15.6 58H20.4V92l-1.7 13c-.2.9-1.4.9-1.6 0l-1.5-13z" fill="#7CF8F7" fillOpacity="0.7" />
      ) : null}
      <path d="M16.4 42V88" fill="none" stroke="#DDF7FF" strokeWidth="1" strokeOpacity="0.45" strokeLinecap="round" />
    </svg>
  )
}

export function TongsSvg({ floating }: { floating?: boolean }) {
  return (
    <svg
      viewBox="0 0 132 40"
      className={floating ? 'h-8 w-28' : 'h-10 w-32'}
      aria-hidden
      data-tongs
    >
      <rect x="6" y="8" width="16" height="24" rx="4" fill="#6A6E95" stroke="#DDF7FF" strokeWidth="1.6" />
      <path
        d="M22 12c28 4 72 4 98 10"
        fill="none"
        stroke="#C4D2ED"
        strokeWidth="3.2"
        strokeLinecap="round"
      />
      <path
        d="M22 28c28-4 72-4 98-10"
        fill="none"
        stroke="#C4D2ED"
        strokeWidth="3.2"
        strokeLinecap="round"
      />
    </svg>
  )
}

export function EvaporationDishSvg({ amountMl, floating }: { amountMl?: number | null; floating?: boolean }) {
  const fill = dishFillRatio(amountMl)
  const floorY = 42
  const height = 16 * fill
  const topY = floorY - height
  return (
    <svg
      viewBox="0 0 120 56"
      className={floating ? 'h-10 w-20' : 'h-14 w-28'}
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

export function BurnerSvg({ on }: { on: boolean }) {
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

/** Fill fraction 0..1 from server amount_ml relative to the filtrate beaker capacity. */
export function filtrateFillRatio(amountMl: number | null | undefined): number {
  if (amountMl == null || amountMl <= 0) return 0
  return Math.min(1, amountMl / FILTRATE_CAPACITY_ML)
}

export function FiltrateBeakerSvg({ amountMl, floating }: { amountMl?: number | null; floating?: boolean }) {
  const fill = filtrateFillRatio(amountMl)
  const floorY = 148
  const fullHeight = 62
  const topY = floorY - fullHeight * fill
  const leftTop = 30 + 6 * fill
  const rightTop = 90 - 6 * fill
  return (
    <svg
      viewBox="0 0 120 168"
      className={floating ? 'h-24 w-16' : 'h-32 w-20'}
      aria-hidden
      data-filtrate-fill={fill.toFixed(2)}
    >
      <path d="M28 18h64v10H28z" fill="#86A7DF" opacity="0.85" />
      <path
        d="M32 28h56l10 118c1 8-5 14-13 14H35c-8 0-14-6-13-14L32 28z"
        fill="#3E4058"
        fillOpacity="0.35"
        stroke="#C4D2ED"
        strokeWidth="2.4"
      />
      {fill > 0 ? (
        <path
          d={`M${leftTop} ${topY} H${rightTop} L90 ${floorY - 10} C90 ${floorY - 4} 86 ${floorY} 80 ${floorY} H40 C34 ${floorY} 30 ${floorY - 4} 30 ${floorY - 10} Z`}
          fill="#7CF8F7"
          fillOpacity="0.55"
        />
      ) : null}
    </svg>
  )
}

export function FunnelPaperSvg({ residue, floating }: { residue: boolean; floating?: boolean }) {
  // Cone/quarter-fold lining the inner glass wall. Firefox: presentation
  // attributes on paths only (no ellipse disk, clipPath, or CSS `d`).
  const funnelRimY = 6
  const funnelApexY = 52
  const paperRimY = 8
  const paperApexY = 50
  const stemTopY = 52
  return (
    <svg
      viewBox="0 0 120 88"
      className={floating ? 'h-16 w-24' : 'h-20 w-28'}
      aria-hidden
      data-funnel
      data-paper-residue={residue ? 'true' : 'false'}
      data-floating={floating ? 'true' : undefined}
    >
      <path
        data-funnel-cone
        data-rim-y={funnelRimY}
        data-apex-y={funnelApexY}
        d="M20 6h80L68 52H52L20 6z"
        fill="#6A6E95"
        fillOpacity="0.28"
        stroke="#DDF7FF"
        strokeWidth="1.8"
        strokeLinejoin="round"
      />
      <path
        data-paper-cone
        data-rim-y={paperRimY}
        data-apex-y={paperApexY}
        d="M24 8h72L64 50H56L24 8z"
        fill="#E8D9B8"
        stroke="#C4B48A"
        strokeWidth="1.4"
        strokeLinejoin="round"
      />
      <path d="M60 8h36L64 50l-4-2z" fill="#D4C194" fillOpacity="0.92" />
      <path
        d="M60 8L60 50"
        fill="none"
        stroke="#B8A574"
        strokeWidth="1.15"
        strokeLinecap="round"
      />
      <path
        data-funnel-stem
        data-top-y={stemTopY}
        d="M54 52h12v24H54z"
        fill="#6A6E95"
        fillOpacity="0.45"
        stroke="#C4D2ED"
        strokeWidth="1.4"
      />
      {residue ? (
        <g fill="#C9A36A" opacity="0.95">
          <circle cx="56" cy="42" r="2.8" />
          <circle cx="62" cy="44" r="2.2" />
          <circle cx="59" cy="38" r="2.4" />
        </g>
      ) : null}
    </svg>
  )
}
