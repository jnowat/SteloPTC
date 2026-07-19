<script lang="ts">
  import { onMount } from 'svelte';
  import { listComplianceRecords, getComplianceFlags, createComplianceRecord, listComplianceRecordTypes, listComplianceAgencies, listComplianceRules, waiveComplianceFlag, listComplianceWaivers, revokeComplianceWaiver } from '../api';
  import { addNotification } from '../stores/app';
  import { currentUser } from '../stores/auth';
  import DataState from './DataState.svelte';
  import ComplianceExportWizard from './ComplianceExportWizard.svelte';
  import SubmissionPipelinePanel from './SubmissionPipelinePanel.svelte';

  let records = $state<any[]>([]);
  let flags = $state<any[]>([]);
  let activeRules = $state<{ flag_type: string; title: string; severity: string; scope: string }[]>([]);
  // WP-77: flag waivers
  let waivers = $state<any[]>([]);
  let showWaivers = $state(false);
  let waiveTarget = $state<any | null>(null);
  let waiveReason = $state('');
  let waiveExpiry = $state('');
  let loading = $state(true);
  let error = $state<string | null>(null);
  let showForm = $state(false);
  let showExportWizard = $state(false);
  let showPipeline = $state(false);
  let activeTab = $state<'flags' | 'records'>('flags');
  let page = $state(1);
  let totalPages = $state(1);
  let total = $state(0);
  let form = $state({
    specimen_id: '', record_type: 'disease_test', agency: '',
    test_type: '', test_method: '', test_date: new Date().toISOString().split('T')[0],
    test_lab: '', test_result: '', permit_number: '', permit_expiry: '', notes: '',
  });

  let recordTypes = $state<any[]>([]);
  let agencies = $state<any[]>([]);

  onMount(() => {
    load();
    listComplianceRecordTypes().then(r => recordTypes = r).catch((e: any) => addNotification(e.message, 'error'));
    listComplianceAgencies().then(a => agencies = a).catch((e: any) => addNotification(e.message, 'error'));
  });

  async function load() {
    loading = true;
    error = null;
    try {
      const [r, f, rules, w] = await Promise.all([listComplianceRecords(undefined, page), getComplianceFlags(), listComplianceRules().catch(() => []), listComplianceWaivers().catch(() => [])]);
      records = r.items;
      total = r.total;
      totalPages = r.total_pages;
      flags = f;
      activeRules = rules;
      waivers = w;
    } catch (e: any) { error = e.message; addNotification(e.message, 'error'); }
    finally { loading = false; }
  }

  async function handleCreate(e: Event) {
    e.preventDefault();
    try {
      await createComplianceRecord({
        specimen_id: form.specimen_id,
        record_type: form.record_type,
        agency: form.agency || undefined,
        test_type: form.test_type || undefined,
        test_method: form.test_method || undefined,
        test_date: form.test_date || undefined,
        test_lab: form.test_lab || undefined,
        test_result: form.test_result || undefined,
        permit_number: form.permit_number || undefined,
        permit_expiry: form.permit_expiry || undefined,
        notes: form.notes || undefined,
      });
      addNotification('Compliance record created', 'success');
      showForm = false;
      load();
    } catch (e: any) { addNotification(e.message, 'error'); }
  }

  function getSeverityClass(s: string) {
    return s === 'critical' ? 'badge-red' : s === 'high' ? 'badge-yellow' : 'badge-blue';
  }

  // WP-77: waive / revoke a compliance flag.
  function openWaive(flag: any) {
    waiveTarget = flag;
    waiveReason = '';
    waiveExpiry = '';
  }

  async function submitWaive(e: Event) {
    e.preventDefault();
    if (!waiveTarget || !waiveReason.trim()) return;
    try {
      await waiveComplianceFlag(waiveTarget.flag_type, waiveTarget.specimen_id, waiveReason.trim(), waiveExpiry || undefined);
      addNotification('Flag waived.', 'success');
      waiveTarget = null;
      await load();
    } catch (e: any) { addNotification(e.message, 'error'); }
  }

  async function revokeWaiver(id: string) {
    try {
      await revokeComplianceWaiver(id);
      addNotification('Waiver revoked — the flag will reappear if still applicable.', 'success');
      await load();
    } catch (e: any) { addNotification(e.message, 'error'); }
  }

  function toggleExportWizard() {
    showExportWizard = !showExportWizard;
  }

  const canExport = $derived($currentUser?.role === 'admin' || $currentUser?.role === 'supervisor');
