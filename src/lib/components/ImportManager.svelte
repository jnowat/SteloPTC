<script lang="ts">
  import * as XLSX from 'xlsx';
  import { importXlsx } from '../api';
  import { addNotification } from '../stores/app';

  // ── State ─────────────────────────────────────────────────────────────────

  type SheetStats = { creates: number; updates: number; skips: number };
  type RowError   = { sheet: string; row: number; message: string };
  type ImportResult = {
    specimens: SheetStats;
    subcultures: SheetStats;
    media: SheetStats;
    prepared_solutions: SheetStats;
    inventory: SheetStats;
    compliance: SheetStats;
    errors: RowError[];
    dry_run: boolean;
  };

  let busy       = $state(false);
  let progress   = $state('');
  let fileName   = $state('');
  let preview    = $state<ImportResult | null>(null);
  let committed  = $state<ImportResult | null>(null);
  let parseError = $state('');

  // Parsed payload held in memory between dry-run and commit
  let pendingPayload: Parameters<typeof importXlsx>[0] | null = null;

  // ── Sheet parsing ─────────────────────────────────────────────────────────

  const SHEET_NAMES = [
    'Specimens',
    'Subcultures',
    'Media Batches',
    'Prepared Solutions',
    'Inventory',
    'Compliance',
  ] as const;

  function sheetToRows(ws: XLSX.WorkSheet): string[][] {
    const aoa: any[][] = XLSX.utils.sheet_to_json(ws, { header: 1, defval: '' });
    // Skip the header row; stringify every cell
    return aoa.slice(1).map((row: any[]) => row.map((cell: any) => String(cell ?? '')));
  }

  async function parseFile(file: File): Promise<Parameters<typeof importXlsx>[0] | null> {
    const buf = await file.arrayBuffer();
    const wb  = XLSX.read(buf, { type: 'array', cellDates: false });

    const missing = SHEET_NAMES.filter(n => !wb.SheetNames.includes(n));
    if (missing.length > 0) {
      parseError = `Missing sheet(s): ${missing.join(', ')}. Make sure this file was exported by SteloPTC.`;
      return null;
    }

    return {
      specimens:          sheetToRows(wb.Sheets['Specimens']),
      subcultures:        sheetToRows(wb.Sheets['Subcultures']),
      media:              sheetToRows(wb.Sheets['Media Batches']),
      prepared_solutions: sheetToRows(wb.Sheets['Prepared Solutions']),
      inventory:          sheetToRows(wb.Sheets['Inventory']),
      compliance:         sheetToRows(wb.Sheets['Compliance']),
    };
  }

  // ── Handlers ──────────────────────────────────────────────────────────────

  async function handleFileChange(e: Event) {
    const input = e.target as HTMLInputElement;
    const file  = input.files?.[0];
    if (!file) return;

    parseError    = '';
    preview       = null;
    committed     = null;
    pendingPayload = null;
    fileName      = file.name;

    busy     = true;
    progress = 'Reading file…';
    try {
      const payload = await parseFile(file);
      if (!payload) { busy = false; progress = ''; return; }

      progress = 'Validating…';
      const result = await importXlsx(payload, true);
      pendingPayload = payload;
      preview        = result;
    } catch (err: any) {
      parseError = err.message ?? 'Failed to parse file';
    } finally {
      busy     = false;
      progress = '';
    }
  }

  async function handleCommit() {
    if (!pendingPayload) return;
    busy     = true;
    progress = 'Importing…';
    try {
      const result = await importXlsx(pendingPayload, false);
      committed      = result;
      pendingPayload = null;
      preview        = null;
      addNotification('Import completed successfully', 'success');
    } catch (err: any) {
      addNotification(err.message ?? 'Import failed', 'error');
    } finally {
      busy     = false;
      progress = '';
    }
  }

  function handleReset() {
    fileName       = '';
    preview        = null;
    committed      = null;
    pendingPayload = null;
    parseError     = '';
    // Reset the file input by clearing its value via DOM
    const inp = document.getElementById('xlsx-input') as HTMLInputElement | null;
    if (inp) inp.value = '';
  }

  // ── Display helpers ───────────────────────────────────────────────────────

  const SHEET_LABELS: Record<keyof Omit<ImportResult, 'errors' | 'dry_run'>, string> = {
    specimens:          'Specimens',
    subcultures:        'Subcultures',
    media:              'Media Batches',
    prepared_solutions: 'Prepared Solutions',
    inventory:          'Inventory',
    compliance:         'Compliance',
  };

  function totalRows(r: ImportResult) {
    return (['specimens','subcultures','media','prepared_solutions','inventory','compliance'] as const)
      .reduce((s, k) => s + r[k].creates + r[k].updates + r[k].skips, 0);
  }
