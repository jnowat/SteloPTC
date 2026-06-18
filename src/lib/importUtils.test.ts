import { describe, it, expect } from 'vitest';
import { findMissingSheets, REQUIRED_SHEET_NAMES } from './importUtils';

// ── REQUIRED_SHEET_NAMES ──────────────────────────────────────────────────────

describe('REQUIRED_SHEET_NAMES', () => {
  it('contains exactly 6 required sheets', () => {
    expect(REQUIRED_SHEET_NAMES).toHaveLength(6);
  });

  it('includes all expected sheet names', () => {
    expect(REQUIRED_SHEET_NAMES).toContain('Specimens');
    expect(REQUIRED_SHEET_NAMES).toContain('Subcultures');
    expect(REQUIRED_SHEET_NAMES).toContain('Media Batches');
    expect(REQUIRED_SHEET_NAMES).toContain('Prepared Solutions');
    expect(REQUIRED_SHEET_NAMES).toContain('Inventory');
    expect(REQUIRED_SHEET_NAMES).toContain('Compliance');
  });
});

// ── findMissingSheets ─────────────────────────────────────────────────────────

describe('findMissingSheets', () => {
  it('returns empty array when all sheets are present', () => {
    const all = [...REQUIRED_SHEET_NAMES];
    expect(findMissingSheets(all)).toEqual([]);
  });

  it('returns all required names when workbook is empty', () => {
    const missing = findMissingSheets([]);
    expect(missing).toHaveLength(6);
  });

  it('returns only the missing sheet names', () => {
    const present = ['Specimens', 'Subcultures', 'Media Batches', 'Inventory', 'Compliance'];
    const missing = findMissingSheets(present);
    expect(missing).toEqual(['Prepared Solutions']);
  });

  it('ignores extra unrelated sheet names in the workbook', () => {
    const sheets = [...REQUIRED_SHEET_NAMES, 'Dashboard', 'Summary'];
    expect(findMissingSheets(sheets)).toEqual([]);
  });

  it('is case-sensitive — "specimens" does not satisfy "Specimens"', () => {
    const lower = REQUIRED_SHEET_NAMES.map(n => n.toLowerCase());
    expect(findMissingSheets(lower)).toHaveLength(6);
  });

  it('returns multiple missing sheets in declaration order', () => {
    const missing = findMissingSheets(['Specimens']);
    expect(missing).toEqual([
      'Subcultures',
      'Media Batches',
      'Prepared Solutions',
      'Inventory',
      'Compliance',
    ]);
  });
});
