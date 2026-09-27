# Lab scene: items, actions, and pour→dissolve

Short wire contract for the lab-scene redesign. Chemistry stays in `chemlab-core`; this document locks **ownership**, **item properties**, and the **action vocabulary** the browser may send. Dissolve substance ids stay exact as in [`dissolve-spec.md`](dissolve-spec.md).

## Authority

| Concern | Owner |
| --- | --- |
| Item catalog in a lab (spoon, pipette, beakers, evaporation dish, burner), locations, properties (volume ml, fill ml, colour/transparency, temperature, composition, burner `on`, pipette `source_item_id`) | Backend / `chemlab-core` scene |
| Dissolve prediction table | `chemlab-core::dissolve` (unchanged ids + explanations) |
| Render scene + click → action POST | Frontend |
| Persist `LabScene` JSON blob per lab row | `chemlab-db` → `labs.state_blob` |

The server owns items, locations, and properties. The frontend posts **actions only** (`use_tool`, `pour`, `put_away`, `reset`, `toggle_burner`, `select_mode`). It never chooses a dissolve triple (`substance_id` / `solvent_id` / `temperature_c`) for scene play — those come from item properties on the server.

**Dissolve is a consequence of pouring** a solid into water inside `chemlab-core`, not a separate client decision.

## Exact dissolve ids (unchanged)

Matched as-is (no trim or case-fold):

| Role | Allowed values |
| --- | --- |
| Substance | `nacl`, `cacl2`, `naoh`, `sand` |
| Solvent | `water` |
| Temperature (°C) | `20` |

Explanations stay verbatim from the dissolve spec.

## Action vocabulary (v1)

| `type` | Meaning |
| --- | --- |
| `use_tool` | Active tool item used on a target item (e.g. spoon → NaCl beaker scoops salt onto spoon; empty pipette → water or dish fills 1.00 ml of solution from a source holding at least 3.00 ml; full pipette → water or dish empties) |
| `pour` | Pour from held/source item into target (e.g. spoon with NaCl → water beaker, or full pipette → dish). Server may call `dissolve()` when solid meets water. Dish liquid is capped at **25.00 ml**. |
| `put_away` | Return the spoon or pipette to the bench. Held scoops restore to matching stock; a full pipette returns 1.00 ml to its last source. |
| `reset` | Rebuild the start bench of the lab's current mode (empty dish, burner off, empty pipette, T = 20 °C). |
| `toggle_burner` | Flip the burner. Stays off when the dish has no liquid water (liquid H₂SO₄ alone is not enough). |
| `select_mode` | Switch the bench to `"free"` or a challenge id from the [challenge catalog](challenges.md); always a hard reset into that mode's start scene. Unknown id → `400 unknown_mode`. |

The scene carries `mode` (`"free"` when a save predates modes) and the derived
`challenge_completed`; both are server-owned.

Successful responses return the full updated `LabScene` (and optional `events` / `last_events` for UI copy).

## Example: water beaker item (scene fragment)

```json
{
  "id": "beaker-water",
  "kind": "beaker",
  "label": "Water",
  "location": "bench",
  "properties": {
    "volume_ml": 250,
    "fill_ml": 200,
    "transparent": true,
    "colourless": true,
    "temperature_c": 20,
    "composition": [
      { "substance_id": "water", "phase": "liquid", "amount_ml": 200 }
    ]
  }
}
```

## Example: action `use_tool` spoon → NaCl

```json
{
  "type": "use_tool",
  "tool_item_id": "spoon-1",
  "target_item_id": "beaker-nacl"
}
```

After this action the spoon item should carry scooped NaCl in its properties (server-owned), e.g.:

```json
{
  "id": "spoon-1",
  "kind": "spoon",
  "label": "Spoon",
  "location": "hand",
  "properties": {
    "holding": [
      { "substance_id": "nacl", "phase": "solid", "amount_scoop": 1, "amount_g": 0.2 }
    ]
  }
}
```

Pour into water is a separate action, e.g. `{ "type": "pour", "source_item_id": "spoon-1", "target_item_id": "beaker-water" }`. For `nacl`, dissolve succeeds and the water composition gains **server-authored** aqueous ions (`na+`, `cl-`, phase `aqueous`) with no leftover solid grains — the client must not invent ion dissociation. For `sand`, dissolve returns `dissolved: false` and the water beaker (or bench residue) gains **server-authored** undissolved sand properties — the client must not infer leftovers from a boolean alone.

