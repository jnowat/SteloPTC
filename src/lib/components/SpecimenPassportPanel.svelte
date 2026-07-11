<script lang="ts">
  import { addNotification } from '../stores/app';
  import { currentUser } from '../stores/auth';
  import {
    getLabIdentity, setLabName, issueSpecimenPassport, verifySpecimenPassport,
    importSpecimenPassport, listSpecimenPassports, getSpecimenPassportJson,
    type IssuerIdentity, type PassportVerification, type PassportRecord,
  } from '../api';

  // WP-70: Federated identity & inter-lab specimen transfer — the specimen
  // passport. A signed, self-contained document a partner lab verifies with only
  // the issuer's public key and the embedded, recomputable audit chain. Importing
  // one folds it into this lab's own audit chain. SteloPTC does not transport
  // passports over a network — issuing downloads a JSON file; importing reads one.
  // See docs/specimen-passport.md.

  const canWrite = $derived(
    $currentUser?.role === 'admin' || $currentUser?.role === 'supervisor' || $currentUser?.role === 'tech',
  );
  const canManage = $derived($currentUser?.role === 'admin' || $currentUser?.role === 'supervisor');

  let open = $state(false);
  let identity = $state<IssuerIdentity | null>(null);
  let editingName = $state(false);
  let nameDraft = $state('');

  let issueId = $state('');
  let issuing = $state(false);

  let inbox = $state('');
  let verifying = $state(false);
  let importing = $state(false);
  let lastVerification = $state<PassportVerification | null>(null);

  let records = $state<PassportRecord[]>([]);
  let loadingRecords = $state(false);

  async function toggle() {
    open = !open;
    if (open && !identity) await refresh();
  }

  async function refresh() {
    try {
      identity = await getLabIdentity();
    } catch (e: any) {
      addNotification(e?.message || 'Failed to load lab identity', 'error');
    }
    await loadRecords();
  }

  async function loadRecords() {
    loadingRecords = true;
    try {
      records = await listSpecimenPassports();
    } catch (e: any) {
      addNotification(e?.message || 'Failed to load passport register', 'error');
    } finally {
      loadingRecords = false;
    }
  }

  async function saveName() {
    try {
      await setLabName(nameDraft);
      editingName = false;
      await refresh();
      addNotification('Lab name updated', 'success');
    } catch (e: any) {
      addNotification(e?.message || 'Failed to update lab name', 'error');
    }
  }

  function downloadJson(json: string, filename: string) {
    const blob = new Blob([json], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    a.click();
    URL.revokeObjectURL(url);
  }

  async function doIssue() {
    if (!issueId.trim()) {
      addNotification('Enter a specimen ID to issue a passport', 'error');
      return;
    }
    issuing = true;
    try {
      const passport = await issueSpecimenPassport(issueId.trim());
      const json = JSON.stringify(passport, null, 2);
      downloadJson(json, `passport-${passport.specimen.accession_number || passport.passport_id}.json`);
      addNotification(`Passport issued for ${passport.specimen.accession_number}`, 'success');
      issueId = '';
      await loadRecords();
    } catch (e: any) {
      addNotification(e?.message || 'Failed to issue passport', 'error');
    } finally {
      issuing = false;
    }
  }

  async function onFile(event: Event) {
    const input = event.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    inbox = await file.text();
    input.value = '';
  }

  async function doVerify() {
    if (!inbox.trim()) {
      addNotification('Paste or load a passport to verify', 'error');
      return;
    }
    verifying = true;
    try {
      lastVerification = await verifySpecimenPassport(inbox);
      addNotification(lastVerification.message, lastVerification.verified ? 'success' : 'error');
    } catch (e: any) {
      lastVerification = null;
      addNotification(e?.message || 'Verification failed', 'error');
    } finally {
      verifying = false;
    }
  }

  async function doImport() {
    if (!inbox.trim()) {
      addNotification('Paste or load a passport to import', 'error');
      return;
    }
    importing = true;
    try {
      const result = await importSpecimenPassport(inbox);
      lastVerification = result.verification;
      addNotification(
        `Imported passport for ${result.verification.subject_accession} — recorded in this lab's audit chain`,
        'success',
      );
      inbox = '';
      await loadRecords();
    } catch (e: any) {
      addNotification(e?.message || 'Import failed', 'error');
    } finally {
      importing = false;
    }
  }

  async function reexport(rec: PassportRecord) {
    try {
      const json = await getSpecimenPassportJson(rec.id);
      downloadJson(json, `passport-${rec.subject_accession || rec.passport_id}.json`);
    } catch (e: any) {
      addNotification(e?.message || 'Failed to export passport', 'error');
    }
  }

  async function copy(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      addNotification('Copied to clipboard', 'success');
    } catch {
      addNotification('Copy failed', 'error');
    }
  }

  function short(s: string | null, n = 12): string {
    if (!s) return '—';
    return s.length > n ? `${s.slice(0, n)}…` : s;
  }
