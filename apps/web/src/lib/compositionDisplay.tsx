/** Display labels for server composition entries — formatting only, no chemistry. */

import type { ReactNode } from 'react'
import type { CompositionEntry } from '../generated/contracts'

const PHASE_ABBREV: Record<string, string> = {
  solid: 's',
  liquid: 'l',
  aqueous: 'aq',
}

type FormulaPart =
  | { kind: 'text'; value: string }
  | { kind: 'sub'; value: string }
  | { kind: 'sup'; value: string }

/** Known substance_id → formula parts with numeric subscripts / charge superscripts. */
const FORMULA_PARTS_BY_SUBSTANCE_ID: Record<string, FormulaPart[]> = {
  water: [
    { kind: 'text', value: 'H' },
    { kind: 'sub', value: '2' },
    { kind: 'text', value: 'O' },
  ],
  nacl: [{ kind: 'text', value: 'NaCl' }],
  cacl2: [
    { kind: 'text', value: 'CaCl' },
    { kind: 'sub', value: '2' },
  ],
  sand: [
    { kind: 'text', value: 'SiO' },
    { kind: 'sub', value: '2' },
  ],
  'na+': [
    { kind: 'text', value: 'Na' },
    { kind: 'sup', value: '+' },
  ],
  'ca2+': [
    { kind: 'text', value: 'Ca' },
    { kind: 'sup', value: '2+' },
  ],
  'cl-': [
    { kind: 'text', value: 'Cl' },
    { kind: 'sup', value: '−' },
  ],
}

function formulaParts(substanceId: string): FormulaPart[] {
  return FORMULA_PARTS_BY_SUBSTANCE_ID[substanceId] ?? [{ kind: 'text', value: substanceId }]
}

/** Plain-text formula for tests / aria (ASCII digits and +/-). */
export function formatFormulaPlain(substanceId: string): string {
  return formulaParts(substanceId)
    .map((part) => {
      if (part.kind === 'sup' && part.value === '−') return '-'
      return part.value
    })
    .join('')
}

/** Accessible HTML formula via `<sub>` / `<sup>` (Firefox-friendly). */
export function formatFormulaNodes(substanceId: string): ReactNode {
  return formulaParts(substanceId).map((part, index) => {
    if (part.kind === 'sub') {
      return <sub key={index}>{part.value}</sub>
    }
    if (part.kind === 'sup') {
      return <sup key={index}>{part.value}</sup>
    }
    return <span key={index}>{part.value}</span>
  })
}

/** Format a server composition row as e.g. `H2O (l)` or `Na+ (aq)` (plain text). */
export function formatCompositionLabel(substanceId: string, phase: string): string {
  const formula = formatFormulaPlain(substanceId)
  const phaseLabel = PHASE_ABBREV[phase] ?? phase
  return `${formula} (${phaseLabel})`
}

/** Solvent volume in litres from a composition list (water liquid amount_ml). */
export function solventVolumeLitres(composition: CompositionEntry[]): number | null {
  const water = composition.find(
    (entry) => entry.substance_id === 'water' && entry.phase === 'liquid' && entry.amount_ml != null,
  )
  if (!water || water.amount_ml == null || water.amount_ml <= 0) return null
  return water.amount_ml / 1000
}

/** Format molarity, mass, or volume suffix from server amounts (display only). */
export function formatCompositionAmount(
  entry: CompositionEntry,
  solventVolumeL: number | null,
): string | null {
  if (entry.phase === 'aqueous' && entry.amount_mol != null && solventVolumeL != null) {
    const molarity = entry.amount_mol / solventVolumeL
    return `${formatMolarity(molarity)} M`
  }
  if (entry.phase === 'solid' && entry.amount_g != null) {
    return `${formatFixedAmount(entry.amount_g)} g`
  }
  if (entry.phase === 'liquid' && entry.amount_ml != null) {
    return `${formatFixedAmount(entry.amount_ml)} ml`
  }
  return null
}

function formatMolarity(value: number): string {
  if (value === 0) return '0'
  if (value >= 1) return value.toFixed(2).replace(/\.?0+$/, '')
  if (value >= 0.01) return value.toFixed(3).replace(/0+$/, '').replace(/\.$/, '')
  return value.toExponential(2)
}

/** Shared number formatting for mass (g) and volume (ml) suffixes. */
function formatFixedAmount(value: number): string {
  if (Number.isInteger(value)) return String(value)
  return value.toFixed(2).replace(/0+$/, '').replace(/\.$/, '')
}

/** Format beaker/scene temperature for inspect labels (always two decimals). */
export function formatTemperatureC(temperatureC: number): string {
  return temperatureC.toFixed(2)
}

/** Label + optional amount for the beaker inspect list. */
export function CompositionInspectLine({
  entry,
  solventVolumeL,
}: {
  entry: CompositionEntry
  solventVolumeL: number | null
}): ReactNode {
  const phaseLabel = PHASE_ABBREV[entry.phase] ?? entry.phase
  const amount = formatCompositionAmount(entry, solventVolumeL)
  return (
    <>
      {formatFormulaNodes(entry.substance_id)} ({phaseLabel})
      {amount ? <> — {amount}</> : null}
    </>
  )
}

/** Stock jar captions under salt / sand beakers (display only). */
export type StockSubstanceId = 'nacl' | 'cacl2' | 'sand'

const STOCK_SUBSTANCE_LABELS: Record<
  StockSubstanceId,
  { chemicalName: string; commonName: string; ariaName: string }
> = {
  nacl: {
    chemicalName: 'Sodium chloride',
    commonName: 'Table salt',
    ariaName: 'Sodium chloride (NaCl)',
  },
  cacl2: {
    chemicalName: 'Calcium chloride',
    commonName: 'De-icing salt',
    ariaName: 'Calcium chloride (CaCl2)',
  },
  sand: {
    chemicalName: 'Silicon dioxide',
    commonName: 'Sand',
    ariaName: 'Sand',
  },
}

export function stockSubstanceAriaLabel(substanceId: StockSubstanceId): string {
  return STOCK_SUBSTANCE_LABELS[substanceId].ariaName
}

/** Three-line stock beaker label: formula, (chemical name), (common name). */
export function StockSubstanceLabel({ substanceId }: { substanceId: StockSubstanceId }) {
  const meta = STOCK_SUBSTANCE_LABELS[substanceId]
  return (
    <span className="lab-item-label lab-item-label-stack" data-stock-label={substanceId}>
      <span className="lab-item-label-formula">{formatFormulaNodes(substanceId)}</span>
      <span className="lab-item-label-name">({meta.chemicalName})</span>
      <span className="lab-item-label-common">({meta.commonName})</span>
    </span>
  )
}
