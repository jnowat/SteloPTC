<script lang="ts">
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import {
    listSpecimens, searchSpecimens, deleteSpecimen, listSpecies, listProjects,
    bulkArchiveSpecimens, bulkUpdateLocation, bulkUpdateStage, listStages,
  } from '../api';
  import { navigateTo, addNotification, selectedSpecimenId } from '../stores/app';
  import { currentUser } from '../stores/auth';
  import { escHtml, stageFmt, healthLabel } from '../utils';
  import { deliverPrint, ageDays, fmtAge, healthNum } from '../printUtils';
  import SpecimenForm from './SpecimenForm.svelte';
  import QrModal from './QrModal.svelte';
  import QrScanner from './QrScanner.svelte';
  import Tooltip from './Tooltip.svelte';
  import FirstRun from './FirstRun.svelte';
  import DataState from './DataState.svelte';

  // `specimens` holds only the currently-loaded page (used for page-scoped
  // things like "select all on this page" and the print report, which
  // intentionally reports on what's currently in view).
  // `loadedSpecimens` accumulates every page fetched so far and is what the
  // virtualized table actually renders from — this is what grows as the
  // user scrolls, while `specimens` stays "current page" sized.
  let specimens = $state<any[]>([]);
  let loadedSpecimens = $state<any[]>([]);
  let species = $state<any[]>([]);
  let projects = $state<any[]>([]);
  let total = $state(0);
  let page = $state(1);
  let perPage = $state(200);
  let totalPages = $state(0);
  let loading = $state(true);
  let loadingMore = $state(false);
  let error = $state<string | null>(null);
  let searchQuery = $state('');
  let filterSpecies = $state('');
  let filterStage = $state('');
  let filterProject = $state('');
  let showForm = $state(false);
  let qrSpecimen = $state<any>(null);
  let showScanner = $state(false);

  // Print report options
  let showPrintOptions = $state(false);
  let printGroupBy = $state<'stage' | 'health' | 'none'>('stage');

  // Batch selection
  let selectedIds = $state(new Set<string>());
  let batchAction = $state<'location' | 'stage' | null>(null);
  let batchLoading = $state(false);
  // Batch location
  const rooms  = ['1','2','3','4','5'];
  const racks  = ['A','B','C','D'];
  const shelves = ['1','2','3','4','5'];
  const trays  = ['A','B','C','D','E','F'];
  let batchRoom = $state('');
  let batchRack = $state('');
  let batchShelf = $state('');
  let batchTray = $state('');
  // Batch stage
  let batchStage = $state('');

  let stages = $state<any[]>([]);

  // "select all" now spans everything currently loaded into the buffer
  // (there's no longer a discrete "page" of rows on screen — continuous
  // scroll accumulates rows into `loadedSpecimens`).
  let allPageSelected = $derived(
    loadedSpecimens.length > 0 && loadedSpecimens.every(s => selectedIds.has(s.id))
  );
  let someSelected = $derived(
    loadedSpecimens.some(s => selectedIds.has(s.id)) && !allPageSelected
  );

  // ── Virtual scrolling ───────────────────────────────────────────
  // The table body only ever renders the rows within [start, end] (plus a
  // small overscan buffer) no matter how many rows are loaded — the rest of
  // the scroll height is represented purely by the spacer's computed height,
  // and each rendered row is absolutely positioned at `index * ROW_HEIGHT`.
  const ROW_HEIGHT = 44; // px — enforced via CSS below so this stays authoritative
  const OVERSCAN = 10; // rows rendered above/below the visible window
  const PREFETCH_THRESHOLD = 0.8; // fetch next page at 80% scroll through the buffer

  let scrollContainer = $state<HTMLDivElement | null>(null);
  let sentinelEl = $state<HTMLDivElement | null>(null);
  let scrollTop = $state(0);
  let viewportHeight = $state(600);
  let containerObserver: ResizeObserver | null = null;
  let sentinelObserver: IntersectionObserver | null = null;

  let startIndex = $derived(
    Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN)
  );
  let endIndex = $derived(
    Math.min(
      loadedSpecimens.length,
      Math.ceil((scrollTop + viewportHeight) / ROW_HEIGHT) + OVERSCAN
    )
  );
  let visibleSpecimens = $derived(loadedSpecimens.slice(startIndex, endIndex));
  let totalScrollHeight = $derived(loadedSpecimens.length * ROW_HEIGHT);
  // Sentinel sits 80% of the way through the currently-loaded buffer so it
  // intersects (and triggers the next page fetch) well before the user
  // reaches the bottom of what's actually rendered.
  let sentinelOffset = $derived(
    Math.max(0, Math.floor(loadedSpecimens.length * PREFETCH_THRESHOLD) * ROW_HEIGHT)
  );

  function handleScroll() {
    if (scrollContainer) scrollTop = scrollContainer.scrollTop;
  }

  function setupSentinelObserver() {
    sentinelObserver?.disconnect();
    if (!sentinelEl || !scrollContainer) return;
    sentinelObserver = new IntersectionObserver(
      (entries) => {
        if (entries.some(e => e.isIntersecting)) loadNextPage();
      },
      { root: scrollContainer, rootMargin: '0px', threshold: 0 }
    );
    sentinelObserver.observe(sentinelEl);
  }

  // Re-attach the observer whenever the sentinel's position moves (i.e. the
  // loaded buffer changed), since its target element is re-rendered at a new
  // offset rather than being a stable DOM node across loads.
  $effect(() => {
    // touch reactive deps so this reruns when the buffer grows/shrinks
    void loadedSpecimens.length;
    void sentinelEl;
    setupSentinelObserver();
  });

  // `scrollContainer` only exists once the table (rather than a loading/
  // empty state) is actually rendered, so wire up the ResizeObserver
  // reactively rather than once at onMount — it may not be mounted yet the
  // first time onMount runs.
  $effect(() => {
    const el = scrollContainer;
    containerObserver?.disconnect();
    if (!el) return;
    viewportHeight = el.clientHeight;
    containerObserver = new ResizeObserver(() => {
      viewportHeight = el.clientHeight;
    });
    containerObserver.observe(el);
    return () => containerObserver?.disconnect();
  });

  onMount(() => {
    load();
    loadSpecies();
    loadProjects();
    listStages().then(s => stages = s).catch((e: any) => addNotification(e.message, 'error'));

    return () => {
      containerObserver?.disconnect();
      sentinelObserver?.disconnect();
    };
  });

  async function loadSpecies() {
    try { species = await listSpecies(); } catch (_e) {}
  }

  async function loadProjects() {
    try { projects = await listProjects(); } catch (_e) {}
  }

  // `reset` = true: fresh load (filters changed, initial mount) — clears the
  // accumulated buffer and starts back at page 1.
  // `reset` = false: append the next page to the buffer (virtual-scroll
  // prefetch triggered by the sentinel below).
  async function load(reset = true) {
    if (reset) {
      page = 1;
      loading = true;
      loadedSpecimens = [];
    } else {
      if (loadingMore || loading) return;
      if (totalPages > 0 && page >= totalPages) return; // no more pages
      loadingMore = true;
      page += 1;
    }
    error = null;
    try {
      let result;
      if (searchQuery || filterSpecies || filterStage || filterProject) {
        result = await searchSpecimens({
          query: searchQuery || undefined,
          species_id: filterSpecies || undefined,
          stage: filterStage || undefined,
          project_id: filterProject || undefined,
          page,
          per_page: perPage,
        });
      } else {
        result = await listSpecimens(page, perPage);
      }
      specimens = result.items;
      total = result.total;
      totalPages = result.total_pages;
      loadedSpecimens = reset ? result.items : [...loadedSpecimens, ...result.items];
    } catch (e: any) {
      error = e.message;
      addNotification(e.message, 'error');
      if (!reset) page -= 1; // roll back the optimistic page bump on failure
    } finally {
      loading = false;
      loadingMore = false;
    }
  }

  function loadNextPage() {
    load(false);
  }

  function openDetail(id: string) {
    selectedSpecimenId.set(id);
    navigateTo('specimen-detail', id);
  }

  async function handleDelete(id: string) {
    if (!confirm('Archive this specimen? It can still be found in searches.')) return;
    try {
      await deleteSpecimen(id);
      addNotification('Specimen archived', 'success');
      load();
    } catch (e: any) {
      addNotification(e.message, 'error');
    }
  }

  function handleFormDone() {
    showForm = false;
    load();
  }

  function handleSearch() {
    page = 1;
    load();
  }

  function openQr(e: MouseEvent, s: any) {
    e.stopPropagation();
    qrSpecimen = s;
  }

  // ── Batch selection helpers ────────────────────────────────────
  function toggleSelect(e: MouseEvent, id: string) {
    e.stopPropagation();
    const next = new Set(selectedIds);
    if (next.has(id)) next.delete(id); else next.add(id);
    selectedIds = next;
  }

  function toggleSelectAll(e: MouseEvent) {
    e.stopPropagation();
    if (allPageSelected) {
      // deselect all loaded rows
      const next = new Set(selectedIds);
      for (const s of loadedSpecimens) next.delete(s.id);
      selectedIds = next;
    } else {
      // select all loaded rows
      const next = new Set(selectedIds);
      for (const s of loadedSpecimens) next.add(s.id);
      selectedIds = next;
    }
  }

  function clearSelection() {
    selectedIds = new Set();
    batchAction = null;
  }


  function openBatchAction(action: 'location' | 'stage') {
    batchAction = batchAction === action ? null : action;
  }

  function composeBatchLocation(): string {
    const parts: string[] = [];
    if (batchRoom)  parts.push(`Room ${batchRoom}`);
    if (batchRack)  parts.push(`Rack ${batchRack}`);
    if (batchShelf) parts.push(`Shelf ${batchShelf}`);
    if (batchTray)  parts.push(`Tray ${batchTray}`);
    return parts.join(' / ');
  }

  async function executeBatchLocation() {
    const location = composeBatchLocation();
    if (!location) { addNotification('Select at least one location component', 'warning'); return; }
    batchLoading = true;
    try {
      const ids = Array.from(selectedIds);
      const n = await bulkUpdateLocation(ids, location);
      addNotification(`Location updated for ${n} specimen${n !== 1 ? 's' : ''}`, 'success');
      clearSelection();
      batchRoom = batchRack = batchShelf = batchTray = '';
      load();
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      batchLoading = false;
    }
  }

  async function executeBatchStage() {
    if (!batchStage) { addNotification('Select a stage', 'warning'); return; }
    batchLoading = true;
    try {
      const ids = Array.from(selectedIds);
      const n = await bulkUpdateStage(ids, batchStage);
      addNotification(`Stage updated for ${n} specimen${n !== 1 ? 's' : ''}`, 'success');
      clearSelection();
      batchStage = '';
      load();
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      batchLoading = false;
    }
  }

  async function executeBatchArchive() {
    const n = selectedIds.size;
    if (!confirm(`Archive ${n} specimen${n !== 1 ? 's' : ''}? They can still be found in searches.`)) return;
    batchLoading = true;
    try {
      const ids = Array.from(selectedIds);
      const archived = await bulkArchiveSpecimens(ids);
      addNotification(`${archived} specimen${archived !== 1 ? 's' : ''} archived`, 'success');
      clearSelection();
      load();
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      batchLoading = false;
    }
  }

  function printSummaryReport() {
    showPrintOptions = false;

    const user = get(currentUser);
    const username = (user as any)?.display_name || (user as any)?.username || 'Unknown';
    const now = new Date();
    const reportDate = now.toISOString().split('T')[0];
    const reportTime = now.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit' });

    // Aliases for terser template expressions inside the HTML string builders.
    const esc = escHtml;
    const stageFmtP = stageFmt;
    const healthText = healthLabel;

    // ── Executive Summary Statistics ───────────────────────────────────────────
    const totalShown = specimens.length;
    const quarantineCount = specimens.filter((s: any) => s.quarantine_flag).length;
    const contaminatedCount = specimens.filter((s: any) => s.has_contamination).length;
    const criticalCount = specimens.filter((s: any) => {
      const n = healthNum(s.health_status);
      return s.quarantine_flag || s.has_contamination || (n !== null && n !== -1 && n <= 1);
    }).length;

    // Stage distribution (sorted by count desc)
    const stageCounts = new Map<string, number>();
    for (const s of specimens) {
      const k = s.stage || 'unknown';
      stageCounts.set(k, (stageCounts.get(k) || 0) + 1);
    }
    const stageDistHtml = [...stageCounts.entries()]
      .sort((a, b) => b[1] - a[1])
      .map(([stage, count]) =>
        `<span class="chip">${esc(stageFmtP(stage))} <b>${count}</b></span>`
      ).join('') || '<span class="chip">—</span>';

    // Health distribution
    const hb = { dead: 0, poor: 0, fair: 0, good: 0, healthy: 0, unknown: 0 };
    for (const s of specimens) {
      const n = healthNum(s.health_status);
      if (n === null || n === -1) hb.unknown++;
      else if (n === 0) hb.dead++;
      else if (n === 1) hb.poor++;
      else if (n === 2) hb.fair++;
      else if (n === 3) hb.good++;
      else hb.healthy++;
    }
    const healthDistHtml = [
      hb.dead     > 0 ? `<span class="chip ch-dead"><b>${hb.dead}</b> Dead</span>` : '',
      hb.poor     > 0 ? `<span class="chip ch-poor"><b>${hb.poor}</b> Poor</span>` : '',
      hb.fair     > 0 ? `<span class="chip ch-fair"><b>${hb.fair}</b> Fair</span>` : '',
      hb.good     > 0 ? `<span class="chip ch-good"><b>${hb.good}</b> Good</span>` : '',
      hb.healthy  > 0 ? `<span class="chip ch-healthy"><b>${hb.healthy}</b> Healthy</span>` : '',
      hb.unknown  > 0 ? `<span class="chip"><b>${hb.unknown}</b> Unknown</span>` : '',
    ].filter(Boolean).join('') || '<span class="chip">—</span>';

    // Average health (excluding unknowns)
    const healthNums = specimens
      .map((s: any) => healthNum(s.health_status))
      .filter((n): n is number => n !== null && n !== -1);
    const avgHealth = healthNums.length > 0
      ? (healthNums.reduce((a, b) => a + b, 0) / healthNums.length).toFixed(1)
      : '—';

    // ── Filter description ─────────────────────────────────────────────────────
    const filterParts: string[] = [];
    if (searchQuery) filterParts.push(`Search: &ldquo;${esc(searchQuery)}&rdquo;`);
    if (filterStage) filterParts.push(`Stage: ${stageFmtP(filterStage)}`);
    if (filterSpecies) {
      const sp = species.find((s: any) => s.id === filterSpecies);
      if (sp) filterParts.push(`Species: ${esc(sp.species_code)}`);
    }
    if (filterProject) {
      const proj = projects.find((p: any) => p.id === filterProject);
      if (proj) filterParts.push(`Project: ${esc(proj.name)}`);
    }
    const isFiltered = filterParts.length > 0;
    const filterBar = isFiltered
      ? `<div class="filter-bar"><b>Active filters:</b> ${filterParts.join(' &nbsp;·&nbsp; ')} &nbsp;·&nbsp; Showing ${totalShown} of ${total} total records (page&nbsp;${page}/${totalPages || 1})</div>`
      : `<div class="filter-bar">All active specimens &mdash; showing ${totalShown} of ${total} total records (page&nbsp;${page}/${totalPages || 1})</div>`;

    // ── Grouping ───────────────────────────────────────────────────────────────
    type Group = { key: string; label: string; items: any[]; colorClass: string; };
    let groups: Group[];
    const groupLabel =
      printGroupBy === 'stage'  ? 'Grouped by Development Stage' :
      printGroupBy === 'health' ? 'Grouped by Health / Urgency' :
                                  'All Specimens — Flat List';

    if (printGroupBy === 'stage') {
      const stageOrder = ['explant','callus','suspension','protoplast','shoot',
        'shoot_meristem','apical_meristem','root','root_meristem','embryogenic',
        'plantlet','acclimatized','stock'];
      const map = new Map<string, any[]>();
      for (const s of specimens) {
        const k = s.stage || 'unknown';
        if (!map.has(k)) map.set(k, []);
        map.get(k)!.push(s);
      }
      groups = [...map.entries()]
        .sort((a, b) => {
          const ia = stageOrder.indexOf(a[0]), ib = stageOrder.indexOf(b[0]);
          if (ia === -1 && ib === -1) return a[0].localeCompare(b[0]);
          return (ia === -1 ? 999 : ia) - (ib === -1 ? 999 : ib);
        })
        .map(([k, items]) => ({ key: k, label: stageFmtP(k), items, colorClass: 'gh-default' }));

    } else if (printGroupBy === 'health') {
      const buckets: Record<string, any[]> = { critical: [], fair: [], good: [], pending: [] };
      for (const s of specimens) {
        const n = healthNum(s.health_status);
        if (n === null || n === -1)          buckets.pending.push(s);
        else if (n <= 1)                     buckets.critical.push(s);
        else if (n === 2)                    buckets.fair.push(s);
        else                                 buckets.good.push(s);
      }
      groups = [
        { key: 'critical', label: 'Critical — Requires Immediate Attention (Health 0–1)', items: buckets.critical, colorClass: 'gh-critical' },
        { key: 'fair',     label: 'Fair — Monitor Closely (Health 2)',                    items: buckets.fair,     colorClass: 'gh-fair' },
        { key: 'good',     label: 'Good / Healthy (Health 3–4)',                          items: buckets.good,     colorClass: 'gh-good' },
        { key: 'pending',  label: 'Unknown / Pending Health Assessment',                  items: buckets.pending,  colorClass: 'gh-default' },
      ].filter(g => g.items.length > 0);

    } else {
      groups = [{ key: 'all', label: '', items: specimens, colorClass: 'gh-default' }];
    }

    const isGrouped = printGroupBy !== 'none';

    // ── Table row builder ──────────────────────────────────────────────────────
    const buildRows = (items: any[]): string =>
      items.map((s: any) => {
        const n = healthNum(s.health_status);
        const isCrit = n !== null && n !== -1 && n <= 1;
        const status = s.quarantine_flag
          ? '<span class="tag t-warn">Quarantine</span>'
          : '<span class="tag t-ok">Active</span>';
        const contam = s.has_contamination ? ' <span class="tag t-danger">Contam.</span>' : '';
        const days = ageDays(s.initiation_date);
        const ageStr = fmtAge(s.initiation_date);
        // Flag old cultures (> 730 days) as potentially overdue for review
        const ageFlag = days !== null && days > 730 ? ' <span class="tag t-warn">Old</span>' : '';

        return `<tr class="${isCrit ? 'row-crit' : ''}">
          <td class="mono">${esc(s.accession_number)}</td>
          <td>${esc(s.species_code)}</td>
          ${!isGrouped ? `<td>${stageFmtP(s.stage)}</td>` : ''}
          <td>${esc(s.location)}</td>
          <td class="num">${esc(s.subculture_count)}</td>
          <td>${esc(s.initiation_date)}</td>
          <td class="num">${ageStr}${ageFlag}</td>
          <td class="${isCrit ? 'h-crit' : ''}">${healthText(s.health_status)}</td>
          <td>${status}${contam}</td>
        </tr>`;
      }).join('');

    // ── Group section builder ──────────────────────────────────────────────────
    const buildGroup = (g: Group): string => {
      const cnt = g.items.length;
      const gQuar   = g.items.filter((s: any) => s.quarantine_flag).length;
      const gContam = g.items.filter((s: any) => s.has_contamination).length;
      const gNums   = g.items.map((s: any) => healthNum(s.health_status))
                             .filter((n): n is number => n !== null && n !== -1);
      const gAvg    = gNums.length > 0
        ? (gNums.reduce((a, b) => a + b, 0) / gNums.length).toFixed(1)
        : null;

      const metaParts = [`${cnt} specimen${cnt !== 1 ? 's' : ''}`];
      if (gAvg !== null) metaParts.push(`avg health ${gAvg}/4`);
      if (gQuar   > 0)  metaParts.push(`<span class="meta-warn">${gQuar} quarantine</span>`);
      if (gContam > 0)  metaParts.push(`<span class="meta-danger">${gContam} contaminated</span>`);

      const header = isGrouped ? `
        <div class="group-header ${g.colorClass}">
          <span class="group-title">${esc(g.label)}</span>
          <span class="group-meta">${metaParts.join(' &nbsp;·&nbsp; ')}</span>
        </div>` : '';

      const thead = `<thead><tr>
        <th>Accession</th>
        <th>Species</th>
        ${!isGrouped ? '<th>Stage</th>' : ''}
        <th>Location</th>
        <th class="num">Passages</th>
        <th>Initiated</th>
        <th class="num">Age</th>
        <th>Health</th>
        <th>Status</th>
      </tr></thead>`;

      return `<div class="group-wrap">${header}<table>${thead}<tbody>${buildRows(g.items)}</tbody></table></div>`;
    };

    const mainHtml = groups.map(buildGroup).join('');

    // ── Print CSS ──────────────────────────────────────────────────────────────
    const printCss = `
*,*::before,*::after{margin:0;padding:0;box-sizing:border-box}
html,body{background:#fff}
body{font-family:'Segoe UI',-apple-system,Helvetica,Arial,sans-serif;font-size:9.5px;color:#0f172a;line-height:1.4}

/* ── Document header ── */
.doc-header{display:flex;align-items:flex-end;justify-content:space-between;border-bottom:2.5px solid #0f172a;padding-bottom:10px;margin-bottom:12px;gap:16px}
.doc-logo{width:56px;height:40px;border:1.5px dashed #cbd5e1;border-radius:4px;display:flex;align-items:center;justify-content:center;font-size:7.5px;color:#94a3b8;flex-shrink:0}
.doc-title-block{flex:1}
.doc-brand{font-size:20px;font-weight:900;letter-spacing:-.5px;color:#0f172a;line-height:1}
.doc-sub{font-size:10.5px;color:#475569;margin-top:3px;font-weight:600}
.doc-groupby{font-size:8.5px;color:#94a3b8;margin-top:2px;font-weight:500;font-style:italic}
.doc-meta{text-align:right;font-size:8.5px;color:#64748b;line-height:1.85;flex-shrink:0}
.doc-meta b{color:#0f172a}

/* ── Filter bar ── */
.filter-bar{font-size:8px;color:#475569;margin-bottom:14px;padding:5px 9px;background:#f8fafc;border-left:3px solid #cbd5e1}
.filter-bar b{color:#334155}

/* ── Executive summary ── */
.exec-section{margin-bottom:16px}
.exec-title{font-size:7.5px;font-weight:800;text-transform:uppercase;letter-spacing:1.5px;color:#1d4ed8;border-bottom:1.5px solid #e2e8f0;padding-bottom:4px;margin-bottom:10px}
.stat-grid{display:grid;grid-template-columns:repeat(4,1fr);gap:9px;margin-bottom:12px}
.stat-box{border:1px solid #e2e8f0;border-radius:5px;padding:9px 10px;text-align:center}
.stat-num{font-size:26px;font-weight:900;color:#0f172a;line-height:1}
.stat-lbl{font-size:7.5px;color:#64748b;text-transform:uppercase;letter-spacing:.6px;margin-top:4px}
.stat-box.s-attn{border-color:#fca5a5}.stat-box.s-attn .stat-num{color:#dc2626}
.stat-box.s-quar{border-color:#fcd34d}.stat-box.s-quar .stat-num{color:#b45309}
.stat-box.s-contam{border-color:#f0abfc}.stat-box.s-contam .stat-num{color:#7e22ce}
.stat-box.s-health{border-color:#86efac}.stat-box.s-health .stat-num{color:#15803d}

.dist-row{display:flex;align-items:flex-start;gap:8px;margin-bottom:5px}
.dist-lbl{font-size:7.5px;font-weight:700;text-transform:uppercase;letter-spacing:.5px;color:#94a3b8;min-width:52px;padding-top:3px;flex-shrink:0}
.chips{display:flex;flex-wrap:wrap;gap:4px}
.chip{display:inline-block;padding:2px 7px;border-radius:3px;font-size:8px;background:#f1f5f9;color:#334155}
.chip b{font-weight:700}
.ch-dead{background:#fee2e2;color:#7f1d1d}.ch-poor{background:#ffedd5;color:#7c2d12}
.ch-fair{background:#fef9c3;color:#713f12}.ch-good{background:#dcfce7;color:#14532d}
.ch-healthy{background:#d1fae5;color:#065f46}

/* ── Group sections ── */
.group-wrap{margin-bottom:18px;page-break-inside:avoid}
.group-header{display:flex;justify-content:space-between;align-items:center;padding:7px 10px;border-left:3.5px solid #334155;margin-bottom:0}
.gh-default{background:#f1f5f9;border-color:#334155}
.gh-critical{background:#fff1f2;border-color:#dc2626}
.gh-fair{background:#fffbeb;border-color:#d97706}
.gh-good{background:#f0fdf4;border-color:#16a34a}
.group-title{font-size:10.5px;font-weight:700;color:#0f172a}
.group-meta{font-size:8px;color:#475569}
.meta-warn{color:#92400e;font-weight:600}
.meta-danger{color:#991b1b;font-weight:600}

/* ── Data table ── */
table{width:100%;border-collapse:collapse;font-size:8.5px;margin-top:0}
thead{display:table-header-group}
th{background:#1e293b;color:#e2e8f0;font-weight:700;text-align:left;padding:5px 8px;white-space:nowrap;font-size:7.5px;letter-spacing:.4px;text-transform:uppercase}
th.num{text-align:center}
td{padding:4px 8px;border-bottom:1px solid #f1f5f9;vertical-align:middle;color:#1e293b}
tr:nth-child(even) td{background:#f8fafc}
tr.row-crit td{background:#fff5f5}
tr.row-crit:nth-child(even) td{background:#fee2e2}
tr{page-break-inside:avoid}
.num{text-align:center}
.mono{font-family:'Consolas','SF Mono',monospace;font-size:8px;font-weight:700;letter-spacing:-.2px}
.h-crit{color:#dc2626;font-weight:700}

/* ── Tags ── */
.tag{display:inline-block;padding:1px 5px;border-radius:3px;font-size:7.5px;font-weight:700;line-height:1.5}
.t-ok{background:#dcfce7;color:#166534}
.t-warn{background:#fef3c7;color:#92400e}
.t-danger{background:#fee2e2;color:#991b1b}

/* ── Footer ── */
.doc-footer{margin-top:18px;border-top:1px solid #e2e8f0;padding-top:7px;display:flex;justify-content:space-between;align-items:center;font-size:8px;color:#94a3b8}
.doc-footer span+span{font-style:italic}
.page-num::after{content:"Page " counter(page) " of " counter(pages)}
`.trim();

    // ── HTML body ──────────────────────────────────────────────────────────────
    const bodyHtml = `
<div class="doc-header">
  <div class="doc-logo">LOGO</div>
  <div class="doc-title-block">
    <div class="doc-brand">SteloPTC</div>
    <div class="doc-sub">Specimen Inventory Report</div>
    <div class="doc-groupby">${groupLabel}</div>
  </div>
  <div class="doc-meta">
    <div><b>Generated:</b> ${reportDate} &nbsp;${reportTime}</div>
    <div><b>Prepared by:</b> ${esc(username)}</div>
    <div><b>Showing:</b> ${totalShown} of ${total} active records</div>
    <div><b>Page:</b> ${page} of ${totalPages || 1}</div>
  </div>
</div>

${filterBar}

<div class="exec-section">
  <div class="exec-title">Executive Summary</div>
  <div class="stat-grid">
    <div class="stat-box">
      <div class="stat-num">${totalShown}</div>
      <div class="stat-lbl">Specimens Shown</div>
    </div>
    <div class="stat-box${criticalCount > 0 ? ' s-attn' : ''}">
      <div class="stat-num">${criticalCount}</div>
      <div class="stat-lbl">Needs Attention</div>
    </div>
    <div class="stat-box${quarantineCount > 0 ? ' s-quar' : ''}">
      <div class="stat-num">${quarantineCount}</div>
      <div class="stat-lbl">In Quarantine</div>
    </div>
    <div class="stat-box${contaminatedCount > 0 ? ' s-contam' : ' s-health'}">
      <div class="stat-num">${contaminatedCount > 0 ? contaminatedCount : avgHealth}</div>
      <div class="stat-lbl">${contaminatedCount > 0 ? 'Contaminated' : 'Avg Health Score'}</div>
    </div>
  </div>
  <div class="dist-row">
    <span class="dist-lbl">By Stage</span>
    <span class="chips">${stageDistHtml}</span>
  </div>
  <div class="dist-row">
    <span class="dist-lbl">By Health</span>
    <span class="chips">${healthDistHtml}</span>
  </div>
</div>

${mainHtml}

<div class="doc-footer">
  <span>SteloPTC &nbsp;·&nbsp; Tissue Culture Management System &nbsp;·&nbsp; ${reportDate}</span>
  <span class="page-num"></span>
</div>`.trim();

    // ── Print delivery ─────────────────────────────────────────────────────────
    deliverPrint({
      frameId: 'ptc-summary-frame',
      title: `Specimen Inventory Report – ${reportDate}`,
      css: printCss,
      body: bodyHtml,
      onError: (msg) => addNotification(msg, 'error'),
    });
  }
