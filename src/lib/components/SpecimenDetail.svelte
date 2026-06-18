<script lang="ts">
  import { untrack } from 'svelte';
  import { get } from 'svelte/store';
  import { getSpecimen, listSubcultures, createSubculture, splitSpecimen, previewSplitAccessions, createDraftMediaBatch, getSpecimenFamily, listMedia, listComplianceRecords, listAttachments } from '../api';
  import SpecimenPhotoGallery from './SpecimenPhotoGallery.svelte';
  import SpecimenComplianceTable from './SpecimenComplianceTable.svelte';
  import SpecimenPassageTimeline from './SpecimenPassageTimeline.svelte';
  import { selectedSpecimenId, navigateTo, addNotification, devMode } from '../stores/app';
  import { currentUser } from '../stores/auth';
  import { escHtml, stageFmt, healthLabel } from '../utils';
  import { deliverPrint } from '../printUtils';
  import QrModal from './QrModal.svelte';
  import QrScanner from './QrScanner.svelte';
  import Tooltip from './Tooltip.svelte';

  let specimen = $state<any>(null);
  let showQrModal = $state(false);
  let showQrScanner = $state(false);
  let subcultures = $state<any[]>([]);
  let mediaBatches = $state<any[]>([]);
  let complianceRecords = $state<any[]>([]);
  let parentSpecimen = $state<any>(null);
  let childSpecimens = $state<any[]>([]);
  let familyMembers = $state<any[]>([]);
  let loading = $state(true);
  let showPassageForm = $state(false);
  let activeTab = $state<'history' | 'compliance' | 'photos'>('history');
  let photos = $state<any[]>([]);
  let isSplitting = $state(false);
  let splitCount = $state(2);
  let submitting = $state(false);
  let showSplitConfirm = $state(false);
  let showDraftMediaDialog = $state(false);
  let draftMediaForChild = $state(-1);
  let draftMediaName = $state('');
  let draftMediaSubmitting = $state(false);

  const stageOptions = [
    { value: 'explant', label: 'Explant' },
    { value: 'callus', label: 'Callus' },
    { value: 'suspension', label: 'Suspension' },
    { value: 'protoplast', label: 'Protoplast' },
    { value: 'shoot', label: 'Shoot' },
    { value: 'root', label: 'Root' },
    { value: 'embryogenic', label: 'Embryogenic' },
    { value: 'plantlet', label: 'Plantlet' },
    { value: 'acclimatized', label: 'Acclimatized' },
    { value: 'stock', label: 'Stock' },
    { value: 'custom', label: 'Custom' },
  ];

  // Per-child configuration array for split mode
  function makeChild() {
    const parentParts = (specimen?.location || '').split(' / ');
    const lastSub = subcultures[0]; // subcultures is newest-first
    return {
      accessionNumber: '',
      stage: specimen?.stage || '',
      health_value: 4,
      health_unknown: false,
      locRoom: parentParts[0] || localStorage.getItem('sc_lastRoom') || '',
      locRack: parentParts[1] || localStorage.getItem('sc_lastRack') || '',
      locShelf: parentParts[2] || localStorage.getItem('sc_lastShelf') || '',
      locTray: parentParts[3] || localStorage.getItem('sc_lastTray') || '',
      media_batch_id: lastSub?.media_batch_id || '',
      vessel_type: lastSub?.vessel_type || '',
      custom_vessel: false,
      vessel_input: '',
      notes: '',
      reminder_enabled: true,
      reminder_days: 7,
    };
  }
  let splitChildren = $state([makeChild(), makeChild()]);

  // Keep splitChildren in sync with splitCount
  $effect(() => {
    const n = splitCount;
    if (splitChildren.length < n) {
      splitChildren = [...splitChildren, ...Array.from({ length: n - splitChildren.length }, makeChild)];
    } else if (splitChildren.length > n) {
      splitChildren = splitChildren.slice(0, n);
    }
  });

  // Load preview accession numbers when splitting starts or count changes
  $effect(() => {
    if (isSplitting && $selectedSpecimenId) {
      const _count = splitCount; // explicit dependency
      void loadSplitPreview($selectedSpecimenId, splitCount);
    }
  });

  async function loadSplitPreview(parentId: string, count: number) {
    try {
      const accessions = await previewSplitAccessions(parentId, count);
      for (let i = 0; i < splitChildren.length && i < accessions.length; i++) {
        if (!splitChildren[i].accessionNumber) {
          splitChildren[i].accessionNumber = accessions[i];
        }
      }
    } catch {
      // non-fatal: user can manually enter accession numbers
    }
  }

  async function openDraftMediaDialog(childIdx: number) {
    draftMediaForChild = childIdx;
    draftMediaName = '';
    showDraftMediaDialog = true;
  }

  async function createDraftMedia() {
    if (!draftMediaName.trim()) return;
    draftMediaSubmitting = true;
    try {
      const newBatch = await createDraftMediaBatch(draftMediaName.trim());
      mediaBatches = [newBatch, ...mediaBatches];
      if (draftMediaForChild >= 0 && draftMediaForChild < splitChildren.length) {
        splitChildren[draftMediaForChild].media_batch_id = newBatch.id;
      }
      showDraftMediaDialog = false;
      draftMediaName = '';
      addNotification(`Draft media batch "${newBatch.name}" created. Complete it in Media Management.`, 'success');
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      draftMediaSubmitting = false;
    }
  }

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
    contamination_flag: false,
    contamination_notes: '',
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

  $effect(() => {
    if ($selectedSpecimenId) untrack(() => loadAll($selectedSpecimenId));
  });

  async function loadPhotos(id: string) {
    try {
      photos = await listAttachments('specimen', id);
    } catch {
      // non-fatal
    }
  }

  async function loadAll(id: string) {
    loading = true;
    try {
      const [s, sc, cr, mb, ph] = await Promise.all([
        getSpecimen(id),
        listSubcultures(id),
        listComplianceRecords(id),
        listMedia(),
        listAttachments('specimen', id).catch(() => []),
      ]);
      specimen = s;
      subcultures = [...sc].reverse(); // newest first
      complianceRecords = cr;
      mediaBatches = mb;
      photos = ph as any[];

      // Lineage: fetch parent if present
      if (s.parent_specimen_id) {
        parentSpecimen = await getSpecimen(s.parent_specimen_id).catch(() => null);
      } else {
        parentSpecimen = null;
      }

      // Family tree: all specimens sharing the same root
      const family = await getSpecimenFamily(id).catch(() => []);
      familyMembers = family;
      childSpecimens = family.filter((m: any) => m.parent_specimen_id === id && !m.is_archived);
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      loading = false;
    }
  }

  function resetPassageForm() {
    showPassageForm = false;
    isSplitting = false;
    showSplitConfirm = false;
    splitCount = 2;
    splitChildren = [makeChild(), makeChild()];
    passageHealthValue = 4;
    subcultureForm = {
      date: new Date().toISOString().split('T')[0],
      media_batch_id: '', vessel_type: '', temperature_c: '',
      ph: '', light_cycle: '', notes: '', observations: '',
      health_status: '', health_unknown: false, employee_id: '',
      contamination_flag: false, contamination_notes: '',
    };
  }

  async function executeSplit() {
    if (!$selectedSpecimenId || !specimen) return;
    showSplitConfirm = false;
    submitting = true;
    try {
      const result = await splitSpecimen({
        parent_specimen_id: $selectedSpecimenId,
        date: subcultureForm.date,
        children: splitChildren.slice(0, splitCount).map(c => ({
          accession_number: c.accessionNumber || undefined,
          location: [c.locRoom, c.locRack, c.locShelf, c.locTray].filter(Boolean).join(' / ') || undefined,
          media_batch_id: c.media_batch_id || undefined,
          vessel_type: (c.custom_vessel ? c.vessel_input : c.vessel_type) || undefined,
          notes: c.notes || undefined,
          health_status: c.health_unknown ? '-1' : String(c.health_value),
          stage: c.stage || undefined,
          reminder_days: c.reminder_enabled ? (c.reminder_days || null) : null,
        })),
        observations: subcultureForm.observations || undefined,
        notes: subcultureForm.notes || undefined,
        health_status: effectivePassageHealth() !== '' ? effectivePassageHealth() : undefined,
        employee_id: subcultureForm.employee_id || undefined,
        contamination_flag: subcultureForm.contamination_flag || undefined,
        contamination_notes: subcultureForm.contamination_notes || undefined,
        temperature_c: subcultureForm.temperature_c ? parseFloat(subcultureForm.temperature_c) : undefined,
        ph: subcultureForm.ph ? parseFloat(subcultureForm.ph) : undefined,
        light_cycle: subcultureForm.light_cycle || undefined,
      });
      addNotification(
        `Split complete. ${result.children.length} new specimens: ${result.children.map(c => c.accession_number).join(', ')}. Parent archived.`,
        'success'
      );
      resetPassageForm();
      navigateTo('specimens');
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      submitting = false;
    }
  }

  async function handlePassage(e: Event) {
    e.preventDefault();
    if (!$selectedSpecimenId || !specimen) return;

    if (isSplitting) {
      // Show confirmation dialog instead of executing directly
      showSplitConfirm = true;
      return;
    }

    submitting = true;
    try {
      // ── Normal passage path ──
      const locationTo = composeLocationTo();
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
        notes: subcultureForm.notes || undefined,
        observations: subcultureForm.observations || undefined,
        health_status: effectivePassageHealth() !== '' ? effectivePassageHealth() : undefined,
        employee_id: subcultureForm.employee_id || undefined,
        contamination_flag: subcultureForm.contamination_flag || undefined,
        contamination_notes: subcultureForm.contamination_notes || undefined,
      });
      localStorage.setItem('sc_lastRoom', locToRoom);
      localStorage.setItem('sc_lastRack', locToRack);
      localStorage.setItem('sc_lastShelf', locToShelf);
      localStorage.setItem('sc_lastTray', locToTray);
      addNotification('Passage recorded.', 'success');
      resetPassageForm();
      loadAll($selectedSpecimenId!);
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      submitting = false;
    }
  }

  function navigateToSpecimen(id: string) {
    selectedSpecimenId.set(id);
  }

  function printCultureReport() {
    if (!specimen) return;
    const user = get(currentUser);
    const username = (user as any)?.display_name || (user as any)?.username || 'Unknown';
    const reportDate = new Date().toISOString().split('T')[0];

    // Shorter aliases for use inside the HTML template string.
    const esc = escHtml;

    // Passages oldest→newest for the report
    const passageRows = [...subcultures].reverse().map((sc: any) => {
      const batch = mediaBatches.find((m: any) => m.id === sc.media_batch_id);
      const batchName = batch ? esc(batch.batch_name || batch.id) : '—';
      const contam = sc.contamination_flag
        ? `<span class="b-red">Yes${sc.contamination_notes ? ' – ' + esc(sc.contamination_notes) : ''}</span>`
        : '<span class="b-green">No</span>';
      return `<tr>
        <td class="ctr"><b>${esc(sc.passage_number)}</b></td>
        <td>${esc(sc.date)}</td><td>${batchName}</td>
        <td>${esc(sc.vessel_type)}</td>
        <td>${esc(sc.location_to || sc.location_from)}</td>
        <td>${healthLabel(sc.health_status)}</td>
        <td>${contam}</td>
        <td class="note-cell">${esc(sc.observations || sc.notes)}</td>
      </tr>`;
    }).join('');

    const complianceRows = complianceRecords.map((cr: any) => `<tr>
      <td>${esc(cr.record_type)}</td>
      <td>${esc(cr.test_date || cr.issue_date)}</td>
      <td>${esc(cr.agency)}</td>
      <td>${esc(cr.test_result || cr.status || cr.result)}</td>
      <td>${esc(cr.permit_expiry || cr.expiry_date)}</td>
      <td class="note-cell">${esc(cr.notes)}</td>
    </tr>`).join('');

    const lineage = (parentSpecimen || childSpecimens.length > 0) ? `
      <h2>Lineage</h2>
      <div class="ig">
        ${parentSpecimen ? `<span class="il">Split From</span><span class="iv"><b>${esc(parentSpecimen.accession_number)}</b></span>` : ''}
        ${childSpecimens.length > 0 ? `<span class="il">Split Into</span><span class="iv">${childSpecimens.map((c: any) => `<span class="chip">${esc(c.accession_number)}</span>`).join(' ')}</span>` : ''}
      </div>` : '';

    const printCss = `*{margin:0;padding:0;box-sizing:border-box}html,body{height:100%}body{font-family:'Segoe UI',-apple-system,Helvetica,Arial,sans-serif;font-size:10.5px;color:#0f172a;background:#fff}.doc-header{display:flex;align-items:flex-end;justify-content:space-between;border-bottom:2.5px solid #0f172a;padding-bottom:11px;margin-bottom:16px;gap:16px}.doc-logo-area{width:64px;height:44px;border:1.5px dashed #cbd5e1;border-radius:4px;display:flex;align-items:center;justify-content:center;font-size:8px;color:#94a3b8;letter-spacing:.5px;flex-shrink:0}.doc-title-block{flex:1}.doc-brand{font-size:22px;font-weight:900;letter-spacing:-.5px;color:#0f172a;line-height:1}.doc-report-name{font-size:12px;color:#475569;margin-top:3px;font-weight:500}.doc-meta{text-align:right;font-size:9.5px;color:#64748b;line-height:1.8;flex-shrink:0}.doc-meta b{color:#0f172a}h2{font-size:9.5px;font-weight:700;color:#1d4ed8;text-transform:uppercase;letter-spacing:1px;margin:18px 0 7px;border-bottom:1px solid #e2e8f0;padding-bottom:4px}.ig{display:grid;grid-template-columns:155px 1fr;gap:4px 12px;page-break-inside:avoid}.il{font-size:9.5px;color:#64748b;font-weight:600;text-align:right;padding:2px 0}.iv{font-size:10.5px;padding:2px 0;color:#0f172a}table{width:100%;border-collapse:collapse;font-size:9.5px;margin-top:5px}thead{display:table-header-group}th{background:#f1f5f9;font-weight:700;text-align:left;padding:5px 8px;color:#475569;border:1px solid #e2e8f0;white-space:nowrap;font-size:9px;letter-spacing:.2px}td{padding:4px 8px;border:1px solid #e2e8f0;vertical-align:top}tr:nth-child(even) td{background:#f8fafc}tr{page-break-inside:avoid}.ctr{text-align:center}.note-cell{max-width:150px;word-break:break-word}.b-red{background:#fee2e2;color:#991b1b;padding:1px 5px;border-radius:3px;font-size:8.5px;font-weight:700}.b-green{background:#dcfce7;color:#166534;padding:1px 5px;border-radius:3px;font-size:8.5px;font-weight:700}.b-blue{background:#dbeafe;color:#1e40af;padding:1px 5px;border-radius:3px;font-size:8.5px;font-weight:700}.chip{display:inline-block;background:#e2e8f0;color:#334155;padding:1px 5px;border-radius:3px;font-size:9.5px;margin:1px}.doc-footer{margin-top:22px;border-top:1px solid #e2e8f0;padding-top:8px;display:flex;justify-content:space-between;align-items:center;font-size:8.5px;color:#94a3b8}.doc-footer-pagenum::after{content:"Page " counter(page) " of " counter(pages)}`;

    // ── Specimen info grid ─────────────────────────────────────────────────────
    const infoRows = [
      ['Accession',          `<b>${esc(specimen.accession_number)}</b>`],
      ['Species',            `${esc(specimen.species_name)} <span style="color:#64748b">(${esc(specimen.species_code)})</span>`],
      ['Stage',              `<span class="b-blue">${stageFmt(specimen.stage)}</span>`],
      ['Health Status',      healthLabel(specimen.health_status)],
      ['Initiated',          esc(specimen.initiation_date)],
      ['Current Location',   esc(specimen.location)],
      ['Propagation Method', esc(specimen.propagation_method)],
      ['Provenance',         esc(specimen.provenance)],
      ['Source Plant',       esc(specimen.source_plant)],
      ['Quarantine',         (specimen.quarantine_flag ? '<span class="b-red">Yes</span>' : '<span class="b-green">No</span>') +
                             (specimen.quarantine_release_date ? ` — Release: ${esc(specimen.quarantine_release_date)}` : '')],
      ['IP Protected',       (specimen.ip_flag ? '<span class="b-red">Yes</span>' : 'No') +
                             (specimen.ip_notes ? ` — ${esc(specimen.ip_notes)}` : '')],
      ['Total Passages',     esc(specimen.subculture_count)],
      ...(specimen.employee_id ? [['Employee ID', esc(specimen.employee_id)]] : []),
      ...(specimen.notes       ? [['Notes',       esc(specimen.notes)]]       : []),
    ].map(([label, value]) =>
      `<span class="il">${label}</span><span class="iv">${value}</span>`
    ).join('');

    const passageTable = subcultures.length === 0
      ? '<p style="color:#64748b;font-size:9.5px;margin-top:4px;">No passages recorded yet.</p>'
      : `<table><thead><tr>
           <th>#</th><th>Date</th><th>Media Batch</th><th>Vessel</th>
           <th>Transfer To</th><th>Health</th><th>Contamination</th><th>Notes</th>
         </tr></thead><tbody>${passageRows}</tbody></table>`;

    const complianceSection = complianceRecords.length > 0
      ? `<h2>Compliance Records (${complianceRecords.length})</h2>
         <table><thead><tr>
           <th>Type</th><th>Test/Issue Date</th><th>Agency</th>
           <th>Result/Status</th><th>Expiry</th><th>Notes</th>
         </tr></thead><tbody>${complianceRows}</tbody></table>`
      : '';

    const bodyHtml = `
<div class="doc-header">
  <div class="doc-logo-area">LOGO</div>
  <div class="doc-title-block">
    <div class="doc-brand">SteloPTC</div>
    <div class="doc-report-name">Culture Certificate</div>
  </div>
  <div class="doc-meta">
    <div><b>Accession:</b> ${esc(specimen.accession_number)}</div>
    <div><b>Generated:</b> ${reportDate}</div>
    <div><b>Prepared by:</b> ${esc(username)}</div>
  </div>
</div>

<h2>Specimen Information</h2>
<div class="ig">${infoRows}</div>

${lineage}

<h2>Passage History (${subcultures.length} passage${subcultures.length !== 1 ? 's' : ''})</h2>
${passageTable}

${complianceSection}

<div class="doc-footer">
  <span>SteloPTC · Tissue Culture Management System · ${reportDate}</span>
  <span class="doc-footer-pagenum"></span>
</div>`.trim();

    // ── Print delivery ─────────────────────────────────────────────────────────
    deliverPrint({
      frameId: 'ptc-cert-frame',
      title: `Culture Certificate – ${esc(specimen.accession_number)}`,
      css: printCss,
      body: bodyHtml,
      margin: '0.6in 0.65in',
      onError: (msg) => addNotification(msg, 'error'),
    });
  }
