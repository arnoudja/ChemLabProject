# First dissolve: NaCl vs sand in water

Short chemistry spec for [issue #6](https://github.com/arnoudja/ChemLabProject/issues/6). This slice is **two lookup cases**, not a general chemistry engine.

Same document in the repo: `docs/dissolve-spec.md`. Follow-on work: [#7](https://github.com/arnoudja/ChemLabProject/issues/7) core API, [#8](https://github.com/arnoudja/ChemLabProject/issues/8) HTTP, [#9](https://github.com/arnoudja/ChemLabProject/issues/9) welcome-page control.

## What the player sees

At the bench, water is the solvent and the temperature is ordinary room/lab conditions. The player tries **one** named solid:

- **Sodium chloride** mixes into the water and is gone as a solid.
- **Sand** stays as solid grains.

A short sentence from the **server** explains which of those happened. The browser only shows that sentence; it does not decide.

## Authority

Chemistry rules live only in `chemlab-core`. Axum and the Vite app transport ids and render the result. There is no client-side solubility table and no “did it dissolve?” branch in the browser.

## Inputs (this slice only)

Exact string ids, lowercase ASCII, matched as-is. Do not trim or case-fold (`NaCl` is not `nacl`; `" nacl "` is not `nacl`).

| Input | Allowed values | Meaning |
| --- | --- | --- |
| `substance_id` | `nacl`, `sand` | Table salt vs silica sand (SiO₂) |
| `solvent_id` | `water` | Liquid water; no other solvents |
| `temperature_c` | `20` | Bench / room temperature in integer Celsius |

Amounts, stirring, time, and saturation are not modeled. The question is qualitative: does this named solid dissolve in water at the bench?

## Outcomes

A successful call returns `dissolved` (boolean) plus a stable English `explanation` for the UI. Use these strings verbatim so tests and copy stay aligned.

### `nacl` + `water` + `20`

- **dissolved:** `true`
- **explanation:** `Sodium chloride (NaCl) dissolves in water at bench temperature.`

### `sand` + `water` + `20`

- **dissolved:** `false`
- **explanation:** `Sand (silica) does not dissolve in water at bench temperature.`

`dissolved: false` is a successful prediction (sand in water). It is not an error.

## Errors (not dissolve results)

Anything outside the table is an error. Unknown materials must **not** be treated as insoluble.

| Case | Suggested code | Meaning |
| --- | --- | --- |
| `substance_id` not `nacl` or `sand` | `unknown_substance` | Not in this slice |
| `solvent_id` not `water` | `unsupported_solvent` | Only water |
| `temperature_c` not `20` | `unsupported_temperature` | No heat / temperature model |
| empty or whitespace-only ids | `invalid_input` | Reject, do not guess |

Do not invent boiling-water, ethanol, or “mystery powder” chemistry. Wrong temperature or solvent fails; it does not return a different `dissolved` flag.

## Display names (later UI picker)

| id | Label |
| --- | --- |
| `nacl` | Sodium chloride (NaCl) |
| `sand` | Sand |
| `water` | Water (implicit; no solvent picker in this slice) |

## Out of scope

- Evaporate, heat sources, concentrations, stoichiometry, 3D glassware
- Other substances or solvents; mixing two solutes; inventory
- Solubility numbers (g/L), ions, leftover solid, Ksp
- A general reaction or solubility engine
- HTTP route, CSRF, and welcome-page control (this spec does not implement them)

## Hint for issue #7 (do not implement here)

Pure function in `chemlab-core`, tests first, table-driven:

`dissolve(substance_id, solvent_id, temperature_c) → Result<{ dissolved, explanation }, error>`

`LabEngine::can_simulate()` may stay `false`; this slice is two rows, not a simulator.
