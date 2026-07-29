/**
 * The lab layout model: what a room contains, where it sits on the floor, and
 * what addresses that generates.
 *
 * The thing that makes a lab plan different from a generic floor planner is the
 * third dimension. A five-shelf rack occupies one rectangle on the floor but is
 * *five* places you can put a culture — and each of those shelves holds a grid
 * of trays. So every piece of furniture carries a footprint (`x`/`y`/`w`/`h` in
 * grid cells) **and** a storage breakdown (`tiers` shelves, each `rows` × `cols`
 * positions). A bench or a flow hood is the degenerate case: real footprint,
 * zero tiers, nothing to store.
 *
 * Those addresses are the payoff. SteloPTC already records a specimen's place as
 * a "Room 2 / Rack B / Shelf 3 / Tray C" string, composed in the Add Specimen
 * form from four **hardcoded** dropdowns — rooms 1–5, racks A–D, shelves 1–5,
 * trays A–F — that have nothing to do with any real lab. Once a room is drawn,
 * the same dropdowns are generated from the drawing instead, so the plan and the
 * records describe the same building.
 *
 * Pure data and pure functions only: no Svelte, no DOM, no API calls, so the
 * geometry and the address grammar can be tested directly.
 */

export type FurnitureKind =
  | 'rack'
  | 'cabinet'
  | 'shelf'
  | 'bench'
  | 'hood'
  | 'incubator'
  | 'growth-chamber'
  | 'fridge'
  | 'freezer'
  | 'dewar'
  | 'autoclave'
  | 'sink'
  | 'door'
  | 'wall';

export interface FurnitureItem {
  id: string;
  kind: FurnitureKind;
  /** Operator-facing name; becomes the middle part of every slot address. */
  label: string;
  /** Footprint, in grid cells, with the origin at the top-left of the plan. */
  x: number;
  y: number;
  w: number;
  h: number;
  /** Shelves / levels. 0 means "this holds nothing" (bench, hood, door, wall). */
  tiers: number;
  /** Positions on each shelf. `rows` are lettered, `cols` are numbered. */
  rows: number;
  cols: number;
  notes?: string;
}

export interface LabLayout {
  version: 1;
  gridCols: number;
  gridRows: number;
  items: FurnitureItem[];
}

/** Definition of one palette entry, including the defaults a stamp places. */
export interface FurnitureSpec {
  kind: FurnitureKind;
  label: string;
  glyph: string;
  /** Fill colour for the plan. Deliberately muted — occupancy shading sits on top. */
  color: string;
  w: number;
  h: number;
  tiers: number;
  rows: number;
  cols: number;
  /** What this is, in one line, for the palette tooltip. */
  hint: string;
}

/**
 * The palette, with defaults chosen to match how the equipment is actually
 * built: a mobile culture rack is five shelves of two rows by three trays, a
 * bench-top incubator is three shelves of four, a flow hood stores nothing.
 * Every number is editable after placing — these are the starting point, not a
 * constraint.
 */
