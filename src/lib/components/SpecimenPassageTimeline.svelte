<script lang="ts">
  import { updateSubculture } from '../api';
  import { addNotification, devMode } from '../stores/app';

  let {
    subcultures,
    specimenId,
    onreload,
    onnavigate = undefined,
  }: {
    subcultures: any[];
    specimenId: string;
    onreload: () => void;
    onnavigate?: (id: string) => void;
  } = $props();

  let expandedPassages = $state(new Set<string>());
  let editingPassageId = $state<string | null>(null);
  let passageEditForm = $state({ notes: '', observations: '', vessel_type: '', location_to: '' });

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

  function stageLabel(s: string | undefined | null) {
    return s?.replace(/_/g, ' ').replace(/\b\w/g, (c: string) => c.toUpperCase()) || '—';
  }

  function dotColor(passageNumber: number) {
    return dotColors[(passageNumber - 1) % dotColors.length];
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
      onreload();
    } catch (e: any) {
      addNotification(e.message, 'error');
    }
  }
</script>

{#if subcultures.length === 0}
  <div class="empty-state" style="padding:40px 0;">
    No passages recorded yet.<br/>
    <span style="font-size:12px;color:#9ca3af;">Use "Record Passage" above to log the first subculture event.</span>
  </div>
{:else}
  <div class="timeline">
    {#each subcultures as sc, i}
      {#if sc.isAncestralDivider}
        <!-- ── Ancestral section divider ── -->
        <div class="tl-ancestral-section">
          <div class="tl-ancestral-line"></div>
          <span class="tl-ancestral-label">Ancestral passages{sc.ancestorAccession ? ` — ${sc.ancestorAccession}` : ''}</span>
          <div class="tl-ancestral-line"></div>
        </div>
      {:else if sc.isSplitEvent}
        <!-- ── Split event card (expandable) ── -->
        {@const isSplitExpanded = expandedPassages.has(sc.id)}
        <div class="timeline-item">
          <div class="timeline-left">
            <div class="tl-dot tl-dot-split" title={sc.splitRole === 'child' ? 'Origin: split from parent' : 'End: specimen was split into children'}></div>
            {#if i < subcultures.length - 1 && !subcultures[i + 1]?.isAncestralDivider}
              <div class="tl-line"></div>
            {/if}
          </div>
          <div class="tl-card tl-card-split" class:expanded={isSplitExpanded}>
            <div
              class="tl-split-header tl-split-header-btn"
              role="button"
              tabindex="0"
              onclick={() => togglePassage(sc.id)}
              onkeydown={(e) => e.key === 'Enter' && togglePassage(sc.id)}
            >
              {#if sc.splitRole === 'child'}
                <span class="tl-split-icon">⑃</span>
                <div class="tl-card-left">
                  <span class="tl-passage-num tl-split-label" title="This specimen was created by splitting from a parent at P{sc.passage_number}">P{sc.passage_number} · Split</span>
                  <div class="tl-card-summary">
                    <span class="tl-date">{sc.date}</span>
                    <span class="tl-pill split-pill" title="This specimen was created by splitting from {sc.relatedAccession}">Split from {sc.relatedAccession}</span>
                  </div>
                </div>
              {:else}
                <span class="tl-split-icon">⑂</span>
                <div class="tl-card-left">
                  <span class="tl-passage-num tl-split-label" title="P{sc.passage_number} — this specimen was split into {sc.childCount} child specimen{sc.childCount > 1 ? 's' : ''}; children continue from P{sc.passage_number + 1}">P{sc.passage_number} · Split</span>
                  <div class="tl-card-summary">
                    <span class="tl-date">{sc.date}</span>
                    <span class="tl-pill split-pill">Split into {sc.childCount} child{sc.childCount > 1 ? 'ren' : ''}</span>
                  </div>
                </div>
              {/if}
              <span class="tl-chevron" style="color:#7c3aed;">{isSplitExpanded ? '▴' : '▾'}</span>
            </div>
            {#if isSplitExpanded}
              <div class="tl-split-body">
                {#if sc.splitRole === 'child'}
                  <!-- Expanded child split: show parent + siblings + initial state -->
                  <div class="tl-detail-grid" style="margin-bottom:10px;">
                    {#if sc.selfStage}
                      <div class="tl-detail-item">
                        <span class="tl-detail-label">Stage at Creation</span>
                        <span class="tl-detail-value">{stageLabel(sc.selfStage)}</span>
                      </div>
                    {/if}
                    {#if sc.selfLocation}
                      <div class="tl-detail-item">
                        <span class="tl-detail-label">Initial Location</span>
                        <span class="tl-detail-value">{sc.selfLocation}</span>
                      </div>
                    {/if}
                    {#if sc.selfHealth !== null && sc.selfHealth !== '' && !isNaN(Number(sc.selfHealth))}
                      {@const hb = healthInfo(sc.selfHealth)}
                      {#if hb}
                        <div class="tl-detail-item">
                          <span class="tl-detail-label">Health at Creation</span>
                          <span class="tl-detail-value">
                            <span style="display:inline-block;padding:2px 8px;border-radius:10px;font-size:12px;font-weight:700;background:{hb.color}20;color:{hb.color};border:1px solid {hb.color}60;">{hb.label}</span>
                          </span>
                        </div>
                      {/if}
                    {/if}
                  </div>
                  <div style="margin-bottom:8px;">
                    <div class="tl-detail-label" style="margin-bottom:5px;">Parent Specimen</div>
                    <button
                      class="tl-nav-chip"
                      onclick={() => onnavigate?.(sc.relatedId)}
                      title="Navigate to parent specimen {sc.relatedAccession}"
                      disabled={!onnavigate}
                    >{sc.relatedAccession}</button>
                  </div>
                  {#if sc.siblings?.length > 0}
                    <div>
                      <div class="tl-detail-label" style="margin-bottom:5px;">Siblings from Same Split</div>
                      <div style="display:flex;flex-wrap:wrap;gap:5px;">
                        {#each sc.siblings as sib}
                          <button
                            class="tl-nav-chip"
                            onclick={() => onnavigate?.(sib.id)}
                            title="Navigate to sibling {sib.accession_number}"
                            disabled={!onnavigate}
                            style={sib.is_archived ? 'opacity:0.6;' : ''}
                          >{sib.accession_number}{sib.is_archived ? ' (archived)' : ''}</button>
                        {/each}
                      </div>
                    </div>
                  {/if}
                  {#if sc.selfContaminationFlag}
                    <div class="tl-contam-block" style="margin-top:10px;">
                      <span>⚠ <strong>Contamination inherited from split parent</strong></span>
                      {#if sc.selfContaminationNotes || sc.parentContaminationNotes}
                        <p class="tl-detail-p" style="margin-top:4px;">{sc.selfContaminationNotes || sc.parentContaminationNotes}</p>
                      {/if}
                    </div>
                  {:else if sc.parentContaminationFlag}
                    <div class="tl-contam-block tl-contam-info" style="margin-top:10px;">
                      <span>ℹ <strong>Parent specimen was contaminated — this specimen was not flagged at creation</strong></span>
                      {#if sc.parentContaminationNotes}
                        <p class="tl-detail-p" style="margin-top:4px;">{sc.parentContaminationNotes}</p>
                      {/if}
                    </div>
                  {/if}
                {:else}
                  <!-- Expanded parent split: show parent state + children chips -->
                  <div class="tl-detail-grid" style="margin-bottom:10px;">
                    {#if sc.parentStage}
                      <div class="tl-detail-item">
                        <span class="tl-detail-label">Stage at Split</span>
                        <span class="tl-detail-value">{stageLabel(sc.parentStage)}</span>
                      </div>
                    {/if}
                    {#if sc.parentLocation}
                      <div class="tl-detail-item">
                        <span class="tl-detail-label">Location at Split</span>
                        <span class="tl-detail-value">{sc.parentLocation}</span>
                      </div>
                    {/if}
                    {#if sc.parentHealth !== null && sc.parentHealth !== '' && !isNaN(Number(sc.parentHealth))}
                      {@const hb = healthInfo(sc.parentHealth)}
                      {#if hb}
                        <div class="tl-detail-item">
                          <span class="tl-detail-label">Health at Split</span>
                          <span class="tl-detail-value">
                            <span style="display:inline-block;padding:2px 8px;border-radius:10px;font-size:12px;font-weight:700;background:{hb.color}20;color:{hb.color};border:1px solid {hb.color}60;">{hb.label}</span>
                          </span>
                        </div>
                      {/if}
                    {/if}
                  </div>
                  {#if sc.parentNotes}
                    <div class="tl-detail-text" style="margin-bottom:10px;">
                      <span class="tl-detail-label">Notes</span>
                      <p class="tl-detail-p">{sc.parentNotes}</p>
                    </div>
                  {/if}
                  {#if sc.parentContaminationFlag}
                    <div class="tl-contam-block">
                      <span>⚠ <strong>Contamination detected at time of split</strong></span>
                      {#if sc.parentContaminationNotes}
                        <p class="tl-detail-p" style="margin-top:4px;">{sc.parentContaminationNotes}</p>
                      {/if}
                    </div>
                  {/if}
                  <div>
                    <div class="tl-detail-label" style="margin-bottom:5px;">Children Created (P{sc.passage_number + 1}+)</div>
                    <div style="display:flex;flex-wrap:wrap;gap:5px;">
                      {#each (sc.childAccessions || []) as acc, idx}
                        {@const childId = sc.childIds?.[idx]}
                        <button
                          class="tl-nav-chip"
                          onclick={() => onnavigate?.(childId)}
                          title="Navigate to child specimen {acc}"
                          disabled={!childId || !onnavigate}
                        >{acc}</button>
                      {/each}
                    </div>
                  </div>
                {/if}
              </div>
            {/if}
          </div>
        </div>
      {:else if sc.event_type === 'death'}
        <!-- ── Death event card ── -->
        <div class="timeline-item">
          <div class="timeline-left">
            <div class="tl-dot tl-dot-death" title="Terminal event — specimen was marked dead and archived"></div>
            {#if i < subcultures.length - 1 && !subcultures[i + 1]?.isAncestralDivider}
              <div class="tl-line tl-line-death"></div>
            {/if}
          </div>
          <div class="tl-card tl-card-death">
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            {@const isExpanded = expandedPassages.has(sc.id)}
            <div class="tl-card-header tl-card-death-header" role="button" tabindex="0" onclick={() => togglePassage(sc.id)} onkeydown={(e) => e.key === 'Enter' && togglePassage(sc.id)}>
              <div class="tl-card-left">
                <span class="tl-passage-num tl-death-label" title="Terminal death event — specimen marked dead and archived">☠ Death</span>
                <div class="tl-card-summary">
                  <span class="tl-date">{sc.date}</span>
                  <span class="tl-pill death-pill" title="Specimen was permanently archived">Archived</span>
                </div>
              </div>
              <span class="tl-chevron" style="color:#dc2626;">{isExpanded ? '▴' : '▾'}</span>
            </div>
            {#if isExpanded}
              <div class="tl-card-body tl-death-body">
                <div class="tl-detail-grid">
                  {#if sc.employee_id}
                    <div class="tl-detail-item">
                      <span class="tl-detail-label">Employee ID</span>
                      <span class="tl-detail-value">{sc.employee_id}</span>
                    </div>
                  {/if}
                  {#if sc.performer_name}
                    <div class="tl-detail-item">
                      <span class="tl-detail-label">Recorded By</span>
                      <span class="tl-detail-value">{sc.performer_name}</span>
                    </div>
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
              </div>
            {/if}
          </div>
        </div>
      {:else}
        <!-- ── Normal passage card (with optional ancestral tint) ── -->
        {@const color = dotColor(sc.passage_number)}
        {@const isExpanded = expandedPassages.has(sc.id)}
        <div class="timeline-item">
          <div class="timeline-left">
            <div class="tl-dot" style="background:{color};box-shadow:0 0 0 3px {color}30;"></div>
            {#if i < subcultures.length - 1 && !subcultures[i + 1]?.isAncestralDivider}
              <div class="tl-line"></div>
            {/if}
          </div>
          <div class="tl-card" class:expanded={isExpanded} class:ancestral={sc.isAncestral}>
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div class="tl-card-header" role="button" tabindex="0" onclick={() => togglePassage(sc.id)} onkeydown={(e) => e.key === 'Enter' && togglePassage(sc.id)}>
              <div class="tl-card-left">
                <span class="tl-passage-num" title="Passage number — number of times this specimen has been subcultured (P{sc.passage_number})" style="color:{color};">P{sc.passage_number}</span>
                <div class="tl-card-summary">
                  <span class="tl-date">{sc.date}</span>
                  {#if sc.contamination_flag}
                    <span class="tl-pill contam-pill" title="Contamination was detected during this passage">⚠ Contaminated</span>
                  {/if}
                  {#if sc.media_batch_name}
                    <span class="tl-pill media-pill" title="Media batch used for this passage: {sc.media_batch_name}">{sc.media_batch_name}</span>
                  {/if}
                  {#if sc.vessel_type}
                    <span class="tl-pill vessel-pill" title="Vessel type used for this passage: {sc.vessel_type}">{sc.vessel_type}</span>
                  {/if}
                  {#if sc.location_to}
                    <span class="tl-pill loc-pill" title="Destination location for this passage: {sc.location_to}">→ {sc.location_to}</span>
                  {/if}
                </div>
              </div>
              <div style="display:flex;align-items:center;gap:8px;">
                {#if $devMode && isExpanded}
                  <button
                    type="button"
                    class="btn btn-sm"
                    title={editingPassageId === sc.id ? 'Discard changes and exit inline edit mode for this passage' : 'Edit the notes, vessel, location, and observations for this passage record (dev mode)'}
                    style="background:#dc2626; color:white;"
                    onclick={(e) => { e.stopPropagation(); if (editingPassageId === sc.id) { cancelEditPassage(); } else { startEditPassage(sc); } }}
                  >
                    {editingPassageId === sc.id ? 'Cancel Edit' : 'Edit'}
                  </button>
                {/if}
                <span class="tl-chevron">{isExpanded ? '▴' : '▾'}</span>
              </div>
            </div>
            {#if isExpanded}
              <div class="tl-card-body">
                {#if $devMode && editingPassageId === sc.id}
                  <form onsubmit={(e) => handleEditPassage(e, sc.id)} style="margin-top:12px;display:flex;flex-direction:column;gap:10px;">
                    <div class="form-row">
                      <div class="form-group" style="flex:2;">
                        <label for="tl-vessel-{sc.id}" title="Edit the vessel type used for this passage">Vessel Type</label>
                        <select id="tl-vessel-{sc.id}" title="Edit the vessel type used for this passage" bind:value={passageEditForm.vessel_type}>
                          <option value="">Select vessel…</option>
                          {#each vesselTypes as v}
                            <option value={v}>{v}</option>
                          {/each}
                        </select>
                      </div>
                      <div class="form-group" style="flex:2;">
                        <label for="tl-loc-{sc.id}" title="Edit the destination location recorded for this passage">Location To</label>
                        <input id="tl-loc-{sc.id}" type="text" title="Edit the destination location recorded for this passage" bind:value={passageEditForm.location_to} placeholder="e.g., Room 1 / Rack A / Shelf 2" />
                      </div>
                    </div>
                    <div class="form-row">
                      <div class="form-group" style="flex:1;">
                        <label for="tl-obs-{sc.id}" title="Edit the visual or qualitative observations recorded for this passage">Observations</label>
                        <textarea id="tl-obs-{sc.id}" title="Edit the visual or qualitative observations recorded for this passage" bind:value={passageEditForm.observations} rows="2" placeholder="Growth observations, morphology…"></textarea>
                      </div>
                      <div class="form-group" style="flex:1;">
                        <label for="tl-notes-{sc.id}" title="Edit the procedural notes recorded for this passage">Notes</label>
                        <textarea id="tl-notes-{sc.id}" title="Edit the procedural notes recorded for this passage" bind:value={passageEditForm.notes} rows="2" placeholder="Protocol notes, reagent lots…"></textarea>
                      </div>
                    </div>
                    <div style="text-align:right;">
                      <button type="button" class="btn btn-sm" title="Discard changes and exit inline edit mode" onclick={cancelEditPassage} style="margin-right:6px;">Cancel</button>
                      <button type="submit" class="btn btn-primary btn-sm" title="Save the edited fields for this passage record">Save Changes</button>
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
                            <span title="Health score at time of this passage (0=Dead, 1=Poor, 2=Fair, 3=Good, 4=Healthy)" style="display:inline-block;padding:2px 8px;border-radius:10px;font-size:12px;font-weight:700;background:{hb.color}20;color:{hb.color};border:1px solid {hb.color}60;">
                              {hb.label}
                            </span>
                          </span>
                        </div>
                      {/if}
                    {/if}
                  </div>
                  {#if sc.contamination_flag}
                    <div class="tl-detail-text contam-detail">
                      <span class="tl-detail-label">Contamination</span>
                      <p class="tl-detail-p">
                        {sc.contamination_notes || 'Contamination flagged — no notes recorded.'}
                      </p>
                    </div>
                  {/if}
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
      {/if}
    {/each}
  </div>
{/if}

<style>
  .form-row { display: flex; gap: 10px; flex-wrap: wrap; margin-bottom: 10px; }
  .form-row .form-group { flex: 1; min-width: 120px; margin-bottom: 0; }

  .timeline { display: flex; flex-direction: column; gap: 0; }
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
    padding: 12px 14px; width: 100%;
    cursor: pointer; gap: 10px; user-select: none;
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
  .contam-pill { background: #fee2e2; color: #b91c1c; font-weight: 700; }
  :global(.dark) .media-pill { background: #3b0764; color: #c4b5fd; }
  :global(.dark) .vessel-pill { background: #0c4a6e; color: #7dd3fc; }
  :global(.dark) .loc-pill { background: #14532d; color: #86efac; }
  :global(.dark) .contam-pill { background: #7f1d1d; color: #fca5a5; }
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
  .contam-detail { background: #fff1f2; border-radius: 6px; padding: 8px 10px; margin-top: 10px; }
  :global(.dark) .contam-detail { background: #450a0a; }
  .contam-detail .tl-detail-label { color: #b91c1c; }
  :global(.dark) .contam-detail .tl-detail-label { color: #f87171; }
  .contam-detail .tl-detail-p { color: #7f1d1d; }
  :global(.dark) .contam-detail .tl-detail-p { color: #fca5a5; }

  /* Split event cards */
  .tl-dot-split {
    width: 14px; height: 14px; border-radius: 4px;
    background: #7c3aed;
    box-shadow: 0 0 0 3px #7c3aed30;
    flex-shrink: 0; z-index: 1; position: relative;
  }
  .tl-card-split {
    flex: 1; margin: 8px 0 8px 8px;
    border: 1px dashed #c4b5fd; border-radius: 8px;
    background: #faf5ff;
    overflow: hidden;
  }
  :global(.dark) .tl-card-split { background: #2e1065; border-color: #4c1d95; }
  .tl-split-header {
    display: flex; align-items: center; gap: 10px;
    padding: 10px 14px;
  }
  .tl-split-icon {
    font-size: 18px; color: #7c3aed; flex-shrink: 0;
    line-height: 1;
  }
  .tl-split-label {
    color: #7c3aed !important;
  }
  .split-pill { background: #ede9fe; color: #5b21b6; }
  :global(.dark) .split-pill { background: #3b0764; color: #c4b5fd; }

  /* Expandable split card states */
  .tl-split-header-btn { cursor: pointer; user-select: none; justify-content: space-between; }
  .tl-split-header-btn:hover { background: #f3e8ff; }
  :global(.dark) .tl-split-header-btn:hover { background: #3b0764; }
  .tl-card-split.expanded { border-color: #a78bfa; box-shadow: 0 2px 12px rgba(124,58,237,0.15); }
  :global(.dark) .tl-card-split.expanded { border-color: #7c3aed; box-shadow: 0 2px 12px rgba(124,58,237,0.25); }
  .tl-split-body { padding: 0 14px 14px; border-top: 1px solid #ede9fe; }
  :global(.dark) .tl-split-body { border-color: #4c1d95; }

  /* Navigation chips inside expanded split events */
  .tl-nav-chip {
    display: inline-block; padding: 3px 10px; border-radius: 12px;
    font-size: 12px; font-weight: 600; font-family: 'JetBrains Mono', monospace;
    background: #ede9fe; color: #5b21b6;
    border: 1px solid #c4b5fd; cursor: pointer;
    transition: background 0.12s, color 0.12s;
  }
  .tl-nav-chip:hover:not(:disabled) { background: #7c3aed; color: #fff; border-color: #7c3aed; }
  .tl-nav-chip:disabled { opacity: 0.5; cursor: default; }
  :global(.dark) .tl-nav-chip { background: #3b0764; color: #e9d5ff; border-color: #6d28d9; }
  :global(.dark) .tl-nav-chip:hover:not(:disabled) { background: #7c3aed; color: #fff; }

  /* Ancestral section divider */
  .tl-ancestral-section {
    display: flex; align-items: center; gap: 10px;
    padding: 16px 0 8px; margin-left: 40px;
  }
  .tl-ancestral-line { flex: 1; height: 1px; background: #d1d5db; border: none; }
  :global(.dark) .tl-ancestral-line { background: #475569; }
  .tl-ancestral-label {
    font-size: 11px; font-weight: 700; text-transform: uppercase;
    letter-spacing: 0.6px; color: #9ca3af; white-space: nowrap;
  }
  :global(.dark) .tl-ancestral-label { color: #64748b; }

  /* Ancestral passage cards — subtle tint to distinguish from current lineage */
  .tl-card.ancestral { background: #f8fafc; border-color: #e2e8f0; opacity: 0.85; }
  :global(.dark) .tl-card.ancestral { background: #0f172a; border-color: #1e293b; }
  .tl-card.ancestral:hover { opacity: 1; box-shadow: 0 2px 8px rgba(0,0,0,0.06); }

  /* Death event cards */
  .tl-dot-death {
    width: 14px; height: 14px; border-radius: 50%;
    background: #dc2626;
    box-shadow: 0 0 0 3px #dc262630;
    flex-shrink: 0; z-index: 1; position: relative;
  }
  .tl-line-death { background: #fecaca; }
  :global(.dark) .tl-line-death { background: #7f1d1d; }
  .tl-card-death {
    border: 1px solid #fca5a5;
    background: #fff1f2;
  }
  :global(.dark) .tl-card-death { background: #1f0000; border-color: #7f1d1d; }
  .tl-card-death-header:hover { background: #fee2e2; }
  :global(.dark) .tl-card-death-header:hover { background: #2d0000; }
  .tl-death-label { color: #dc2626 !important; }
  .death-pill { background: #fee2e2; color: #b91c1c; font-weight: 700; }
  :global(.dark) .death-pill { background: #7f1d1d; color: #fca5a5; }
  .tl-death-body { border-top-color: #fecaca; }
  :global(.dark) .tl-death-body { border-top-color: #7f1d1d; }

  /* Contamination block inside expanded parent split card */
  .tl-contam-block {
    padding: 8px 10px; border-radius: 6px; margin-bottom: 10px;
    background: #fff1f2; border: 1px solid #fecdd3;
    font-size: 12px; color: #b91c1c;
  }
  :global(.dark) .tl-contam-block { background: #450a0a; border-color: #7f1d1d; color: #fca5a5; }
  /* Amber variant: pre-v1.9.0 informational block for children of contaminated parents */
  .tl-contam-info { background: #fff8f1; border-color: #fed7aa; color: #92400e; }
  :global(.dark) .tl-contam-info { background: #431a02; border-color: #92400e; color: #fcd34d; }
  .tl-contam-info .tl-detail-p { color: #78350f; }
  :global(.dark) .tl-contam-info .tl-detail-p { color: #fbbf24; }
</style>
