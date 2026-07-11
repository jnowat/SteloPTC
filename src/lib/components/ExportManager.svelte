<script lang="ts">
  import * as XLSX from 'xlsx';
  import {
    exportSpecimensCsv,
    exportSpecimensJson,
    listAllSubcultures,
    listMedia,
    listInventory,
    listComplianceRecords,
    listPreparedSolutions,
  } from '../api';
  import { addNotification } from '../stores/app';
  import {
    specimenRows,
    subcultureRows,
    mediaRows,
    inventoryRows,
    complianceRows,
    prepSolutionRows,
  } from '../exportUtils';
  import { datestamp } from '../utils';

  let busy = $state(false);
  let progress = $state('');

  // ── helpers ──────────────────────────────────────────────────────────────────

  function triggerDownload(blob: Blob, filename: string) {
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    a.click();
    // Defer revocation to a later tick — revoking synchronously right after click()
    // can cancel the download before the engine has read the blob.
    setTimeout(() => URL.revokeObjectURL(url), 0);
  }

  // ── CSV / JSON (specimens only) ────────────────────────────────────────────

  async function handleCsvExport() {
    busy = true;
    progress = 'Fetching specimens…';
    try {
      const data = await exportSpecimensCsv();
      triggerDownload(new Blob([data], { type: 'text/csv' }), `specimens_${datestamp()}.csv`);
      addNotification('Specimens exported as CSV', 'success');
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      busy = false;
      progress = '';
    }
  }

  async function handleJsonExport() {
    busy = true;
    progress = 'Fetching specimens…';
    try {
      const data = await exportSpecimensJson();
      triggerDownload(new Blob([data], { type: 'application/json' }), `specimens_${datestamp()}.json`);
      addNotification('Specimens exported as JSON', 'success');
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      busy = false;
      progress = '';
    }
  }

  // ── Excel multi-sheet ──────────────────────────────────────────────────────

  function makeSheet(data: any[][]): XLSX.WorkSheet {
    const ws = XLSX.utils.aoa_to_sheet(data);
    // Bold header row
    if (data.length > 0) {
      const cols = data[0].length;
      for (let c = 0; c < cols; c++) {
        const ref = XLSX.utils.encode_cell({ r: 0, c });
        if (!ws[ref]) ws[ref] = { v: data[0][c] };
        ws[ref].s = { font: { bold: true } };
      }
      // Auto column widths (approximate)
      ws['!cols'] = data[0].map((_: any, ci: number) => {
        const max = data.reduce((w, row) => Math.max(w, String(row[ci] ?? '').length), data[0][ci]?.length ?? 8);
        return { wch: Math.min(max + 2, 50) };
      });
    }
    return ws;
  }

  async function handleExcelExport() {
    busy = true;
    try {
      progress = 'Fetching specimens…';
      const [specimenJson, subcultures, media, inventory, compliance, prepSolutions] =
        await Promise.all([
          exportSpecimensJson(),
          listAllSubcultures().catch(() => []),
          listMedia().catch(() => []),
          listInventory().catch(() => []),
          listComplianceRecords().catch(() => []),
          listPreparedSolutions().catch(() => []),
        ]);

      progress = 'Building workbook…';
      const wb = XLSX.utils.book_new();

      XLSX.utils.book_append_sheet(wb, makeSheet(specimenRows(specimenJson)), 'Specimens');
      XLSX.utils.book_append_sheet(wb, makeSheet(subcultureRows(subcultures)), 'Subcultures');
      XLSX.utils.book_append_sheet(wb, makeSheet(mediaRows(media)), 'Media Batches');
      XLSX.utils.book_append_sheet(wb, makeSheet(prepSolutionRows(prepSolutions)), 'Prepared Solutions');
      XLSX.utils.book_append_sheet(wb, makeSheet(inventoryRows(inventory)), 'Inventory');
      XLSX.utils.book_append_sheet(wb, makeSheet(complianceRows(compliance)), 'Compliance');

      progress = 'Writing file…';
      const buf = XLSX.write(wb, { type: 'array', bookType: 'xlsx' });
      triggerDownload(
        new Blob([buf], { type: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet' }),
        `stelo_export_${datestamp()}.xlsx`,
      );

      addNotification('Excel workbook exported (6 sheets)', 'success');
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      busy = false;
      progress = '';
    }
  }
</script>

<div class="page-header">
  <h1>Export Data</h1>
</div>

<div class="export-grid">
  <!-- Excel -->
  <div class="card export-card featured">
    <div class="card-icon">&#128196;</div>
    <h2>Excel Workbook</h2>
    <p class="desc">
      Multi-sheet <code>.xlsx</code> file containing all active specimens,
      subculture history, media batches, prepared solutions, inventory
      and compliance records — ready for Excel, LibreOffice, or Google Sheets.
    </p>
    <ul class="sheet-list">
      <li>&#x1F33F; Specimens</li>
      <li>&#x1F9EA; Subcultures</li>
      <li>&#x1F9B9; Media Batches</li>
      <li>&#x1F9F4; Prepared Solutions</li>
      <li>&#x1F4E6; Inventory</li>
      <li>&#x1F4CB; Compliance</li>
    </ul>
    <button class="btn btn-excel" onclick={handleExcelExport} disabled={busy}>
      {#if busy && progress}
        <span class="spinner"></span> {progress}
      {:else}
        &#8659; Export .xlsx
      {/if}
    </button>
  </div>

  <!-- CSV -->
  <div class="card export-card">
    <div class="card-icon">&#128196;</div>
    <h2>CSV</h2>
    <p class="desc">
      All active specimens as a flat comma-separated values file.
      Compatible with any spreadsheet application or data pipeline.
    </p>
    <button class="btn btn-primary" onclick={handleCsvExport} disabled={busy}>
      &#8659; Export .csv
    </button>
  </div>

  <!-- JSON -->
  <div class="card export-card">
    <div class="card-icon">&#128196;</div>
    <h2>JSON</h2>
    <p class="desc">
      Specimens in structured JSON format — ideal for scripting,
      database migrations, or importing into other tools.
    </p>
    <button class="btn btn-primary" onclick={handleJsonExport} disabled={busy}>
      &#8659; Export .json
    </button>
  </div>
</div>

<style>
  .export-grid {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    gap: 20px;
    align-items: start;
  }

  @media (max-width: 1024px) {
    .export-grid { grid-template-columns: 1fr; }
  }

  .export-card {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 28px;
  }

  .export-card.featured {
    border-color: #16a34a;
    background: linear-gradient(135deg, #f0fdf4 0%, #fff 60%);
    grid-column: 1 / -1;
  }

  :global(.dark) .export-card.featured {
    background: linear-gradient(135deg, #052e16 0%, #1e293b 60%);
    border-color: #166534;
  }

  .card-icon { font-size: 32px; }

  h2 { font-size: 18px; font-weight: 700; }

  .desc {
    font-size: 13px;
    color: #6b7280;
    line-height: 1.6;
  }

  :global(.dark) .desc { color: #94a3b8; }

  .sheet-list {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    list-style: none;
    padding: 0;
    margin: 0;
  }

  .sheet-list li {
    padding: 4px 12px;
    background: #dcfce7;
    color: #166534;
    border-radius: 20px;
    font-size: 12px;
    font-weight: 600;
  }

  :global(.dark) .sheet-list li {
    background: #166534;
    color: #dcfce7;
  }

  .btn-excel {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 10px 20px;
    background: #16a34a;
    color: white;
    border: none;
    border-radius: 6px;
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.15s;
    align-self: flex-start;
  }

  .btn-excel:hover:not(:disabled) { background: #15803d; }
  .btn-excel:disabled { opacity: 0.6; cursor: not-allowed; }

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
