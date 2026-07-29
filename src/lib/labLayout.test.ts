import { describe, it, expect } from 'vitest';
import {
  emptyLayout,
  createItem,
  normalizeItem,
  rotateItem,
  nextLabel,
  itemsOverlap,
  findOverlaps,
  rowLetter,
  positionLabel,
  capacityOf,
  totalCapacity,
  slotAddress,
  tierAddress,
  enumerateSlots,
  storageItems,
  parseLayout,
  serializeLayout,
  occupancyByItem,
  occupancyBySlot,
  specFor,
  FURNITURE_SPECS,
  MAX_SLOTS_PER_LAYOUT,
  MAX_TIERS,
  type FurnitureItem,
  type LabLayout,
} from './labLayout';

function item(over: Partial<FurnitureItem> = {}): FurnitureItem {
  return {
    id: 'i1',
    kind: 'rack',
    label: 'Rack A',
    x: 0,
    y: 0,
    w: 3,
    h: 1,
    tiers: 5,
    rows: 2,
    cols: 3,
    ...over,
  };
}

describe('the palette', () => {
  it('gives the culture rack five shelves — the case the design is built around', () => {
    const rack = specFor('rack');
    expect(rack.tiers).toBe(5);
    expect(rack.rows * rack.cols).toBeGreaterThan(1);
  });

  it('gives the cabinet two shelves', () => {
    expect(specFor('cabinet').tiers).toBe(2);
  });

  it('gives work surfaces no storage at all', () => {
    for (const kind of ['hood', 'bench', 'autoclave', 'sink', 'door', 'wall'] as const) {
      expect(specFor(kind).tiers).toBe(0);
    }
  });

  it('falls back to a known spec for a kind this build has never seen', () => {
    // A layout saved by a newer version must still render rather than blanking.
    expect(specFor('teleporter' as any)).toBeDefined();
  });

  it('has a unique kind per palette entry', () => {
    const kinds = FURNITURE_SPECS.map((s) => s.kind);
    expect(new Set(kinds).size).toBe(kinds.length);
  });
});

describe('createItem and auto-labelling', () => {
  it('names the first rack "Rack A" and the second "Rack B"', () => {
    const layout = emptyLayout();
    const first = createItem(layout, 'rack', 0, 0, 'a');
    layout.items.push(first);
    const second = createItem(layout, 'rack', 4, 0, 'b');
    expect(first.label).toBe('Culture rack A');
    expect(second.label).toBe('Culture rack B');
  });

  it('numbers a second alphabet rather than colliding', () => {
    const layout = emptyLayout();
    for (let i = 0; i < 26; i++) {
      layout.items.push({ ...item({ id: `x${i}`, label: `Culture rack ${String.fromCharCode(65 + i)}` }) });
    }
    expect(nextLabel(layout, 'rack')).toBe('Culture rack A2');
  });

  it('takes its footprint and shelf breakdown from the palette', () => {
    const created = createItem(emptyLayout(), 'freezer', 1, 1, 'f');
    const spec = specFor('freezer');
    expect(created.w).toBe(spec.w);
    expect(created.tiers).toBe(spec.tiers);
  });
});

describe('normalizeItem', () => {
  const layout: LabLayout = { version: 1, gridCols: 10, gridRows: 8, items: [] };

  it('keeps an item inside the grid', () => {
    const n = normalizeItem(item({ x: 50, y: 50, w: 3, h: 1 }), layout);
    expect(n.x).toBe(7); // gridCols 10 - w 3
    expect(n.y).toBe(7); // gridRows 8 - h 1
  });

  it('refuses a zero or negative footprint', () => {
    const n = normalizeItem(item({ w: 0, h: -4 }), layout);
    expect(n.w).toBe(1);
    expect(n.h).toBe(1);
  });

  it('caps tiers so one typo cannot generate a hundred thousand addresses', () => {
    const n = normalizeItem(item({ tiers: 9999 }), layout);
    expect(n.tiers).toBe(MAX_TIERS);
  });

  it('gives a blank label a placeholder rather than producing "Room / / Shelf 1"', () => {
    expect(normalizeItem(item({ label: '   ' }), layout).label).toBe('Unnamed');
  });

  it('survives NaN coming out of a number input', () => {
    const n = normalizeItem(item({ x: NaN, w: NaN }), layout);
    expect(Number.isFinite(n.x)).toBe(true);
    expect(n.w).toBeGreaterThanOrEqual(1);
  });
});

describe('rotateItem', () => {
  it('swaps width and height and keeps the corner put', () => {
    const layout: LabLayout = { version: 1, gridCols: 20, gridRows: 20, items: [] };
    const r = rotateItem(item({ x: 2, y: 3, w: 4, h: 1 }), layout);
    expect([r.w, r.h]).toEqual([1, 4]);
    expect([r.x, r.y]).toEqual([2, 3]);
  });

  it('pulls a rotated item back inside the grid when it would hang off the edge', () => {
    const layout: LabLayout = { version: 1, gridCols: 10, gridRows: 4, items: [] };
    const r = rotateItem(item({ x: 0, y: 2, w: 4, h: 1 }), layout);
    expect(r.y + r.h).toBeLessThanOrEqual(layout.gridRows);
  });
});

