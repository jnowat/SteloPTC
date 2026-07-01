<script lang="ts">
  import { onMount } from 'svelte';
  import {
    listInstalledPlugins, validatePluginManifest, installPlugin,
    installPluginFromZip, uninstallPlugin, type InstalledPlugin,
  } from '../api';
  import { addNotification } from '../stores/app';
  import DataState from './DataState.svelte';

  let plugins = $state<InstalledPlugin[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  async function load() {
    loading = true;
    error = null;
    try {
      plugins = await listInstalledPlugins();
    } catch (e: any) {
      error = e.message;
    } finally {
      loading = false;
    }
  }

  onMount(load);

  // ── Install from manifest JSON ──────────────────────────────────────────
  let showManifestForm = $state(false);
  let manifestText = $state('');
  let manifestPreview = $state<any>(null);
  let validating = $state(false);
  let installingManifest = $state(false);
  let manifestFileInput: HTMLInputElement | null = $state(null);

  function openManifestForm() {
    showManifestForm = true;
    showZipForm = false;
    manifestText = '';
    manifestPreview = null;
  }

  function cancelManifestForm() {
    showManifestForm = false;
    manifestText = '';
    manifestPreview = null;
  }

  async function handleManifestFileChosen(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    manifestText = await file.text();
    manifestPreview = null;
  }

  async function handleValidateManifest() {
    if (!manifestText.trim()) {
      addNotification('Paste or upload a manifest.json first.', 'warning');
      return;
    }
    validating = true;
    manifestPreview = null;
    try {
      manifestPreview = await validatePluginManifest(manifestText);
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      validating = false;
    }
  }

  async function handleInstallManifest() {
    if (!manifestPreview) return;
    installingManifest = true;
    try {
      const installed = await installPlugin(manifestText);
      addNotification(`Plugin "${installed.plugin_name}" v${installed.version} installed`, 'success');
      showManifestForm = false;
      manifestText = '';
      manifestPreview = null;
      await load();
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      installingManifest = false;
    }
  }

  // ── Install from .steloplugin zip ───────────────────────────────────────
  let showZipForm = $state(false);
  let installingZip = $state(false);
  let zipFileName = $state('');

  function openZipForm() {
    showZipForm = true;
    showManifestForm = false;
    zipFileName = '';
  }

  function cancelZipForm() {
    showZipForm = false;
    zipFileName = '';
  }

  function readFileAsBase64(file: File): Promise<string> {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => {
        const result = reader.result as string;
        // Strip the data URL prefix (e.g. "data:application/zip;base64,")
        const base64 = result.includes(',') ? result.split(',')[1] : result;
        resolve(base64);
      };
      reader.onerror = () => reject(reader.error);
      reader.readAsDataURL(file);
    });
  }

  async function handleZipFileChosen(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    zipFileName = file.name;
    installingZip = true;
    try {
      const b64 = await readFileAsBase64(file);
      const installed = await installPluginFromZip(b64);
      addNotification(`Plugin "${installed.plugin_name}" v${installed.version} installed`, 'success');
      showZipForm = false;
      zipFileName = '';
      input.value = '';
      await load();
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      installingZip = false;
    }
  }

  // ── Uninstall (confirm step) ────────────────────────────────────────────
  let confirmUninstallId = $state<string | null>(null);
  let uninstallingId = $state<string | null>(null);

  function openUninstallConfirm(p: InstalledPlugin) {
    confirmUninstallId = p.id;
  }

  function cancelUninstallConfirm() {
    confirmUninstallId = null;
  }

  async function handleUninstall(p: InstalledPlugin) {
    uninstallingId = p.id;
    try {
      await uninstallPlugin(p.id);
      addNotification(`Plugin "${p.plugin_name}" uninstalled`, 'success');
      confirmUninstallId = null;
      await load();
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      uninstallingId = null;
    }
  }

  function formatDate(iso: string): string {
    try {
      return new Date(iso).toLocaleString();
    } catch {
      return iso;
    }
  }
</script>

