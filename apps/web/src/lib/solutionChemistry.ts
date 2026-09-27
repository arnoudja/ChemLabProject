import type { CompositionEntry } from '../generated/contracts'
import {
  HCL_STOCK_CAPACITY_ML,
  HCL_STOCK_HCL_MOLES,
  HCL_STOCK_WATER_ML,
} from './benchAmounts'

/** Apparent molar volumes (ml/mol) — mirrors chemlab-core `hcl::PHI_V_*`. */
export const PHI_V_HCL_ML_PER_MOL =
  (HCL_STOCK_CAPACITY_ML - HCL_STOCK_WATER_ML) / HCL_STOCK_HCL_MOLES
export const PHI_V_H2SO4_ML_PER_MOL = 40.0
export const PHI_V_NAHSO4_ML_PER_MOL = 30.0
export const PHI_V_NA2SO4_ML_PER_MOL = 20.0
export const PHI_V_CASO4_ML_PER_MOL = 15.0
export const PHI_V_NACL_ML_PER_MOL = 22.0
export const PHI_V_CACL2_ML_PER_MOL = 34.0
export const PHI_V_NAOH_ML_PER_MOL = 4.0

function aqueousMol(composition: CompositionEntry[], substanceId: string): number {
  const entry = composition.find((c) => c.substance_id === substanceId && c.phase === 'aqueous')
  return Math.max(0, entry?.amount_mol ?? 0)
}

/** Solution volume (ml) = water ml + liquid H₂SO₄ ml + Σ n · Φ_V. */
export function solutionVolumeMl(composition: CompositionEntry[]): number {
  const waterEntry = composition.find(
    (c) => c.substance_id === 'water' && c.phase === 'liquid',
  )
  const waterMl = Math.max(0, waterEntry?.amount_ml ?? 0)
  const h2so4Liquid = composition.find(
    (c) => c.substance_id === 'h2so4' && c.phase === 'liquid',
  )
  const h2so4Ml = Math.max(0, h2so4Liquid?.amount_ml ?? 0)
  const nH = aqueousMol(composition, 'h+')
  const nOh = aqueousMol(composition, 'oh-')
  const nNa = aqueousMol(composition, 'na+')
  const nCa = aqueousMol(composition, 'ca2+')
  const nCl = aqueousMol(composition, 'cl-')
  const nSo4 = aqueousMol(composition, 'so4^2-')
  const nHso4 = aqueousMol(composition, 'hso4-')
  const nHExcess = Math.max(0, nH - nOh)
  const nHcl = Math.min(nHExcess, nCl)
  const nHAfterHcl = Math.max(0, nHExcess - nHcl)
  const nClAfterHcl = Math.max(0, nCl - nHcl)
  // Excess OH above free H marks strong-base NaOH (Kw leaves both present).
  const nNaoh = nOh > nH + 1e-9 ? Math.max(0, nOh - nH) : 0
  const nNaSalt = Math.max(0, nNa - nNaoh)
  // Reserve Na for chloride salts before NaHSO₄ / Na₂SO₄ pairing.
  const nNaForCl = Math.min(nNaSalt, nClAfterHcl)
  const nNaSulfateBudget = Math.max(0, nNaSalt - nNaForCl)
  const nNahso4 = Math.min(nNaSulfateBudget, nHso4)
  const nHso4Acid = Math.max(0, nHso4 - nNahso4)
  const nNaAfterNahso4 = Math.max(0, nNaSulfateBudget - nNahso4)
  // Salt SO₄ before free-acid SO₄ (Ka2 remnant → ½ Na₂SO₄ + ½ H₂SO₄).
  const nNa2so4 = Math.min(nNaAfterNahso4 * 0.5, nSo4)
  const nSo4AfterNa2so4 = Math.max(0, nSo4 - nNa2so4)
  const nSo4Acid = Math.min(Math.max(0, nHAfterHcl - nHso4Acid), nSo4AfterNa2so4)
  const nH2so4 = nHso4Acid + nSo4Acid
  const nSo4AfterAcid = Math.max(0, nSo4AfterNa2so4 - nSo4Acid)
  const nCaso4 = Math.min(nCa, nSo4AfterAcid)
  const nCaAfterSulfate = Math.max(0, nCa - nCaso4)
  const nNacl = nNaForCl
  const nClAfterNacl = Math.max(0, nClAfterHcl - nNacl)
  const nCacl2 = Math.min(nCaAfterSulfate, nClAfterNacl * 0.5)
  return (
    waterMl +
    h2so4Ml +
    nHcl * PHI_V_HCL_ML_PER_MOL +
    nH2so4 * PHI_V_H2SO4_ML_PER_MOL +
    nNahso4 * PHI_V_NAHSO4_ML_PER_MOL +
    nNaoh * PHI_V_NAOH_ML_PER_MOL +
    nNa2so4 * PHI_V_NA2SO4_ML_PER_MOL +
    nCaso4 * PHI_V_CASO4_ML_PER_MOL +
    nNacl * PHI_V_NACL_ML_PER_MOL +
    nCacl2 * PHI_V_CACL2_ML_PER_MOL
  )
}

/** Strong-acid / strong-base approximate pH display. */
export function phFromComposition(composition: CompositionEntry[]): number | null {
  const hEntry = composition.find((c) => c.substance_id === 'h+' && c.phase === 'aqueous')
  const ohEntry = composition.find((c) => c.substance_id === 'oh-' && c.phase === 'aqueous')
  const nH = hEntry?.amount_mol ?? 0
  const nOh = ohEntry?.amount_mol ?? 0
  const volumeMl = solutionVolumeMl(composition)
  if (volumeMl <= 0) return null
  const volumeL = volumeMl / 1000
  // Prefer solved [H+] (post-speciation both h+ and oh- are present at Kw levels).
  if (nH > 0) {
    const conc = nH / volumeL
    if (conc <= 0) return null
    return -Math.log10(conc)
  }
  if (nOh > 0) {
    const conc = nOh / volumeL
    if (conc <= 0) return null
    return 14 + Math.log10(conc)
  }
  const water = composition.find((c) => c.substance_id === 'water' && c.phase === 'liquid')
  if ((water?.amount_ml ?? 0) > 0) return 7
  return null
}
