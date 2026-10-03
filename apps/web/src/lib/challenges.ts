/**
 * UI mirror of `docs/challenges.md` and `chemlab-core::challenges` — keep in sync.
 *
 * Only the player-facing copy lives here; the bench layout, the win condition, and
 * `challenge_completed` stay server-authoritative and arrive on the scene.
 */
export const FREE_MODE = 'free'

export type Challenge = {
  id: string
  title: string
  prompt: string
  done: string
}

export const CHALLENGES: Challenge[] = [
  {
    id: 'separate-nacl-sio2',
    title: 'Separate salt from sand',
    prompt:
      'The previous student accidentally put all the salt and sand in the main beaker, can you please separate them and put them back in their containers?',
    done: 'Thank you.',
  },
  {
    id: 'create-table-salt',
    title: 'Create table salt',
    prompt: "We're out of NaCl again, can you create some for us?",
    done: 'Thank you again.',
  },
  {
    id: 'create-sodium-sulfate',
    title: 'Create sodium sulfate',
    prompt: "We're out of sodium sulfate, can you make some from sulfuric acid and sodium hydroxide?",
    done: 'Thank you.',
  },
  {
    id: 'precipitate-gypsum',
    title: 'Precipitate gypsum',
    prompt: 'We need gypsum. Mix calcium chloride with a sulfate and filter off the solid.',
    done: 'Thank you.',
  },
  {
    id: 'make-hcl-from-gypsum',
    title: 'Make hydrochloric acid',
    prompt:
      'Make hydrochloric acid by mixing calcium chloride with sulfuric acid, then filter off the gypsum.',
    done: 'Thank you.',
  },
  {
    id: 'hot-pack-cacl2',
    title: 'Hot pack',
    prompt:
      'Dissolve the calcium chloride in a little distilled water and check that the beaker warms up.',
    done: 'Thank you.',
  },
  {
    id: 'common-ion-nacl',
    title: 'Crash salt with acid',
    prompt: 'Make a salty solution, then add hydrochloric acid until extra salt crashes out.',
    done: 'Thank you.',
  },
  {
    id: 'neutralize-to-ph7',
    title: 'Neutralise to pH 7',
    prompt: 'Neutralise sodium hydroxide with hydrochloric acid until the inspect pH is about 7.',
    done: 'Thank you.',
  },
  {
    id: 'dilute-sulfuric-safe',
    title: 'Dilute sulfuric acid safely',
    prompt:
      'Dilute the concentrated sulfuric acid the safe way: add the acid into water, not water onto the acid.',
    done: 'Thank you.',
  },
  {
    id: 'concentrate-hcl-azeotrope',
    title: 'Concentrate hydrochloric acid',
    prompt:
      'Heat hydrochloric acid in the dish and stop near the azeotrope — you cannot boil it to pure HCl.',
    done: 'Thank you.',
  },
]

/** Picker options: Free mode first (the default), then the catalog. */
export const LAB_MODE_OPTIONS: { id: string; title: string }[] = [
  { id: FREE_MODE, title: 'Free mode' },
  ...CHALLENGES.map(({ id, title }) => ({ id, title })),
]

export function findChallenge(mode: string): Challenge | null {
  return CHALLENGES.find((challenge) => challenge.id === mode) ?? null
}