## Wire types (summary)

- `CompositionEntry` — `substance_id`, `phase` (`solid` \| `liquid` \| `aqueous`), optional `amount_ml` / `amount_scoop` / `amount_g` / `amount_mol`
- One spoon scoop of solid is **0.2 g** (`amount_g`). Scooping with `use_tool` **depletes** the source beaker's `amount_scoop` / `amount_g` (server-authoritative). Using the spoon on the **matching** stock beaker while holding that solid **returns** the scoop (empties holding, restores stock). Wrong stock (e.g. salt→sand) is rejected. Putting the spoon **away** (`put_away`) while holding a scoop also returns it to the matching stock (same substance matching) and sets the spoon back on the bench; empty-spoon put-away is a no-op for stock. Dissolved NaCl authors aqueous `na+` / `cl-` with `amount_mol` (aggregated on repeat pours). Dissolved CaCl₂ authors aqueous `ca2+` / `cl-` (1:2) with `amount_mol` and **exothermic** heating (ΔH_sol ≈ −81.3 kJ/mol). Undissolved solids (e.g. sand / SiO₂) aggregate `amount_g` on one composition line. Liquid water uses `amount_ml` for inspect volume display and for the water beaker fill height (relative to the initial 200 ml bench volume), mirroring how stock solids use `amount_g` for pile height.
- `ItemProperties` — optional volume/fill/flags/temperature (°C as `f64`); `composition` and `holding` lists; optional burner `on`; optional pipette `source_item_id`
- `Item` — `id`, `kind` (`beaker` \| `spoon` \| `pipette` \| `evaporation_dish` \| `burner`), `label`, `location`, `properties`
- `LabScene` — `lab_id`, `version`, ambient `temperature_c` (default `20.0`), `items`, optional `last_events`, optional `last_applied_unix_ms`
- `LabEvent` — `kind` (e.g. `scooped`, `returned`, `poured`, `pipetted`, `toggled`, `dissolved`, `did_not_dissolve`), `message`
- `LabAction` — tagged `type`: `use_tool` \| `pour` \| `put_away` \| `reset` \| `toggle_burner`
- `LabActionResponse` — `{ "scene": LabScene }`

When NaCl dissolves into water, the scene engine applies **endothermic** cooling to the water beaker's `properties.temperature_c` (ΔH_sol ≈ 3.88 kJ/mol; ΔT = −n·ΔH / C_eff). When CaCl₂ dissolves, it applies **exothermic** heating (ΔH_sol ≈ −81.3 kJ/mol) with the same C_eff model. Ambient `LabScene.temperature_c` is unchanged. Sand that does not dissolve still energy-weights its solid heat capacity into the target. The frontend formats inspect mass (g) and volume (ml) to **two decimal places**, aqueous molarity (M) to **three significant digits**, and beaker/scene temperature to two decimal places.

### Heat capacity and temperature blending

`effective_heat_capacity(item) = C_vessel(kind) + Σ contents m·c_p` (tools / burner / filter paper skipped; pipette = liquid holding only):

| Carrier | `c_p` / `C` |
|---------|-------------|
| Liquid water | 4.184 J/(g·K), mass ≈ ml |
| Dissolved ions | counted with the water solvent (no extra term) |
| Solid `nacl` | 0.88 J/(g·K) |
| Solid `cacl2` | 0.67 J/(g·K) |
| Solid `sand` | 0.74 J/(g·K) |
| Solid `naoh` | 1.49 J/(g·K) |
| Glass beaker body | `C_BEAKER = 150` J/K (incl. filtrate / distilled-water stocks) |
| Evaporation dish body | `C_DISH = 80` J/K |

Transfers (pipette empty, tongs liquid/solid pour, filter fluid → filtrate, spoon pour) use energy-weighted blends:

`T_f = (C_dest·T_dest + C_add·T_source) / (C_dest + C_add)`

where `C_dest` is the destination's effective heat capacity **before** the add. A **spoon** holding solids carries `temperature_c` from the source vessel and clears it on empty / put-away.

