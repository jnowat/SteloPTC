<script lang="ts">
  import { addNotification } from '../stores/app';
  import {
    evaluateSubmissionReadiness, createSubmission, reevaluateSubmission,
    generateSubmissionPackage, markSubmissionSubmitted, listSubmissions, runSubmissionMonitor,
    type Readiness, type RegulatorySubmission,
  } from '../api';

  // WP-68: Regulatory submission pipeline. Monitors compliance state, evaluates
  // whether a submission's preconditions are met, and generates + signs the
  // WP-60 bundle when ready. SteloPTC does NOT submit to a government portal —
  // the operator submits the signed package through the official channel and
  // records the returned reference here.

  let { onclose }: { onclose?: () => void } = $props();

  const KINDS = [
    { value: 'part11', label: 'FDA 21 CFR Part 11' },
    { value: 'usda', label: 'USDA APHIS PPQ 526' },
    { value: 'cites', label: 'CITES dossier' },
  ];

  let kind = $state('part11');
  let title = $state('');
  let autoGenerate = $state(false);

  // Kind-specific scope inputs.
  let fromDate = $state('');
  let toDate = $state('');
  let labName = $state('');
  let specimenIds = $state('');
  let authorizedScientist = $state('');
  let rootSpecimenId = $state('');
  let citesAppendix = $state('Appendix II');

  let preview = $state<Readiness | null>(null);
  let submissions = $state<RegulatorySubmission[]>([]);
  let busy = $state(false);
  let refInputs = $state<Record<string, string>>({});

  function buildScope(): Record<string, unknown> {
    if (kind === 'part11') return { from_date: fromDate, to_date: toDate, lab_name: labName };
    if (kind === 'usda') {
      const ids = specimenIds.split(/[\s,]+/).map((s) => s.trim()).filter(Boolean);
      return { specimen_ids: ids, authorized_scientist: authorizedScientist };
    }
    return { root_specimen_id: rootSpecimenId, cites_appendix: citesAppendix };
  }

  async function load() {
    try {
      submissions = await listSubmissions();
    } catch (e: any) {
      addNotification(e?.message || 'Failed to load submissions', 'error');
    }
  }

  $effect(() => { if (submissions.length === 0) load(); });

  async function doEvaluate() {
    busy = true;
    try {
      preview = await evaluateSubmissionReadiness(kind, buildScope());
    } catch (e: any) {
      addNotification(e?.message || 'Readiness check failed', 'error');
    } finally {
      busy = false;
    }
  }

  async function doCreate() {
    if (!title.trim()) { addNotification('Give the submission a title first.', 'error'); return; }
    busy = true;
    try {
      await createSubmission(kind, title.trim(), buildScope(), autoGenerate);
      addNotification('Submission created.', 'success');
      title = '';
      preview = null;
      await load();
    } catch (e: any) {
      addNotification(e?.message || 'Failed to create submission', 'error');
    } finally {
      busy = false;
    }
  }

  async function doReevaluate(s: RegulatorySubmission) {
    busy = true;
    try {
      await reevaluateSubmission(s.id);
      await load();
    } catch (e: any) {
      addNotification(e?.message || 'Re-evaluation failed', 'error');
    } finally {
      busy = false;
    }
  }

  async function doGenerate(s: RegulatorySubmission) {
    busy = true;
    try {
      const updated = await generateSubmissionPackage(s.id);
      addNotification(`Signed package generated: ${updated.package_path}`, 'success');
      await load();
    } catch (e: any) {
      addNotification(e?.message || 'Package generation failed', 'error');
    } finally {
      busy = false;
    }
  }

  async function doMarkSubmitted(s: RegulatorySubmission) {
    const reference = (refInputs[s.id] || '').trim();
    if (!reference) { addNotification('Enter the submission reference first.', 'error'); return; }
    busy = true;
    try {
      await markSubmissionSubmitted(s.id, reference);
      addNotification('Submission marked as submitted.', 'success');
      refInputs[s.id] = '';
      await load();
    } catch (e: any) {
      addNotification(e?.message || 'Failed to record submission', 'error');
    } finally {
      busy = false;
    }
  }

  async function doMonitor() {
    busy = true;
    try {
      const r = await runSubmissionMonitor();
      addNotification(
        `Monitor: ${r.evaluated} evaluated, ${r.became_ready} ready, ${r.auto_generated} auto-generated, ${r.still_blocked} blocked.`,
        'success',
      );
      await load();
    } catch (e: any) {
      addNotification(e?.message || 'Monitor run failed', 'error');
    } finally {
      busy = false;
    }
  }

  function statusClass(status: string): string {
    if (status === 'ready') return 'sub-ready';
    if (status === 'blocked') return 'sub-blocked';
    if (status === 'generated') return 'sub-generated';
    if (status === 'submitted' || status === 'acknowledged') return 'sub-submitted';
    return 'sub-draft';
  }

  function short(s: string | null, n = 14): string {
    if (!s) return '—';
    return s.length > n ? `…${s.slice(-n)}` : s;
  }
