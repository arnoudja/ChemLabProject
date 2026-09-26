/** @vitest-environment jsdom */
import { cleanup, render } from '@testing-library/react'
import { afterEach, describe, expect, it } from 'vitest'
import type { CompositionEntry } from '../generated/contracts'
import {
  CompositionInspectLine,
  formatCompositionAmount,
  formatCompositionLabel,
  formatFormulaNodes,
  formatFormulaPlain,
  formatPh,
  formatTemperatureC,
  phFromComposition,
  solventVolumeLitres,
  StockSubstanceLabel,
  stockSubstanceAriaLabel,
} from './compositionDisplay'

afterEach(() => {
  cleanup()
})

function entry(
  partial: Partial<CompositionEntry> & Pick<CompositionEntry, 'substance_id' | 'phase'>,
): CompositionEntry {
  return {
    amount_ml: null,
    amount_scoop: null,
    amount_g: null,
    amount_mol: null,
    ...partial,
  }
}

describe('formatCompositionLabel', () => {
  it('maps known substances and phases to readable plain-text formulas', () => {
    expect(formatCompositionLabel('water', 'liquid')).toBe('H2O (l)')
    expect(formatCompositionLabel('nacl', 'solid')).toBe('NaCl (s)')
    expect(formatCompositionLabel('cacl2', 'solid')).toBe('CaCl2 (s)')
    expect(formatCompositionLabel('sand', 'solid')).toBe('SiO2 (s)')
    expect(formatCompositionLabel('naoh', 'solid')).toBe('NaOH (s)')
    expect(formatCompositionLabel('oh-', 'aqueous')).toBe('OH- (aq)')
    expect(formatCompositionLabel('na+', 'aqueous')).toBe('Na+ (aq)')
    expect(formatCompositionLabel('ca2+', 'aqueous')).toBe('Ca2+ (aq)')
    expect(formatCompositionLabel('cl-', 'aqueous')).toBe('Cl- (aq)')
    expect(formatCompositionLabel('h+', 'aqueous')).toBe('H+ (aq)')
  })

  it('falls back to the raw server ids when unknown', () => {
    expect(formatCompositionLabel('mystery', 'plasma')).toBe('mystery (plasma)')
  })
})

describe('formatFormulaNodes', () => {
  it('renders numeric subscripts with accessible sub elements', () => {
    const { container } = render(<span>{formatFormulaNodes('water')}</span>)
    expect(container.querySelector('sub')?.textContent).toBe('2')
    expect(container.textContent).toBe('H2O')

    cleanup()
    const sand = render(<span>{formatFormulaNodes('sand')}</span>)
    expect(sand.container.querySelector('sub')?.textContent).toBe('2')
    expect(sand.container.textContent).toBe('SiO2')
  })

  it('renders charge superscripts with accessible sup elements', () => {
    const na = render(<span>{formatFormulaNodes('na+')}</span>)
    expect(na.container.querySelector('sup')?.textContent).toBe('+')
    expect(na.container.textContent).toBe('Na+')

    cleanup()
    const cl = render(<span>{formatFormulaNodes('cl-')}</span>)
    expect(cl.container.querySelector('sup')?.textContent).toBe('−')
    expect(formatFormulaPlain('cl-')).toBe('Cl-')
  })
})

