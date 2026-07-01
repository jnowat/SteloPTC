<script lang="ts">
  import { onMount } from 'svelte';
  import {
    listBackupTargets, createBackupTarget, deleteBackupTarget,
    cloudBackup, restoreFromCloud, reconcileCloudSync,
    type BackupTargetSummary,
  } from '../api';
  import { addNotification } from '../stores/app';
  import DataState from './DataState.svelte';

  const TARGET_TYPES: { value: string; label: string; live: boolean }[] = [
    { value: 'local_nas', label: 'Local / Network Share (NAS)', live: true },
    { value: 'smb', label: 'SMB Share', live: true },
    { value: 's3', label: 'S3-Compatible Object Storage', live: false },
    { value: 'sftp', label: 'SFTP', live: false },
  ];

  function typeLabel(t: string): string {
    return TARGET_TYPES.find((x) => x.value === t)?.label ?? t;
  }
  function isLive(t: string): boolean {
    return TARGET_TYPES.find((x) => x.value === t)?.live ?? false;
  }
  function isPathType(t: string): boolean {
    return t === 'local_nas' || t === 'smb';
  }

  function getDeviceId(): string {
    let id = localStorage.getItem('stelo_device_id');
    if (!id) {
      id = crypto.randomUUID();
      localStorage.setItem('stelo_device_id', id);
    }
    return id;
  }

  let targets = $state<BackupTargetSummary[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  async function load() {
    loading = true;
    error = null;
    try {
      targets = await listBackupTargets();
    } catch (e: any) {
      error = e.message;
    } finally {
      loading = false;
    }
  }

  onMount(load);

  // ── Add Target form ─────────────────────────────────────────────────────
  let showAddForm = $state(false);
  let creating = $state(false);
  let newTarget = $state({
    name: '', targetType: 'local_nas', passphrase: '', bucketOrPath: '',
    endpoint: '', accessKey: '', secretKey: '', scheduleCron: '',
  });

  let passphraseTooShort = $derived(
    newTarget.passphrase.length > 0 && newTarget.passphrase.length < 8,
  );

  function resetAddForm() {
    newTarget = {
      name: '', targetType: 'local_nas', passphrase: '', bucketOrPath: '',
      endpoint: '', accessKey: '', secretKey: '', scheduleCron: '',
    };
  }

  function openAddForm() {
    resetAddForm();
    showAddForm = true;
  }

  function cancelAddForm() {
    showAddForm = false;
    resetAddForm();
  }

  async function handleCreateTarget() {
    if (!newTarget.name.trim() || !newTarget.bucketOrPath.trim() || newTarget.passphrase.length < 8) {
      addNotification('Name, path/bucket, and a passphrase of at least 8 characters are required.', 'warning');
      return;
    }
    creating = true;
    try {
      await createBackupTarget({
        name: newTarget.name.trim(),
        targetType: newTarget.targetType,
        passphrase: newTarget.passphrase,
        bucketOrPath: newTarget.bucketOrPath.trim(),
        endpoint: newTarget.endpoint.trim() || undefined,
        accessKey: newTarget.accessKey.trim() || undefined,
        secretKey: newTarget.secretKey.trim() || undefined,
        scheduleCron: newTarget.scheduleCron.trim() || undefined,
      });
      addNotification(`Backup target "${newTarget.name.trim()}" created`, 'success');
      showAddForm = false;
      resetAddForm();
      await load();
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      creating = false;
    }
  }

  async function handleDeleteTarget(t: BackupTargetSummary) {
    if (!confirm(`Delete backup target "${t.name}"? This only removes the target configuration — it does not delete any backups already stored there.`)) {
      return;
    }
    try {
      await deleteBackupTarget(t.id);
      addNotification(`Target "${t.name}" deleted`, 'success');
      await load();
    } catch (e: any) {
      addNotification(e.message, 'error');
    }
  }

  // ── Backup Now (inline passphrase prompt, transient in-memory only) ────────
  let backupPromptFor = $state<string | null>(null);
  let backupPassphrase = $state('');
  let backingUpId = $state<string | null>(null);

  function openBackupPrompt(t: BackupTargetSummary) {
    backupPromptFor = t.id;
    backupPassphrase = '';
  }

  function cancelBackupPrompt() {
    backupPromptFor = null;
    backupPassphrase = '';
  }

  async function handleBackupNow(t: BackupTargetSummary) {
    if (!backupPassphrase) {
      addNotification('Enter the passphrase for this target.', 'warning');
      return;
    }
    backingUpId = t.id;
    try {
      const result = await cloudBackup(t.id, backupPassphrase);
      addNotification(
        `Backup complete (${(result.size_bytes / 1024).toFixed(1)} KB in ${result.duration_ms} ms)` +
        (result.merkle_root_included ? ' — integrity root included.' : '.'),
        'success',
      );
      backupPromptFor = null;
      backupPassphrase = '';
      await load();
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      backingUpId = null;
    }
  }

  // ── Restore (two-step destructive confirm, replicating Dashboard.svelte) ──
  let restoreTarget = $state<BackupTargetSummary | null>(null);
  let restoreStep = $state<1 | 2>(1);
  let restoreFileName = $state('');
  let restorePassphrase = $state('');
  let restorePhrase = $state('');
  let restoring = $state(false);

  function openRestorePanel(t: BackupTargetSummary) {
    restoreTarget = t;
    restoreStep = 1;
    restoreFileName = '';
    restorePassphrase = '';
    restorePhrase = '';
  }

  function cancelRestore() {
    restoreTarget = null;
    restoreStep = 1;
    restoreFileName = '';
    restorePassphrase = '';
    restorePhrase = '';
  }

  function confirmRestoreStep1() {
    if (!restoreFileName.trim() || !restorePassphrase) {
      addNotification('Enter the backup file name and passphrase before continuing.', 'warning');
      return;
    }
    restoreStep = 2;
  }

  async function handleRestore() {
    if (restorePhrase !== 'RESTORE') {
      addNotification('Type exactly: RESTORE', 'warning');
      return;
    }
    if (!restoreTarget) return;
    restoring = true;
    try {
      await restoreFromCloud(restoreTarget.id, restorePassphrase, restoreFileName.trim());
      // App restarts automatically after successful restore; message is a fallback.
      addNotification('Restore successful — restarting…', 'success');
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      restoring = false;
    }
  }

  // ── Sync Now (local_nas / smb only) ────────────────────────────────────────
  let syncPromptFor = $state<string | null>(null);
  let syncPassphrase = $state('');
  let syncingId = $state<string | null>(null);
  let syncResult = $state<{ id: string; segments_published: boolean; peer_segments_found: number; new_changes: number; duplicates: number; conflicts_recorded: number } | null>(null);

  function openSyncPrompt(t: BackupTargetSummary) {
    syncPromptFor = t.id;
    syncPassphrase = '';
    syncResult = null;
  }

  function cancelSyncPrompt() {
    syncPromptFor = null;
    syncPassphrase = '';
  }

  async function handleSyncNow(t: BackupTargetSummary) {
    if (!syncPassphrase) {
      addNotification('Enter the passphrase for this target.', 'warning');
      return;
    }
    syncingId = t.id;
    try {
      const deviceId = getDeviceId();
      const result = await reconcileCloudSync(t.id, syncPassphrase, deviceId);
      syncResult = { id: t.id, ...result };
      syncPromptFor = null;
      syncPassphrase = '';
      addNotification('Sync reconciliation complete', 'success');
      await load();
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      syncingId = null;
    }
  }

  function formatDate(iso: string | null): string {
    if (!iso) return 'Never';
    try {
      return new Date(iso).toLocaleString();
    } catch {
      return iso;
    }
  }

  function statusBadgeClass(status: string | null): string {
    if (!status) return 'badge-gray';
    const s = status.toLowerCase();
    if (s.includes('success') || s.includes('ok')) return 'badge-green';
    if (s.includes('fail') || s.includes('error')) return 'badge-red';
    return 'badge-blue';
  }
</script>

<div class="card" style="max-width: 900px; margin-top: 24px;">
  <h2 style="font-size: 16px; font-weight: 700; margin-bottom: 4px;">
    Cloud Backup &amp; Multi-Device Sync <span class="new-feature-badge">New</span>
  </h2>
  <p style="font-size: 13px; color: #6b7280; margin-bottom: 20px;">
    Configure encrypted backup targets and reconcile changes across devices. All backups are
    encrypted client-side with a passphrase before upload.
  </p>

  <div class="info-notice" role="note" style="margin-bottom: 16px;">
    <strong>Your passphrase is never stored</strong>
    <p>
      Passphrases are used only in memory for the duration of a single backup, restore, or sync
      action, and are never written to disk or persisted between actions. You'll need to
      re-enter it every time.
    </p>
  </div>

  <DataState {loading} {error} empty={!loading && !error && targets.length === 0}
    emptyIcon="☁️"
    emptyTitle="No backup targets configured"
    emptyMessage="Add a target to start backing up to a network share, SMB mount, or (configuration-only) S3/SFTP destination."
    onretry={load}>

    <div class="target-list">
      {#each targets as t (t.id)}
        <div class="target-row">
          <div class="target-main">
            <div class="target-title">
              <strong>{t.name}</strong>
              <span class="badge-gray badge-pill">{typeLabel(t.target_type)}</span>
              {#if !t.is_enabled}
                <span class="badge-gray badge-pill">Disabled</span>
              {/if}
              {#if !isLive(t.target_type)}
                <span class="badge-yellow badge-pill">Configuration only</span>
              {/if}
            </div>
            <div class="target-meta">
              Last backup: {formatDate(t.last_backup_at)}
              {#if t.last_backup_size_display}
                · {t.last_backup_size_display}
              {/if}
              {#if t.last_status}
                <span class="{statusBadgeClass(t.last_status)} badge-pill" style="margin-left:6px;">{t.last_status}</span>
              {/if}
              {#if t.schedule_cron}
                · Schedule: <code>{t.schedule_cron}</code>
              {/if}
            </div>
            {#if t.last_error}
              <div class="target-error">{t.last_error}</div>
            {/if}
          </div>

          <div class="target-actions">
            <button
              class="btn btn-sm"
              onclick={() => openBackupPrompt(t)}
              disabled={!isLive(t.target_type)}
              title={isLive(t.target_type) ? 'Run a backup to this target now' : 'Live backup is not yet connected for this target type'}
            >
              Backup Now
            </button>
            <button
              class="btn btn-sm"
              onclick={() => openRestorePanel(t)}
              disabled={!isLive(t.target_type)}
              title={isLive(t.target_type) ? 'Restore the database from this target' : 'Live restore is not yet connected for this target type'}
            >
              Restore…
            </button>
            {#if isPathType(t.target_type)}
              <button
                class="btn btn-sm"
                onclick={() => openSyncPrompt(t)}
                title="Reconcile changes with other devices using this target"
              >
                Sync Now
              </button>
            {/if}
            <button
              class="btn btn-sm btn-danger"
              onclick={() => handleDeleteTarget(t)}
              aria-label={`Delete target ${t.name}`}
              title="Delete this backup target configuration"
            >
              Delete
            </button>
          </div>

          {#if backupPromptFor === t.id}
            <div class="inline-prompt">
              <label for="backup-passphrase-{t.id}">Passphrase for "{t.name}"</label>
              <input
                id="backup-passphrase-{t.id}"
                type="password"
                bind:value={backupPassphrase}
                placeholder="Enter passphrase"
                autocomplete="off"
                title="Passphrase used to encrypt/decrypt backups for this target"
              />
              <div class="inline-prompt-actions">
                <button class="btn btn-sm" onclick={cancelBackupPrompt} title="Cancel">Cancel</button>
                <button
                  class="btn btn-sm btn-primary"
                  onclick={() => handleBackupNow(t)}
                  disabled={backingUpId === t.id || !backupPassphrase}
                  title="Encrypt and upload a backup now"
                >
                  {backingUpId === t.id ? 'Backing up…' : 'Confirm Backup'}
                </button>
              </div>
            </div>
          {/if}

          {#if syncPromptFor === t.id}
            <div class="inline-prompt">
              <label for="sync-passphrase-{t.id}">Passphrase for "{t.name}"</label>
              <input
                id="sync-passphrase-{t.id}"
                type="password"
                bind:value={syncPassphrase}
                placeholder="Enter passphrase"
                autocomplete="off"
                title="Passphrase used to encrypt/decrypt sync segments for this target"
              />
              <div class="inline-prompt-actions">
                <button class="btn btn-sm" onclick={cancelSyncPrompt} title="Cancel">Cancel</button>
                <button
                  class="btn btn-sm btn-primary"
                  onclick={() => handleSyncNow(t)}
                  disabled={syncingId === t.id || !syncPassphrase}
                  title="Publish local changes and pull peer changes from this target"
                >
                  {syncingId === t.id ? 'Syncing…' : 'Confirm Sync'}
                </button>
              </div>
            </div>
          {/if}

          {#if syncResult && syncResult.id === t.id}
            <div class="sync-result-box">
              <div class="sync-stats-grid">
                <div class="sync-stat">
                  <span class="sync-stat-value">{syncResult.peer_segments_found}</span>
                  <span class="sync-stat-label">Peer segments found</span>
                </div>
                <div class="sync-stat">
                  <span class="sync-stat-value">{syncResult.new_changes}</span>
                  <span class="sync-stat-label">New changes</span>
                </div>
                <div class="sync-stat">
                  <span class="sync-stat-value">{syncResult.duplicates}</span>
                  <span class="sync-stat-label">Duplicates</span>
                </div>
                <div class="sync-stat">
                  <span class="sync-stat-value">{syncResult.conflicts_recorded}</span>
                  <span class="sync-stat-label">Conflicts recorded</span>
                </div>
              </div>
              {#if syncResult.conflicts_recorded > 0}
                <div class="warning-box" role="alert" style="margin-top: 10px;">
                  <strong>Conflicts need manual review</strong>
                  <p style="margin:0;">
                    {syncResult.conflicts_recorded} conflicting change{syncResult.conflicts_recorded === 1 ? '' : 's'}
                    were recorded during reconciliation. Review them in the Audit Log, which
                    records conflicting lineage events for this lab.
                  </p>
                </div>
              {/if}
            </div>
          {/if}
        </div>
      {/each}
    </div>
  </DataState>

  {#if !showAddForm}
    <div class="action-row">
      <button class="btn btn-primary btn-sm" onclick={openAddForm} title="Configure a new backup target">
        + Add Target
      </button>
    </div>
  {:else}
    <div class="add-target-form">
      <h3 style="font-size: 14px; font-weight: 700; margin: 20px 0 12px;">Add Backup Target</h3>

      <div class="form-row">
        <div class="form-group">
          <label for="target-name">Name</label>
          <input id="target-name" type="text" bind:value={newTarget.name} placeholder="e.g. Office NAS" autocomplete="off" />
        </div>
        <div class="form-group">
          <label for="target-type">Target Type</label>
          <select id="target-type" bind:value={newTarget.targetType}>
            {#each TARGET_TYPES as opt}
              <option value={opt.value}>{opt.label}</option>
            {/each}
          </select>
          {#if !isLive(newTarget.targetType)}
            <p class="field-hint field-hint-warn">
              Configuration only — live backup/restore not yet connected for this target type.
            </p>
          {/if}
        </div>
      </div>

      <div class="form-group">
        <label for="target-path">
          {isPathType(newTarget.targetType) ? 'Filesystem Path (e.g. a mounted network drive)' : 'Bucket / Remote Path'}
        </label>
        <input
          id="target-path"
          type="text"
          bind:value={newTarget.bucketOrPath}
          placeholder={isPathType(newTarget.targetType) ? '/mnt/lab-nas/backups' : 'my-bucket/stelo-backups'}
          autocomplete="off"
        />
      </div>

      {#if newTarget.targetType === 's3' || newTarget.targetType === 'sftp'}
        <div class="form-row">
          <div class="form-group">
            <label for="target-endpoint">Endpoint (optional)</label>
            <input id="target-endpoint" type="text" bind:value={newTarget.endpoint} placeholder="https://s3.example.com" autocomplete="off" />
          </div>
          <div class="form-group">
            <label for="target-access-key">Access Key (optional)</label>
            <input id="target-access-key" type="text" bind:value={newTarget.accessKey} autocomplete="off" />
          </div>
        </div>
        <div class="form-group">
          <label for="target-secret-key">Secret Key (optional)</label>
          <input id="target-secret-key" type="password" bind:value={newTarget.secretKey} autocomplete="new-password" />
        </div>
      {/if}

      <div class="form-group">
        <label for="target-passphrase">Passphrase</label>
        <input
          id="target-passphrase"
          type="password"
          bind:value={newTarget.passphrase}
          autocomplete="new-password"
          title="Used to encrypt backups for this target — never stored"
        />
        {#if passphraseTooShort}
          <p class="field-hint field-hint-warn">Passphrase should be at least 8 characters.</p>
        {/if}
        <p class="field-hint">This passphrase is not stored anywhere. Keep it safe — it's required for every backup, restore, or sync.</p>
      </div>

      <div class="form-group">
        <label for="target-cron">Schedule (optional, cron)</label>
        <input id="target-cron" type="text" bind:value={newTarget.scheduleCron} placeholder="0 2 * * *" autocomplete="off" />
        <p class="field-hint">5-field cron, e.g. <code>0 2 * * *</code> for 2am daily.</p>
      </div>

      <div class="action-row">
        <button class="btn btn-sm" onclick={cancelAddForm} disabled={creating} title="Discard this target">Cancel</button>
        <button
          class="btn btn-primary btn-sm"
          onclick={handleCreateTarget}
          disabled={creating || !newTarget.name.trim() || !newTarget.bucketOrPath.trim() || newTarget.passphrase.length < 8}
          title="Save this backup target"
        >
          {creating ? 'Creating…' : 'Create Target'}
        </button>
      </div>
    </div>
  {/if}

  {#if restoreTarget}
    <div class="restore-panel">
      <h3 style="font-size: 14px; font-weight: 700; color: #dc2626; margin: 20px 0 8px;">
        ⚠ Restore from "{restoreTarget.name}"
      </h3>
      <p style="font-size:13px; color:#6b7280; margin-bottom:12px;">
        Replaces all current data with the state contained in the selected cloud backup.
        <strong>This cannot be undone.</strong>
      </p>

      {#if restoreStep === 1}
        <div class="form-group">
          <label for="restore-filename">Backup file name</label>
          <input
            id="restore-filename"
            type="text"
            bind:value={restoreFileName}
            placeholder="stelo_cloud_20260615_140000.stelobak"
            style="font-family:monospace;"
            autocomplete="off"
          />
        </div>
        <div class="form-group">
          <label for="restore-passphrase">Passphrase</label>
          <input
            id="restore-passphrase"
            type="password"
            bind:value={restorePassphrase}
            autocomplete="off"
            title="Passphrase used to decrypt this backup"
          />
        </div>
        <div style="display:flex; gap:8px;">
          <button class="btn btn-sm" onclick={cancelRestore} title="Cancel the restore">Cancel</button>
          <button
            class="btn btn-danger btn-sm"
            onclick={confirmRestoreStep1}
            disabled={!restoreFileName.trim() || !restorePassphrase}
            title="Proceed to final confirmation"
          >
            Yes, continue →
          </button>
        </div>
      {:else}
        <p style="font-size:13px; margin-bottom:10px;">
          You are about to restore:<br />
          <strong style="font-family:monospace; font-size:11px;">{restoreFileName.trim()}</strong><br />
          <span style="color:#dc2626;">All current data will be permanently replaced.</span>
        </p>
        <p style="font-size:12px; font-weight:600; margin-bottom:8px;">
          Final confirmation — type <code>RESTORE</code> to proceed:
        </p>
        <input
          type="text"
          bind:value={restorePhrase}
          placeholder="RESTORE"
          style="margin-bottom:8px; font-family:monospace;"
          title="Type RESTORE exactly to unlock the restore button"
        />
        <div style="display:flex; gap:8px;">
          <button class="btn btn-sm" onclick={cancelRestore} title="Cancel the restore">Cancel</button>
          <button
            class="btn btn-danger btn-sm"
            onclick={handleRestore}
            disabled={restoring || restorePhrase !== 'RESTORE'}
            title="Permanently replace the current database with the selected backup"
          >
            {restoring ? 'Restoring…' : 'Restore Now'}
          </button>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .info-notice {
    background: #eff6ff;
    border: 1px solid #bfdbfe;
    border-radius: 8px;
    padding: 14px 16px;
    font-size: 13px;
    color: #1e40af;
  }
  .info-notice strong { display: block; font-weight: 700; margin-bottom: 6px; }
  .info-notice p { margin: 0; line-height: 1.6; }
  :global(.dark) .info-notice { background: #1e3a5f; border-color: #2563eb; color: #bfdbfe; }

  .warning-box {
    background: #fef9c3;
    border: 1px solid #fde047;
    border-radius: 8px;
    padding: 14px 16px;
    font-size: 13px;
    color: #713f12;
  }
  .warning-box strong { display: block; font-weight: 700; margin-bottom: 8px; }
  :global(.dark) .warning-box { background: #422006; border-color: #92400e; color: #fef3c7; }

  .target-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin-bottom: 16px;
  }
  .target-row {
    border: 1px solid var(--color-border, #e2e8f0);
    border-radius: 8px;
    padding: 14px 16px;
  }
  .target-main {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 10px;
  }
  .target-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 14px;
  }
  .target-meta {
    font-size: 12px;
    color: var(--color-text-muted, #6b7280);
  }
  .target-error {
    font-size: 12px;
    color: var(--color-danger, #dc2626);
    margin-top: 4px;
  }
  .target-actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
  .badge-pill {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 10px;
    font-size: 11px;
    font-weight: 600;
  }

  .inline-prompt {
    margin-top: 12px;
    padding: 12px;
    background: var(--color-surface-raised, #f8fafc);
    border-radius: 6px;
  }
  .inline-prompt label {
    display: block;
    font-size: 12px;
    font-weight: 600;
    margin-bottom: 6px;
  }
  .inline-prompt-actions {
    display: flex;
    gap: 8px;
    margin-top: 8px;
  }

  .sync-result-box {
    margin-top: 12px;
    padding: 12px;
    background: var(--color-surface-raised, #f8fafc);
    border-radius: 6px;
  }
  .sync-stats-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 12px;
  }
  .sync-stat {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 10px 6px;
    background: var(--color-surface, #ffffff);
    border-radius: 8px;
  }
  .sync-stat-value {
    font-size: 18px;
    font-weight: 700;
    color: var(--color-text, #1e293b);
  }
  .sync-stat-label {
    font-size: 10px;
    color: var(--color-text-muted, #6b7280);
    margin-top: 2px;
    text-align: center;
  }

  .add-target-form {
    border-top: 1px solid var(--color-border, #e2e8f0);
    padding-top: 4px;
  }
  .field-hint {
    font-size: 12px;
    color: var(--color-text-muted, #6b7280);
    margin-top: 4px;
  }
  .field-hint-warn {
    color: #92400e;
  }
  :global(.dark) .field-hint-warn { color: #fbbf24; }

  .restore-panel {
    margin-top: 16px;
    border-top: 1px solid var(--color-border, #e2e8f0);
    padding-top: 12px;
  }

  .action-row {
    display: flex;
    gap: 10px;
    margin-top: 16px;
  }

  code {
    font-family: 'Courier New', monospace;
    background: #f1f5f9;
    padding: 1px 5px;
    border-radius: 3px;
    font-size: 12px;
  }
  :global(.dark) code { background: #0f172a; color: #e2e8f0; }

  .new-feature-badge {
    display: inline-block;
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.03em;
    padding: 2px 7px;
    border-radius: 10px;
    background: #dbeafe;
    color: #1e40af;
    vertical-align: middle;
  }
  :global(.dark) .new-feature-badge { background: #1e3a5f; color: #93c5fd; }
</style>