</script>

<div>
  <div class="page-header">
    <h1>Compliance</h1>
    <div style="display:flex; gap:8px;">
      {#if canExport}
        <button
          class="btn"
          title={showExportWizard ? 'Close the regulatory export wizard' : 'Open the regulatory export wizard to generate FDA, USDA, or CITES compliance bundles'}
          onclick={toggleExportWizard}
        >
          {showExportWizard ? 'Hide Regulatory Export' : 'Regulatory Export ↗'}
        </button>
        <button
          class="btn"
          title={showPipeline ? 'Close the submission pipeline' : 'Monitor compliance state and generate signed regulatory submission packages when ready'}
          onclick={() => showPipeline = !showPipeline}
        >
          {showPipeline ? 'Hide Submission Pipeline' : 'Submission Pipeline ⛭'}
        </button>
      {/if}
      {#if $currentUser?.role !== 'guest'}
        <button class="btn btn-primary" title={showForm ? 'Cancel and close the form' : 'Open form to add a new compliance record'} onclick={() => showForm = !showForm}>
          {showForm ? 'Cancel' : '+ New Record'}
        </button>
      {/if}
    </div>
  </div>

  {#if showExportWizard && canExport}
    <ComplianceExportWizard onclose={() => showExportWizard = false} />
  {/if}

  {#if showPipeline && canExport}
    <SubmissionPipelinePanel onclose={() => showPipeline = false} />
  {/if}

  {#if waiveTarget}
    <div class="card" style="margin-bottom:16px;">
      <form onsubmit={submitWaive}>
        <h3 style="margin-bottom:8px;">Waive compliance flag</h3>
        <p style="font-size:13px;color:#6b7280;margin-bottom:12px;">
          Suppress <strong>{waiveTarget.flag_type.replace(/_/g, ' ')}</strong> for
          <strong>{waiveTarget.accession_number}</strong>. The flag stops appearing until the
          waiver expires or is revoked; the underlying condition is unchanged.
        </p>
        <div class="form-row">
          <div class="form-group" style="flex:2;">
            <label for="waive-reason">Reason *</label>
            <input id="waive-reason" type="text" bind:value={waiveReason} required placeholder="e.g. Permit renewal filed 2026-07-10" />
          </div>
          <div class="form-group">
            <label for="waive-expiry" title="Leave blank for a permanent waiver">Expires (optional)</label>
            <input id="waive-expiry" type="date" bind:value={waiveExpiry} />
          </div>
        </div>
        <div style="display:flex;gap:8px;justify-content:flex-end;">
          <button type="button" class="btn" onclick={() => waiveTarget = null}>Cancel</button>
          <button type="submit" class="btn btn-primary" disabled={!waiveReason.trim()}>Waive Flag</button>
        </div>
      </form>
    </div>
  {/if}

  {#if showForm}
    <div class="card" style="margin-bottom:16px;">
      <form onsubmit={handleCreate}>
        <h3 style="margin-bottom:16px;">New Compliance Record</h3>
        <div class="form-row">
          <div class="form-group">
            <label for="compliance-specimen-id" title="The unique identifier of the specimen this record applies to">Specimen ID *</label>
            <input id="compliance-specimen-id" type="text" title="Enter the UUID of the specimen" bind:value={form.specimen_id} required placeholder="Specimen UUID" />
          </div>
          <div class="form-group">
            <label for="compliance-record-type" title="Category of compliance record being created">Record Type</label>
            <select id="compliance-record-type" title="Select the type of compliance record" bind:value={form.record_type}>
              {#each recordTypes as t}
                <option value={t.code}>{t.label}</option>
              {/each}
            </select>
          </div>
        </div>
        <div class="form-row">
          <div class="form-group">
            <label for="compliance-agency" title="Regulatory agency responsible for this compliance record">Agency</label>
            <select id="compliance-agency" title="Select the governing regulatory agency" bind:value={form.agency}>
              <option value="">Select...</option>
              {#each agencies as a}
                <option value={a.code}>{a.label}</option>
              {/each}
            </select>
          </div>
          <div class="form-group">
            <label for="compliance-test-type" title="Name of the disease or diagnostic test performed">Test Type</label>
            <input id="compliance-test-type" type="text" title="Enter the test or disease type, e.g. HLB or ELISA" bind:value={form.test_type} placeholder="e.g., HLB, ELISA, PCR" />
          </div>
        </div>
        <div class="form-row">
          <div class="form-group">
            <label for="compliance-test-method" title="Laboratory method used to conduct the test">Test Method</label>
            <input id="compliance-test-method" type="text" title="Enter the testing method, e.g. PCR or ELISA" bind:value={form.test_method} placeholder="e.g., PCR, ELISA" />
          </div>
          <div class="form-group">
            <label for="compliance-test-date" title="Date the test or inspection was conducted">Test Date</label>
            <input id="compliance-test-date" type="date" title="Select the date the test was conducted" bind:value={form.test_date} />
          </div>
        </div>
        <div class="form-row">
          <div class="form-group">
            <label for="compliance-test-lab" title="Name of the laboratory that performed the test">Lab</label>
            <input id="compliance-test-lab" type="text" title="Enter the name of the testing laboratory" bind:value={form.test_lab} />
          </div>
          <div class="form-group">
            <label for="compliance-test-result" title="Outcome of the test or inspection">Result</label>
            <select id="compliance-test-result" title="Select the test result" bind:value={form.test_result}>
              <option value="">Pending</option>
              <option value="negative">Negative</option>
              <option value="positive">Positive</option>
              <option value="inconclusive">Inconclusive</option>
            </select>
          </div>
        </div>
        <div class="form-group">
          <label for="compliance-notes" title="Additional notes or observations about this compliance record">Notes</label>
          <textarea id="compliance-notes" title="Enter any additional notes about this record" bind:value={form.notes} rows="2"></textarea>
        </div>
        <div style="text-align:right;">
          <button type="submit" class="btn btn-primary" title="Save this new compliance record">Create Record</button>
        </div>
      </form>
    </div>
  {/if}

  <div class="tabs" style="margin-bottom:16px;">
    <button class="tab" title="View active compliance flags requiring attention" class:active={activeTab === 'flags'} onclick={() => activeTab = 'flags'}>
      Compliance Flags ({flags.length})
    </button>
    <button class="tab" title="View all compliance records in the system" class:active={activeTab === 'records'} onclick={() => activeTab = 'records'}>
      All Records ({records.length})
    </button>
  </div>

  <DataState
    {loading}
    {error}
    rows={5}
    cols={5}
    onretry={load}
  >
    {#if activeTab === 'flags'}
      {#if activeRules.length > 0}
        <div class="active-rules" title="Auto-flag rules currently evaluated for this lab profile (WP-74). Rules for other profiles stay silent.">
          <span class="active-rules-label">Active checks for this profile:</span>
          {#each activeRules as rule}
            <span class="rule-chip {getSeverityClass(rule.severity)}" title="{rule.title} — severity {rule.severity}{rule.scope !== 'all' ? ` · ${rule.scope}` : ''}">{rule.title}</span>
          {/each}
        </div>
      {/if}

      {#if waivers.length > 0}
        <div class="waivers-bar">
          <button class="waivers-toggle" onclick={() => showWaivers = !showWaivers} title="Compliance flags currently suppressed by an active waiver">
            {showWaivers ? '▾' : '▸'} {waivers.length} active waiver{waivers.length !== 1 ? 's' : ''}
          </button>
          {#if showWaivers}
            <div class="card" style="overflow-x:auto;margin-top:8px;">
              <table>
                <thead><tr><th>Specimen</th><th>Flag</th><th>Reason</th><th>Expires</th><th></th></tr></thead>
                <tbody>
                  {#each waivers as w}
                    <tr>
                      <td>{w.specimen_accession ?? w.specimen_id}</td>
                      <td>{w.flag_type.replace(/_/g, ' ')}</td>
                      <td>{w.reason}</td>
                      <td>{w.expires_at ?? 'never'}</td>
                      <td><button class="btn btn-sm" onclick={() => revokeWaiver(w.id)} title="Reinstate this flag">Revoke</button></td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          {/if}
        </div>
      {/if}
      <DataState
        empty={flags.length === 0}
        emptyIcon="✅"
        emptyTitle="No compliance flags"
        emptyMessage="All specimens are clear — no active compliance issues were detected."
      >
        <div class="card" style="overflow-x:auto;">
          <table>
            <thead>
              <tr>
                <th title="Risk level of the compliance flag">Severity</th>
                <th title="Specimen accession number with the flag">Accession</th>
                <th title="Species code of the flagged specimen">Species</th>
                <th title="Type of compliance issue detected">Flag</th>
                <th title="Details about the compliance issue">Message</th>
                <th title="Date of the most recent relevant test, if any">Last Test</th>
                <th title="Waive this flag with a documented reason">Action</th>
              </tr>
            </thead>
            <tbody>
              {#each flags as f}
                <tr>
                  <td><span class="badge {getSeverityClass(f.severity)}" title="Severity level: {f.severity}">{f.severity}</span></td>
                  <td><strong>{f.accession_number}</strong></td>
                  <td>{f.species_code}</td>
                  <td>{f.flag_type.replace(/_/g, ' ')}</td>
                  <td>{f.message}</td>
                  <td>{f.last_test_date ?? '—'}</td>
                  <td><button class="btn btn-sm" onclick={() => openWaive(f)} title="Suppress this flag with a documented reason">Waive</button></td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      </DataState>
    {:else}
      <DataState
        empty={records.length === 0}
        emptyIcon="📄"
        emptyTitle="No compliance records yet"
        emptyMessage="Add compliance records such as disease tests, permits, and inspections."
        emptyActionLabel="+ New Record"
        onemptyaction={() => (showForm = true)}
      >
        <div class="card" style="overflow-x:auto;">
          <table>
            <thead>
              <tr>
                <th title="Specimen accession number or ID">Specimen</th>
                <th title="Category of compliance record">Type</th>
                <th title="Regulatory agency associated with this record">Agency</th>
                <th title="Test type or permit number">Test/Permit</th>
                <th title="Outcome of the test">Result</th>
                <th title="Current validity status of the record">Status</th>
                <th title="Date the test or permit was issued">Date</th>
              </tr>
            </thead>
            <tbody>
              {#each records as cr}
                <tr>
                  <td>{cr.specimen_accession || cr.specimen_id}</td>
                  <td>{cr.record_type.replace(/_/g, ' ')}</td>
                  <td>{cr.agency?.replace(/_/g, ' ') || '—'}</td>
                  <td>{cr.test_type || cr.permit_number || '—'}</td>
                  <td>
                    {#if cr.test_result === 'positive'}
                      <span class="badge badge-red" title="Test returned a positive result">Positive</span>
                    {:else if cr.test_result === 'negative'}
                      <span class="badge badge-green" title="Test returned a negative result">Negative</span>
                    {:else if cr.test_result === 'pending'}
                      <span class="badge badge-yellow" title="Test result is still pending">Pending</span>
                    {:else}
                      {cr.test_result || '—'}
                    {/if}
                  </td>
                  <td><span class="badge" title="Record status: {cr.status}" class:badge-green={cr.status === 'valid'} class:badge-red={cr.status === 'flagged'}>{cr.status}</span></td>
                  <td>{cr.test_date || '—'}</td>
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
      </DataState>
    {/if}
  </DataState>
</div>

<style>
  .tabs { display: flex; gap: 0; border-bottom: 2px solid #e2e8f0; }
  :global(.dark) .tabs { border-color: #334155; }
  .tab { padding: 10px 20px; background: none; border: none; border-bottom: 2px solid transparent; margin-bottom: -2px; cursor: pointer; font-size: 13px; font-weight: 600; color: #6b7280; }
  .tab.active { color: #2563eb; border-bottom-color: #2563eb; }
  .pagination { display: flex; align-items: center; gap: 8px; padding: 12px 0 4px; justify-content: center; font-size: 13px; }
  .active-rules { display: flex; flex-wrap: wrap; align-items: center; gap: 6px; margin: 8px 0 12px; }
  .active-rules-label { font-size: 12px; font-weight: 600; color: #6b7280; margin-right: 2px; }
  :global(.dark) .active-rules-label { color: #94a3b8; }
  .rule-chip { display: inline-block; padding: 2px 9px; border-radius: 999px; font-size: 11px; font-weight: 600; }
  .waivers-bar { margin: 0 0 12px; }
  .waivers-toggle { background: none; border: none; cursor: pointer; font-size: 13px; font-weight: 600; color: #6b7280; padding: 2px 0; }
  :global(.dark) .waivers-toggle { color: #94a3b8; }
</style>