</script>

<div class="card sub-panel" style="margin-bottom:16px;">
  <div class="sub-header">
    <strong>📤 Regulatory Submission Pipeline</strong>
    {#if onclose}<button class="btn btn-sm" onclick={onclose}>Close</button>{/if}
  </div>
  <p class="sub-intro">
    Monitor compliance state and, when a submission's preconditions are met, generate and
    sign the corresponding regulatory bundle (built on the FDA/USDA/CITES exports).
    SteloPTC produces a ready-to-submit signed package — it does <strong>not</strong> submit
    to a government portal; submit through the official channel and record the reference here.
    See <code>docs/regulatory-exports.md</code>.
  </p>

  <!-- Create form -->
  <div class="sub-create">
    <div class="sub-row">
      <label>Type
        <select bind:value={kind}>
          {#each KINDS as k}<option value={k.value}>{k.label}</option>{/each}
        </select>
      </label>
      <label class="sub-grow">Title
        <input bind:value={title} placeholder="e.g. Q2 2026 Part 11 attestation" />
      </label>
    </div>

    {#if kind === 'part11'}
      <div class="sub-row">
        <label>From date <input type="date" bind:value={fromDate} /></label>
        <label>To date <input type="date" bind:value={toDate} /></label>
        <label class="sub-grow">Lab name <input bind:value={labName} placeholder="Laboratory name" /></label>
      </div>
    {:else if kind === 'usda'}
      <div class="sub-row">
        <label class="sub-grow">Specimen IDs <input bind:value={specimenIds} placeholder="comma or space separated specimen IDs" /></label>
        <label class="sub-grow">Authorized scientist <input bind:value={authorizedScientist} placeholder="Dr. …" /></label>
      </div>
    {:else}
      <div class="sub-row">
        <label class="sub-grow">Root specimen ID <input bind:value={rootSpecimenId} placeholder="specimen id" /></label>
        <label>CITES Appendix
          <select bind:value={citesAppendix}>
            <option>Appendix I</option>
            <option>Appendix II</option>
            <option>Appendix III</option>
          </select>
        </label>
      </div>
    {/if}

    <div class="sub-row sub-actions">
      <label class="sub-check"><input type="checkbox" bind:checked={autoGenerate} /> Auto-generate when ready</label>
      <button class="btn btn-sm" disabled={busy} onclick={doEvaluate}>Check Readiness</button>
      <button class="btn btn-sm btn-primary" disabled={busy} onclick={doCreate}>Create Submission</button>
      <button class="btn btn-sm" disabled={busy} onclick={doMonitor} title="Re-evaluate all submissions and auto-generate ready ones">Run Monitor</button>
    </div>

    {#if preview}
      <div class="sub-readiness {preview.ready ? 'sub-ready-box' : 'sub-blocked-box'}">
        <strong>{preview.ready ? '✓ Ready' : `✗ Blocked (${preview.blocking_count})`}</strong>
        <ul>
          {#each preview.checks as c}
            <li class={c.passed ? 'chk-ok' : 'chk-fail'}>
              {c.passed ? '✓' : '✗'} {c.label} <span class="chk-detail">— {c.detail}</span>
            </li>
          {/each}
        </ul>
      </div>
    {/if}
  </div>

  <!-- Submissions list -->
  {#if submissions.length === 0}
    <p class="sub-empty">No submissions yet — create one above.</p>
  {:else}
    <div class="sub-table-wrap">
      <table class="sub-table">
        <thead>
          <tr><th>Status</th><th>Type</th><th>Title</th><th>Package / Ref</th><th>Actions</th></tr>
        </thead>
        <tbody>
          {#each submissions as s}
            <tr>
              <td><span class="sub-badge {statusClass(s.status)}">{s.status}</span></td>
              <td>{s.kind}</td>
              <td>{s.title}</td>
              <td class="sub-mono">
                {#if s.submission_reference}ref: {s.submission_reference}
                {:else if s.package_path}<span title={s.package_path}>{short(s.package_path)}</span>
                {:else}—{/if}
              </td>
              <td class="sub-row-actions">
                {#if s.status === 'blocked' || s.status === 'ready'}
                  <button class="btn btn-xs" disabled={busy} onclick={() => doReevaluate(s)}>Re-check</button>
                {/if}
                {#if s.status === 'ready'}
                  <button class="btn btn-xs btn-primary" disabled={busy} onclick={() => doGenerate(s)}>Generate</button>
                {/if}
                {#if s.status === 'generated'}
                  <input class="sub-ref-input" placeholder="submission ref #" bind:value={refInputs[s.id]} />
                  <button class="btn btn-xs" disabled={busy} onclick={() => doMarkSubmitted(s)}>Mark Submitted</button>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

<style>
  .sub-header { display: flex; justify-content: space-between; align-items: center; }
  .sub-intro { font-size: 0.85rem; color: var(--color-text-secondary, #555); line-height: 1.45; margin: 0.4rem 0 0.75rem; }
  .sub-create { margin-bottom: 1rem; }
  .sub-row { display: flex; gap: 0.75rem; flex-wrap: wrap; align-items: flex-end; margin-bottom: 0.6rem; }
  .sub-row label { display: flex; flex-direction: column; font-size: 0.78rem; font-weight: 600; gap: 0.2rem; }
  .sub-row label input, .sub-row label select { font-weight: 400; padding: 0.35rem; }
  .sub-grow { flex: 1; min-width: 12rem; }
  .sub-grow input { width: 100%; }
  .sub-actions { align-items: center; }
  .sub-check { flex-direction: row !important; align-items: center; gap: 0.35rem !important; font-weight: 400 !important; }
  .sub-readiness { margin-top: 0.5rem; padding: 0.6rem 0.75rem; border-radius: 6px; border: 1px solid var(--color-border, #ddd); }
  .sub-ready-box { background: #ecfdf5; border-color: #a7f3d0; }
  .sub-blocked-box { background: #fef2f2; border-color: #fecaca; }
  .sub-readiness ul { margin: 0.35rem 0 0; padding-left: 1rem; }
  .sub-readiness li { font-size: 0.8rem; margin: 0.15rem 0; }
  .chk-ok { color: #166534; }
  .chk-fail { color: #b91c1c; }
  .chk-detail { color: var(--color-text-secondary, #777); font-weight: 400; }
  .sub-table-wrap { overflow-x: auto; }
  .sub-table { width: 100%; border-collapse: collapse; font-size: 0.82rem; }
  .sub-table th, .sub-table td { text-align: left; padding: 0.4rem 0.5rem; border-bottom: 1px solid var(--color-border, #eee); vertical-align: middle; }
  .sub-row-actions { display: flex; gap: 0.35rem; align-items: center; flex-wrap: wrap; }
  .sub-ref-input { padding: 0.25rem; font-size: 0.75rem; min-width: 9rem; }
  .sub-mono { font-family: var(--font-mono, monospace); font-size: 0.75rem; }
  .sub-badge { padding: 0.1rem 0.45rem; border-radius: 999px; font-size: 0.72rem; font-weight: 600; text-transform: capitalize; }
  .sub-draft { background: #eee; color: #555; }
  .sub-ready { background: #bbf7d0; color: #166534; }
  .sub-blocked { background: #fecaca; color: #991b1b; }
  .sub-generated { background: #bfdbfe; color: #1e40af; }
  .sub-submitted { background: #ddd6fe; color: #5b21b6; }
  .sub-empty { font-size: 0.85rem; color: var(--color-text-secondary, #777); padding: 0.5rem 0; }
</style>
