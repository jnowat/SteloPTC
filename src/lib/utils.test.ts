import { describe, it, expect } from 'vitest';
import {
  escHtml,
  healthLabel,
  stageFmt,
  composeLocation,
  formatAccessionNumber,
  computeStockAdjustment,
  datestamp,
} from './utils';
import { ageDays, fmtAge, healthNum } from './printUtils';

// ── escHtml ───────────────────────────────────────────────────────────────────

describe('escHtml', () => {
  it('escapes ampersands', () => {
    expect(escHtml('a & b')).toBe('a &amp; b');
  });

  it('escapes angle brackets', () => {
    expect(escHtml('<script>')).toBe('&lt;script&gt;');
  });

  it('returns em-dash for null', () => {
    expect(escHtml(null)).toBe('—');
  });

  it('returns em-dash for undefined', () => {
    expect(escHtml(undefined)).toBe('—');
  });

  it('returns em-dash for empty string', () => {
    expect(escHtml('')).toBe('—');
  });

  it('coerces numbers to strings', () => {
    expect(escHtml(42)).toBe('42');
  });

  it('leaves safe text unchanged', () => {
    expect(escHtml('SteloPTC')).toBe('SteloPTC');
  });
});

// ── healthLabel ───────────────────────────────────────────────────────────────

describe('healthLabel', () => {
  it('returns em-dash for null', () => {
    expect(healthLabel(null)).toBe('—');
  });

  it('returns em-dash for NaN string', () => {
    expect(healthLabel('abc')).toBe('—');
  });

  it('returns unknown label for -1', () => {
    expect(healthLabel(-1)).toBe('? – Unknown / Awaiting');
  });

  it('returns Dead for 0', () => {
    expect(healthLabel(0)).toBe('0 – Dead');
  });

  it('returns Healthy for 4', () => {
    expect(healthLabel(4)).toBe('4 – Healthy');
  });

  it('clamps values below 0 to Dead (floor at 0)', () => {
    expect(healthLabel(-2)).toBe('0 – Dead');
  });

  it('clamps values above 4 to Healthy', () => {
    expect(healthLabel(10)).toBe('4 – Healthy');
  });

  it('rounds float values', () => {
    expect(healthLabel(2.7)).toBe('3 – Good');
  });
});

// ── stageFmt ──────────────────────────────────────────────────────────────────

describe('stageFmt', () => {
  it('converts underscore_stage to Title Case', () => {
    expect(stageFmt('shoot_meristem')).toBe('Shoot Meristem');
  });

  it('handles single word', () => {
    expect(stageFmt('explant')).toBe('Explant');
  });

  it('returns em-dash for empty string', () => {
    expect(stageFmt('')).toBe('—');
  });
});

// ── composeLocation ───────────────────────────────────────────────────────────

describe('composeLocation', () => {
  it('composes full four-part location', () => {
    expect(composeLocation('1', 'A', '3', 'B')).toBe('Room 1 / Rack A / Shelf 3 / Tray B');
  });

  it('omits empty parts', () => {
    expect(composeLocation('2', '', '1', '')).toBe('Room 2 / Shelf 1');
  });

  it('returns empty string when all parts empty', () => {
    expect(composeLocation('', '', '', '')).toBe('');
  });

  it('handles room-only', () => {
    expect(composeLocation('3', '', '', '')).toBe('Room 3');
  });
});

// ── formatAccessionNumber ─────────────────────────────────────────────────────

describe('formatAccessionNumber', () => {
  it('zero-pads sequence to three digits', () => {
    expect(formatAccessionNumber('2026-06-13', 'CIT-01', 1)).toBe('2026-06-13-CIT-01-001');
  });

  it('handles sequence >= 100', () => {
    expect(formatAccessionNumber('2026-06-13', 'VAC-02', 100)).toBe('2026-06-13-VAC-02-100');
  });

  it('handles two-digit sequence', () => {
    expect(formatAccessionNumber('2025-01-01', 'ABC', 42)).toBe('2025-01-01-ABC-042');
  });
});

// ── datestamp ─────────────────────────────────────────────────────────────────

