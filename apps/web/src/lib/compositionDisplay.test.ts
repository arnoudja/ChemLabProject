import { describe, expect, it } from 'vitest'
import { formatCompositionLabel } from './compositionDisplay'

describe('formatCompositionLabel', () => {
  it('maps known substances and phases to readable formulas', () => {
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
