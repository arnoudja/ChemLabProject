# Lab scene: items, actions, and pour→dissolve

Short wire contract for the lab-scene redesign. Chemistry stays in `chemlab-core`; this document locks **ownership**, **item properties**, and the **action vocabulary** the browser may send. Dissolve substance ids stay exact as in [`dissolve-spec.md`](dissolve-spec.md).

## Authority

| Concern | Owner |
| --- | --- |
| Item catalog in a lab (spoon, beakers), locations, properties (volume ml, fill ml, colour/transparency, temperature, composition) | Backend / `chemlab-core` scene |
| Dissolve prediction table | `chemlab-core::dissolve` (unchanged ids + explanations) |
| Render scene + click → action POST | Frontend |
| Persist `LabScene` JSON blob per lab row | `chemlab-db` → `labs.state_blob` |

The server owns items, locations, and properties. The frontend posts **actions only** (`use_tool`, `pour`). It never chooses a dissolve triple (`substance_id` / `solvent_id` / `temperature_c`) for scene play — those come from item properties on the server.

**Dissolve is a consequence of pouring** a solid into water inside `chemlab-core`, not a separate client decision.

## Exact dissolve ids (unchanged)

Matched as-is (no trim or case-fold):

| Role | Allowed values |
| --- | --- |
| Substance | `nacl`, `cacl2`, `sand` |
| Solvent | `water` |
| Temperature (°C) | `20` |

Explanations stay verbatim from the dissolve spec.

## Action vocabulary (v1)

| `type` | Meaning |
| --- | --- |
| `use_tool` | Active tool item used on a target item (e.g. spoon → NaCl beaker scoops salt onto spoon) |
| `pour` | Pour from held/source item into target (e.g. spoon with NaCl → water beaker). Server may call `dissolve()` when solid meets water @ 20 °C. |

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
- `ItemProperties` — optional volume/fill/flags/temperature (°C as `f64`); `composition` and `holding` lists
- `Item` — `id`, `kind`, `label`, `location`, `properties`
- `LabScene` — `lab_id`, `version`, ambient `temperature_c` (default `20.0`), `items`, optional `last_events`
- `LabEvent` — `kind` (e.g. `scooped`, `returned`, `poured`, `dissolved`, `did_not_dissolve`), `message`
- `LabAction` — tagged `type`: `use_tool` \| `pour` \| `put_away` \| `reset`
- `LabActionResponse` — `{ "scene": LabScene }`

When NaCl dissolves into water, the scene engine applies **endothermic** cooling to the water beaker's `properties.temperature_c` (ΔH_sol ≈ 3.88 kJ/mol; water c_p = 4.184 J/(g·K); water mass ≈ liquid `amount_ml`). When CaCl₂ dissolves, it applies **exothermic** heating (ΔH_sol ≈ −81.3 kJ/mol) with the same mass/c_p model. Ambient `LabScene.temperature_c` is unchanged. Sand that does not dissolve leaves temperature unchanged. The frontend formats inspect mass (g) and volume (ml) to **two decimal places**, aqueous molarity (M) to **three significant digits**, and beaker/scene temperature to two decimal places.

## Out of scope

- Evaporate, heat sources, stoichiometry, concentrations beyond crude scoop/ml props
- Multiplayer command log / lab sharing
- 3D glassware
- A general reaction or solubility engine
- HTTP scene routes and LabBench rewrite (later PRs; this spec only locks the contract)