export const FURNITURE_SPECS: FurnitureSpec[] = [
  { kind: 'rack',           label: 'Culture rack',   glyph: '▤', color: '#60a5fa', w: 3, h: 1, tiers: 5, rows: 2, cols: 3, hint: 'Five-shelf mobile rack — the workhorse of a growth room' },
  { kind: 'shelf',          label: 'Shelf unit',     glyph: '▥', color: '#818cf8', w: 4, h: 1, tiers: 4, rows: 1, cols: 4, hint: 'Fixed wall shelving' },
  { kind: 'cabinet',        label: 'Cabinet',        glyph: '▦', color: '#a78bfa', w: 2, h: 1, tiers: 2, rows: 2, cols: 2, hint: 'Enclosed two-shelf cabinet' },
  { kind: 'incubator',      label: 'Incubator',      glyph: '◫', color: '#34d399', w: 2, h: 2, tiers: 3, rows: 2, cols: 2, hint: 'Temperature-controlled incubator' },
  { kind: 'growth-chamber', label: 'Growth chamber', glyph: '☀', color: '#4ade80', w: 3, h: 2, tiers: 4, rows: 2, cols: 4, hint: 'Lit, climate-controlled growth chamber' },
  { kind: 'fridge',         label: 'Fridge',         glyph: '❄', color: '#38bdf8', w: 2, h: 2, tiers: 4, rows: 2, cols: 2, hint: '4 °C storage' },
  { kind: 'freezer',        label: 'Freezer',        glyph: '✳', color: '#22d3ee', w: 2, h: 2, tiers: 5, rows: 2, cols: 3, hint: '−20 / −80 °C storage' },
  { kind: 'dewar',          label: 'Cryo dewar',     glyph: '⬤', color: '#67e8f9', w: 1, h: 1, tiers: 6, rows: 1, cols: 5, hint: 'Liquid-nitrogen dewar — canisters × boxes' },
  { kind: 'hood',           label: 'Flow hood',      glyph: '▭', color: '#fbbf24', w: 3, h: 2, tiers: 0, rows: 0, cols: 0, hint: 'Laminar flow hood — a work surface, not storage' },
  { kind: 'bench',          label: 'Bench',          glyph: '▬', color: '#d4a373', w: 4, h: 1, tiers: 0, rows: 0, cols: 0, hint: 'Work bench' },
  { kind: 'autoclave',      label: 'Autoclave',      glyph: '♨', color: '#f87171', w: 2, h: 2, tiers: 0, rows: 0, cols: 0, hint: 'Autoclave / steriliser' },
  { kind: 'sink',           label: 'Sink',           glyph: '≈', color: '#94a3b8', w: 1, h: 1, tiers: 0, rows: 0, cols: 0, hint: 'Sink or wash station' },
  { kind: 'door',           label: 'Door',           glyph: '⌷', color: '#cbd5e1', w: 1, h: 1, tiers: 0, rows: 0, cols: 0, hint: 'Doorway — for orientation' },
  { kind: 'wall',           label: 'Wall',           glyph: '█', color: '#64748b', w: 1, h: 1, tiers: 0, rows: 0, cols: 0, hint: 'Wall segment — drag to draw a run' },
];

const SPEC_BY_KIND = new Map(FURNITURE_SPECS.map((s) => [s.kind, s]));

export function specFor(kind: FurnitureKind): FurnitureSpec {
  // Falling back to the rack rather than throwing keeps a layout that was saved
  // by a newer version — with a kind this build has never heard of — renderable
  // instead of blanking the whole plan.
  return SPEC_BY_KIND.get(kind) ?? FURNITURE_SPECS[0];
}

// ── Limits ───────────────────────────────────────────────────────────────────
//
// Address enumeration is tiers × rows × cols per item, and the Add Specimen
// dropdowns list them. Caps keep a fat-fingered "500 shelves" from generating a
// hundred thousand options and locking up the form.

export const MAX_GRID = 60;
export const MIN_GRID = 4;
export const MAX_TIERS = 30;
export const MAX_ROWS = 26; // one letter per row: A–Z
export const MAX_COLS = 40;
/** Hard ceiling on generated addresses per room, across all furniture. */
export const MAX_SLOTS_PER_LAYOUT = 5000;

export const DEFAULT_GRID_COLS = 20;
export const DEFAULT_GRID_ROWS = 14;

export function emptyLayout(): LabLayout {
  return { version: 1, gridCols: DEFAULT_GRID_COLS, gridRows: DEFAULT_GRID_ROWS, items: [] };
}

function clamp(value: number, min: number, max: number): number {
  if (!Number.isFinite(value)) return min;
  return Math.min(max, Math.max(min, Math.round(value)));
}

// ── Construction ─────────────────────────────────────────────────────────────

/**
 * Next free label for a kind: "Rack A", "Rack B", … then "Rack A2" once the
 * alphabet runs out. Auto-naming matters more than it looks — the label is the
 * middle of every address this item generates, so an operator who never renames
 * anything still gets addresses that read properly.
 */
export function nextLabel(layout: LabLayout, kind: FurnitureKind): string {
  const spec = specFor(kind);
  const base = spec.label;
  const used = new Set(layout.items.filter((i) => i.kind === kind).map((i) => i.label));
  for (let round = 0; round < 40; round++) {
    for (let i = 0; i < 26; i++) {
      const suffix = String.fromCharCode(65 + i) + (round === 0 ? '' : String(round + 1));
      const candidate = `${base} ${suffix}`;
      if (!used.has(candidate)) return candidate;
    }
  }
  return `${base} ${layout.items.length + 1}`;
}

export function createItem(
  layout: LabLayout,
  kind: FurnitureKind,
  x: number,
  y: number,
  id: string,
): FurnitureItem {
  const spec = specFor(kind);
  return normalizeItem(
    {
      id,
      kind,
      label: nextLabel(layout, kind),
      x,
      y,
      w: spec.w,
      h: spec.h,
      tiers: spec.tiers,
      rows: spec.rows,
      cols: spec.cols,
    },
    layout,
  );
}

