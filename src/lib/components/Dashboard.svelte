<script lang="ts">
  import { onMount } from 'svelte';
  import { getSpecimenStats, getActiveReminders, getComplianceFlags, getLowStockAlerts, createBackup, listBackups, restoreBackup, resetDatabase, getContaminationStats, getSubcultureSchedule } from '../api';
  import { navigateTo, addNotification, devMode } from '../stores/app';
  import { currentUser } from '../stores/auth';
  import FirstRun from './FirstRun.svelte';

  let stats = $state<any>(null);
  let reminders = $state<any[]>([]);
  let flags = $state<any[]>([]);
  let lowStock = $state<any[]>([]);
  let contaminationStats = $state<any>(null);
  let schedule = $state<any[]>([]);
  let loading = $state(true);

  let overdueItems = $derived(schedule.filter((e: any) => e.is_overdue));
  let dueSoonItems = $derived(schedule.filter((e: any) => !e.is_overdue && e.days_until_due !== null && e.days_until_due <= 7));
  let firstRun = $derived(!loading && stats !== null && stats.total_specimens === 0);
  let backingUp = $state(false);
  let showResetPanel = $state(false);
  let resetPhrase = $state('');
  let resetting = $state(false);

  // Restore state
  let backups = $state<any[]>([]);
  let loadingBackups = $state(false);
  let showRestorePanel = $state(false);
  let restoreTarget = $state<any>(null);
  let restoreStep = $state<1 | 2>(1);
  let restorePhrase = $state('');
  let restoring = $state(false);

  onMount(() => {
    loadDashboard();
  });

  async function loadDashboard() {
    loading = true;
    try {
      const [s, r, f, ls, cs, sch] = await Promise.all([
        getSpecimenStats(),
        getActiveReminders(),
        getComplianceFlags(),
        getLowStockAlerts(),
        getContaminationStats(),
        getSubcultureSchedule(),
      ]);
      stats = s;
      reminders = r;
      flags = f;
      lowStock = ls;
      contaminationStats = cs;
      schedule = sch;
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      loading = false;
    }
  }

  async function handleBackup() {
    backingUp = true;
    try {
      const path = await createBackup();
      addNotification(`Backup saved to: ${path}`, 'success');
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      backingUp = false;
    }
  }

  async function openRestorePanel() {
    showRestorePanel = true;
    restoreTarget = null;
    restoreStep = 1;
    restorePhrase = '';
    loadingBackups = true;
    try {
      backups = await listBackups();
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      loadingBackups = false;
    }
  }

  function selectRestoreTarget(backup: any) {
    restoreTarget = backup;
    restoreStep = 1;
    restorePhrase = '';
  }

  function confirmRestoreStep1() {
    restoreStep = 2;
  }

  async function handleRestore() {
    if (restorePhrase !== 'RESTORE') {
      addNotification('Type exactly: RESTORE', 'warning');
      return;
    }
    restoring = true;
    try {
      await restoreBackup(restoreTarget.path);
      // App restarts automatically after successful restore; message is a fallback.
      addNotification('Restore successful — restarting…', 'success');
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      restoring = false;
    }
  }

  function cancelRestore() {
    showRestorePanel = false;
    restoreTarget = null;
    restoreStep = 1;
    restorePhrase = '';
  }

  async function handleReset() {
    if (resetPhrase !== 'RESET DATABASE') {
      addNotification('Type exactly: RESET DATABASE', 'warning');
      return;
    }
    resetting = true;
    try {
      const msg = await resetDatabase(resetPhrase);
      addNotification(msg, 'success');
      showResetPanel = false;
      resetPhrase = '';
      loadDashboard();
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      resetting = false;
    }
  }

  function getSeverityClass(severity: string): string {
    switch (severity) {
      case 'critical': return 'badge-red';
      case 'high': return 'badge-yellow';
      default: return 'badge-blue';
    }
  }

  function getUrgencyClass(urgency: string): string {
    switch (urgency) {
      case 'critical': return 'badge-red';
      case 'high': return 'badge-yellow';
      case 'normal': return 'badge-blue';
      default: return 'badge-gray';
    }
  }
</script>

