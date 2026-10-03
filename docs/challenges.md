# Lab challenges

Human catalog of the lab modes. The bench runs in exactly one **mode** at a time: `free`
(today's default bench) or one challenge id. The server owns the mode, the start scene, and
the completion check; the browser only posts `select_mode` and renders what the scene says.

Engine mirror: [`crates/chemlab-core/src/challenges.rs`](../crates/chemlab-core/src/challenges.rs).
UI copy mirror: [`apps/web/src/lib/challenges.ts`](../apps/web/src/lib/challenges.ts).
Adding a challenge = a section here + a `Challenge` entry in the Rust catalog + a catalog entry
in the web mirror + tests.

## Rules that hold for every mode

- **Full tool set.** Challenges never hide the spoon, pipette, tongs, filter paper, evaporation
  dish, or burner.
- **Switching mode is a hard reset.** Selecting Free mode or a challenge (`select_mode`) always
  rebuilds that mode's start scene; there is no partial progress across a switch.
- **Reset resets the current mode.** The bench Reset button rebuilds the start scene of the mode
  the lab is in, not always Free.
- **Free mode is never "completed".** Completion (`challenge_completed` on the scene) is derived
  after every action from the challenge's win condition, so undoing the winning move un-wins it.
- Win clauses may include **solid mass in any named vessel** (stocks, filter paper, main beaker)
  plus extra server-side checks (pH, temperature, aqueous ions, leftover liquid/solid, HCl w/w,
  and acid-into-water dilution order). The browser never evaluates wins.
- A challenge start scene is the Free bench with the challenge's edits applied: ingredient stocks
  outside the allowed list are removed, listed stocks start empty, and the main beaker
  (`beaker-water`) is preloaded with the listed dry solids. Optional `distilled_water_ml`
  overrides the Free-mode H2O stock fill (capacity stays 100 ml). Free mode also
  includes a 10 ml 30% w/w HCl stock (`beaker-hcl`), a 2.00 g solid NaOH stock
  (`beaker-naoh`), and a 2.00 g solid Na₂SO₄ stock (`beaker-na2so4`); challenges omit
  them unless listed.

| Mode | Id | Selected by default |
| --- | --- | --- |
| Free mode | `free` | yes |
| Separate salt from sand | `separate-nacl-sio2` | no |
| Create table salt | `create-table-salt` | no |
| Create sodium sulfate | `create-sodium-sulfate` | no |
| Precipitate gypsum | `precipitate-gypsum` | no |
| Make hydrochloric acid | `make-hcl-from-gypsum` | no |
| Hot pack | `hot-pack-cacl2` | no |
| Crash salt with acid | `common-ion-nacl` | no |
| Neutralise to pH 7 | `neutralize-to-ph7` | no |
| Dilute sulfuric acid safely | `dilute-sulfuric-safe` | no |
| Concentrate hydrochloric acid | `concentrate-hcl-azeotrope` | no |

## `separate-nacl-sio2` — Separate salt from sand

| Field | Value |
| --- | --- |
| Id | `separate-nacl-sio2` |
| Title | Separate salt from sand |
| Prompt | The previous student accidentally put all the salt and sand in the main beaker, can you please separate them and put them back in their containers? |
| Done | Thank you. |
| Main beaker (`beaker-water`) | 2.0 g solid `nacl` + 2.0 g solid `sand`, dry (no water) |
| Allowed ingredient stocks | `beaker-h2o` (10 ml; Free keeps 100 ml), `beaker-nacl`, `beaker-sand` (no `beaker-hcl`, no `beaker-naoh`, no `beaker-na2so4`) |
| Empty at start | `beaker-nacl`, `beaker-sand` |
| Removed from the bench | `beaker-cacl2`, `beaker-hcl`, `beaker-naoh`, `beaker-na2so4`, `beaker-h2so4` |
| Win | `beaker-nacl` holds ~2.0 g solid `nacl` **and** `beaker-sand` holds ~2.0 g solid `sand` (Exact mass compare with the core float tolerance) |
| Tools | Full set |

Intended solution: dissolve the salt, filter off the sand, evaporate the filtrate to recover the
salt, then spoon each solid back into its own stock beaker.

## `create-table-salt` — Create table salt

| Field | Value |
| --- | --- |
| Id | `create-table-salt` |
| Title | Create table salt |
| Prompt | We're out of NaCl again, can you create some for us? |
| Done | Thank you again. |
| Main beaker (`beaker-water`) | Empty (no preload) |
| Allowed ingredient stocks | `beaker-h2o` (full 100 ml), `beaker-hcl` (full), `beaker-naoh` (full), `beaker-nacl` |
| Empty at start | `beaker-nacl` |
| Removed from the bench | `beaker-cacl2`, `beaker-sand`, `beaker-na2so4`, `beaker-h2so4` |
| Win | `beaker-nacl` holds **at least** 0.20 g solid `nacl` (AtLeast; aqueous NaCl does not count) |
| Tools | Full set |

Intended solution: combine aqueous HCl with solid NaOH (acid excess preferred), evaporate the brine
to dryness, then spoon pure solid NaCl back into the empty NaCl stock.

## `create-sodium-sulfate` — Create sodium sulfate

| Field | Value |
| --- | --- |
| Id | `create-sodium-sulfate` |
| Title | Create sodium sulfate |
| Prompt | We're out of sodium sulfate, can you make some from sulfuric acid and sodium hydroxide? |
| Done | Thank you. |
| Main beaker (`beaker-water`) | Empty (no preload) |
| Allowed ingredient stocks | `beaker-h2o` (full), `beaker-h2so4` (full), `beaker-naoh` (full), `beaker-na2so4` |
| Empty at start | `beaker-na2so4` |
| Removed from the bench | `beaker-hcl`, `beaker-nacl`, `beaker-cacl2`, `beaker-sand` |
| Win | **at least** 0.20 g solid `na2so4` in `beaker-na2so4` **or** `dish-1` (leftover liquid H₂SO₄ can block spooning from the dish, so the dish solid also counts) |
| Tools | Full set |

Intended solution: combine H₂SO₄ with NaOH (about 1 : 2), evaporate in the dish. Spoon into the empty stock when the dish is dry; if leftover sulfuric remains as liquid, the solid in the dish still counts.

## `precipitate-gypsum` — Precipitate gypsum

| Field | Value |
| --- | --- |
| Id | `precipitate-gypsum` |
| Title | Precipitate gypsum |
| Prompt | We need gypsum. Mix calcium chloride with a sulfate and filter off the solid. |
| Done | Thank you. |
| Main beaker (`beaker-water`) | Empty (no preload) |
| Allowed ingredient stocks | `beaker-h2o` (full), `beaker-cacl2` (full), `beaker-na2so4` (full), `beaker-h2so4` (full) |
| Empty at start | none |
| Removed from the bench | `beaker-hcl`, `beaker-nacl`, `beaker-naoh`, `beaker-sand` |
| Win | `filter-paper-1` holds **at least** 0.15 g solid `caso4` (AtLeast; filter-paper solids count) |
| Tools | Full set |

Intended solution: dissolve CaCl₂ and Na₂SO₄ (or add H₂SO₄), mix so gypsum precipitates, pour through the filter.

## `make-hcl-from-gypsum` — Make hydrochloric acid

| Field | Value |
| --- | --- |
| Id | `make-hcl-from-gypsum` |
| Title | Make hydrochloric acid |
| Prompt | Make hydrochloric acid by mixing calcium chloride with sulfuric acid, then filter off the gypsum. |
| Done | Thank you. |
| Main beaker (`beaker-water`) | Empty (no preload) |
| Allowed ingredient stocks | `beaker-h2o` (full), `beaker-cacl2` (full), `beaker-h2so4` (full) |
| Empty at start | none |
| Removed from the bench | `beaker-hcl`, `beaker-nacl`, `beaker-naoh`, `beaker-na2so4`, `beaker-sand` |
| Win | `filter-paper-1` holds **at least** 0.15 g solid `caso4`, **and** a **single** vessel (filtrate or dish) is acidic HCl (pH ≤ 3 with aqueous `h+` and `cl-` on that same vessel) |
| Tools | Full set |

Intended solution: dissolve CaCl₂, add H₂SO₄ so gypsum forms and aqueous HCl remains, filter. Optional dish boil toward the azeotrope is not required.

## `hot-pack-cacl2` — Hot pack

| Field | Value |
| --- | --- |
| Id | `hot-pack-cacl2` |
| Title | Hot pack |
| Prompt | Dissolve the calcium chloride in a little distilled water and check that the beaker warms up. |
| Done | Thank you. |
| Main beaker (`beaker-water`) | Empty (no preload) |
| Allowed ingredient stocks | `beaker-h2o` (10 ml), `beaker-cacl2` (full 2 g) |
| Empty at start | none |
| Removed from the bench | `beaker-hcl`, `beaker-h2so4`, `beaker-nacl`, `beaker-naoh`, `beaker-na2so4`, `beaker-sand` |
| Win | Main beaker T ≥ **25 °C**, aqueous `ca2+` and `cl-` present, leftover solid `cacl2` negligible. (Beaker heat capacity keeps a 2 g dump well below a calorimeter 30 °C; 25 °C is the judged bar.) |
| Tools | Full set |

Intended solution: pour the 10 ml water into the main beaker, dump the CaCl₂ stock, wait for kinetic dissolve, inspect temperature.

## `common-ion-nacl` — Crash salt with acid

| Field | Value |
| --- | --- |
| Id | `common-ion-nacl` |
| Title | Crash salt with acid |
| Prompt | Make a salty solution, then add hydrochloric acid until extra salt crashes out. |
| Done | Thank you. |
| Main beaker (`beaker-water`) | Empty (no preload) |
| Allowed ingredient stocks | `beaker-h2o` (10 ml), `beaker-hcl` (full), `beaker-nacl` (full) |
| Empty at start | none |
| Removed from the bench | `beaker-h2so4`, `beaker-naoh`, `beaker-na2so4`, `beaker-cacl2`, `beaker-sand` |
| Win | Main beaker holds **at least** 0.05 g solid `nacl`, plus aqueous `h+` and `na+` (so the solid is not just dry leftover salt with no brine) |
| Tools | Full set |

Intended solution: dissolve the 2 g NaCl in the 10 ml water (clear brine), then pipette in 30% HCl so the common-ion effect crashes extra solid.

## `neutralize-to-ph7` — Neutralise to pH 7

| Field | Value |
| --- | --- |
| Id | `neutralize-to-ph7` |
| Title | Neutralise to pH 7 |
| Prompt | Neutralise sodium hydroxide with hydrochloric acid and check the inspect pH — a 1 ml pipette cannot land on 7. |
| Done | Thank you. |
| Main beaker (`beaker-water`) | Empty (no preload) |
| Allowed ingredient stocks | `beaker-h2o` (full), `beaker-hcl` (full), `beaker-naoh` (full) |
| Empty at start | none |
| Removed from the bench | `beaker-h2so4`, `beaker-nacl`, `beaker-na2so4`, `beaker-cacl2`, `beaker-sand` |
| Win | Main beaker inspect pH in **0–13**, aqueous `na+` and `cl-` present, leftover solid `naoh` negligible |
| Tools | Full set |

Intended solution: dissolve NaOH, pipette HCl, inspect pH. A 1 ml pipette / 0.2 g scoop cannot titre into 6.5–7.5; the judged band is whatever those aliquots actually hit.

## `dilute-sulfuric-safe` — Dilute sulfuric acid safely

| Field | Value |
| --- | --- |
| Id | `dilute-sulfuric-safe` |
| Title | Dilute sulfuric acid safely |
| Prompt | Dilute the concentrated sulfuric acid the safe way: add the acid into water, not water onto the acid. |
| Done | Thank you. |
| Main beaker (`beaker-water`) | Empty (no preload) |
| Allowed ingredient stocks | `beaker-h2o` (full), `beaker-h2so4` (full) |
| Empty at start | none |
| Removed from the bench | `beaker-hcl`, `beaker-nacl`, `beaker-naoh`, `beaker-na2so4`, `beaker-cacl2`, `beaker-sand` |
| Win | Main beaker: acid was added **into water** (water-onto-acid does not count), no leftover liquid `h2so4`, aqueous `h+` present, T ≥ 20.5 °C, pH ≤ 2 |
| Tools | Full set |

Intended solution: water in the main beaker first, then pipette or pour concentrated H₂SO₄ into that water.

## `concentrate-hcl-azeotrope` — Concentrate hydrochloric acid

| Field | Value |
| --- | --- |
| Id | `concentrate-hcl-azeotrope` |
| Title | Concentrate hydrochloric acid |
| Prompt | Dilute the hydrochloric acid, heat it in the dish, and stop near the azeotrope — you cannot boil it to pure HCl. |
| Done | Thank you. |
| Main beaker (`beaker-water`) | Empty (no preload) |
| Allowed ingredient stocks | `beaker-h2o` (full), `beaker-hcl` (full 30% w/w) |
| Empty at start | none |
| Removed from the bench | `beaker-h2so4`, `beaker-nacl`, `beaker-naoh`, `beaker-na2so4`, `beaker-cacl2`, `beaker-sand` |
| Win | Dish was diluted below 18% w/w HCl, then boiled so liquid HCl is **18–22% w/w** at T ≥ 99 °C |
| Tools | Full set |

Intended solution: dilute the 30% stock in the dish (inspect HCl % w/w), then heat so composition **rises** toward ~20% w/w. Mixing stock to 20% without heat, or boiling 30% stock down, does not count.