describe('formatCompositionAmount', () => {
  it('formats aqueous molarity with three significant digits', () => {
    const ion = entry({ substance_id: 'na+', phase: 'aqueous', amount_mol: 0.003424 })
    // 0.003424 mol / 0.2 L ≈ 0.01712 M → 0.0171 M at three significant digits
    expect(formatCompositionAmount(ion, 0.2)).toBe('0.0171 M')
  })

  it('keeps three significant digits for dilute and concentrated molarities', () => {
    const dilute = entry({ substance_id: 'ca2+', phase: 'aqueous', amount_mol: 0.0001802 })
    // 0.0001802 / 0.2 = 0.000901 → 0.000901 M
    expect(formatCompositionAmount(dilute, 0.2)).toBe('0.000901 M')
    const concentrated = entry({ substance_id: 'na+', phase: 'aqueous', amount_mol: 0.246 })
    // 0.246 / 0.2 = 1.23 M
    expect(formatCompositionAmount(concentrated, 0.2)).toBe('1.23 M')
  })

  it('formats solid mass with two decimal places including trailing zeros', () => {
    const sand = entry({ substance_id: 'sand', phase: 'solid', amount_g: 0.4 })
    expect(formatCompositionAmount(sand, 0.2)).toBe('0.40 g')
    const scoop = entry({ substance_id: 'sand', phase: 'solid', amount_g: 0.2 })
    expect(formatCompositionAmount(scoop, 0.2)).toBe('0.20 g')
  })

  it('formats liquid volume with two decimal places including trailing zeros', () => {
    const water = entry({ substance_id: 'water', phase: 'liquid', amount_ml: 200 })
    expect(formatCompositionAmount(water, 0.2)).toBe('200.00 ml')
  })

  it('omits amount when data is missing', () => {
    const ion = entry({ substance_id: 'na+', phase: 'aqueous' })
    expect(formatCompositionAmount(ion, 0.2)).toBeNull()
    const solid = entry({ substance_id: 'sand', phase: 'solid' })
    expect(formatCompositionAmount(solid, null)).toBeNull()
    const liquid = entry({ substance_id: 'water', phase: 'liquid' })
    expect(formatCompositionAmount(liquid, null)).toBeNull()
  })
})

describe('solventVolumeLitres', () => {
  it('uses solution volume including salt apparent molar volume', () => {
    // 0.01 mol NaCl · 22 ml/mol → V = 200.22 ml
    expect(
      solventVolumeLitres([
        entry({ substance_id: 'water', phase: 'liquid', amount_ml: 200 }),
        entry({ substance_id: 'na+', phase: 'aqueous', amount_mol: 0.01 }),
        entry({ substance_id: 'cl-', phase: 'aqueous', amount_mol: 0.01 }),
      ]),
    ).toBeCloseTo(0.20022, 5)
  })

  it('uses Φ_V solution volume for stock HCl (10.00 ml)', () => {
    expect(
      solventVolumeLitres([
        entry({ substance_id: 'water', phase: 'liquid', amount_ml: 8.043 }),
        entry({ substance_id: 'h+', phase: 'aqueous', amount_mol: 3.447 / 36.46 }),
        entry({ substance_id: 'cl-', phase: 'aqueous', amount_mol: 3.447 / 36.46 }),
      ]),
    ).toBeCloseTo(0.01, 5)
  })
})

describe('phFromComposition', () => {
  it('returns strongly acidic pH for the stock HCl composition', () => {
    const composition = [
      entry({ substance_id: 'water', phase: 'liquid', amount_ml: 8.043 }),
      entry({ substance_id: 'h+', phase: 'aqueous', amount_mol: 3.447 / 36.46 }),
      entry({ substance_id: 'cl-', phase: 'aqueous', amount_mol: 3.447 / 36.46 }),
    ]
    const ph = phFromComposition(composition)
    expect(ph).not.toBeNull()
    expect(ph!).toBeLessThan(0)
    expect(formatPh(ph!)).toBe('-0.98')
  })

  it('returns null without aqueous H+', () => {
    expect(phFromComposition([entry({ substance_id: 'water', phase: 'liquid', amount_ml: 100 })])).toBeNull()
  })

  it('returns alkaline pH for aqueous OH-', () => {
    const composition = [
      entry({ substance_id: 'water', phase: 'liquid', amount_ml: 100 }),
      entry({ substance_id: 'na+', phase: 'aqueous', amount_mol: 0.005 }),
      entry({ substance_id: 'oh-', phase: 'aqueous', amount_mol: 0.005 }),
    ]
    const ph = phFromComposition(composition)
    expect(ph).not.toBeNull()
    // V = 100 + 0.005·4 ml/mol NaOH → [OH-] slightly below 0.05 M
    expect(ph!).toBeCloseTo(14 + Math.log10(0.005 / 0.10002), 4)
  })
})

