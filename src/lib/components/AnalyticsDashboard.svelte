<script lang="ts">
  import { onMount } from 'svelte';
  import * as XLSX from 'xlsx';
  import {
    getSpecimenGrowthRate,
    getSubcultureFrequencyTrend,
    getContaminationRateTrend,
    getPassageSuccessRate,
    getMediaBatchEfficiency,
    getStrainPerformance,
    getCryoUtilization,
    getTechnicianActivity,
    getAnalyticsKpiSummary,
    getAnalyticsPanelConfig,
    setAnalyticsPanelConfig,
    listSpecies,
  } from '../api';
  import type { AnalyticsTimeRange } from '../api';
  import { addNotification } from '../stores/app';
  import { currentUser } from '../stores/auth';
  import { datestamp } from '../utils';
  import DataState from './DataState.svelte';

  type Point = { bucket: string; value: number };

  type PanelKey =
    | 'growth'
    | 'subculture'
    | 'contamination'
    | 'passageSuccess'
    | 'mediaEfficiency'
    | 'strainPerformance'
    | 'cryoUtilization'
    | 'technicianActivity';

  const PANEL_LABELS: Record<PanelKey, string> = {
    growth: 'Specimen Growth Rate',
    subculture: 'Subculture Frequency Trend',
    contamination: 'Contamination Rate Trend',
    passageSuccess: 'Passage Success Rate',
    mediaEfficiency: 'Media Batch Efficiency',
    strainPerformance: 'Strain Performance',
    cryoUtilization: 'Cryo Utilization',
    technicianActivity: 'Technician Activity',
  };

  const DEFAULT_PANELS: Record<PanelKey, boolean> = {
    growth: true,
    subculture: true,
    contamination: true,
    passageSuccess: true,
    mediaEfficiency: true,
    strainPerformance: true,
    cryoUtilization: true,
    technicianActivity: true,
  };

  const TIME_RANGES: { value: AnalyticsTimeRange; label: string }[] = [
    { value: '30d', label: '30 Days' },
    { value: '90d', label: '90 Days' },
    { value: '1y', label: '1 Year' },
    { value: 'all', label: 'All Time' },
  ];

  const isSupervisorOrAdmin = $derived(
    $currentUser?.role === 'admin' || $currentUser?.role === 'supervisor',
  );

  // ── Top-level state ─────────────────────────────────────────────────────────
  let timeRange = $state<AnalyticsTimeRange>('90d');
  let panels = $state<Record<PanelKey, boolean>>({ ...DEFAULT_PANELS });
  let showCustomize = $state(false);

  let loadingKpi = $state(true);
  let kpiError = $state('');
  let kpi = $state<any>(null);

  let loadingSeries = $state(true);
  let seriesError = $state('');

  let growthData = $state<Point[]>([]);
  let subcultureData = $state<Point[]>([]);
  let contaminationData = $state<Point[]>([]);
  let passageSuccess = $state<any>(null);
  let mediaEfficiency = $state<any[]>([]);
  let cryoUtilization = $state<any[]>([]);
  let technicianActivity = $state<any[]>([]);

  let species = $state<any[]>([]);
  let selectedSpeciesId = $state('');
  let strainData = $state<any[]>([]);
  let loadingStrains = $state(false);
  let strainError = $state('');
  let strainSort = $state<{ key: string; dir: 1 | -1 }>({ key: 'strain_name', dir: 1 });

  let exporting = $state(false);

  // ── Lifecycle ────────────────────────────────────────────────────────────────
  onMount(() => {
    loadPanelConfig();
    loadKpi();
    loadSeries();
    loadSpecies();
  });

  // Refetch every time-series panel whenever the global time range changes.
  $effect(() => {
    // reference so the effect re-runs on change
    const _tr = timeRange;
    loadSeries();
  });

  // Refetch strain performance whenever the selected species changes.
  $effect(() => {
    const spId = selectedSpeciesId;
    if (!spId) {
      strainData = [];
      return;
    }
    loadStrainPerformance(spId);
  });

  async function loadPanelConfig() {
    try {
      const raw = await getAnalyticsPanelConfig();
      const parsed = raw ? JSON.parse(raw) : {};
      panels = { ...DEFAULT_PANELS, ...parsed };
    } catch {
      panels = { ...DEFAULT_PANELS };
    }
  }

  async function loadKpi() {
    loadingKpi = true;
    kpiError = '';
    try {
      kpi = await getAnalyticsKpiSummary();
    } catch (e: any) {
      kpiError = e.message || 'Failed to load KPI summary';
    } finally {
      loadingKpi = false;
    }
  }

  async function loadSeries() {
    loadingSeries = true;
    seriesError = '';
    try {
      const [growth, subculture, contamination, passage, media, cryo, techActivity] = await Promise.all([
        getSpecimenGrowthRate(timeRange),
        getSubcultureFrequencyTrend(timeRange),
        getContaminationRateTrend(timeRange),
        getPassageSuccessRate(timeRange),
        getMediaBatchEfficiency(timeRange),
        getCryoUtilization(),
        isSupervisorOrAdmin ? getTechnicianActivity(timeRange).catch(() => []) : Promise.resolve([]),
      ]);
      growthData = growth;
      subcultureData = subculture;
      contaminationData = contamination;
      passageSuccess = passage;
      mediaEfficiency = media;
      cryoUtilization = cryo;
      technicianActivity = techActivity;
    } catch (e: any) {
      seriesError = e.message || 'Failed to load analytics data';
    } finally {
      loadingSeries = false;
    }
  }

  async function loadSpecies() {
    try {
      species = await listSpecies();
    } catch (e: any) {
      addNotification(e.message, 'error');
    }
  }

  async function loadStrainPerformance(speciesId: string) {
    loadingStrains = true;
    strainError = '';
    try {
      strainData = await getStrainPerformance(speciesId);
    } catch (e: any) {
      strainError = e.message || 'Failed to load strain performance';
    } finally {
      loadingStrains = false;
    }
  }

  async function togglePanel(key: PanelKey) {
    panels = { ...panels, [key]: !panels[key] };
    try {
      await setAnalyticsPanelConfig(JSON.stringify(panels));
    } catch (e: any) {
      addNotification(e.message, 'error');
    }
  }

  function sortStrains(key: string) {
    if (strainSort.key === key) {
      strainSort = { key, dir: strainSort.dir === 1 ? -1 : 1 };
    } else {
      strainSort = { key, dir: 1 };
    }
  }

  const sortedStrains = $derived.by(() => {
    const { key, dir } = strainSort;
    const rows = [...strainData];
    rows.sort((a, b) => {
      const av = a[key];
      const bv = b[key];
      if (av === null || av === undefined) return dir;
      if (bv === null || bv === undefined) return -dir;
      if (typeof av === 'string' || typeof bv === 'string') {
        return String(av).localeCompare(String(bv)) * dir;
      }
      return (av - bv) * dir;
    });
    return rows;
  });

  function speciesLabel(sp: any): string {
    return `${sp.species_code} - ${sp.genus} ${sp.species_name}`;
  }

  function fmtNum(v: number | null | undefined, digits = 1): string {
    if (v === null || v === undefined || isNaN(v)) return '—';
    return v.toFixed(digits);
  }

  function growthDelta(): { up: boolean; delta: number; pct: number } | null {
    if (!kpi) return null;
    const cur = kpi.new_specimens_this_month ?? 0;
    const prev = kpi.new_specimens_last_month ?? 0;
    const delta = cur - prev;
    const pct = prev === 0 ? (cur === 0 ? 0 : 100) : (delta / prev) * 100;
    return { up: delta >= 0, delta, pct };
  }

  // ── Chart helpers (hand-rolled inline SVG, 0-100 viewBox) ───────────────────
  // Guards against empty arrays and divide-by-zero when there's a single point
  // or when all values are equal.
  function computeScale(pts: Point[]) {
    const values = pts.map((p) => p.value);
    const max = values.length ? Math.max(...values) : 0;
    const min = values.length ? Math.min(...values) : 0;
    const range = max - min || 1; // avoid divide-by-zero when flat/single-point
    return { max, min, range };
  }

  function linePoints(pts: Point[]): string {
    if (pts.length === 0) return '';
    const { min, range } = computeScale(pts);
    const n = pts.length;
    return pts
      .map((p, i) => {
        const x = n === 1 ? 50 : (i / (n - 1)) * 100;
        const y = 100 - ((p.value - min) / range) * 100;
        return `${x},${y}`;
      })
      .join(' ');
  }

  function barRects(pts: Point[]): { x: number; y: number; w: number; h: number; value: number; bucket: string }[] {
    if (pts.length === 0) return [];
    const { min, range } = computeScale(pts);
    const n = pts.length;
    const gap = 2;
    const w = 100 / n - gap;
    return pts.map((p, i) => {
      const h = ((p.value - min) / range) * 90 + 10; // keep a visible sliver at min
      return {
        x: i * (100 / n) + gap / 2,
        y: 100 - h,
        w: Math.max(w, 1),
        h,
        value: p.value,
        bucket: p.bucket,
      };
    });
  }

  // ── Export ───────────────────────────────────────────────────────────────────
  function triggerDownload(blob: Blob, filename: string) {
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    a.click();
    URL.revokeObjectURL(url);
  }

  function reportHeaderRows(title: string): any[][] {
    return [
      ['SteloPTC — Analytics & Reporting'],
      [title],
      [`Generated: ${datestamp()}`],
      [],
    ];
  }

  function seriesSheet(title: string, pts: Point[]): XLSX.WorkSheet {
    const rows: any[][] = [
      ...reportHeaderRows(title),
      ['Period', 'Value'],
      ...pts.map((p) => [p.bucket, p.value]),
    ];
    return XLSX.utils.aoa_to_sheet(rows);
  }

  function mediaEfficiencySheet(): XLSX.WorkSheet {
    const rows: any[][] = [
      ...reportHeaderRows('Media Batch Efficiency'),
      ['Batch ID', 'Name', 'Specimens Supported', 'Waste Rate %'],
      ...mediaEfficiency.map((m) => [m.batch_id, m.name, m.specimens_supported, m.waste_rate_pct]),
    ];
    return XLSX.utils.aoa_to_sheet(rows);
  }

  function strainPerformanceSheet(): XLSX.WorkSheet {
    const rows: any[][] = [
      ...reportHeaderRows('Strain Performance'),
      ['Strain', 'Mean Health', 'Total Specimens', 'Avg Days Between Passages', 'Best Performer Rate %'],
      ...strainData.map((s) => [
        s.strain_name,
        s.mean_health ?? '—',
        s.total_specimens,
        s.avg_days_between_passages ?? '—',
        s.best_performer_rate_pct,
      ]),
    ];
    return XLSX.utils.aoa_to_sheet(rows);
  }

  function cryoUtilizationSheet(): XLSX.WorkSheet {
    const rows: any[][] = [
      ...reportHeaderRows('Cryo Utilization'),
      ['Species Code', 'Vials Active', 'Vials Depleted/Discarded', 'Utilization Rate %'],
      ...cryoUtilization.map((c) => [c.species_code, c.vials_active, c.vials_depleted_or_discarded, c.utilization_rate_pct]),
    ];
    return XLSX.utils.aoa_to_sheet(rows);
  }

  function technicianActivitySheet(): XLSX.WorkSheet {
    const rows: any[][] = [
      ...reportHeaderRows('Technician Activity'),
      ['Technician', 'Passages Recorded', 'Contamination Events'],
      ...technicianActivity.map((t) => [t.display_name, t.passages_recorded, t.contamination_events]),
    ];
    return XLSX.utils.aoa_to_sheet(rows);
  }

  async function handleExportReport() {
    exporting = true;
    try {
      const wb = XLSX.utils.book_new();
      let sheetCount = 0;

      if (panels.growth) {
        XLSX.utils.book_append_sheet(wb, seriesSheet('Specimen Growth Rate', growthData), 'Growth Rate');
        sheetCount++;
      }
      if (panels.subculture) {
        XLSX.utils.book_append_sheet(wb, seriesSheet('Subculture Frequency Trend', subcultureData), 'Subculture Freq');
        sheetCount++;
      }
      if (panels.contamination) {
        XLSX.utils.book_append_sheet(wb, seriesSheet('Contamination Rate Trend', contaminationData), 'Contamination Rate');
        sheetCount++;
      }
      if (panels.mediaEfficiency) {
        XLSX.utils.book_append_sheet(wb, mediaEfficiencySheet(), 'Media Efficiency');
        sheetCount++;
      }
      if (panels.strainPerformance && selectedSpeciesId && strainData.length > 0) {
        XLSX.utils.book_append_sheet(wb, strainPerformanceSheet(), 'Strain Performance');
        sheetCount++;
      }
      if (panels.cryoUtilization) {
        XLSX.utils.book_append_sheet(wb, cryoUtilizationSheet(), 'Cryo Utilization');
        sheetCount++;
      }
      if (panels.technicianActivity && isSupervisorOrAdmin) {
        XLSX.utils.book_append_sheet(wb, technicianActivitySheet(), 'Technician Activity');
        sheetCount++;
      }

      if (sheetCount === 0) {
        addNotification('No visible panels to export — enable at least one panel first', 'warning');
        return;
      }

      const buf = XLSX.write(wb, { type: 'array', bookType: 'xlsx' });
      triggerDownload(
        new Blob([buf], { type: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet' }),
        `analytics_report_${datestamp()}.xlsx`,
      );
      addNotification(`Analytics report exported (${sheetCount} sheet${sheetCount === 1 ? '' : 's'})`, 'success');
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      exporting = false;
    }
  }
</script>

<div class="analytics-dashboard">
  <div class="page-header">
    <h1>Analytics &amp; Reporting</h1>
    <div class="header-actions">
      <div class="range-selector" role="group" aria-label="Time range">
        {#each TIME_RANGES as tr}
          <button
            class="btn btn-sm range-btn"
            class:active={timeRange === tr.value}
            onclick={() => (timeRange = tr.value)}
            aria-pressed={timeRange === tr.value}
          >
            {tr.label}
          </button>
        {/each}
      </div>
      <button class="btn btn-primary" onclick={handleExportReport} disabled={exporting} title="Export a multi-sheet Excel report of the currently visible panels">
        {exporting ? 'Exporting…' : 'Export Report'}
      </button>
    </div>
  </div>

  <!-- Customize panels — the layout is a single shared lab-wide setting, so
       only supervisors/admins can change it (the backend enforces this too;
       hiding the control for others avoids a confusing permission-denied
       toast on toggle). -->
  {#if isSupervisorOrAdmin}
  <div class="card customize-card">
    <button class="customize-toggle" onclick={() => (showCustomize = !showCustomize)} aria-expanded={showCustomize}>
      <span>Customize panels</span>
      <span class="chevron" class:open={showCustomize} aria-hidden="true">&#9662;</span>
    </button>
    {#if showCustomize}
      <div class="customize-grid">
        {#each Object.keys(PANEL_LABELS) as key (key)}
          {#if key !== 'technicianActivity' || isSupervisorOrAdmin}
            <label class="customize-item">
              <input
                type="checkbox"
                checked={panels[key as PanelKey]}
                onchange={() => togglePanel(key as PanelKey)}
              />
              <span>{PANEL_LABELS[key as PanelKey]}</span>
            </label>
          {/if}
        {/each}
      </div>
    {/if}
  </div>
  {/if}

  <!-- KPI strip -->
  <DataState loading={loadingKpi} error={kpiError} onretry={loadKpi}>
    {#if kpi}
      <div class="stats-grid">
        <div class="stat-card" title="Active specimens currently tracked">
          <div class="stat-value">{kpi.total_active_specimens}</div>
          <div class="stat-label">Active Specimens</div>
        </div>
        <div class="stat-card" title="Subculture passages recorded in the last 7 days">
          <div class="stat-value">{kpi.passages_this_week}</div>
          <div class="stat-label">Passages This Week</div>
        </div>
        <div class="stat-card" class:alert={kpi.contamination_rate_this_month_pct > 10} title="Contamination rate for the current calendar month">
          <div class="stat-value">{fmtNum(kpi.contamination_rate_this_month_pct)}%</div>
          <div class="stat-label">Contamination Rate (mo.)</div>
        </div>
        <div class="stat-card" class:warn={kpi.pending_work_queue_items > 0} title="Items currently in the work queue">
          <div class="stat-value">{kpi.pending_work_queue_items}</div>
          <div class="stat-label">Pending Work Queue</div>
        </div>
        <div class="stat-card" title="Average passages per active specimen — a throughput indicator">
          <div class="stat-value">{fmtNum(kpi.passages_per_active_specimen, 2)}</div>
          <div class="stat-label">Passages / Specimen</div>
        </div>
        {#if growthDelta()}
          {@const g = growthDelta()!}
          <div class="stat-card" class:warn={!g.up} title="New specimens this month vs. last month">
            <div class="stat-value growth-value">
              <span class="growth-arrow" class:down={!g.up} aria-hidden="true">{g.up ? '▲' : '▼'}</span>
              {kpi.new_specimens_this_month}
            </div>
            <div class="stat-label">New Specimens (mo.) &middot; {g.up ? '+' : ''}{g.delta} vs last mo.</div>
          </div>
        {/if}
      </div>
    {/if}
  </DataState>

  <!-- Time-series panels -->
  <DataState loading={loadingSeries} error={seriesError} onretry={loadSeries}>
    <div class="panel-grid">
      {#if panels.growth}
        <div class="panel">
          <h3 title="Rate of new specimen creation over the selected time range">Specimen Growth Rate</h3>
          {#if growthData.length === 0}
            <p class="empty-state">No data for this time range yet</p>
          {:else}
            <svg class="chart" viewBox="0 0 100 100" preserveAspectRatio="none" role="img" aria-label="Specimen growth rate line chart">
              <polyline points={linePoints(growthData)} fill="none" stroke="var(--color-fill-stage)" stroke-width="2" vector-effect="non-scaling-stroke" />
            </svg>
            <div class="chart-footer">
              <span>{growthData[0].bucket}</span>
              <span>{growthData[growthData.length - 1].bucket}</span>
            </div>
          {/if}
        </div>
      {/if}

      {#if panels.subculture}
        <div class="panel">
          <h3 title="Frequency of subculture passages over the selected time range">Subculture Frequency Trend</h3>
          {#if subcultureData.length === 0}
            <p class="empty-state">No data for this time range yet</p>
          {:else}
            <svg class="chart" viewBox="0 0 100 100" preserveAspectRatio="none" role="img" aria-label="Subculture frequency bar chart">
              {#each barRects(subcultureData) as r}
                <rect x={r.x} y={r.y} width={r.w} height={r.h} fill="var(--color-fill-species)">
                  <title>{r.bucket}: {r.value}</title>
                </rect>
              {/each}
            </svg>
            <div class="chart-footer">
              <span>{subcultureData[0].bucket}</span>
              <span>{subcultureData[subcultureData.length - 1].bucket}</span>
            </div>
          {/if}
        </div>
      {/if}

      {#if panels.contamination}
        <div class="panel">
          <h3 title="Contamination rate trend over the selected time range">Contamination Rate Trend</h3>
          {#if contaminationData.length === 0}
            <p class="empty-state">No data for this time range yet</p>
          {:else}
            <svg class="chart" viewBox="0 0 100 100" preserveAspectRatio="none" role="img" aria-label="Contamination rate line chart">
              <polyline points={linePoints(contaminationData)} fill="none" stroke="var(--color-fill-contam)" stroke-width="2" vector-effect="non-scaling-stroke" />
            </svg>
            <div class="chart-footer">
              <span>{contaminationData[0].bucket}</span>
              <span>{contaminationData[contaminationData.length - 1].bucket}</span>
            </div>
          {/if}
        </div>
      {/if}

      {#if panels.passageSuccess}
        <div class="panel">
          <h3 title="Share of subculture passages that did not result in contamination or failure">Passage Success Rate</h3>
          {#if !passageSuccess || passageSuccess.total_passages === 0}
            <p class="empty-state">No data for this time range yet</p>
          {:else}
            <div class="big-stat-row">
              <div class="big-stat">
                <div class="big-stat-value">{fmtNum(passageSuccess.success_rate_pct)}%</div>
                <div class="big-stat-label">Success Rate</div>
              </div>
              <div class="big-stat-meta">
                <div>{passageSuccess.successful_passages} / {passageSuccess.total_passages} passages</div>
                <div class:trend-up={passageSuccess.trend_delta_pct >= 0} class:trend-down={passageSuccess.trend_delta_pct < 0}>
                  {passageSuccess.trend_delta_pct >= 0 ? '+' : ''}{fmtNum(passageSuccess.trend_delta_pct)}% vs prior period
                </div>
              </div>
            </div>
          {/if}
        </div>
      {/if}

      {#if panels.mediaEfficiency}
        <div class="panel">
          <h3 title="Specimens supported and waste rate per media batch">Media Batch Efficiency</h3>
          {#if mediaEfficiency.length === 0}
            <p class="empty-state">No data for this time range yet</p>
          {:else}
            <div class="table-wrap">
              <table class="data-table">
                <thead>
                  <tr>
                    <th>Batch</th>
                    <th class="num">Specimens Supported</th>
                    <th class="num">Waste Rate %</th>
                  </tr>
                </thead>
                <tbody>
                  {#each mediaEfficiency as m}
                    <tr>
                      <td>{m.name}</td>
                      <td class="num">{m.specimens_supported}</td>
                      <td class="num" class:h-crit={m.waste_rate_pct > 20}>{fmtNum(m.waste_rate_pct)}%</td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          {/if}
        </div>
      {/if}

      {#if panels.cryoUtilization}
        <div class="panel">
          <h3 title="Cryostorage vial utilization by species">Cryo Utilization</h3>
          {#if cryoUtilization.length === 0}
            <p class="empty-state">No data for this time range yet</p>
          {:else}
            <div class="chart-bars">
              {#each cryoUtilization as c}
                <div class="bar-row">
                  <span class="bar-label" title={c.species_code}>{c.species_code}</span>
                  <div class="bar-track">
                    <div class="bar-fill" style="width: {Math.max(4, c.utilization_rate_pct)}%"></div>
                  </div>
                  <span class="bar-value">{fmtNum(c.utilization_rate_pct)}%</span>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {/if}

      {#if panels.strainPerformance}
        <div class="panel panel-wide">
          <h3 title="Health, throughput, and passage cadence per strain within a species">Strain Performance</h3>
          <div class="form-group species-select-row">
            <label for="strain-species-select">Species</label>
            <select id="strain-species-select" bind:value={selectedSpeciesId}>
              <option value="">Select a species…</option>
              {#each species as sp}
                <option value={sp.id}>{speciesLabel(sp)}</option>
              {/each}
            </select>
          </div>
          {#if !selectedSpeciesId}
            <p class="empty-state">Select a species to view strain performance</p>
          {:else}
            <DataState loading={loadingStrains} error={strainError} onretry={() => loadStrainPerformance(selectedSpeciesId)}>
              {#if strainData.length === 0}
                <p class="empty-state">No data for this time range yet</p>
              {:else}
                <svg class="chart" viewBox="0 0 100 100" preserveAspectRatio="none" role="img" aria-label="Mean health per strain bar chart">
                  {#each barRects(strainData.map((s) => ({ bucket: s.strain_name, value: s.mean_health ?? 0 }))) as r}
                    <rect x={r.x} y={r.y} width={r.w} height={r.h} fill="var(--color-fill-stage)">
                      <title>{r.bucket}: {fmtNum(r.value)}</title>
                    </rect>
                  {/each}
                </svg>
                <div class="table-wrap">
                  <table class="data-table">
                    <thead>
                      <tr>
                        <th class="sortable" onclick={() => sortStrains('strain_name')}>Strain</th>
                        <th class="num sortable" onclick={() => sortStrains('mean_health')}>Mean Health</th>
                        <th class="num sortable" onclick={() => sortStrains('total_specimens')}>Total Specimens</th>
                        <th class="num sortable" onclick={() => sortStrains('avg_days_between_passages')}>Avg Days Between Passages</th>
                        <th class="num sortable" onclick={() => sortStrains('best_performer_rate_pct')}>Best Performer Rate %</th>
                      </tr>
                    </thead>
                    <tbody>
                      {#each sortedStrains as s}
                        <tr>
                          <td>{s.strain_name}</td>
                          <td class="num">{s.mean_health === null ? '—' : fmtNum(s.mean_health, 2)}</td>
                          <td class="num">{s.total_specimens}</td>
                          <td class="num">{s.avg_days_between_passages === null ? '—' : fmtNum(s.avg_days_between_passages)}</td>
                          <td class="num">{fmtNum(s.best_performer_rate_pct)}%</td>
                        </tr>
                      {/each}
                    </tbody>
                  </table>
                </div>
              {/if}
            </DataState>
          {/if}
        </div>
      {/if}

      {#if panels.technicianActivity && isSupervisorOrAdmin}
        <div class="panel panel-wide">
          <h3 title="Passages recorded and contamination events per technician">Technician Activity</h3>
          <p class="hint-note">Workload visibility, not a performance review.</p>
          {#if technicianActivity.length === 0}
            <p class="empty-state">No data for this time range yet</p>
          {:else}
            <div class="table-wrap">
              <table class="data-table">
                <thead>
                  <tr>
                    <th>Technician</th>
                    <th class="num">Passages Recorded</th>
                    <th class="num">Contamination Events</th>
                  </tr>
                </thead>
                <tbody>
                  {#each technicianActivity as t}
                    <tr>
                      <td>{t.display_name}</td>
                      <td class="num">{t.passages_recorded}</td>
                      <td class="num">{t.contamination_events}</td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          {/if}
        </div>
      {/if}
    </div>
  </DataState>
</div>

<style>
  .header-actions {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    flex-wrap: wrap;
  }

  .range-selector {
    display: flex;
    gap: var(--space-1);
    background: var(--color-surface-raised);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: 2px;
  }
  .range-btn {
    border: none;
    background: transparent;
  }
  .range-btn.active {
    background: var(--color-accent);
    color: white;
  }

  .customize-card {
    margin-bottom: var(--space-5);
    padding: var(--space-4) var(--space-5);
  }
  .customize-toggle {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    background: none;
    border: none;
    cursor: pointer;
    font-size: var(--font-size-lg);
    font-weight: 700;
    color: var(--color-text);
    padding: 0;
  }
  .chevron {
    transition: transform var(--transition-fast);
    color: var(--color-text-muted);
  }
  .chevron.open {
    transform: rotate(180deg);
  }
  .customize-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: var(--space-3);
    margin-top: var(--space-4);
  }
  .customize-item {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--font-size-base);
    cursor: pointer;
  }
  .customize-item input {
    width: auto;
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: var(--space-4);
    margin-bottom: var(--space-6);
  }
  .stat-card {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: var(--space-5);
    text-align: center;
  }
  .stat-card.warn { border-left: 4px solid var(--color-warn); }
  .stat-card.alert { border-left: 4px solid var(--color-danger); }
  .stat-value { font-size: var(--font-size-3xl); font-weight: 800; color: var(--color-text-strong); }
  .stat-label { font-size: var(--font-size-sm); color: var(--color-text-muted); text-transform: uppercase; letter-spacing: 0.5px; margin-top: var(--space-1); }

  .growth-value { display: flex; align-items: center; justify-content: center; gap: var(--space-2); }
  .growth-arrow { color: var(--color-success); font-size: var(--font-size-xl); }
  .growth-arrow.down { color: var(--color-danger); }

  .panel-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-5);
  }
  .panel {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: var(--space-5);
  }
  .panel-wide {
    grid-column: 1 / -1;
  }
  .panel h3 { font-size: var(--font-size-lg); font-weight: 700; margin-bottom: var(--space-4); }

  .chart {
    width: 100%;
    height: 160px;
    display: block;
  }
  .chart-footer {
    display: flex;
    justify-content: space-between;
    font-size: var(--font-size-xs);
    color: var(--color-text-muted);
    margin-top: var(--space-2);
  }

  .empty-state {
    color: var(--color-text-muted);
    font-size: var(--font-size-base);
    text-align: center;
    padding: var(--space-6) 0;
  }

  .big-stat-row {
    display: flex;
    align-items: center;
    gap: var(--space-5);
  }
  .big-stat-value { font-size: var(--font-size-stat); font-weight: 800; color: var(--color-text-strong); }
  .big-stat-label { font-size: var(--font-size-sm); color: var(--color-text-muted); text-transform: uppercase; letter-spacing: 0.5px; }
  .big-stat-meta { display: flex; flex-direction: column; gap: 4px; font-size: var(--font-size-base); color: var(--color-text-muted); }
  .trend-up { color: var(--color-success); font-weight: 600; }
  .trend-down { color: var(--color-danger); font-weight: 600; }

  .chart-bars { display: flex; flex-direction: column; gap: var(--space-2); }
  .bar-row { display: flex; align-items: center; gap: 10px; }
  .bar-label { width: 100px; font-size: var(--font-size-sm); font-weight: 600; }
  .bar-track { flex: 1; height: 20px; background: var(--color-fill-track); border-radius: var(--radius-sm); overflow: hidden; }
  .bar-fill { height: 100%; background: var(--color-fill-stage); border-radius: var(--radius-sm); transition: width 0.3s; }
  .bar-value { width: 50px; text-align: right; font-size: var(--font-size-base); font-weight: 700; }

  .table-wrap { overflow-x: auto; }
  .data-table { width: 100%; border-collapse: collapse; font-size: var(--font-size-base); }
  .data-table th {
    text-align: left;
    padding: var(--space-2) var(--space-3);
    border-bottom: 2px solid var(--color-border);
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
    text-transform: uppercase;
    letter-spacing: 0.4px;
  }
  .data-table td {
    padding: var(--space-2) var(--space-3);
    border-bottom: 1px solid var(--color-border-subtle);
  }
  .data-table th.num, .data-table td.num { text-align: right; }
  .data-table th.sortable { cursor: pointer; user-select: none; }
  .data-table th.sortable:hover { color: var(--color-text); }
  .h-crit { color: var(--color-danger); font-weight: 700; }

  .species-select-row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    margin-bottom: var(--space-4);
  }
  .species-select-row label { margin: 0; white-space: nowrap; }
  .species-select-row select { max-width: 360px; }

  .hint-note {
    font-size: var(--font-size-sm);
    color: var(--color-text-muted);
    font-style: italic;
    margin-bottom: var(--space-3);
  }

  @media (max-width: 1024px) {
    .panel-grid { grid-template-columns: 1fr; }
  }
</style>
