<script lang="ts">
  import { loadDemoData } from '../api';
  import { navigateTo, addNotification } from '../stores/app';
  import { currentUser } from '../stores/auth';

  interface Props {
    onAddSpecimen?: () => void;
    onDemoLoaded: () => void;
  }

  let { onAddSpecimen, onDemoLoaded }: Props = $props();

  let loadingDemo = $state(false);

  async function handleLoadDemo() {
    loadingDemo = true;
    try {
      const msg = await loadDemoData();
      addNotification(msg, 'success');
      onDemoLoaded();
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      loadingDemo = false;
    }
  }

  function handleAddSpecimen() {
    if (onAddSpecimen) {
      onAddSpecimen();
    } else {
      navigateTo('specimens');
    }
  }

  const canManage = $derived(
    $currentUser?.role === 'admin' || $currentUser?.role === 'supervisor'
  );
</script>

<div class="first-run">
  <div class="first-run-inner">
    <div class="first-run-icon">🌿</div>
    <h2 class="first-run-title">Welcome to SteloPTC</h2>
    <p class="first-run-subtitle">
      Your lab is set up and ready. Follow the steps below to get started,
      or load sample data to explore immediately.
    </p>

    <div class="steps">
      <div class="step">
        <div class="step-number">1</div>
        <div class="step-body">
          <div class="step-heading">Configure your species registry</div>
          <div class="step-desc">
            Add the plant species you work with — genus, species name, common name,
            and recommended subculture interval.
          </div>
          <button class="btn btn-primary step-btn" onclick={() => navigateTo('species')}>
            Manage Species →
          </button>
        </div>
      </div>

      <div class="step-connector"></div>

      <div class="step">
        <div class="step-number">2</div>
        <div class="step-body">
          <div class="step-heading">Accession your first specimen</div>
          <div class="step-desc">
            Register a tissue culture accession — the app auto-generates the
            accession number from the species code and initiation date.
          </div>
          <button class="btn btn-primary step-btn" onclick={handleAddSpecimen}>
            Add First Specimen →
          </button>
        </div>
      </div>
    </div>

    <div class="demo-section">
      <div class="demo-divider"><span>or explore with sample data</span></div>
      <p class="demo-desc">
        Load a ready-made sample lab with Asparagus, Nandina, and Citrus
        specimens — including media batches and passage history — so you can
        try every feature right now.
      </p>
      {#if canManage}
        <button
          class="btn btn-demo"
          onclick={handleLoadDemo}
          disabled={loadingDemo}
          title="Creates demo specimens using the built-in species registry"
        >
          {loadingDemo ? 'Loading sample data…' : '🧪 Load Sample Data'}
        </button>
        <p class="demo-remove-note">
          Sample data is clearly labelled and can be removed at any time via
          <strong>Dashboard → Dev Tools → Reset Database</strong>.
        </p>
      {:else}
        <p class="demo-remove-note">
          Ask a supervisor or admin to load sample data.
        </p>
      {/if}
    </div>
  </div>
</div>

<style>
  .first-run {
    display: flex;
    justify-content: center;
    padding: 40px 16px;
  }

  .first-run-inner {
    max-width: 600px;
    width: 100%;
    text-align: center;
  }

  .first-run-icon {
    font-size: 56px;
    margin-bottom: 16px;
    line-height: 1;
  }

  .first-run-title {
    font-size: 26px;
    font-weight: 800;
    margin-bottom: 10px;
    color: #1e293b;
  }
  :global(.dark) .first-run-title { color: #f1f5f9; }

  .first-run-subtitle {
    font-size: 14px;
    color: #6b7280;
    margin-bottom: 36px;
    line-height: 1.6;
  }

  /* ── Steps ── */
  .steps {
    display: flex;
    flex-direction: column;
    gap: 0;
    margin-bottom: 40px;
    text-align: left;
  }

  .step {
    display: flex;
    gap: 20px;
    align-items: flex-start;
    background: white;
    border: 1px solid #e2e8f0;
    border-radius: 10px;
    padding: 20px 24px;
  }
  :global(.dark) .step { background: #1e293b; border-color: #334155; }

  .step-connector {
    width: 2px;
    height: 20px;
    background: #e2e8f0;
    margin: 0 auto;
    flex-shrink: 0;
  }
  :global(.dark) .step-connector { background: #334155; }

  .step-number {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    background: #ffd2a1;
    color: #1a0e00;
    font-size: 16px;
    font-weight: 800;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    margin-top: 2px;
  }

  .step-body {
    flex: 1;
  }

  .step-heading {
    font-size: 15px;
    font-weight: 700;
    margin-bottom: 6px;
    color: #1e293b;
  }
  :global(.dark) .step-heading { color: #f1f5f9; }

  .step-desc {
    font-size: 13px;
    color: #6b7280;
    line-height: 1.5;
    margin-bottom: 14px;
  }

  .step-btn {
    font-size: 13px;
  }

  /* ── Demo section ── */
  .demo-section {
    background: white;
    border: 1px solid #e2e8f0;
    border-radius: 10px;
    padding: 24px;
  }
  :global(.dark) .demo-section { background: #1e293b; border-color: #334155; }

  .demo-divider {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 16px;
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: #9ca3af;
  }
  .demo-divider::before,
  .demo-divider::after {
    content: '';
    flex: 1;
    height: 1px;
    background: #e2e8f0;
  }
  :global(.dark) .demo-divider::before,
  :global(.dark) .demo-divider::after { background: #334155; }

  .demo-desc {
    font-size: 13px;
    color: #6b7280;
    line-height: 1.6;
    margin-bottom: 16px;
  }

  .btn-demo {
    background: #059669;
    color: white;
    border: none;
    padding: 10px 24px;
    border-radius: 6px;
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.15s;
  }
  .btn-demo:hover:not(:disabled) { background: #047857; }
  .btn-demo:disabled { opacity: 0.6; cursor: not-allowed; }

  .demo-remove-note {
    font-size: 12px;
    color: #9ca3af;
    margin-top: 12px;
    line-height: 1.5;
  }

  @media (max-width: 640px) {
    .first-run { padding: 24px 12px; }
    .first-run-title { font-size: 22px; }
    .first-run-icon { font-size: 44px; }
  }
</style>
