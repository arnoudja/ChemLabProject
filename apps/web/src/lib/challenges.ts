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
]

/** Picker options: Free mode first (the default), then the catalog. */
export const LAB_MODE_OPTIONS: { id: string; title: string }[] = [
  { id: FREE_MODE, title: 'Free mode' },
  ...CHALLENGES.map(({ id, title }) => ({ id, title })),
]

export function findChallenge(mode: string): Challenge | null {
  return CHALLENGES.find((challenge) => challenge.id === mode) ?? null
}
