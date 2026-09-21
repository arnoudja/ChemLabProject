import { useEffect, useState, type MouseEvent } from 'react'
import type { LabScene } from '../generated/contracts'
import { fetchLabScene, postLabAction } from '../lib/api'
import {
  StockSubstanceLabel,
  stockSubstanceAriaLabel,
} from '../lib/compositionDisplay'
import { optionalArray } from '../lib/scene'
import {
  SPOON_SCOOP_MASS_G,
  STOCK_FULL_MASS_G,
  STOCK_FULL_SCOOPS,
  WATER_FULL_ML,
  DISTILLED_WATER_CAPACITY_ML,
  DISH_CAPACITY_ML,
  PIPETTE_VOLUME_ML,
  FILTRATE_CAPACITY_ML,
} from '../lib/benchAmounts'
import { BeakerInspectPanel } from './BeakerInspectPanel'
import {
  BurnerSvg,
  DistilledWaterBeakerSvg,
  EvaporationDishSvg,
  FiltrateBeakerSvg,
  FunnelPaperSvg,
  PipetteSvg,
  SolidBeakerSvg,
  SpoonSvg,
  TongsSvg,
  WaterBeakerSvg,
  dishFillRatio,
  distilledWaterFillRatio,
  filtrateFillRatio,
  stockFillRatio,
  waterFillRatio,
  type StockSolid,
} from './LabBenchIcons'
import {
  BURNER_ID,
  CACL2_ID,
  DISH_ID,
  FILTRATE_ID,
  H2O_ID,
  NACL_ID,
  PAPER_ID,
  PIPETTE_ID,
  SAND_ID,
  SPOON_ID,
  TONGS_ID,
  WATER_ID,
  burnerIsOn,
  dishAmountMl,
  dissolveCueFromEvents,
  distilledWaterAmountMl,
  filtrateAmountMl,
  findItem,
  outcomeLabel,
  paperHasResidue,
  pipetteIsFilled,
  solidStockKind,
  spoonHoldingSpecies,
  spoonHoldingSubstance,
  stockAmountG,
  tongsHeldVesselId,
  undissolvedSolidInWater,
  waterAmountMl,
  waterHasAqueous,
} from './labBenchScene'

export {
  SPOON_SCOOP_MASS_G,
  STOCK_FULL_MASS_G,
  STOCK_FULL_SCOOPS,
  WATER_FULL_ML,
  DISTILLED_WATER_CAPACITY_ML,
  DISH_CAPACITY_ML,
  PIPETTE_VOLUME_ML,
  FILTRATE_CAPACITY_ML,
  dishFillRatio,
  distilledWaterFillRatio,
  filtrateFillRatio,
  stockFillRatio,
  waterFillRatio,
}

const BURNER_POLL_MS = 300

type IngredientKind = 'h2o' | StockSolid

const INGREDIENT_CAROUSEL: { itemId: string; kind: IngredientKind }[] = [
  { itemId: H2O_ID, kind: 'h2o' },
  { itemId: NACL_ID, kind: 'nacl' },
  { itemId: CACL2_ID, kind: 'cacl2' },
  { itemId: SAND_ID, kind: 'sand' },
]

const TOOL_CAROUSEL: { itemId: string; kind: 'pipette' | 'spoon' | 'tongs' }[] = [
  { itemId: PIPETTE_ID, kind: 'pipette' },
  { itemId: SPOON_ID, kind: 'spoon' },
  { itemId: TONGS_ID, kind: 'tongs' },
]

type ToolUi = 'none' | 'spoon' | 'pipette' | 'tongs' | StockSolid

