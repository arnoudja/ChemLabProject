/** Display labels for server composition entries — formatting only, no chemistry. */

const FORMULA_BY_SUBSTANCE_ID: Record<string, string> = {
  water: 'H2O',
  nacl: 'NaCl',
  sand: 'SiO2',
  'na+': 'Na+',
  'cl-': 'Cl-',
}

const PHASE_ABBREV: Record<string, string> = {
  solid: 's',
  liquid: 'l',
  aqueous: 'aq',
}

/** Format a server composition row as e.g. `H2O (l)` or `Na+ (aq)`. */
export function formatCompositionLabel(substanceId: string, phase: string): string {
  const formula = FORMULA_BY_SUBSTANCE_ID[substanceId] ?? substanceId
  const phaseLabel = PHASE_ABBREV[phase] ?? phase
  return `${formula} (${phaseLabel})`
}
