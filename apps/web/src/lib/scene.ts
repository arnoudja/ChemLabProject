import type { Item, LabScene } from '../generated/contracts'

/** Wire JSON may omit empty Vec fields (`skip_serializing_if = Vec::is_empty`). */
export function optionalArray<T>(value: T[] | null | undefined): T[] {
  return value ?? []
}

/** Fill omitted optional scene arrays so renderers can index safely. */
export function normalizeLabScene(scene: LabScene): LabScene {
  return {
    ...scene,
    last_events: optionalArray(scene.last_events),
    items: scene.items.map(
      (item): Item => ({
        ...item,
        properties: {
          ...item.properties,
          composition: optionalArray(item.properties.composition),
          holding: optionalArray(item.properties.holding),
        },
      }),
    ),
  }
}