describe('formatTemperatureC', () => {
  it('always shows two decimal places for beaker/scene temperatures', () => {
    expect(formatTemperatureC(20)).toBe('20.00')
    expect(formatTemperatureC(19.98413172)).toBe('19.98')
    expect(formatTemperatureC(20.0)).toBe('20.00')
  })
})

describe('CompositionInspectLine', () => {
  it('shows formula typography plus molarity for aqueous ions', () => {
    const { container } = render(
      <CompositionInspectLine
        entry={entry({ substance_id: 'na+', phase: 'aqueous', amount_mol: 0.003424 })}
        solventVolumeL={0.2}
      />,
    )
    expect(container.querySelector('sup')?.textContent).toBe('+')
    expect(container.textContent).toContain('Na+ (aq)')
    expect(container.textContent).toContain('0.0171 M')
  })

  it('shows solid SiO2 mass aggregate with two decimal places', () => {
    const { container } = render(
      <CompositionInspectLine
        entry={entry({ substance_id: 'sand', phase: 'solid', amount_g: 0.4 })}
        solventVolumeL={0.2}
      />,
    )
    expect(container.querySelector('sub')?.textContent).toBe('2')
    expect(container.textContent).toContain('SiO2 (s)')
    expect(container.textContent).toContain('0.40 g')
  })

  it('shows liquid water volume from amount_ml with two decimal places', () => {
    const { container } = render(
      <CompositionInspectLine
        entry={entry({ substance_id: 'water', phase: 'liquid', amount_ml: 200 })}
        solventVolumeL={0.2}
      />,
    )
    expect(container.querySelector('sub')?.textContent).toBe('2')
    expect(container.textContent).toContain('H2O (l)')
    expect(container.textContent).toContain('200.00 ml')
  })
})

describe('StockSubstanceLabel', () => {
  it('shows formula, chemical name, and common name for salts and sand', () => {
    const salt = render(<StockSubstanceLabel substanceId="nacl" />)
    expect(salt.container.textContent).toContain('NaCl')
    expect(salt.container.textContent).toContain('(Sodium chloride)')
    expect(salt.container.textContent).toContain('(Table salt)')
    expect(stockSubstanceAriaLabel('nacl')).toBe('Sodium chloride (NaCl)')

    cleanup()
    const cacl2 = render(<StockSubstanceLabel substanceId="cacl2" />)
    expect(cacl2.container.querySelector('sub')?.textContent).toBe('2')
    expect(cacl2.container.textContent).toContain('CaCl2')
    expect(cacl2.container.textContent).toContain('(Calcium chloride)')
    expect(cacl2.container.textContent).toContain('(De-icing salt)')
    expect(stockSubstanceAriaLabel('cacl2')).toBe('Calcium chloride (CaCl2)')

    cleanup()
    const sand = render(<StockSubstanceLabel substanceId="sand" />)
    expect(sand.container.querySelector('sub')?.textContent).toBe('2')
    expect(sand.container.textContent).toContain('SiO2')
    expect(sand.container.textContent).toContain('(Silicon dioxide)')
    expect(sand.container.textContent).toContain('(Sand)')
    expect(stockSubstanceAriaLabel('sand')).toBe('Sand')
    expect(stockSubstanceAriaLabel('naoh')).toBe('Sodium hydroxide (NaOH)')

    cleanup()
    const water = render(<StockSubstanceLabel substanceId="water" />)
    expect(water.container.querySelector('sub')?.textContent).toBe('2')
    expect(water.container.textContent).toContain('H2O')
    expect(water.container.textContent).toContain('(Water)')
    expect(water.container.textContent).toContain('(distilled water)')
    expect(stockSubstanceAriaLabel('water')).toBe('Distilled water (H2O)')

    cleanup()
    const hcl = render(<StockSubstanceLabel substanceId="hcl" />)
    expect(hcl.container.textContent).toContain('HCl')
    expect(hcl.container.textContent).toContain('(Hydrochloric acid)')
    expect(hcl.container.textContent).toContain('(30% w/w)')
    expect(stockSubstanceAriaLabel('hcl')).toBe('Hydrochloric acid (30%)')
  })
})
