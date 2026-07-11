<script lang="ts">
  import { onMount } from 'svelte';
  import { getAuditLog, verifyAuditEntry, verifyAuditLineage,
           createAuditCheckpoint, verifyAgainstCheckpoint, listAuditCheckpoints,
           exportAuditProof, verifyExportedProof,
           getAutoCheckpointConfig, setAutoCheckpointConfig, runAutoCheckpoint,
           listAuditEntriesCursor } from '../api';
  import { addNotification } from '../stores/app';
  import DataState from './DataState.svelte';
  import OnChainAnchorPanel from './OnChainAnchorPanel.svelte';
  import SignedLedgerPanel from './SignedLedgerPanel.svelte';
  import SpecimenPassportPanel from './SpecimenPassportPanel.svelte';

  let entries = $state<any[]>([]);
  let total = $state(0);
  let page = $state(1);
  let totalPages = $state(0);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let filterEntity = $state('');
  let filterAction = $state('');

  // Per-row verification state: entry_id → { pending, ok, message }
  let rowVerify = $state<Record<string, { pending: boolean; ok?: boolean; message?: string }>>({});

  // Clipboard copy state: unique key → true while "copied" is shown
  let copiedId = $state<string | null>(null);

  // Batch verification state (Verify All Lineages button)
  let batchVerifying = $state(false);
  let batchResult = $state<{ total: number; passed: number; failed: number } | null>(null);

  // WP-63: cursor-paginated single-lineage detail panel. At 1M+ entries per
  // lineage, loading the whole chain at once (as any naive "view lineage"
  // feature would) is prohibitively slow — this loads a fixed-size window
  // oldest-first using chain_seq as the stable cursor, with an explicit
  // "Load more" control rather than an unbounded fetch.
  let viewingLineageId = $state<string | null>(null);
  let lineageEntries = $state<any[]>([]);
  let lineageHasMore = $state(false);
  let lineageNextCursor = $state<number | null>(null);
  let lineageLoadingMore = $state(false);
  const LINEAGE_PAGE_SIZE = 50;

  async function openLineageView(lineageId: string) {
    viewingLineageId = lineageId;
    lineageEntries = [];
    lineageHasMore = false;
    lineageNextCursor = null;
    await loadLineagePage(null);
  }

  function closeLineageView() {
    viewingLineageId = null;
    lineageEntries = [];
  }

  async function loadLineagePage(afterSeq: number | null) {
    if (!viewingLineageId) return;
    lineageLoadingMore = true;
    try {
      const result = await listAuditEntriesCursor(viewingLineageId, afterSeq, LINEAGE_PAGE_SIZE);
      lineageEntries = afterSeq === null ? result.items : [...lineageEntries, ...result.items];
      lineageHasMore = result.has_more;
      lineageNextCursor = result.next_cursor;
    } catch (e: any) {
      addNotification(e?.message || 'Failed to load lineage entries', 'error');
    } finally {
      lineageLoadingMore = false;
    }
  }

  onMount(() => { load(); });

  async function load() {
    loading = true;
    error = null;
    rowVerify = {};
    batchResult = null;
    try {
      const result = await getAuditLog({
        entity_type: filterEntity || undefined,
        action: filterAction || undefined,
        page,
        per_page: 50,
      });
      entries = result.items;
      total = result.total;
      totalPages = result.total_pages;
    } catch (e: any) {
      error = e.message;
      addNotification(e.message, 'error');
    } finally { loading = false; }
  }

  function trunc(val: string | null | undefined, len = 10): string {
    if (!val) return '—';
    return val.length > len ? val.slice(0, len) + '…' : val;
  }

  async function copyHash(val: string, key: string) {
    try {
      await navigator.clipboard.writeText(val);
      copiedId = key;
      setTimeout(() => { copiedId = null; }, 1500);
    } catch { /* clipboard unavailable */ }
  }

  async function verifyRow(entry: any) {
    rowVerify[entry.id] = { pending: true };
    try {
      const result = await verifyAuditEntry(entry.id);
      rowVerify[entry.id] = { pending: false, ok: result.ok, message: result.message };
    } catch (e: any) {
      rowVerify[entry.id] = { pending: false, ok: false, message: e.message };
    }
  }

  async function verifyLineage(entry: any) {
    if (!entry.lineage_id) return;
    rowVerify[entry.id] = { pending: true };
    try {
      const result = await verifyAuditLineage(entry.lineage_id);
      // result.message already contains the seq number and failure reason;
      // avoid duplicating it in a "Chain break at seq N: Chain broken at seq N" prefix.
      const msg = result.ok
        ? `All ${result.checked} entries verified — chain is intact.`
        : result.message;
      rowVerify[entry.id] = { pending: false, ok: result.ok, message: msg };
    } catch (e: any) {
      rowVerify[entry.id] = { pending: false, ok: false, message: e.message };
    }
  }

  async function verifyAllLineages() {
    const uniqueLineages = [...new Set(
      entries.filter((e: any) => e.lineage_id).map((e: any) => e.lineage_id as string)
    )];
    if (uniqueLineages.length === 0) return;

    batchVerifying = true;
    batchResult = null;
    let passed = 0, failed = 0;

    for (const lineageId of uniqueLineages) {
      const firstEntry = entries.find((e: any) => e.lineage_id === lineageId);
      if (!firstEntry) continue;

      rowVerify[firstEntry.id] = { pending: true };
      try {
        const result = await verifyAuditLineage(lineageId);
        const msg = result.ok
          ? `All ${result.checked} entries verified — chain is intact.`
          : result.message;
        rowVerify[firstEntry.id] = { pending: false, ok: result.ok, message: msg };
        if (result.ok) passed++; else failed++;
      } catch (e: any) {
        rowVerify[firstEntry.id] = { pending: false, ok: false, message: e.message };
        failed++;
      }
    }

    batchResult = { total: uniqueLineages.length, passed, failed };
    batchVerifying = false;
  }

  // Counts of chained vs legacy rows on the current page
  let chainedCount = $derived(entries.filter((e: any) => e.chain_seq != null).length);
  let legacyCount = $derived(entries.length - chainedCount);

  // --- Checkpoint state ---
  let showCheckpoints = $state(false);
  let checkpoints = $state<any[]>([]);
  let checkpointsLoading = $state(false);
  let newCpLineage = $state('');
  let newCpStartSeq = $state<number | null>(null);
  let newCpEndSeq = $state<number | null>(null);
  let cpCreating = $state(false);
  let cpVerifyState = $state<Record<string, { pending: boolean; ok?: boolean; message?: string }>>({});

  // Unique lineages on current page (for the create-checkpoint dropdown)
  let pageLineages = $derived(
    entries
      .filter((e: any) => e.lineage_id)
      .map((e: any) => ({
        lineage_id: e.lineage_id as string,
        label: `${e.entity_type} (${(e.lineage_id as string).slice(0, 8)}…)`,
      }))
      .filter((v, i, a) => a.findIndex((x: any) => x.lineage_id === v.lineage_id) === i)
  );

  async function toggleCheckpoints() {
    showCheckpoints = !showCheckpoints;
    if (showCheckpoints && checkpoints.length === 0) {
      await reloadCheckpoints();
    }
  }

  async function reloadCheckpoints() {
    checkpointsLoading = true;
    try {
      checkpoints = await listAuditCheckpoints();
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      checkpointsLoading = false;
    }
  }

  async function doCreateCheckpoint() {
    if (!newCpLineage) return;
    cpCreating = true;
    try {
      const result = await createAuditCheckpoint(
        newCpLineage,
        newCpStartSeq ?? undefined,
        newCpEndSeq ?? undefined,
      );
      const rootSnippet = result.merkle_root.slice(0, 12) + '…';
      addNotification(
        `Checkpoint ${result.checkpoint_id.slice(0, 8)}… created — ${result.entry_count} entr${result.entry_count === 1 ? 'y' : 'ies'}, seq ${result.start_seq}–${result.end_seq}, root ${rootSnippet}`,
        'success',
      );
      newCpLineage = '';
      newCpStartSeq = null;
      newCpEndSeq = null;
      await reloadCheckpoints();
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      cpCreating = false;
    }
  }

  async function doVerifyCheckpoint(cp: any) {
    cpVerifyState[cp.id] = { pending: true };
    try {
      const result = await verifyAgainstCheckpoint(cp.id);
      cpVerifyState[cp.id] = { pending: false, ok: result.ok, message: result.message };
      if (!result.ok) {
        addNotification(`Checkpoint ${cp.id.slice(0, 8)}…: ${result.message}`, 'error');
      }
    } catch (e: any) {
      cpVerifyState[cp.id] = { pending: false, ok: false, message: e.message };
      addNotification(e.message, 'error');
    }
  }

  // WP-21: Export proof (download as JSON file)
  let cpExporting = $state<Record<string, boolean>>({});

  async function doExportProof(cp: any) {
    cpExporting[cp.id] = true;
    try {
      const proofJson = await exportAuditProof(cp.id);
      const blob = new Blob([proofJson], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `merkle-proof-${cp.id.slice(0, 8)}.json`;
      a.click();
      setTimeout(() => URL.revokeObjectURL(url), 5000);
      addNotification(`Proof for checkpoint ${cp.id.slice(0, 8)}… downloaded.`, 'success');
    } catch (e: any) {
      addNotification(`Export failed: ${e.message}`, 'error');
    } finally {
      cpExporting[cp.id] = false;
    }
  }

  // WP-21: Imported proof verification panel
  let showProofVerify = $state(false);
  let proofPasteText = $state('');
  let proofVerifyResult = $state<any | null>(null);
  let proofVerifying = $state(false);

  async function doVerifyImportedProof() {
    if (!proofPasteText.trim()) return;
    proofVerifying = true;
    proofVerifyResult = null;
    try {
      proofVerifyResult = await verifyExportedProof(proofPasteText.trim());
    } catch (e: any) {
      proofVerifyResult = { ok: false, message: e.message };
    } finally {
      proofVerifying = false;
    }
  }

  // WP-21: Auto-checkpoint config
  let autoConfig = $state<{ enabled: boolean; interval: number; on_backup: boolean } | null>(null);
  let autoConfigSaving = $state(false);
  let autoRunning = $state(false);
  let autoRunResult = $state<any | null>(null);

  async function loadAutoConfig() {
    try {
      autoConfig = await getAutoCheckpointConfig();
    } catch { /* non-fatal */ }
  }

  async function saveAutoConfig() {
    if (!autoConfig) return;
    autoConfigSaving = true;
    try {
      await setAutoCheckpointConfig(autoConfig);
      addNotification('Auto-checkpoint settings saved.', 'success');
    } catch (e: any) {
      addNotification(`Save failed: ${e.message}`, 'error');
    } finally {
      autoConfigSaving = false;
    }
  }

  async function doRunAutoCheckpoint() {
    autoRunning = true;
    autoRunResult = null;
    try {
      autoRunResult = await runAutoCheckpoint();
      const n = autoRunResult.checkpoints_created;
      addNotification(
        n > 0
          ? `Auto-checkpoint: created ${n} checkpoint${n !== 1 ? 's' : ''}.`
          : 'Auto-checkpoint: no lineages met the threshold.',
        n > 0 ? 'success' : 'info',
      );
      if (n > 0) await reloadCheckpoints();
    } catch (e: any) {
      addNotification(`Auto-checkpoint failed: ${e.message}`, 'error');
    } finally {
      autoRunning = false;
    }
  }

  // Load auto-config whenever checkpoints panel opens
  $effect(() => {
    if (showCheckpoints && autoConfig === null) {
      loadAutoConfig();
    }
  });
</script>

<div>
  <div class="page-header">
    <h1>Audit Log ({total})</h1>
  </div>

  <!-- Chain integrity summary banner -->
  {#if !loading && entries.length > 0}
    <div class="chain-banner">
      {#if chainedCount > 0}
        <span class="chain-banner-icon">🔒</span>
        <span>
          <strong>{chainedCount}</strong> of {entries.length} visible entries are hash-chained.
          Use <strong>Row</strong> (single hash check) or <strong>Chain</strong> (full lineage walk) on any row,
          or verify all visible lineages at once below.
        </span>
        {#if legacyCount > 0}
          <span class="legacy-note">({legacyCount} legacy row{legacyCount !== 1 ? 's' : ''} not in count)</span>
        {/if}
        <div class="chain-banner-actions">
          <button
            class="btn btn-sm"
            disabled={batchVerifying}
            onclick={verifyAllLineages}
            title="Walk the full hash chain for every unique lineage visible on this page"
          >{batchVerifying ? 'Verifying…' : 'Verify All Lineages'}</button>
          <button
            class="btn btn-sm"
            onclick={toggleCheckpoints}
            title="Create or verify Merkle checkpoints over audit history"
          >{showCheckpoints ? 'Hide Checkpoints' : 'Checkpoints'}</button>
          {#if batchResult}
            {#if batchResult.failed === 0}
              <span class="batch-ok">✓ All {batchResult.passed} lineage{batchResult.passed !== 1 ? 's' : ''} intact</span>
            {:else}
              <span class="batch-fail">✗ {batchResult.failed} of {batchResult.total} lineage{batchResult.total !== 1 ? 's' : ''} failed</span>
            {/if}
            <button class="dismiss-btn" onclick={() => { batchResult = null; }} title="Dismiss batch result">×</button>
          {/if}
        </div>
      {:else}
        <span class="chain-banner-icon chain-banner-icon--legacy">📋</span>
        <span>All visible entries are legacy (pre-v1.5.0) — no chain data to verify.</span>
      {/if}
    </div>
  {/if}

  <!-- Merkle Checkpoint Panel -->
  {#if showCheckpoints}
    <div class="cp-panel">
      <div class="cp-panel-header">
        <strong>Merkle Checkpoints</strong>
        <span class="cp-panel-hint">
          A checkpoint seals a range of a lineage's audit chain into a single Merkle root.
          Verify it later to confirm history has not changed.
          See <code>docs/merkle-checkpoints.md</code> for the full specification.
        </span>
      </div>

      <!-- Create checkpoint form -->
      <div class="cp-create-row">
        <select
          bind:value={newCpLineage}
          title="Choose the lineage to checkpoint (from entries visible on this page)"
          class="cp-select"
        >
          <option value="">— select lineage —</option>
          {#each pageLineages as pl}
            <option value={pl.lineage_id}>{pl.label}</option>
          {/each}
        </select>
        <input
          type="number"
          bind:value={newCpStartSeq}
          placeholder="Start seq (optional)"
          title="First chain_seq to include — leave blank for the earliest entry"
          class="cp-seq-input"
          min="1"
        />
        <input
          type="number"
          bind:value={newCpEndSeq}
          placeholder="End seq (optional)"
          title="Last chain_seq to include — leave blank for the latest entry"
          class="cp-seq-input"
          min="1"
        />
        <button
          class="btn btn-sm"
          disabled={!newCpLineage || cpCreating}
          onclick={doCreateCheckpoint}
          title="Build a Merkle tree over the selected range and store the root"
        >{cpCreating ? 'Creating…' : 'Create Checkpoint'}</button>
      </div>

      <!-- Checkpoint list -->
      {#if checkpointsLoading}
        <p class="cp-empty">Loading checkpoints…</p>
      {:else if checkpoints.length === 0}
        <p class="cp-empty">No checkpoints yet — create one above to seal the current audit history.</p>
      {:else}
        <div style="overflow-x:auto;">
          <table class="cp-table">
            <thead>
              <tr>
                <th>ID</th>
                <th>Lineage</th>
                <th>Seq range</th>
                <th title="Number of audit entries sealed in this checkpoint">Entries</th>
                <th>Merkle root</th>
                <th>Created</th>
                <th>Verify</th>
                <th title="Export a portable Merkle proof JSON for offline verification">Proof</th>
              </tr>
            </thead>
            <tbody>
              {#each checkpoints as cp}
                {@const cv = cpVerifyState[cp.id]}
                <tr>
                  <td>
                    <code class="mono-sm">{cp.id.slice(0, 8)}…</code>
                    {#if cp.is_auto}
                      <span class="auto-badge" title="Created automatically — source: {cp.auto_source ?? 'unknown'}">Auto</span>
                    {/if}
                  </td>
                  <td><code class="mono-sm">{cp.lineage_id.slice(0, 8)}…</code></td>
                  <td class="nowrap">{cp.start_seq}–{cp.end_seq}</td>
                  <td>{cp.entry_count}</td>
                  <td>
                    <button
                      class="hash-btn"
                      title="Merkle root — click to copy:\n{cp.merkle_root}"
                      onclick={() => copyHash(cp.merkle_root, `cp-${cp.id}`)}
                    ><code>{copiedId === `cp-${cp.id}` ? '✓ copied' : trunc(cp.merkle_root)}</code></button>
                  </td>
                  <td class="nowrap">{cp.created_at.slice(0, 10)}</td>
                  <td class="nowrap">
                    {#if cv?.pending}
                      <span class="verify-pending">…</span>
                    {:else if cv?.ok === true}
                      <span class="verify-ok" title={cv.message}>✓ OK</span>
                      <button class="dismiss-btn" onclick={() => { const s = {...cpVerifyState}; delete s[cp.id]; cpVerifyState = s; }}>×</button>
                    {:else if cv?.ok === false}
                      <span class="verify-fail" title={cv.message}>✗ Fail</span>
                      <button class="dismiss-btn" onclick={() => { const s = {...cpVerifyState}; delete s[cp.id]; cpVerifyState = s; }}>×</button>
                    {:else}
                      <button class="btn btn-sm" onclick={() => doVerifyCheckpoint(cp)} title="Verify this checkpoint against the current chain state">Verify</button>
                    {/if}
                  </td>
                  <td class="nowrap">
                    <button
                      class="btn btn-sm"
                      disabled={cpExporting[cp.id]}
                      onclick={() => doExportProof(cp)}
                      title="Download a self-contained Merkle proof JSON for this checkpoint"
                    >{cpExporting[cp.id] ? '…' : 'Export'}</button>
                  </td>
                </tr>
                {#if cv && !cv.pending && cv.message}
                  <tr class="verify-detail-row">
                    <td colspan="8">
                      <span class={cv.ok ? 'verify-detail-ok' : 'verify-detail-fail'}>
                        {cv.ok ? '✓' : '✗'} {cv.message}
                      </span>
                    </td>
                  </tr>
                {/if}
              {/each}
            </tbody>
          </table>
        </div>
      {/if}

      <!-- Proof verification import -->
      <div class="proof-import-panel">
        <div class="proof-import-header">
          <strong>Verify Exported Proof</strong>
          <button class="btn btn-sm" onclick={() => { showProofVerify = !showProofVerify; proofVerifyResult = null; proofPasteText = ''; }}>
            {showProofVerify ? 'Hide' : 'Import &amp; Verify'}
          </button>
        </div>
        {#if showProofVerify}
          <textarea
            class="proof-paste"
            bind:value={proofPasteText}
            placeholder="Paste a merkle-proof-*.json file here…"
            rows="5"
          ></textarea>
          <div class="proof-import-actions">
            <button
              class="btn btn-sm"
              disabled={!proofPasteText.trim() || proofVerifying}
              onclick={doVerifyImportedProof}
            >{proofVerifying ? 'Verifying…' : 'Verify Proof'}</button>
            {#if proofVerifyResult}
              <span class={proofVerifyResult.ok ? 'verify-ok' : 'verify-fail'}>
                {proofVerifyResult.ok ? '✓' : '✗'} {proofVerifyResult.message}
              </span>
            {/if}
          </div>
        {/if}
      </div>

      <!-- Auto-checkpoint configuration -->
      {#if autoConfig}
        <div class="auto-config-panel">
          <strong>Auto-Checkpointing</strong>
          <div class="auto-config-row">
            <label class="auto-config-label">
              <input type="checkbox" bind:checked={autoConfig.enabled} />
              Enabled
            </label>
            <label class="auto-config-label">
              Interval (min uncovered entries):
              <input
                type="number"
                class="auto-config-num"
                bind:value={autoConfig.interval}
                min="0"
                title="0 = checkpoint any lineage with uncovered entries"
              />
            </label>
            <label class="auto-config-label">
              <input type="checkbox" bind:checked={autoConfig.on_backup} />
              Checkpoint before backup
            </label>
            <button class="btn btn-sm" disabled={autoConfigSaving} onclick={saveAutoConfig}>
              {autoConfigSaving ? 'Saving…' : 'Save'}
            </button>
            <button class="btn btn-sm" disabled={autoRunning} onclick={doRunAutoCheckpoint}>
              {autoRunning ? 'Running…' : 'Run Now'}
            </button>
            {#if autoRunResult}
              <span class="auto-run-result">
                {autoRunResult.checkpoints_created} checkpoint{autoRunResult.checkpoints_created !== 1 ? 's' : ''} created
              </span>
            {/if}
          </div>
        </div>
      {/if}

      <!-- On-chain anchoring — Trust Layer Phase 2 (WP-66) -->
      <OnChainAnchorPanel {checkpoints} />
    </div>
  {/if}

  <!-- Signed event ledger — Trust Layer Phase 3 (WP-67) -->
  <SignedLedgerPanel />

  <!-- Specimen passports — federated inter-lab transfer (WP-70) -->
  <SpecimenPassportPanel />

  <div class="card" style="margin-bottom:16px;">
    <div class="form-row-3">
      <div>
        <select title="Filter audit entries by entity type" bind:value={filterEntity} onchange={() => { page = 1; load(); }}>
          <option value="">All entities</option>
          <option value="specimen">Specimens</option>
          <option value="media_batch">Media</option>
          <option value="subculture">Subcultures</option>
          <option value="compliance">Compliance</option>
          <option value="user">Users</option>
          <option value="reminder">Reminders</option>
        </select>
      </div>
      <div>
        <select title="Filter by action" bind:value={filterAction} onchange={() => { page = 1; load(); }}>
          <option value="">All actions</option>
          <option value="create">Create</option>
          <option value="update">Update</option>
          <option value="delete">Delete</option>
          <option value="archive">Archive</option>
          <option value="login">Login</option>
        </select>
      </div>
      <div>
        <button title="Clear all filters" class="btn" onclick={() => { filterEntity = ''; filterAction = ''; page = 1; load(); }}>Reset</button>
      </div>
    </div>
  </div>

  <DataState
    {loading}
    {error}
    empty={entries.length === 0}
    rows={6}
    cols={9}
    emptyIcon="📋"
    emptyTitle="No audit entries found"
    emptyMessage="Audit events will appear here as users create, update, or delete records."
    onretry={load}
  >
    <div class="card" style="overflow-x:auto;">
      <table>
        <thead>
          <tr>
            <th title="Date and time the event was recorded">Timestamp</th>
            <th title="User who performed the action">User</th>
            <th title="Action type: create, update, delete, archive, login">Action</th>
            <th title="Entity type and short ID">Entity</th>
            <th title="Change description or new value">Details</th>
            <th class="chain-th" title="Per-lineage sequence number. 🔒 = chained entry; — = legacy row.">#</th>
            <th class="chain-th" title="SHA-256 of the previous entry in this lineage's chain. Click to copy full hash.">Prev Hash</th>
            <th class="chain-th" title="SHA-256 of this entry's content + prev_hash. Click to copy full hash.">Entry Hash</th>
            <th class="chain-th" title="Verify this entry's hash or its full lineage chain">Verify</th>
          </tr>
        </thead>
        <tbody>
          {#each entries as e}
            {@const chained = e.chain_seq != null}
            {@const rv = rowVerify[e.id]}
            <tr class={chained ? 'row-chained' : 'row-legacy'}>
              <td style="white-space:nowrap;">{e.created_at}</td>
              <td>{e.username || '—'}</td>
              <td><span class="badge badge-blue" title="Action: {e.action}">{e.action}</span></td>
              <td>{e.entity_type}{e.entity_id ? ` (${e.entity_id.slice(0, 8)}…)` : ''}</td>
              <td>{e.details || e.new_value || '—'}</td>

              <!-- Chain seq / lineage badge -->
              <td class="chain-cell">
                {#if chained}
                  <span class="seq-badge" title="Lineage: {e.lineage_id}  |  chain_seq: {e.chain_seq}">
                    🔒 {e.chain_seq}
                  </span>
                  <button
                    class="hash-btn"
                    style="margin-left:4px;"
                    title="View this lineage's full entry history (paginated, does not load everything at once)"
                    onclick={() => openLineageView(e.lineage_id)}
                  >view</button>
                {:else}
                  <span class="dim">—</span>
                {/if}
              </td>

              <!-- prev_hash -->
              <td class="hash-cell">
                {#if e.prev_hash}
                  {@const key = `prev-${e.id}`}
                  <button
                    class="hash-btn"
                    title="prev_hash — click to copy:\n{e.prev_hash}"
                    onclick={() => copyHash(e.prev_hash, key)}
                  ><code>{copiedId === key ? '✓ copied' : trunc(e.prev_hash)}</code></button>
                {:else}
                  <span class="dim">—</span>
                {/if}
              </td>

              <!-- entry_hash -->
              <td class="hash-cell">
                {#if e.entry_hash}
                  {@const key = `entry-${e.id}`}
                  <button
                    class="hash-btn"
                    title="entry_hash — click to copy:\n{e.entry_hash}"
                    onclick={() => copyHash(e.entry_hash, key)}
                  ><code>{copiedId === key ? '✓ copied' : trunc(e.entry_hash)}</code></button>
                {:else}
                  <span class="dim">—</span>
                {/if}
              </td>

              <!-- Verify column -->
              <td class="verify-cell">
                {#if chained}
                  {#if rv?.pending}
                    <span class="verify-pending">…</span>
                  {:else if rv?.ok === true}
                    <span class="verify-ok" title={rv.message}>✓ OK</span>
                  {:else if rv?.ok === false}
                    <span class="verify-fail" title={rv.message}>✗ Fail</span>
                  {:else}
                    <div class="verify-btns">
                      <button class="btn btn-sm" title="Recompute and verify this single entry's hash" onclick={() => verifyRow(e)}>Row</button>
                      <button class="btn btn-sm" title="Verify the entire hash chain for lineage: {e.lineage_id}" onclick={() => verifyLineage(e)}>Chain</button>
                    </div>
                  {/if}
                {:else}
                  <span class="dim">—</span>
                {/if}
              </td>
            </tr>
            <!-- Inline verification message if present -->
            {#if rv && !rv.pending && rv.message}
              <tr class="verify-detail-row">
                <td colspan="9">
                  <span class={rv.ok ? 'verify-detail-ok' : 'verify-detail-fail'}>
                    {rv.ok ? '✓' : '✗'} {rv.message}
                  </span>
                  <button class="dismiss-btn" onclick={() => { const n = {...rowVerify}; delete n[e.id]; rowVerify = n; }}>×</button>
                </td>
              </tr>
            {/if}
          {/each}
        </tbody>
      </table>
    </div>

    {#if totalPages > 1}
      <div style="display:flex;align-items:center;justify-content:center;gap:16px;margin-top:16px;font-size:13px;">
        <button class="btn btn-sm" disabled={page <= 1} onclick={() => { page--; load(); }}>Prev</button>
        <span>Page {page} of {totalPages}</span>
        <button class="btn btn-sm" disabled={page >= totalPages} onclick={() => { page++; load(); }}>Next</button>
      </div>
    {/if}
  </DataState>
</div>

<!-- WP-63: cursor-paginated single-lineage detail panel -->
{#if viewingLineageId}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="lineage-modal-backdrop"
    role="dialog"
    aria-modal="true"
    aria-label="Lineage history"
    onclick={closeLineageView}
    onkeydown={(e) => { if (e.key === 'Escape') closeLineageView(); }}
    tabindex="-1"
  >
    <div class="lineage-modal" onclick={(e) => e.stopPropagation()}>
      <div class="lineage-modal-header">
        <h3 style="font-size:15px;margin:0;">Lineage history — <code>{viewingLineageId.slice(0, 12)}…</code></h3>
        <button class="btn btn-sm" aria-label="Close lineage history" onclick={closeLineageView}>✕</button>
      </div>
      <p style="font-size:12px;color:var(--color-text-muted, #64748b);margin:0 0 10px;">
        Loaded oldest-first, {LINEAGE_PAGE_SIZE} entries at a time (chain_seq-based cursor) — never the full lineage at once.
      </p>
      <div class="lineage-entries-scroll">
        <table>
          <thead>
            <tr><th>Seq</th><th>Date</th><th>Action</th><th>Entity</th><th>Details</th></tr>
          </thead>
          <tbody>
            {#each lineageEntries as le}
              <tr>
                <td>{le.chain_seq}</td>
                <td style="white-space:nowrap;">{le.created_at}</td>
                <td><span class="badge badge-blue">{le.action}</span></td>
                <td>{le.entity_type}</td>
                <td>{le.details || le.new_value || '—'}</td>
              </tr>
            {/each}
          </tbody>
        </table>
        {#if lineageEntries.length === 0 && !lineageLoadingMore}
          <p class="dim" style="text-align:center;padding:12px;">No entries found for this lineage.</p>
        {/if}
      </div>
      {#if lineageHasMore}
        <div style="text-align:center;margin-top:10px;">
          <button class="btn btn-sm" disabled={lineageLoadingMore} onclick={() => loadLineagePage(lineageNextCursor)}>
            {lineageLoadingMore ? 'Loading…' : 'Load more'}
          </button>
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .chain-banner {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    margin-bottom: 14px;
    background: color-mix(in srgb, var(--color-success, #22c55e) 8%, var(--color-surface, #fff));
    border: 1px solid color-mix(in srgb, var(--color-success, #22c55e) 30%, transparent);
    border-radius: 6px;
    font-size: 13px;
    color: var(--color-text, #111);
  }

  .chain-banner-icon { font-size: 16px; }
  .chain-banner-icon--legacy { opacity: 0.5; }
  .legacy-note { color: var(--color-text-muted, #9ca3af); font-size: 12px; }

  .chain-banner-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-left: auto;
    flex-shrink: 0;
  }
  .batch-ok { font-size: 12px; font-weight: 600; color: var(--color-success, #16a34a); }
  .batch-fail { font-size: 12px; font-weight: 600; color: var(--color-danger, #dc2626); }

  .chain-th {
    white-space: nowrap;
    font-size: 12px;
  }

  .row-chained {
    background-color: color-mix(in srgb, var(--color-success, #22c55e) 5%, transparent);
  }

  .seq-badge {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    font-size: 11px;
    font-weight: 600;
    color: var(--color-success, #16a34a);
    white-space: nowrap;
    cursor: help;
  }

  .dim { color: var(--color-text-muted, #9ca3af); font-size: 12px; }

  .chain-cell { white-space: nowrap; min-width: 72px; }
  .hash-cell  { min-width: 110px; }
  .verify-cell { min-width: 110px; white-space: nowrap; }

  .hash-btn {
    background: none;
    border: 1px solid var(--color-border, #e5e7eb);
    border-radius: 4px;
    padding: 2px 5px;
    cursor: pointer;
    font-size: 11px;
    color: var(--color-text-secondary, #6b7280);
    transition: background 0.1s, color 0.1s;
  }
  .hash-btn:hover { background: var(--color-surface-hover, #f3f4f6); color: var(--color-text, #111); }
  .hash-btn code { font-family: monospace; letter-spacing: 0.02em; }

  .verify-btns { display: flex; gap: 4px; }

  .verify-pending { font-size: 12px; color: var(--color-text-muted, #9ca3af); }

  .verify-ok {
    font-size: 12px;
    font-weight: 600;
    color: var(--color-success, #16a34a);
    cursor: help;
  }
  .verify-fail {
    font-size: 12px;
    font-weight: 600;
    color: var(--color-danger, #dc2626);
    cursor: help;
  }

  .verify-detail-row td {
    padding: 4px 12px 6px 20px;
    font-size: 12px;
    border-top: none;
    background: color-mix(in srgb, var(--color-surface, #fff) 80%, transparent);
  }

  .verify-detail-ok {
    color: var(--color-success, #16a34a);
    font-weight: 500;
  }
  .verify-detail-fail {
    color: var(--color-danger, #dc2626);
    font-weight: 500;
  }

  .dismiss-btn {
    margin-left: 10px;
    background: none;
    border: none;
    cursor: pointer;
    color: var(--color-text-muted, #9ca3af);
    font-size: 14px;
    padding: 0 4px;
    line-height: 1;
  }
  .dismiss-btn:hover { color: var(--color-text, #111); }

  /* Checkpoint panel */
  .cp-panel {
    background: color-mix(in srgb, var(--color-primary, #3b82f6) 6%, var(--color-surface, #fff));
    border: 1px solid color-mix(in srgb, var(--color-primary, #3b82f6) 25%, transparent);
    border-radius: 6px;
    padding: 14px 16px;
    margin-bottom: 14px;
    font-size: 13px;
  }

  .cp-panel-header {
    display: flex;
    align-items: baseline;
    gap: 10px;
    margin-bottom: 12px;
  }

  .cp-panel-hint {
    font-size: 12px;
    color: var(--color-text-muted, #6b7280);
  }

  .cp-create-row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    margin-bottom: 14px;
  }

  .cp-select {
    min-width: 220px;
    max-width: 320px;
    font-size: 13px;
  }

  .cp-seq-input {
    width: 160px;
    font-size: 13px;
  }

  .cp-empty {
    font-size: 12px;
    color: var(--color-text-muted, #6b7280);
    margin: 6px 0 0;
  }

  .cp-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 12px;
  }

  .cp-table th, .cp-table td {
    padding: 5px 10px;
    border-bottom: 1px solid var(--color-border, #e5e7eb);
    text-align: left;
  }

  .cp-table th {
    font-weight: 600;
    color: var(--color-text-muted, #6b7280);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .mono-sm { font-family: monospace; font-size: 11px; }
  .nowrap  { white-space: nowrap; }

  /* WP-21: Auto badge on auto-created checkpoints */
  .auto-badge {
    display: inline-block;
    margin-left: 4px;
    padding: 1px 4px;
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    border-radius: 3px;
    background: color-mix(in srgb, var(--color-primary, #3b82f6) 15%, transparent);
    color: var(--color-primary, #3b82f6);
    vertical-align: middle;
  }

  /* WP-21: Imported proof verification panel */
  .proof-import-panel {
    margin-top: 14px;
    padding-top: 12px;
    border-top: 1px solid color-mix(in srgb, var(--color-border, #e5e7eb) 60%, transparent);
  }

  .proof-import-header {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 8px;
    font-size: 13px;
  }

  .proof-paste {
    width: 100%;
    font-family: monospace;
    font-size: 11px;
    resize: vertical;
    border: 1px solid var(--color-border, #e5e7eb);
    border-radius: 4px;
    padding: 6px 8px;
    background: var(--color-surface, #fff);
    color: var(--color-text, #111);
    box-sizing: border-box;
  }

  .proof-import-actions {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 8px;
  }

  /* WP-21: Auto-checkpoint config panel */
  .auto-config-panel {
    margin-top: 14px;
    padding-top: 12px;
    border-top: 1px solid color-mix(in srgb, var(--color-border, #e5e7eb) 60%, transparent);
    font-size: 13px;
  }

  .auto-config-row {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
    margin-top: 8px;
  }

  .auto-config-label {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 12px;
    cursor: pointer;
  }

  .auto-config-num {
    width: 70px;
    font-size: 12px;
  }

  .auto-run-result {
    font-size: 12px;
    font-weight: 600;
    color: var(--color-success, #16a34a);
  }

  .lineage-modal-backdrop {
    position: fixed; inset: 0; z-index: 2000;
    background: rgba(0,0,0,0.5);
    display: flex; align-items: center; justify-content: center;
    padding: 24px;
  }
  .lineage-modal {
    background: var(--color-surface, #fff);
    border-radius: 10px;
    padding: 18px 20px;
    width: min(720px, 100%);
    max-height: 80vh;
    display: flex; flex-direction: column;
    box-shadow: 0 12px 48px rgba(0,0,0,0.3);
  }
  :global(.dark) .lineage-modal { background: #0f172a; }
  .lineage-modal-header {
    display: flex; align-items: center; justify-content: space-between;
    margin-bottom: 8px;
  }
  .lineage-entries-scroll {
    overflow-y: auto;
    flex: 1;
    border: 1px solid #e2e8f0;
    border-radius: 6px;
  }
  :global(.dark) .lineage-entries-scroll { border-color: #334155; }
  .lineage-entries-scroll table { width: 100%; border-collapse: collapse; font-size: 12.5px; }
  .lineage-entries-scroll th {
    position: sticky; top: 0;
    background: #f1f5f9; text-align: left; padding: 6px 8px;
  }
  :global(.dark) .lineage-entries-scroll th { background: #1e293b; }
  .lineage-entries-scroll td { padding: 5px 8px; border-top: 1px solid #f1f5f9; }
  :global(.dark) .lineage-entries-scroll td { border-top-color: #1e293b; }
</style>
