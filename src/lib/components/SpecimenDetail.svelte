<script lang="ts">
  import { untrack } from 'svelte';
  import { getSpecimen, listSubcultures, createSubculture, updateSubculture, createSpecimen, listMedia, listComplianceRecords, listSpecimens } from '../api';
  import { selectedSpecimenId, navigateTo, addNotification, devMode } from '../stores/app';
  import QrModal from './QrModal.svelte';
  import QrScanner from './QrScanner.svelte';

  let specimen = $state<any>(null);
  let showQrModal = $state(false);
  let showQrScanner = $state(false);
  let subcultures = $state<any[]>([]);
  let mediaBatches = $state<any[]>([]);
  let complianceRecords = $state<any[]>([]);
  let parentSpecimen = $state<any>(null);
  let childSpecimens = $state<any[]>([]);
  let loading = $state(true);
  let showPassageForm = $state(false);
  let expandedPassages = $state(new Set<string>());
  let activeTab = $state<'history' | 'compliance'>('history');
  let isSplitting = $state(false);
  let splitCount = $state(2);
  let submitting = $state(false);

  // Dev mode: inline edit state
  let editingPassageId = $state<string | null>(null);
  let passageEditForm = $state({ notes: '', observations: '', vessel_type: '', location_to: '' });

  // Location dropdowns for transfer destination
  let locToRoom = $state(localStorage.getItem('sc_lastRoom') || '');
  let locToRack = $state(localStorage.getItem('sc_lastRack') || '');
  let locToShelf = $state(localStorage.getItem('sc_lastShelf') || '');
  let locToTray = $state(localStorage.getItem('sc_lastTray') || '');

  const rooms = ['Room 1', 'Room 2', 'Room 3', 'Room 4', 'Room 5'];
  const racks = ['Rack A', 'Rack B', 'Rack C', 'Rack D'];
  const shelves = ['Shelf 1', 'Shelf 2', 'Shelf 3', 'Shelf 4', 'Shelf 5'];
  const trays = ['Tray A', 'Tray B', 'Tray C', 'Tray D', 'Tray E', 'Tray F'];

  function composeLocationTo() {
    return [locToRoom, locToRack, locToShelf, locToTray].filter(Boolean).join(' / ') || '';
  }

  let subcultureForm = $state({
    date: new Date().toISOString().split('T')[0],
    media_batch_id: '',
    vessel_type: '',
    temperature_c: '',
    ph: '',
    light_cycle: '',
    notes: '',
    observations: '',
    health_status: '',
    health_unknown: false,
    employee_id: '',
  });

  // Media date warning: show if selected media batch was prepared after the passage date
  let mediaDateWarning = $state(false);

  $effect(() => {
    const batchId = subcultureForm.media_batch_id;
    const passageDate = subcultureForm.date;
    if (batchId && passageDate) {
      const batch = mediaBatches.find((mb: any) => mb.id === batchId);
      if (batch && batch.preparation_date && passageDate) {
        mediaDateWarning = batch.preparation_date > passageDate;
      } else {
        mediaDateWarning = false;
      }
    } else {
      mediaDateWarning = false;
    }
  });

  // Health slider value for the passage form (0–4)
  let passageHealthValue = $state(4);
  const healthLabels = ['Dead', 'Poor', 'Fair', 'Good', 'Healthy'];
  const healthColors = ['#dc2626', '#d97706', '#ca8a04', '#65a30d', '#16a34a'];

  function effectivePassageHealth(): string {
    return subcultureForm.health_unknown ? '-1' : String(passageHealthValue);
  }

  const vesselTypes = [
    '250ml glass jar with vented lid', '500ml glass jar with vented lid',
    '100ml Erlenmeyer flask', '250ml Erlenmeyer flask',
    'Magenta GA-7 vessel', 'Petri dish 90mm', 'Petri dish 60mm',
    'Culture tube 25x150mm', 'Culture tube 18x150mm',
    'Baby food jar', 'Tissue culture flask T-25', 'Tissue culture flask T-75',
    'Plantcon vessel', 'PhytatrayII', 'Microbox',
  ];

  const hlabels = ['Dead', 'Poor', 'Fair', 'Good', 'Healthy'];
  const hcolors = ['#dc2626', '#d97706', '#ca8a04', '#65a30d', '#16a34a'];
  const dotColors = ['#2563eb', '#059669', '#7c3aed', '#0891b2', '#d97706', '#db2777'];

  function healthInfo(val: any) {
    if (val === null || val === '' || isNaN(Number(val))) return null;
    const n = Math.round(Number(val));
    if (n === -1) return { label: '? – Unknown / Awaiting', color: '#7c3aed' };
    const i = Math.max(0, Math.min(4, n));
    return { label: `${i} – ${hlabels[i]}`, color: hcolors[i] };
  }

  function stageLabel(stage: string) {
    return stage?.replace(/_/g, ' ').replace(/\b\w/g, c => c.toUpperCase()) || stage || '—';
  }

  function dotColor(passageNumber: number) {
    return dotColors[(passageNumber - 1) % dotColors.length];
  }

  $effect(() => {
    if ($selectedSpecimenId) untrack(() => loadAll($selectedSpecimenId));
  });

  async function loadAll(id: string) {
    loading = true;
    try {
      const [s, sc, cr, mb] = await Promise.all([
        getSpecimen(id),
        listSubcultures(id),
        listComplianceRecords(id),
        listMedia(),
      ]);
      specimen = s;
      subcultures = [...sc].reverse(); // newest first
      complianceRecords = cr;
      mediaBatches = mb;

      // Lineage: fetch parent if present
      if (s.parent_specimen_id) {
        parentSpecimen = await getSpecimen(s.parent_specimen_id).catch(() => null);
      } else {
        parentSpecimen = null;
      }

      // Lineage: find children via full list
      const all = await listSpecimens(1, 500);
      childSpecimens = (all.items || []).filter((sp: any) => sp.parent_specimen_id === id);
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      loading = false;
    }
  }

  async function handlePassage(e: Event) {
    e.preventDefault();
    if (!$selectedSpecimenId || !specimen) return;
    submitting = true;
    const locationTo = composeLocationTo();
    const splitNote = isSplitting ? `Split into ${splitCount} container${splitCount > 1 ? 's' : ''}.` : '';
    const combinedNotes = [splitNote, subcultureForm.notes].filter(Boolean).join(' ');
    try {
      await createSubculture({
        specimen_id: $selectedSpecimenId,
        date: subcultureForm.date,
        media_batch_id: subcultureForm.media_batch_id || undefined,
        vessel_type: subcultureForm.vessel_type || undefined,
        temperature_c: subcultureForm.temperature_c ? parseFloat(subcultureForm.temperature_c) : undefined,
        ph: subcultureForm.ph ? parseFloat(subcultureForm.ph) : undefined,
        light_cycle: subcultureForm.light_cycle || undefined,
        location_from: specimen.location || undefined,
        location_to: locationTo || undefined,
        notes: combinedNotes || undefined,
        observations: subcultureForm.observations || undefined,
        health_status: effectivePassageHealth() !== '' ? effectivePassageHealth() : undefined,
        employee_id: subcultureForm.employee_id || undefined,
      });

      if (isSplitting && splitCount > 1) {
        const childPromises = Array.from({ length: splitCount }, (_, i) =>
          createSpecimen({
            species_id: specimen.species_id,
            stage: specimen.stage,
            health_status: specimen.health_status,
            location: locationTo || undefined,
            propagation_method: specimen.propagation_method || undefined,
            initiation_date: subcultureForm.date,
            parent_specimen_id: $selectedSpecimenId,
            notes: `Split from ${specimen.accession_number} on ${subcultureForm.date}. Container ${i + 1} of ${splitCount}.`,
            provenance: specimen.provenance || undefined,
            source_plant: specimen.source_plant || undefined,
          })
        );
        await Promise.all(childPromises);
        addNotification(`Passage recorded. ${splitCount} new specimens created.`, 'success');
      } else {
        addNotification('Passage recorded.', 'success');
      }

      // Persist location prefs
      localStorage.setItem('sc_lastRoom', locToRoom);
      localStorage.setItem('sc_lastRack', locToRack);
      localStorage.setItem('sc_lastShelf', locToShelf);
      localStorage.setItem('sc_lastTray', locToTray);

      showPassageForm = false;
      isSplitting = false;
      splitCount = 2;
      passageHealthValue = 4;
      subcultureForm = {
        date: new Date().toISOString().split('T')[0],
        media_batch_id: '', vessel_type: '', temperature_c: '',
        ph: '', light_cycle: '', notes: '', observations: '',
        health_status: '', health_unknown: false, employee_id: '',
      };
      loadAll($selectedSpecimenId!);
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      submitting = false;
    }
  }

  function togglePassage(id: string) {
    if (expandedPassages.has(id)) {
      expandedPassages = new Set([...expandedPassages].filter(x => x !== id));
    } else {
      expandedPassages = new Set([...expandedPassages, id]);
    }
  }

  function startEditPassage(sc: any) {
    editingPassageId = sc.id;
    passageEditForm = {
      notes: sc.notes || '',
      observations: sc.observations || '',
      vessel_type: sc.vessel_type || '',
      location_to: sc.location_to || '',
    };
  }

  function cancelEditPassage() {
    editingPassageId = null;
    passageEditForm = { notes: '', observations: '', vessel_type: '', location_to: '' };
  }

  async function handleEditPassage(e: Event, scId: string) {
    e.preventDefault();
    try {
      await updateSubculture({
        id: scId,
        notes: passageEditForm.notes || undefined,
        observations: passageEditForm.observations || undefined,
        vessel_type: passageEditForm.vessel_type || undefined,
        location_to: passageEditForm.location_to || undefined,
      });
      addNotification('Passage updated.', 'success');
      editingPassageId = null;
      passageEditForm = { notes: '', observations: '', vessel_type: '', location_to: '' };
      loadAll($selectedSpecimenId!);
    } catch (e: any) {
      addNotification(e.message, 'error');
    }
  }

  function navigateToSpecimen(id: string) {
    selectedSpecimenId.set(id);
  }