describe('overlap detection', () => {
  it('sees two rectangles that share cells', () => {
    expect(itemsOverlap(item({ x: 0, y: 0, w: 2, h: 2 }), item({ x: 1, y: 1, w: 2, h: 2 }))).toBe(true);
  });

  it('does not count edge-to-edge neighbours as overlapping', () => {
    expect(itemsOverlap(item({ x: 0, y: 0, w: 2, h: 2 }), item({ x: 2, y: 0, w: 2, h: 2 }))).toBe(false);
  });

  it('reports both sides of every collision', () => {
    const layout: LabLayout = {
      version: 1,
      gridCols: 20,
      gridRows: 20,
      items: [
        item({ id: 'a', x: 0, y: 0, w: 2, h: 2 }),
        item({ id: 'b', x: 1, y: 1, w: 2, h: 2 }),
        item({ id: 'c', x: 10, y: 10, w: 2, h: 2 }),
      ],
    };
    expect(findOverlaps(layout)).toEqual(new Set(['a', 'b']));
  });
});

describe('address grammar', () => {
  it('letters rows A, B, … Z then AA', () => {
    expect(rowLetter(0)).toBe('A');
    expect(rowLetter(25)).toBe('Z');
    expect(rowLetter(26)).toBe('AA');
  });

  it('numbers positions from 1', () => {
    expect(positionLabel(0, 0)).toBe('A1');
    expect(positionLabel(1, 2)).toBe('B3');
  });

  it('builds the room / furniture / shelf / position path the specimen form already writes', () => {
    expect(slotAddress('Growth Room B', item(), 2, 1, 1)).toBe('Growth Room B / Rack A / Shelf 3 / B2');
  });

  it('omits the room segment when the plan is not attached to one', () => {
    expect(slotAddress('', item(), 0, 0, 0)).toBe('Rack A / Shelf 1 / A1');
  });

  it('drops the position segment for furniture with no grid on its shelves', () => {
    expect(slotAddress('Room 1', item({ rows: 0, cols: 0 }), 0, 0, 0)).toBe('Room 1 / Rack A / Shelf 1');
  });

  it('addresses a whole shelf for the tier heading', () => {
    expect(tierAddress('Room 1', item(), 4)).toBe('Room 1 / Rack A / Shelf 5');
  });
});

describe('capacity', () => {
  it('multiplies shelves by the grid on each shelf', () => {
    expect(capacityOf(item({ tiers: 5, rows: 2, cols: 3 }))).toBe(30);
  });

  it('reports zero for a work surface', () => {
    expect(capacityOf(item({ tiers: 0, rows: 0, cols: 0 }))).toBe(0);
  });

  it('reports zero when a shelf has no positions, rather than counting shelves', () => {
    expect(capacityOf(item({ tiers: 4, rows: 0, cols: 0 }))).toBe(0);
  });

  it('totals across the layout', () => {
    const layout: LabLayout = {
      version: 1,
      gridCols: 20,
      gridRows: 20,
      items: [item({ id: 'a', tiers: 5, rows: 2, cols: 3 }), item({ id: 'b', tiers: 2, rows: 2, cols: 2 })],
    };
    expect(totalCapacity(layout)).toBe(38);
  });
});

describe('enumerateSlots', () => {
  const layout: LabLayout = {
    version: 1,
    gridCols: 20,
    gridRows: 20,
    items: [item({ tiers: 2, rows: 1, cols: 2 })],
  };

  it('lists every address in reading order', () => {
    expect(enumerateSlots(layout, 'Room 1')).toEqual([
      'Room 1 / Rack A / Shelf 1 / A1',
      'Room 1 / Rack A / Shelf 1 / A2',
      'Room 1 / Rack A / Shelf 2 / A1',
      'Room 1 / Rack A / Shelf 2 / A2',
    ]);
  });

  it('skips furniture that holds nothing', () => {
    const withBench: LabLayout = {
      ...layout,
      items: [...layout.items, item({ id: 'b', kind: 'bench', label: 'Bench A', tiers: 0, rows: 0, cols: 0 })],
    };
    expect(enumerateSlots(withBench).every((a) => !a.includes('Bench'))).toBe(true);
  });

  it('stops at the cap instead of generating an unbounded list', () => {
    const huge: LabLayout = {
      version: 1,
      gridCols: 40,
      gridRows: 40,
      items: Array.from({ length: 20 }, (_, i) =>
        item({ id: `i${i}`, label: `Rack ${i}`, tiers: 30, rows: 20, cols: 40 })
      ),
    };
    expect(enumerateSlots(huge)).toHaveLength(MAX_SLOTS_PER_LAYOUT);
  });

  it('returns only storage furniture from storageItems', () => {
    const mixed: LabLayout = {
      ...layout,
      items: [...layout.items, item({ id: 'h', kind: 'hood', label: 'Hood A', tiers: 0, rows: 0, cols: 0 })],
    };
    expect(storageItems(mixed).map((i) => i.label)).toEqual(['Rack A']);
  });
});

