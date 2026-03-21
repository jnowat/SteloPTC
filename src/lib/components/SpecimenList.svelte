<script lang="ts">
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import {
    listSpecimens, searchSpecimens, deleteSpecimen, listSpecies,
    bulkArchiveSpecimens, bulkUpdateLocation, bulkUpdateStage,
  } from '../api';
  import { navigateTo, addNotification } from '../stores/app';
  import { selectedSpecimenId } from '../stores/app';
  import { currentUser } from '../stores/auth';
  import SpecimenForm from './SpecimenForm.svelte';
  import QrModal from './QrModal.svelte';
  import QrScanner from './QrScanner.svelte';
  import Tooltip from './Tooltip.svelte';

  let specimens = $state<any[]>([]);
  let species = $state<any[]>([]);
  let total = $state(0);
  let page = $state(1);
  let perPage = $state(50);
  let totalPages = $state(0);
  let loading = $state(true);
  let searchQuery = $state('');
  let filterSpecies = $state('');
  let filterStage = $state('');
  let showForm = $state(false);
  let qrSpecimen = $state<any>(null);
  let showScanner = $state(false);

  // Batch selection
  let selectedIds = $state(new Set<string>());
  let batchAction = $state<'location' | 'stage' | null>(null);
  let batchLoading = $state(false);
  // Batch location
  const rooms  = ['1','2','3','4','5'];
  const racks  = ['A','B','C','D'];
  const shelves = ['1','2','3','4','5'];
  const trays  = ['A','B','C','D','E','F'];
  let batchRoom = $state('');
  let batchRack = $state('');
  let batchShelf = $state('');
  let batchTray = $state('');
  // Batch stage
  let batchStage = $state('');

  const stages = [
    { value: 'explant',        label: 'Explant' },
    { value: 'callus',         label: 'Callus' },
    { value: 'suspension',     label: 'Suspension' },
    { value: 'protoplast',     label: 'Protoplast' },
    { value: 'shoot',          label: 'Shoot' },
    { value: 'shoot_meristem', label: 'Shoot Meristem' },
    { value: 'apical_meristem',label: 'Apical Meristem' },
    { value: 'root',           label: 'Root' },
    { value: 'root_meristem',  label: 'Root Meristem' },
    { value: 'embryogenic',    label: 'Embryogenic' },
    { value: 'plantlet',       label: 'Plantlet' },
    { value: 'acclimatized',   label: 'Acclimatized' },
    { value: 'stock',          label: 'Stock' },
  ];

  let allPageSelected = $derived(
    specimens.length > 0 && specimens.every(s => selectedIds.has(s.id))
  );
  let someSelected = $derived(
    specimens.some(s => selectedIds.has(s.id)) && !allPageSelected
  );

  onMount(() => {
    load();
    loadSpecies();
  });

  async function loadSpecies() {
    try { species = await listSpecies(); } catch (_e) {}
  }

  async function load() {
    loading = true;
    try {
      let result;
      if (searchQuery || filterSpecies || filterStage) {
        result = await searchSpecimens({
          query: searchQuery || undefined,
          species_id: filterSpecies || undefined,
          stage: filterStage || undefined,
          page,
          per_page: perPage,
        });
      } else {
        result = await listSpecimens(page, perPage);
      }
      specimens = result.items;
      total = result.total;
      totalPages = result.total_pages;
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      loading = false;
    }
  }

  function openDetail(id: string) {
    selectedSpecimenId.set(id);
    navigateTo('specimen-detail', id);
  }

  async function handleDelete(id: string) {
    if (!confirm('Archive this specimen? It can still be found in searches.')) return;
    try {
      await deleteSpecimen(id);
      addNotification('Specimen archived', 'success');
      load();
    } catch (e: any) {
      addNotification(e.message, 'error');
    }
  }

  function handleFormDone() {
    showForm = false;
    load();
  }

  function handleSearch() {
    page = 1;
    load();
  }

  function openQr(e: MouseEvent, s: any) {
    e.stopPropagation();
    qrSpecimen = s;
  }

  // ── Batch selection helpers ────────────────────────────────────
  function toggleSelect(e: MouseEvent, id: string) {
    e.stopPropagation();
    const next = new Set(selectedIds);
    if (next.has(id)) next.delete(id); else next.add(id);
    selectedIds = next;
  }

  function toggleSelectAll(e: MouseEvent) {
    e.stopPropagation();
    if (allPageSelected) {
      // deselect all on page
      const next = new Set(selectedIds);
      for (const s of specimens) next.delete(s.id);
      selectedIds = next;
    } else {
      // select all on page
      const next = new Set(selectedIds);
      for (const s of specimens) next.add(s.id);
      selectedIds = next;
    }
  }

  function clearSelection() {
    selectedIds = new Set();
    batchAction = null;
  }

  function openBatchAction(action: 'location' | 'stage') {
    batchAction = batchAction === action ? null : action;
  }

  function composeBatchLocation(): string {
    const parts: string[] = [];
    if (batchRoom)  parts.push(`Room ${batchRoom}`);
    if (batchRack)  parts.push(`Rack ${batchRack}`);
    if (batchShelf) parts.push(`Shelf ${batchShelf}`);
    if (batchTray)  parts.push(`Tray ${batchTray}`);
    return parts.join(' / ');
  }

  async function executeBatchLocation() {
    const location = composeBatchLocation();
    if (!location) { addNotification('Select at least one location component', 'warning'); return; }
    batchLoading = true;
    try {
      const ids = Array.from(selectedIds);
      const n = await bulkUpdateLocation(ids, location);
      addNotification(`Location updated for ${n} specimen${n !== 1 ? 's' : ''}`, 'success');
      clearSelection();
      batchRoom = batchRack = batchShelf = batchTray = '';
      load();
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      batchLoading = false;
    }
  }

  async function executeBatchStage() {
    if (!batchStage) { addNotification('Select a stage', 'warning'); return; }
    batchLoading = true;
    try {
      const ids = Array.from(selectedIds);
      const n = await bulkUpdateStage(ids, batchStage);
      addNotification(`Stage updated for ${n} specimen${n !== 1 ? 's' : ''}`, 'success');
      clearSelection();
      batchStage = '';
      load();
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      batchLoading = false;
    }
  }

  async function executeBatchArchive() {
    const n = selectedIds.size;
    if (!confirm(`Archive ${n} specimen${n !== 1 ? 's' : ''}? They can still be found in searches.`)) return;
    batchLoading = true;
    try {
      const ids = Array.from(selectedIds);
      const archived = await bulkArchiveSpecimens(ids);
      addNotification(`${archived} specimen${archived !== 1 ? 's' : ''} archived`, 'success');
      clearSelection();
      load();
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      batchLoading = false;
    }
  }

  function printSummaryReport() {
    const user = get(currentUser);
    const username = (user as any)?.display_name || (user as any)?.username || 'Unknown';
    const reportDate = new Date().toISOString().split('T')[0];

    const esc = (s: any) => String(s ?? '').replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;') || '—';
    const healthLabel = (val: any) => {
      if (val === null || val === undefined || val === '' || isNaN(Number(val))) return '—';
      const n = Math.round(Number(val));
      if (n === -1) return '?';
      return ['0-Dead','1-Poor','2-Fair','3-Good','4-Healthy'][Math.max(0,Math.min(4,n))];
    };
    const stageFmt = (s: string) => s?.replace(/_/g,' ').replace(/\b\w/g,c=>c.toUpperCase()) || '—';

    // Build filter description
    const filterParts: string[] = [];
    if (searchQuery) filterParts.push(`Search: "${searchQuery}"`);
    if (filterStage) filterParts.push(`Stage: ${stageFmt(filterStage)}`);
    if (filterSpecies) {
      const sp = species.find((s: any) => s.id === filterSpecies);
      if (sp) filterParts.push(`Species: ${sp.species_code}`);
    }
    const filterLine = filterParts.length > 0
      ? `<div class="filter-line">Filters: ${filterParts.join(' · ')}</div>`
      : '<div class="filter-line">Showing all active specimens</div>';

    const rows = specimens.map((s: any) => `<tr>
      <td><b>${esc(s.accession_number)}</b></td>
      <td>${esc(s.species_code)}</td>
      <td>${stageFmt(s.stage)}</td>
      <td>${esc(s.location)}</td>
      <td class="ctr">${esc(s.subculture_count)}</td>
      <td>${healthLabel(s.health_status)}</td>
      <td>${s.quarantine_flag ? '<span class="b-red">Quarantine</span>' : '<span class="b-green">Active</span>'}</td>
      <td>${esc(s.initiation_date)}</td>
    </tr>`).join('');

    const win = window.open('', '_blank', 'width=1050,height=900');
    if (!win) return;
    win.document.write(`<!DOCTYPE html><html><head><meta charset="UTF-8">
<title>Specimens Summary – ${reportDate}</title>
<style>
*{margin:0;padding:0;box-sizing:border-box}
body{font-family:-apple-system,'Segoe UI',Arial,sans-serif;font-size:11px;color:#0f172a;background:#fff;padding:.45in}
@page{size:letter landscape;margin:.45in}
.hdr{border-bottom:2px solid #0f172a;padding-bottom:10px;margin-bottom:12px;display:flex;justify-content:space-between;align-items:flex-end}
.brand{font-size:20px;font-weight:900;letter-spacing:-.5px}
.rpt{font-size:12px;color:#475569;margin-top:2px}
.meta{text-align:right;font-size:10px;color:#64748b;line-height:1.7}
.filter-line{font-size:10px;color:#475569;margin-bottom:10px;font-style:italic}
.summary{font-size:11px;font-weight:600;margin-bottom:8px;color:#0f172a}
table{width:100%;border-collapse:collapse;font-size:10px}
th{background:#0f172a;color:#e2e8f0;font-weight:700;text-align:left;padding:6px 8px;white-space:nowrap}
td{padding:4px 8px;border-bottom:1px solid #e2e8f0;vertical-align:top}
tr:nth-child(even) td{background:#f8fafc}
.ctr{text-align:center}
.b-red{background:#fee2e2;color:#991b1b;padding:1px 5px;border-radius:3px;font-size:9px;font-weight:600}
.b-green{background:#dcfce7;color:#166534;padding:1px 5px;border-radius:3px;font-size:9px;font-weight:600}
.footer{margin-top:14px;border-top:1px solid #e2e8f0;padding-top:8px;display:flex;justify-content:space-between;font-size:9px;color:#94a3b8}
</style></head><body>
<div class="hdr">
  <div><div class="brand">SteloPTC</div><div class="rpt">Specimens Summary Report</div></div>
  <div class="meta"><div>Generated: ${reportDate}</div><div>By: ${esc(username)}</div><div>Page 1 of ${totalPages || 1}</div></div>
</div>
${filterLine}
<div class="summary">Showing ${specimens.length} of ${total} total active specimens (page ${page} of ${totalPages || 1})</div>
<table>
  <thead><tr>
    <th>Accession</th><th>Species</th><th>Stage</th><th>Location</th>
    <th>Passages</th><th>Health</th><th>Status</th><th>Initiated</th>
  </tr></thead>
  <tbody>${rows}</tbody>
</table>
<div class="footer">
  <span>SteloPTC · Tissue Culture Management System</span>
  <span>Generated ${reportDate}</span>
</div>
<script>window.onload=function(){window.print();}<\/script>
</body></html>`);
    win.document.close();
  }
