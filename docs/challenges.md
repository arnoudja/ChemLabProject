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
- A challenge start scene is the Free bench with the challenge's edits applied: ingredient stocks
  outside the allowed list are removed, listed stocks start empty, and the main beaker
  (`beaker-water`) is preloaded with the listed dry solids. Optional `distilled_water_ml`
  overrides the Free-mode H2O stock fill (capacity stays 100 ml).

| Mode | Id | Selected by default |
| --- | --- | --- |
| Free mode | `free` | yes |
| Separate salt from sand | `separate-nacl-sio2` | no |

## `separate-nacl-sio2` — Separate salt from sand

| Field | Value |
| --- | --- |
| Id | `separate-nacl-sio2` |
| Title | Separate salt from sand |
| Prompt | The previous student accidentally put all the salt and sand in the main beaker, can you please separate them and put them back in their containers? |
| Done | Thank you. |
| Main beaker (`beaker-water`) | 2.0 g solid `nacl` + 2.0 g solid `sand`, dry (no water) |
| Allowed ingredient stocks | `beaker-h2o` (10 ml; Free keeps 100 ml), `beaker-nacl`, `beaker-sand` |
| Empty at start | `beaker-nacl`, `beaker-sand` |
| Removed from the bench | `beaker-cacl2` |
| Win | `beaker-nacl` holds ~2.0 g solid `nacl` **and** `beaker-sand` holds ~2.0 g solid `sand` (mass compared with the core float tolerance) |
| Tools | Full set |

Intended solution: dissolve the salt, filter off the sand, evaporate the filtrate to recover the
salt, then spoon each solid back into its own stock beaker.