**Filter pour wash:** after the phase split (solids → paper, liquid+aqueous → fluid parcel) and before mixing into the filtrate, fine soluble solids on the paper (`nacl`, `cacl2`, `naoh`) partially dissolve into the fluid with contact time `τ ∝ V_fluid` at the energy-weighted blend T of fluid + paper solids. For salts `m_diss = min(avail, unsaturated_cap) · (1 − exp(−k·τ))` (common-ion capacity from `solubility` with HCl gate `min(max(n_h − n_oh, 0), n_cl)` and aq SO₄ in I — sulfuric filtrate does not invent Cl⁻); NaOH is highly soluble (no SI cap): `m_diss = avail · (1 − exp(−k·τ))`. Sand never dissolves. Dissolve ΔH adjusts the fluid parcel temperature before the filtrate blend.

### Free-mode hydrochloric acid stock

Free mode includes `beaker-hcl` (**Hydrochloric acid (30%)**): **10.00 ml** solution at **30% w/w**, density **1.149 g/ml** → mass **11.49 g** (HCl **3.447 g** → aqueous `h+`/`cl-` moles; water **8.043 g** as liquid `water`). Solution volume uses additive apparent molar volumes Φ_V (ml/mol): HCl **20.70**, H₂SO₄ **40.0**, NaHSO₄ **30.0**, NaOH **4.0**, Na₂SO₄ **20.0**, CaSO₄ **15.0**, NaCl **22.0**, CaCl₂ **34.0**, with pairing **HCl → NaOH → NaHSO₄ → H₂SO₄ → Na₂SO₄ → CaSO₄ → NaCl → CaCl₂** (`V = water_ml + liquid_h2so4_ml + Σ n·Φ_V`). Na⁺ needed for leftover Cl⁻ is reserved before bisulfate salt pairing; free H₂SO₄ is leftover acid `hso4-` / free `h+`·`so4^2-` only (salt bisulfate is NaHSO₄). Ca²⁺ without Cl⁻ pairs as CaSO₄(aq) when SO₄ remains; CaCl₂ Φ_V is only used for Ca matched to leftover Cl⁻. **HCl inventory** for Φ_V_HCl, dilution Φ_L, dish VLE, and SI / filter-wash common-ion is `min(max(n(h+) − n(oh-), 0), n(cl-))` — Kw residuals and bare sulfuric / `hso4-` are not treated as HCl. Stock HCl is locked at **10.00 ml** via Φ_HCl. Pipette and tongs use this solution volume for fill / capacity; SI and filter-wash τ stay on liquid-water ml. Put-back into the HCl stock requires water + stoichiometric H⁺/Cl⁻ at 30% w/w (±0.5% w/w). Spoon is disallowed on the stock. Challenge layouts that omit `beaker-hcl` from their allow-list do not get it (e.g. `separate-nacl-sio2`).

Mixing parcels that change HCl concentration applies tabulated **integral dilution enthalpy** (exothermic when concentrating → diluting) as `ΔT = −Q / C_eff` after the sensible blend. Salt dissolve into acidic water reuses the water-solvent path; `enforce_saturation` preserves acid inventory (collapses `hso4-` into free `h+`+`so4^2-` for the SI mass balance) and includes **HCl** Cl⁻ with the same free-H⁺ gate `min(max(n_h − n_oh, 0), total_cl)` — sulfuric `h+` / `hso4-` alone never invents chloride (filter-wash capacity uses the same gate and threads aq SO₄ into I). Na⁺ paired with OH⁻ is excluded from the NaCl/Na₂SO₄ inventory so dissolving NaOH cannot invent Cl⁻. Mixed SI uses Davies γ with soft-capped effective ionic strength `I_eff = I/(1+I/3)` (`I` includes all aqueous H⁺, OH⁻, and SO₄²⁻; no hard I=1 clamp; no Pitzer). Chloride IAP uses partitioned salt Na/Ca + HCl inventory after sulfate pairing (no `-2 m_SO4` undercount). Pure-curve K(T) is fitted with the same `I_eff` so pure sat still matches literature.

### Free-mode sulfuric acid stock