describe('datestamp', () => {
  it('returns a string matching YYYY-MM-DD', () => {
    expect(datestamp()).toMatch(/^\d{4}-\d{2}-\d{2}$/);
  });

  it('matches today in UTC', () => {
    const today = new Date().toISOString().slice(0, 10);
    expect(datestamp()).toBe(today);
  });
});

// ── computeStockAdjustment ────────────────────────────────────────────────────

describe('computeStockAdjustment', () => {
  it('adds positive adjustment', () => {
    const r = computeStockAdjustment(10.0, 5.0);
    expect(r).toEqual({ ok: true, value: 15.0 });
  });

  it('subtracts negative adjustment', () => {
    const r = computeStockAdjustment(10.0, -3.5);
    expect(r).toEqual({ ok: true, value: 6.5 });
  });

  it('allows adjustment to exact zero', () => {
    const r = computeStockAdjustment(5.0, -5.0);
    expect(r).toEqual({ ok: true, value: 0.0 });
  });

  it('rejects adjustment that would go negative', () => {
    const r = computeStockAdjustment(2.0, -3.0);
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.error).toMatch(/below zero/i);
  });

  it('handles zero current stock with positive adjustment', () => {
    const r = computeStockAdjustment(0.0, 100.0);
    expect(r).toEqual({ ok: true, value: 100.0 });
  });
});

// ── ageDays (printUtils) ──────────────────────────────────────────────────────

describe('ageDays', () => {
  it('returns null for null input', () => {
    expect(ageDays(null)).toBeNull();
  });

  it('returns null for undefined', () => {
    expect(ageDays(undefined)).toBeNull();
  });

  it('returns null for invalid date string', () => {
    expect(ageDays('not-a-date')).toBeNull();
  });

  it('returns 0 for today', () => {
    const today = new Date().toISOString().slice(0, 10);
    expect(ageDays(today)).toBe(0);
  });

  it('returns positive integer for a past date', () => {
    const past = new Date(Date.now() - 5 * 86400000).toISOString().slice(0, 10);
    expect(ageDays(past)).toBe(5);
  });

  it('returns null for a future date', () => {
    const future = new Date(Date.now() + 2 * 86400000).toISOString().slice(0, 10);
    expect(ageDays(future)).toBeNull();
  });
});

// ── fmtAge (printUtils) ───────────────────────────────────────────────────────

describe('fmtAge', () => {
  it('returns em-dash for null', () => {
    expect(fmtAge(null)).toBe('—');
  });

  it('formats days under 30 as Xd', () => {
    const past = new Date(Date.now() - 14 * 86400000).toISOString().slice(0, 10);
    expect(fmtAge(past)).toBe('14d');
  });

  it('formats exact months with no remainder', () => {
    // 60 days = 2mo exactly
    const past = new Date(Date.now() - 60 * 86400000).toISOString().slice(0, 10);
    expect(fmtAge(past)).toBe('2mo');
  });

  it('formats months with remaining days', () => {
    // 35 days = 1mo 5d
    const past = new Date(Date.now() - 35 * 86400000).toISOString().slice(0, 10);
    expect(fmtAge(past)).toBe('1mo 5d');
  });
});

// ── healthNum (printUtils) ────────────────────────────────────────────────────

describe('healthNum', () => {
  it('returns null for null', () => {
    expect(healthNum(null)).toBeNull();
  });

  it('returns null for empty string', () => {
    expect(healthNum('')).toBeNull();
  });

  it('returns null for NaN string', () => {
    expect(healthNum('abc')).toBeNull();
  });

  it('returns -1 for -1', () => {
    expect(healthNum(-1)).toBe(-1);
  });

  it('returns 0 for 0', () => {
    expect(healthNum(0)).toBe(0);
  });

  it('returns 4 for 4', () => {
    expect(healthNum(4)).toBe(4);
  });

  it('rounds float values', () => {
    expect(healthNum(2.6)).toBe(3);
  });

  it('returns null for values outside [-1, 4]', () => {
    expect(healthNum(5)).toBeNull();
    expect(healthNum(-2)).toBeNull();
  });
});
