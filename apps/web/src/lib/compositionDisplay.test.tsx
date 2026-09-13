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
  formatTemperatureC,
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
    expect(formatCompositionLabel('sand', 'solid')).toBe('SiO2 (s)')
    expect(formatCompositionLabel('na+', 'aqueous')).toBe('Na+ (aq)')
    expect(formatCompositionLabel('cl-', 'aqueous')).toBe('Cl- (aq)')
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
  it('formats aqueous molarity from server moles and solvent volume', () => {
    const ion = entry({ substance_id: 'na+', phase: 'aqueous', amount_mol: 0.003424 })
    // 0.003424 mol / 0.2 L ≈ 0.0171 M
    expect(formatCompositionAmount(ion, 0.2)).toBe('0.017 M')
  })

  it('formats solid mass from server grams', () => {
    const sand = entry({ substance_id: 'sand', phase: 'solid', amount_g: 0.4 })
    expect(formatCompositionAmount(sand, 0.2)).toBe('0.4 g')
  })

  it('formats liquid volume from server millilitres', () => {
    const water = entry({ substance_id: 'water', phase: 'liquid', amount_ml: 200 })
    expect(formatCompositionAmount(water, 0.2)).toBe('200 ml')
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
  it('derives litres from the water liquid amount_ml', () => {
    expect(
      solventVolumeLitres([
        entry({ substance_id: 'water', phase: 'liquid', amount_ml: 200 }),
        entry({ substance_id: 'na+', phase: 'aqueous', amount_mol: 0.01 }),
      ]),
    ).toBe(0.2)
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
    expect(container.textContent).toContain('0.017 M')
  })

  it('shows solid SiO2 mass aggregate', () => {
    const { container } = render(
      <CompositionInspectLine
        entry={entry({ substance_id: 'sand', phase: 'solid', amount_g: 0.4 })}
        solventVolumeL={0.2}
      />,
    )
    expect(container.querySelector('sub')?.textContent).toBe('2')
    expect(container.textContent).toContain('SiO2 (s)')
    expect(container.textContent).toContain('0.4 g')
  })

  it('shows liquid water volume from amount_ml', () => {
    const { container } = render(
      <CompositionInspectLine
        entry={entry({ substance_id: 'water', phase: 'liquid', amount_ml: 200 })}
        solventVolumeL={0.2}
      />,
    )
    expect(container.querySelector('sub')?.textContent).toBe('2')
    expect(container.textContent).toContain('H2O (l)')
    expect(container.textContent).toContain('200 ml')
  })
})

describe('StockSubstanceLabel', () => {
  it('shows formula, chemical name, and common name for salt and sand', () => {
    const salt = render(<StockSubstanceLabel substanceId="nacl" />)
    expect(salt.container.textContent).toContain('NaCl')
    expect(salt.container.textContent).toContain('(Sodium chloride)')
    expect(salt.container.textContent).toContain('(Table salt)')
    expect(stockSubstanceAriaLabel('nacl')).toBe('Sodium chloride (NaCl)')

    cleanup()
    const sand = render(<StockSubstanceLabel substanceId="sand" />)
    expect(sand.container.querySelector('sub')?.textContent).toBe('2')
    expect(sand.container.textContent).toContain('SiO2')
    expect(sand.container.textContent).toContain('(Silicon dioxide)')
    expect(sand.container.textContent).toContain('(Sand)')
    expect(stockSubstanceAriaLabel('sand')).toBe('Sand')
  })
})