</script>

<div class="page-header">
  <h1>Import Data</h1>
</div>

<div class="import-layout">
  <!-- ── Upload card ──────────────────────────────────────────────────────── -->
  <div class="card upload-card">
    <div class="card-icon">&#8657;</div>
    <h2>Import from Excel Workbook</h2>
    <p class="desc">
      Select a <code>.xlsx</code> file exported by SteloPTC. The file must contain
      all six sheets — Specimens, Subcultures, Media Batches, Prepared Solutions,
      Inventory, and Compliance — with the original column structure intact.
    </p>
    <p class="desc">
      A dry-run preview is shown first so you can review what will change before
      committing. Imports run inside a transaction and are rolled back automatically
      if any step fails at the database level.
    </p>

    <label class="file-label" for="xlsx-input">
      {fileName || 'Choose .xlsx file…'}
    </label>
    <input
      id="xlsx-input"
      type="file"
      accept=".xlsx,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
      onchange={handleFileChange}
      disabled={busy}
      class="file-input"
    />

    {#if busy}
      <div class="progress-row">
        <span class="spinner"></span>
        <span>{progress}</span>
      </div>
    {/if}

    {#if parseError}
      <div class="error-box">{parseError}</div>
    {/if}
  </div>

  <!-- ── Dry-run preview ─────────────────────────────────────────────────── -->
  {#if preview && !committed}
    <div class="card preview-card">
      <h2>Dry-Run Preview — <span class="fname">{fileName}</span></h2>
      <p class="preview-note">
        No data has been changed yet. Review the counts below, then click
        <strong>Confirm Import</strong> to apply.
      </p>

      <table class="stats-table">
        <thead>
          <tr>
            <th>Sheet</th>
            <th class="num">Creates</th>
            <th class="num">Updates</th>
            <th class="num">Skips</th>
          </tr>
        </thead>
        <tbody>
          {#each Object.entries(SHEET_LABELS) as [key, label]}
            {@const stats = preview[key as keyof typeof SHEET_LABELS]}
            <tr>
              <td>{label}</td>
              <td class="num creates">{stats.creates}</td>
              <td class="num updates">{stats.updates}</td>
              <td class="num skips">{stats.skips}</td>
            </tr>
          {/each}
        </tbody>
      </table>

      {#if preview.errors.length > 0}
        <div class="errors-section">
          <h3>Row Errors ({preview.errors.length})</h3>
          <p class="errors-note">
            Rows with errors are skipped. Correct the file and re-import to include them.
          </p>
          <div class="error-list">
            {#each preview.errors as err}
              <div class="error-row">
                <span class="err-badge">{err.sheet} row {err.row}</span>
                <span class="err-msg">{err.message}</span>
              </div>
            {/each}
          </div>
        </div>
      {/if}

      <div class="action-row">
        <button class="btn btn-secondary" onclick={handleReset} disabled={busy}>
          Cancel
        </button>
        <button class="btn btn-import" onclick={handleCommit} disabled={busy}>
          {#if busy}
            <span class="spinner"></span> {progress}
          {:else}
            &#10003; Confirm Import
          {/if}
        </button>
      </div>
    </div>
  {/if}

  <!-- ── Committed result ─────────────────────────────────────────────────── -->
  {#if committed}
    <div class="card result-card">
      <div class="result-icon">&#10003;</div>
      <h2>Import Complete</h2>
      <p class="result-note">
        All rows were written to the database inside a single transaction.
      </p>

      <table class="stats-table">
        <thead>
          <tr>
            <th>Sheet</th>
            <th class="num">Created</th>
            <th class="num">Updated</th>
            <th class="num">Skipped</th>
          </tr>
        </thead>
        <tbody>
          {#each Object.entries(SHEET_LABELS) as [key, label]}
            {@const stats = committed[key as keyof typeof SHEET_LABELS]}
            <tr>
              <td>{label}</td>
              <td class="num creates">{stats.creates}</td>
              <td class="num updates">{stats.updates}</td>
              <td class="num skips">{stats.skips}</td>
            </tr>
          {/each}
        </tbody>
      </table>

      {#if committed.errors.length > 0}
        <div class="errors-section">
          <h3>Skipped Rows ({committed.errors.length})</h3>
          <div class="error-list">
            {#each committed.errors as err}
              <div class="error-row">
                <span class="err-badge">{err.sheet} row {err.row}</span>
                <span class="err-msg">{err.message}</span>
              </div>
            {/each}
          </div>
        </div>
      {/if}

      <button class="btn btn-secondary" onclick={handleReset}>
        Import another file
      </button>
    </div>
  {/if}
</div>

<style>
  .import-layout {
    display: flex;
    flex-direction: column;
    gap: 20px;
    max-width: 860px;
  }

  .upload-card {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 28px;
    border-color: #2563eb;
    background: linear-gradient(135deg, #eff6ff 0%, #fff 60%);
  }

  :global(.dark) .upload-card {
    background: linear-gradient(135deg, #0c1a3a 0%, #1e293b 60%);
    border-color: #1d4ed8;
  }

  .card-icon { font-size: 32px; }

  h2 { font-size: 18px; font-weight: 700; }

  .desc {
    font-size: 13px;
    color: #6b7280;
    line-height: 1.6;
  }

  :global(.dark) .desc { color: #94a3b8; }

  /* File input */
  .file-input {
    position: absolute;
    width: 1px;
    height: 1px;
    opacity: 0;
    pointer-events: none;
  }

  .file-label {
    display: inline-block;
    padding: 10px 18px;
    border: 2px dashed #93c5fd;
    border-radius: 8px;
    font-size: 13px;
    color: #2563eb;
    cursor: pointer;
    background: #fff;
    transition: all 0.15s;
    max-width: 360px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .file-label:hover {
    background: #eff6ff;
    border-color: #2563eb;
  }

  :global(.dark) .file-label {
    background: #0f172a;
    color: #60a5fa;
    border-color: #1d4ed8;
  }

  :global(.dark) .file-label:hover {
    background: #1e293b;
  }

  .progress-row {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 13px;
    color: #6b7280;
  }

  .error-box {
    padding: 10px 14px;
    background: #fef2f2;
    border: 1px solid #fca5a5;
    border-radius: 6px;
    font-size: 13px;
    color: #b91c1c;
  }

  :global(.dark) .error-box {
    background: #2d0707;
    border-color: #7f1d1d;
    color: #fca5a5;
  }

  /* Preview / result cards */
  .preview-card, .result-card {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 28px;
  }

  .preview-card { border-color: #f59e0b; }
  .result-card  { border-color: #16a34a; }

  .fname {
    font-weight: 400;
    font-size: 14px;
    color: #6b7280;
  }

  .preview-note, .result-note {
    font-size: 13px;
    color: #6b7280;
    line-height: 1.5;
  }

  :global(.dark) .preview-note, :global(.dark) .result-note { color: #94a3b8; }

  .result-icon {
    font-size: 36px;
    color: #16a34a;
  }

  /* Stats table */
  .stats-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }

  .stats-table th, .stats-table td {
    padding: 8px 12px;
    text-align: left;
    border-bottom: 1px solid #e5e7eb;
  }

  :global(.dark) .stats-table th,
  :global(.dark) .stats-table td {
    border-color: #334155;
  }

  .stats-table thead th {
    font-weight: 600;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #6b7280;
  }

  .num { text-align: right; font-variant-numeric: tabular-nums; }
  .creates { color: #16a34a; font-weight: 600; }
  .updates { color: #2563eb; font-weight: 600; }
  .skips   { color: #9ca3af; }

  /* Errors */
  .errors-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .errors-section h3 {
    font-size: 14px;
    font-weight: 600;
    color: #b91c1c;
  }

  :global(.dark) .errors-section h3 { color: #fca5a5; }

  .errors-note {
    font-size: 12px;
    color: #6b7280;
  }

  .error-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: 260px;
    overflow-y: auto;
  }

  .error-row {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    font-size: 12px;
    padding: 6px 10px;
    background: #fef2f2;
    border-radius: 4px;
  }

  :global(.dark) .error-row {
    background: #2d0707;
  }

  .err-badge {
    flex-shrink: 0;
    font-weight: 600;
    color: #b91c1c;
    white-space: nowrap;
  }

  :global(.dark) .err-badge { color: #fca5a5; }

  .err-msg { color: #6b7280; }

  :global(.dark) .err-msg { color: #94a3b8; }

  /* Action row */
  .action-row {
    display: flex;
    gap: 12px;
    align-items: center;
  }

  .btn-import {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 10px 22px;
    background: #16a34a;
    color: white;
    border: none;
    border-radius: 6px;
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.15s;
  }

  .btn-import:hover:not(:disabled) { background: #15803d; }
  .btn-import:disabled { opacity: 0.6; cursor: not-allowed; }

  .spinner {
    display: inline-block;
    width: 14px;
    height: 14px;
    border: 2px solid rgba(255,255,255,0.4);
    border-top-color: white;
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }

  @keyframes spin { to { transform: rotate(360deg); } }
</style>
