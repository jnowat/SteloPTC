<script lang="ts">
  import { onMount } from 'svelte';
  import { listStrainsBySpecies, createStrain, updateStrain, archiveStrain, updateStrainStatus } from '../api';
  import { addNotification, addErrorWithContext } from '../stores/app';
  import HybridWizard from './HybridWizard.svelte';
  import StrainDetail from './StrainDetail.svelte';

  let { speciesId, speciesName = '' }: { speciesId: string; speciesName?: string } = $props();

  let strains = $state<any[]>([]);
  let loading = $state(true);
  let statusFilter = $state('all');
  let searchQuery = $state('');

  // Create modal
  let showCreate = $state(false);
  let createForm = $state({ name: '', code: '', strain_type: 'wildtype' });
  let createLoading = $state(false);

  // Edit modal
  let editStrain = $state<any>(null);
  let editForm = $state({ name: '', code: '', strain_type: '' });
  let editLoading = $state(false);

  // Archive confirm
  let archiveTarget = $state<any>(null);
  let archiveLoading = $state(false);

  // Status update modal
  let statusTarget = $state<any>(null);
  let statusForm = $state({ status: '', claimed_by: '', claimed_at: '', confirmation_basis: '', genomic_fingerprint: '' });
  let statusLoading = $state(false);
  let nextStatuses = $derived(statusTarget ? availableStatuses(statusTarget.status) : []);

  // Blocking confirmed_manual acknowledgment modal
  let showConfirmedManualModal = $state(false);

  // Hybrid wizard
  let showHybridWizard = $state(false);

  // Strain detail slide-over
  let detailStrainId = $state<string | null>(null);

  const today = new Date().toISOString().split('T')[0];

  const strainTypes = [
    { value: 'wildtype', label: 'Wild Type' },
    { value: 'cultivar', label: 'Cultivar' },
    { value: 'hybrid', label: 'Hybrid' },
    { value: 'mutant', label: 'Mutant' },
    { value: 'selection', label: 'Selection' },
  ];

  function isOlderThan30Days(createdAt: string): boolean {
    try {
      const created = new Date(createdAt).getTime();
      const now = Date.now();
      return (now - created) / (1000 * 60 * 60 * 24) > 30;
    } catch {
      return false;
    }
  }

  let filtered = $derived(
    strains.filter(s => {
      if (statusFilter !== 'all' && s.status !== statusFilter) return false;
      if (searchQuery.trim()) {
        const q = searchQuery.toLowerCase();
        if (!s.name.toLowerCase().includes(q) && !s.code.toLowerCase().includes(q)) return false;
      }
      return true;
    })
  );

  async function load() {
    loading = true;
    try {
      strains = await listStrainsBySpecies(speciesId);
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      loading = false;
    }
  }

  onMount(load);

  async function handleCreate(e: Event) {
    e.preventDefault();
    if (!createForm.name.trim() || !createForm.code.trim()) {
      addNotification('Name and code are required', 'warning');
      return;
    }
    createLoading = true;
    try {
      await createStrain({ species_id: speciesId, name: createForm.name.trim(), code: createForm.code.trim(), strain_type: createForm.strain_type });
      addNotification('Strain created', 'success');
      showCreate = false;
      createForm = { name: '', code: '', strain_type: 'wildtype' };
      await load();
    } catch (e: any) {
      addErrorWithContext('Failed to Create Strain', e.message, 'strains.create', { speciesId, ...createForm });
    } finally {
      createLoading = false;
    }
  }

  function openEdit(s: any) {
    editStrain = s;
    editForm = { name: s.name, code: s.code, strain_type: s.strain_type };
  }

  async function handleEdit(e: Event) {
    e.preventDefault();
    if (!editStrain) return;
    editLoading = true;
    try {
      await updateStrain({ id: editStrain.id, name: editForm.name.trim(), code: editForm.code.trim(), strain_type: editForm.strain_type });
      addNotification('Strain updated', 'success');
      editStrain = null;
      await load();
    } catch (e: any) {
      addErrorWithContext('Failed to Update Strain', e.message, 'strains.update', { id: editStrain?.id, ...editForm });
    } finally {
      editLoading = false;
    }
  }

  async function handleArchive() {
    if (!archiveTarget) return;
    archiveLoading = true;
    try {
      await archiveStrain(archiveTarget.id);
      addNotification('Strain archived', 'success');
      archiveTarget = null;
      await load();
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      archiveLoading = false;
    }
  }

  function openStatusUpdate(s: any) {
    statusTarget = s;
    statusForm = {
      status: s.status,
      claimed_by: s.claimed_by || '',
      claimed_at: s.claimed_at || today,
      confirmation_basis: s.confirmation_basis || '',
      genomic_fingerprint: s.genomic_fingerprint || '',
    };
  }

  function availableStatuses(current: string): string[] {
    if (current === 'confirmed_genomic') return [];
    if (current === 'confirmed_manual') return ['confirmed_genomic'];
    if (current === 'claimed') return ['confirmed_manual', 'confirmed_genomic'];
    return ['claimed', 'confirmed_manual', 'confirmed_genomic'];
  }

  async function handleStatusUpdate(e: Event) {
    e.preventDefault();
    if (!statusTarget) return;
    statusLoading = true;
    try {
      const result = await updateStrainStatus({
        id: statusTarget.id,
        status: statusForm.status,
        claimed_by: statusForm.claimed_by || undefined,
        claimed_at: statusForm.claimed_at || undefined,
        confirmation_basis: statusForm.confirmation_basis || undefined,
        genomic_fingerprint: statusForm.genomic_fingerprint || undefined,
      });
      statusTarget = null;
      await load();
      if (result && result.status === 'confirmed_manual') {
        showConfirmedManualModal = true;
      } else {
        addNotification('Status updated', 'success');
      }
    } catch (e: any) {
      addErrorWithContext('Failed to Update Status', e.message, 'strains.status', { id: statusTarget?.id, status: statusForm.status });
    } finally {
      statusLoading = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (showConfirmedManualModal) {
      e.preventDefault();
      e.stopPropagation();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="strain-manager">
  <div class="sm-header">
    <div>
      <h2>{speciesName ? `Strains — ${speciesName}` : 'Strain Manager'}</h2>
      <p class="sm-sub">Manage and track strain identities for this species.</p>
    </div>
    <div class="sm-actions">
      <button class="btn btn-sm" onclick={() => (showHybridWizard = true)}>+ New Hybrid Strain</button>
      <button class="btn btn-sm btn-primary" onclick={() => (showCreate = true)}>+ New Strain</button>
    </div>
  </div>

  <div class="sm-toolbar">
    <input
      type="search"
      placeholder="Search by name or code…"
      bind:value={searchQuery}
      class="sm-search"
    />
    <select bind:value={statusFilter} class="sm-filter">
      <option value="all">All Statuses</option>
      <option value="unverified">Unverified</option>
      <option value="claimed">Claimed</option>
      <option value="confirmed_manual">Confirmed (Manual)</option>
      <option value="confirmed_genomic">Confirmed (Genomic)</option>
    </select>
  </div>

  {#if loading}
    <div class="empty-state">Loading strains…</div>
  {:else if filtered.length === 0}
    <div class="empty-state">
      {strains.length === 0 ? 'No strains registered for this species yet.' : 'No strains match the current filters.'}
    </div>
  {:else}
    <div class="sm-table-wrap">
      <table>
        <thead>
          <tr>
            <th>Name</th>
            <th>Code</th>
            <th>Type</th>
            <th>Status</th>
            <th>Specimens</th>
            <th>Created</th>
            <th>Actions</th>
          </tr>
        </thead>
        <tbody>
          {#each filtered as s}
            <tr class:archived={s.is_archived}>
              <td>
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <span class="strain-name-link" onclick={() => (detailStrainId = s.id)} title="View strain details">{s.name}</span>
                {#if s.is_hybrid}
                  <span class="hybrid-chip" title="Hybrid strain — created via hybridization event">&#128300; Hybrid</span>
                {/if}
                {#if s.is_cross_species}
                  <span class="cross-species-chip" title="Cross-species hybrid — permanent audit warning on record">&#9888; Cross-sp.</span>
                {/if}
                {#if s.status === 'unverified' && isOlderThan30Days(s.created_at)}
                  <span class="pulse-dot" title="This strain has been unverified for more than 30 days"></span>
                {/if}
              </td>
              <td><code class="strain-code">{s.code}</code></td>
              <td>{s.strain_type || '—'}</td>
              <td>
                <span class="status-badge status-{s.status}" title={
                  s.status === 'unverified' ? 'No identity assertion has been made for this strain.' :
                  s.status === 'claimed' ? 'Identity asserted by lab staff but not independently verified.' :
                  s.status === 'confirmed_manual' ? 'Manually confirmed. Not equivalent to genomic verification — see audit log for the documented basis.' :
                  'Genomic verification confirmed. Fingerprint data on record.'
                }>
                  {s.status === 'unverified' ? 'Unverified' :
                   s.status === 'claimed' ? 'Claimed' :
                   s.status === 'confirmed_manual' ? '⚠ Manual ID' :
                   '✓ Genomic'}
                </span>
                {#if s.status === 'unverified'}
                  <button
                    class="nudge-link"
                    title="Mark this strain's identity as claimed"
                    onclick={() => { openStatusUpdate(s); statusForm.status = 'claimed'; }}
                  >Mark as Claimed</button>
                {/if}
              </td>
              <td>{s.specimen_count ?? 0}</td>
              <td title={s.created_at}>{s.created_at?.split('T')[0] ?? '—'}</td>
              <td>
                {#if !s.is_archived}
                  <div class="row-actions">
                    <button class="btn btn-sm" onclick={() => openEdit(s)} title="Edit this strain's name, code, or type">Edit</button>
                    <button class="btn btn-sm" onclick={() => openStatusUpdate(s)} title="Update this strain's identification status" disabled={s.status === 'confirmed_genomic'}>Status</button>
                    <button class="btn btn-sm btn-danger" onclick={() => (archiveTarget = s)} title="Archive this strain — removes it from active lists">Archive</button>
                  </div>
                {:else}
                  <span class="badge badge-gray">Archived</span>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

<!-- ── Create Strain Modal ── -->
{#if showCreate}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-backdrop" onclick={() => (showCreate = false)}>
    <div class="modal-box" role="dialog" aria-modal="true" aria-labelledby="create-strain-title">
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div onclick={(e) => e.stopPropagation()}>
        <div class="modal-header">
          <h3 id="create-strain-title">New Strain</h3>
          <button class="modal-close" onclick={() => (showCreate = false)} aria-label="Close">&#10005;</button>
        </div>
        <form onsubmit={handleCreate}>
          <div class="form-group">
            <label for="cn">Name *</label>
            <input id="cn" type="text" bind:value={createForm.name} placeholder="e.g., Clone A-3" required />
          </div>
          <div class="form-group">
            <label for="cc">Code *</label>
            <input id="cc" type="text" bind:value={createForm.code} placeholder="e.g., CLN-A3" required />
          </div>
          <div class="form-group">
            <label for="ct">Strain Type</label>
            <select id="ct" bind:value={createForm.strain_type}>
              {#each strainTypes as t}
                <option value={t.value}>{t.label}</option>
              {/each}
            </select>
          </div>
          <div class="modal-footer">
            <button type="button" class="btn btn-sm" onclick={() => (showCreate = false)}>Cancel</button>
            <button type="submit" class="btn btn-sm btn-primary" disabled={createLoading}>{createLoading ? 'Creating…' : 'Create Strain'}</button>
          </div>
        </form>
      </div>
    </div>
  </div>
{/if}

<!-- ── Edit Strain Modal ── -->
{#if editStrain}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-backdrop" onclick={() => (editStrain = null)}>
    <div class="modal-box" role="dialog" aria-modal="true" aria-labelledby="edit-strain-title">
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div onclick={(e) => e.stopPropagation()}>
        <div class="modal-header">
          <h3 id="edit-strain-title">Edit Strain</h3>
          <button class="modal-close" onclick={() => (editStrain = null)} aria-label="Close">&#10005;</button>
        </div>
        <form onsubmit={handleEdit}>
          <div class="form-group">
            <label for="en">Name</label>
            <input id="en" type="text" bind:value={editForm.name} required />
          </div>
          <div class="form-group">
            <label for="ec">Code</label>
            <input id="ec" type="text" bind:value={editForm.code} required />
          </div>
          <div class="form-group">
            <label for="et">Strain Type</label>
            <select id="et" bind:value={editForm.strain_type}>
              {#each strainTypes as t}
                <option value={t.value}>{t.label}</option>
              {/each}
            </select>
          </div>
          <div class="modal-footer">
            <button type="button" class="btn btn-sm" onclick={() => (editStrain = null)}>Cancel</button>
            <button type="submit" class="btn btn-sm btn-primary" disabled={editLoading}>{editLoading ? 'Saving…' : 'Save Changes'}</button>
          </div>
        </form>
      </div>
    </div>
  </div>
{/if}

<!-- ── Archive Confirm ── -->
{#if archiveTarget}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-backdrop" onclick={() => (archiveTarget = null)}>
    <div class="modal-box modal-sm" role="dialog" aria-modal="true" aria-labelledby="archive-strain-title">
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div onclick={(e) => e.stopPropagation()}>
        <div class="modal-header">
          <h3 id="archive-strain-title">Archive Strain?</h3>
        </div>
        <p style="margin-bottom:16px;color:#6b7280;font-size:13px;">
          Archive <strong>{archiveTarget.name}</strong> ({archiveTarget.code})? It will be hidden from active lists but preserved in audit history.
        </p>
        <div class="modal-footer">
          <button class="btn btn-sm" onclick={() => (archiveTarget = null)}>Cancel</button>
          <button class="btn btn-sm btn-danger" disabled={archiveLoading} onclick={handleArchive}>{archiveLoading ? 'Archiving…' : 'Archive'}</button>
        </div>
      </div>
    </div>
  </div>
{/if}

<!-- ── Status Update Modal ── -->
{#if statusTarget}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-backdrop" onclick={() => (statusTarget = null)}>
    <div class="modal-box" role="dialog" aria-modal="true" aria-labelledby="status-strain-title">
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div onclick={(e) => e.stopPropagation()}>
        <div class="modal-header">
          <h3 id="status-strain-title">Update Status — {statusTarget.name}</h3>
          <button class="modal-close" onclick={() => (statusTarget = null)} aria-label="Close">&#10005;</button>
        </div>
        <form onsubmit={handleStatusUpdate}>
          <div class="current-status-row">
            <span style="font-size:12px;color:#6b7280;">Current:</span>
            <span class="status-badge status-{statusTarget.status}">
              {statusTarget.status === 'unverified' ? 'Unverified' :
               statusTarget.status === 'claimed' ? 'Claimed' :
               statusTarget.status === 'confirmed_manual' ? '⚠ Manual ID' : '✓ Genomic'}
            </span>
          </div>

          {#if nextStatuses.length === 0}
            <p class="info-row">This strain has reached its final status (Genomic) and cannot be downgraded.</p>
          {:else}
            <div class="form-group">
              <label for="ns">New Status</label>
              <select id="ns" bind:value={statusForm.status}>
                <option value="" disabled>Select…</option>
                {#each nextStatuses as ns}
                  <option value={ns}>
                    {ns === 'claimed' ? 'Claimed' :
                     ns === 'confirmed_manual' ? '⚠ Manual ID (Confirmed — Manual)' :
                     '✓ Genomic (Confirmed — Genomic)'}
                  </option>
                {/each}
              </select>
            </div>

            {#if statusForm.status === 'claimed'}
              <div class="form-group">
                <label for="cby">Claimed By</label>
                <input id="cby" type="text" bind:value={statusForm.claimed_by} placeholder="Name or employee ID" />
              </div>
              <div class="form-group">
                <label for="cat">Claimed At</label>
                <input id="cat" type="date" bind:value={statusForm.claimed_at} />
              </div>
            {/if}

            {#if statusForm.status === 'confirmed_manual'}
              <div class="warning-box">
                <strong>⚠ Manual Confirmation</strong>
                <p>This designation is permanent. A non-empty confirmation basis is required. This is not equivalent to genomic verification.</p>
              </div>
              <div class="form-group">
                <label for="cb">Confirmation Basis *</label>
                <textarea id="cb" bind:value={statusForm.confirmation_basis} rows="3" placeholder="Describe the professional basis for this identification…" required></textarea>
              </div>
            {/if}

            {#if statusForm.status === 'confirmed_genomic'}
              <div class="form-group">
                <label for="gf">Genomic Fingerprint *</label>
                <input id="gf" type="text" bind:value={statusForm.genomic_fingerprint} placeholder="Fingerprint ID, hash, or reference" required />
              </div>
            {/if}

            <div class="modal-footer">
              <button type="button" class="btn btn-sm" onclick={() => (statusTarget = null)}>Cancel</button>
              <button type="submit" class="btn btn-sm btn-primary" disabled={statusLoading || !statusForm.status}>{statusLoading ? 'Saving…' : 'Update Status'}</button>
            </div>
          {/if}
        </form>
      </div>
    </div>
  </div>
{/if}

<!-- ── Confirmed Manual Blocking Modal (non-dismissible) ── -->
{#if showConfirmedManualModal}
  <div class="blocking-backdrop" role="alertdialog" aria-modal="true" aria-labelledby="cm-modal-title">
    <div class="blocking-modal">
      <h3 id="cm-modal-title" class="blocking-title">Manual Identification Confirmed</h3>
      <p class="blocking-body">
        This strain has been marked as Confirmed — Manual. Manual confirmation is based on professional judgment,
        not genomic verification. It must NOT be cited as equivalent to genomic confirmation in regulatory submissions,
        IP claims, or research publications without explicit disclosure. The basis for this confirmation has been
        recorded in the audit log.
      </p>
      <button
        class="btn btn-primary blocking-ack"
        onclick={() => { showConfirmedManualModal = false; addNotification('Status updated to Confirmed — Manual', 'success'); }}
      >I Acknowledge</button>
    </div>
  </div>
{/if}

<!-- ── Hybrid Wizard ── -->
{#if showHybridWizard}
  <HybridWizard
    {speciesId}
    {speciesName}
    onclose={() => (showHybridWizard = false)}
    oncreated={async () => { showHybridWizard = false; await load(); addNotification('Hybrid strain created', 'success'); }}
  />
{/if}

<!-- ── Strain Detail Slide-over ── -->
{#if detailStrainId}
  <StrainDetail strainId={detailStrainId} onclose={() => (detailStrainId = null)} />
{/if}

<style>
  .strain-manager { display: flex; flex-direction: column; gap: 16px; }
  .sm-header { display: flex; justify-content: space-between; align-items: flex-start; gap: 12px; flex-wrap: wrap; }
  .sm-header h2 { font-size: 18px; font-weight: 700; margin-bottom: 2px; }
  .sm-sub { font-size: 12px; color: #6b7280; }
  .sm-actions { display: flex; gap: 8px; flex-wrap: wrap; }
  .sm-toolbar { display: flex; gap: 8px; flex-wrap: wrap; }
  .sm-search { flex: 1; min-width: 200px; }
  .sm-filter { width: 200px; }
  .sm-table-wrap { overflow-x: auto; }

  .status-badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 12px;
    font-size: 11px;
    font-weight: 600;
  }
  .status-unverified { background: #f1f5f9; color: #475569; }
  .status-claimed { background: #dbeafe; color: #1e40af; }
  .status-confirmed_manual { background: #fef3c7; color: #92400e; }
  .status-confirmed_genomic { background: #dcfce7; color: #166534; }

  .nudge-link {
    background: none;
    border: none;
    color: #2563eb;
    font-size: 11px;
    cursor: pointer;
    padding: 0 0 0 6px;
    text-decoration: underline;
  }
  .nudge-link:hover { color: #1d4ed8; }

  .pulse-dot {
    display: inline-block;
    width: 8px;
    height: 8px;
    background: #d97706;
    border-radius: 50%;
    margin-left: 6px;
    animation: pulse 1.5s ease-in-out infinite;
    vertical-align: middle;
  }
  @keyframes pulse {
    0%, 100% { opacity: 1; transform: scale(1); }
    50% { opacity: 0.5; transform: scale(1.3); }
  }

  .strain-name-link {
    cursor: pointer;
    color: #2563eb;
    font-weight: 500;
    text-decoration: none;
  }
  .strain-name-link:hover { text-decoration: underline; }

  .hybrid-chip {
    display: inline-block;
    font-size: 10px;
    background: #ede9fe;
    color: #6d28d9;
    border-radius: 10px;
    padding: 1px 6px;
    margin-left: 4px;
  }

  .cross-species-chip {
    display: inline-block;
    font-size: 10px;
    background: #fef2f2;
    color: #dc2626;
    border-radius: 10px;
    padding: 1px 6px;
    margin-left: 4px;
    font-weight: 600;
  }

  .strain-code {
    font-family: monospace;
    font-size: 12px;
    background: #f1f5f9;
    padding: 1px 4px;
    border-radius: 3px;
  }

  .row-actions { display: flex; gap: 4px; }

  tr.archived { opacity: 0.5; }

  /* Modals */
  .modal-backdrop {
    position: fixed;
    inset: 0;
    z-index: 1000;
    background: rgba(0,0,0,0.4);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 20px;
  }
  .modal-box {
    background: white;
    border-radius: 10px;
    padding: 24px;
    width: 100%;
    max-width: 480px;
    max-height: 90vh;
    overflow-y: auto;
    box-shadow: 0 20px 60px rgba(0,0,0,0.3);
  }
  .modal-sm { max-width: 380px; }
  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 20px;
  }
  .modal-header h3 { font-size: 16px; font-weight: 700; }
  .modal-close {
    background: none;
    border: none;
    font-size: 16px;
    cursor: pointer;
    color: #6b7280;
    padding: 2px 6px;
    border-radius: 4px;
  }
  .modal-close:hover { background: #f3f4f6; }
  .modal-footer { display: flex; gap: 8px; justify-content: flex-end; margin-top: 20px; }

  .current-status-row { display: flex; align-items: center; gap: 8px; margin-bottom: 16px; }
  .info-row { font-size: 13px; color: #6b7280; font-style: italic; margin: 12px 0; }

  .warning-box {
    background: #fef3c7;
    border: 1px solid #fde68a;
    border-radius: 6px;
    padding: 12px;
    margin-bottom: 16px;
    font-size: 12px;
    color: #92400e;
  }
  .warning-box strong { display: block; margin-bottom: 4px; }

  /* Blocking modal — non-dismissible */
  .blocking-backdrop {
    position: fixed;
    inset: 0;
    z-index: 9999;
    background: rgba(0,0,0,0.65);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 20px;
  }
  .blocking-modal {
    background: white;
    border-radius: 12px;
    padding: 32px;
    max-width: 480px;
    width: 100%;
    box-shadow: 0 24px 64px rgba(0,0,0,0.4);
    border-top: 4px solid #d97706;
  }
  .blocking-title {
    font-size: 18px;
    font-weight: 700;
    margin-bottom: 16px;
    color: #92400e;
  }
  .blocking-body {
    font-size: 14px;
    line-height: 1.7;
    color: #374151;
    margin-bottom: 24px;
  }
  .blocking-ack {
    width: 100%;
    padding: 12px;
    font-size: 15px;
    font-weight: 600;
  }

  :global(.dark) .modal-box { background: #1e293b; color: #e2e8f0; }
  :global(.dark) .modal-close { color: #94a3b8; }
  :global(.dark) .modal-close:hover { background: #334155; }
  :global(.dark) .blocking-modal { background: #1e293b; color: #e2e8f0; }
  :global(.dark) .blocking-body { color: #cbd5e1; }
  :global(.dark) .strain-code { background: #334155; color: #e2e8f0; }
  :global(.dark) .status-unverified { background: #334155; color: #94a3b8; }
  :global(.dark) .status-confirmed_manual { background: #78350f; color: #fde68a; }
  :global(.dark) .status-confirmed_genomic { background: #166534; color: #dcfce7; }
  :global(.dark) .status-claimed { background: #1e40af; color: #dbeafe; }
  :global(.dark) .warning-box { background: #78350f; border-color: #92400e; color: #fde68a; }
</style>
