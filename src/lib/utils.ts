// Pure utility functions shared across components and testable in isolation.

/** Escape a value for safe HTML insertion. Returns '—' for empty/null. */
export function escHtml(s: unknown): string {
  const str = String(s ?? '').replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  return str || '—';
}

/** Convert a health numeric value to a display label. */
export function healthLabel(val: unknown): string {
  if (val === null || val === undefined || val === '' || isNaN(Number(val))) return '—';
  const n = Math.round(Number(val));
  if (n === -1) return '? – Unknown / Awaiting';
  return ['0 – Dead', '1 – Poor', '2 – Fair', '3 – Good', '4 – Healthy'][Math.max(0, Math.min(4, n))];
}

/** Format an underscore-delimited stage string to Title Case. */
export function stageFmt(s: string): string {
  return s?.replace(/_/g, ' ').replace(/\b\w/g, c => c.toUpperCase()) || '—';
}

/**
 * Compose a structured location string from hierarchical parts.
 * Empty parts are omitted. Returns '' if all parts are empty.
 */
export function composeLocation(room: string, rack: string, shelf: string, tray: string): string {
  const parts: string[] = [];
  if (room) parts.push(`Room ${room}`);
  if (rack) parts.push(`Rack ${rack}`);
  if (shelf) parts.push(`Shelf ${shelf}`);
  if (tray) parts.push(`Tray ${tray}`);
  return parts.join(' / ');
}

/**
 * Format an accession number from its components.
 * Pattern: {date}-{speciesCode}-{seq:03}
 */
export function formatAccessionNumber(date: string, speciesCode: string, seq: number): string {
  return `${date}-${speciesCode}-${String(seq).padStart(3, '0')}`;
}

/**
 * Compute the new stock level after an adjustment.
 * Returns an error string if the result would go below zero.
 */
export function computeStockAdjustment(
  current: number,
  adjustment: number,
): { ok: true; value: number } | { ok: false; error: string } {
  const next = current + adjustment;
  if (next < 0) return { ok: false, error: 'Stock cannot go below zero' };
  return { ok: true, value: next };
}

/** Return today's date as YYYY-MM-DD. */
export function datestamp(): string {
  return new Date().toISOString().slice(0, 10);
}
