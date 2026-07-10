<script lang="ts">
  import { currentUser } from '../stores/auth';
  import { addNotification } from '../stores/app';
  import {
    listCheckpointAnchors, prepareCheckpointAnchor, recordCheckpointAnchor,
    verifyCheckpointAnchor, type CheckpointAnchor,
  } from '../api';

  // WP-66: Trust Layer Phase 2 — publish an audit checkpoint's Merkle root to a
  // public chain (Dogecoin) via OP_RETURN, then independently verify that the
  // on-chain data commits to exactly that root. SteloPTC prepares and verifies
  // the bytes; broadcasting is done with an external wallet the operator already
  // controls (no funded wallet / private keys ever live in the app).

  let { checkpoints = [] }: { checkpoints: any[] } = $props();

  const canManage = $derived($currentUser?.role === 'admin' || $currentUser?.role === 'supervisor');

  let anchors = $state<CheckpointAnchor[]>([]);
  let loading = $state(false);
  let selectedCheckpoint = $state('');
  let preparing = $state(false);
  let prepared = $state<CheckpointAnchor | null>(null);

  // Per-anchor inline inputs, keyed by anchor id.
  let txidInputs = $state<Record<string, string>>({});
  let verifyInputs = $state<Record<string, string>>({});
  let busyAnchor = $state<string | null>(null);

  async function loadAnchors() {
    loading = true;
    try {
      anchors = await listCheckpointAnchors();
    } catch (e: any) {
      addNotification(e?.message || 'Failed to load anchors', 'error');
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    if (anchors.length === 0 && !loading) loadAnchors();
  });

  async function doPrepare() {
    if (!selectedCheckpoint) {
      addNotification('Choose a checkpoint to anchor first.', 'error');
      return;
    }
    preparing = true;
    try {
      prepared = await prepareCheckpointAnchor(selectedCheckpoint, 'dogecoin');
      addNotification('Anchor payload prepared — broadcast it, then record the txid below.', 'success');
      await loadAnchors();
    } catch (e: any) {
      addNotification(e?.message || 'Failed to prepare anchor', 'error');
    } finally {
      preparing = false;
    }
  }

  async function doRecord(anchor: CheckpointAnchor) {
    const txid = (txidInputs[anchor.id] || '').trim();
    if (!txid) {
      addNotification('Paste the broadcast transaction id first.', 'error');
      return;
    }
    busyAnchor = anchor.id;
    try {
      await recordCheckpointAnchor(anchor.id, txid);
      addNotification('Transaction id recorded — anchor marked submitted.', 'success');
      txidInputs[anchor.id] = '';
      await loadAnchors();
    } catch (e: any) {
      addNotification(e?.message || 'Failed to record txid', 'error');
    } finally {
      busyAnchor = null;
    }
  }

  async function doVerify(anchor: CheckpointAnchor) {
    const hex = (verifyInputs[anchor.id] || '').trim();
    if (!hex) {
      addNotification('Paste the on-chain OP_RETURN data (hex) first.', 'error');
      return;
    }
    busyAnchor = anchor.id;
    try {
      const result = await verifyCheckpointAnchor(anchor.id, hex);
      addNotification(result.message, result.ok ? 'success' : 'error');
      if (result.ok) { verifyInputs[anchor.id] = ''; await loadAnchors(); }
    } catch (e: any) {
      addNotification(e?.message || 'Verification failed', 'error');
    } finally {
      busyAnchor = null;
    }
  }

  async function copyText(text: string, label: string) {
    try {
      await navigator.clipboard.writeText(text);
      addNotification(`${label} copied to clipboard.`, 'success');
    } catch {
      addNotification('Could not access the clipboard.', 'error');
    }
  }

  function short(s: string | null, n = 12): string {
    if (!s) return '—';
    return s.length > n ? `${s.slice(0, n)}…` : s;
  }

  function statusClass(status: string): string {
    if (status === 'confirmed') return 'anchor-ok';
    if (status === 'submitted') return 'anchor-mid';
    return 'anchor-pending';
  }
</script>

