<script lang="ts">
  import { onMount } from 'svelte';
  import { listComplianceRecords, getComplianceFlags, createComplianceRecord } from '../api';
  import { addNotification } from '../stores/app';
  import { currentUser } from '../stores/auth';

  let records = $state<any[]>([]);
  let flags = $state<any[]>([]);
  let loading = $state(true);
  let showForm = $state(false);
  let activeTab = $state<'flags' | 'records'>('flags');
  let page = $state(1);
  let totalPages = $state(1);
  let total = $state(0);
  let form = $state({
    specimen_id: '', record_type: 'disease_test', agency: '',
    test_type: '', test_method: '', test_date: new Date().toISOString().split('T')[0],
    test_lab: '', test_result: '', permit_number: '', permit_expiry: '', notes: '',
  });

  const recordTypes = ['disease_test', 'permit', 'phytosanitary_cert', 'inspection', 'quarantine', 'movement_permit', 'pest_risk', 'export_cert', 'other'];
  const agencies = ['USDA_APHIS', 'TX_AG', 'FL_FDACS', 'other'];

  onMount(() => { load(); });

  async function load() {
    loading = true;
    try {
      const [r, f] = await Promise.all([listComplianceRecords(undefined, page), getComplianceFlags()]);
      records = r.items;
      total = r.total;
      totalPages = r.total_pages;
      flags = f;
    } catch (e: any) { addNotification(e.message, 'error'); }
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
</script>

<div>
  <div class="page-header">
    <h1>Compliance</h1>
    {#if $currentUser?.role !== 'guest'}
      <button class="btn btn-primary" title={showForm ? 'Cancel and close the form' : 'Open form to add a new compliance record'} onclick={() => showForm = !showForm}>
        {showForm ? 'Cancel' : '+ New Record'}
      </button>
    {/if}
  </div>

  {#if showForm}
    <div class="card" style="margin-bottom:16px;">
      <form onsubmit={handleCreate}>
        <h3 style="margin-bottom:16px;">New Compliance Record</h3>
        <div class="form-row">
          <div class="form-group">
            <label title="The unique identifier of the specimen this record applies to">Specimen ID *</label>
            <input type="text" title="Enter the UUID of the specimen" bind:value={form.specimen_id} required placeholder="Specimen UUID" />
          </div>
          <div class="form-group">
            <label title="Category of compliance record being created">Record Type</label>
            <select title="Select the type of compliance record" bind:value={form.record_type}>
              {#each recordTypes as t}
                <option value={t}>{t.replace(/_/g, ' ')}</option>
              {/each}
            </select>
          </div>
        </div>
        <div class="form-row">
          <div class="form-group">
            <label title="Regulatory agency responsible for this compliance record">Agency</label>
            <select title="Select the governing regulatory agency" bind:value={form.agency}>
              <option value="">Select...</option>
              {#each agencies as a}
                <option value={a}>{a.replace(/_/g, ' ')}</option>
              {/each}
            </select>
          </div>
          <div class="form-group">
            <label title="Name of the disease or diagnostic test performed">Test Type</label>
            <input type="text" title="Enter the test or disease type, e.g. HLB or ELISA" bind:value={form.test_type} placeholder="e.g., HLB, ELISA, PCR" />
          </div>
        </div>
        <div class="form-row">
          <div class="form-group">
            <label title="Laboratory method used to conduct the test">Test Method</label>
            <input type="text" title="Enter the testing method, e.g. PCR or ELISA" bind:value={form.test_method} placeholder="e.g., PCR, ELISA" />
          </div>
          <div class="form-group">
            <label title="Date the test or inspection was conducted">Test Date</label>
            <input type="date" title="Select the date the test was conducted" bind:value={form.test_date} />
          </div>
        </div>
        <div class="form-row">
          <div class="form-group">
            <label title="Name of the laboratory that performed the test">Lab</label>
            <input type="text" title="Enter the name of the testing laboratory" bind:value={form.test_lab} />
          </div>
          <div class="form-group">
            <label title="Outcome of the test or inspection">Result</label>
            <select title="Select the test result" bind:value={form.test_result}>
              <option value="">Pending</option>
              <option value="negative">Negative</option>
              <option value="positive">Positive</option>
              <option value="inconclusive">Inconclusive</option>
            </select>
          </div>
        </div>
        <div class="form-group">
          <label title="Additional notes or observations about this compliance record">Notes</label>
          <textarea title="Enter any additional notes about this record" bind:value={form.notes} rows="2"></textarea>
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

  {#if loading}
    <div class="empty-state">Loading...</div>
  {:else if activeTab === 'flags'}
    {#if flags.length === 0}
      <div class="card empty-state">No compliance flags - all clear</div>
    {:else}
      <div class="card" style="overflow-x:auto;">
        <table>
          <thead>
            <tr>
              <th title="Risk level of the compliance flag">Severity</th>
              <th title="Specimen accession number with the flag">Accession</th>
              <th title="Species code of the flagged specimen">Species</th>
              <th title="Type of compliance issue detected">Flag</th>
              <th title="Details about the compliance issue">Message</th>
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
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  {:else}
    {#if records.length === 0}
      <div class="card empty-state">No compliance records yet</div>
    {:else}
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
    {/if}
  {/if}
</div>

<style>
  .tabs { display: flex; gap: 0; border-bottom: 2px solid #e2e8f0; }
  :global(.dark) .tabs { border-color: #334155; }
  .tab { padding: 10px 20px; background: none; border: none; border-bottom: 2px solid transparent; margin-bottom: -2px; cursor: pointer; font-size: 13px; font-weight: 600; color: #6b7280; }
  .tab.active { color: #2563eb; border-bottom-color: #2563eb; }
  .pagination { display: flex; align-items: center; gap: 8px; padding: 12px 0 4px; justify-content: center; font-size: 13px; }
</style>
