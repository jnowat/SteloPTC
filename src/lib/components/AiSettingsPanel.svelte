<script lang="ts">
  import { onMount } from 'svelte';
  import { getAiConfig, setAiConfig, getAiStatus, type AiStatus } from '../api';
  import { addNotification } from '../stores/app';

  // WP-56b: configure and health-check the local AI runtime. Two runtimes are
  // supported, both fully on-device — Ollama (default) and any
  // OpenAI-compatible server such as LocalAI. Nothing here ever calls a cloud
  // service; the base URL is always a loopback/LAN host.

  const PROVIDERS: { value: string; label: string; defaultUrl: string; hint: string }[] = [
    {
      value: 'ollama',
      label: 'Ollama (recommended)',
      defaultUrl: 'http://127.0.0.1:11434',
      hint: 'The default local model runtime. Install from ollama.com, then run `ollama pull llama3.1` and `ollama pull llava`.',
    },
    {
      value: 'localai',
      label: 'LocalAI (OpenAI-compatible)',
      defaultUrl: 'http://127.0.0.1:8080',
      hint: 'Any OpenAI-compatible server (LocalAI, or Ollama’s /v1 shim). Uses /v1/chat/completions and /v1/models.',
    },
  ];

  let provider = $state('ollama');
  let baseUrl = $state('http://127.0.0.1:11434');
  let textModel = $state('llama3.1');
  let visionModel = $state('llava');

  let loading = $state(true);
  let saving = $state(false);
  let checking = $state(false);
  let status = $state<AiStatus | null>(null);

  function providerHint(p: string): string {
    return PROVIDERS.find((x) => x.value === p)?.hint ?? '';
  }

  async function load() {
    loading = true;
    try {
      const cfg = await getAiConfig();
      provider = cfg.provider || 'ollama';
      baseUrl = cfg.base_url;
      textModel = cfg.text_model;
      visionModel = cfg.vision_model;
    } catch (e: any) {
      addNotification(e?.message || 'Failed to load AI configuration', 'error');
    } finally {
      loading = false;
    }
  }

  onMount(load);

  // When the provider changes and the URL is still a known default, swap in the
  // matching default port so the user doesn't have to remember it.
  function onProviderChange() {
    const known = PROVIDERS.map((p) => p.defaultUrl);
    if (known.includes(baseUrl)) {
      baseUrl = PROVIDERS.find((p) => p.value === provider)?.defaultUrl ?? baseUrl;
    }
  }

  async function save() {
    saving = true;
    try {
      await setAiConfig(provider, baseUrl.trim(), textModel.trim(), visionModel.trim());
      addNotification('AI settings saved', 'success');
      await checkStatus();
    } catch (e: any) {
      addNotification(e?.message || 'Failed to save AI settings', 'error');
    } finally {
      saving = false;
    }
  }

  async function checkStatus() {
    checking = true;
    try {
      status = await getAiStatus();
    } catch (e: any) {
      status = null;
      addNotification(e?.message || 'Failed to check AI status', 'error');
    } finally {
      checking = false;
    }
  }
</script>

