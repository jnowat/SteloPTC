<script lang="ts">
  import { onMount } from 'svelte';
  import {
    listLocations, createLocation, updateLocation, deleteLocation,
    setSpecimenLocationPin, getLocationMapData, listSpecimens,
    type Location,
  } from '../api';
  import { addNotification } from '../stores/app';
  import { currentUser } from '../stores/auth';
  import DataState from './DataState.svelte';

  type MapPoint = {
    location_id: string; name: string; floor_plan_x: number | null; floor_plan_y: number | null;
    specimen_count: number; contaminated_count: number; avg_age_days: number | null;
  };

  let locations = $state<Location[]>([]);
  let mapData = $state<MapPoint[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  // Floor selection — each Location that has its own floor_plan_image is treated
  // as its own map/room (see design note at bottom of file / final report).
  let selectedLocationId = $state<string | null>(null);

  // Heat-map mode
  type HeatMode = 'density' | 'contamination' | 'age';
  let heatMode = $state<HeatMode>('density');

  // Selected pin (for the info popover)
  let activePinId = $state<string | null>(null);

  // New/edit location form
  let showForm = $state(false);
  let editingId = $state<string | null>(null);
  let form = $state({ name: '', description: '', floor_plan_x: '', floor_plan_y: '' });
  let formImageBase64 = $state<string | null>(null);
  let formImagePreviewUrl = $state<string | null>(null);
  let saving = $state(false);

  // Specimen-assignment panel (per selected location)
  let showAssignPanel = $state<string | null>(null); // location id
  let allSpecimens = $state<any[]>([]);
  let specimensLoaded = $state(false);
  let assignSpecimenId = $state('');

  const canWrite = $derived($currentUser?.role !== 'guest');
  const canManage = $derived($currentUser?.role === 'admin' || $currentUser?.role === 'supervisor');

  // Locations that have their own floor plan image — each is rendered as its
  // own mini floor-plan/map section.
  const locationsWithMap = $derived(locations.filter((l) => !!l.floor_plan_image));
  // Locations with no floor plan image yet — listed in a plain sidebar (no
  // pin can be rendered without an image to place it on).
  const locationsWithoutMap = $derived(locations.filter((l) => !l.floor_plan_image));

  const selectedLocation = $derived(
    locationsWithMap.find((l) => l.id === selectedLocationId) || locationsWithMap[0] || null
  );

  const selectedMapPoint = $derived(
    selectedLocation ? mapData.find((m) => m.location_id === selectedLocation.id) || null : null
  );

  const activePoint = $derived(mapData.find((m) => m.location_id === activePinId) || null);
  const activeLocation = $derived(locations.find((l) => l.id === activePinId) || null);

  onMount(() => {
    load();
  });

  async function load() {
    loading = true;
    error = null;
    try {
      const [locs, md] = await Promise.all([listLocations(), getLocationMapData()]);
      locations = locs;
      mapData = md;
      if (!selectedLocationId && locs.some((l) => l.floor_plan_image)) {
        selectedLocationId = locs.find((l) => l.floor_plan_image)!.id;
      }
    } catch (e: any) {
      error = e.message;
      addNotification(e.message, 'error');
    } finally {
      loading = false;
    }
  }

  function imageSrc(base64: string): string {
    // Sniff JPEG magic bytes (base64 "/9j/") — otherwise default to PNG.
    // Browsers tolerate a mismatched-but-compatible mime for common formats.
    if (base64.startsWith('/9j/')) return `data:image/jpeg;base64,${base64}`;
    if (base64.startsWith('iVBOR')) return `data:image/png;base64,${base64}`;
    if (base64.startsWith('R0lGOD')) return `data:image/gif;base64,${base64}`;
    if (base64.startsWith('UklGR')) return `data:image/webp;base64,${base64}`;
    return `data:image/png;base64,${base64}`;
  }

  // ── Heat-map color/size computation ──────────────────────────────────────

  function valueFor(point: MapPoint, mode: HeatMode): number {
    if (mode === 'density') return point.specimen_count;
    if (mode === 'contamination') return point.specimen_count > 0 ? point.contaminated_count / point.specimen_count : 0;
    return point.avg_age_days ?? 0;
  }

  const modeRange = $derived.by(() => {
    const values = mapData.map((p) => valueFor(p, heatMode));
    const min = values.length ? Math.min(...values) : 0;
    const max = values.length ? Math.max(...values) : 0;
    return { min, max };
  });

  function normalized(point: MapPoint): number {
    const { min, max } = modeRange;
    if (max === min) return point.specimen_count > 0 || heatMode !== 'density' ? 0.5 : 0;
    return Math.max(0, Math.min(1, (valueFor(point, heatMode) - min) / (max - min)));
  }

  // Interpolate green (#059669) -> yellow (#f59e0b) -> red (#dc2626) across 0..1.
  function heatColor(t: number): string {
    const stops = [
      { p: 0, c: [5, 150, 105] },
      { p: 0.5, c: [245, 158, 11] },
      { p: 1, c: [220, 38, 38] },
    ];
    let a = stops[0], b = stops[stops.length - 1];
    for (let i = 0; i < stops.length - 1; i++) {
      if (t >= stops[i].p && t <= stops[i + 1].p) { a = stops[i]; b = stops[i + 1]; break; }
    }
    const span = b.p - a.p || 1;
    const localT = (t - a.p) / span;
    const r = Math.round(a.c[0] + (b.c[0] - a.c[0]) * localT);
    const g = Math.round(a.c[1] + (b.c[1] - a.c[1]) * localT);
    const bl = Math.round(a.c[2] + (b.c[2] - a.c[2]) * localT);
    return `rgb(${r}, ${g}, ${bl})`;
  }

  function pinStyle(point: MapPoint): string {
    const t = normalized(point);
    const size = 16 + t * 18; // 16px..34px
    const color = heatColor(t);
    const x = (point.floor_plan_x ?? 0) * 100;
    const y = (point.floor_plan_y ?? 0) * 100;
    return `left:${x}%; top:${y}%; width:${size}px; height:${size}px; background:${color}; margin-left:-${size / 2}px; margin-top:-${size / 2}px;`;
  }

  function heatLegendLabel(): string {
    if (heatMode === 'density') return 'Specimen count';
    if (heatMode === 'contamination') return 'Contamination ratio';
    return 'Average age (days)';
  }

  function pointSummary(point: MapPoint | null): string {
    if (!point) return '';
    const parts = [`${point.specimen_count} specimen${point.specimen_count !== 1 ? 's' : ''}`];
    parts.push(`${point.contaminated_count} contaminated`);
    if (point.avg_age_days != null) parts.push(`avg age ${point.avg_age_days.toFixed(0)}d`);
    return parts.join(' · ');
  }

  function togglePin(locationId: string) {
    activePinId = activePinId === locationId ? null : locationId;
    showAssignPanel = null;
  }

  // ── New / edit location form ─────────────────────────────────────────────

  function resetForm() {
    form = { name: '', description: '', floor_plan_x: '', floor_plan_y: '' };
    formImageBase64 = null;
    formImagePreviewUrl = null;
    editingId = null;
  }

  function openNewForm() {
    resetForm();
    showForm = true;
  }

  function startEdit(loc: Location) {
    form = {
      name: loc.name,
      description: loc.description || '',
      floor_plan_x: loc.floor_plan_x != null ? String(loc.floor_plan_x) : '',
      floor_plan_y: loc.floor_plan_y != null ? String(loc.floor_plan_y) : '',
    };
    formImageBase64 = loc.floor_plan_image;
    formImagePreviewUrl = loc.floor_plan_image ? imageSrc(loc.floor_plan_image) : null;
    editingId = loc.id;
    showForm = true;
    activePinId = null;
  }

  function handleFileChange(e: Event) {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => {
      const result = reader.result as string;
      // Strip the "data:image/...;base64," prefix — API stores raw base64.
      const base64 = result.substring(result.indexOf(',') + 1);
      formImageBase64 = base64;
      formImagePreviewUrl = result;
    };
    reader.onerror = () => addNotification('Failed to read image file', 'error');
    reader.readAsDataURL(file);
  }

  function handlePreviewClick(e: MouseEvent) {
    const target = e.currentTarget as HTMLElement;
    const rect = target.getBoundingClientRect();
    const x = (e.clientX - rect.left) / rect.width;
    const y = (e.clientY - rect.top) / rect.height;
    form.floor_plan_x = x.toFixed(4);
    form.floor_plan_y = y.toFixed(4);
  }

  async function handleSubmit(e: Event) {
    e.preventDefault();
    if (!form.name.trim()) {
      addNotification('Location name is required', 'warning');
      return;
    }
    saving = true;
    try {
      const payload = {
        name: form.name.trim(),
        description: form.description.trim() || undefined,
        floor_plan_image: formImageBase64 || undefined,
        floor_plan_x: form.floor_plan_x !== '' ? parseFloat(form.floor_plan_x) : undefined,
        floor_plan_y: form.floor_plan_y !== '' ? parseFloat(form.floor_plan_y) : undefined,
      };
      if (editingId) {
        await updateLocation({ id: editingId, ...payload });
        addNotification('Location updated', 'success');
      } else {
        await createLocation(payload);
        addNotification('Location created', 'success');
      }
      showForm = false;
      resetForm();
      await load();
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      saving = false;
    }
  }

  async function handleDelete(loc: Location) {
    if (!confirm(`Delete location "${loc.name}"?`)) return;
    try {
      await deleteLocation(loc.id);
      addNotification('Location deleted', 'success');
      if (selectedLocationId === loc.id) selectedLocationId = null;
      if (activePinId === loc.id) activePinId = null;
      await load();
    } catch (e: any) {
      // Surfaces backend message, e.g. "Cannot delete: N specimen(s) are still pinned..."
      addNotification(e.message, 'error');
    }
  }

  // ── Specimen assignment (pin a specimen to a location) ───────────────────

  async function openAssignPanel(locationId: string) {
    showAssignPanel = showAssignPanel === locationId ? null : locationId;
    assignSpecimenId = '';
    if (showAssignPanel && !specimensLoaded) {
      try {
        const result = await listSpecimens(1, 500);
        allSpecimens = result.items;
        specimensLoaded = true;
      } catch (e: any) {
        addNotification(e.message, 'error');
      }
    }
  }

  async function handleAssignSpecimen(locationId: string) {
    if (!assignSpecimenId) return;
    try {
      await setSpecimenLocationPin(assignSpecimenId, locationId);
      addNotification('Specimen pinned to location', 'success');
      assignSpecimenId = '';
      await load();
    } catch (e: any) {
      addNotification(e.message, 'error');
    }
  }

  async function handleUnpinSpecimen(specimenId: string) {
    try {
      await setSpecimenLocationPin(specimenId, null);
      addNotification('Specimen unpinned', 'success');
      await load();
    } catch (e: any) {
      addNotification(e.message, 'error');
    }
  }
</script>

<div class="lab-map">
  <div class="page-header">
    <h1>Interactive Lab Map</h1>
    {#if canWrite}
      <button
        class="btn btn-primary"
        title={showForm ? 'Cancel and close the new location form without saving' : 'Open the form to add a new location'}
        onclick={() => { if (showForm) { showForm = false; resetForm(); } else { openNewForm(); } }}
      >
        {showForm ? 'Cancel' : '+ New Location'}
      </button>
    {/if}
  </div>

  {#if showForm}
    <div class="card" style="margin-bottom:16px;">
      <form onsubmit={handleSubmit}>
        <h3 style="margin-bottom:16px;">{editingId ? 'Edit Location' : 'New Location'}</h3>
        <div class="form-row">
          <div class="form-group">
            <label for="labmap-location-name" title="The name of this room, bench, or physical location">Name *</label>
            <input id="labmap-location-name" type="text" title="Enter the location name, e.g. Growth Room B" bind:value={form.name} required placeholder="e.g., Growth Room B" />
          </div>
          <div class="form-group">
            <label for="labmap-location-description" title="Optional description of this location">Description</label>
            <input id="labmap-location-description" type="text" title="Enter an optional description" bind:value={form.description} placeholder="e.g., West wing, shelf unit 3" />
          </div>
        </div>

        <div class="form-group">
          <label for="labmap-floor-plan-image" title="Upload a floor plan or room photo, then click on it to drop this location's map pin">Floor Plan Image</label>
          <input id="labmap-floor-plan-image" type="file" accept="image/*" title="Choose an image file for this location's floor plan" onchange={handleFileChange} />
        </div>

        {#if formImagePreviewUrl}
          <div class="form-group">
            <span class="group-label" title="Click anywhere on the image to set this location's pin position">Click to place pin</span>
            <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
            <div class="preview-wrap" style="position:relative; display:inline-block; max-width:100%;">
              <img
                src={formImagePreviewUrl}
                alt="Floor plan preview"
                class="preview-img"
                onclick={handlePreviewClick}
                title="Click on the image to set the pin's x/y position"
              />
              {#if form.floor_plan_x !== '' && form.floor_plan_y !== ''}
                <div
                  class="preview-pin"
                  style="left:{parseFloat(form.floor_plan_x) * 100}%; top:{parseFloat(form.floor_plan_y) * 100}%;"
                  title="Pin position: {form.floor_plan_x}, {form.floor_plan_y}"
                ></div>
              {/if}
            </div>
          </div>
        {/if}

        <div class="form-row">
          <div class="form-group">
            <label for="labmap-pin-x" title="Fractional horizontal position of the pin on the floor plan image, 0.0 (left) to 1.0 (right)">Pin X (0.0–1.0)</label>
            <input id="labmap-pin-x" type="number" min="0" max="1" step="0.0001" title="Horizontal fraction across the image where the pin sits" bind:value={form.floor_plan_x} placeholder="e.g., 0.42" />
          </div>
          <div class="form-group">
            <label for="labmap-pin-y" title="Fractional vertical position of the pin on the floor plan image, 0.0 (top) to 1.0 (bottom)">Pin Y (0.0–1.0)</label>
            <input id="labmap-pin-y" type="number" min="0" max="1" step="0.0001" title="Vertical fraction down the image where the pin sits" bind:value={form.floor_plan_y} placeholder="e.g., 0.66" />
          </div>
        </div>

        <div style="text-align:right;">
          <button type="submit" class="btn btn-primary" disabled={saving} title={editingId ? 'Save changes to this location' : 'Create this new location'}>
            {saving ? 'Saving…' : (editingId ? 'Update Location' : 'Create Location')}
          </button>
        </div>
      </form>
    </div>
  {/if}

  <DataState
    {loading}
    {error}
    empty={!loading && !error && locations.length === 0}
    emptyIcon="🗺️"
    emptyTitle="No locations yet"
    emptyMessage="Add your first location to start mapping the lab."
    emptyActionLabel={canWrite ? '+ New Location' : ''}
    onemptyaction={canWrite ? openNewForm : undefined}
    onretry={load}
  >
    <div class="map-layout">
      <div class="map-main card">
        {#if locationsWithMap.length === 0}
          <p class="empty-state">No locations have a floor plan image yet. Add one via "+ New Location" to see it on the map.</p>
        {:else}
          <div class="map-controls">
            <div class="form-group" style="margin-bottom:0;">
              <label for="labmap-floor-select" title="Switch between floor plan images — each location with its own uploaded image is treated as its own map">Floor</label>
              <select
                id="labmap-floor-select"
                title="Select which location's floor plan to view"
                value={selectedLocation?.id}
                onchange={(e) => selectedLocationId = (e.target as HTMLSelectElement).value}
              >
                {#each locationsWithMap as loc}
                  <option value={loc.id}>{loc.name}</option>
                {/each}
              </select>
            </div>
            <div class="form-group" style="margin-bottom:0;">
              <label for="labmap-heat-mode" title="Choose what the pin color and size represent">Heat-map Mode</label>
              <select id="labmap-heat-mode" title="Select the heat-map mode: density, contamination risk, or age" bind:value={heatMode}>
                <option value="density">Density (specimen count)</option>
                <option value="contamination">Contamination Risk</option>
                <option value="age">Age (avg days)</option>
              </select>
            </div>
          </div>

          {#if selectedLocation}
            <div class="floor-plan-wrap">
              <img
                src={imageSrc(selectedLocation.floor_plan_image!)}
                alt="Floor plan for {selectedLocation.name}"
                class="floor-plan-img"
              />
              {#each mapData.filter((m) => m.location_id === selectedLocation!.id) as point}
                <button
                  type="button"
                  class="map-pin"
                  style={pinStyle(point)}
                  aria-label="View details for {point.name}"
                  title="{point.name}: {pointSummary(point)}"
                  onclick={() => togglePin(point.location_id)}
                ></button>
              {/each}
            </div>

            <div class="heat-legend" title="Color/size scale for the selected heat-map mode: {heatLegendLabel()}">
              <span class="legend-label">{heatLegendLabel()}:</span>
              <span class="legend-swatch" style="background:{heatColor(0)};"></span> Low
              <span class="legend-swatch" style="background:{heatColor(0.5)};"></span> Mid
              <span class="legend-swatch" style="background:{heatColor(1)};"></span> High
            </div>

            {#if selectedMapPoint}
              <div class="pin-summary-row">
                <strong>{selectedLocation.name}</strong>
                <span class="pin-summary-meta">{pointSummary(selectedMapPoint)}</span>
              </div>
            {/if}
          {/if}
        {/if}
      </div>

      <div class="map-sidebar">
        {#if activeLocation && activePoint}
          <div class="card pin-popover">
            <div class="pin-popover-header">
              <h3 style="margin:0;">{activeLocation.name}</h3>
              <button class="btn btn-sm" aria-label="Close location details" title="Close this panel" onclick={() => activePinId = null}>✕</button>
            </div>
            {#if activeLocation.description}
              <p class="pin-popover-desc">{activeLocation.description}</p>
            {/if}
            <div class="pin-stats">
              <div class="pin-stat"><span class="pin-stat-value">{activePoint.specimen_count}</span><span class="pin-stat-label">Specimens</span></div>
              <div class="pin-stat"><span class="pin-stat-value" class:contam-high={activePoint.contaminated_count > 0}>{activePoint.contaminated_count}</span><span class="pin-stat-label">Contaminated</span></div>
              <div class="pin-stat"><span class="pin-stat-value">{activePoint.avg_age_days != null ? activePoint.avg_age_days.toFixed(0) : '—'}</span><span class="pin-stat-label">Avg Age (d)</span></div>
            </div>
            <div class="pin-actions">
              {#if canWrite}
                <button class="btn btn-sm" title="Edit this location's name, description, image, or pin position" onclick={() => startEdit(activeLocation!)}>Edit</button>
                <button class="btn btn-sm" title="Assign or unpin specimens for this location" onclick={() => openAssignPanel(activeLocation!.id)}>
                  {showAssignPanel === activeLocation.id ? 'Hide Specimens' : 'Manage Specimens'}
                </button>
              {/if}
              {#if canManage}
                <button class="btn btn-sm btn-danger" title="Delete this location — blocked if specimens are still pinned here" onclick={() => handleDelete(activeLocation!)}>Delete</button>
              {/if}
            </div>

            {#if showAssignPanel === activeLocation.id}
              <div class="assign-panel">
                <div class="form-group" style="margin-bottom:8px;">
                  <label for="labmap-assign-specimen" title="Pin a specimen to this location">Pin a specimen</label>
                  <div style="display:flex; gap:6px;">
                    <select id="labmap-assign-specimen" title="Select a specimen to pin to this location" bind:value={assignSpecimenId} style="flex:1;">
                      <option value="">— Select specimen —</option>
                      {#each allSpecimens as s}
                        <option value={s.id}>{s.accession_number}{s.species_code ? ` (${s.species_code})` : ''}</option>
                      {/each}
                    </select>
                    <button class="btn btn-sm" title="Pin the selected specimen to this location" onclick={() => handleAssignSpecimen(activeLocation!.id)} disabled={!assignSpecimenId}>Pin</button>
                  </div>
                </div>
                <p class="assign-hint">To unpin a specimen from this location, select it above once pinned elsewhere, or use the specimen's own record.</p>
              </div>
            {/if}
          </div>
        {:else}
          <div class="card pin-popover-placeholder">
            <p class="empty-state">Click a pin on the map to view location details.</p>
          </div>
        {/if}

        {#if locationsWithoutMap.length > 0}
          <div class="card" style="margin-top:16px;">
            <h3 title="Locations that don't have a floor plan image uploaded yet — no pin can be shown until one is added">Locations Without a Floor Plan</h3>
            <div class="plain-location-list">
              {#each locationsWithoutMap as loc}
                {@const point = mapData.find((m) => m.location_id === loc.id)}
                <div class="plain-location-item">
                  <div>
                    <div class="plain-location-name">{loc.name}</div>
                    {#if point}
                      <div class="plain-location-meta">{pointSummary(point)}</div>
                    {/if}
                  </div>
                  <div style="display:flex; gap:4px;">
                    {#if canWrite}
                      <button class="btn btn-sm" title="Edit this location and optionally upload a floor plan image" onclick={() => startEdit(loc)}>Edit</button>
                    {/if}
                    {#if canManage}
                      <button class="btn btn-sm btn-danger" title="Delete this location — blocked if specimens are still pinned here" onclick={() => handleDelete(loc)}>Delete</button>
                    {/if}
                  </div>
                </div>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    </div>
  </DataState>
</div>

<style>
  /* Matches global :global(label) styling for instructional text that isn't
     bound to a single form control (e.g. "click the image" instructions). */
  .group-label {
    display: block;
    font-size: 12px;
    font-weight: 600;
    color: #6b7280;
    margin-bottom: 4px;
  }
  :global(.dark) .group-label { color: #94a3b8; }

  .map-layout {
    display: grid;
    grid-template-columns: 2fr 1fr;
    gap: var(--space-5);
    align-items: start;
  }
  @media (max-width: 900px) {
    .map-layout { grid-template-columns: 1fr; }
  }

  .map-controls {
    display: flex;
    gap: var(--space-4);
    flex-wrap: wrap;
    margin-bottom: var(--space-4);
  }
  .map-controls .form-group { min-width: 200px; }

  .floor-plan-wrap {
    position: relative;
    width: 100%;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    overflow: hidden;
    background: var(--color-surface-raised);
  }
  .floor-plan-img {
    display: block;
    width: 100%;
    height: auto;
  }
  .map-pin {
    position: absolute;
    border-radius: 50%;
    border: 2px solid rgba(255, 255, 255, 0.9);
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.4);
    cursor: pointer;
    padding: 0;
    transition: transform var(--transition-fast);
  }
  .map-pin:hover {
    transform: scale(1.15);
  }

  .heat-legend {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: var(--font-size-xs);
    color: var(--color-text-muted);
    margin-top: var(--space-3);
  }
  .legend-label { font-weight: 600; margin-right: 4px; }
  .legend-swatch {
    display: inline-block;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    margin-right: 2px;
    border: 1px solid rgba(0,0,0,0.15);
  }

  .pin-summary-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-top: var(--space-3);
    padding-top: var(--space-3);
    border-top: 1px solid var(--color-border);
    font-size: var(--font-size-sm);
  }
  .pin-summary-meta { color: var(--color-text-muted); }

  .preview-wrap { max-width: 100%; }
  .preview-img {
    max-width: 100%;
    max-height: 320px;
    display: block;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    cursor: crosshair;
  }
  .preview-pin {
    position: absolute;
    width: 16px;
    height: 16px;
    margin-left: -8px;
    margin-top: -8px;
    border-radius: 50%;
    background: var(--color-accent);
    border: 2px solid white;
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.4);
    pointer-events: none;
  }

  .pin-popover-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: var(--space-2);
  }
  .pin-popover-desc {
    font-size: var(--font-size-sm);
    color: var(--color-text-muted);
    margin-bottom: var(--space-3);
  }
  .pin-stats {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: var(--space-2);
    margin-bottom: var(--space-3);
  }
  .pin-stat {
    text-align: center;
    background: var(--color-surface-raised);
    border-radius: var(--radius-md);
    padding: var(--space-2);
  }
  .pin-stat-value { display: block; font-size: var(--font-size-xl); font-weight: 800; }
  .pin-stat-value.contam-high { color: var(--color-contam-high); }
  .pin-stat-label { display: block; font-size: var(--font-size-xs); color: var(--color-text-muted); text-transform: uppercase; letter-spacing: 0.5px; }
  .pin-actions { display: flex; gap: 6px; flex-wrap: wrap; }
  .pin-popover-placeholder { display: flex; align-items: center; justify-content: center; min-height: 120px; }

  .assign-panel {
    margin-top: var(--space-3);
    padding-top: var(--space-3);
    border-top: 1px solid var(--color-border);
  }
  .assign-hint { font-size: var(--font-size-xs); color: var(--color-text-faint); margin-top: 6px; }

  .plain-location-list { display: flex; flex-direction: column; gap: var(--space-2); }
  .plain-location-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--space-2) 10px;
    border-radius: var(--radius-md);
    background: var(--color-surface-raised);
    gap: var(--space-2);
  }
  .plain-location-name { font-weight: 600; font-size: var(--font-size-base); }
  .plain-location-meta { font-size: var(--font-size-xs); color: var(--color-text-muted); }
</style>