<div class="anchor-panel">
  <div class="anchor-intro">
    <strong>⛓ On-Chain Anchoring (Trust Layer Phase 2)</strong>
    <p>
      Publish a checkpoint's Merkle root to the Dogecoin chain in an
      <code>OP_RETURN</code> output, giving anyone a third-party-verifiable,
      timestamped proof that your audit history existed at that root — without
      trusting this lab's database. SteloPTC builds the exact bytes to broadcast
      and verifies what comes back; the actual broadcast is done with your own
      external wallet (no keys or funds ever live in this app).
      See <code>docs/on-chain-anchoring.md</code>.
    </p>
  </div>

  {#if !canManage}
    <p class="anchor-empty">On-chain anchoring is limited to supervisors and admins.</p>
  {:else}
    <!-- Prepare an anchor -->
    <div class="anchor-prepare">
      <label for="anchor-cp-select">Checkpoint to anchor</label>
      <div class="anchor-row">
        <select id="anchor-cp-select" bind:value={selectedCheckpoint}>
          <option value="">— select a checkpoint —</option>
          {#each checkpoints as cp}
            <option value={cp.id}>
              {short(cp.id, 8)} · {cp.lineage_id ? short(cp.lineage_id, 10) : ''} · root {short(cp.merkle_root, 10)}
            </option>
          {/each}
        </select>
        <button class="btn btn-sm" disabled={preparing || !selectedCheckpoint} onclick={doPrepare}>
          {preparing ? 'Preparing…' : 'Prepare Anchor'}
        </button>
      </div>
      {#if checkpoints.length === 0}
        <p class="anchor-hint">Create a Merkle checkpoint above first — you can only anchor a sealed checkpoint.</p>
      {/if}
    </div>

    {#if prepared}
      <div class="anchor-payload">
        <strong>Broadcast this OP_RETURN data</strong>
        <p class="anchor-hint">
          In your Dogecoin wallet, create a transaction with a data / <code>OP_RETURN</code>
          output carrying the bytes below, then paste the resulting transaction id back here.
        </p>
        <div class="anchor-field">
          <span class="anchor-label">OP_RETURN script (hex)</span>
          <code class="anchor-mono">{prepared.op_return_hex}</code>
          <button class="btn btn-xs" onclick={() => copyText(prepared!.op_return_hex, 'OP_RETURN hex')}>Copy</button>
        </div>
        <div class="anchor-field">
          <span class="anchor-label">Merkle root</span>
          <code class="anchor-mono">{prepared.merkle_root}</code>
          <button class="btn btn-xs" onclick={() => copyText(prepared!.merkle_root, 'Merkle root')}>Copy</button>
        </div>
      </div>
    {/if}

    <!-- Anchor list -->
    {#if loading}
      <p class="anchor-empty">Loading anchors…</p>
    {:else if anchors.length === 0}
      <p class="anchor-empty">No anchors yet — prepare one above to publish a checkpoint root on-chain.</p>
    {:else}
      <table class="anchor-table">
        <thead>
          <tr>
            <th>Status</th>
            <th>Checkpoint</th>
            <th>Merkle root</th>
            <th>Txid</th>
            <th>Actions</th>
          </tr>
        </thead>
        <tbody>
          {#each anchors as a}
            <tr>
              <td><span class="anchor-badge {statusClass(a.status)}">{a.status}</span></td>
              <td><code>{short(a.checkpoint_id, 8)}</code></td>
              <td><code title={a.merkle_root}>{short(a.merkle_root, 12)}</code></td>
              <td>
                {#if a.txid}<code title={a.txid}>{short(a.txid, 12)}</code>{:else}—{/if}
              </td>
              <td class="anchor-actions">
                {#if a.status === 'prepared'}
                  <div class="anchor-inline">
                    <input
                      placeholder="broadcast txid (64 hex)"
                      bind:value={txidInputs[a.id]}
                    />
                    <button class="btn btn-xs" disabled={busyAnchor === a.id} onclick={() => doRecord(a)}>Record txid</button>
                  </div>
                {/if}
                {#if a.status !== 'confirmed'}
                  <div class="anchor-inline">
                    <input
                      placeholder="on-chain OP_RETURN data (hex)"
                      bind:value={verifyInputs[a.id]}
                    />
                    <button class="btn btn-xs" disabled={busyAnchor === a.id} onclick={() => doVerify(a)}>Verify</button>
                  </div>
                {:else}
                  <span class="anchor-verified">✓ verified {a.verified_at ? short(a.verified_at, 10) : ''}</span>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  {/if}
</div>

<style>
  .anchor-panel { margin-top: var(--space-4, 1rem); }
  .anchor-intro { margin-bottom: var(--space-3, 0.75rem); }
  .anchor-intro p { margin: 0.35rem 0 0; color: var(--color-text-secondary, #555); font-size: 0.85rem; line-height: 1.45; }
  .anchor-prepare { margin: 0.75rem 0; }
  .anchor-prepare label { display: block; font-size: 0.8rem; font-weight: 600; margin-bottom: 0.25rem; }
  .anchor-row { display: flex; gap: 0.5rem; align-items: center; flex-wrap: wrap; }
  .anchor-row select { flex: 1; min-width: 14rem; padding: 0.4rem; }
  .anchor-hint { font-size: 0.78rem; color: var(--color-text-secondary, #666); margin: 0.4rem 0 0; }
  .anchor-payload {
    margin: 0.75rem 0; padding: 0.75rem;
    border: 1px solid var(--color-border, #ddd); border-radius: var(--radius-md, 6px);
    background: var(--color-surface-2, #f7f7f8);
  }
  .anchor-field { display: flex; align-items: center; gap: 0.5rem; margin-top: 0.5rem; flex-wrap: wrap; }
  .anchor-label { font-size: 0.75rem; font-weight: 600; min-width: 9rem; }
  .anchor-mono {
    font-family: var(--font-mono, monospace); font-size: 0.75rem; word-break: break-all;
    background: var(--color-surface, #fff); padding: 0.25rem 0.4rem; border-radius: 4px;
    border: 1px solid var(--color-border, #e2e2e2); flex: 1; min-width: 12rem;
  }
  .anchor-table { width: 100%; border-collapse: collapse; margin-top: 0.75rem; font-size: 0.82rem; }
  .anchor-table th, .anchor-table td { text-align: left; padding: 0.4rem 0.5rem; border-bottom: 1px solid var(--color-border, #eee); vertical-align: top; }
  .anchor-actions { min-width: 18rem; }
  .anchor-inline { display: flex; gap: 0.35rem; align-items: center; margin-bottom: 0.3rem; }
  .anchor-inline input { flex: 1; min-width: 10rem; padding: 0.3rem; font-family: var(--font-mono, monospace); font-size: 0.72rem; }
  .anchor-badge { padding: 0.1rem 0.45rem; border-radius: 999px; font-size: 0.72rem; font-weight: 600; text-transform: capitalize; }
  .anchor-pending { background: #eee; color: #555; }
  .anchor-mid { background: #fde68a; color: #7c5b00; }
  .anchor-ok { background: #bbf7d0; color: #166534; }
  .anchor-verified { color: #166534; font-size: 0.76rem; font-weight: 600; }
  .anchor-empty { font-size: 0.85rem; color: var(--color-text-secondary, #777); padding: 0.5rem 0; }
</style>
