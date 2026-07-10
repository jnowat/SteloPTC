<script lang="ts">
  import { addNotification } from '../stores/app';
  import {
    listSignedEvents, verifySignedEventLedger, getUserSigningPublicKey,
    type SignedEvent, type LedgerVerification,
  } from '../api';

  // WP-67: Trust Layer Phase 3 — the signed-event ledger. Each entry is
  // hash-chained (tamper-evident) AND signed with the acting user's Ed25519 key
  // (non-repudiation). New specimens are auto-signed on creation; verification
  // walks the whole ledger (hashes + sequence + every signature).

  let open = $state(false);
  let events = $state<SignedEvent[]>([]);
  let loading = $state(false);
  let verifying = $state(false);
  let verification = $state<LedgerVerification | null>(null);
  let myKey = $state<string | null>(null);

  async function toggle() {
    open = !open;
    if (open && events.length === 0) await load();
  }

  async function load() {
    loading = true;
    try {
      events = await listSignedEvents(undefined, 100);
    } catch (e: any) {
      addNotification(e?.message || 'Failed to load signed events', 'error');
    } finally {
      loading = false;
    }
  }

  async function doVerify() {
    verifying = true;
    try {
      verification = await verifySignedEventLedger();
      addNotification(verification.message, verification.verified ? 'success' : 'error');
    } catch (e: any) {
      addNotification(e?.message || 'Ledger verification failed', 'error');
    } finally {
      verifying = false;
    }
  }

  async function showMyKey() {
    try {
      myKey = await getUserSigningPublicKey();
    } catch (e: any) {
      addNotification(e?.message || 'Failed to load your signing key', 'error');
    }
  }

  function short(s: string | null, n = 12): string {
    if (!s) return '—';
    return s.length > n ? `${s.slice(0, n)}…` : s;
  }
</script>

<div class="card" style="margin-bottom:16px;">
  <div class="ledger-header">
    <strong>🔏 Signed Event Ledger (Trust Layer Phase 3)</strong>
    <button class="btn btn-sm" onclick={toggle}>{open ? 'Hide' : 'Show'}</button>
  </div>

  {#if open}
    <p class="ledger-intro">
      A ledger of specimen lifecycle events, each hash-chained (tamper-evident) and
      individually signed with the acting user's Ed25519 key (non-repudiation) — so
      an entry's authorship cannot be forged by anyone who can write to the database
      but does not hold the signer's private key. New specimens are signed
      automatically on creation. See <code>docs/signed-event-ledger.md</code>.
    </p>

    <div class="ledger-actions">
      <button class="btn btn-sm" disabled={verifying} onclick={doVerify}>
        {verifying ? 'Verifying…' : 'Verify Ledger'}
      </button>
      <button class="btn btn-sm" onclick={showMyKey}>Show My Signing Key</button>
      {#if verification}
        <span class={verification.verified ? 'ledger-ok' : 'ledger-fail'}>
          {verification.verified ? '✓' : '✗'} {verification.message}
        </span>
      {/if}
    </div>

    {#if myKey}
      <div class="ledger-key">
        <span class="ledger-key-label">Your public key (base64)</span>
        <code class="ledger-mono">{myKey}</code>
      </div>
    {/if}

    {#if loading}
      <p class="ledger-empty">Loading signed events…</p>
    {:else if events.length === 0}
      <p class="ledger-empty">No signed events yet — create a specimen to record the first signed transaction.</p>
    {:else}
      <div class="ledger-table-wrap">
        <table class="ledger-table">
          <thead>
            <tr>
              <th>#</th>
              <th>Event</th>
              <th>Entity</th>
              <th>Signed by</th>
              <th>Event hash</th>
              <th>When</th>
            </tr>
          </thead>
          <tbody>
            {#each events as e}
              <tr>
                <td>{e.seq}</td>
                <td>{e.event_type}</td>
                <td><code>{e.entity_type}{e.entity_id ? ` · ${short(e.entity_id, 8)}` : ''}</code></td>
                <td><code>{short(e.user_id, 8)}</code></td>
                <td><code title={e.event_hash}>{short(e.event_hash, 12)}</code></td>
                <td>{short(e.created_at, 19)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  {/if}
</div>

<style>
  .ledger-header { display: flex; justify-content: space-between; align-items: center; }
  .ledger-intro { font-size: 0.85rem; color: var(--color-text-secondary, #555); line-height: 1.45; margin: 0.5rem 0 0.75rem; }
  .ledger-actions { display: flex; gap: 0.5rem; align-items: center; flex-wrap: wrap; margin-bottom: 0.5rem; }
  .ledger-ok { color: #166534; font-size: 0.82rem; font-weight: 600; }
  .ledger-fail { color: #b91c1c; font-size: 0.82rem; font-weight: 600; }
  .ledger-key { margin: 0.5rem 0; display: flex; flex-direction: column; gap: 0.25rem; }
  .ledger-key-label { font-size: 0.75rem; font-weight: 600; }
  .ledger-mono {
    font-family: var(--font-mono, monospace); font-size: 0.72rem; word-break: break-all;
    background: var(--color-surface-2, #f7f7f8); padding: 0.3rem 0.4rem; border-radius: 4px;
    border: 1px solid var(--color-border, #e2e2e2);
  }
  .ledger-table-wrap { overflow-x: auto; }
  .ledger-table { width: 100%; border-collapse: collapse; font-size: 0.82rem; }
  .ledger-table th, .ledger-table td { text-align: left; padding: 0.35rem 0.5rem; border-bottom: 1px solid var(--color-border, #eee); white-space: nowrap; }
  .ledger-empty { font-size: 0.85rem; color: var(--color-text-secondary, #777); padding: 0.5rem 0; }
</style>