/** Clamp one item into the grid and into the size limits. */
export function normalizeItem(item: FurnitureItem, layout: LabLayout): FurnitureItem {
  const w = clamp(item.w, 1, layout.gridCols);
  const h = clamp(item.h, 1, layout.gridRows);
  return {
    ...item,
    w,
    h,
    x: clamp(item.x, 0, Math.max(0, layout.gridCols - w)),
    y: clamp(item.y, 0, Math.max(0, layout.gridRows - h)),
    tiers: clamp(item.tiers, 0, MAX_TIERS),
    rows: clamp(item.rows, 0, MAX_ROWS),
    cols: clamp(item.cols, 0, MAX_COLS),
    label: (item.label ?? '').trim() || 'Unnamed',
  };
}

/** Swap width and height, keeping the top-left corner put. */
export function rotateItem(item: FurnitureItem, layout: LabLayout): FurnitureItem {
  return normalizeItem({ ...item, w: item.h, h: item.w }, layout);
}

// ── Geometry ─────────────────────────────────────────────────────────────────

export function itemsOverlap(a: FurnitureItem, b: FurnitureItem): boolean {
  return a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;
}

/**
 * IDs of items that overlap at least one other item.
 *
 * Overlap is surfaced, not forbidden: real rooms have a rack tucked under a
 * bench and a dewar wedged beside a freezer, and a planner that refuses to let
 * you draw that is a planner people stop using. The editor tints them instead.
 */
export function findOverlaps(layout: LabLayout): Set<string> {
  const hits = new Set<string>();
  for (let i = 0; i < layout.items.length; i++) {
    for (let j = i + 1; j < layout.items.length; j++) {
      if (itemsOverlap(layout.items[i], layout.items[j])) {
        hits.add(layout.items[i].id);
        hits.add(layout.items[j].id);
      }
    }
  }
  return hits;
}

// ── Addresses ────────────────────────────────────────────────────────────────

/** `0 → A`, `25 → Z`, `26 → AA`. */
export function rowLetter(index: number): string {
  let n = Math.max(0, Math.floor(index));
  let out = '';
  do {
    out = String.fromCharCode(65 + (n % 26)) + out;
    n = Math.floor(n / 26) - 1;
  } while (n >= 0);
  return out;
}

/** Positions on one shelf: `A1`, `A2`, `B1`, … */
export function positionLabel(row: number, col: number): string {
  return `${rowLetter(row)}${col + 1}`;
}

/** How many addressable positions a piece of furniture has. */
export function capacityOf(item: FurnitureItem): number {
  if (item.tiers <= 0 || item.rows <= 0 || item.cols <= 0) return 0;
  return item.tiers * item.rows * item.cols;
}

export function totalCapacity(layout: LabLayout): number {
  return layout.items.reduce((sum, item) => sum + capacityOf(item), 0);
}

/**
 * The address of one slot, in the same `A / B / C` grammar the Add Specimen form
 * has always written, so drawn addresses and hand-typed ones sort and read
 * alike: `Growth Room B / Rack A / Shelf 3 / B2`.
 *
 * The room segment is omitted when there is no room name, which is what happens
 * on a plan that has not been attached to a Location yet.
 */
export function slotAddress(
  roomName: string,
  item: FurnitureItem,
  tier: number,
  row: number,
  col: number,
): string {
  const parts: string[] = [];
  if (roomName.trim()) parts.push(roomName.trim());
  parts.push(item.label);
  if (item.tiers > 1 || item.rows > 0 || item.cols > 0) parts.push(`Shelf ${tier + 1}`);
  if (item.rows > 0 && item.cols > 0) parts.push(positionLabel(row, col));
  return parts.join(' / ');
}

/** The address of a whole shelf, used as the heading of a tier in the editor. */
export function tierAddress(roomName: string, item: FurnitureItem, tier: number): string {
  const parts: string[] = [];
  if (roomName.trim()) parts.push(roomName.trim());
  parts.push(item.label);
  parts.push(`Shelf ${tier + 1}`);
  return parts.join(' / ');
}

/**
 * Every address a layout generates, root-to-leaf in reading order, capped at
 * `MAX_SLOTS_PER_LAYOUT`. The cap is silent by design at this layer — callers
 * that need to tell the user compare against `totalCapacity`.
 */