describe('serialisation', () => {
  it('round-trips a layout', () => {
    const layout: LabLayout = { version: 1, gridCols: 12, gridRows: 9, items: [item()] };
    const back = parseLayout(serializeLayout(layout));
    expect(back?.gridCols).toBe(12);
    expect(back?.items[0]).toMatchObject({ label: 'Rack A', tiers: 5 });
  });

  it('returns null for empty or absent input', () => {
    expect(parseLayout(null)).toBeNull();
    expect(parseLayout('')).toBeNull();
    expect(parseLayout('   ')).toBeNull();
  });

  it('returns null rather than throwing on a truncated string', () => {
    // The column is TEXT; a crash mid-write can leave anything in it, and a
    // throw here would take out the whole Lab Map view.
    expect(parseLayout('{"items":[')).toBeNull();
    expect(parseLayout('not json at all')).toBeNull();
    expect(parseLayout('{"gridCols":10}')).toBeNull();
  });

  it('drops malformed items but keeps the good ones', () => {
    const raw = JSON.stringify({
      version: 1,
      gridCols: 10,
      gridRows: 10,
      items: [{ id: 'ok', kind: 'rack', label: 'Rack A', x: 0, y: 0, w: 2, h: 1, tiers: 3, rows: 1, cols: 2 }, null, { label: 'no id' }],
    });
    const parsed = parseLayout(raw);
    expect(parsed?.items).toHaveLength(1);
    expect(parsed?.items[0].id).toBe('ok');
  });

  it('drops duplicate ids, which would otherwise break keyed rendering', () => {
    const raw = JSON.stringify({
      gridCols: 10,
      gridRows: 10,
      items: [
        { id: 'dup', kind: 'rack', label: 'A', x: 0, y: 0, w: 1, h: 1, tiers: 1, rows: 1, cols: 1 },
        { id: 'dup', kind: 'rack', label: 'B', x: 2, y: 0, w: 1, h: 1, tiers: 1, rows: 1, cols: 1 },
      ],
    });
    expect(parseLayout(raw)?.items).toHaveLength(1);
  });

  it('clamps an out-of-range grid from a hand-edited blob', () => {
    const parsed = parseLayout(JSON.stringify({ gridCols: 9999, gridRows: 0, items: [] }));
    expect(parsed!.gridCols).toBeLessThanOrEqual(60);
    expect(parsed!.gridRows).toBeGreaterThanOrEqual(4);
  });
});

describe('occupancy', () => {
  const layout: LabLayout = {
    version: 1,
    gridCols: 20,
    gridRows: 20,
    items: [item({ id: 'a', label: 'Rack A' }), item({ id: 'a2', label: 'Rack A2' })],
  };

  it('attributes a specimen location to the furniture named in its path', () => {
    const totals = occupancyByItem(layout, [{ location: 'Room 1 / Rack A / Shelf 2 / B1', count: 3 }]);
    expect(totals.get('a')).toBe(3);
  });

  it('does not let "Rack A" absorb the count for "Rack A2"', () => {
    // A substring match would; matching whole path segments does not.
    const totals = occupancyByItem(layout, [{ location: 'Room 1 / Rack A2 / Shelf 1 / A1', count: 5 }]);
    expect(totals.get('a')).toBeUndefined();
    expect(totals.get('a2')).toBe(5);
  });

  it('sums several locations onto one item', () => {
    const totals = occupancyByItem(layout, [
      { location: 'Room 1 / Rack A / Shelf 1 / A1', count: 2 },
      { location: 'Room 1 / Rack A / Shelf 3 / B2', count: 4 },
    ]);
    expect(totals.get('a')).toBe(6);
  });

  it('ignores locations that name nothing in the plan', () => {
    const totals = occupancyByItem(layout, [{ location: 'Somewhere else entirely', count: 9 }]);
    expect(totals.size).toBe(0);
  });

  it('matches furniture labels case-insensitively', () => {
    const totals = occupancyByItem(layout, [{ location: 'room 1 / rack a / shelf 1 / a1', count: 1 }]);
    expect(totals.get('a')).toBe(1);
  });

  it('keys slot occupancy on the normalised full address', () => {
    const totals = occupancyBySlot([{ location: '  Room 1 / Rack A / Shelf 1 / A1  ', count: 2 }]);
    expect(totals.get('room 1 / rack a / shelf 1 / a1')).toBe(2);
  });
});
