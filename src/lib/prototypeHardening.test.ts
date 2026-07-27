import { describe, it, expect, beforeAll } from 'vitest';
import * as XLSX from 'xlsx';

/**
 * Guards the prototype-pollution hardening applied in `src/main.ts`.
 *
 * `xlsx` 0.18.5 carries GHSA-4r6h-8v6p-xvw6, a prototype-pollution flaw
 * reachable from `XLSX.read` on a user-supplied workbook — which is exactly
 * what the Import screen feeds it. The advisory is fixed in 0.19.3+, but
 * SheetJS stopped publishing to the npm registry at 0.18.5, so the fix is not
 * reachable via `npm update`; it lives at cdn.sheetjs.com. Freezing
 * `Object.prototype` at startup is the interim containment.
 *
 * These tests exist because the mitigation has a failure mode that would be
 * silent and severe: if some dependency legitimately writes to
 * `Object.prototype`, freezing it breaks the app at runtime rather than at
 * build time. So the round-trip below is the real assertion — the workbook path
 * the Import screen uses must keep working with the prototype frozen.
 */
describe('prototype-pollution hardening', () => {
  beforeAll(() => {
    // Mirrors what main.ts does before any other module runs.
    Object.freeze(Object.prototype);
  });

  it('freezes Object.prototype', () => {
    expect(Object.isFrozen(Object.prototype)).toBe(true);
  });

  it('neutralises a __proto__ write instead of polluting every object', () => {
    const victim: Record<string, unknown> = {};
    // Sloppy mode silently ignores this; strict mode (which ES modules use)
    // throws. Either outcome is acceptable — what matters is that no other
    // object inherits the property afterwards.
    try {
      (victim as any).__proto__.polluted = 'yes';
    } catch {
      /* expected under strict mode */
    }
    try {
      Object.assign(Object.prototype, { alsoPolluted: 'yes' });
    } catch {
      /* expected */
    }
    expect(({} as any).polluted).toBeUndefined();
    expect(({} as any).alsoPolluted).toBeUndefined();
    expect(([] as any).polluted).toBeUndefined();
  });

  it('still round-trips the workbook shape the Import screen parses', () => {
    // This is the assertion that would catch the mitigation breaking the app:
    // write -> read -> sheet_to_json is precisely ImportManager's path.
    const wb = XLSX.utils.book_new();
    const ws = XLSX.utils.aoa_to_sheet([
      ['Accession', 'Species Code', 'Notes'],
      ['PTC-001', 'CIT-SIN', 'healthy shoot culture'],
      ['PTC-002', 'CIT-LIM', 'contaminated — Trichoderma'],
    ]);
    XLSX.utils.book_append_sheet(wb, ws, 'Specimens');
    const buf = XLSX.write(wb, { type: 'array', bookType: 'xlsx' });

    const parsed = XLSX.read(buf, { type: 'array', cellDates: false });
    expect(parsed.SheetNames).toContain('Specimens');

    const aoa = XLSX.utils.sheet_to_json<any[]>(parsed.Sheets['Specimens'], {
      header: 1,
      defval: '',
    });
    expect(aoa[0]).toEqual(['Accession', 'Species Code', 'Notes']);
    expect(aoa[1]).toEqual(['PTC-001', 'CIT-SIN', 'healthy shoot culture']);
    expect(aoa[2][2]).toBe('contaminated — Trichoderma');
  });

  it('parses a workbook whose cell text looks like a prototype key', () => {
    // Cell *values* named __proto__/constructor must be carried through as
    // ordinary strings, not treated as structure.
    const wb = XLSX.utils.book_new();
    const ws = XLSX.utils.aoa_to_sheet([
      ['Accession', 'Notes'],
      ['__proto__', 'constructor'],
      ['PTC-003', '{"__proto__":{"polluted":"yes"}}'],
    ]);
    XLSX.utils.book_append_sheet(wb, ws, 'Specimens');
    const buf = XLSX.write(wb, { type: 'array', bookType: 'xlsx' });

    const aoa = XLSX.utils.sheet_to_json<any[]>(
      XLSX.read(buf, { type: 'array' }).Sheets['Specimens'],
      { header: 1, defval: '' },
    );
    expect(aoa[1]).toEqual(['__proto__', 'constructor']);
    expect(aoa[2][1]).toBe('{"__proto__":{"polluted":"yes"}}');
    expect(({} as any).polluted).toBeUndefined();
  });
});
