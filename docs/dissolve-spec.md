# First dissolve: NaCl / CaCl₂ / NaOH / sand in water

Short chemistry spec for [issue #6](https://github.com/arnoudja/ChemLabProject/issues/6). This slice is a **small lookup table**, not a general chemistry engine.

Same document in the repo: `docs/dissolve-spec.md`. Follow-on work: [#7](https://github.com/arnoudja/ChemLabProject/issues/7) core API, [#8](https://github.com/arnoudja/ChemLabProject/issues/8) HTTP, [#9](https://github.com/arnoudja/ChemLabProject/issues/9) welcome-page control.

## What the player sees

At the bench, water is the solvent and the temperature is ordinary room/lab conditions. The player tries a named solid:

- **Sodium chloride** mixes into the water and is gone as a solid (endothermic cooling on the water beaker).
- **Calcium chloride** mixes into the water and is gone as a solid (exothermic heating on the water beaker).
- **Sodium hydroxide** mixes into the water and is gone as a solid (strongly exothermic heating).
- **Sand** stays as solid grains.

A short sentence from the **server** explains which of those happened. The browser only shows that sentence; it does not decide.

## Authority

Chemistry rules live only in `chemlab-core`. Axum and the Vite app transport ids and render the result. There is no client-side solubility table and no “did it dissolve?” branch in the browser.

## Inputs (this slice only)

Exact string ids, lowercase ASCII, matched as-is. Do not trim or case-fold (`NaCl` is not `nacl`; `" nacl "` is not `nacl`).

| Input | Allowed values | Meaning |
| --- | --- | --- |
| `substance_id` | `nacl`, `cacl2`, `naoh`, `sand` | Table salt, calcium chloride, sodium hydroxide, silica sand (SiO₂) |
| `solvent_id` | `water` | Liquid water; no other solvents |
| `temperature_c` | any integer °C | Beaker / solvent temperature passed into the lookup |

Amounts, stirring, time, and saturation are not modeled for the qualitative dissolve flag. Scoop mass (0.2 g) drives ion moles and ΔT when a salt dissolves in the scene engine.

**Temperature / solubility simplification:** Known solids (`nacl`, `cacl2`, `naoh`, `sand`) succeed in aqueous water at the beaker’s **current** temperature (exothermic CaCl₂ / NaOH heating or endothermic NaCl cooling must not block further scoops, and pouring sand into warm water must not fail). The dissolve flag still uses the qualitative bench solubility table — there is no T-dependent solubility curve yet. Sand stays undissolved (`dissolved: false`); do not invent sand solubility.

## Outcomes

A successful call returns `dissolved` (boolean) plus a stable English `explanation` for the UI. Use these strings verbatim so tests and copy stay aligned.

### `nacl` + `water` (any `temperature_c`)

- **dissolved:** `true`
- **explanation:** `Sodium chloride (NaCl) dissolves in water at bench temperature.`

### `cacl2` + `water` (any `temperature_c`)

- **dissolved:** `true`
- **explanation:** `Calcium chloride (CaCl2) dissolves in water at bench temperature.`

### `naoh` + `water` (any `temperature_c`)

- **dissolved:** `true`
- **explanation:** `Sodium hydroxide (NaOH) dissolves in water at bench temperature.`

### `sand` + `water` (any `temperature_c`)

- **dissolved:** `false`
- **explanation:** `Sand (silica) does not dissolve in water at bench temperature.`

`dissolved: false` is a successful prediction (sand in water). It is not an error. Explanation copy still says “bench temperature” because solubility is the qualitative 20 °C table, even when the beaker has already heated or cooled from prior dissolves.

## Scene ions and heat (server-authored)

When the scene pours a dissolving salt into water, `chemlab-core` authors aqueous ions and adjusts the water beaker temperature (ambient `LabScene.temperature_c` unchanged):

| Salt | Ions | Stoichiometry | ΔH_sol (J/mol) |
| --- | --- | --- | --- |
| `nacl` | `na+`, `cl-` | 1:1 | `+3880` (endothermic) |
| `cacl2` | `ca2+`, `cl-` | 1:2 | `−81300` (exothermic) |
| `naoh` | `na+`, `oh-` | 1:1 | `−44500` (exothermic) |

Molar masses: NaCl `58.44` g/mol, CaCl₂ `110.98` g/mol, NaOH `40.00` g/mol. Water mass ≈ liquid `amount_ml` (1 g/ml); c_p = `4.184` J/(g·K). Solid NaOH c_p ≈ `1.49` J/(g·K).

**Neutralization:** when aqueous `h+` and `oh-` coexist after mix/dissolve, the scene consumes equal moles (`H+ + OH- → H2O`, ΔH_neut = `−55800` J/mol) **before** `enforce_saturation`. Spectators `na+`/`cl-` remain; excess acid or base stays. Strong-base pH ≈ `14 + log10([OH-])`. When both ions are consumed, inspect shows no pH (`None`, same as pure water). Na⁺ paired with OH⁻ is **not** counted as NaCl inventory (so SI does not invent Cl⁻). Solid NaOH also ionizes whenever it meets liquid water outside the spoon qualitative table (tongs dump into a wet vessel, water onto dry solid) via the scene finalize path; filter wash uses contact-time kinetics with no SI cap.

## Errors (not dissolve results)

Anything outside the table is an error. Unknown materials must **not** be treated as insoluble.

| Case | Suggested code | Meaning |
| --- | --- | --- |
| `substance_id` not `nacl`, `cacl2`, `naoh`, or `sand` | `unknown_substance` | Not in this slice |
| `solvent_id` not `water` | `unsupported_solvent` | Only water |
| empty or whitespace-only ids | `invalid_input` | Reject, do not guess |

`unsupported_temperature` remains a reserved wire code but is not raised for known solids in water in this slice.

Do not invent ethanol or “mystery powder” chemistry. Wrong solvent fails; it does not return a different `dissolved` flag. Pours of known solids (including undissolved sand) must not fail solely because prior dissolve ΔT moved the beaker off 20 °C.

## Display names (UI stock labels)

| id | Label |
| --- | --- |
| `nacl` | Sodium chloride (NaCl) / Table salt |
| `cacl2` | Calcium chloride (CaCl₂) / De-icing salt |
| `naoh` | Sodium hydroxide (NaOH) / Caustic soda |
| `sand` | Sand |
| `water` | Water (implicit; no solvent picker in this slice) |

## Out of scope

- Evaporate, heat sources, concentrations beyond inspect molarity from server moles, 3D glassware
- Other substances or solvents; mixing two solutes beyond aggregating shared ions
- Solubility numbers (g/L), Ksp, a general reaction engine

## Hint for issue #7 (do not implement here)

Pure function in `chemlab-core`, tests first, table-driven:

`dissolve(substance_id, solvent_id, temperature_c) → Result<{ dissolved, explanation }, error>`
