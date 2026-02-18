<script lang="ts">
  import { untrack } from 'svelte';
  import { getSpecimen, listSubcultures, createSubculture, listMedia, listComplianceRecords } from '../api';
  import { selectedSpecimenId, navigateTo, addNotification } from '../stores/app';

  let specimen = $state<any>(null);
  let subcultures = $state<any[]>([]);
  let mediaBatches = $state<any[]>([]);
  let complianceRecords = $state<any[]>([]);
  let loading = $state(true);
  let showSubcultureForm = $state(false);
  let activeTab = $state<'history' | 'compliance' | 'notes'>('history');

  let subcultureForm = $state({
    date: new Date().toISOString().split('T')[0],
    media_batch_id: '',
    vessel_type: '',
    temperature_c: '',
    ph: '',
    light_cycle: '',
    location_to: '',
    notes: '',
    observations: '',
  });

  const vesselTypes = [
    '250ml glass jar with vented lid', '500ml glass jar with vented lid',
    '100ml Erlenmeyer flask', '250ml Erlenmeyer flask',
    'Magenta GA-7 vessel', 'Petri dish 90mm', 'Petri dish 60mm',
    'Culture tube 25x150mm', 'Culture tube 18x150mm',
    'Baby food jar', 'Tissue culture flask T-25', 'Tissue culture flask T-75',
    'Plantcon vessel', 'PhytatrayII', 'Microbox',
  ];

  $effect(() => {
    if ($selectedSpecimenId) untrack(() => loadAll($selectedSpecimenId));
  });

  async function loadAll(id: string) {
    loading = true;
    try {
      const [s, sc, cr] = await Promise.all([
        getSpecimen(id),
        listSubcultures(id),
        listComplianceRecords(id),
      ]);
      specimen = s;
      subcultures = sc;
      complianceRecords = cr;
      mediaBatches = await listMedia();
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      loading = false;
    }
  }

  async function handleNewSubculture(e: Event) {
    e.preventDefault();
    if (!$selectedSpecimenId) return;
    try {
      await createSubculture({
        specimen_id: $selectedSpecimenId,
        date: subcultureForm.date,
        media_batch_id: subcultureForm.media_batch_id || undefined,
        vessel_type: subcultureForm.vessel_type || undefined,
        temperature_c: subcultureForm.temperature_c ? parseFloat(subcultureForm.temperature_c) : undefined,
        ph: subcultureForm.ph ? parseFloat(subcultureForm.ph) : undefined,
        light_cycle: subcultureForm.light_cycle || undefined,
        location_to: subcultureForm.location_to || undefined,
        notes: subcultureForm.notes || undefined,
        observations: subcultureForm.observations || undefined,
      });
      addNotification('Subculture recorded', 'success');
      showSubcultureForm = false;
      subcultureForm = { date: new Date().toISOString().split('T')[0], media_batch_id: '', vessel_type: '', temperature_c: '', ph: '', light_cycle: '', location_to: '', notes: '', observations: '' };
      loadAll($selectedSpecimenId!);
    } catch (e: any) {
      addNotification(e.message, 'error');
    }
  }
</script>