export function enumerateSlots(layout: LabLayout, roomName = ''): string[] {
  const out: string[] = [];
  for (const item of layout.items) {
    if (capacityOf(item) === 0) continue;
    for (let t = 0; t < item.tiers; t++) {
      for (let r = 0; r < item.rows; r++) {
        for (let c = 0; c < item.cols; c++) {
          if (out.length >= MAX_SLOTS_PER_LAYOUT) return out;
          out.push(slotAddress(roomName, item, t, r, c));
        }
      }
    }
  }
  return out;
}

/** Furniture that can hold something, for the Add Specimen dropdowns. */
export function storageItems(layout: LabLayout): FurnitureItem[] {
  return layout.items.filter((i) => capacityOf(i) > 0);
}

// ── Serialisation ────────────────────────────────────────────────────────────

/**
 * Parse a stored layout, returning `null` for anything unusable.
 *
 * Written defensively because the input is a TEXT column: it can hold a layout
 * from a newer build, a half-written string from a crash, or nothing at all. A
 * throw here would take out the whole Lab Map view, so every field is checked
 * and bad items are dropped rather than trusted.
 */
export function parseLayout(raw: string | null | undefined): LabLayout | null {
  if (!raw || !raw.trim()) return null;
  let parsed: any;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return null;
  }
  if (!parsed || typeof parsed !== 'object' || !Array.isArray(parsed.items)) return null;

  const layout: LabLayout = {
    version: 1,
    gridCols: clamp(parsed.gridCols ?? DEFAULT_GRID_COLS, MIN_GRID, MAX_GRID),
    gridRows: clamp(parsed.gridRows ?? DEFAULT_GRID_ROWS, MIN_GRID, MAX_GRID),
    items: [],
  };

  const seenIds = new Set<string>();
  for (const raw of parsed.items) {
    if (!raw || typeof raw !== 'object') continue;
    const id = typeof raw.id === 'string' && raw.id ? raw.id : null;
    if (!id || seenIds.has(id)) continue;
    seenIds.add(id);
    layout.items.push(
      normalizeItem(
        {
          id,
          kind: (typeof raw.kind === 'string' ? raw.kind : 'rack') as FurnitureKind,
          label: typeof raw.label === 'string' ? raw.label : '',
          x: Number(raw.x) || 0,
          y: Number(raw.y) || 0,
          w: Number(raw.w) || 1,
          h: Number(raw.h) || 1,
          tiers: Number(raw.tiers) || 0,
          rows: Number(raw.rows) || 0,
          cols: Number(raw.cols) || 0,
          notes: typeof raw.notes === 'string' ? raw.notes : undefined,
        },
        layout,
      ),
    );
  }
  return layout;
}

export function serializeLayout(layout: LabLayout): string {
  return JSON.stringify({
    version: 1,
    gridCols: layout.gridCols,
    gridRows: layout.gridRows,
    items: layout.items,
  });
}

// ── Occupancy ────────────────────────────────────────────────────────────────

/**
 * Fold a list of `{ location, count }` rows into a per-item total.
 *
 * Specimen locations are the free-text strings the Add Specimen form composes,
 * so an item's occupancy is every recorded location whose path passes through
 * that item's label. Matching on the ` / `-delimited segments rather than a
 * substring keeps "Rack A" from also counting "Rack A2", which a naive
 * `includes` would.
 */
export function occupancyByItem(
  layout: LabLayout,
  rows: Array<{ location: string; count: number }>,
): Map<string, number> {
  const byLabel = new Map<string, string>();
  for (const item of layout.items) byLabel.set(item.label.toLowerCase(), item.id);

  const totals = new Map<string, number>();
  for (const row of rows) {
    if (!row.location) continue;
    for (const segment of row.location.split('/')) {
      const id = byLabel.get(segment.trim().toLowerCase());
      if (id) {
        totals.set(id, (totals.get(id) ?? 0) + row.count);
        break;
      }
    }
  }
  return totals;
}

/** Occupancy of individual slots, keyed by the full address (case-normalised). */
export function occupancyBySlot(
  rows: Array<{ location: string; count: number }>,
): Map<string, number> {
  const totals = new Map<string, number>();
  for (const row of rows) {
    if (!row.location) continue;
    const key = row.location.trim().toLowerCase();
    totals.set(key, (totals.get(key) ?? 0) + row.count);
  }
  return totals;
}