</script>

<div class="card" style="margin-bottom:16px;">
  <div class="pp-header">
    <strong>🛂 Specimen Passports (Federated Transfer)</strong>
    <button class="btn btn-sm" onclick={toggle}>{open ? 'Hide' : 'Show'}</button>
  </div>

  {#if open}
    <p class="pp-intro">
      A specimen passport is a signed, self-contained record of a specimen's identity
      and full provenance that a partner lab can verify <em>independently</em> — using
      only this lab's public key and the embedded, recomputable audit chain. Importing a
      passport folds it into this lab's own audit chain. No central authority, and no
      network transfer: issuing downloads a JSON file; importing reads one. See
      <code>docs/specimen-passport.md</code>.
    </p>

    <!-- Lab identity -->
    <div class="pp-section">
      <div class="pp-section-title">This lab's issuer identity</div>
      {#if identity}
        <div class="pp-identity">
          <div class="pp-id-row">
            <span class="pp-label">Lab name</span>
            {#if editingName}
              <input class="pp-name-input" bind:value={nameDraft} placeholder="Laboratory name" />
              <button class="btn btn-sm" onclick={saveName}>Save</button>
              <button class="btn btn-sm" onclick={() => (editingName = false)}>Cancel</button>
            {:else}
              <span class="pp-name">{identity.lab_name}</span>
              {#if canManage}
                <button class="btn btn-sm" onclick={() => { nameDraft = identity!.lab_name; editingName = true; }}>Edit</button>
              {/if}
            {/if}
          </div>
          <div class="pp-id-row">
            <span class="pp-label">Public key</span>
            <code class="pp-mono">{identity.public_key}</code>
            <button class="btn btn-sm" onclick={() => copy(identity!.public_key)}>Copy</button>
          </div>
          <p class="pp-hint">Share this public key with partner labs so they can verify the passports you issue.</p>
        </div>
      {:else}
        <p class="pp-empty">Loading identity…</p>
      {/if}
    </div>

    <!-- Issue -->
    {#if canWrite}
      <div class="pp-section">
        <div class="pp-section-title">Issue a passport</div>
        <div class="pp-issue-row">
          <input class="pp-grow" bind:value={issueId} placeholder="Specimen ID" />
          <button class="btn btn-sm" disabled={issuing} onclick={doIssue}>
            {issuing ? 'Issuing…' : 'Issue & Download'}
          </button>
        </div>
        <p class="pp-hint">Generates a signed passport for the specimen and downloads it as JSON.</p>
      </div>
    {/if}

    <!-- Verify / Import -->
    <div class="pp-section">
      <div class="pp-section-title">Verify or import a received passport</div>
      <textarea class="pp-textarea" rows="4" bind:value={inbox} placeholder="Paste passport JSON here, or load a file…"></textarea>
      <div class="pp-actions">
        <label class="btn btn-sm pp-file-btn">
          Load file…
          <input type="file" accept="application/json,.json" onchange={onFile} hidden />
        </label>
        <button class="btn btn-sm" disabled={verifying} onclick={doVerify}>{verifying ? 'Verifying…' : 'Verify'}</button>
        {#if canWrite}
          <button class="btn btn-sm btn-primary" disabled={importing} onclick={doImport}>{importing ? 'Importing…' : 'Verify & Import'}</button>
        {/if}
      </div>

      {#if lastVerification}
        <div class="pp-verdict {lastVerification.verified ? 'pp-ok' : 'pp-fail'}">
          <div class="pp-verdict-head">
            {lastVerification.verified ? '✓' : '✗'} {lastVerification.message}
          </div>
          <div class="pp-verdict-meta">
            Issuer: <strong>{lastVerification.issuer_lab}</strong> ·
            Subject: <strong>{lastVerification.subject_accession}</strong>
            {#if lastVerification.subject_scientific_name}(<em>{lastVerification.subject_scientific_name}</em>){/if}
            · {lastVerification.entry_count} provenance entries
          </div>
          <ul class="pp-checks">
            {#each lastVerification.checks as c}
              <li class={c.ok ? 'pp-check-ok' : 'pp-check-fail'}>
                {c.ok ? '✓' : '✗'} <strong>{c.name}</strong> — {c.detail}
              </li>
            {/each}
          </ul>
        </div>
      {/if}
    </div>

    <!-- Register -->
    <div class="pp-section">
      <div class="pp-section-title">Passport register</div>
      {#if loadingRecords}
        <p class="pp-empty">Loading…</p>
      {:else if records.length === 0}
        <p class="pp-empty">No passports issued or imported yet.</p>
      {:else}
        <div class="pp-table-wrap">
          <table class="pp-table">
            <thead>
              <tr>
                <th>Direction</th>
                <th>Accession</th>
                <th>Issuer</th>
                <th>Entries</th>
                <th>Content hash</th>
                <th>When</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {#each records as r}
                <tr>
                  <td><span class="pp-dir pp-dir-{r.direction}">{r.direction}</span></td>
                  <td>{r.subject_accession}</td>
                  <td>{r.issuer_lab}</td>
                  <td>{r.entry_count}</td>
                  <td><code title={r.content_hash}>{short(r.content_hash, 12)}</code></td>
                  <td>{short(r.created_at, 19)}</td>
                  <td>
                    {#if r.direction === 'issued'}
                      <button class="btn btn-sm" onclick={() => reexport(r)}>Export</button>
                    {/if}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .pp-header { display: flex; justify-content: space-between; align-items: center; }
  .pp-intro { font-size: 0.85rem; color: var(--color-text-secondary, #555); line-height: 1.45; margin: 0.5rem 0 0.75rem; }
  .pp-section { border-top: 1px solid var(--color-border, #eee); padding: 0.6rem 0; }
  .pp-section-title { font-weight: 600; font-size: 0.85rem; margin-bottom: 0.4rem; }
  .pp-identity { display: flex; flex-direction: column; gap: 0.35rem; }
  .pp-id-row { display: flex; align-items: center; gap: 0.5rem; flex-wrap: wrap; }
  .pp-label { font-size: 0.75rem; font-weight: 600; min-width: 5.5rem; color: var(--color-text-secondary, #666); }
  .pp-name { font-weight: 600; }
  .pp-name-input { flex: 1; min-width: 12rem; }
  .pp-hint { font-size: 0.75rem; color: var(--color-text-secondary, #777); margin: 0.1rem 0 0; }
  .pp-mono {
    font-family: var(--font-mono, monospace); font-size: 0.72rem; word-break: break-all;
    background: var(--color-surface-2, #f7f7f8); padding: 0.25rem 0.4rem; border-radius: 4px;
    border: 1px solid var(--color-border, #e2e2e2); flex: 1; min-width: 12rem;
  }
  .pp-issue-row { display: flex; gap: 0.5rem; align-items: center; flex-wrap: wrap; }
  .pp-grow { flex: 1; min-width: 12rem; }
  .pp-textarea { width: 100%; font-family: var(--font-mono, monospace); font-size: 0.72rem; resize: vertical; }
  .pp-actions { display: flex; gap: 0.5rem; align-items: center; flex-wrap: wrap; margin-top: 0.4rem; }
  .pp-file-btn { cursor: pointer; }
  .pp-verdict { margin-top: 0.6rem; padding: 0.5rem 0.6rem; border-radius: 6px; font-size: 0.82rem; }
  .pp-ok { background: rgba(22, 101, 52, 0.08); border: 1px solid rgba(22, 101, 52, 0.35); }
  .pp-fail { background: rgba(185, 28, 28, 0.08); border: 1px solid rgba(185, 28, 28, 0.35); }
  .pp-verdict-head { font-weight: 600; }
  .pp-verdict-meta { font-size: 0.78rem; color: var(--color-text-secondary, #555); margin: 0.25rem 0; }
  .pp-checks { list-style: none; margin: 0.25rem 0 0; padding: 0; display: flex; flex-direction: column; gap: 0.15rem; }
  .pp-check-ok { color: #166534; font-size: 0.78rem; }
  .pp-check-fail { color: #b91c1c; font-size: 0.78rem; }
  .pp-empty { font-size: 0.85rem; color: var(--color-text-secondary, #777); padding: 0.3rem 0; }
  .pp-table-wrap { overflow-x: auto; }
  .pp-table { width: 100%; border-collapse: collapse; font-size: 0.8rem; }
  .pp-table th, .pp-table td { text-align: left; padding: 0.35rem 0.5rem; border-bottom: 1px solid var(--color-border, #eee); white-space: nowrap; }
  .pp-dir { font-size: 0.7rem; font-weight: 600; padding: 0.1rem 0.4rem; border-radius: 999px; text-transform: uppercase; }
  .pp-dir-issued { background: rgba(21, 101, 192, 0.12); color: #1565c0; }
  .pp-dir-imported { background: rgba(46, 125, 50, 0.12); color: #2e7d32; }
</style>