Free mode includes `beaker-h2so4` (**Sulfuric acid**): **10.00 ml** of nearly anhydrous liquid H₂SO₄ at room temperature (~**98% w/w**, density **1.83 g/ml** → moles `(V·ρ·w)/M` with `M = 98.079`). Composition is liquid `h2so4` (not pre-ionized). When the water:H₂SO₄ mole ratio is **above** the stock ~98% w/w threshold, the engine ionizes each formula unit to aqueous **`H⁺ + HSO₄⁻`** (strong first proton), then a charge- and mass-balanced aqueous solver applies school **`K_w`** and sulfuric **`K_a2 ≈ 0.012`** (`HSO₄⁻ ⇌ H⁺ + SO₄²⁻`) over the present ion set (`h+`, `oh-`, `cl-`, `na+`, `ca2+`, `so4^2-`, `hso4-`). Dilution uses an H₂SO₄-specific Φ_L table (strongly exothermic; not HCl’s table). At or below the ~98% threshold (continuous concentrate / dry-out), free sulfuric inventory (`hso4-` + acid `so4^2-`) reforms to liquid `h2so4` after SI — inverse of ionization. Finalize order: ionize → NaOH dissolve → **speciate** (subsumes neutralization + heat) → SI → speciate → reform → speciate → `sync_fill_ml`. Put-back into the stock requires pure liquid H₂SO₄ at the stock concentration (±1% relative). Spoon is disallowed. Free-only — current challenges omit `beaker-h2so4`.

**Dish evaporation:** H₂SO₄ is treated as **non-volatile**. With sulfuric-only acid (no HCl inventory), boil/ambient MT removes **water only**; acid moles are conserved through wet concentration **and dry-out**. While still dilute (above the ~98% threshold) inventory stays as aqueous `h+` / `hso4-` / `so4^2-`; once at/below threshold (including dry-out), free sulfuric is re-authored as liquid `h2so4` (not wiped, not left as orphan ions). When HCl is also present, HCl follows the gated azeotrope path and stays aqueous while only the free sulfuric reforms.

**Reactions:** neutralizing with NaOH is handled by the aqueous solver (1 mol H₂SO₄ needs 2 mol OH⁻ overall; heat from `0.5 · Δ(n_h + n_oh + n_hso4)`, covering H⁺+OH⁻ and HSO₄⁻+OH⁻); spectators are aqueous Na₂SO₄ / NaHSO₄ with **Na₂SO₄(s) SI** when saturated. Mixing with CaCl₂ precipitates **CaSO₄(s) gypsum** (school solubility / SI; bisulfate collapses into free SO₄ for the SI pass) and leaves Cl⁻ with remaining H⁺ as HCl inventory. NaCl + H₂SO₄ coexist as mixed ions with charge-preserving SI. Sand is inert. Oleum, organics charring, acetic/NH₃, and a dedicated H₂SO₄ challenge are out of scope.

Inspect shows **pH** from solved `[H⁺]` after speciation (`pH = −log₁₀([H⁺])`), including ~7 for pure water / exact neutral salt. Strong-base display uses `14 + log₁₀([OH⁻])` only when no free `h+` is authored.

### Free-mode sodium hydroxide stock

Free mode includes `beaker-naoh` (**Sodium hydroxide**): **2.00 g** solid (`10 × 0.2 g` scoops), tongs + spoon parity with NaCl. Dissolves to aqueous `na+` + `oh-` (1:1, M = 40.00 g/mol) with ΔH_sol = **−44500 J/mol** whenever solid NaOH meets liquid water (spoon pour, tongs dump into a wet vessel, water poured onto dry solid, and filter-paper wash — highly soluble, contact-time fraction with no SI cap). After mix/dissolve/filter wash, aqueous speciation (`K_w` + `K_a2`) subsumes `H+ + OH- → H2O` (ΔH_neut = **−55800 J/mol**) **before** saturation. Challenge layouts omit `beaker-naoh` unless listed (e.g. `separate-nacl-sio2`). Out of scope: base dish evaporation, buffers / extra safety UX.

### Free-mode sodium sulfate stock

Free mode includes `beaker-na2so4` (**Sodium sulfate**): **2.00 g** solid (`10 × 0.2 g` scoops), same solid-stock pattern as NaCl / NaOH / CaCl₂ / sand. Substance id `na2so4` matches the existing aqueous / SI / Φ_V paths from H₂SO₄ neutralization. Tongs + spoon scoop/put-away parity with other solids. Challenge layouts omit `beaker-na2so4` unless listed. Kinetic dissolve-rate tuning for this salt is out of scope for the stock addition.

### Burner heat, ambient cooling, clock

A **pipette** transfers **1.00 ml** of mixed solution (water + aqueous ions in proportion, by **solution volume**). The source vessel must hold at least **3.00 ml** of liquid, so a vessel can never be pipetted dry: a fill from a dry vessel fails with `no_fluid_available` ("No fluid available.") and a fill from a vessel below 3.00 ml fails with `not_enough_fluid_available` ("Not enough fluid available."). Solid SiO₂ / other solids stay in the vessel. The evaporation dish holds at most **25.00 ml**.