</script>

<div class="specimen-detail">
  <div class="page-header">
    <div style="display:flex;align-items:center;gap:12px;flex-wrap:wrap;">
      <button class="btn btn-sm" onclick={() => navigateTo('specimens')}>&larr; Back</button>
      <div>
        <h1 style="margin-bottom:3px;">{specimen?.accession_number || 'Loading...'}</h1>
        {#if specimen}
          <span style="font-size:13px;color:#6b7280;">{specimen.species_code} — {specimen.species_name}</span>
        {/if}
      </div>
      {#if specimen}
        {#if specimen.health_status !== null && specimen.health_status !== '' && !isNaN(Number(specimen.health_status))}
          {@const hb = healthInfo(specimen.health_status)}
          {#if hb}
            <span class="health-badge" style="background:{hb.color}20;color:{hb.color};border:1px solid {hb.color}60;">{hb.label}</span>
          {/if}
        {/if}
        {#if specimen.quarantine_flag}
          <span class="badge badge-red">Quarantined</span>
        {:else}
          <span class="badge badge-green">Active</span>
        {/if}
      {/if}
    </div>
    {#if specimen}
      <div style="display:flex;gap:8px;flex-wrap:wrap;">
        <button class="btn btn-qr-detail" onclick={() => (showQrScanner = true)}>
          &#128247; Scan QR
        </button>
        <button class="btn btn-qr-detail btn-qr-generate" onclick={() => (showQrModal = true)}>
          &#9641; Generate QR
        </button>
      </div>
    {/if}
  </div>

  {#if loading}
    <div class="empty-state">Loading specimen…</div>
  {:else if specimen}

    <!-- ── Lineage Banner ── -->
    {#if parentSpecimen || childSpecimens.length > 0}
      <div class="lineage-banner">
        {#if parentSpecimen}
          <div class="lineage-row">
            <span class="lineage-icon">↑</span>
            <span class="lineage-label">Split from</span>
            <button class="lineage-chip parent-chip" onclick={() => navigateToSpecimen(parentSpecimen.id)}>
              {parentSpecimen.accession_number}
              <span class="lineage-chip-sub">{parentSpecimen.species_code}</span>
            </button>
          </div>
        {/if}
        {#if childSpecimens.length > 0}
          <div class="lineage-row">
            <span class="lineage-icon">↓</span>
            <span class="lineage-label">Split into {childSpecimens.length} container{childSpecimens.length > 1 ? 's' : ''}</span>
            <div class="lineage-children">
              {#each childSpecimens as child}
                <button class="lineage-chip child-chip" onclick={() => navigateToSpecimen(child.id)}>
                  {child.accession_number}
                </button>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    {/if}

    <!-- ── Specimen Info Card ── -->
    <div class="card info-card">
      <h3 style="margin-bottom:14px;font-size:15px;">Specimen Information</h3>
      <div class="info-grid">
        <div class="info-item">
          <span class="info-label">Accession</span>
          <span class="info-value mono">{specimen.accession_number}</span>
        </div>
        <div class="info-item">
          <span class="info-label">Stage</span>
          <span class="info-value"><span class="badge badge-blue">{stageLabel(specimen.stage)}</span></span>
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
          <span class="info-label">Passages</span>
          <span class="info-value">{specimen.subculture_count}</span>
        </div>
        <div class="info-item">
          <span class="info-label">Provenance</span>
          <span class="info-value">{specimen.provenance || '—'}</span>
        </div>
        <div class="info-item">
          <span class="info-label">Source Plant</span>
          <span class="info-value">{specimen.source_plant || '—'}</span>
        </div>
        {#if specimen.permit_number}
          <div class="info-item">
            <span class="info-label">Permit</span>
            <span class="info-value">{specimen.permit_number}{specimen.permit_expiry ? ` (exp: ${specimen.permit_expiry})` : ''}</span>
          </div>
        {/if}
      </div>
      {#if specimen.notes}
        <div style="margin-top:14px;padding-top:12px;border-top:1px solid #e2e8f0;">
          <span class="info-label">Notes</span>
          <p style="margin-top:4px;font-size:13px;white-space:pre-wrap;color:#374151;">{specimen.notes}</p>
        </div>
      {/if}
    </div>

    <!-- ── Tabs ── -->
    <div class="tabs">
      <button class="tab" class:active={activeTab === 'history'} onclick={() => activeTab = 'history'}>
        Passage Timeline {#if subcultures.length > 0}<span class="tab-count">{subcultures.length}</span>{/if}
      </button>
      <button class="tab" class:active={activeTab === 'compliance'} onclick={() => activeTab = 'compliance'}>
        Compliance {#if complianceRecords.length > 0}<span class="tab-count">{complianceRecords.length}</span>{/if}
      </button>
    </div>

    <!-- ── History / Timeline Tab ── -->
    {#if activeTab === 'history'}
      <div class="card" style="margin-top:0;border-top-left-radius:0;border-top-right-radius:0;">

        <!-- Record Passage header -->
        <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:{showPassageForm ? 16 : 0}px;">
          <h3 style="font-size:15px;">Passage History</h3>
          <button class="btn btn-primary btn-sm" onclick={() => showPassageForm = !showPassageForm}>
            {showPassageForm ? '✕ Cancel' : '+ Record Passage'}
          </button>
        </div>

        <!-- ── Record Passage Form ── -->
        {#if showPassageForm}
          <form onsubmit={handlePassage} class="passage-form">

            <!-- Date + Media -->
            <div class="form-row">
              <div class="form-group">
                <label>Date</label>
                <input type="date" bind:value={subcultureForm.date} required />
              </div>
              <div class="form-group" style="flex:2;">
                <label>Media Batch</label>
                <select bind:value={subcultureForm.media_batch_id}>
                  <option value="">No media / not recorded</option>
                  {#each mediaBatches.slice(0, 20) as mb}
                    <option value={mb.id}>{mb.batch_id} — {mb.name}</option>
                  {/each}
                </select>
                {#if mediaDateWarning}
                  <div style="color:#dc2626;font-size:12px;margin-top:4px;">
                    ⚠ Warning: this media batch was prepared AFTER the passage date — please verify.
                  </div>
                {/if}
              </div>
            </div>

            <!-- Vessel + Env -->
            <div class="form-row">
              <div class="form-group" style="flex:2;">
                <label>Vessel Type</label>
                <select bind:value={subcultureForm.vessel_type}>
                  <option value="">Select vessel…</option>
                  {#each vesselTypes as v}
                    <option value={v}>{v}</option>
                  {/each}
                </select>
              </div>
              <div class="form-group env-field">
                <label>Temp (°C)</label>
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

            <!-- Transfer To Location -->
            <div class="section-header">Transfer To Location</div>
            <div class="form-row">
              <div class="form-group">
                <label>Room</label>
                <select bind:value={locToRoom}>
                  <option value="">—</option>
                  {#each rooms as r}<option value={r}>{r}</option>{/each}
                </select>
              </div>
              <div class="form-group">
                <label>Rack</label>
                <select bind:value={locToRack}>
                  <option value="">—</option>
                  {#each racks as r}<option value={r}>{r}</option>{/each}
                </select>
              </div>
              <div class="form-group">
                <label>Shelf</label>
                <select bind:value={locToShelf}>
                  <option value="">—</option>
                  {#each shelves as s}<option value={s}>{s}</option>{/each}
                </select>
              </div>
              <div class="form-group">
                <label>Tray</label>
                <select bind:value={locToTray}>
                  <option value="">—</option>
                  {#each trays as t}<option value={t}>{t}</option>{/each}
                </select>
              </div>
            </div>

            <!-- Health Status -->
            <div class="form-group">
              <label>Health Status</label>
              <div class="health-slider-wrap">
                <label class="unknown-toggle">
                  <input type="checkbox" bind:checked={subcultureForm.health_unknown} style="width:auto;" />
                  Unknown / Awaiting Assessment
                </label>
                {#if subcultureForm.health_unknown}
                  <div class="health-display" style="color:#7c3aed;">? – Unknown / Awaiting Assessment</div>
                {:else}
                  <input
                    type="range"
                    min="0"
                    max="4"
                    step="1"
                    bind:value={passageHealthValue}
                    class="health-slider"
                    style="--track-color: {healthColors[passageHealthValue]};"
                  />
                  <div class="health-ticks">
                    {#each healthLabels as lbl, i}
                      <span class="health-tick" class:active={passageHealthValue === i} style={passageHealthValue === i ? `color:${healthColors[i]};` : ''}>
                        {i} {lbl}
                      </span>
                    {/each}
                  </div>
                  <div class="health-display" style="color:{healthColors[passageHealthValue]};">
                    {passageHealthValue} – {healthLabels[passageHealthValue]}
                  </div>
                {/if}
              </div>
            </div>

            <!-- Employee ID -->
            <div class="form-group">
              <label>Employee ID / Badge #</label>
              <input type="text" bind:value={subcultureForm.employee_id} placeholder="e.g., EMP-042" />
            </div>

            <!-- Observations + Notes -->
            <div class="form-row">
              <div class="form-group" style="flex:1;">
                <label>Observations</label>
                <textarea bind:value={subcultureForm.observations} rows="2" placeholder="Growth observations, morphology…"></textarea>
              </div>
              <div class="form-group" style="flex:1;">
                <label>Notes</label>
                <textarea bind:value={subcultureForm.notes} rows="2" placeholder="Protocol notes, reagent lots…"></textarea>
              </div>
            </div>

            <!-- Split Culture Toggle -->
            <div class="split-toggle-row">
              <label class="split-toggle-label">
                <input type="checkbox" bind:checked={isSplitting} style="margin-right:6px;" />
                Split culture into multiple containers
              </label>
              {#if isSplitting}
                <div class="split-count-row">
                  <span class="split-desc">Number of new specimens to create:</span>
                  <input type="number" min="2" max="100" bind:value={splitCount} class="split-count-input" />
                  <span class="split-hint">Each will be linked to this specimen as parent.</span>
                </div>
                <div class="split-preview">
                  <span class="split-preview-parent">{specimen.accession_number}</span>
                  <span class="split-arrow">→</span>
                  <div class="split-preview-children">
                    {#each Array.from({length: Math.min(splitCount, 5)}) as _, i}
                      <span class="split-preview-child">Child {i + 1}</span>
                    {/each}
                    {#if splitCount > 5}
                      <span class="split-preview-child muted">+{splitCount - 5} more</span>
                    {/if}
                  </div>
                </div>
              {/if}
            </div>

            <div style="text-align:right;margin-top:12px;">
              <button type="submit" class="btn btn-primary" disabled={submitting}>
                {submitting ? 'Recording…' : isSplitting ? `Record + Create ${splitCount} Splits` : 'Record Passage'}
              </button>
            </div>
          </form>
        {/if}

        <!-- ── Timeline ── -->
        {#if subcultures.length === 0}
          <div class="empty-state" style="padding:40px 0;">
            No passages recorded yet.<br/>
            <span style="font-size:12px;color:#9ca3af;">Use "Record Passage" above to log the first subculture event.</span>
          </div>
        {:else}
          <div class="timeline" class:with-form={showPassageForm}>
            {#each subcultures as sc, i}
              {@const color = dotColor(sc.passage_number)}
              {@const isExpanded = expandedPassages.has(sc.id)}
              <div class="timeline-item">
                <!-- Left: connector -->
                <div class="timeline-left">
                  <div class="tl-dot" style="background:{color};box-shadow:0 0 0 3px {color}30;"></div>
                  {#if i < subcultures.length - 1}
                    <div class="tl-line"></div>
                  {/if}
                </div>

                <!-- Right: card -->
                <div class="tl-card" class:expanded={isExpanded}>
                  <button class="tl-card-header" onclick={() => togglePassage(sc.id)}>
                    <div class="tl-card-left">
                      <span class="tl-passage-num" style="color:{color};">P{sc.passage_number}</span>
                      <div class="tl-card-summary">
                        <span class="tl-date">{sc.date}</span>
                        {#if sc.media_batch_name}
                          <span class="tl-pill media-pill">{sc.media_batch_name}</span>
                        {/if}
                        {#if sc.vessel_type}
                          <span class="tl-pill vessel-pill">{sc.vessel_type}</span>
                        {/if}
                        {#if sc.location_to}
                          <span class="tl-pill loc-pill">→ {sc.location_to}</span>
                        {/if}
                      </div>
                    </div>
                    <div style="display:flex;align-items:center;gap:8px;">
                      {#if $devMode && isExpanded}
                        <button
                          type="button"
                          class="btn btn-sm"
                          style="background:#dc2626; color:white;"
                          onclick={(e) => { e.stopPropagation(); if (editingPassageId === sc.id) { cancelEditPassage(); } else { startEditPassage(sc); } }}
                        >
                          {editingPassageId === sc.id ? 'Cancel Edit' : 'Edit'}
                        </button>
                      {/if}
                      <span class="tl-chevron">{isExpanded ? '▴' : '▾'}</span>
                    </div>
                  </button>

                  {#if isExpanded}
                    <div class="tl-card-body">
                      {#if $devMode && editingPassageId === sc.id}
                        <!-- Inline edit form -->
                        <form onsubmit={(e) => handleEditPassage(e, sc.id)} style="margin-top:12px;display:flex;flex-direction:column;gap:10px;">
                          <div class="form-row">
                            <div class="form-group" style="flex:2;">
                              <label>Vessel Type</label>
                              <select bind:value={passageEditForm.vessel_type}>
                                <option value="">Select vessel…</option>
                                {#each vesselTypes as v}
                                  <option value={v}>{v}</option>
                                {/each}
                              </select>
                            </div>
                            <div class="form-group" style="flex:2;">
                              <label>Location To</label>
                              <input type="text" bind:value={passageEditForm.location_to} placeholder="e.g., Room 1 / Rack A / Shelf 2" />
                            </div>
                          </div>
                          <div class="form-row">
                            <div class="form-group" style="flex:1;">
                              <label>Observations</label>
                              <textarea bind:value={passageEditForm.observations} rows="2" placeholder="Growth observations, morphology…"></textarea>
                            </div>
                            <div class="form-group" style="flex:1;">
                              <label>Notes</label>
                              <textarea bind:value={passageEditForm.notes} rows="2" placeholder="Protocol notes, reagent lots…"></textarea>
                            </div>
                          </div>
                          <div style="text-align:right;">
                            <button type="button" class="btn btn-sm" onclick={cancelEditPassage} style="margin-right:6px;">Cancel</button>
                            <button type="submit" class="btn btn-primary btn-sm">Save Changes</button>
                          </div>
                        </form>
                      {:else}
                        <div class="tl-detail-grid">
                          {#if sc.media_batch_name}
                            <div class="tl-detail-item">
                              <span class="tl-detail-label">Media Batch</span>
                              <span class="tl-detail-value">{sc.media_batch_name}</span>
                            </div>
                          {/if}
                          {#if sc.vessel_type}
                            <div class="tl-detail-item span2">
                              <span class="tl-detail-label">Vessel</span>
                              <span class="tl-detail-value">{sc.vessel_type}</span>
                            </div>
                          {/if}
                          {#if sc.temperature_c}
                            <div class="tl-detail-item">
                              <span class="tl-detail-label">Temperature</span>
                              <span class="tl-detail-value">{sc.temperature_c} °C</span>
                            </div>
                          {/if}
                          {#if sc.ph}
                            <div class="tl-detail-item">
                              <span class="tl-detail-label">pH</span>
                              <span class="tl-detail-value">{sc.ph}</span>
                            </div>
                          {/if}
                          {#if sc.light_cycle}
                            <div class="tl-detail-item">
                              <span class="tl-detail-label">Light Cycle</span>
                              <span class="tl-detail-value">{sc.light_cycle} hrs on/off</span>
                            </div>
                          {/if}
                          {#if sc.location_from}
                            <div class="tl-detail-item">
                              <span class="tl-detail-label">From Location</span>
                              <span class="tl-detail-value">{sc.location_from}</span>
                            </div>
                          {/if}
                          {#if sc.location_to}
                            <div class="tl-detail-item">
                              <span class="tl-detail-label">To Location</span>
                              <span class="tl-detail-value">{sc.location_to}</span>
                            </div>
                          {/if}
                          {#if sc.performer_name}
                            <div class="tl-detail-item">
                              <span class="tl-detail-label">Performed By</span>
                              <span class="tl-detail-value">{sc.performer_name}</span>
                            </div>
                          {/if}
                          {#if sc.employee_id}
                            <div class="tl-detail-item">
                              <span class="tl-detail-label">Employee ID</span>
                              <span class="tl-detail-value">{sc.employee_id}</span>
                            </div>
                          {/if}
                          {#if sc.health_status !== null && sc.health_status !== '' && !isNaN(Number(sc.health_status))}
                            {@const hb = healthInfo(sc.health_status)}
                            {#if hb}
                              <div class="tl-detail-item">
                                <span class="tl-detail-label">Health</span>
                                <span class="tl-detail-value">
                                  <span style="display:inline-block;padding:2px 8px;border-radius:10px;font-size:12px;font-weight:700;background:{hb.color}20;color:{hb.color};border:1px solid {hb.color}60;">
                                    {hb.label}
                                  </span>
                                </span>
                              </div>
                            {/if}
                          {/if}
                        </div>
                        {#if sc.observations}
                          <div class="tl-detail-text">
                            <span class="tl-detail-label">Observations</span>
                            <p class="tl-detail-p">{sc.observations}</p>
                          </div>
                        {/if}
                        {#if sc.notes}
                          <div class="tl-detail-text">
                            <span class="tl-detail-label">Notes</span>
                            <p class="tl-detail-p">{sc.notes}</p>
                          </div>
                        {/if}
                      {/if}
                    </div>
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>

    <!-- ── Compliance Tab ── -->
    {:else if activeTab === 'compliance'}
      <div class="card" style="margin-top:0;border-top-left-radius:0;border-top-right-radius:0;">
        <h3 style="margin-bottom:12px;font-size:15px;">Compliance Records</h3>
        {#if complianceRecords.length === 0}
          <div class="empty-state">No compliance records</div>
        {:else}
          <table>
            <thead>
              <tr>
                <th>Type</th>
                <th>Agency</th>
                <th>Test / Permit</th>
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
                    <span class="badge"
                      class:badge-green={cr.status === 'valid'}
                      class:badge-red={cr.status === 'flagged' || cr.status === 'expired'}
                      class:badge-yellow={cr.status === 'pending'}>
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

  {/if}
</div>

<!-- QR Code Modal -->
{#if showQrModal && specimen}
  <QrModal specimen={specimen} onclose={() => (showQrModal = false)} />
{/if}

<!-- QR Scanner Modal -->
{#if showQrScanner}
  <QrScanner onclose={() => (showQrScanner = false)} />
{/if}

<style>
  .specimen-detail { max-width: 900px; }

  /* QR buttons in header */
  .btn-qr-detail {
    background: #f0fdf4;
    color: #15803d;
    border-color: #86efac;
    font-size: 13px;
    min-height: 36px;
  }
  .btn-qr-detail:hover { background: #dcfce7; }
  :global(.dark) .btn-qr-detail { background: rgba(34,197,94,0.1); color: #4ade80; border-color: #166534; }
  .btn-qr-generate {
    background: #eff6ff;
    color: #1d4ed8;
    border-color: #93c5fd;
  }
  .btn-qr-generate:hover { background: #dbeafe; }
  :global(.dark) .btn-qr-generate { background: rgba(37,99,235,0.1); color: #60a5fa; border-color: #1e40af; }

  @media (max-width: 768px) {
    .btn-qr-detail { min-height: 44px; font-size: 14px; }
  }

  /* ── Info Card ── */
  .info-card { margin-bottom: 0; border-bottom-left-radius: 0; border-bottom-right-radius: 0; border-bottom: none; }
  .info-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 12px; }
  .info-item { display: flex; flex-direction: column; }
  .info-label { font-size: 11px; font-weight: 700; color: #6b7280; text-transform: uppercase; letter-spacing: 0.5px; }
  .info-value { font-size: 14px; margin-top: 2px; color: #111827; }
  :global(.dark) .info-value { color: #f1f5f9; }
  .mono { font-family: 'JetBrains Mono', monospace; }
  .health-badge { display: inline-block; padding: 3px 12px; border-radius: 12px; font-size: 12px; font-weight: 700; }

  /* ── Lineage Banner ── */
  .lineage-banner {
    background: linear-gradient(135deg, #eff6ff, #f0fdf4);
    border: 1px solid #bfdbfe;
    border-radius: 8px;
    padding: 12px 16px;
    margin-bottom: 16px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  :global(.dark) .lineage-banner { background: linear-gradient(135deg, #1e3a5f, #14532d); border-color: #1e40af; }
  .lineage-row { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
  .lineage-icon { font-size: 18px; color: #2563eb; font-weight: 700; width: 20px; text-align: center; }
  .lineage-label { font-size: 12px; font-weight: 600; color: #374151; white-space: nowrap; }
  :global(.dark) .lineage-label { color: #d1d5db; }
  .lineage-children { display: flex; flex-wrap: wrap; gap: 6px; }
  .lineage-chip {
    display: inline-flex; align-items: center; gap: 6px;
    padding: 4px 10px; border-radius: 20px; font-size: 12px; font-weight: 600;
    cursor: pointer; border: none; transition: all 0.15s;
  }
  .parent-chip { background: #dbeafe; color: #1d4ed8; }
  .parent-chip:hover { background: #bfdbfe; }
  .child-chip { background: #dcfce7; color: #166534; }
  .child-chip:hover { background: #bbf7d0; }
  :global(.dark) .parent-chip { background: #1e3a8a; color: #93c5fd; }
  :global(.dark) .child-chip { background: #14532d; color: #86efac; }
  .lineage-chip-sub { font-size: 10px; font-weight: 400; opacity: 0.7; }

  /* ── Tabs ── */
  .tabs {
    display: flex; gap: 0;
    border-bottom: 2px solid #e2e8f0;
    margin-bottom: 0;
  }
  :global(.dark) .tabs { border-color: #334155; }
  .tab {
    display: flex; align-items: center; gap: 6px;
    padding: 10px 20px; background: none; border: none;
    border-bottom: 2px solid transparent; margin-bottom: -2px;
    cursor: pointer; font-size: 13px; font-weight: 600; color: #6b7280;
  }
  .tab.active { color: #2563eb; border-bottom-color: #2563eb; }
  .tab:hover { color: #374151; }
  :global(.dark) .tab:hover { color: #e2e8f0; }
  .tab-count {
    background: #e2e8f0; color: #374151; border-radius: 10px;
    padding: 1px 7px; font-size: 11px; font-weight: 700;
  }
  .tab.active .tab-count { background: #dbeafe; color: #1d4ed8; }

  /* ── Passage Form ── */
  .passage-form {
    border: 1px solid #e2e8f0; border-radius: 8px;
    padding: 16px; margin-bottom: 24px;
    background: #f8fafc;
  }
  :global(.dark) .passage-form { background: #1e293b; border-color: #334155; }
  .section-header {
    font-size: 11px; font-weight: 700; text-transform: uppercase;
    letter-spacing: 0.5px; color: #6b7280; margin: 12px 0 6px;
  }
  .env-field { flex: 0 0 110px; }
  .env-field-wide { flex: 0 0 175px; }
  .form-row { display: flex; gap: 10px; flex-wrap: wrap; margin-bottom: 10px; }
  .form-row .form-group { flex: 1; min-width: 120px; margin-bottom: 0; }

  /* Health slider */
  .health-slider-wrap {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .unknown-toggle {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    color: #7c3aed;
    cursor: pointer;
    text-transform: none;
    letter-spacing: 0;
    font-weight: 500;
  }
  .health-slider {
    -webkit-appearance: none;
    appearance: none;
    width: 100%;
    height: 6px;
    border-radius: 3px;
    background: linear-gradient(to right, #dc2626, #d97706, #ca8a04, #65a30d, #16a34a);
    outline: none;
    border: none !important;
    padding: 0 !important;
    cursor: pointer;
  }
  .health-slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--track-color, #16a34a);
    border: 2px solid white;
    box-shadow: 0 1px 4px rgba(0,0,0,0.3);
    cursor: pointer;
  }
  .health-slider::-moz-range-thumb {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--track-color, #16a34a);
    border: 2px solid white;
    box-shadow: 0 1px 4px rgba(0,0,0,0.3);
    cursor: pointer;
  }
  .health-ticks {
    display: flex;
    justify-content: space-between;
    font-size: 11px;
    color: #9ca3af;
  }
  .health-tick.active {
    font-weight: 700;
  }
  .health-display {
    font-size: 13px;
    font-weight: 700;
    margin-top: 2px;
  }

  /* Split toggle */
  .split-toggle-row {
    border: 1px dashed #d1d5db; border-radius: 6px;
    padding: 12px; margin-top: 4px; background: #fff;
  }
  :global(.dark) .split-toggle-row { background: #0f172a; border-color: #475569; }
  .split-toggle-label { font-size: 13px; font-weight: 600; color: #374151; cursor: pointer; display: flex; align-items: center; }
  :global(.dark) .split-toggle-label { color: #cbd5e1; }
  .split-count-row { display: flex; align-items: center; gap: 10px; margin-top: 10px; flex-wrap: wrap; }
  .split-desc { font-size: 13px; color: #374151; }
  :global(.dark) .split-desc { color: #94a3b8; }
  .split-count-input { width: 70px; padding: 4px 8px; border: 1px solid #d1d5db; border-radius: 4px; font-size: 14px; text-align: center; }
  :global(.dark) .split-count-input { background: #1e293b; color: #f1f5f9; border-color: #475569; }
  .split-hint { font-size: 11px; color: #6b7280; }
  .split-preview { display: flex; align-items: center; gap: 10px; margin-top: 10px; flex-wrap: wrap; }
  .split-preview-parent {
    padding: 4px 12px; background: #dbeafe; color: #1d4ed8;
    border-radius: 6px; font-size: 12px; font-weight: 700; font-family: monospace;
  }
  .split-arrow { font-size: 16px; color: #9ca3af; font-weight: 700; }
  .split-preview-children { display: flex; gap: 5px; flex-wrap: wrap; }
  .split-preview-child {
    padding: 3px 10px; background: #dcfce7; color: #166534;
    border-radius: 12px; font-size: 11px; font-weight: 600;
  }
  .split-preview-child.muted { background: #f3f4f6; color: #6b7280; }
  :global(.dark) .split-preview-parent { background: #1e3a8a; color: #93c5fd; }
  :global(.dark) .split-preview-child { background: #14532d; color: #86efac; }

  /* ── Timeline ── */
  .timeline { display: flex; flex-direction: column; gap: 0; }
  .timeline.with-form { margin-top: 0; }
  .timeline-item { display: flex; gap: 0; position: relative; }

  .timeline-left {
    display: flex; flex-direction: column; align-items: center;
    width: 32px; flex-shrink: 0; padding-top: 16px;
  }
  .tl-dot {
    width: 12px; height: 12px; border-radius: 50%;
    flex-shrink: 0; z-index: 1; position: relative;
  }
  .tl-line {
    width: 2px; flex: 1; background: #e2e8f0; margin-top: 4px; min-height: 16px;
  }
  :global(.dark) .tl-line { background: #334155; }

  .tl-card {
    flex: 1; margin: 8px 0 8px 8px;
    border: 1px solid #e2e8f0; border-radius: 8px;
    overflow: hidden; transition: box-shadow 0.15s;
    background: #fff;
  }
  :global(.dark) .tl-card { background: #1e293b; border-color: #334155; }
  .tl-card:hover { box-shadow: 0 2px 8px rgba(0,0,0,0.08); }
  .tl-card.expanded { border-color: #93c5fd; box-shadow: 0 2px 12px rgba(37,99,235,0.1); }
  :global(.dark) .tl-card.expanded { border-color: #1d4ed8; }

  .tl-card-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 12px 14px; width: 100%; background: none; border: none;
    cursor: pointer; text-align: left; gap: 10px;
  }
  .tl-card-header:hover { background: #f8fafc; }
  :global(.dark) .tl-card-header:hover { background: #0f172a; }
  .tl-card-left { display: flex; align-items: center; gap: 12px; flex: 1; min-width: 0; flex-wrap: wrap; }
  .tl-passage-num { font-size: 15px; font-weight: 800; font-family: monospace; flex-shrink: 0; }
  .tl-card-summary { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; min-width: 0; }
  .tl-date { font-size: 13px; font-weight: 600; color: #374151; }
  :global(.dark) .tl-date { color: #cbd5e1; }
  .tl-chevron { font-size: 12px; color: #9ca3af; flex-shrink: 0; }

  .tl-pill {
    display: inline-block; padding: 2px 8px; border-radius: 10px;
    font-size: 11px; font-weight: 500; white-space: nowrap;
    max-width: 200px; overflow: hidden; text-overflow: ellipsis;
  }
  .media-pill { background: #ede9fe; color: #5b21b6; }
  .vessel-pill { background: #e0f2fe; color: #0369a1; }
  .loc-pill { background: #f0fdf4; color: #166534; }
  :global(.dark) .media-pill { background: #3b0764; color: #c4b5fd; }
  :global(.dark) .vessel-pill { background: #0c4a6e; color: #7dd3fc; }
  :global(.dark) .loc-pill { background: #14532d; color: #86efac; }

  /* Card expanded body */
  .tl-card-body { padding: 0 14px 14px; border-top: 1px solid #f1f5f9; }
  :global(.dark) .tl-card-body { border-color: #334155; }
  .tl-detail-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
    gap: 10px; margin-top: 12px;
  }
  .tl-detail-item { display: flex; flex-direction: column; }
  .tl-detail-item.span2 { grid-column: span 2; }
  .tl-detail-label { font-size: 10px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.5px; color: #9ca3af; }
  .tl-detail-value { font-size: 13px; color: #111827; margin-top: 2px; }
  :global(.dark) .tl-detail-value { color: #f1f5f9; }
  .tl-detail-text { margin-top: 10px; }
  .tl-detail-p { margin: 3px 0 0; font-size: 13px; color: #374151; white-space: pre-wrap; line-height: 1.5; }
  :global(.dark) .tl-detail-p { color: #cbd5e1; }
</style>
