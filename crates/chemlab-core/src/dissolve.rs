//! Qualitative dissolve lookup for this slice: NaCl / CaCl₂ / sand in water at 20 °C.

use thiserror::Error;

/// Successful dissolve prediction for a named solid in a named solvent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DissolveOutcome {
    /// Whether the solid mixes into the solvent (gone as a solid).
    pub dissolved: bool,
    /// Stable English sentence for the UI. Do not paraphrase in callers.
    pub explanation: &'static str,
}

/// Errors for inputs outside this slice's lookup table.
///
/// These are not dissolve results: unknown materials must not be treated as insoluble.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DissolveError {
    /// `substance_id` is not `nacl`, `cacl2`, or `sand`.
    #[error("unknown substance")]
    UnknownSubstance,
    /// `solvent_id` is not `water`.
    #[error("unsupported solvent")]
    UnsupportedSolvent,
    /// `temperature_c` is not bench temperature (`20`).
    #[error("unsupported temperature")]
    UnsupportedTemperature,
    /// An id is empty or whitespace-only.
    #[error("invalid input")]
    InvalidInput,
}

impl DissolveError {
    /// Stable machine code from the dissolve spec.
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnknownSubstance => "unknown_substance",
            Self::UnsupportedSolvent => "unsupported_solvent",
            Self::UnsupportedTemperature => "unsupported_temperature",
            Self::InvalidInput => "invalid_input",
        }
    }
}

fn is_known_substance(substance_id: &str) -> bool {
    matches!(substance_id, "nacl" | "cacl2" | "sand")
}

/// Predict whether a named solid dissolves under this slice's bench conditions.
///
/// Ids are matched as-is: no trim, no case-fold. `dissolved: false` is a successful
/// prediction (sand in water), not an error.
pub fn dissolve(
    substance_id: &str,
    solvent_id: &str,
    temperature_c: i32,
) -> Result<DissolveOutcome, DissolveError> {
    if is_blank(substance_id) || is_blank(solvent_id) {
        return Err(DissolveError::InvalidInput);
    }

    match (substance_id, solvent_id, temperature_c) {
        ("nacl", "water", 20) => Ok(DissolveOutcome {
            dissolved: true,
            explanation: "Sodium chloride (NaCl) dissolves in water at bench temperature.",
        }),
        ("cacl2", "water", 20) => Ok(DissolveOutcome {
            dissolved: true,
            explanation: "Calcium chloride (CaCl2) dissolves in water at bench temperature.",
        }),
        ("sand", "water", 20) => Ok(DissolveOutcome {
            dissolved: false,
            explanation: "Sand (silica) does not dissolve in water at bench temperature.",
        }),
        (substance, solvent, _) => {
            if !is_known_substance(substance) {
                Err(DissolveError::UnknownSubstance)
            } else if solvent != "water" {
                Err(DissolveError::UnsupportedSolvent)
            } else {
                Err(DissolveError::UnsupportedTemperature)
            }
        }
    }
}

fn is_blank(id: &str) -> bool {
    id.trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dissolve_outcomes_match_spec_table() {
        let cases = [
            (
                "nacl",
                "water",
                20,
                true,
                "Sodium chloride (NaCl) dissolves in water at bench temperature.",
            ),
            (
                "cacl2",
                "water",
                20,
                true,
                "Calcium chloride (CaCl2) dissolves in water at bench temperature.",
            ),
            (
                "sand",
                "water",
                20,
                false,
                "Sand (silica) does not dissolve in water at bench temperature.",
            ),
        ];

        for (substance_id, solvent_id, temperature_c, dissolved, explanation) in cases {
            let outcome = dissolve(substance_id, solvent_id, temperature_c).unwrap_or_else(|err| {
                panic!(
                    "expected success for {substance_id}/{solvent_id}/{temperature_c}, got {err:?}"
                )
            });
            assert_eq!(
                (outcome.dissolved, outcome.explanation),
                (dissolved, explanation),
                "{substance_id} in {solvent_id} at {temperature_c}°C"
            );
        }
    }

    #[test]
    fn dissolve_errors_match_spec_table() {
        let cases = [
            (
                "sugar",
                "water",
                20,
                DissolveError::UnknownSubstance,
                "unknown_substance",
            ),
            (
                "NaCl",
                "water",
                20,
                DissolveError::UnknownSubstance,
                "unknown_substance",
            ),
            (
                "CaCl2",
                "water",
                20,
                DissolveError::UnknownSubstance,
                "unknown_substance",
            ),
            (
                " nacl ",
                "water",
                20,
                DissolveError::UnknownSubstance,
                "unknown_substance",
            ),
            (
                "nacl",
                "ethanol",
                20,
                DissolveError::UnsupportedSolvent,
                "unsupported_solvent",
            ),
            (
                "cacl2",
                "ethanol",
                20,
                DissolveError::UnsupportedSolvent,
                "unsupported_solvent",
            ),
            (
                "sand",
                "Water",
                20,
                DissolveError::UnsupportedSolvent,
                "unsupported_solvent",
            ),
            (
                "nacl",
                "water",
                21,
                DissolveError::UnsupportedTemperature,
                "unsupported_temperature",
            ),
            (
                "cacl2",
                "water",
                21,
                DissolveError::UnsupportedTemperature,
                "unsupported_temperature",
            ),
            (
                "sand",
                "water",
                100,
                DissolveError::UnsupportedTemperature,
                "unsupported_temperature",
            ),
            (
                "nacl",
                "water",
                0,
                DissolveError::UnsupportedTemperature,
                "unsupported_temperature",
            ),
            (
                "",
                "water",
                20,
                DissolveError::InvalidInput,
                "invalid_input",
            ),
            ("nacl", "", 20, DissolveError::InvalidInput, "invalid_input"),
            (
                " ",
                "water",
                20,
                DissolveError::InvalidInput,
                "invalid_input",
            ),
            (
                "nacl",
                "\t",
                20,
                DissolveError::InvalidInput,
                "invalid_input",
            ),
            (
                "\n",
                "water",
                20,
                DissolveError::InvalidInput,
                "invalid_input",
            ),
            ("", "", 20, DissolveError::InvalidInput, "invalid_input"),
            (
                "",
                "ethanol",
                20,
                DissolveError::InvalidInput,
                "invalid_input",
            ),
            (
                "sugar",
                "ethanol",
                20,
                DissolveError::UnknownSubstance,
                "unknown_substance",
            ),
            (
                "nacl",
                "ethanol",
                100,
                DissolveError::UnsupportedSolvent,
                "unsupported_solvent",
            ),
        ];

        for (substance_id, solvent_id, temperature_c, expected, code) in cases {
            let err = dissolve(substance_id, solvent_id, temperature_c)
                .expect_err("spec table row must be an error, not a dissolve outcome");
            assert_eq!(
                err, expected,
                "{substance_id:?}/{solvent_id:?}/{temperature_c}"
            );
            assert_eq!(
                err.code(),
                code,
                "{substance_id:?}/{solvent_id:?}/{temperature_c} code"
            );
        }
    }
}
