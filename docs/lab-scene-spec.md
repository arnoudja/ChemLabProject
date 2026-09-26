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
| `toggle_burner` | Flip the burner. Stays off when the dish has no liquid. |
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

**Filter pour wash:** after the phase split (solids → paper, liquid+aqueous → fluid parcel) and before mixing into the filtrate, fine soluble solids on the paper (`nacl`, `cacl2`, `naoh`) partially dissolve into the fluid with contact time `τ ∝ V_fluid` at the energy-weighted blend T of fluid + paper solids. For salts `m_diss = min(avail, unsaturated_cap) · (1 − exp(−k·τ))` (common-ion capacity from `solubility`, including Cl⁻ from aqueous HCl); NaOH is highly soluble (no SI cap): `m_diss = avail · (1 − exp(−k·τ))`. Sand never dissolves. Dissolve ΔH adjusts the fluid parcel temperature before the filtrate blend.

### Free-mode hydrochloric acid stock

Free mode includes `beaker-hcl` (**Hydrochloric acid (30%)**): **10.00 ml** solution at **30% w/w**, density **1.149 g/ml** → mass **11.49 g** (HCl **3.447 g** → aqueous `h+`/`cl-` moles; water **8.043 g** as liquid `water`). Pipette and tongs treat it like distilled water (solution volume = composition mass / HCl–water density; pure water stays 1 g/ml). Put-back into the HCl stock requires water + stoichiometric H⁺/Cl⁻ at 30% w/w (±0.5% w/w). Spoon is disallowed on the stock. Challenge layouts that omit `beaker-hcl` from their allow-list do not get it (e.g. `separate-nacl-sio2`).

Mixing parcels that change HCl concentration applies tabulated **integral dilution enthalpy** (exothermic when concentrating → diluting) as `ΔT = −Q / C_eff` after the sensible blend. Salt dissolve into acidic water reuses the water-solvent path; `enforce_saturation` preserves `h+` and includes its Cl⁻ in the mixed SI / unsaturated capacity. Na⁺ paired with OH⁻ is excluded from the NaCl inventory so dissolving NaOH cannot invent Cl⁻.

Inspect shows **pH** for aqueous acid or base: strong-acid approx `pH = −log₁₀([H⁺])`; strong-base approx `pH = 14 + log₁₀([OH⁻])`. When neutralization has consumed both `h+` and `oh-`, no pH is shown (same as pure water — the APIs return `None`).

### Free-mode sodium hydroxide stock

Free mode includes `beaker-naoh` (**Sodium hydroxide**): **2.00 g** solid (`10 × 0.2 g` scoops), tongs + spoon parity with NaCl. Dissolves to aqueous `na+` + `oh-` (1:1, M = 40.00 g/mol) with ΔH_sol = **−44500 J/mol** whenever solid NaOH meets liquid water (spoon pour, tongs dump into a wet vessel, water poured onto dry solid, and filter-paper wash — highly soluble, contact-time fraction with no SI cap). After mix/dissolve/filter wash, `H+ + OH- → H2O` (ΔH_neut = **−55800 J/mol**) runs **before** saturation. Challenge layouts omit `beaker-naoh` unless listed (e.g. `separate-nacl-sio2`). Out of scope: NaOH(aq) density table, base dish evaporation, buffers / extra safety UX.

### Burner heat, ambient cooling, clock

A **pipette** transfers **1.00 ml** of mixed solution (water + aqueous ions in proportion, by **solution volume**). The source vessel must hold at least **3.00 ml** of liquid, so a vessel can never be pipetted dry: a fill from a dry vessel fails with `no_fluid_available` ("No fluid available.") and a fill from a vessel below 3.00 ml fails with `not_enough_fluid_available` ("Not enough fluid available."). Solid SiO₂ / other solids stay in the vessel. The evaporation dish holds at most **25.00 ml**.

While the **burner** is on and heating the dish, dish temperature rises with `dT = (BURNER_POWER_W / C_eff) · dt` toward the boil point. **Documented acid boil rule:** when aqueous HCl (`h+`) is present, the dish boils at the HCl–water azeotrope temperature **108.6 °C**; without acid, boiling stays on the composition-aware Antoine/Raoult curve `T_boil(x_w)`. Fixed lab air: **RH = 50%**, **P_atm = 1.00 bar**, ambient **20 °C**. Water mole fraction `x_w` counts liquid water and aqueous ions only (sand / undissolved solids excluded).

**Burner off (sub-boil):** mass-transfer loss `m_dot = k · A · max(0, p_w − p_air) / P_atm` with `p_w = x_w · P_sat(T)`, `p_air = RH · P_sat(T_amb)`; `A = DISH_EVAP_AREA_M2`, `k` chosen so pure water at 20 °C / 50% RH loses ≈ **2 ml/h**. Every evaporated mass cools the dish by `ΔT = −m · H_vap / C_eff` (`WATER_LATENT_HEAT_J_PER_G = 2257`, used as a water-like latent-heat approx even when HCl co-evaporates). Sub-boil mass transfer is skipped while the burner is heating so Antoine-scaled driving force cannot dry/concentrate the dish before boil.

**At boil** (burner on): clamp `T` to the boil rule above; heat-limited rate `m_dot = BURNER_POWER_W / H_vap` (~0.487 g/s at `BURNER_POWER_W = 1100`). With aqueous HCl, mass loss removes **both water and HCl** with an azeotrope-style vapor split (~**20.2% w/w** azeotrope): vapor is biased so the liquid approaches the azeotrope (lean liquid → vapor leaner in HCl; rich liquid → vapor richer in HCl), removing proportional `h+`/`cl-` with evaporated HCl. Without acid, boil still removes water only and leaves salt ions behind. While heating, Newton cool is skipped for the dish (same as before), so boil `Q_net` is burner power only. Burner off at/above boil does not heat-limit evaporate (`Q_net ≤ 0`); ambient cool lowers `T`. Beakers never evaporate. Mixed NaCl/CaCl₂ equilibrium then precipitates or redissolves on **every aqueous vessel**. The burner turns off when the dish has no liquid.

Every `apply_elapsed` tick also applies **Newton cooling** toward ambient 20.00 °C for thermal vessels (and a filled pipette): `dT = −(UA / C_eff) · (T − T_amb) · dt`, clamped so T does not cross ambient; snap when `|T − 20| < 0.005`. `UA_DISH = 4.0` W/K, `UA_BEAKER = 3.0` W/K. While the burner is actively heating the dish, that dish skips ambient cool for the tick; other vessels still cool. Elapsed heat/cool/evap is `chemlab-core::apply_elapsed`; the HTTP layer owns the clock. The web client polls the lab scene (~300 ms) while the burner is on **or** any thermal item has `|T − 20| ≥ 0.5`.

## Out of scope

- Evaporating the water beaker itself; radiative exchange between adjacent vessels; UI thermometer on the dish SVG
- Multiplayer command log / lab sharing
- 3D glassware
- A general reaction engine
- Other acids/bases; buffer pH; activity-coefficient pH; glass etching / safety UX beyond copy
- HTTP LabBench pipette/burner UI (later PR)