</script>

<div class="specimen-detail">
  <div class="page-header">
    <div style="display:flex;align-items:center;gap:12px;flex-wrap:wrap;">
      <button class="btn btn-sm" title="Return to specimen list" onclick={() => navigateTo('specimens')}>&larr; Back</button>
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
            <span class="health-badge" title="Current health score for this specimen (0=Dead, 4=Healthy)" style="background:{hb.color}20;color:{hb.color};border:1px solid {hb.color}60;">{hb.label}</span>
          {/if}
        {/if}
        {#if specimen.generation > 0}
          <span class="badge badge-purple" title="Generation {specimen.generation} — this specimen was derived from {specimen.generation} successive split{specimen.generation > 1 ? 's' : ''}">Gen {specimen.generation}</span>
        {/if}
        {#if specimen.quarantine_flag}
          <span class="badge badge-red" title="This specimen is under quarantine — movement restricted">Quarantined</span>
        {:else}
          <span class="badge badge-green" title="This specimen is active and not under quarantine">Active</span>
        {/if}
      {/if}
    </div>
    {#if specimen}
      <div style="display:flex;gap:8px;flex-wrap:wrap;">
        <button class="btn btn-qr-detail" onclick={() => (showQrScanner = true)}>
          &#128247; Scan QR <Tooltip text="Open camera to scan a QR code and navigate to the matching specimen" position="bottom" />
        </button>
        <button class="btn btn-qr-detail btn-qr-generate" onclick={() => (showQrModal = true)}>
          &#9641; Generate QR <Tooltip text="Generate a printable QR code label for this specimen — includes accession number, species, stage, and location" position="bottom" />
        </button>
        <button class="btn btn-print-report" onclick={printCultureReport} title="Print a full culture certificate for this specimen — includes all passage history and compliance records">
          &#128438; Print Report <Tooltip text="Open a print-ready culture certificate with specimen details, passage history, and compliance records" position="bottom" />
        </button>
      </div>
    {/if}
  </div>

  {#if loading}
    <div class="empty-state">Loading specimen…</div>
  {:else if specimen}

    <!-- ── Lineage Banner ── -->
    {#if parentSpecimen || childSpecimens.length > 0}
      {@const siblings = familyMembers.filter((m: any) => m.parent_specimen_id === specimen.parent_specimen_id && m.id !== specimen.id && !m.is_archived && specimen.parent_specimen_id)}
      <div class="lineage-banner">
        {#if parentSpecimen}
          <div class="lineage-row">
            <span class="lineage-icon">↑</span>
            <span class="lineage-label">Split from</span>
            <button class="lineage-chip parent-chip" title="Navigate to parent specimen — this specimen was split from {parentSpecimen.accession_number}" onclick={() => navigateToSpecimen(parentSpecimen.id)}>
              {parentSpecimen.accession_number}
              <span class="lineage-chip-sub">{parentSpecimen.species_code}</span>
            </button>
          </div>
        {/if}
        {#if siblings.length > 0}
          <div class="lineage-row">
            <span class="lineage-icon">↔</span>
            <span class="lineage-label">Sibling{siblings.length > 1 ? 's' : ''}</span>
            <div class="lineage-children">
              {#each siblings as sib}
                <button class="lineage-chip sibling-chip" title="Navigate to sibling specimen {sib.accession_number} — split from the same parent" onclick={() => navigateToSpecimen(sib.id)}>
                  {sib.accession_number}
                </button>
              {/each}
            </div>
          </div>
        {/if}
        {#if childSpecimens.length > 0}
          <div class="lineage-row">
            <span class="lineage-icon">↓</span>
            <span class="lineage-label">Split into {childSpecimens.length} container{childSpecimens.length > 1 ? 's' : ''}</span>
            <div class="lineage-children">
              {#each childSpecimens as child}
                <button class="lineage-chip child-chip" title="Navigate to child specimen {child.accession_number} — created by splitting this specimen" onclick={() => navigateToSpecimen(child.id)}>
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
          <span class="info-label" title="Unique accession identifier for this specimen">Accession</span>
          <span class="info-value mono">{specimen.accession_number}</span>
        </div>
        <div class="info-item">
          <span class="info-label" title="Current growth stage of this specimen (e.g. initiation, multiplication, rooting)">Stage</span>
          <span class="info-value"><span class="badge badge-blue">{stageLabel(specimen.stage)}</span></span>
        </div>
        <div class="info-item">
          <span class="info-label" title="Date this specimen was first brought into tissue culture">Initiation Date</span>
          <span class="info-value">{specimen.initiation_date}</span>
        </div>
        <div class="info-item">
          <span class="info-label" title="Current physical storage location of this specimen">Location</span>
          <span class="info-value">{specimen.location || '—'}</span>
        </div>
        <div class="info-item">
          <span class="info-label" title="Propagation technique used for this specimen (e.g. shoot tip, callus, embryogenesis)">Propagation</span>
          <span class="info-value">{specimen.propagation_method || '—'}</span>
        </div>
        <div class="info-item">
          <span class="info-label" title="Passages recorded for this specimen (P-total = cumulative passages from the root ancestor across all splits)">Passages</span>
          {#if specimen.generation > 0}
            {@const totalFromRoot = specimen.lineage_passage_offset + specimen.subculture_count}
            <span class="info-value">{specimen.subculture_count} <span style="color:#6b7280;font-size:12px;" title="P{totalFromRoot} = {specimen.lineage_passage_offset} ancestor passages + {specimen.subculture_count} own passages">(P{totalFromRoot} from root)</span></span>
          {:else}
            <span class="info-value">{specimen.subculture_count}</span>
          {/if}
        </div>
        <div class="info-item">
          <span class="info-label" title="Origin or history of this specimen (wild-collected, ex-situ, cultivar, etc.)">Provenance</span>
          <span class="info-value">{specimen.provenance || '—'}</span>
        </div>
        <div class="info-item">
          <span class="info-label" title="The donor or mother plant from which this specimen was derived">Source Plant</span>
          <span class="info-value">{specimen.source_plant || '—'}</span>
        </div>
        {#if specimen.permit_number}
          <div class="info-item">
            <span class="info-label" title="Regulatory permit number associated with this specimen (CITES, import/export, etc.)">Permit</span>
            <span class="info-value">{specimen.permit_number}{specimen.permit_expiry ? ` (exp: ${specimen.permit_expiry})` : ''}</span>
          </div>
        {/if}
      </div>
      {#if specimen.notes}
        <div style="margin-top:14px;padding-top:12px;border-top:1px solid #e2e8f0;">
          <span class="info-label" title="General notes recorded for this specimen">Notes</span>
          <p style="margin-top:4px;font-size:13px;white-space:pre-wrap;color:#374151;">{specimen.notes}</p>
        </div>
      {/if}
    </div>

    <!-- ── Tabs ── -->
    <div class="tabs">
      <button class="tab" title="View the chronological subculture/transfer history for this specimen" class:active={activeTab === 'history'} onclick={() => activeTab = 'history'}>
        Passage Timeline {#if subcultures.length > 0}<span class="tab-count">{subcultures.length}</span>{/if}
      </button>
      <button class="tab" title="View regulatory compliance and phytosanitary test records for this specimen" class:active={activeTab === 'compliance'} onclick={() => activeTab = 'compliance'}>
        Compliance {#if complianceRecords.length > 0}<span class="tab-count">{complianceRecords.length}</span>{/if}
      </button>
      <button class="tab" title="View and manage photo attachments for this specimen" class:active={activeTab === 'photos'} onclick={() => activeTab = 'photos'}>
        Photos {#if photos.length > 0}<span class="tab-count">{photos.length}</span>{/if}
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
            {#if !showPassageForm}<Tooltip text="Log a new subculture or transfer event for this specimen — records date, media batch, vessel, health, location, and observations" position="bottom" />{/if}
          </button>
        </div>

        <!-- ── Record Passage Form ── -->
        {#if showPassageForm}
          <form onsubmit={handlePassage} class="passage-form">

            <!-- Date + Media -->
            <div class="form-row">
              <div class="form-group">
                <label>Date <Tooltip text="Date on which this passage/subculture was performed" /></label>
                <input type="date" title="Date on which this passage/subculture was performed" bind:value={subcultureForm.date} required />
              </div>
              <div class="form-group" style="flex:2;">
                <label>Media Batch <Tooltip text="Select the nutrient media batch used for this transfer — must be a batch prepared on or before the passage date" /></label>
                <select title="Select the media batch used for this transfer" bind:value={subcultureForm.media_batch_id}>
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
                <label title="Type of container used for this passage (jar, flask, Petri dish, etc.)">Vessel Type</label>
                <select title="Type of container used for this passage (jar, flask, Petri dish, etc.)" bind:value={subcultureForm.vessel_type}>
                  <option value="">Select vessel…</option>
                  {#each vesselTypes as v}
                    <option value={v}>{v}</option>
                  {/each}
                </select>
              </div>
              <div class="form-group env-field">
                <label title="Incubation/growth room temperature in degrees Celsius">Temp (°C)</label>
                <input type="number" step="0.1" title="Incubation/growth room temperature in degrees Celsius" bind:value={subcultureForm.temperature_c} placeholder="25" />
              </div>
              <div class="form-group env-field">
                <label title="pH of the culture media used for this passage">pH</label>
                <input type="number" step="0.01" title="pH of the culture media used for this passage" bind:value={subcultureForm.ph} placeholder="5.7" />
              </div>
              <div class="form-group env-field-wide">
                <label title="Photoperiod applied during this passage — format: hours on / hours off (e.g. 16/8)">Light Cycle (hrs on/hrs off)</label>
                <input type="text" title="Photoperiod applied during this passage — format: hours on / hours off (e.g. 16/8)" bind:value={subcultureForm.light_cycle} placeholder="16/8" />
              </div>
            </div>

            <!-- Transfer To Location -->
            <div class="section-header">Transfer To Location</div>
            <div class="form-row">
              <div class="form-group">
                <label title="Growth room where this specimen will be placed after transfer">Room</label>
                <select title="Growth room where this specimen will be placed after transfer" bind:value={locToRoom}>
                  <option value="">—</option>
                  {#each rooms as r}<option value={r}>{r}</option>{/each}
                </select>
              </div>
              <div class="form-group">
                <label title="Storage rack within the room where this specimen will be placed">Rack</label>
                <select title="Storage rack within the room where this specimen will be placed" bind:value={locToRack}>
                  <option value="">—</option>
                  {#each racks as r}<option value={r}>{r}</option>{/each}
                </select>
              </div>
              <div class="form-group">
                <label title="Shelf level on the rack where this specimen will be placed">Shelf</label>
                <select title="Shelf level on the rack where this specimen will be placed" bind:value={locToShelf}>
                  <option value="">—</option>
                  {#each shelves as s}<option value={s}>{s}</option>{/each}
                </select>
              </div>
              <div class="form-group">
                <label title="Tray position on the shelf where this specimen will be placed">Tray</label>
                <select title="Tray position on the shelf where this specimen will be placed" bind:value={locToTray}>
                  <option value="">—</option>
                  {#each trays as t}<option value={t}>{t}</option>{/each}
                </select>
              </div>
            </div>

            <!-- Health Status -->
            <div class="form-group">
              <label title="Observed health condition of this specimen at the time of this passage">Health Status</label>
              <div class="health-slider-wrap">
                <label class="unknown-toggle" title="Check this if health cannot be assessed yet — records health as unknown/awaiting">
                  <input type="checkbox" title="Mark health as unknown or awaiting assessment" bind:checked={subcultureForm.health_unknown} style="width:auto;" />
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
                    aria-label="Health status"
                    aria-valuemin="0"
                    aria-valuemax="4"
                    aria-valuenow={passageHealthValue}
                    aria-valuetext="{passageHealthValue} – {healthLabels[passageHealthValue]}"
                    title="Drag to set health score: 0=Dead, 1=Poor, 2=Fair, 3=Good, 4=Healthy"
                    style="--track-color: {healthColors[passageHealthValue]};"
                  />
                  <div class="health-ticks">
                    {#each healthLabels as lbl, i}
                      <span class="health-tick" title="Health score {i} — {lbl}" class:active={passageHealthValue === i} style={passageHealthValue === i ? `color:${healthColors[i]};` : ''}>
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
              <label title="ID or badge number of the technician who performed this passage (for traceability)">Employee ID / Badge #</label>
              <input type="text" title="ID or badge number of the technician who performed this passage (for traceability)" bind:value={subcultureForm.employee_id} placeholder="e.g., EMP-042" />
            </div>

            <!-- Contamination -->
            <div class="contamination-row" class:active={subcultureForm.contamination_flag}>
              <label class="contam-toggle-label" title="Flag this vessel as contaminated (bacterial, fungal, yeast, or other)">
                <input type="checkbox" title="Flag this vessel as contaminated (bacterial, fungal, yeast, or other)" bind:checked={subcultureForm.contamination_flag} style="width:auto;" />
                <span class="contam-toggle-text">Contamination detected in this vessel</span>
              </label>
              {#if subcultureForm.contamination_flag}
                <div class="form-group" style="margin-top:8px;">
                  <label title="Describe the contamination observed — type (bacterial, fungal, yeast), extent, and corrective action taken">Contamination Notes</label>
                  <textarea
                    title="Describe the contamination observed — type (bacterial, fungal, yeast), extent, and corrective action taken"
                    bind:value={subcultureForm.contamination_notes}
                    rows="2"
                    placeholder="Describe type (bacterial, fungal, yeast…), extent, and any action taken…"
                  ></textarea>
                </div>
              {/if}
            </div>

            <!-- Observations + Notes -->
            <div class="form-row">
              <div class="form-group" style="flex:1;">
                <label title="Visual or qualitative observations made at time of passage (growth, morphology, colour, etc.)">Observations</label>
                <textarea title="Visual or qualitative observations made at time of passage (growth, morphology, colour, etc.)" bind:value={subcultureForm.observations} rows="2" placeholder="Growth observations, morphology…"></textarea>
              </div>
              <div class="form-group" style="flex:1;">
                <label title="Procedural notes for this passage — protocol deviations, reagent lot numbers, special conditions, etc.">Notes</label>
                <textarea title="Procedural notes for this passage — protocol deviations, reagent lot numbers, special conditions, etc." bind:value={subcultureForm.notes} rows="2" placeholder="Protocol notes, reagent lots…"></textarea>
              </div>
            </div>

            <!-- Split Culture Toggle -->
            <div class="split-toggle-row" class:active={isSplitting}>
              <label class="split-toggle-label" title="Split this specimen into multiple child specimens — parent will be archived and each child gets its own passage record and audit chain">
                <input type="checkbox" title="Enable split mode — parent specimen will be archived" bind:checked={isSplitting} style="margin-right:6px;" />
                Split culture into multiple child specimens
              </label>

              {#if isSplitting}
                <div class="split-count-row">
                  <span class="split-desc">Number of children:</span>
                  <button type="button" class="split-count-btn" onclick={() => { if (splitCount > 2) splitCount--; }} title="Remove one child">−</button>
                  <span class="split-count-display">{splitCount}</span>
                  <button type="button" class="split-count-btn" onclick={() => { if (splitCount < 26) splitCount++; }} title="Add one child">+</button>
                  <span class="split-hint">Parent will be <strong>archived</strong>. Each child starts its own chain at passage 1.</span>
                </div>

                <!-- Per-child cards -->
                {#each splitChildren.slice(0, splitCount) as child, i}
                  {@const letter = String.fromCharCode(65 + i)}
                  <div class="split-child-card">
                    <!-- Card header -->
                    <div class="split-card-header">
                      <span class="split-letter-badge">{letter}</span>
                      <div class="form-group" style="flex:1;margin-bottom:0;">
                        <label style="font-size:10px;font-weight:700;text-transform:uppercase;color:#6b7280;letter-spacing:.4px;">Accession Number</label>
                        <input
                          type="text"
                          class="split-accession-input"
                          bind:value={child.accessionNumber}
                          placeholder="Auto-generated…"
                          title="Accession number for child {letter} — auto-generated from parent with letter suffix, or enter custom"
                        />
                      </div>
                      <div class="form-group" style="flex:0 0 160px;margin-bottom:0;">
                        <label style="font-size:10px;font-weight:700;text-transform:uppercase;color:#6b7280;letter-spacing:.4px;">Stage</label>
                        <select bind:value={child.stage} title="Stage for child {letter}">
                          {#each stageOptions as opt}
                            <option value={opt.value}>{opt.label}</option>
                          {/each}
                        </select>
                      </div>
                    </div>

                    <!-- Health -->
                    <div class="form-group" style="margin-bottom:8px;">
                      <label style="font-size:10px;font-weight:700;text-transform:uppercase;color:#6b7280;letter-spacing:.4px;">Health Status</label>
                      <div class="split-health-row">
                        <label class="unknown-toggle" title="Mark health as unknown">
                          <input type="checkbox" bind:checked={child.health_unknown} style="width:auto;" />
                          Unknown
                        </label>
                        {#if child.health_unknown}
                          <span style="font-size:12px;color:#7c3aed;font-weight:600;">? – Unknown / Awaiting</span>
                        {:else}
                          <input
                            type="range" min="0" max="4" step="1"
                            bind:value={child.health_value}
                            class="health-slider"
                            style="--track-color:{hcolors[child.health_value]};flex:1;"
                            title="Health: 0=Dead, 4=Healthy"
                          />
                          <span class="split-health-label" style="color:{hcolors[child.health_value]};">
                            {child.health_value} – {hlabels[child.health_value]}
                          </span>
                        {/if}
                      </div>
                    </div>

                    <!-- Location -->
                    <div class="section-header">Location</div>
                    <div class="form-row" style="margin-bottom:8px;">
                      <div class="form-group">
                        <select title="Room" bind:value={child.locRoom}>
                          <option value="">Room—</option>
                          {#each rooms as r}<option value={r}>{r}</option>{/each}
                        </select>
                      </div>
                      <div class="form-group">
                        <select title="Rack" bind:value={child.locRack}>
                          <option value="">Rack—</option>
                          {#each racks as r}<option value={r}>{r}</option>{/each}
                        </select>
                      </div>
                      <div class="form-group">
                        <select title="Shelf" bind:value={child.locShelf}>
                          <option value="">Shelf—</option>
                          {#each shelves as s}<option value={s}>{s}</option>{/each}
                        </select>
                      </div>
                      <div class="form-group">
                        <select title="Tray" bind:value={child.locTray}>
                          <option value="">Tray—</option>
                          {#each trays as t}<option value={t}>{t}</option>{/each}
                        </select>
                      </div>
                    </div>

                    <!-- Media + Vessel -->
                    <div class="form-row" style="margin-bottom:8px;">
                      <div class="form-group" style="flex:2;">
                        <label style="font-size:10px;font-weight:700;text-transform:uppercase;color:#6b7280;letter-spacing:.4px;">Media Batch</label>
                        <select bind:value={child.media_batch_id} title="Media batch for child {letter}">
                          <option value="">No media / not recorded</option>
                          {#each mediaBatches as mb}
                            <option value={mb.id}>{mb.is_draft ? '⚠ ' : ''}{mb.batch_id} — {mb.name}</option>
                          {/each}
                          <option value="__new_draft__">＋ Add new (draft)…</option>
                        </select>
                        {#if child.media_batch_id === '__new_draft__'}
                          <button
                            type="button"
                            class="btn btn-sm"
                            style="margin-top:4px;font-size:11px;"
                            onclick={() => { child.media_batch_id = ''; openDraftMediaDialog(i); }}
                          >Create Draft Batch</button>
                        {/if}
                      </div>
                      <div class="form-group" style="flex:2;">
                        <label style="font-size:10px;font-weight:700;text-transform:uppercase;color:#6b7280;letter-spacing:.4px;">Vessel Type</label>
                        {#if child.custom_vessel}
                          <div style="display:flex;gap:4px;">
                            <input type="text" bind:value={child.vessel_input} placeholder="Custom vessel name…" style="flex:1;" />
                            <button type="button" class="btn btn-sm" onclick={() => { child.custom_vessel = false; child.vessel_input = ''; }} style="font-size:11px;white-space:nowrap;">× Clear</button>
                          </div>
                        {:else}
                          <select
                            value={child.vessel_type}
                            onchange={(e) => {
                              const val = (e.target as HTMLSelectElement).value;
                              if (val === '__custom__') { child.vessel_type = ''; child.custom_vessel = true; }
                              else { child.vessel_type = val; }
                            }}
                            title="Vessel type for child {letter}"
                          >
                            <option value="">Select vessel…</option>
                            {#each vesselTypes as v}<option value={v}>{v}</option>{/each}
                            <option value="__custom__">— Custom / other…</option>
                          </select>
                        {/if}
                      </div>
                    </div>

                    <!-- Notes + Reminder -->
                    <div class="form-row" style="margin-bottom:0;">
                      <div class="form-group" style="flex:2;">
                        <label style="font-size:10px;font-weight:700;text-transform:uppercase;color:#6b7280;letter-spacing:.4px;">Notes (optional)</label>
                        <input type="text" bind:value={child.notes} placeholder="Per-container notes…" title="Notes for child {letter}" />
                      </div>
                      <div class="form-group split-reminder-group">
                        <label style="font-size:10px;font-weight:700;text-transform:uppercase;color:#6b7280;letter-spacing:.4px;">Check-in Reminder</label>
                        <div class="split-reminder-row">
                          <label class="unknown-toggle">
                            <input type="checkbox" bind:checked={child.reminder_enabled} style="width:auto;" />
                            In
                          </label>
                          <input
                            type="number" min="1" max="365"
                            bind:value={child.reminder_days}
                            disabled={!child.reminder_enabled}
                            style="width:60px;padding:4px 6px;font-size:12px;"
                            title="Days after split date to create a check-in reminder"
                          />
                          <span style="font-size:12px;color:#6b7280;">days</span>
                        </div>
                      </div>
                    </div>
                  </div>
                {/each}

                <!-- Split summary preview -->
                <div class="split-summary-box">
                  <div class="split-summary-title">Split Preview</div>
                  <div class="split-summary-row">
                    <span class="split-summary-label">Parent (archived):</span>
                    <span class="split-summary-value" style="font-family:monospace;color:#dc2626;">{specimen?.accession_number}</span>
                  </div>
                  <div class="split-summary-row">
                    <span class="split-summary-label">Creating {splitCount} children:</span>
                    <div class="split-summary-chips">
                      {#each splitChildren.slice(0, splitCount) as child, i}
                        <span class="split-summary-chip">{child.accessionNumber || `Child ${i+1}`}</span>
                      {/each}
                    </div>
                  </div>
                </div>
              {/if}
            </div>

            <div style="display:flex;justify-content:flex-end;margin-top:12px;">
              <button type="submit" class="btn btn-primary"
                title={isSplitting
                  ? `Review and confirm split of this specimen into ${splitCount} children`
                  : 'Save this passage event to the specimen record'}
                disabled={submitting}>
                {submitting
                  ? (isSplitting ? 'Splitting…' : 'Recording…')
                  : isSplitting
                    ? `Review Split (${splitCount} children) →`
                    : 'Record Passage'}
              </button>
            </div>
          </form>
        {/if}

        <!-- ── Timeline ── -->
        <SpecimenPassageTimeline
          subcultures={subcultures}
          specimenId={$selectedSpecimenId!}
          onreload={() => loadAll($selectedSpecimenId!)}
        />
      </div>

    <!-- ── Photos Tab ── -->
    {:else if activeTab === 'photos'}
      <div class="card" style="margin-top:0;border-top-left-radius:0;border-top-right-radius:0;">
        <SpecimenPhotoGallery
          specimenId={$selectedSpecimenId!}
          photos={photos}
          onphotoschanged={() => loadPhotos($selectedSpecimenId!)}
        />
      </div>

    <!-- ── Compliance Tab ── -->
    {:else if activeTab === 'compliance'}
      <div class="card" style="margin-top:0;border-top-left-radius:0;border-top-right-radius:0;">
        <h3 style="margin-bottom:12px;font-size:15px;">Compliance Records</h3>
        <SpecimenComplianceTable records={complianceRecords} />
      </div>
    {/if}

  {/if}
</div>

<!-- Split Confirmation Dialog -->
{#if showSplitConfirm}
  <div class="modal-overlay" onclick={() => showSplitConfirm = false} role="dialog" aria-modal="true" aria-label="Confirm split">
    <div class="modal-box confirm-dialog" onclick={(e) => e.stopPropagation()}>
      <div class="confirm-header">
        <span class="confirm-icon">&#9888;</span>
        <h3 class="confirm-title">Confirm Specimen Split</h3>
      </div>
      <div class="confirm-warning">
        <strong>Before proceeding, verify:</strong>
        <ul>
          <li>Specimens are in the correct physical order matching the child letters (A, B, C…).</li>
          <li>All containers are properly labelled.</li>
          <li>Parent specimen <strong style="font-family:monospace;">{specimen?.accession_number}</strong> will be <strong>permanently archived</strong>.</li>
        </ul>
      </div>
      <div class="confirm-children">
        <div class="confirm-children-label">Will create {splitCount} new specimen{splitCount !== 1 ? 's' : ''}:</div>
        <div class="confirm-children-chips">
          {#each splitChildren.slice(0, splitCount) as child, i}
            {@const letter = String.fromCharCode(65 + i)}
            <div class="confirm-child-chip">
              <span class="confirm-chip-letter">{letter}</span>
              <span class="confirm-chip-accession">{child.accessionNumber || `Child ${i + 1}`}</span>
              {#if child.reminder_enabled}
                <span class="confirm-chip-reminder">&#128276; {child.reminder_days}d</span>
              {/if}
            </div>
          {/each}
        </div>
      </div>
      <div class="confirm-actions">
        <button class="btn" onclick={() => showSplitConfirm = false} disabled={submitting}>Cancel</button>
        <button class="btn btn-danger" onclick={executeSplit} disabled={submitting}>
          {submitting ? 'Splitting…' : `Confirm Split — ${splitCount} children`}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Draft Media Batch Dialog -->
{#if showDraftMediaDialog}
  <div class="modal-overlay" onclick={() => { showDraftMediaDialog = false; }} role="dialog" aria-modal="true" aria-label="Create draft media batch">
    <div class="modal-box" onclick={(e) => e.stopPropagation()}>
      <h3 class="modal-title">Create Draft Media Batch</h3>
      <p class="modal-desc">Enter a working name for this draft. You can complete the preparation details later in Media Management.</p>
      <div class="form-group" style="margin-bottom:16px;">
        <label>Batch Name</label>
        <input type="text" bind:value={draftMediaName} placeholder="e.g., MS Half-Strength (in prep)" />
      </div>
      <div class="modal-actions">
        <button class="btn" onclick={() => { showDraftMediaDialog = false; draftMediaName = ''; }} disabled={draftMediaSubmitting}>Cancel</button>
        <button class="btn btn-primary" onclick={createDraftMedia} disabled={draftMediaSubmitting || !draftMediaName.trim()}>
          {draftMediaSubmitting ? 'Creating…' : 'Create Draft'}
        </button>
      </div>
    </div>
  </div>
{/if}

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

  .btn-print-report {
    background: #f5f3ff;
    color: #5b21b6;
    border: 1px solid #c4b5fd;
    border-radius: 7px;
    padding: 7px 14px;
    font-size: 12px;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    transition: background 0.1s;
  }
  .btn-print-report:hover { background: #ede9fe; }
  :global(.dark) .btn-print-report { background: rgba(139,92,246,0.12); color: #a78bfa; border-color: #5b21b6; }

  @media (max-width: 768px) {
    .btn-qr-detail { min-height: 44px; font-size: 14px; }
    .btn-print-report { min-height: 44px; font-size: 14px; }
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
  .sibling-chip { background: #ede9fe; color: #6d28d9; }
  .sibling-chip:hover { background: #ddd6fe; }
  :global(.dark) .parent-chip { background: #1e3a8a; color: #93c5fd; }
  :global(.dark) .child-chip { background: #14532d; color: #86efac; }
  :global(.dark) .sibling-chip { background: #4c1d95; color: #c4b5fd; }
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
  .split-count-btn {
    width: 28px; height: 28px; border-radius: 6px; border: 1px solid #d1d5db;
    background: #f9fafb; font-size: 18px; font-weight: 700; cursor: pointer;
    display: inline-flex; align-items: center; justify-content: center; line-height: 1;
  }
  .split-count-btn:hover { background: #e5e7eb; }
  :global(.dark) .split-count-btn { background: #1e293b; border-color: #475569; color: #f1f5f9; }
  .split-count-display {
    min-width: 28px; text-align: center; font-size: 16px; font-weight: 700; color: #111827;
  }
  :global(.dark) .split-count-display { color: #f1f5f9; }

  .split-children-header {
    display: grid;
    grid-template-columns: 28px 1fr 1fr 1fr 1fr;
    gap: 6px;
    margin-top: 12px;
    font-size: 10px; font-weight: 700; text-transform: uppercase;
    letter-spacing: 0.5px; color: #9ca3af; padding: 0 2px;
  }
  .split-child-row {
    display: grid;
    grid-template-columns: 28px 1fr 1fr 1fr 1fr;
    gap: 6px;
    align-items: center;
    margin-top: 6px;
    padding: 6px 4px;
    border: 1px solid #e2e8f0;
    border-radius: 6px;
    background: #fff;
  }
  :global(.dark) .split-child-row { background: #0f172a; border-color: #334155; }
  .split-child-num {
    font-size: 12px; font-weight: 700; color: #6b7280; text-align: center;
  }
  .split-child-location {
    display: flex; gap: 4px; flex-wrap: wrap;
  }
  .split-child-location select {
    flex: 1; min-width: 80px; padding: 4px 6px; font-size: 11px;
  }
  .split-child-select {
    padding: 4px 6px; font-size: 11px; border: 1px solid #d1d5db; border-radius: 4px;
  }
  :global(.dark) .split-child-select { background: #1e293b; color: #f1f5f9; border-color: #475569; }
  .split-child-notes {
    padding: 4px 6px; font-size: 11px; border: 1px solid #d1d5db; border-radius: 4px;
    width: 100%;
  }
  :global(.dark) .split-child-notes { background: #1e293b; color: #f1f5f9; border-color: #475569; }

  /* ── Split child cards ── */
  .split-child-card {
    border: 1px solid #e2e8f0;
    border-radius: 8px;
    padding: 14px;
    margin-top: 12px;
    background: #fff;
  }
  :global(.dark) .split-child-card { background: #0f172a; border-color: #334155; }

  .split-card-header {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    margin-bottom: 12px;
    flex-wrap: wrap;
  }

  .split-letter-badge {
    flex-shrink: 0;
    width: 32px;
    height: 32px;
    border-radius: 50%;
    background: linear-gradient(135deg, #2563eb, #7c3aed);
    color: #fff;
    font-size: 14px;
    font-weight: 800;
    display: flex;
    align-items: center;
    justify-content: center;
    margin-top: 18px;
    box-shadow: 0 2px 6px rgba(37,99,235,0.25);
  }

  .split-accession-input {
    font-family: 'JetBrains Mono', monospace;
    font-size: 13px;
    letter-spacing: 0.5px;
  }

  .split-health-row {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }
  .split-health-label {
    font-size: 12px;
    font-weight: 700;
    min-width: 80px;
    white-space: nowrap;
  }

  .split-reminder-group { flex: 0 0 200px; }
  .split-reminder-row {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
    margin-top: 4px;
  }

  /* Split summary preview box */
  .split-summary-box {
    border: 1px solid #bfdbfe;
    border-radius: 8px;
    background: linear-gradient(135deg, #eff6ff, #f0fdf4);
    padding: 14px 16px;
    margin-top: 16px;
  }
  :global(.dark) .split-summary-box { background: linear-gradient(135deg, #1e3a5f22, #14532d22); border-color: #1e40af; }
  .split-summary-title {
    font-size: 11px;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.6px;
    color: #1d4ed8;
    margin-bottom: 10px;
  }
  :global(.dark) .split-summary-title { color: #60a5fa; }
  .split-summary-row {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    margin-bottom: 8px;
    flex-wrap: wrap;
  }
  .split-summary-label {
    font-size: 12px;
    font-weight: 600;
    color: #6b7280;
    white-space: nowrap;
    min-width: 140px;
  }
  .split-summary-value {
    font-size: 13px;
    font-weight: 700;
  }
  .split-summary-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .split-summary-chip {
    display: inline-block;
    background: #dbeafe;
    color: #1e40af;
    border-radius: 4px;
    padding: 2px 8px;
    font-size: 12px;
    font-weight: 700;
    font-family: 'JetBrains Mono', monospace;
    letter-spacing: 0.3px;
  }
  :global(.dark) .split-summary-chip { background: #1e3a8a; color: #93c5fd; }

  /* ── Modal overlays ── */
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    padding: 16px;
  }
  .modal-box {
    background: #fff;
    border-radius: 12px;
    padding: 24px;
    max-width: 500px;
    width: 100%;
    box-shadow: 0 20px 60px rgba(0,0,0,0.25);
  }
  :global(.dark) .modal-box { background: #1e293b; }
  .modal-title {
    font-size: 16px;
    font-weight: 700;
    color: #111827;
    margin-bottom: 8px;
  }
  :global(.dark) .modal-title { color: #f1f5f9; }
  .modal-desc {
    font-size: 13px;
    color: #6b7280;
    margin-bottom: 16px;
    line-height: 1.5;
  }
  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 20px;
  }

  /* Confirmation dialog specifics */
  .confirm-dialog { max-width: 520px; }
  .confirm-header {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 14px;
  }
  .confirm-icon {
    font-size: 22px;
    color: #d97706;
  }
  .confirm-title {
    font-size: 17px;
    font-weight: 700;
    color: #92400e;
    margin: 0;
  }
  :global(.dark) .confirm-title { color: #fcd34d; }
  .confirm-warning {
    background: #fffbeb;
    border: 1px solid #fcd34d;
    border-radius: 8px;
    padding: 12px 14px;
    margin-bottom: 16px;
    font-size: 13px;
    color: #78350f;
    line-height: 1.6;
  }
  :global(.dark) .confirm-warning { background: #292524; border-color: #92400e; color: #fcd34d; }
  .confirm-warning ul {
    margin: 8px 0 0 16px;
    padding: 0;
  }
  .confirm-warning li { margin-bottom: 4px; }
  .confirm-children { margin-bottom: 16px; }
  .confirm-children-label {
    font-size: 12px;
    font-weight: 700;
    color: #374151;
    margin-bottom: 8px;
    text-transform: uppercase;
    letter-spacing: 0.4px;
  }
  :global(.dark) .confirm-children-label { color: #94a3b8; }
  .confirm-children-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .confirm-child-chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 5px 10px;
    background: #f1f5f9;
    border: 1px solid #e2e8f0;
    border-radius: 6px;
    font-size: 12px;
  }
  :global(.dark) .confirm-child-chip { background: #0f172a; border-color: #334155; }
  .confirm-chip-letter {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: #2563eb;
    color: #fff;
    font-size: 11px;
    font-weight: 800;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .confirm-chip-accession {
    font-family: 'JetBrains Mono', monospace;
    font-size: 12px;
    font-weight: 600;
    color: #111827;
  }
  :global(.dark) .confirm-chip-accession { color: #f1f5f9; }
  .confirm-chip-reminder {
    font-size: 10px;
    color: #7c3aed;
    background: #ede9fe;
    border-radius: 4px;
    padding: 1px 5px;
  }
  :global(.dark) .confirm-chip-reminder { background: #4c1d95; color: #c4b5fd; }
  .confirm-actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 20px;
  }
  .btn-danger {
    background: #dc2626;
    color: #fff;
    border-color: #b91c1c;
    font-weight: 700;
  }
  .btn-danger:hover { background: #b91c1c; }
  .btn-danger:disabled { background: #fca5a5; border-color: #fca5a5; }

  /* Contamination toggle in passage form */
  .contamination-row {
    border: 1px dashed #fca5a5; border-radius: 6px;
    padding: 12px; margin-top: 4px; margin-bottom: 8px;
    background: #fff1f2;
  }
  :global(.dark) .contamination-row { background: #1c0404; border-color: #7f1d1d; }
  .contamination-row.active { border-color: #ef4444; background: #fee2e2; }
  :global(.dark) .contamination-row.active { background: #450a0a; }
  .contam-toggle-label { display: inline-flex; align-items: center; gap: 8px; cursor: pointer; font-size: 13px; font-weight: 600; }
  .contam-toggle-text { color: #b91c1c; }
  :global(.dark) .contam-toggle-text { color: #f87171; }

</style>
