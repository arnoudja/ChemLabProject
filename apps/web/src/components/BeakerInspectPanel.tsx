import type { Item, LabScene } from '../generated/contracts'
import {
  CompositionInspectLine,
  formatTemperatureC,
  formatPh,
  phFromComposition,
  solventVolumeLitres,
} from '../lib/compositionDisplay'
import { optionalArray } from '../lib/scene'
import { itemTemperatureC } from './labBenchScene'

export function BeakerInspectPanel({
  scene,
  item,
  onClose,
}: {
  scene: LabScene
  item: Item
  onClose: () => void
}) {
  const composition = optionalArray(item.properties.composition)
  const temperatureC = itemTemperatureC(scene, item)
  const solventL = solventVolumeLitres(composition)
  const ph = phFromComposition(composition)

  return (
    <aside
      className="lab-beaker-inspect mt-3 rounded-xl border border-[var(--border)] bg-[var(--input-bg)] p-3 text-sm"
      role="dialog"
      aria-label={`Contents of ${item.label}`}
      data-beaker-inspect
    >
      <div className="flex items-start justify-between gap-3">
        <div>
          <p className="font-medium text-[var(--ink)]">{item.label}</p>
          <p className="mt-1 text-[var(--ink-soft)]">Temperature: {formatTemperatureC(temperatureC)}°C</p>
          {ph != null ? (
            <p className="text-[var(--ink-soft)]">pH: {formatPh(ph)}</p>
          ) : null}
        </div>
        <button
          type="button"
          className="rounded-md border border-[var(--border)] px-2 py-1 text-xs text-[var(--ink-soft)] hover:bg-[var(--surface-hover)]"
          onClick={onClose}
        >
          Close
        </button>
      </div>
      {composition.length > 0 ? (
        <ul className="mt-2 list-disc space-y-1 pl-5 text-[var(--ink)]">
          {composition.map((entry, index) => (
            <li key={`${entry.substance_id}-${entry.phase}-${index}`}>
              <CompositionInspectLine entry={entry} solventVolumeL={solventL} />
            </li>
          ))}
        </ul>
      ) : (
        <p className="mt-2 text-[var(--ink-soft)]">No composition reported by the server.</p>
      )}
    </aside>
  )
}