<div class="card" style="max-width: 900px; margin-top: 24px;">
  <h2 style="font-size: 16px; font-weight: 700; margin-bottom: 4px;">
    AI Assistant (Local) <span class="new-feature-badge">New</span>
  </h2>
  <p style="font-size: 13px; color: #6b7280; margin-bottom: 16px;">
    On-device AI for note summaries, passage-comment drafts, and photo contamination checks.
    Every suggestion is a draft that a person must explicitly approve before it touches a record.
  </p>

  <div class="info-notice" role="note" style="margin-bottom: 16px;">
    <strong>Nothing leaves this machine</strong>
    <p>
      All AI runs against a local runtime you control — Ollama or LocalAI on this computer or your
      LAN. SteloPTC never sends specimen data, notes, or photos to any cloud AI service.
      AI assistance is entirely optional; the app works identically with it turned off.
    </p>
  </div>

  {#if loading}
    <p style="font-size: 13px; color: #6b7280;">Loading…</p>
  {:else}
    <div class="form-row">
      <div class="form-group">
        <label for="ai-provider">Runtime</label>
        <select id="ai-provider" bind:value={provider} onchange={onProviderChange}>
          {#each PROVIDERS as p}
            <option value={p.value}>{p.label}</option>
          {/each}
        </select>
      </div>
      <div class="form-group">
        <label for="ai-base-url">Base URL</label>
        <input id="ai-base-url" type="text" bind:value={baseUrl} placeholder="http://127.0.0.1:11434" autocomplete="off" style="font-family:monospace;" />
      </div>
    </div>
    <p class="field-hint" style="margin-top:-4px; margin-bottom:12px;">{providerHint(provider)}</p>

    <div class="form-row">
      <div class="form-group">
        <label for="ai-text-model">Text model</label>
        <input id="ai-text-model" type="text" bind:value={textModel} placeholder="llama3.1" autocomplete="off" />
        <p class="field-hint">Used for note summaries and passage-comment drafts.</p>
      </div>
      <div class="form-group">
        <label for="ai-vision-model">Vision model</label>
        <input id="ai-vision-model" type="text" bind:value={visionModel} placeholder="llava" autocomplete="off" />
        <p class="field-hint">Used for photo contamination analysis. Must be a vision-capable model.</p>
      </div>
    </div>

    <div class="action-row">
      <button class="btn btn-primary btn-sm" onclick={save} disabled={saving} title="Save AI settings">
        {saving ? 'Saving…' : 'Save Settings'}
      </button>
      <button class="btn btn-sm" onclick={checkStatus} disabled={checking} title="Check whether the local AI runtime is reachable and which models are installed">
        {checking ? 'Checking…' : 'Test Connection'}
      </button>
    </div>

    {#if status}
      <div class="status-box" class:status-ok={status.reachable} class:status-bad={!status.reachable}>
        {#if status.reachable}
          <div class="status-title">✓ Reachable at <code>{status.base_url}</code></div>
          <div class="status-detail">
            {status.models.length} model{status.models.length === 1 ? '' : 's'} installed:
            {#if status.models.length > 0}
              {#each status.models as m}
                <span class="model-pill">{m}</span>
              {/each}
            {:else}
              <em>none yet — pull a model to get started</em>
            {/if}
          </div>
          <div class="status-detail" style="margin-top:8px;">
            <span class="check-line" class:ok={status.text_model_installed} class:bad={!status.text_model_installed}>
              {status.text_model_installed ? '✓' : '✗'} Text model <code>{status.text_model}</code>
              {status.text_model_installed ? 'installed' : 'not found'}
            </span>
            <br />
            <span class="check-line" class:ok={status.vision_model_installed} class:bad={!status.vision_model_installed}>
              {status.vision_model_installed ? '✓' : '✗'} Vision model <code>{status.vision_model}</code>
              {status.vision_model_installed ? 'installed' : 'not found'}
            </span>
          </div>
          {#if !status.text_model_installed || !status.vision_model_installed}
            <div class="status-hint">
              {#if status.provider === 'ollama'}
                Install missing models with <code>ollama pull {status.text_model}</code>
                {#if !status.vision_model_installed}and <code>ollama pull {status.vision_model}</code>{/if}.
              {:else}
                Add the missing model(s) to your LocalAI server, or change the model name above.
              {/if}
            </div>
          {/if}
        {:else}
          <div class="status-title">✗ Not reachable</div>
          <div class="status-detail">{status.error}</div>
          {#if status.provider === 'ollama'}
            <div class="status-hint">Start Ollama (<code>ollama serve</code>) and make sure it is listening at <code>{status.base_url}</code>.</div>
          {:else}
            <div class="status-hint">Start your LocalAI server and make sure it is listening at <code>{status.base_url}</code>.</div>
          {/if}
        {/if}
      </div>
    {/if}
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

  .form-row {
    display: flex;
    gap: 16px;
    flex-wrap: wrap;
  }
  .form-group {
    flex: 1;
    min-width: 220px;
    margin-bottom: 12px;
  }
  .form-group label {
    display: block;
    font-size: 12px;
    font-weight: 600;
    margin-bottom: 6px;
  }
  .field-hint {
    font-size: 12px;
    color: var(--color-text-muted, #6b7280);
    margin-top: 4px;
  }
  .action-row {
    display: flex;
    gap: 10px;
    margin-top: 8px;
    margin-bottom: 4px;
  }

  .status-box {
    margin-top: 16px;
    padding: 14px 16px;
    border-radius: 8px;
    font-size: 13px;
    border: 1px solid var(--color-border, #e2e8f0);
  }
  .status-ok { background: #f0fdf4; border-color: #bbf7d0; color: #166534; }
  .status-bad { background: #fef2f2; border-color: #fecaca; color: #991b1b; }
  :global(.dark) .status-ok { background: #052e16; border-color: #166534; color: #bbf7d0; }
  :global(.dark) .status-bad { background: #450a0a; border-color: #991b1b; color: #fecaca; }
  .status-title { font-weight: 700; margin-bottom: 6px; }
  .status-detail { line-height: 1.6; }
  .status-hint { margin-top: 8px; font-size: 12px; opacity: 0.85; }
  .check-line.ok { color: #166534; }
  .check-line.bad { color: #991b1b; }
  :global(.dark) .check-line.ok { color: #86efac; }
  :global(.dark) .check-line.bad { color: #fca5a5; }

  .model-pill {
    display: inline-block;
    padding: 2px 8px;
    margin: 2px 3px;
    border-radius: 10px;
    font-size: 11px;
    font-weight: 600;
    background: #dbeafe;
    color: #1e40af;
    font-family: monospace;
  }
  :global(.dark) .model-pill { background: #1e3a5f; color: #93c5fd; }

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
