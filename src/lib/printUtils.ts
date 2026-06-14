/**
 * Shared print delivery utilities used by SpecimenList, SpecimenDetail, and QrModal.
 *
 * Strategy: prefer a popup window so the report renders in isolation; fall back to an
 * in-page hidden frame when window.open is unavailable (Tauri, restricted WebViews).
 *
 * IMPORTANT: window.print() is always called from the PARENT WebView context, never
 * via an inline <script> tag.  Tauri's CSP `script-src 'self'` blocks inline scripts
 * in popup windows, which would silently prevent the print dialog from appearing.
 */

export type DeliverPrintOptions = {
  /** Unique DOM id for the in-page fallback frame — must be stable across calls. */
  frameId: string;
  /** Browser window title / <title> element text. */
  title: string;
  /** Raw CSS to inject (no surrounding <style> tags). */
  css: string;
  /** HTML body content. */
  body: string;
  /** @page margin shorthand. Default: '0.65in 0.7in' */
  margin?: string;
  /** @page size value. Default: 'auto' */
  pageSize?: string;
  /** Called with a user-facing message when delivery fails. */
  onError: (msg: string) => void;
};

/**
 * Deliver a print document.
 *
 * Tries to open a popup window first.  If the popup is blocked or unavailable
 * (common in Tauri), falls back to injecting a hidden frame into the current
 * page and calling window.print() with @media print CSS that hides everything else.
 */
export function deliverPrint(opts: DeliverPrintOptions): void {
  const {
    frameId, title, css, body,
    margin = '0.65in 0.7in',
    pageSize = 'auto',
    onError,
  } = opts;

  const pageRule = `@page{size:${pageSize};margin:${margin}}`;
  const fullHtml = [
    '<!DOCTYPE html>',
    '<html lang="en">',
    `<head><meta charset="UTF-8"><title>${title}</title>`,
    `<style>${pageRule}${css}</style></head>`,
    `<body>${body}</body>`,
    '</html>',
  ].join('');

  // ── Popup window path ─────────────────────────────────────────────────────
  let win: Window | null = null;
  try { win = window.open('', '_blank', 'width=1200,height=900'); } catch (_) {}

  if (win) {
    try {
      win.document.write(fullHtml);
      win.document.close();
      win.focus();
      // Trigger print from the parent context — avoids CSP blocking in the popup.
      win.print();
    } catch (_e) {
      onError('Failed to generate the print report. Please try again.');
      try { win?.close(); } catch (_) {}
    }
    return;
  }

  // ── In-page fallback (Tauri / restricted WebView) ─────────────────────────
  // Inject an off-screen frame and a @media print stylesheet that hides the
  // rest of the page, then call window.print() on the host window.
  try {
    const styleEl = document.createElement('style');
    styleEl.setAttribute('data-ptc-print', frameId);
    styleEl.textContent = [
      pageRule,
      `@media print{`,
      `body>*:not(#${frameId}){display:none!important}`,
      `#${frameId}{display:block!important}`,
      css,
      `}`,
    ].join('');

    const frame = document.createElement('div');
    frame.id = frameId;
    frame.style.cssText = 'display:none';
    frame.innerHTML = body;

    document.head.appendChild(styleEl);
    document.body.appendChild(frame);

    // Clean up after the print dialog is dismissed.
    window.addEventListener('afterprint', () => {
      styleEl.remove();
      frame.remove();
    }, { once: true });

    // Small delay gives the browser time to paint before opening the dialog.
    setTimeout(() => window.print(), 80);
  } catch (_e) {
    onError('Failed to generate the print report. Please try again.');
  }
}

// ── Date / age helpers ────────────────────────────────────────────────────────

/**
 * Compute the age in whole days from a YYYY-MM-DD date string.
 * Returns null for invalid or missing input, or if the date is in the future.
 */
export function ageDays(dateStr: string | null | undefined): number | null {
  if (!dateStr) return null;
  const [y, m, d] = dateStr.split('-').map(Number);
  if (!y || !m || !d) return null;
  const ms = Date.now() - new Date(y, m - 1, d).getTime();
  const days = Math.floor(ms / 86400000);
  return days >= 0 ? days : null;
}

/**
 * Format a YYYY-MM-DD date as a compact age string.
 * Examples: "14d", "3mo", "3mo 5d".
 */
export function fmtAge(dateStr: string | null | undefined): string {
  const d = ageDays(dateStr);
  if (d === null) return '—';
  if (d < 30) return `${d}d`;
  const months = Math.floor(d / 30);
  const rem = d % 30;
  return rem > 0 ? `${months}mo ${rem}d` : `${months}mo`;
}

// ── Health numeric helper ─────────────────────────────────────────────────────

/**
 * Parse a raw health status value into an integer in [−1, 4], or null.
 *
 * Returns null for empty / non-numeric values so callers can distinguish
 * "no data" from a real score.  Does NOT clamp — values outside [−1, 4]
 * are treated as invalid and return null.
 */
export function healthNum(val: unknown): number | null {
  if (val === null || val === undefined || val === '' || isNaN(Number(val))) return null;
  const n = Math.round(Number(val));
  return n >= -1 && n <= 4 ? n : null;
}
