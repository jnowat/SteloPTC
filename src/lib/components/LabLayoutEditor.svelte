<script lang="ts">
  /**
   * Draw a room: drop furniture on a grid, then say how many shelves each piece
   * has and how many trays sit on a shelf.
   *
   * The interaction is deliberately borrowed from tile-map editors rather than
   * CAD. Pick a stamp from the palette, click to drop it, drag to move it, drag
   * a corner to resize, `R` to rotate, `Delete` to remove, `Ctrl+Z` to undo.
   * There are no modal dialogs and nothing to confirm — every action is one
   * gesture and every action is undoable, which is what makes a drawing tool
   * feel like a toy instead of a form.
   *
   * The second half is the part a generic floor planner does not have: the
   * elevation. Selecting a rack shows its shelves stacked, each as a small grid
   * of trays, each tray labelled with the address a specimen would record if it
   * lived there. That is the whole point of drawing the room — the plan and the
   * records end up speaking the same language.
   *
   * SVG rather than canvas: hit-testing, focus, keyboard handling, and dark-mode
   * theming all come free, and a lab plan is tens of rectangles, not thousands.
   */
  import { untrack } from 'svelte';
  import {
    FURNITURE_SPECS,
    specFor,
    createItem,
    normalizeItem,
    rotateItem,
    findOverlaps,
    capacityOf,
    totalCapacity,
    slotAddress,
    tierAddress,
    positionLabel,
    occupancyByItem,
    occupancyBySlot,
    parseLayout,
    serializeLayout,
    emptyLayout,
    MIN_GRID,
    MAX_GRID,
    MAX_TIERS,
    MAX_ROWS,
    MAX_COLS,
    type LabLayout,
    type FurnitureItem,
    type FurnitureKind,
  } from '../labLayout';
  import type { LocationOccupancy } from '../api';

  let {
    initialJson = null,
    roomName = '',
    occupancy = [],
    canEdit = true,
    onsave,
  }: {
    initialJson?: string | null;
    roomName?: string;
    occupancy?: LocationOccupancy[];
    canEdit?: boolean;
    onsave?: (json: string) => void;
  } = $props();

  const CELL = 30; // px per grid cell

  // Read once, on purpose. The editor owns the layout from here on, and
  // re-deriving it from the prop would throw away unsaved edits every time the
  // parent refetched. The parent keys this component by location id, so
  // switching rooms creates a fresh instance rather than mutating this one.
  let layout = $state<LabLayout>(untrack(() => parseLayout(initialJson)) ?? emptyLayout());
  let selectedId = $state<string | null>(null);
  let armedKind = $state<FurnitureKind | null>(null);
  let dirty = $state(false);

  // Undo/redo. Snapshots of the serialised layout — a lab plan is a few
  // kilobytes, so keeping 50 of them costs less than the code to diff them.
  let past = $state<string[]>([]);
  let future = $state<string[]>([]);
  const UNDO_LIMIT = 50;

  // Drag state lives outside $state: it changes on every pointermove, and
  // re-rendering the palette sixty times a second to move one rectangle is
  // wasted work. Only the layout it produces is reactive.
  type Drag =
    | { mode: 'move'; id: string; grabDx: number; grabDy: number; before: string }
    | { mode: 'resize'; id: string; originX: number; originY: number; before: string }
    | null;
  let drag: Drag = null;
  let svgEl: SVGSVGElement | undefined = $state();

  const selected = $derived(layout.items.find((i) => i.id === selectedId) ?? null);
  const overlaps = $derived(findOverlaps(layout));
  const capacity = $derived(totalCapacity(layout));

  const itemOccupancy = $derived(
    occupancyByItem(
      layout,
      occupancy.map((o) => ({ location: o.location, count: o.specimen_count })),
    ),
  );
  const slotOccupancy = $derived(
    occupancyBySlot(occupancy.map((o) => ({ location: o.location, count: o.specimen_count }))),
  );

  // ── History ───────────────────────────────────────────────────────────────

  function commit(next: LabLayout) {
    past = [...past.slice(-(UNDO_LIMIT - 1)), serializeLayout(layout)];
    future = [];
    layout = next;
    dirty = true;
  }

  function undo() {
    const prev = past[past.length - 1];
    if (prev === undefined) return;
    future = [serializeLayout(layout), ...future];
    past = past.slice(0, -1);
    layout = parseLayout(prev) ?? emptyLayout();
    dirty = true;
    if (selectedId && !layout.items.some((i) => i.id === selectedId)) selectedId = null;
  }

  function redo() {
    const next = future[0];
    if (next === undefined) return;
    past = [...past, serializeLayout(layout)];
    future = future.slice(1);
    layout = parseLayout(next) ?? emptyLayout();
    dirty = true;
    if (selectedId && !layout.items.some((i) => i.id === selectedId)) selectedId = null;
  }

  // ── Mutations ─────────────────────────────────────────────────────────────

  function replaceItem(updated: FurnitureItem) {
    commit({ ...layout, items: layout.items.map((i) => (i.id === updated.id ? updated : i)) });
  }

  /// Apply a live drag without pushing history — otherwise one drag across the
  /// room would fill the undo stack with a hundred intermediate positions.
  function moveItemLive(updated: FurnitureItem) {
    layout = { ...layout, items: layout.items.map((i) => (i.id === updated.id ? updated : i)) };
  }

  function addAt(kind: FurnitureKind, x: number, y: number) {
    const id = crypto.randomUUID();
    const item = createItem(layout, kind, x, y, id);
    commit({ ...layout, items: [...layout.items, item] });
    selectedId = id;
  }

  function removeSelected() {
    if (!selected) return;
    commit({ ...layout, items: layout.items.filter((i) => i.id !== selected.id) });
    selectedId = null;
  }

  function duplicateSelected() {
    if (!selected) return;
    const id = crypto.randomUUID();
    const copy = normalizeItem(
      { ...selected, id, x: selected.x + 1, y: selected.y + 1, label: `${selected.label} copy` },
      layout,
    );
    commit({ ...layout, items: [...layout.items, copy] });
    selectedId = id;
  }

  function rotateSelected() {
    if (!selected) return;
    replaceItem(rotateItem(selected, layout));
  }

  function nudge(dx: number, dy: number) {
    if (!selected) return;
    replaceItem(normalizeItem({ ...selected, x: selected.x + dx, y: selected.y + dy }, layout));
  }

  function resizeGrid(cols: number, rows: number) {
    const next: LabLayout = {
      ...layout,
      gridCols: Math.min(MAX_GRID, Math.max(MIN_GRID, cols)),
      gridRows: Math.min(MAX_GRID, Math.max(MIN_GRID, rows)),
      items: layout.items,
    };
    // Shrinking the room must pull the furniture back inside it, or items would
    // sit outside the drawing with no way to select them.
    commit({ ...next, items: next.items.map((i) => normalizeItem(i, next)) });
  }

  function updateSelectedField<K extends keyof FurnitureItem>(field: K, value: FurnitureItem[K]) {
    if (!selected) return;
    replaceItem(normalizeItem({ ...selected, [field]: value }, layout));
  }

  // ── Pointer handling ──────────────────────────────────────────────────────

  function cellFromEvent(e: PointerEvent | MouseEvent): { x: number; y: number } | null {
    if (!svgEl) return null;
    const rect = svgEl.getBoundingClientRect();
    // The SVG is scaled to fit its container, so client px must be converted
    // through the rendered size rather than assuming 1 px = 1 px.
    const scaleX = layout.gridCols * CELL / rect.width;
    const scaleY = layout.gridRows * CELL / rect.height;
    return {
      x: Math.floor(((e.clientX - rect.left) * scaleX) / CELL),
      y: Math.floor(((e.clientY - rect.top) * scaleY) / CELL),
    };
  }

  function handleCanvasPointerDown(e: PointerEvent) {
    if (!canEdit) return;
    const cell = cellFromEvent(e);
    if (!cell) return;
    if (armedKind) {
      addAt(armedKind, cell.x, cell.y);
      // Stay armed so a run of racks is one click each — the single most common
      // thing anyone does when drawing a growth room.
      return;
    }
    selectedId = null;
  }

  function handleItemPointerDown(e: PointerEvent, item: FurnitureItem) {
    e.stopPropagation();
    selectedId = item.id;
    if (!canEdit || armedKind) return;
    const cell = cellFromEvent(e);
    if (!cell) return;
    // Snapshot now, but only push it on pointerup *if the drag changed
    // something*. Pushing on pointerdown would fill the undo stack with a no-op
    // entry every time someone clicks an item to select it.
    drag = {
      mode: 'move',
      id: item.id,
      grabDx: cell.x - item.x,
      grabDy: cell.y - item.y,
      before: serializeLayout(layout),
    };
    (e.currentTarget as Element).setPointerCapture?.(e.pointerId);
  }

  function handleHandlePointerDown(e: PointerEvent, item: FurnitureItem) {
    e.stopPropagation();
    if (!canEdit) return;
    selectedId = item.id;
    drag = {
      mode: 'resize',
      id: item.id,
      originX: item.x,
      originY: item.y,
      before: serializeLayout(layout),
    };
    (e.currentTarget as Element).setPointerCapture?.(e.pointerId);
  }

  function handlePointerMove(e: PointerEvent) {
    if (!drag) return;
    const cell = cellFromEvent(e);
    if (!cell) return;
    const item = layout.items.find((i) => i.id === drag!.id);
    if (!item) return;

    if (drag.mode === 'move') {
      moveItemLive(normalizeItem({ ...item, x: cell.x - drag.grabDx, y: cell.y - drag.grabDy }, layout));
    } else {
      moveItemLive(
        normalizeItem(
          { ...item, w: cell.x - drag.originX + 1, h: cell.y - drag.originY + 1 },
          layout,
        ),
      );
    }
  }

  function handlePointerUp() {
    if (!drag) return;
    const { before } = drag;
    drag = null;
    // A click that selected an item without moving it is not an edit: it must
    // not become an undo step, and it must not mark the plan unsaved.
    if (serializeLayout(layout) === before) return;
    past = [...past.slice(-(UNDO_LIMIT - 1)), before];
    future = [];
    dirty = true;
  }

  // ── Keyboard ──────────────────────────────────────────────────────────────

  function handleKeydown(e: KeyboardEvent) {
    // Never steal keys from a field the operator is typing in.
    const target = e.target as HTMLElement | null;
    if (target && ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName)) return;
    if (!canEdit) return;

    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'z') {
      e.preventDefault();
      if (e.shiftKey) redo(); else undo();
      return;
    }
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'y') {
      e.preventDefault();
      redo();
      return;
    }
    if (e.key === 'Escape') {
      armedKind = null;
      selectedId = null;
      return;
    }
    if (!selected) return;

    switch (e.key) {
      case 'Delete':
      case 'Backspace':
        e.preventDefault();
        removeSelected();
        break;
      case 'r':
      case 'R':
        e.preventDefault();
        rotateSelected();
        break;
      case 'd':
      case 'D':
        if (e.ctrlKey || e.metaKey) {
          e.preventDefault();
          duplicateSelected();
        }
        break;
      case 'ArrowLeft': e.preventDefault(); nudge(-1, 0); break;
      case 'ArrowRight': e.preventDefault(); nudge(1, 0); break;
      case 'ArrowUp': e.preventDefault(); nudge(0, -1); break;
      case 'ArrowDown': e.preventDefault(); nudge(0, 1); break;
    }
  }

  // ── Occupancy shading ─────────────────────────────────────────────────────

  function fillFor(item: FurnitureItem): string {
    const spec = specFor(item.kind);
    const cap = capacityOf(item);
    if (cap === 0) return spec.color;
    const used = itemOccupancy.get(item.id) ?? 0;
    if (used === 0) return spec.color;
    // Blend the palette colour toward a warm "getting full" tone. Full racks
    // should be findable at a glance without reading a single number.
    const ratio = Math.min(1, used / cap);
    return `color-mix(in srgb, ${spec.color} ${Math.round((1 - ratio) * 100)}%, #ef4444)`;
  }

  function occupancyLabel(item: FurnitureItem): string {
    const cap = capacityOf(item);
    const used = itemOccupancy.get(item.id) ?? 0;
    if (cap === 0) return used > 0 ? `${used} here` : '';
    return `${used}/${cap}`;
  }

  function slotCount(item: FurnitureItem, tier: number, row: number, col: number): number {
    return slotOccupancy.get(slotAddress(roomName, item, tier, row, col).toLowerCase()) ?? 0;
  }

  // ── Save ──────────────────────────────────────────────────────────────────

  function save() {
    onsave?.(serializeLayout(layout));
    dirty = false;
  }

  function clearAll() {
    if (layout.items.length === 0) return;
    if (!confirm(`Remove all ${layout.items.length} items from this plan?`)) return;
    commit({ ...layout, items: [] });
    selectedId = null;
  }