export function LabBench() {
  const [scene, setScene] = useState<LabScene | null>(null)
  const [selectedToolItemId, setSelectedToolItemId] = useState<string | null>(null)
  const [inspectItemId, setInspectItemId] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [pointer, setPointer] = useState<{ x: number; y: number } | null>(null)
  const [stockCarouselIndex, setStockCarouselIndex] = useState(0)
  const [toolCarouselIndex, setToolCarouselIndex] = useState(0)

  const burnerOn = scene ? burnerIsOn(scene) : false

  useEffect(() => {
    let cancelled = false
    ;(async () => {
      setError(null)
      try {
        const next = await fetchLabScene()
        if (!cancelled) setScene(next)
      } catch (err) {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : 'Could not load lab scene')
        }
      }
    })()
    return () => {
      cancelled = true
    }
  }, [])

  useEffect(() => {
    if (!burnerOn) return
    let cancelled = false
    const id = window.setInterval(() => {
      void fetchLabScene()
        .then((next) => {
          if (!cancelled) setScene(next)
        })
        .catch(() => {
          /* Poll errors stay quiet so a transient GET failure does not clear the bench. */
        })
    }, BURNER_POLL_MS)
    return () => {
      cancelled = true
      window.clearInterval(id)
    }
  }, [burnerOn])

  function trackPointer(event: MouseEvent<HTMLElement>) {
    const root = event.currentTarget.closest('[data-lab-bench]')
    if (!(root instanceof HTMLElement)) return
    const rect = root.getBoundingClientRect()
    setPointer({ x: event.clientX - rect.left, y: event.clientY - rect.top })
  }

  type BenchToolId = typeof SPOON_ID | typeof PIPETTE_ID | typeof TONGS_ID

  function isBenchToolId(id: string | null): id is BenchToolId {
    return id === SPOON_ID || id === PIPETTE_ID || id === TONGS_ID
  }

  /** True when put-away must hit the server (tool is carrying something). */
  function toolNeedsServerPutAway(toolId: BenchToolId): boolean {
    if (!scene) return false
    switch (toolId) {
      case SPOON_ID:
        return spoonHoldingSubstance(scene) != null
      case PIPETTE_ID:
        return pipetteIsFilled(scene)
      case TONGS_ID:
        return tongsHeldVesselId(scene) != null
    }
  }

  async function putToolAway(toolId: BenchToolId): Promise<boolean> {
    if (!scene) {
      setSelectedToolItemId(null)
      return true
    }
    // Empty tool put-away is a client no-op (no server round-trip).
    if (!toolNeedsServerPutAway(toolId)) {
      setSelectedToolItemId(null)
      return true
    }
    if (busy) return false
    setError(null)
    setBusy(true)
    try {
      const response = await postLabAction({
        type: 'put_away',
        tool_item_id: toolId,
      })
      setScene(response.scene)
      setSelectedToolItemId(null)
      return true
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Action failed')
      return false
    } finally {
      setBusy(false)
    }
  }

  async function selectTool(toolId: BenchToolId, event: MouseEvent<HTMLButtonElement>) {
    trackPointer(event)
    // Keep inspect open across tool pick-up / put-away; only Close dismisses it.
    if (selectedToolItemId === toolId) {
      await putToolAway(toolId)
      return
    }
    if (isBenchToolId(selectedToolItemId)) {
      const putAway = await putToolAway(selectedToolItemId)
      if (!putAway) return
    }
    setSelectedToolItemId(toolId)
  }

  function onSpoon(event: MouseEvent<HTMLButtonElement>) {
    return selectTool(SPOON_ID, event)
  }

  function onPipette(event: MouseEvent<HTMLButtonElement>) {
    return selectTool(PIPETTE_ID, event)
  }

  function onTongs(event: MouseEvent<HTMLButtonElement>) {
    return selectTool(TONGS_ID, event)
  }

  async function applyUseTool(toolItemId: BenchToolId, targetItemId: string) {
    if (busy) return
    setError(null)
    setBusy(true)
    try {
      const response = await postLabAction({
        type: 'use_tool',
        tool_item_id: toolItemId,
        target_item_id: targetItemId,
      })
      setScene(response.scene)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Action failed')
    } finally {
      setBusy(false)
    }
  }

  async function applyPour(sourceItemId: string, targetItemId: string) {
    if (busy) return
    setError(null)
    setBusy(true)
    try {
      const response = await postLabAction({
        type: 'pour',
        source_item_id: sourceItemId,
        target_item_id: targetItemId,
      })
      setScene(response.scene)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Action failed')
    } finally {
      setBusy(false)
    }
  }

  /**
   * Shared vessel-click dispatch: tongs home → pipette → spoon → inspect.
   * Options encode the few vessel-specific exceptions without copying the chain.
   */
  async function onVessel(
    targetItemId: string,
    event: MouseEvent<HTMLButtonElement>,
    opts: {
      allowPipette?: boolean
      allowSpoon?: boolean
      blockMultiSpeciesSpoon?: boolean
      spoonPourIfHolding?: boolean
      tongsRejectHeldId?: string
    } = {},
  ) {
    const {
      allowPipette = true,
      allowSpoon = true,
      blockMultiSpeciesSpoon = false,
      spoonPourIfHolding = false,
      tongsRejectHeldId,
    } = opts

    trackPointer(event)
    if (!scene) return

    if (selectedToolItemId === TONGS_ID) {
      const held = tongsHeldVesselId(scene)
      if (held === targetItemId) {
        await putToolAway(TONGS_ID)
        return
      }
      if (tongsRejectHeldId != null && held === tongsRejectHeldId) {
        return
      }
      await applyUseTool(TONGS_ID, targetItemId)
      return
    }
    if (selectedToolItemId === PIPETTE_ID) {
      if (!allowPipette) return
      await applyUseTool(PIPETTE_ID, targetItemId)
      return
    }
    if (selectedToolItemId === SPOON_ID) {
      if (!allowSpoon) return
      if (blockMultiSpeciesSpoon && spoonHoldingSpecies(scene).length > 1) return
      if (spoonPourIfHolding && spoonHoldingSubstance(scene)) {
        await applyPour(SPOON_ID, targetItemId)
        return
      }
      await applyUseTool(SPOON_ID, targetItemId)
      return
    }
    if (selectedToolItemId === null) {
      setInspectItemId(targetItemId)
    }
  }

  async function onSolid(targetItemId: string, event: MouseEvent<HTMLButtonElement>) {
    return onVessel(targetItemId, event, {
      allowPipette: false,
      blockMultiSpeciesSpoon: true,
    })
  }

  async function onDistilledWater(event: MouseEvent<HTMLButtonElement>) {
    return onVessel(H2O_ID, event, { allowSpoon: false })
  }

  async function onWater(event: MouseEvent<HTMLButtonElement>) {
    return onVessel(WATER_ID, event, { spoonPourIfHolding: true })
  }

  async function onDish(event: MouseEvent<HTMLButtonElement>) {
    return onVessel(DISH_ID, event)
  }

  async function onFilterPaper(event: MouseEvent<HTMLButtonElement>) {
    return onVessel(PAPER_ID, event, {
      allowPipette: false,
      tongsRejectHeldId: FILTRATE_ID,
    })
  }

  async function onFiltrate(event: MouseEvent<HTMLButtonElement>) {
    return onVessel(FILTRATE_ID, event)
  }

  async function onBurner(event: MouseEvent<HTMLButtonElement>) {
    trackPointer(event)
    if (selectedToolItemId !== null) return
    if (busy || !scene) return
    setError(null)
    setBusy(true)
    try {
      const response = await postLabAction({
        type: 'toggle_burner',
        burner_item_id: BURNER_ID,
      })
      setScene(response.scene)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Action failed')
    } finally {
      setBusy(false)
    }
  }

  async function onReset() {
    if (busy) return
    setError(null)
    setBusy(true)
    try {
      const response = await postLabAction({ type: 'reset' })
      setScene(response.scene)
      setSelectedToolItemId(null)
      setStockCarouselIndex(0)
      setToolCarouselIndex(0)
      // Inspect stays open and rebinds to the reset scene item by id.
      setPointer(null)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Reset failed')
    } finally {
      setBusy(false)
    }
  }

  const spoonSelected = selectedToolItemId === SPOON_ID
  const pipetteSelected = selectedToolItemId === PIPETTE_ID
  const tongsSelected = selectedToolItemId === TONGS_ID
  const holdingSelected = spoonSelected || pipetteSelected || tongsSelected
  const spoonFill = scene ? spoonHoldingSubstance(scene) : null
  const leftoverSolid = scene ? undissolvedSolidInWater(scene) : null
  const heldVesselId = scene ? tongsHeldVesselId(scene) : null
  const heldSolid = heldVesselId ? solidStockKind(heldVesselId) : null
  const toolUi: ToolUi = pipetteSelected
    ? 'pipette'
    : tongsSelected
      ? 'tongs'
      : spoonSelected
        ? (spoonFill ?? 'spoon')
        : 'none'
  const water = scene ? findItem(scene, WATER_ID) : undefined
  const spoon = scene ? findItem(scene, SPOON_ID) : undefined
  const pipette = scene ? findItem(scene, PIPETTE_ID) : undefined
  const tongs = scene ? findItem(scene, TONGS_ID) : undefined
  const dish = scene ? findItem(scene, DISH_ID) : undefined
  const burner = scene ? findItem(scene, BURNER_ID) : undefined
  const filtrate = scene ? findItem(scene, FILTRATE_ID) : undefined
  const paper = scene ? findItem(scene, PAPER_ID) : undefined
  const waterHeld = water?.location === 'held'
  const dishHeld = dish?.location === 'held'
  const h2oHeld = scene ? findItem(scene, H2O_ID)?.location === 'held' : false
  const filtrateHeld = filtrate?.location === 'held'
  const paperHeld = paper?.location === 'held'
  const lastEvents = scene ? optionalArray(scene.last_events) : []
  const dissolveCue = dissolveCueFromEvents(lastEvents)
  const hasAqueous = scene ? waterHasAqueous(scene) : false
  const inspectItem = scene && inspectItemId ? findItem(scene, inspectItemId) : undefined
  const pipetteFilled = scene ? pipetteIsFilled(scene) : false
  const visibleIngredient = INGREDIENT_CAROUSEL[stockCarouselIndex] ?? INGREDIENT_CAROUSEL[0]
  const visibleTool = TOOL_CAROUSEL[toolCarouselIndex] ?? TOOL_CAROUSEL[0]

  function stepStockCarousel(delta: number, event: MouseEvent<HTMLButtonElement>) {
    event.stopPropagation()
    setStockCarouselIndex(
      (index) => (index + delta + INGREDIENT_CAROUSEL.length) % INGREDIENT_CAROUSEL.length,
    )
  }

  function stepToolCarousel(delta: number, event: MouseEvent<HTMLButtonElement>) {
    event.stopPropagation()
    setToolCarouselIndex(
      (index) => (index + delta + TOOL_CAROUSEL.length) % TOOL_CAROUSEL.length,
    )
  }

  return (
    <section
      data-lab-bench
      data-tool={toolUi}
      aria-label="Lab bench"
      className={`lab-bench relative w-full overflow-hidden rounded-2xl border border-[var(--border)] bg-[var(--surface)] p-4 shadow-[var(--shadow)] backdrop-blur-md sm:p-5 ${
        holdingSelected ? 'lab-bench--holding' : ''
      }`}
      onMouseMove={holdingSelected ? trackPointer : undefined}
    >
      <div className="mb-3 flex items-start justify-between gap-3">
        <p className="text-sm text-[var(--ink-soft)]">
          Pick up the spoon to scoop dry solids, the pipette to move {PIPETTE_VOLUME_ML.toFixed(2)} ml
          of solution, or the tongs to lift vessels, filter paper, or solid ingredients and pour.
          With no tool selected, click a vessel to inspect it, or the burner to heat the dish.
        </p>
        <button
          type="button"
          className="lab-bench-reset shrink-0 rounded-md border border-[var(--border)] px-2.5 py-1 text-xs font-medium text-[var(--ink-soft)] hover:bg-[var(--surface-hover)] disabled:opacity-50"
          aria-label="Reset lab"
          disabled={busy || !scene}
          onClick={onReset}
        >
          Reset
        </button>
      </div>

      {!scene && !error ? (
        <p className="text-sm text-[var(--ink-soft)]">Loading lab scene…</p>
      ) : null}

      {scene ? (
        <div className="lab-bench-surface flex flex-wrap items-end justify-center gap-6 rounded-xl px-4 pb-4 pt-8 sm:gap-10">
          <div className="lab-filter-stack">
            <button
              type="button"
              className="lab-item"
              aria-label="Filter paper"
              disabled={busy}
              onClick={onFilterPaper}
            >
              {paperHeld ? (
                <svg viewBox="0 0 120 88" className="h-20 w-28" aria-hidden />
              ) : (
                <FunnelPaperSvg residue={scene ? paperHasResidue(scene) : false} />
              )}
              <span className="lab-item-label">{paper?.label ?? 'Filter paper'}</span>
            </button>
            <button
              type="button"
              className="lab-item"
              aria-label="Filtrate beaker"
              disabled={busy}
              onClick={onFiltrate}
            >
              {filtrateHeld ? (
                <svg viewBox="0 0 120 168" className="h-32 w-20" aria-hidden />
              ) : (
                <FiltrateBeakerSvg amountMl={scene ? filtrateAmountMl(scene) : null} />
              )}
              <span className="lab-item-label">{filtrate?.label ?? 'Filtrate'}</span>
            </button>
          </div>

          {dish || burner ? (
            <div className="lab-evap-stack">
              {dish ? (
                <button
                  type="button"
                  className="lab-item"
                  aria-label="Evaporation dish"
                  disabled={busy}
                  onClick={onDish}
                >
                  {dishHeld ? (
                    <svg viewBox="0 0 120 56" className="h-14 w-28" aria-hidden />
                  ) : (
                    <EvaporationDishSvg amountMl={dishAmountMl(scene)} />
                  )}
                  <span className="lab-item-label">{dish.label}</span>
                </button>
              ) : null}
              {burner ? (
                <button
                  type="button"
                  className="lab-item"
                  aria-label="Burner"
                  aria-pressed={burnerOn}
                  disabled={busy}
                  onClick={onBurner}
                >
                  <BurnerSvg on={burnerOn} />
                  <span className="lab-item-label">{burner.label}</span>
                </button>
              ) : null}
            </div>
          ) : null}

          <button
            type="button"
            className="lab-item"
            aria-label={water?.label ?? 'Beaker'}
            disabled={busy}
            onClick={onWater}
          >
            {waterHeld ? (
              <svg viewBox="0 0 120 168" className="h-40 w-28" aria-hidden />
            ) : (
              <WaterBeakerSvg
                leftoverSolid={leftoverSolid}
                busy={busy}
                amountMl={waterAmountMl(scene)}
                hasAqueous={hasAqueous}
                dissolveCue={dissolveCue}
              />
            )}
            <span className="lab-item-label">{water?.label ?? 'Beaker'}</span>
          </button>

          <div className="lab-stock-carousel">
            <button
              type="button"
              className="lab-stock-carousel-arrow"
              aria-label="Previous ingredient"
              onClick={(event) => stepStockCarousel(-1, event)}
            >
              ‹
            </button>
            <div className="lab-stock-carousel-slot">
              {visibleIngredient.kind === 'h2o' ? (
                <button
                  type="button"
                  className="lab-item"
                  aria-label={stockSubstanceAriaLabel('water')}
                  disabled={busy}
                  onClick={onDistilledWater}
                >
                  {h2oHeld ? (
                    <svg viewBox="0 0 80 118" className="h-28 w-20" aria-hidden />
                  ) : (
                    <DistilledWaterBeakerSvg amountMl={scene ? distilledWaterAmountMl(scene) : null} />
                  )}
                  <StockSubstanceLabel substanceId="water" />
                </button>
              ) : (
                <button
                  type="button"
                  className="lab-item"
                  aria-label={stockSubstanceAriaLabel(visibleIngredient.kind)}
                  disabled={busy}
                  onClick={(event) => onSolid(visibleIngredient.itemId, event)}
                >
                  {heldVesselId === visibleIngredient.itemId ? (
                    <svg viewBox="0 0 80 118" className="h-28 w-20" aria-hidden />
                  ) : (
                    <SolidBeakerSvg
                      solid={visibleIngredient.kind}
                      amountG={stockAmountG(scene, visibleIngredient.itemId, visibleIngredient.kind)}
                    />
                  )}
                  <StockSubstanceLabel substanceId={visibleIngredient.kind} />
                </button>
              )}
            </div>
            <button
              type="button"
              className="lab-stock-carousel-arrow"
              aria-label="Next ingredient"
              onClick={(event) => stepStockCarousel(1, event)}
            >
              ›
            </button>
          </div>

          <div className="lab-tool-carousel">
            <button
              type="button"
              className="lab-tool-carousel-arrow"
              aria-label="Previous tool"
              onClick={(event) => stepToolCarousel(-1, event)}
            >
              ‹
            </button>
            <div className="lab-tool-carousel-slot">
              {visibleTool.kind === 'pipette' ? (
                <button
                  type="button"
                  className={`lab-item ${pipetteSelected ? 'opacity-40' : ''}`}
                  aria-label="Pipette"
                  aria-pressed={pipetteSelected}
                  disabled={busy}
                  onClick={onPipette}
                >
                  <PipetteSvg filled={pipetteFilled} />
                  <span className="lab-item-label">{pipette?.label ?? 'Pipette'}</span>
                </button>
              ) : visibleTool.kind === 'spoon' ? (
                <button
                  type="button"
                  className={`lab-item ${spoonSelected ? 'opacity-40' : ''}`}
                  aria-label="Spoon"
                  aria-pressed={spoonSelected}
                  disabled={busy}
                  onClick={onSpoon}
                >
                  <SpoonSvg fill={null} />
                  <span className="lab-item-label">{spoon?.label ?? 'Spoon'}</span>
                </button>
              ) : (
                <button
                  type="button"
                  className={`lab-item ${tongsSelected ? 'opacity-40' : ''}`}
                  aria-label="Tongs"
                  aria-pressed={tongsSelected}
                  disabled={busy}
                  onClick={onTongs}
                >
                  <TongsSvg />
                  <span className="lab-item-label">{tongs?.label ?? 'Tongs'}</span>
                </button>
              )}
            </div>
            <button
              type="button"
              className="lab-tool-carousel-arrow"
              aria-label="Next tool"
              onClick={(event) => stepToolCarousel(1, event)}
            >
              ›
            </button>
          </div>
        </div>
      ) : null}

      {holdingSelected && pointer ? (
        <div
          className={`pointer-events-none absolute z-20 ${
            pipetteSelected
              ? 'lab-cursor-pipette'
              : tongsSelected
                ? heldVesselId
                  ? 'lab-cursor-vessel'
                  : 'lab-cursor-tongs'
                : 'lab-cursor-spoon'
          }`}
          style={{ left: pointer.x, top: pointer.y }}
          aria-hidden
        >
          {pipetteSelected ? (
            <PipetteSvg filled={pipetteFilled} floating />
          ) : tongsSelected ? (
            heldVesselId === WATER_ID && scene ? (
              <WaterBeakerSvg
                leftoverSolid={leftoverSolid}
                busy={busy}
                amountMl={waterAmountMl(scene)}
                hasAqueous={hasAqueous}
                dissolveCue={dissolveCue}
                floating
              />
            ) : heldVesselId === H2O_ID && scene ? (
              <DistilledWaterBeakerSvg amountMl={distilledWaterAmountMl(scene)} floating />
            ) : heldSolid && heldVesselId && scene ? (
              <SolidBeakerSvg
                solid={heldSolid}
                amountG={stockAmountG(scene, heldVesselId, heldSolid)}
                floating
              />
            ) : heldVesselId === DISH_ID && scene ? (
              <EvaporationDishSvg amountMl={dishAmountMl(scene)} floating />
            ) : heldVesselId === FILTRATE_ID && scene ? (
              <FiltrateBeakerSvg amountMl={filtrateAmountMl(scene)} floating />
            ) : heldVesselId === PAPER_ID && scene ? (
              <FunnelPaperSvg residue={paperHasResidue(scene)} floating />
            ) : (
              <TongsSvg floating />
            )
          ) : (
            <SpoonSvg fill={spoonFill} floating />
          )}
        </div>
      ) : null}

      {scene && inspectItem ? (
        <BeakerInspectPanel
          scene={scene}
          item={inspectItem}
          onClose={() => setInspectItemId(null)}
        />
      ) : null}

      {error ? (
        <p className="mt-3 text-sm text-[var(--danger)]" role="alert">
          {error}
        </p>
      ) : null}

      {lastEvents.length > 0 ? (
        <div className="lab-bench-events mt-3 space-y-2 text-sm" role="status" aria-live="polite">
          {dissolveCue ? (
            <p
              className="lab-bench-status text-[var(--ink)]"
              data-bench-status={dissolveCue}
            >
              <span className="font-medium text-[var(--hackerman-bright-aqua)]">
                {outcomeLabel(dissolveCue)}
              </span>
              {': '}
              <span className="text-[var(--ink-soft)]">
                {dissolveCue === 'dissolved'
                  ? lastEvents.find((event) => event.kind === 'dissolved')?.message
                  : lastEvents.find((event) => event.kind === 'did_not_dissolve')?.message}
              </span>
            </p>
          ) : (
            lastEvents.map((event, index) => {
              const outcome = outcomeLabel(event.kind)
              return (
                <div key={`${event.kind}-${index}`} className="space-y-1" data-event-kind={event.kind}>
                  {outcome ? (
                    <p className="font-medium text-[var(--ink)]">{outcome}</p>
                  ) : null}
                  <p className="text-[var(--ink-soft)]">{event.message}</p>
                </div>
              )
            })
          )}
        </div>
      ) : null}
    </section>
  )
}