</script>

<div>
  <div class="page-header">
    <h1>Specimens ({total})</h1>
    <div class="header-actions">
      <button class="btn btn-scan" onclick={() => (showScanner = true)}>
        &#128247; Scan QR <Tooltip text="Open camera to scan a QR code label and jump directly to the matching specimen" position="bottom" />
      </button>
      <button class="btn btn-sm btn-print-summary" onclick={printSummaryReport} title="Print a summary report of the currently visible specimens">&#128438; Print Summary <Tooltip text="Print a formatted summary table of the current specimen list view — respects active filters" position="bottom" /></button>
      {#if $currentUser?.role !== 'guest'}
        <button class="btn btn-primary" onclick={() => (showForm = true)}>+ New Specimen <Tooltip text="Register a new tissue culture specimen — auto-generates an accession number on save" position="bottom" /></button>
      {/if}
    </div>
  </div>

  <div class="filters card" style="margin-bottom:16px;">
    <div class="form-row-3">
      <div>
        <input type="text" placeholder="Search accession, notes, location..." bind:value={searchQuery} onkeydown={(e) => e.key === 'Enter' && handleSearch()} title="Search by accession number, notes, or storage location — press Enter to apply" />
      </div>
      <div>
        <select bind:value={filterSpecies} onchange={handleSearch} title="Filter specimens by species">
          <option value="">All species</option>
          {#each species as sp}
            <option value={sp.id}>{sp.species_code} - {sp.genus} {sp.species_name}</option>
          {/each}
        </select>
      </div>
      <div style="display:flex;gap:8px;">
        <select bind:value={filterStage} onchange={handleSearch} title="Filter specimens by development stage">
          <option value="">All stages</option>
          {#each stages as s}
            <option value={s.value}>{s.label}</option>
          {/each}
        </select>
        <button class="btn btn-sm" onclick={handleSearch} title="Apply current search query and filters">Search</button>
      </div>
    </div>
  </div>

  {#if showForm}
    <div class="card" style="margin-bottom:16px;">
      <SpecimenForm onclose={() => (showForm = false)} onsave={handleFormDone} />
    </div>
  {/if}

  {#if loading}
    <div class="empty-state">Loading...</div>
  {:else if specimens.length === 0}
    <div class="empty-state">
      <p>No specimens found</p>
      <p style="margin-top:8px;font-size:12px;">Create your first specimen or adjust your filters.</p>
    </div>
  {:else}
    <div class="card table-card">
      <table>
        <thead>
          <tr>
            <th class="check-col" title="Select specimens for bulk actions">
              <input
                type="checkbox"
                style="width:auto;margin:0;"
                checked={allPageSelected}
                indeterminate={someSelected}
                onclick={toggleSelectAll}
                title={allPageSelected ? 'Deselect all on this page' : 'Select all on this page'}
              />
            </th>
            <th title="Unique accession number assigned to this specimen">Accession</th>
            <th title="Scientific species this specimen belongs to">Species</th>
            <th title="Current development stage of the specimen">Stage</th>
            <th title="Physical storage location within the facility">Location</th>
            <th title="Number of subcultures (transfers to fresh media) performed">Passages</th>
            <th title="Current health rating (0 = Dead, 4 = Healthy)">Health</th>
            <th title="Quarantine or active status">Status</th>
            <th title="Date the specimen culture was first initiated">Initiated</th>
            <th class="action-col" title="Row actions: view QR code, archive specimen"></th>
          </tr>
        </thead>
        <tbody>
          {#each specimens as s}
            <tr
              class="clickable"
              class:selected={selectedIds.has(s.id)}
              onclick={() => openDetail(s.id)}
            >
              <td class="check-col" onclick={(e) => toggleSelect(e, s.id)}>
                <input
                  type="checkbox"
                  style="width:auto;margin:0;"
                  checked={selectedIds.has(s.id)}
                  onclick={(e) => toggleSelect(e, s.id)}
                  title="Select this specimen for bulk actions"
                />
              </td>
              <td><strong>{s.accession_number}</strong></td>
              <td>{s.species_code || '—'}</td>
              <td><span class="badge badge-blue" title="Development stage: {s.stage}">{s.stage}</span></td>
              <td>{s.location || '—'}</td>
              <td>{s.subculture_count}</td>
              <td>{s.health_status || '—'}</td>
              <td>
                {#if s.quarantine_flag}
                  <span class="badge badge-red" title="This specimen is under quarantine restrictions">Quarantine</span>
                {:else}
                  <span class="badge badge-green" title="This specimen is active and cleared for normal handling">Active</span>
                {/if}
              </td>
              <td>{s.initiation_date}</td>
              <td class="action-col">
                <div class="row-actions">
                  <button class="btn btn-sm btn-qr" onclick={(e) => openQr(e, s)} title="View and print the QR code label for this specimen">
                    &#9641; QR
                  </button>
                  {#if $currentUser?.role === 'admin' || $currentUser?.role === 'supervisor'}
                    <button class="btn btn-sm btn-danger" onclick={(e) => { e.stopPropagation(); handleDelete(s.id); }} title="Archive this specimen (soft delete — record is preserved)">Archive</button>
                  {/if}
                </div>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

    {#if totalPages > 1}
      <div class="pagination">
        <button class="btn btn-sm" disabled={page <= 1} onclick={() => { page--; load(); }} title="Go to the previous page">Prev</button>
        <span title="Current page position">Page {page} of {totalPages}</span>
        <button class="btn btn-sm" disabled={page >= totalPages} onclick={() => { page++; load(); }} title="Go to the next page">Next</button>
      </div>
    {/if}
  {/if}
</div>

<!-- ── Batch Action Bar ─────────────────────────────────────────── -->
{#if selectedIds.size > 0}
  <div class="batch-bar" role="toolbar" aria-label="Batch actions for selected specimens">
    <span class="batch-count">{selectedIds.size} selected</span>
    <button class="batch-clear" onclick={clearSelection} title="Clear selection">✕</button>
    <div class="batch-divider"></div>

    {#if $currentUser?.role !== 'guest'}
      <button
        class="batch-btn"
        class:active={batchAction === 'location'}
        onclick={() => openBatchAction('location')}
        title="Move all selected specimens to a new location"
        disabled={batchLoading}
      >&#128205; Transfer Location</button>

      <button
        class="batch-btn"
        class:active={batchAction === 'stage'}
        onclick={() => openBatchAction('stage')}
        title="Update the development stage for all selected specimens"
        disabled={batchLoading}
      >&#128260; Update Stage</button>
    {/if}

    {#if $currentUser?.role === 'admin' || $currentUser?.role === 'supervisor'}
      <button
        class="batch-btn batch-btn-danger"
        onclick={executeBatchArchive}
        title="Archive all selected specimens (soft delete — records are preserved)"
        disabled={batchLoading}
      >&#128465; Archive</button>
    {/if}

    {#if batchAction === 'location'}
      <div class="batch-form">
        <select bind:value={batchRoom} title="Room">
          <option value="">Room…</option>
          {#each rooms as r}<option value={r}>{r}</option>{/each}
        </select>
        <select bind:value={batchRack} title="Rack">
          <option value="">Rack…</option>
          {#each racks as r}<option value={r}>{r}</option>{/each}
        </select>
        <select bind:value={batchShelf} title="Shelf">
          <option value="">Shelf…</option>
          {#each shelves as r}<option value={r}>{r}</option>{/each}
        </select>
        <select bind:value={batchTray} title="Tray">
          <option value="">Tray…</option>
          {#each trays as r}<option value={r}>{r}</option>{/each}
        </select>
        <button class="batch-apply" onclick={executeBatchLocation} disabled={batchLoading}>
          {batchLoading ? 'Moving…' : 'Apply'}
        </button>
      </div>
    {:else if batchAction === 'stage'}
      <div class="batch-form">
        <select bind:value={batchStage} title="New development stage for selected specimens">
          <option value="">Stage…</option>
          {#each stages as s}
            <option value={s.value}>{s.label}</option>
          {/each}
        </select>
        <button class="batch-apply" onclick={executeBatchStage} disabled={batchLoading}>
          {batchLoading ? 'Updating…' : 'Apply'}
        </button>
      </div>
    {/if}
  </div>
{/if}

<!-- QR Code Modal -->
{#if qrSpecimen}
  <QrModal specimen={qrSpecimen} onclose={() => (qrSpecimen = null)} />
{/if}

<!-- QR Scanner Modal -->
{#if showScanner}
  <QrScanner onclose={() => (showScanner = false)} />
{/if}

<style>
  .clickable { cursor: pointer; }
  .clickable:hover td { background: #eff6ff !important; }
  :global(.dark) .clickable:hover td { background: #1e3a5f !important; }
  .clickable.selected td { background: #dbeafe !important; }
  :global(.dark) .clickable.selected td { background: #1e3a8f !important; }

  .check-col {
    width: 36px;
    text-align: center;
    padding: 0 8px;
  }

  .header-actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    align-items: center;
  }

  .btn-print-summary {
    background: #f5f3ff;
    color: #5b21b6;
    border-color: #c4b5fd;
  }
  .btn-print-summary:hover { background: #ede9fe; }
  :global(.dark) .btn-print-summary { background: rgba(139,92,246,0.12); color: #a78bfa; border-color: #5b21b6; }

  .btn-scan {
    background: #0f172a;
    color: #94a3b8;
    border-color: #334155;
    font-size: 13px;
    gap: 6px;
  }
  .btn-scan:hover { background: #1e293b; color: #e2e8f0; }
  :global(.dark) .btn-scan { background: #334155; color: #e2e8f0; border-color: #475569; }

  .btn-qr {
    background: #f0fdf4;
    color: #15803d;
    border-color: #86efac;
    font-size: 11px;
    padding: 4px 8px;
    min-height: 32px;
  }
  .btn-qr:hover { background: #dcfce7; }
  :global(.dark) .btn-qr { background: rgba(34,197,94,0.1); color: #4ade80; border-color: #166534; }

  .table-card { overflow-x: auto; }

  .row-actions {
    display: flex;
    gap: 4px;
    align-items: center;
  }

  .action-col { white-space: nowrap; }

  .pagination {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 16px;
    margin-top: 16px;
    font-size: 13px;
  }

  /* ── Batch Action Bar ── */
  .batch-bar {
    position: fixed;
    bottom: 0;
    left: 0;
    right: 0;
    background: #1e293b;
    border-top: 1px solid #334155;
    color: #e2e8f0;
    padding: 10px 20px;
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
    z-index: 500;
    box-shadow: 0 -4px 16px rgba(0,0,0,0.4);
    animation: slideUp 0.15s ease;
  }
  :global(.dark) .batch-bar { background: #0f172a; border-top-color: #1e293b; }

  @keyframes slideUp {
    from { transform: translateY(100%); opacity: 0; }
    to   { transform: translateY(0);    opacity: 1; }
  }

  .batch-count {
    font-size: 13px;
    font-weight: 600;
    color: #94a3b8;
    white-space: nowrap;
  }

  .batch-clear {
    background: none;
    border: none;
    color: #64748b;
    cursor: pointer;
    font-size: 14px;
    padding: 2px 6px;
    border-radius: 4px;
    line-height: 1;
    transition: color 0.1s;
  }
  .batch-clear:hover { color: #e2e8f0; background: #334155; }

  .batch-divider {
    width: 1px;
    height: 24px;
    background: #334155;
    margin: 0 4px;
  }

  .batch-btn {
    background: #334155;
    color: #e2e8f0;
    border: 1px solid #475569;
    border-radius: 6px;
    padding: 6px 14px;
    font-size: 12px;
    cursor: pointer;
    transition: background 0.1s;
    white-space: nowrap;
  }
  .batch-btn:hover:not(:disabled) { background: #475569; }
  .batch-btn.active { background: #1d4ed8; border-color: #3b82f6; color: #fff; }
  .batch-btn-danger { background: #7f1d1d; border-color: #991b1b; }
  .batch-btn-danger:hover:not(:disabled) { background: #991b1b; }
  .batch-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  .batch-form {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }
  .batch-form select {
    background: #334155;
    color: #e2e8f0;
    border: 1px solid #475569;
    border-radius: 6px;
    padding: 5px 10px;
    font-size: 12px;
    min-height: 32px;
  }

  .batch-apply {
    background: #1d4ed8;
    color: #fff;
    border: 1px solid #2563eb;
    border-radius: 6px;
    padding: 6px 16px;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.1s;
    white-space: nowrap;
  }
  .batch-apply:hover:not(:disabled) { background: #2563eb; }
  .batch-apply:disabled { opacity: 0.5; cursor: not-allowed; }

  /* ── Mobile ── */
  @media (max-width: 768px) {
    .header-actions { gap: 6px; }
    .btn-scan { min-height: 44px; font-size: 14px; }
    .btn-qr   { min-height: 40px; font-size: 12px; }
    .batch-bar { padding: 10px 12px; gap: 6px; }
    .batch-btn { padding: 8px 12px; font-size: 13px; min-height: 40px; }
    .batch-form select { min-height: 40px; font-size: 13px; }
    .batch-apply { min-height: 40px; font-size: 13px; }
  }
</style>