<div class="card" style="max-width: 900px; margin-top: 24px;">
  <h2 style="font-size: 16px; font-weight: 700; margin-bottom: 4px;">
    Plugin Manager <span class="new-feature-badge">New</span>
  </h2>
  <p style="font-size: 13px; color: #6b7280; margin-bottom: 20px;">
    Install and manage extensions that add vocabulary, dashboard panels, compliance rule
    definitions, and report templates.
  </p>

  <DataState {loading} {error} empty={!loading && !error && plugins.length === 0}
    emptyIcon="🧩"
    emptyTitle="No plugins installed"
    emptyMessage="Install a plugin manifest or .steloplugin package to get started."
    onretry={load}>

    <div class="plugin-list">
      {#each plugins as p (p.id)}
        <div class="plugin-row">
          <div class="plugin-main">
            <div class="plugin-title">
              <strong>{p.plugin_name}</strong>
              <span class="badge-gray badge-pill">v{p.version}</span>
              {#if p.profile}
                <span class="badge-blue badge-pill">{p.profile}</span>
              {/if}
              {#if p.vocabulary_seeded}
                <span class="badge-green badge-pill">Vocabulary seeded</span>
              {/if}
            </div>
            <div class="plugin-meta">Installed: {formatDate(p.installed_at)}</div>
          </div>

          <div class="plugin-actions">
            {#if confirmUninstallId !== p.id}
              <button
                class="btn btn-sm btn-danger"
                onclick={() => openUninstallConfirm(p)}
                aria-label={`Uninstall plugin ${p.plugin_name}`}
                title="Uninstall this plugin"
              >
                Uninstall
              </button>
            {/if}
          </div>

          {#if confirmUninstallId === p.id}
            <div class="inline-prompt">
              <p style="margin: 0 0 8px;">
                Uninstall <strong>{p.plugin_name}</strong>?
              </p>
              <div class="info-notice" role="note" style="margin-bottom: 10px;">
                <p style="margin:0;">
                  Vocabulary entries already seeded by this plugin will <strong>not</strong> be
                  removed — this is intentional, so existing records that reference them remain
                  valid. Only the plugin registration is removed.
                </p>
              </div>
              <div style="display:flex; gap:8px;">
                <button class="btn btn-sm" onclick={cancelUninstallConfirm} title="Cancel">Cancel</button>
                <button
                  class="btn btn-sm btn-danger"
                  onclick={() => handleUninstall(p)}
                  disabled={uninstallingId === p.id}
                  title="Confirm uninstall"
                >
                  {uninstallingId === p.id ? 'Uninstalling…' : 'Confirm Uninstall'}
                </button>
              </div>
            </div>
          {/if}
        </div>
      {/each}
    </div>
  </DataState>

  <div class="action-row">
    <button class="btn btn-primary btn-sm" onclick={openManifestForm} title="Install a plugin from a manifest.json file">
      + Install from Manifest (.json)
    </button>
    <button class="btn btn-sm" onclick={openZipForm} title="Install a plugin from a .steloplugin package">
      + Install from .steloplugin
    </button>
  </div>

  {#if showManifestForm}
    <div class="install-form">
      <h3 style="font-size: 14px; font-weight: 700; margin: 20px 0 12px;">Install from Manifest</h3>

      <div class="form-group">
        <label for="manifest-file">Upload manifest.json</label>
        <input id="manifest-file" type="file" accept=".json,application/json" bind:this={manifestFileInput} onchange={handleManifestFileChosen} />
      </div>

      <div class="form-group">
        <label for="manifest-text">Or paste manifest JSON</label>
        <textarea
          id="manifest-text"
          rows="8"
          bind:value={manifestText}
          placeholder={'{ "name": "...", "version": "1.0.0", ... }'}
          style="font-family: monospace; font-size: 12px; width: 100%;"
        ></textarea>
      </div>

      <div class="action-row">
        <button class="btn btn-sm" onclick={cancelManifestForm} title="Cancel">Cancel</button>
        <button
          class="btn btn-sm btn-primary"
          onclick={handleValidateManifest}
          disabled={validating || !manifestText.trim()}
          title="Validate and preview this manifest"
        >
          {validating ? 'Validating…' : 'Validate & Preview'}
        </button>
      </div>

      {#if manifestPreview}
        <div class="manifest-preview">
          <h4 style="font-size: 13px; font-weight: 700; margin: 16px 0 8px;">Preview</h4>
          <div class="preview-grid">
            <div><span class="preview-label">Name</span><span>{manifestPreview.name}</span></div>
            <div><span class="preview-label">Version</span><span>{manifestPreview.version}</span></div>
            <div><span class="preview-label">Profile</span><span>{manifestPreview.profile ?? '—'}</span></div>
            <div><span class="preview-label">Vocabulary rows</span><span>{manifestPreview.vocabulary_seed?.length ?? 0}</span></div>
            <div><span class="preview-label">Dashboard panels</span><span>{manifestPreview.dashboard_panels?.length ?? 0}</span></div>
            <div><span class="preview-label">Report templates</span><span>{manifestPreview.report_templates?.length ?? 0}</span></div>
          </div>

          {#if manifestPreview.compliance_rules?.length > 0}
            <div class="info-notice" role="note" style="margin-top: 12px;">
              <strong>{manifestPreview.compliance_rules.length} compliance rule{manifestPreview.compliance_rules.length === 1 ? '' : 's'} included</strong>
              <p>
                Compliance rule execution is not yet wired up — {manifestPreview.compliance_rules.length === 1 ? 'this rule is' : 'these rules are'}
                recorded but will not run automatically.
              </p>
            </div>
          {/if}

          <div class="action-row">
            <button
              class="btn btn-sm btn-primary"
              onclick={handleInstallManifest}
              disabled={installingManifest}
              title="Install this plugin"
            >
              {installingManifest ? 'Installing…' : 'Install Plugin'}
            </button>
          </div>
        </div>
      {/if}
    </div>
  {/if}

  {#if showZipForm}
    <div class="install-form">
      <h3 style="font-size: 14px; font-weight: 700; margin: 20px 0 12px;">Install from .steloplugin package</h3>
      <p style="font-size: 12px; color: #6b7280; margin-bottom: 12px;">
        The zip file must contain a top-level <code>manifest.json</code>. There is no preview
        step for zip packages — installation runs immediately after upload.
      </p>
      <div class="form-group">
        <label for="zip-file">Upload .steloplugin file</label>
        <input id="zip-file" type="file" accept=".steloplugin" onchange={handleZipFileChosen} disabled={installingZip} />
      </div>
      {#if installingZip}
        <p style="font-size: 12px; color: #6b7280;">Installing {zipFileName}…</p>
      {/if}
      <div class="action-row">
        <button class="btn btn-sm" onclick={cancelZipForm} disabled={installingZip} title="Cancel">Cancel</button>
      </div>
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

  .plugin-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin-bottom: 16px;
  }
  .plugin-row {
    border: 1px solid var(--color-border, #e2e8f0);
    border-radius: 8px;
    padding: 14px 16px;
  }
  .plugin-main {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 10px;
  }
  .plugin-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 14px;
    flex-wrap: wrap;
  }
  .plugin-meta {
    font-size: 12px;
    color: var(--color-text-muted, #6b7280);
  }
  .plugin-actions {
    display: flex;
    gap: 8px;
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

  .install-form {
    border-top: 1px solid var(--color-border, #e2e8f0);
    padding-top: 4px;
  }

  .manifest-preview {
    border-top: 1px solid var(--color-border, #e2e8f0);
    margin-top: 16px;
    padding-top: 4px;
  }
  .preview-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px 16px;
    font-size: 13px;
  }
  .preview-grid > div {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    padding: 6px 10px;
    background: var(--color-surface-raised, #f8fafc);
    border-radius: 6px;
  }
  .preview-label {
    color: var(--color-text-muted, #6b7280);
    font-weight: 600;
  }

  .action-row {
    display: flex;
    gap: 10px;
    margin-top: 16px;
    flex-wrap: wrap;
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
