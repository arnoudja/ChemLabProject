# First dissolve: NaCl / CaCl₂ / sand in water

Short chemistry spec for [issue #6](https://github.com/arnoudja/ChemLabProject/issues/6). This slice is a **small lookup table**, not a general chemistry engine.

Same document in the repo: `docs/dissolve-spec.md`. Follow-on work: [#7](https://github.com/arnoudja/ChemLabProject/issues/7) core API, [#8](https://github.com/arnoudja/ChemLabProject/issues/8) HTTP, [#9](https://github.com/arnoudja/ChemLabProject/issues/9) welcome-page control.

## What the player sees

At the bench, water is the solvent and the temperature is ordinary room/lab conditions. The player tries a named solid:

- **Sodium chloride** mixes into the water and is gone as a solid (endothermic cooling on the water beaker).
- **Calcium chloride** mixes into the water and is gone as a solid (exothermic heating on the water beaker).
- **Sand** stays as solid grains.

A short sentence from the **server** explains which of those happened. The browser only shows that sentence; it does not decide.

## Authority

Chemistry rules live only in `chemlab-core`. Axum and the Vite app transport ids and render the result. There is no client-side solubility table and no “did it dissolve?” branch in the browser.

## Inputs (this slice only)

Exact string ids, lowercase ASCII, matched as-is. Do not trim or case-fold (`NaCl` is not `nacl`; `" nacl "` is not `nacl`).

| Input | Allowed values | Meaning |
| --- | --- | --- |
| `substance_id` | `nacl`, `cacl2`, `sand` | Table salt, calcium chloride, silica sand (SiO₂) |
| `solvent_id` | `water` | Liquid water; no other solvents |
| `temperature_c` | any integer °C for soluble salts; `20` for sand | Beaker / solvent temperature passed into the lookup |

Amounts, stirring, time, and saturation are not modeled for the qualitative dissolve flag. Scoop mass (0.2 g) drives ion moles and ΔT when a salt dissolves in the scene engine.

**Temperature / solubility simplification:** Soluble salts (`nacl`, `cacl2`) keep dissolving into aqueous water at the beaker’s **current** temperature (exothermic CaCl₂ heating or endothermic NaCl cooling must not block further scoops). The dissolve flag still uses the qualitative bench solubility table — there is no T-dependent solubility curve yet. Sand (insoluble) still requires exact `20` °C for the lookup.

## Outcomes

A successful call returns `dissolved` (boolean) plus a stable English `explanation` for the UI. Use these strings verbatim so tests and copy stay aligned.

### `nacl` + `water` (any `temperature_c`)

- **dissolved:** `true`
- **explanation:** `Sodium chloride (NaCl) dissolves in water at bench temperature.`

### `cacl2` + `water` (any `temperature_c`)

- **dissolved:** `true`
- **explanation:** `Calcium chloride (CaCl2) dissolves in water at bench temperature.`

### `sand` + `water` + `20`

- **dissolved:** `false`
- **explanation:** `Sand (silica) does not dissolve in water at bench temperature.`

`dissolved: false` is a successful prediction (sand in water). It is not an error. Explanation copy still says “bench temperature” because solubility is the qualitative 20 °C table, even when the beaker has already heated or cooled from prior dissolves.

## Scene ions and heat (server-authored)

When the scene pours a dissolving salt into water, `chemlab-core` authors aqueous ions and adjusts the water beaker temperature (ambient `LabScene.temperature_c` unchanged):

| Salt | Ions | Stoichiometry | ΔH_sol (J/mol) |
| --- | --- | --- | --- |
| `nacl` | `na+`, `cl-` | 1:1 | `+3880` (endothermic) |
| `cacl2` | `ca2+`, `cl-` | 1:2 | `−81300` (exothermic) |

Molar masses: NaCl `58.44` g/mol, CaCl₂ `110.98` g/mol. Water mass ≈ liquid `amount_ml` (1 g/ml); c_p = `4.184` J/(g·K).

## Errors (not dissolve results)

Anything outside the table is an error. Unknown materials must **not** be treated as insoluble.

| Case | Suggested code | Meaning |
| --- | --- | --- |
| `substance_id` not `nacl`, `cacl2`, or `sand` | `unknown_substance` | Not in this slice |
| `solvent_id` not `water` | `unsupported_solvent` | Only water |
| `sand` + `water` with `temperature_c` ≠ `20` | `unsupported_temperature` | Insoluble sand lookup still gated to bench °C |
| empty or whitespace-only ids | `invalid_input` | Reject, do not guess |

Do not invent ethanol or “mystery powder” chemistry. Wrong solvent fails; it does not return a different `dissolved` flag. Soluble-salt pours must not fail solely because prior dissolve ΔT moved the beaker off 20 °C.

## Display names (UI stock labels)

| id | Label |
| --- | --- |
| `nacl` | Sodium chloride (NaCl) / Table salt |
| `cacl2` | Calcium chloride (CaCl₂) / De-icing salt |
| `sand` | Sand |
| `water` | Water (implicit; no solvent picker in this slice) |

## Out of scope

- Evaporate, heat sources, concentrations beyond inspect molarity from server moles, 3D glassware
- Other substances or solvents; mixing two solutes beyond aggregating shared ions
- Solubility numbers (g/L), Ksp, a general reaction engine

## Hint for issue #7 (do not implement here)

Pure function in `chemlab-core`, tests first, table-driven:

`dissolve(substance_id, solvent_id, temperature_c) → Result<{ dissolved, explanation }, error>`