While the **burner** is on and heating the dish, dish temperature rises with `dT = (BURNER_POWER_W / C_eff) · dt` toward the boil point. **Documented acid boil rule:** when aqueous **HCl inventory** (`min(max(n_h − n_oh, 0), n_cl) > 0`) is present, the dish boils on a tabulated `T_boil(w_HCl)` curve (piecewise linear; dilute near pure-water Antoine ~99.6 °C, peak **108.6 °C** at the ~**20.2% w/w** azeotrope, stock ~30% w/w ≈ **105 °C**). If non-HCl solutes are also present (salt Na⁺/Ca²⁺ / salt Cl⁻ / sulfate or bisulfate concentrates), `T_boil = max(T_hcl(w), T_raoult(x_w))` so elevation is not ignored. Without HCl inventory, boiling stays on the composition-aware Antoine/Raoult curve `T_boil(x_w)` (sulfuric-only solutions use this water path; the acid itself does not evaporate). Fixed lab air: **RH = 50%**, **P_atm = 1.00 bar**, ambient **20 °C**. Water mole fraction `x_w` counts liquid water and aqueous ions only (sand / undissolved solids excluded).

**Burner off (sub-boil):** mass-transfer loss `m_dot = k · A · max(0, p_w − p_air) / P_atm` with `p_w = x_w · P_sat(T)`, `p_air = RH · P_sat(T_amb)`; `A = DISH_EVAP_AREA_M2`, `k` chosen so pure water at 20 °C / 50% RH loses ≈ **2 ml/h**. Evaporated mass cools the dish by `ΔT = −m · H_vap / C_eff`. With HCl inventory, `H_vap = (1 − y)·2257 + y·HCL_LATENT` (school `HCL_LATENT ≈ 440` J/g); water-only uses `WATER_LATENT_HEAT_J_PER_G = 2257`. Sub-boil mass transfer is skipped while the burner is heating so Antoine-scaled driving force cannot dry/concentrate the dish before boil.

**At boil** (burner on): clamp `T` to the boil rule above; heat-limited rate `m_dot = BURNER_POWER_W / H_vap` (same weighted `H_vap` when HCl leaves). With HCl inventory, mass loss removes **both water and HCl** with a documented school `y(w)` vapor table consistent with the boil-T knots (~**20.2% w/w** azeotrope): lean liquid → vapor leaner in HCl; rich liquid → vapor richer in HCl; `|y − w_l|` shrinks as `w_l → w_az`, removing proportional `h+`/`cl-` with evaporated HCl. Without HCl inventory, boil removes water only and leaves salt / sulfate ions (and non-volatile H₂SO₄) behind. While heating, Newton cool is skipped for the dish (same as before), so boil `Q_net` is burner power only. Burner off at/above boil does not heat-limit evaporate (`Q_net ≤ 0`); ambient cool lowers `T`. Beakers never evaporate. Mixed NaCl/CaCl₂/Na₂SO₄/CaSO₄ equilibrium then precipitates or redissolves on **every aqueous vessel**. The burner turns off when the dish has no liquid water (leftover liquid H₂SO₄ after dry-out does not keep it on).

Every `apply_elapsed` tick also applies **Newton cooling** toward ambient 20.00 °C for thermal vessels (and a filled pipette): `dT = −(UA / C_eff) · (T − T_amb) · dt`, clamped so T does not cross ambient; snap when `|T − 20| < 0.005`. `UA_DISH = 4.0` W/K, `UA_BEAKER = 3.0` W/K. While the burner is actively heating the dish, that dish skips ambient cool for the tick; other vessels still cool. Elapsed heat/cool/evap is `chemlab-core::apply_elapsed`; the HTTP layer owns the clock. The web client polls the lab scene (~300 ms) while the burner is on **or** any thermal item has `|T − 20| ≥ 0.5`.

## Out of scope

- Evaporating the water beaker itself; radiative exchange between adjacent vessels; UI thermometer on the dish SVG
- Multiplayer command log / lab sharing
- 3D glassware
- A general reaction engine
- Other acids/bases; buffer pH; activity-coefficient pH; glass etching / safety UX beyond copy
- HTTP LabBench pipette/burner UI (later PR)