<div class="dashboard">
  <div class="page-header">
    <h1>Dashboard</h1>
    <button class="btn" onclick={loadDashboard} title="Reload dashboard data">Refresh</button>
  </div>

  {#if loading}
    <div class="empty-state">Loading dashboard...</div>
  {:else if firstRun}
    <FirstRun onDemoLoaded={loadDashboard} />
  {:else if stats}
    <div class="stats-grid">
      <div class="stat-card" title="Number of specimens currently in active culture">
        <div class="stat-value">{stats.active_specimens}</div>
        <div class="stat-label">Active Specimens</div>
      </div>
      <div class="stat-card" title="Total number of specimens ever accessioned into the system">
        <div class="stat-value">{stats.total_specimens}</div>
        <div class="stat-label">Total Specimens</div>
      </div>
      <div class="stat-card warn" title="Specimens currently under quarantine restrictions">
        <div class="stat-value">{stats.quarantined}</div>
        <div class="stat-label">Quarantined</div>
      </div>
      <div class="stat-card" title="Number of subculture passages performed in the last 7 days">
        <div class="stat-value">{stats.recent_subcultures}</div>
        <div class="stat-label">Subcultures (7d)</div>
      </div>
      <div class="stat-card" title="Specimens that have been archived and are no longer in active culture">
        <div class="stat-value">{stats.archived}</div>
        <div class="stat-label">Archived</div>
      </div>
      <div class="stat-card alert" title="Open compliance flags requiring review or corrective action">
        <div class="stat-value">{flags.length}</div>
        <div class="stat-label">Compliance Flags</div>
      </div>
      <div class="stat-card" class:warn={lowStock.length > 0} title="Inventory items currently below their minimum stock threshold">
        <div class="stat-value">{lowStock.length}</div>
        <div class="stat-label">Low Stock Items</div>
      </div>
      {#if contaminationStats}
        <div class="stat-card" class:alert={contaminationStats.contaminated_specimens > 0} title="Number of active specimens that have at least one contaminated vessel event">
          <div class="stat-value">{contaminationStats.contaminated_specimens}</div>
          <div class="stat-label">Contaminated Vessels</div>
        </div>
        <div class="stat-card" class:alert={contaminationStats.contamination_rate_pct > 10} title="Percentage of active specimens with at least one contaminated vessel">
          <div class="stat-value">{contaminationStats.contamination_rate_pct.toFixed(1)}%</div>
          <div class="stat-label">Contamination Rate</div>
        </div>
      {/if}
      <div class="stat-card" class:alert={overdueItems.length > 0} title="Specimens past their scheduled subculture date based on species interval">
        <div class="stat-value">{overdueItems.length}</div>
        <div class="stat-label">Overdue Subcultures</div>
      </div>
    </div>

    <div class="dashboard-panels">
      <div class="panel">
        <h3 title="Reminders due soon, sorted by urgency">Upcoming Reminders</h3>
        {#if reminders.length === 0}
          <p class="empty-state">No upcoming reminders</p>
        {:else}
          <div class="reminder-list">
            {#each reminders.slice(0, 8) as r}
              <div class="reminder-item">
                <div>
                  <div class="reminder-title">{r.title}</div>
                  <div class="reminder-meta">
                    {r.specimen_accession || 'General'} &middot; Due: {r.due_date}
                  </div>
                </div>
                <span class="badge {getUrgencyClass(r.urgency)}" title="Urgency level: {r.urgency}">{r.urgency}</span>
              </div>
            {/each}
          </div>
          <button class="btn btn-sm" style="margin-top:12px" onclick={() => navigateTo('reminders')} title="Go to the full Reminders list">
            View all reminders
          </button>
        {/if}
      </div>

      <div class="panel">
        <h3 title="Active compliance flags detected across all specimens">Compliance Alerts</h3>
        {#if flags.length === 0}
          <p class="empty-state">No compliance issues detected</p>
        {:else}
          <div class="flag-list">
            {#each flags.slice(0, 8) as f}
              <div class="flag-item">
                <span class="badge {getSeverityClass(f.severity)}" title="Severity: {f.severity}">{f.severity}</span>
                <div>
                  <div class="flag-accession">{f.accession_number}</div>
                  <div class="flag-message">{f.message}</div>
                </div>
              </div>
            {/each}
          </div>
          <button class="btn btn-sm" style="margin-top:12px" onclick={() => navigateTo('compliance')} title="Go to the full Compliance module">
            View compliance
          </button>
        {/if}
      </div>

      <div class="panel">
        <h3 title="Distribution of active specimens across stages defined for the active lab profile">Specimens by Stage</h3>
        {#if stats.by_stage.length === 0}
          <p class="empty-state">No specimens yet</p>
        {:else}
          <div class="chart-bars">
            {#each stats.by_stage as s}
              <div class="bar-row">
                <span class="bar-label">{s.stage}</span>
                <div class="bar-track">
                  <div
                    class="bar-fill"
                    style="width: {Math.max(4, (s.count / Math.max(...stats.by_stage.map((x: any) => x.count))) * 100)}%"
                  ></div>
                </div>
                <span class="bar-value">{s.count}</span>
              </div>
            {/each}
          </div>
        {/if}
      </div>

      <div class="panel">
        <h3 title="Distribution of active specimens grouped by species code">Specimens by Species</h3>
        {#if stats.by_species.length === 0}
          <p class="empty-state">No specimens yet</p>
        {:else}
          <div class="chart-bars">
            {#each stats.by_species as s}
              <div class="bar-row">
                <span class="bar-label">{s.species_code}</span>
                <div class="bar-track">
                  <div
                    class="bar-fill species-fill"
                    style="width: {Math.max(4, (s.count / Math.max(...stats.by_species.map((x: any) => x.count))) * 100)}%"
                  ></div>
                </div>
                <span class="bar-value">{s.count}</span>
              </div>
            {/each}
          </div>
        {/if}
      </div>

      <!-- Subculture Schedule panel -->
      <div class="panel">
        <h3 title="Upcoming and overdue subculture passages based on each species' recommended interval">Subculture Schedule</h3>
        {#if schedule.length === 0}
          <p class="empty-state">No specimens with scheduling data</p>
        {:else}
          {#if overdueItems.length > 0}
            <div class="schedule-section-label overdue-label">Overdue ({overdueItems.length})</div>
            <div class="schedule-list">
              {#each overdueItems.slice(0, 5) as entry}
                <div class="schedule-item overdue-item">
                  <div class="schedule-item-left">
                    <span class="schedule-accession">{entry.accession_number}</span>
                    <span class="schedule-species">{entry.species_code}</span>
                  </div>
                  <div class="schedule-item-right">
                    <span class="badge badge-red" title="This specimen is {Math.abs(entry.days_until_due)} day(s) past its scheduled subculture date">{Math.abs(entry.days_until_due)}d overdue</span>
                    {#if entry.next_due_date}
                      <span class="schedule-date">Due {entry.next_due_date}</span>
                    {/if}
                  </div>
                </div>
              {/each}
            </div>
          {/if}
          {#if dueSoonItems.length > 0}
            <div class="schedule-section-label" style="margin-top:{overdueItems.length > 0 ? 12 : 0}px;">Due within 7 days ({dueSoonItems.length})</div>
            <div class="schedule-list">
              {#each dueSoonItems.slice(0, 5) as entry}
                <div class="schedule-item">
                  <div class="schedule-item-left">
                    <span class="schedule-accession">{entry.accession_number}</span>
                    <span class="schedule-species">{entry.species_code}</span>
                  </div>
                  <div class="schedule-item-right">
                    <span class="badge badge-yellow" title="This specimen is due for subculture in {entry.days_until_due} day(s)">{entry.days_until_due}d left</span>
                    {#if entry.next_due_date}
                      <span class="schedule-date">Due {entry.next_due_date}</span>
                    {/if}
                  </div>
                </div>
              {/each}
            </div>
          {/if}
          {#if overdueItems.length === 0 && dueSoonItems.length === 0}
            <p class="empty-state">All subcultures on schedule</p>
          {/if}
          <button class="btn btn-sm" style="margin-top:12px" onclick={() => navigateTo('specimens')} title="Go to the full Specimens list">
            View specimens
          </button>
        {/if}
      </div>

      <!-- Contamination Stats panel -->
      {#if contaminationStats}
        <div class="panel">
          <h3 title="Summary of contamination events across all specimens and vessel types">Contamination Overview</h3>
          <div class="contam-rate-row">
            <div class="contam-rate-value" class:contam-high={contaminationStats.contamination_rate_pct > 10}>
              {contaminationStats.contamination_rate_pct.toFixed(1)}%
            </div>
            <div class="contam-rate-meta">
              <span>{contaminationStats.contaminated_specimens} / {contaminationStats.total_specimens} specimens affected</span>
              <span>{contaminationStats.contaminated_vessels} total contaminated vessel events</span>
            </div>
          </div>
          {#if contaminationStats.by_vessel_type.length > 0}
            <div class="contam-breakdown-label">By vessel type</div>
            <div class="chart-bars">
              {#each contaminationStats.by_vessel_type as vt}
                <div class="bar-row">
                  <span class="bar-label" title={vt.vessel_type}>{vt.vessel_type}</span>
                  <div class="bar-track">
                    <div
                      class="bar-fill contam-fill"
                      style="width: {Math.max(4, (vt.count / Math.max(...contaminationStats.by_vessel_type.map((x: any) => x.count))) * 100)}%"
                    ></div>
                  </div>
                  <span class="bar-value">{vt.count}</span>
                </div>
              {/each}
            </div>
          {/if}
          {#if contaminationStats.recent_events.length > 0}
            <div class="contam-breakdown-label" style="margin-top:12px;">Recent events</div>
            <div class="flag-list">
              {#each contaminationStats.recent_events.slice(0, 5) as ev}
                <div class="flag-item">
                  <span class="badge badge-red" title="Passage number {ev.passage_number} at which contamination was recorded">P{ev.passage_number}</span>
                  <div>
                    <div class="flag-accession">{ev.accession_number} <span style="font-weight:400;color:#6b7280;">({ev.species_code})</span></div>
                    <div class="flag-message">{ev.date}{ev.vessel_type ? ` · ${ev.vessel_type}` : ''}{ev.contamination_notes ? ` · ${ev.contamination_notes}` : ''}</div>
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {/if}

      <div class="panel">
        <h3 title="Inventory items that have fallen below their minimum stock threshold">Inventory Alerts</h3>
        {#if lowStock.length === 0}
          <p class="empty-state">All stock levels OK</p>
        {:else}
          <div class="flag-list">
            {#each lowStock.slice(0, 8) as item}
              <div class="flag-item">
                <span class="badge badge-yellow" title="Stock level is below the minimum threshold">Low</span>
                <div>
                  <div class="flag-accession">{item.name}</div>
                  <div class="flag-message">
                    {item.current_stock} {item.unit} remaining (min: {item.minimum_stock})
                  </div>
                </div>
              </div>
            {/each}
          </div>
          <button class="btn btn-sm" style="margin-top:12px" onclick={() => navigateTo('inventory')} title="Go to the full Inventory module">
            View inventory
          </button>
        {/if}
      </div>

      <div class="panel">
        <h3 title="Create a snapshot backup of the entire database to the configured backup directory">Database Backup</h3>
        <p style="font-size:13px; color:#6b7280; margin-bottom:12px;">
          Create a backup of the database to the default backup directory.
        </p>
        {#if $currentUser?.role === 'admin' || $currentUser?.role === 'supervisor'}
          <button class="btn btn-primary" onclick={handleBackup} disabled={backingUp} title="Create a backup of the database to the default backup directory">
            {backingUp ? 'Backing up...' : 'Backup Now'}
          </button>
        {:else}
          <p style="font-size:12px; color:#6b7280;">Supervisor or admin access required.</p>
        {/if}
      </div>

      {#if $currentUser?.role === 'admin'}
        <div class="panel danger-panel">
          <h3 style="color:#dc2626;" title="Replace the current database with a previously created backup — this cannot be undone">⚠ Restore from Backup</h3>
          <p style="font-size:13px; color:#6b7280; margin-bottom:12px;">
            Replaces all current data with the state at the time of the selected backup.
            <strong>This cannot be undone.</strong>
          </p>
          {#if !showRestorePanel}
            <button class="btn btn-danger btn-sm" onclick={openRestorePanel} title="Show available backups to restore from">
              Show Restore Controls
            </button>
          {:else}
            <div class="reset-confirm">
              {#if loadingBackups}
                <p style="font-size:13px; color:#6b7280;">Loading backups…</p>
              {:else if backups.length === 0}
                <p style="font-size:13px; color:#6b7280;">No backups found. Create a backup first.</p>
                <button class="btn btn-sm" onclick={cancelRestore} style="margin-top:8px;" title="Close the restore panel">Cancel</button>
              {:else if !restoreTarget}
                <p style="font-size:12px; font-weight:600; margin-bottom:8px;">Select a backup to restore:</p>
                <div style="display:flex; flex-direction:column; gap:6px; margin-bottom:10px; max-height:200px; overflow-y:auto;">
                  {#each backups as b}
                    <button
                      class="btn btn-sm"
                      style="text-align:left; font-family:monospace; font-size:11px;"
                      onclick={() => selectRestoreTarget(b)}
                      title="Restore from {b.file_name}"
                    >
                      {b.file_name} — {b.created_at} ({Math.round(b.size_bytes / 1024)} KB)
                    </button>
                  {/each}
                </div>
                <button class="btn btn-sm" onclick={cancelRestore} title="Cancel and close the restore panel">Cancel</button>
              {:else if restoreStep === 1}
                <p style="font-size:13px; margin-bottom:10px;">
                  You are about to restore:<br />
                  <strong style="font-family:monospace; font-size:11px;">{restoreTarget.file_name}</strong><br />
                  <span style="color:#dc2626;">All current data will be permanently replaced.</span>
                </p>
                <div style="display:flex; gap:8px;">
                  <button class="btn btn-sm" onclick={cancelRestore} title="Cancel the restore">Cancel</button>
                  <button class="btn btn-danger btn-sm" onclick={confirmRestoreStep1} title="Proceed to final confirmation">Yes, continue →</button>
                </div>
              {:else}
                <p style="font-size:12px; font-weight:600; margin-bottom:8px;">
                  Final confirmation — type <code>RESTORE</code> to proceed:
                </p>
                <input
                  type="text"
                  bind:value={restorePhrase}
                  placeholder="RESTORE"
                  style="margin-bottom:8px; font-family:monospace;"
                  title="Type RESTORE exactly to unlock the restore button"
                />
                <div style="display:flex; gap:8px;">
                  <button class="btn btn-sm" onclick={cancelRestore} title="Cancel the restore">Cancel</button>
                  <button
                    class="btn btn-danger btn-sm"
                    onclick={handleRestore}
                    disabled={restoring || restorePhrase !== 'RESTORE'}
                    title="Permanently replace the current database with the selected backup"
                  >
                    {restoring ? 'Restoring…' : 'Restore Now'}
                  </button>
                </div>
              {/if}
            </div>
          {/if}
        </div>
      {/if}

      {#if $currentUser?.role === 'admin'}
        <div class="panel danger-panel">
          <h3 style="color:#dc2626;" title="Admin-only developer tools for editing protected records">⚠ Dev Tools — Developer Mode</h3>
          <p style="font-size:13px; color:#6b7280; margin-bottom:12px;">
            Enables in-app editing of passages and other protected records.
            Visible only to admins. Toggle persists across sessions.
          </p>
          <label style="display:inline-flex; align-items:center; gap:10px; cursor:pointer; font-size:14px;" title="Toggle developer mode to enable editing of passages and other protected records">
            <input type="checkbox" bind:checked={$devMode} style="width:auto; accent-color:#dc2626;" title="Enable or disable developer mode" />
            <span style="font-weight:600; color:{$devMode ? '#dc2626' : '#6b7280'};">
              Dev Mode {$devMode ? 'ON' : 'OFF'}
            </span>
          </label>
        </div>

        <div class="panel danger-panel">
          <h3 style="color:#dc2626;" title="Permanently wipe all specimen data, media, subcultures, compliance records, inventory, and audit logs — this cannot be undone">⚠ Dev Tools — Reset Database</h3>
          <p style="font-size:13px; color:#6b7280; margin-bottom:12px;">
            Permanently deletes all specimens, media batches, subcultures, compliance records,
            inventory, and audit logs. Users and species definitions are preserved.
            <strong>This cannot be undone.</strong>
          </p>
          {#if !showResetPanel}
            <button class="btn btn-danger btn-sm" onclick={() => showResetPanel = true} title="Reveal the database reset confirmation controls">
              Show Reset Controls
            </button>
          {:else}
            <div class="reset-confirm">
              <p style="font-size:12px; font-weight:600; margin-bottom:8px;">
                Type <code>RESET DATABASE</code> to confirm:
              </p>
              <input
                type="text"
                bind:value={resetPhrase}
                placeholder="RESET DATABASE"
                style="margin-bottom:8px; font-family:monospace;"
                title="Type RESET DATABASE exactly to unlock the reset button"
              />
              <div style="display:flex; gap:8px;">
                <button class="btn btn-sm" onclick={() => { showResetPanel = false; resetPhrase = ''; }} title="Cancel and hide the reset controls">
                  Cancel
                </button>
                <button
                  class="btn btn-danger btn-sm"
                  onclick={handleReset}
                  disabled={resetting || resetPhrase !== 'RESET DATABASE'}
                  title="Permanently delete all specimen data — this cannot be undone"
                >
                  {resetting ? 'Resetting...' : 'Reset Now'}
                </button>
              </div>
            </div>
          {/if}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
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

  .dashboard-panels {
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
  .panel h3 { font-size: var(--font-size-lg); font-weight: 700; margin-bottom: var(--space-4); }

  .reminder-list, .flag-list { display: flex; flex-direction: column; gap: 10px; }
  .reminder-item, .flag-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px;
    border-radius: var(--radius-md);
    background: var(--color-surface-raised);
  }
  .flag-item { gap: var(--space-3); justify-content: flex-start; }
  .reminder-title { font-weight: 600; font-size: var(--font-size-base); }
  .reminder-meta { font-size: var(--font-size-xs); color: var(--color-text-muted); margin-top: 2px; }
  .flag-accession { font-weight: 600; font-size: var(--font-size-base); }
  .flag-message { font-size: var(--font-size-sm); color: var(--color-text-muted); }

  .chart-bars { display: flex; flex-direction: column; gap: var(--space-2); }
  .bar-row { display: flex; align-items: center; gap: 10px; }
  .bar-label { width: 100px; font-size: var(--font-size-sm); font-weight: 600; text-transform: capitalize; }
  .bar-track { flex: 1; height: 20px; background: var(--color-fill-track); border-radius: var(--radius-sm); overflow: hidden; }
  .bar-fill { height: 100%; background: var(--color-fill-stage); border-radius: var(--radius-sm); transition: width 0.3s; }
  .species-fill { background: var(--color-fill-species); }
  .bar-value { width: 40px; text-align: right; font-size: var(--font-size-base); font-weight: 700; }

  .danger-panel {
    border: 1px solid rgba(220, 38, 38, 0.35);
    background: var(--color-surface-danger);
  }

  .reset-confirm {
    display: flex;
    flex-direction: column;
  }

  /* ── Subculture Schedule ── */
  .schedule-list { display: flex; flex-direction: column; gap: var(--space-2); }
  .schedule-item {
    display: flex; justify-content: space-between; align-items: center;
    padding: var(--space-2) 10px; border-radius: var(--radius-md);
    background: var(--color-surface-raised); gap: var(--space-2);
  }
  .overdue-item { background: var(--color-surface-overdue); }
  .schedule-item-left { display: flex; flex-direction: column; gap: 2px; }
  .schedule-item-right { display: flex; flex-direction: column; align-items: flex-end; gap: 2px; }
  .schedule-accession { font-size: var(--font-size-base); font-weight: 700; font-family: monospace; }
  .schedule-species { font-size: var(--font-size-xs); color: var(--color-text-muted); }
  .schedule-date { font-size: var(--font-size-xs); color: var(--color-text-muted); }
  .schedule-section-label {
    font-size: var(--font-size-xs); font-weight: 700; text-transform: uppercase;
    letter-spacing: 0.5px; color: var(--color-text-muted); margin-bottom: 6px;
  }
  .overdue-label { color: var(--color-overdue-label); }

  /* ── Contamination Overview ── */
  .contam-rate-row {
    display: flex; align-items: center; gap: var(--space-4);
    padding: var(--space-3); border-radius: var(--radius-lg);
    background: var(--color-surface-overdue);
    margin-bottom: var(--space-3);
  }
  .contam-rate-value {
    font-size: var(--font-size-stat); font-weight: 800; color: var(--color-text); flex-shrink: 0;
  }
  .contam-high { color: var(--color-contam-high); }
  .contam-rate-meta { display: flex; flex-direction: column; gap: 2px; font-size: var(--font-size-sm); color: var(--color-text-muted); }
  .contam-breakdown-label {
    font-size: var(--font-size-xs); font-weight: 700; text-transform: uppercase;
    letter-spacing: 0.5px; color: var(--color-text-muted); margin-bottom: 6px;
  }
  .contam-fill { background: var(--color-fill-contam); }
</style>