</script>

<svelte:window onkeydown={handleKeydown} onpointerup={handlePointerUp} onpointermove={handlePointerMove} />

<div class="editor">
  <!-- ── Palette ──────────────────────────────────────────────────────── -->
  {#if canEdit}
    <div class="palette" role="toolbar" aria-label="Furniture palette">
      <span class="palette-label">Drop a piece:</span>
      {#each FURNITURE_SPECS as spec}
        <button
          class="stamp"
          class:armed={armedKind === spec.kind}
          style="--stamp-color: {spec.color};"
          onclick={() => (armedKind = armedKind === spec.kind ? null : spec.kind)}
          aria-pressed={armedKind === spec.kind}
          title="{spec.hint} — {spec.w}×{spec.h} cells{spec.tiers > 0 ? `, ${spec.tiers} shelves of ${spec.rows}×${spec.cols}` : ', no storage'}. Click to arm, then click the plan."
        >
          <span class="stamp-glyph" aria-hidden="true">{spec.glyph}</span>
          <span class="stamp-label">{spec.label}</span>
        </button>
      {/each}
    </div>

    <div class="hint-bar">
      {#if armedKind}
        <strong>{specFor(armedKind).label} armed</strong> — click the plan to drop one, Esc to stop.
      {:else if selected}
        <strong>{selected.label}</strong> selected — drag to move, corner to resize,
        <kbd>R</kbd> rotate, <kbd>Del</kbd> remove, arrows nudge.
      {:else}
        Pick a piece above, then click the plan. <kbd>Ctrl</kbd>+<kbd>Z</kbd> undoes anything.
      {/if}
    </div>
  {/if}

  <div class="workspace">
    <!-- ── Plan ───────────────────────────────────────────────────────── -->
    <div class="plan-pane">
      <div class="plan-toolbar">
        <div class="grid-size">
          <label for="grid-cols" title="Width of the room in grid cells">Room width</label>
          <input
            id="grid-cols"
            type="number"
            min={MIN_GRID}
            max={MAX_GRID}
            value={layout.gridCols}
            disabled={!canEdit}
            onchange={(e) => resizeGrid(parseInt((e.currentTarget as HTMLInputElement).value) || layout.gridCols, layout.gridRows)}
            title="How many grid cells wide this room is"
          />
          <label for="grid-rows" title="Depth of the room in grid cells">depth</label>
          <input
            id="grid-rows"
            type="number"
            min={MIN_GRID}
            max={MAX_GRID}
            value={layout.gridRows}
            disabled={!canEdit}
            onchange={(e) => resizeGrid(layout.gridCols, parseInt((e.currentTarget as HTMLInputElement).value) || layout.gridRows)}
            title="How many grid cells deep this room is"
          />
        </div>
        {#if canEdit}
          <div class="plan-actions">
            <button class="btn btn-sm" onclick={undo} disabled={past.length === 0} title="Undo the last change (Ctrl+Z)">↶ Undo</button>
            <button class="btn btn-sm" onclick={redo} disabled={future.length === 0} title="Redo (Ctrl+Shift+Z)">↷ Redo</button>
            <button class="btn btn-sm" onclick={clearAll} disabled={layout.items.length === 0} title="Remove every piece from this plan">Clear</button>
            <button class="btn btn-primary btn-sm" onclick={save} disabled={!dirty} title="Save this floor plan to the location">
              {dirty ? 'Save plan' : 'Saved'}
            </button>
          </div>
        {/if}
      </div>

      <div class="plan-scroll">
        <svg
          bind:this={svgEl}
          class="plan"
          class:arming={!!armedKind}
          viewBox="0 0 {layout.gridCols * CELL} {layout.gridRows * CELL}"
          width={layout.gridCols * CELL}
          height={layout.gridRows * CELL}
          role="application"
          aria-label="Room floor plan — {layout.items.length} items"
          onpointerdown={handleCanvasPointerDown}
        >
          <defs>
            <pattern id="labgrid" width={CELL} height={CELL} patternUnits="userSpaceOnUse">
              <path d="M {CELL} 0 L 0 0 0 {CELL}" fill="none" stroke="currentColor" stroke-width="1" opacity="0.18" />
            </pattern>
          </defs>
          <rect width="100%" height="100%" fill="url(#labgrid)" class="grid-bg" />

          {#each layout.items as item (item.id)}
            {@const spec = specFor(item.kind)}
            <g
              class="item"
              class:selected={selectedId === item.id}
              class:overlapping={overlaps.has(item.id)}
              role="button"
              tabindex="0"
              aria-label="{item.label}, {spec.label}, {capacityOf(item)} positions"
              onpointerdown={(e) => handleItemPointerDown(e, item)}
              onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); selectedId = item.id; } }}
            >
              <title>{item.label} — {spec.label}{capacityOf(item) > 0 ? `, ${occupancyLabel(item)} positions used` : ''}</title>
              <rect
                x={item.x * CELL + 1}
                y={item.y * CELL + 1}
                width={item.w * CELL - 2}
                height={item.h * CELL - 2}
                rx="4"
                fill={fillFor(item)}
                class="item-rect"
              />
              <text
                x={item.x * CELL + item.w * CELL / 2}
                y={item.y * CELL + item.h * CELL / 2 - 2}
                text-anchor="middle"
                class="item-glyph"
              >{spec.glyph}</text>
              <text
                x={item.x * CELL + item.w * CELL / 2}
                y={item.y * CELL + item.h * CELL / 2 + 11}
                text-anchor="middle"
                class="item-label"
              >{item.label}</text>
              {#if capacityOf(item) > 0}
                <text
                  x={item.x * CELL + item.w * CELL - 4}
                  y={item.y * CELL + 12}
                  text-anchor="end"
                  class="item-count"
                >{occupancyLabel(item)}</text>
              {/if}
              {#if selectedId === item.id && canEdit}
                <rect
                  class="resize-handle"
                  x={(item.x + item.w) * CELL - 9}
                  y={(item.y + item.h) * CELL - 9}
                  width="8"
                  height="8"
                  onpointerdown={(e) => handleHandlePointerDown(e, item)}
                  role="presentation"
                />
              {/if}
            </g>
          {/each}
        </svg>
      </div>

      <div class="plan-footer">
        <span>{layout.items.length} item{layout.items.length === 1 ? '' : 's'} · <strong>{capacity}</strong> addressable position{capacity === 1 ? '' : 's'}</span>
        {#if overlaps.size > 0}
          <span class="overlap-warn" title="Overlapping pieces are allowed — this is just a heads-up in case it was a slip">
            ⚠ {overlaps.size} piece{overlaps.size === 1 ? '' : 's'} overlap
          </span>
        {/if}
      </div>
    </div>

    <!-- ── Inspector + elevation ──────────────────────────────────────── -->
    <div class="inspector">
      {#if !selected}
        <div class="inspector-empty">
          <p><strong>Nothing selected.</strong></p>
          <p>
            Click a piece on the plan to rename it and set how many shelves it has and how many
            trays sit on each shelf. Those numbers are what generate the storage addresses the
            Add Specimen form offers.
          </p>
        </div>
      {:else}
        {@const spec = specFor(selected.kind)}
        <div class="inspector-head">
          <span class="inspector-glyph" style="color: {spec.color};" aria-hidden="true">{spec.glyph}</span>
          <div>
            <div class="inspector-title">{selected.label}</div>
            <div class="inspector-sub">{spec.label} · {selected.w}×{selected.h} cells</div>
          </div>
        </div>

        <div class="form-group">
          <label for="item-label" title="The name that appears in every address this piece generates">Label</label>
          <input
            id="item-label"
            type="text"
            value={selected.label}
            disabled={!canEdit}
            oninput={(e) => updateSelectedField('label', (e.currentTarget as HTMLInputElement).value)}
            title="Rename this piece — the name becomes part of each storage address"
          />
        </div>

        {#if spec.tiers > 0 || selected.tiers > 0}
          <div class="form-row-3">
            <div class="form-group">
              <label for="item-tiers" title="How many shelves or levels this piece has">Shelves</label>
              <input id="item-tiers" type="number" min="0" max={MAX_TIERS} value={selected.tiers} disabled={!canEdit}
                onchange={(e) => updateSelectedField('tiers', parseInt((e.currentTarget as HTMLInputElement).value) || 0)}
                title="Number of shelves — a five-shelf rack is one footprint but five levels" />
            </div>
            <div class="form-group">
              <label for="item-rows" title="Tray rows on each shelf — lettered A, B, C…">Rows</label>
              <input id="item-rows" type="number" min="0" max={MAX_ROWS} value={selected.rows} disabled={!canEdit}
                onchange={(e) => updateSelectedField('rows', parseInt((e.currentTarget as HTMLInputElement).value) || 0)}
                title="Tray rows per shelf, lettered A onwards" />
            </div>
            <div class="form-group">
              <label for="item-cols" title="Tray columns on each shelf — numbered 1, 2, 3…">Columns</label>
              <input id="item-cols" type="number" min="0" max={MAX_COLS} value={selected.cols} disabled={!canEdit}
                onchange={(e) => updateSelectedField('cols', parseInt((e.currentTarget as HTMLInputElement).value) || 0)}
                title="Tray columns per shelf, numbered from 1" />
            </div>
          </div>
          <p class="capacity-line">
            {capacityOf(selected)} position{capacityOf(selected) === 1 ? '' : 's'}
            {#if capacityOf(selected) > 0}
              — first is <code>{slotAddress(roomName, selected, 0, 0, 0)}</code>
            {/if}
          </p>
        {/if}

        {#if canEdit}
          <div class="inspector-actions">
            <button class="btn btn-sm" onclick={rotateSelected} title="Swap width and height (R)">Rotate</button>
            <button class="btn btn-sm" onclick={duplicateSelected} title="Place a copy beside this one (Ctrl+D)">Duplicate</button>
            <button class="btn btn-sm btn-danger" onclick={removeSelected} title="Remove this piece (Delete)">Remove</button>
          </div>
        {/if}

        <!-- Elevation: the reason a floor grid alone is not enough -->
        {#if capacityOf(selected) > 0}
          <div class="elevation">
            <h4 title="Each shelf of this piece, with the trays on it. Shaded trays already hold specimens.">
              Shelves — top to bottom
            </h4>
            {#each Array.from({ length: selected.tiers }, (_, i) => selected.tiers - 1 - i) as tier}
              <div class="tier">
                <div class="tier-head" title={tierAddress(roomName, selected, tier)}>Shelf {tier + 1}</div>
                <div class="tier-grid" style="grid-template-columns: repeat({selected.cols}, minmax(0, 1fr));">
                  {#each Array.from({ length: selected.rows * selected.cols }, (_, n) => n) as n}
                    {@const row = Math.floor(n / selected.cols)}
                    {@const col = n % selected.cols}
                    {@const used = slotCount(selected, tier, row, col)}
                    <div
                      class="slot"
                      class:filled={used > 0}
                      title="{slotAddress(roomName, selected, tier, row, col)}{used > 0 ? ` — ${used} specimen${used === 1 ? '' : 's'}` : ' — empty'}"
                    >
                      <span class="slot-label">{positionLabel(row, col)}</span>
                      {#if used > 0}<span class="slot-count">{used}</span>{/if}
                    </div>
                  {/each}
                </div>
              </div>
            {/each}
          </div>
        {/if}
      {/if}
    </div>
  </div>
</div>

<style>
  .editor { display: flex; flex-direction: column; gap: 10px; }

  /* ── Palette ─────────────────────────────────────────────────────────── */
  .palette {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
    padding: 8px;
    border: 1px solid var(--color-border, #e2e8f0);
    border-radius: var(--radius-md, 8px);
    background: var(--color-surface-raised, #f8fafc);
  }
  :global(.dark) .palette { background: #0f172a; border-color: #334155; }
  .palette-label {
    font-size: 12px;
    font-weight: 600;
    color: var(--color-text-muted, #6b7280);
    margin-right: 2px;
  }
  .stamp {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 5px 9px;
    border: 1px solid var(--color-border, #d1d5db);
    border-left: 4px solid var(--stamp-color);
    border-radius: 6px;
    background: var(--color-surface, #fff);
    color: inherit;
    font-size: 12px;
    cursor: pointer;
    transition: transform 0.08s, box-shadow 0.08s;
  }
  :global(.dark) .stamp { background: #1e293b; border-color: #334155; border-left-color: var(--stamp-color); }
  .stamp:hover { transform: translateY(-1px); box-shadow: 0 2px 6px rgba(0,0,0,0.12); }
  .stamp.armed {
    background: var(--stamp-color);
    color: #0f172a;
    font-weight: 600;
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--stamp-color) 35%, transparent);
  }
  .stamp-glyph { font-size: 14px; line-height: 1; }
  .stamp-label { white-space: nowrap; }

  .hint-bar {
    font-size: 12px;
    color: var(--color-text-muted, #6b7280);
    padding: 0 2px;
    line-height: 1.6;
  }
  .hint-bar kbd {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 11px;
    padding: 1px 5px;
    border: 1px solid var(--color-border, #cbd5e1);
    border-bottom-width: 2px;
    border-radius: 4px;
  }

  /* ── Workspace ───────────────────────────────────────────────────────── */
  .workspace {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 320px;
    gap: 14px;
    align-items: start;
  }
  @media (max-width: 1024px) {
    .workspace { grid-template-columns: 1fr; }
  }

  .plan-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    flex-wrap: wrap;
    margin-bottom: 8px;
  }
  .grid-size { display: flex; align-items: center; gap: 6px; }
  .grid-size label { margin: 0; white-space: nowrap; }
  .grid-size input { width: 72px; }
  .plan-actions { display: flex; gap: 6px; flex-wrap: wrap; }

  .plan-scroll {
    overflow: auto;
    border: 1px solid var(--color-border, #e2e8f0);
    border-radius: var(--radius-md, 8px);
    background: var(--color-surface, #fff);
    max-height: 62vh;
  }
  :global(.dark) .plan-scroll { background: #0b1220; border-color: #334155; }

  .plan { display: block; touch-action: none; color: var(--color-text-muted, #94a3b8); }
  .plan.arming { cursor: copy; }
  .grid-bg { pointer-events: none; }

  .item { cursor: grab; }
  .item:active { cursor: grabbing; }
  .item-rect {
    stroke: rgba(15, 23, 42, 0.35);
    stroke-width: 1;
    transition: filter 0.1s;
  }
  .item:hover .item-rect { filter: brightness(1.08); }
  .item.selected .item-rect { stroke: #2563eb; stroke-width: 2.5; }
  .item.overlapping .item-rect { stroke: #dc2626; stroke-dasharray: 4 2; stroke-width: 2; }
  .item-glyph { font-size: 13px; fill: rgba(15, 23, 42, 0.75); pointer-events: none; }
  .item-label { font-size: 9px; font-weight: 600; fill: rgba(15, 23, 42, 0.85); pointer-events: none; }
  .item-count { font-size: 8px; font-weight: 700; fill: rgba(15, 23, 42, 0.6); pointer-events: none; }
  .resize-handle { fill: #2563eb; cursor: nwse-resize; }

  .plan-footer {
    display: flex;
    justify-content: space-between;
    gap: 10px;
    flex-wrap: wrap;
    font-size: 12px;
    color: var(--color-text-muted, #6b7280);
    margin-top: 6px;
  }
  .overlap-warn { color: var(--color-warning, #d97706); font-weight: 600; }

  /* ── Inspector ───────────────────────────────────────────────────────── */
  .inspector {
    border: 1px solid var(--color-border, #e2e8f0);
    border-radius: var(--radius-md, 8px);
    padding: 14px;
    background: var(--color-surface, #fff);
    max-height: 74vh;
    overflow-y: auto;
  }
  :global(.dark) .inspector { background: #1e293b; border-color: #334155; }

  .inspector-empty { font-size: 13px; color: var(--color-text-muted, #6b7280); line-height: 1.6; }
  .inspector-empty p + p { margin-top: 8px; }

  .inspector-head { display: flex; align-items: center; gap: 10px; margin-bottom: 14px; }
  .inspector-glyph { font-size: 24px; line-height: 1; }
  .inspector-title { font-size: 15px; font-weight: 700; }
  .inspector-sub { font-size: 12px; color: var(--color-text-muted, #6b7280); }

  .capacity-line {
    font-size: 12px;
    color: var(--color-text-muted, #6b7280);
    margin: -6px 0 12px;
    line-height: 1.6;
  }
  .capacity-line code { font-size: 11px; word-break: break-all; }

  .inspector-actions { display: flex; gap: 6px; flex-wrap: wrap; margin-bottom: 14px; }

  /* ── Elevation ───────────────────────────────────────────────────────── */
  .elevation { border-top: 1px solid var(--color-border, #e2e8f0); padding-top: 12px; }
  .elevation h4 { font-size: 12px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.5px; color: var(--color-text-muted, #6b7280); margin-bottom: 8px; }

  .tier { margin-bottom: 8px; }
  .tier-head {
    font-size: 11px;
    font-weight: 600;
    color: var(--color-text-muted, #6b7280);
    margin-bottom: 3px;
  }
  .tier-grid { display: grid; gap: 3px; }
  .slot {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 26px;
    border: 1px solid var(--color-border, #e2e8f0);
    border-radius: 4px;
    background: var(--color-surface-raised, #f8fafc);
    font-size: 9px;
    color: var(--color-text-muted, #6b7280);
  }
  :global(.dark) .slot { background: #0f172a; border-color: #334155; }
  .slot.filled {
    background: color-mix(in srgb, #2563eb 22%, transparent);
    border-color: #2563eb;
    color: var(--color-text, #1e293b);
    font-weight: 600;
  }
  :global(.dark) .slot.filled { color: #e2e8f0; }
  .slot-count {
    position: absolute;
    top: 1px;
    right: 3px;
    font-size: 8px;
    font-weight: 700;
    color: #2563eb;
  }
</style>
