<script lang="ts">
  import { listComplianceRecords, getComplianceFlags, createComplianceRecord } from '../api';
  import { addNotification } from '../stores/app';
  import { currentUser } from '../stores/auth';

  let records = $state<any[]>([]);
  let flags = $state<any[]>([]);
  let loading = $state(true);
  let showForm = $state(false);
  let activeTab = $state<'flags' | 'records'>('flags');
  let form = $state({
    specimen_id: '', record_type: 'disease_test', agency: '',
    test_type: '', test_method: '', test_date: new Date().toISOString().split('T')[0],
    test_lab: '', test_result: '', permit_number: '', permit_expiry: '', notes: '',
  });

  const recordTypes = ['disease_test', 'permit', 'phytosanitary_cert', 'inspection', 'quarantine', 'movement_permit', 'pest_risk', 'export_cert', 'other'];
  const agencies = ['USDA_APHIS', 'TX_AG', 'FL_FDACS', 'other'];

  $effect(() => { load(); });

  async function load() {
    loading = true;
    try {
      const [r, f] = await Promise.all([listComplianceRecords(), getComplianceFlags()]);
      records = r;
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
      <button class="btn btn-primary" onclick={() => showForm = !showForm}>
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
            <label>Specimen ID *</label>
            <input type="text" bind:value={form.specimen_id} required placeholder="Specimen UUID" />
          </div>
          <div class="form-group">
            <label>Record Type</label>
            <select bind:value={form.record_type}>
              {#each recordTypes as t}
                <option value={t}>{t.replace(/_/g, ' ')}</option>
              {/each}
            </select>
          </div>
        </div>
        <div class="form-row">
          <div class="form-group">
            <label>Agency</label>
            <select bind:value={form.agency}>
              <option value="">Select...</option>
              {#each agencies as a}
                <option value={a}>{a.replace(/_/g, ' ')}</option>
              {/each}
            </select>
          </div>
          <div class="form-group">
            <label>Test Type</label>
            <input type="text" bind:value={form.test_type} placeholder="e.g., HLB, ELISA, PCR" />
          </div>
        </div>
        <div class="form-row">
          <div class="form-group">
            <label>Test Method</label>
            <input type="text" bind:value={form.test_method} placeholder="e.g., PCR, ELISA" />
          </div>
          <div class="form-group">
            <label>Test Date</label>
            <input type="date" bind:value={form.test_date} />
          </div>
        </div>
        <div class="form-row">
          <div class="form-group">
            <label>Lab</label>
            <input type="text" bind:value={form.test_lab} />
          </div>
          <div class="form-group">
            <label>Result</label>
            <select bind:value={form.test_result}>
              <option value="">Pending</option>
              <option value="negative">Negative</option>
              <option value="positive">Positive</option>
              <option value="inconclusive">Inconclusive</option>
            </select>
          </div>
        </div>
        <div class="form-group">
          <label>Notes</label>
          <textarea bind:value={form.notes} rows="2"></textarea>
        </div>
        <div style="text-align:right;">
          <button type="submit" class="btn btn-primary">Create Record</button>
        </div>
      </form>
    </div>
  {/if}

  <div class="tabs" style="margin-bottom:16px;">
    <button class="tab" class:active={activeTab === 'flags'} onclick={() => activeTab = 'flags'}>
      Compliance Flags ({flags.length})
    </button>
    <button class="tab" class:active={activeTab === 'records'} onclick={() => activeTab = 'records'}>
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
              <th>Severity</th>
              <th>Accession</th>
              <th>Species</th>
              <th>Flag</th>
              <th>Message</th>
            </tr>
          </thead>
          <tbody>
            {#each flags as f}
              <tr>
                <td><span class="badge {getSeverityClass(f.severity)}">{f.severity}</span></td>
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
              <th>Specimen</th>
              <th>Type</th>
              <th>Agency</th>
              <th>Test/Permit</th>
              <th>Result</th>
              <th>Status</th>
              <th>Date</th>
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
                    <span class="badge badge-red">Positive</span>
                  {:else if cr.test_result === 'negative'}
                    <span class="badge badge-green">Negative</span>
                  {:else if cr.test_result === 'pending'}
                    <span class="badge badge-yellow">Pending</span>
                  {:else}
                    {cr.test_result || '—'}
                  {/if}
                </td>
                <td><span class="badge" class:badge-green={cr.status === 'valid'} class:badge-red={cr.status === 'flagged'}>{cr.status}</span></td>
                <td>{cr.test_date || '—'}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  {/if}
</div>

<style>
  .tabs { display: flex; gap: 0; border-bottom: 2px solid #e2e8f0; }
  :global(.dark) .tabs { border-color: #334155; }
  .tab { padding: 10px 20px; background: none; border: none; border-bottom: 2px solid transparent; margin-bottom: -2px; cursor: pointer; font-size: 13px; font-weight: 600; color: #6b7280; }
  .tab.active { color: #2563eb; border-bottom-color: #2563eb; }
</style>