<div>
  <div class="page-header">
    <div style="display:flex;align-items:center;gap:12px;">
      <button class="btn btn-sm" onclick={() => navigateTo('specimens')}>&larr; Back</button>
      <h1>{specimen?.accession_number || 'Loading...'}</h1>
    </div>
  </div>

  {#if loading}
    <div class="empty-state">Loading specimen...</div>
  {:else if specimen}
    <div class="detail-grid">
      <div class="detail-main">
        <div class="card" style="margin-bottom:16px;">
          <h3 style="margin-bottom:16px;">Specimen Information</h3>
          <div class="info-grid">
            <div class="info-item">
              <span class="info-label">Accession</span>
              <span class="info-value">{specimen.accession_number}</span>
            </div>
            <div class="info-item">
              <span class="info-label">Species</span>
              <span class="info-value">{specimen.species_code} ({specimen.species_name})</span>
            </div>
            <div class="info-item">
              <span class="info-label">Stage</span>
              <span class="info-value"><span class="badge badge-blue">{specimen.stage}</span></span>
            </div>
            <div class="info-item">
              <span class="info-label">Initiation Date</span>
              <span class="info-value">{specimen.initiation_date}</span>
            </div>
            <div class="info-item">
              <span class="info-label">Location</span>
              <span class="info-value">{specimen.location || '—'}</span>
            </div>
            <div class="info-item">
              <span class="info-label">Propagation</span>
              <span class="info-value">{specimen.propagation_method || '—'}</span>
            </div>
            <div class="info-item">
              <span class="info-label">Health</span>
              <span class="info-value">
                {#if specimen.health_status !== null && specimen.health_status !== '' && !isNaN(Number(specimen.health_status))}
                  {@const hv = Math.max(0, Math.min(4, Number(specimen.health_status)))}
                  {@const hlabels = ['Dead','Poor','Fair','Good','Healthy']}
                  {@const hcolors = ['#dc2626','#d97706','#ca8a04','#65a30d','#16a34a']}
                  <span class="health-badge" style="background:{hcolors[hv]}20;color:{hcolors[hv]};border:1px solid {hcolors[hv]}60;">
                    {hv} – {hlabels[hv]}
                  </span>
                {:else}
                  {specimen.health_status || '—'}
                {/if}
              </span>
            </div>
            <div class="info-item">
              <span class="info-label">Passages</span>
              <span class="info-value">{specimen.subculture_count}</span>
            </div>
            <div class="info-item">
              <span class="info-label">Status</span>
              <span class="info-value">
                {#if specimen.quarantine_flag}
                  <span class="badge badge-red">Quarantined</span>
                {:else}
                  <span class="badge badge-green">Active</span>
                {/if}
              </span>
            </div>
            <div class="info-item">
              <span class="info-label">Provenance</span>
              <span class="info-value">{specimen.provenance || '—'}</span>
            </div>
            <div class="info-item">
              <span class="info-label">Source Plant</span>
              <span class="info-value">{specimen.source_plant || '—'}</span>
            </div>
            <div class="info-item">
              <span class="info-label">Permit</span>
              <span class="info-value">{specimen.permit_number || '—'} {specimen.permit_expiry ? `(exp: ${specimen.permit_expiry})` : ''}</span>
            </div>
          </div>
          {#if specimen.notes}
            <div style="margin-top:16px;">
              <span class="info-label">Notes</span>
              <p style="margin-top:4px;font-size:13px;white-space:pre-wrap;">{specimen.notes}</p>
            </div>
          {/if}
          {#if specimen.qr_code_data}
            <div style="margin-top:12px;font-size:11px;color:#6b7280;">QR: {specimen.qr_code_data}</div>
          {/if}
        </div>

        <!-- Tabs -->
        <div class="tabs">
          <button class="tab" class:active={activeTab === 'history'} onclick={() => activeTab = 'history'}>
            Subculture History ({subcultures.length})
          </button>
          <button class="tab" class:active={activeTab === 'compliance'} onclick={() => activeTab = 'compliance'}>
            Compliance ({complianceRecords.length})
          </button>
        </div>

        {#if activeTab === 'history'}
          <div class="card">
            <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:12px;">
              <h3>Subculture History</h3>
              <button class="btn btn-primary btn-sm" onclick={() => showSubcultureForm = !showSubcultureForm}>
                {showSubcultureForm ? 'Cancel' : '+ Record Passage'}
              </button>
            </div>

            {#if showSubcultureForm}
              <form onsubmit={handleNewSubculture} style="border:1px solid #e2e8f0;border-radius:6px;padding:16px;margin-bottom:16px;">
                <div class="form-row">
                  <div class="form-group">
                    <label>Date</label>
                    <input type="date" bind:value={subcultureForm.date} required />
                  </div>
                  <div class="form-group">
                    <label>Media Batch</label>
                    <select bind:value={subcultureForm.media_batch_id}>
                      <option value="">Select media...</option>
                      {#each mediaBatches.slice(0, 10) as mb}
                        <option value={mb.id}>{mb.batch_id} - {mb.name}</option>
                      {/each}
                    </select>
                  </div>
                </div>
                <div class="form-row">
                  <div class="form-group">
                    <label>Vessel Type</label>
                    <select bind:value={subcultureForm.vessel_type}>
                      <option value="">Select vessel...</option>
                      {#each vesselTypes as v}
                        <option value={v}>{v}</option>
                      {/each}
                    </select>
                  </div>
                  <div class="form-group">
                    <label>Transfer To Location</label>
                    <input type="text" bind:value={subcultureForm.location_to} placeholder="New location" />
                  </div>
                </div>
                <div class="env-row">
                  <div class="form-group env-field">
                    <label>Temperature (°C)</label>
                    <input type="number" step="0.1" bind:value={subcultureForm.temperature_c} placeholder="25" />
                  </div>
                  <div class="form-group env-field">
                    <label>pH</label>
                    <input type="number" step="0.01" bind:value={subcultureForm.ph} placeholder="5.7" />
                  </div>
                  <div class="form-group env-field-wide">
                    <label>Light Cycle (hrs on/hrs off)</label>
                    <input type="text" bind:value={subcultureForm.light_cycle} placeholder="16/8" />
                  </div>
                </div>
                <div class="form-group">
                  <label>Observations</label>
                  <textarea bind:value={subcultureForm.observations} rows="2" placeholder="Growth observations..."></textarea>
                </div>
                <div class="form-group">
                  <label>Notes</label>
                  <textarea bind:value={subcultureForm.notes} rows="2" placeholder="Additional notes..."></textarea>
                </div>
                <div style="text-align:right;">
                  <button type="submit" class="btn btn-primary">Record Passage</button>
                </div>
              </form>
            {/if}

            {#if subcultures.length === 0}
              <div class="empty-state">No subcultures recorded yet</div>
            {:else}
              <table>
                <thead>
                  <tr>
                    <th>#</th>
                    <th>Date</th>
                    <th>Media</th>
                    <th>Vessel</th>
                    <th>Temp</th>
                    <th>pH</th>
                    <th>By</th>
                    <th>Notes</th>
                  </tr>
                </thead>
                <tbody>
                  {#each subcultures as sc}
                    <tr>
                      <td><strong>P{sc.passage_number}</strong></td>
                      <td>{sc.date}</td>
                      <td>{sc.media_batch_name || '—'}</td>
                      <td style="max-width:150px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;" title={sc.vessel_type}>{sc.vessel_type || '—'}</td>
                      <td>{sc.temperature_c ? `${sc.temperature_c}°C` : '—'}</td>
                      <td>{sc.ph || '—'}</td>
                      <td>{sc.performer_name || '—'}</td>
                      <td style="max-width:200px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;" title={sc.observations || sc.notes}>{sc.observations || sc.notes || '—'}</td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            {/if}
          </div>
        {:else if activeTab === 'compliance'}
          <div class="card">
            <h3 style="margin-bottom:12px;">Compliance Records</h3>
            {#if complianceRecords.length === 0}
              <div class="empty-state">No compliance records</div>
            {:else}
              <table>
                <thead>
                  <tr>
                    <th>Type</th>
                    <th>Agency</th>
                    <th>Test/Permit</th>
                    <th>Result</th>
                    <th>Status</th>
                    <th>Date</th>
                  </tr>
                </thead>
                <tbody>
                  {#each complianceRecords as cr}
                    <tr>
                      <td>{cr.record_type}</td>
                      <td>{cr.agency || '—'}</td>
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
                      <td>
                        <span class="badge" class:badge-green={cr.status === 'valid'} class:badge-red={cr.status === 'flagged' || cr.status === 'expired'} class:badge-yellow={cr.status === 'pending'}>
                          {cr.status}
                        </span>
                      </td>
                      <td>{cr.test_date || cr.created_at?.split(' ')[0] || '—'}</td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            {/if}
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .detail-grid { display: grid; grid-template-columns: 1fr; gap: 20px; }
  .info-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(220px, 1fr)); gap: 12px; }
  .info-item { display: flex; flex-direction: column; }
  .info-label { font-size: 11px; font-weight: 700; color: #6b7280; }
  .info-value { font-size: 14px; margin-top: 2px; }
  .health-badge { display: inline-block; padding: 2px 10px; border-radius: 10px; font-size: 12px; font-weight: 700; }
  .env-row { display: flex; gap: 12px; flex-wrap: wrap; margin-bottom: 0; }
  .env-field { flex: 0 0 120px; }
  .env-field-wide { flex: 0 0 180px; }

  .tabs {
    display: flex;
    gap: 0;
    border-bottom: 2px solid #e2e8f0;
    margin-bottom: 0;
  }
  :global(.dark) .tabs { border-color: #334155; }
  .tab {
    padding: 10px 20px;
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    margin-bottom: -2px;
    cursor: pointer;
    font-size: 13px;
    font-weight: 600;
    color: #6b7280;
  }
  .tab.active {
    color: #2563eb;
    border-bottom-color: #2563eb;
  }
  .tab:hover { color: #374151; }
</style>