</script>

<div>
  <div class="page-header">
    <h1>Specimens ({total})</h1>
    <div class="header-actions">
      <button class="btn btn-scan" onclick={() => (showScanner = true)}>
        &#128247; Scan QR <Tooltip text="Open camera to scan a QR code label and jump directly to the matching specimen" position="bottom" />
      </button>
      <div class="print-summary-wrap">
        <button
          class="btn btn-sm btn-print-summary"
          class:active={showPrintOptions}
          onclick={() => (showPrintOptions = !showPrintOptions)}
          title="Configure and print a professional lab report of the currently visible specimens"
        >
          &#128438; Print Summary
          <Tooltip text="Opens print options — choose how specimens are grouped in the report" position="bottom" />
        </button>
        {#if showPrintOptions}
          <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
          <div
            class="print-options-panel"
            role="dialog"
            aria-label="Print report options"
            aria-modal="true"
            tabindex="-1"
            onkeydown={(e) => { if (e.key === 'Escape') { e.stopPropagation(); showPrintOptions = false; } }}
          >
            <div class="pop-title">Report Options</div>
            <div class="pop-field" role="radiogroup" aria-labelledby="pop-group-label">
              <div class="pop-label" id="pop-group-label">Group specimens by</div>
              <label class="pop-radio">
                <input type="radio" bind:group={printGroupBy} value="stage" />
                Development Stage
              </label>
              <label class="pop-radio">
                <input type="radio" bind:group={printGroupBy} value="health" />
                Health / Urgency
              </label>
              <label class="pop-radio">
                <input type="radio" bind:group={printGroupBy} value="none" />
                No grouping &mdash; flat list
              </label>
            </div>
            <div class="pop-hint">
              Showing {specimens.length} specimen{specimens.length !== 1 ? 's' : ''} on this page
            </div>
            <div class="pop-actions">
              <button class="btn btn-sm" onclick={() => (showPrintOptions = false)}>Cancel</button>
              <button class="btn btn-primary btn-sm" onclick={printSummaryReport}>&#128438; Generate Report</button>
            </div>
          </div>
        {/if}
      </div>
      {#if $currentUser?.role !== 'guest'}
        <button class="btn btn-primary" onclick={() => (showForm = true)}>+ New Specimen <Tooltip text="Register a new tissue culture specimen — auto-generates an accession number on save" position="bottom" /></button>
      {/if}
    </div>
  </div>

  <div class="filters card" style="margin-bottom:16px;">
    <div class="form-row-3">
      <div>
        <input type="text" placeholder="Search accession, notes, location..." bind:value={searchQuery} onkeydown={(e) => e.key === 'Enter' && handleSearch()} title="Search by accession number, notes, or storage location — press Enter to apply" />
      </div>
      <div>
        <select bind:value={filterSpecies} onchange={handleSearch} title="Filter specimens by species">
          <option value="">All species</option>
          {#each species as sp}
            <option value={sp.id}>{sp.species_code} - {sp.genus} {sp.species_name}</option>
          {/each}
        </select>
      </div>
      <div style="display:flex;gap:8px;">
        <select bind:value={filterStage} onchange={handleSearch} title="Filter specimens by development stage">
          <option value="">All stages</option>
          {#each stages as s}
            <option value={s.code}>{s.label}</option>
          {/each}
        </select>
        <select bind:value={filterProject} onchange={handleSearch} title="Filter specimens by project">
          <option value="">All projects</option>
          {#each projects as proj}
            <option value={proj.id}>{proj.name}</option>
          {/each}
        </select>
        <button class="btn btn-sm" onclick={handleSearch} title="Apply current search query and filters">Search</button>
      </div>
    </div>
  </div>

  {#if showForm}
    <div class="card" style="margin-bottom:16px;">
      <SpecimenForm onclose={() => (showForm = false)} onsave={handleFormDone} />
    </div>
  {/if}

  <DataState {loading} {error} rows={6} cols={5} onretry={() => load()}>
    {#if loadedSpecimens.length === 0 && !searchQuery && !filterSpecies && !filterStage && !filterProject && total === 0}
      <FirstRun
        onAddSpecimen={() => { showForm = true; window.scrollTo({ top: 0, behavior: 'smooth' }); }}
        onDemoLoaded={() => { load(); loadSpecies(); }}
      />
    {:else if loadedSpecimens.length === 0}
      <DataState
        empty={true}
        emptyIcon="🔍"
        emptyTitle="No specimens found"
        emptyMessage="Try adjusting your search or filters."
      />
    {:else}
    {#snippet colgroup()}
      <colgroup>
        <col class="col-check" />
        <col class="col-accession" />
        <col class="col-species" />
        <col class="col-stage" />
        <col class="col-location" />
        <col class="col-passages" />
        <col class="col-health" />
        <col class="col-status" />
        <col class="col-initiated" />
        <col class="col-action" />
      </colgroup>
    {/snippet}
    <div class="card table-card">
      <table class="vscroll-header-table">
        {@render colgroup()}
        <thead>
          <tr>
            <th class="check-col" title="Select specimens for bulk actions">
              <input
                type="checkbox"
                style="width:auto;margin:0;"
                checked={allPageSelected}
                indeterminate={someSelected}
                onclick={toggleSelectAll}
                title={allPageSelected ? 'Deselect all loaded rows' : 'Select all loaded rows'}
              />
            </th>
            <th title="Unique accession number assigned to this specimen">Accession</th>
            <th title="Scientific species this specimen belongs to">Species</th>
            <th title="Current development stage of the specimen">Stage</th>
            <th title="Physical storage location within the facility">Location</th>
            <th title="Number of subcultures (transfers to fresh media) performed">Passages</th>
            <th title="Current health rating (0 = Dead, 4 = Healthy)">Health</th>
            <th title="Quarantine or active status">Status</th>
            <th title="Date the specimen culture was first initiated">Initiated</th>
            <th class="action-col" title="Row actions: view QR code, archive specimen"></th>
          </tr>
        </thead>
      </table>

      <!-- Virtual-scroll viewport: fixed height, scrolls internally. Only
           rows in [startIndex, endIndex) are ever mounted in the DOM — the
           `.vscroll-spacer` below is sized to the full (unrendered) row
           count so the scrollbar behaves as if every row were present. -->
      <div
        class="vscroll-viewport"
        bind:this={scrollContainer}
        onscroll={handleScroll}
      >
        <div class="vscroll-spacer" style="height:{totalScrollHeight}px;">
          <table class="vscroll-table">
            {@render colgroup()}
            <tbody>
              {#each visibleSpecimens as s, i (s.id)}
                <tr
                  class="clickable vscroll-row"
                  class:selected={selectedIds.has(s.id)}
                  style="top:{(startIndex + i) * ROW_HEIGHT}px; height:{ROW_HEIGHT}px;"
                  onclick={() => openDetail(s.id)}
                >
                  <td class="check-col" onclick={(e) => toggleSelect(e, s.id)}>
                    <input
                      type="checkbox"
                      style="width:auto;margin:0;"
                      checked={selectedIds.has(s.id)}
                      onclick={(e) => toggleSelect(e, s.id)}
                      title="Select this specimen for bulk actions"
                    />
                  </td>
                  <td><strong>{s.accession_number}</strong></td>
                  <td>{s.species_code || '—'}</td>
                  <td><span class="badge badge-blue" title="Development stage: {s.stage}">{stageFmt(s.stage)}</span></td>
                  <td>{s.location || '—'}</td>
                  <td>{s.subculture_count}</td>
                  <td>{healthLabel(s.health_status)}</td>
                  <td>
                    {#if s.quarantine_flag}
                      <span class="badge badge-red" title="This specimen is under quarantine restrictions">Quarantine</span>
                    {:else}
                      <span class="badge badge-green" title="This specimen is active and cleared for normal handling">Active</span>
                    {/if}
                    {#if s.has_contamination}
                      <span class="badge badge-red" title="One or more subcultures for this specimen have been flagged as contaminated">Contaminated</span>
                    {/if}
                  </td>
                  <td>{s.initiation_date}</td>
                  <td class="action-col">
                    <div class="row-actions">
                      <button class="btn btn-sm btn-qr" onclick={(e) => openQr(e, s)} title="View and print the QR code label for this specimen">
                        &#9641; QR
                      </button>
                      {#if $currentUser?.role === 'admin' || $currentUser?.role === 'supervisor'}
                        <button class="btn btn-sm btn-danger" onclick={(e) => { e.stopPropagation(); handleDelete(s.id); }} title="Archive this specimen (soft delete — record is preserved)">Archive</button>
                      {/if}
                    </div>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>

          <!-- Sentinel positioned 80% through the loaded buffer — when it
               scrolls into view we prefetch the next page (if any). -->
          {#if totalPages > 1 && page < totalPages}
            <div
              bind:this={sentinelEl}
              class="vscroll-sentinel"
              style="top:{sentinelOffset}px;"
              aria-hidden="true"
            ></div>
          {/if}
        </div>
      </div>
    </div>

    <div class="pagination">
      <span title="Rows loaded into memory vs. total matching records">
        Showing {loadedSpecimens.length} of {total}
        {#if totalPages > 0}&nbsp;&middot;&nbsp;Page {Math.min(page, totalPages)} of {totalPages}{/if}
        {#if loadingMore}&nbsp;&middot;&nbsp;Loading more…{/if}
      </span>
    </div>
    {/if}
  </DataState>
</div>

<!-- ── Batch Action Bar ─────────────────────────────────────────── -->
{#if selectedIds.size > 0}
  <div class="batch-bar" role="toolbar" aria-label="Batch actions for selected specimens">
    <span class="batch-count">{selectedIds.size} selected</span>
    <button class="batch-clear" onclick={clearSelection} title="Clear selection">✕</button>
    <div class="batch-divider"></div>

    {#if $currentUser?.role !== 'guest'}
      <button
        class="batch-btn"
        class:active={batchAction === 'location'}
        onclick={() => openBatchAction('location')}
        title="Move all selected specimens to a new location"
        disabled={batchLoading}
      >&#128205; Transfer Location</button>

      <button
        class="batch-btn"
        class:active={batchAction === 'stage'}
        onclick={() => openBatchAction('stage')}
        title="Update the development stage for all selected specimens"
        disabled={batchLoading}
      >&#128260; Update Stage</button>
    {/if}

    {#if $currentUser?.role === 'admin' || $currentUser?.role === 'supervisor'}
      <button
        class="batch-btn batch-btn-danger"
        onclick={executeBatchArchive}
        title="Archive all selected specimens (soft delete — records are preserved)"
        disabled={batchLoading}
      >&#128465; Archive</button>
    {/if}

    {#if batchAction === 'location'}
      <div class="batch-form">
        <select bind:value={batchRoom} title="Room">
          <option value="">Room…</option>
          {#each rooms as r}<option value={r}>{r}</option>{/each}
        </select>
        <select bind:value={batchRack} title="Rack">
          <option value="">Rack…</option>
          {#each racks as r}<option value={r}>{r}</option>{/each}
        </select>
        <select bind:value={batchShelf} title="Shelf">
          <option value="">Shelf…</option>
          {#each shelves as r}<option value={r}>{r}</option>{/each}
        </select>
        <select bind:value={batchTray} title="Tray">
          <option value="">Tray…</option>
          {#each trays as r}<option value={r}>{r}</option>{/each}
        </select>
        <button class="batch-apply" onclick={executeBatchLocation} disabled={batchLoading}>
          {batchLoading ? 'Moving…' : 'Apply'}
        </button>
      </div>
    {:else if batchAction === 'stage'}
      <div class="batch-form">
        <select bind:value={batchStage} title="New development stage for selected specimens">
          <option value="">Stage…</option>
          {#each stages.filter(s => !s.is_terminal) as s}
            <option value={s.code}>{s.label}</option>
          {/each}
        </select>
        <button class="batch-apply" onclick={executeBatchStage} disabled={batchLoading}>
          {batchLoading ? 'Updating…' : 'Apply'}
        </button>
      </div>
    {/if}
  </div>
{/if}

<!-- QR Code Modal -->
{#if qrSpecimen}
  <QrModal specimen={qrSpecimen} onclose={() => (qrSpecimen = null)} />
{/if}

<!-- QR Scanner Modal -->
{#if showScanner}
  <QrScanner onclose={() => (showScanner = false)} />
{/if}

<style>
  .clickable { cursor: pointer; }
  .clickable:hover td { background: #eff6ff !important; }
  :global(.dark) .clickable:hover td { background: #1e3a5f !important; }
  .clickable.selected td { background: #dbeafe !important; }
  :global(.dark) .clickable.selected td { background: #1e3a8f !important; }

  .check-col {
    width: 36px;
    text-align: center;
    padding: 0 8px;
  }

  /* Explicit column widths shared by the header table and the virtualized
     body table (via matching <colgroup>s) so the two independently-scrolled
     tables stay pixel-aligned regardless of cell content. */
  .col-check     { width: 52px; }
  .col-accession { width: 14%; }
  .col-species   { width: 10%; }
  .col-stage     { width: 12%; }
  .col-location  { width: 16%; }
  .col-passages  { width: 9%; }
  .col-health    { width: 10%; }
  .col-status    { width: 14%; }
  .col-initiated { width: 11%; }
  .col-action    { width: 140px; }

  .header-actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    align-items: center;
  }

  /* ── Print options ── */
  .print-summary-wrap {
    position: relative;
  }

  .btn-print-summary {
    background: #f5f3ff;
    color: #5b21b6;
    border-color: #c4b5fd;
  }
  .btn-print-summary:hover,
  .btn-print-summary.active { background: #ede9fe; border-color: #a78bfa; }
  :global(.dark) .btn-print-summary { background: rgba(139,92,246,0.12); color: #a78bfa; border-color: #5b21b6; }

  .print-options-panel {
    position: absolute;
    top: calc(100% + 8px);
    right: 0;
    background: #fff;
    border: 1px solid #e2e8f0;
    border-radius: 10px;
    padding: 16px;
    box-shadow: 0 8px 32px rgba(0,0,0,0.14);
    z-index: 300;
    min-width: 230px;
    animation: popIn 0.12s ease;
  }
  :global(.dark) .print-options-panel {
    background: #1e293b;
    border-color: #334155;
    box-shadow: 0 8px 32px rgba(0,0,0,0.4);
  }
  @keyframes popIn {
    from { opacity: 0; transform: translateY(-6px) scale(0.97); }
    to   { opacity: 1; transform: translateY(0) scale(1); }
  }

  .pop-title {
    font-size: 12px;
    font-weight: 700;
    color: #0f172a;
    margin-bottom: 12px;
    padding-bottom: 8px;
    border-bottom: 1px solid #e2e8f0;
  }
  :global(.dark) .pop-title { color: #f1f5f9; border-color: #334155; }

  .pop-label {
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: #6b7280;
    margin-bottom: 6px;
  }

  .pop-field { margin-bottom: 12px; }

  .pop-radio {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: #374151;
    cursor: pointer;
    padding: 5px 4px;
    border-radius: 5px;
    transition: background 0.1s;
  }
  .pop-radio:hover { background: #f8fafc; }
  :global(.dark) .pop-radio { color: #cbd5e1; }
  :global(.dark) .pop-radio:hover { background: #0f172a; }
  .pop-radio input[type="radio"] { width: auto; margin: 0; accent-color: #7c3aed; }

  .pop-hint {
    font-size: 11px;
    color: #9ca3af;
    margin-bottom: 12px;
    padding: 5px 4px;
    background: #f8fafc;
    border-radius: 4px;
    text-align: center;
  }
  :global(.dark) .pop-hint { background: #0f172a; color: #64748b; }

  .pop-actions {
    display: flex;
    gap: 6px;
    justify-content: flex-end;
  }

  .btn-scan {
    background: #0f172a;
    color: #94a3b8;
    border-color: #334155;
    font-size: 13px;
    gap: 6px;
  }
  .btn-scan:hover { background: #1e293b; color: #e2e8f0; }
  :global(.dark) .btn-scan { background: #334155; color: #e2e8f0; border-color: #475569; }

  .btn-qr {
    background: #f0fdf4;
    color: #15803d;
    border-color: #86efac;
    font-size: 11px;
    padding: 4px 8px;
    min-height: 32px;
  }
  .btn-qr:hover { background: #dcfce7; }
  :global(.dark) .btn-qr { background: rgba(34,197,94,0.1); color: #4ade80; border-color: #166534; }

  .table-card { overflow-x: auto; }

  .vscroll-header-table { table-layout: fixed; }

  /* ── Virtual scroll ──────────────────────────────────────────
     The viewport is a fixed-height scroll container; the spacer inside it
     is sized to `totalRows * ROW_HEIGHT` so the native scrollbar behaves
     exactly as if every row were rendered. Only the rows within the
     current window are mounted, each absolutely positioned at
     `index * ROW_HEIGHT` — this keeps DOM node count bounded regardless
     of how many specimens are loaded. */
  .vscroll-viewport {
    max-height: 65vh;
    overflow-y: auto;
    position: relative;
  }
  .vscroll-spacer {
    position: relative;
    width: 100%;
  }
  .vscroll-table {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    table-layout: fixed;
  }
  .vscroll-row {
    position: absolute;
    left: 0;
    width: 100%;
    display: table;
    table-layout: fixed;
    box-sizing: border-box;
    overflow: hidden;
  }
  .vscroll-row td {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    /* Tighter vertical padding than the default :global(td) rule so cell
       content (badges, the 32px-tall QR/Archive buttons) fits within the
       fixed ROW_HEIGHT used for absolute positioning. */
    padding-top: 4px;
    padding-bottom: 4px;
    vertical-align: middle;
  }
  .vscroll-row .row-actions {
    flex-wrap: nowrap;
  }
  .vscroll-sentinel {
    position: absolute;
    left: 0;
    width: 1px;
    height: 1px;
    pointer-events: none;
  }

  @media (max-width: 1024px) {
    .vscroll-viewport { max-height: 55vh; }
  }

  .row-actions {
    display: flex;
    gap: 4px;
    align-items: center;
  }

  .action-col { white-space: nowrap; }

  .pagination {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 16px;
    margin-top: 16px;
    font-size: 13px;
  }

  /* ── Batch Action Bar ── */
  .batch-bar {
    position: fixed;
    bottom: 0;
    left: 0;
    right: 0;
    background: #1e293b;
    border-top: 1px solid #334155;
    color: #e2e8f0;
    padding: 10px 20px;
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
    z-index: 500;
    box-shadow: 0 -4px 16px rgba(0,0,0,0.4);
    animation: slideUp 0.15s ease;
  }
  :global(.dark) .batch-bar { background: #0f172a; border-top-color: #1e293b; }

  @keyframes slideUp {
    from { transform: translateY(100%); opacity: 0; }
    to   { transform: translateY(0);    opacity: 1; }
  }

  .batch-count {
    font-size: 13px;
    font-weight: 600;
    color: #94a3b8;
    white-space: nowrap;
  }

  .batch-clear {
    background: none;
    border: none;
    color: #64748b;
    cursor: pointer;
    font-size: 14px;
    padding: 2px 6px;
    border-radius: 4px;
    line-height: 1;
    transition: color 0.1s;
  }
  .batch-clear:hover { color: #e2e8f0; background: #334155; }

  .batch-divider {
    width: 1px;
    height: 24px;
    background: #334155;
    margin: 0 4px;
  }

  .batch-btn {
    background: #334155;
    color: #e2e8f0;
    border: 1px solid #475569;
    border-radius: 6px;
    padding: 6px 14px;
    font-size: 12px;
    cursor: pointer;
    transition: background 0.1s;
    white-space: nowrap;
  }
  .batch-btn:hover:not(:disabled) { background: #475569; }
  .batch-btn.active { background: #1d4ed8; border-color: #3b82f6; color: #fff; }
  .batch-btn-danger { background: #7f1d1d; border-color: #991b1b; }
  .batch-btn-danger:hover:not(:disabled) { background: #991b1b; }
  .batch-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  .batch-form {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }
  .batch-form select {
    background: #334155;
    color: #e2e8f0;
    border: 1px solid #475569;
    border-radius: 6px;
    padding: 5px 10px;
    font-size: 12px;
    min-height: 32px;
  }

  .batch-apply {
    background: #1d4ed8;
    color: #fff;
    border: 1px solid #2563eb;
    border-radius: 6px;
    padding: 6px 16px;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.1s;
    white-space: nowrap;
  }
  .batch-apply:hover:not(:disabled) { background: #2563eb; }
  .batch-apply:disabled { opacity: 0.5; cursor: not-allowed; }

  /* ── Mobile ── */
  @media (max-width: 768px) {
    .header-actions { gap: 6px; }
    .btn-scan { min-height: 44px; font-size: 14px; }
    .btn-qr   { min-height: 40px; font-size: 12px; }
    .batch-bar { padding: 10px 12px; gap: 6px; }
    .batch-btn { padding: 8px 12px; font-size: 13px; min-height: 40px; }
    .batch-form select { min-height: 40px; font-size: 13px; }
    .batch-apply { min-height: 40px; font-size: 13px; }
  }
</style>
