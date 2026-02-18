<script lang="ts">
  import { onMount } from 'svelte';
  import { getSpecimenStats, getActiveReminders, getComplianceFlags, getLowStockAlerts, createBackup, resetDatabase } from '../api';
  import { navigateTo, addNotification, devMode } from '../stores/app';
  import { currentUser } from '../stores/auth';

  let stats = $state<any>(null);
  let reminders = $state<any[]>([]);
  let flags = $state<any[]>([]);
  let lowStock = $state<any[]>([]);
  let loading = $state(true);
  let backingUp = $state(false);
  let showResetPanel = $state(false);
  let resetPhrase = $state('');
  let resetting = $state(false);

  onMount(() => {
    loadDashboard();
  });

  async function loadDashboard() {
    loading = true;
    try {
      const [s, r, f, ls] = await Promise.all([
        getSpecimenStats(),
        getActiveReminders(),
        getComplianceFlags(),
        getLowStockAlerts(),
      ]);
      stats = s;
      reminders = r;
      flags = f;
      lowStock = ls;
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
    <button class="btn" onclick={loadDashboard}>Refresh</button>
  </div>

  {#if loading}
    <div class="empty-state">Loading dashboard...</div>
  {:else if stats}
    <div class="stats-grid">
      <div class="stat-card">
        <div class="stat-value">{stats.active_specimens}</div>
        <div class="stat-label">Active Specimens</div>
      </div>
      <div class="stat-card">
        <div class="stat-value">{stats.total_specimens}</div>
        <div class="stat-label">Total Specimens</div>
      </div>
      <div class="stat-card warn">
        <div class="stat-value">{stats.quarantined}</div>
        <div class="stat-label">Quarantined</div>
      </div>
      <div class="stat-card">
        <div class="stat-value">{stats.recent_subcultures}</div>
        <div class="stat-label">Subcultures (7d)</div>
      </div>
      <div class="stat-card">
        <div class="stat-value">{stats.archived}</div>
        <div class="stat-label">Archived</div>
      </div>
      <div class="stat-card alert">
        <div class="stat-value">{flags.length}</div>
        <div class="stat-label">Compliance Flags</div>
      </div>
      <div class="stat-card" class:warn={lowStock.length > 0}>
        <div class="stat-value">{lowStock.length}</div>
        <div class="stat-label">Low Stock Items</div>
      </div>
    </div>

    <div class="dashboard-panels">
      <div class="panel">
        <h3>Upcoming Reminders</h3>
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
                <span class="badge {getUrgencyClass(r.urgency)}">{r.urgency}</span>
              </div>
            {/each}
          </div>
          <button class="btn btn-sm" style="margin-top:12px" onclick={() => navigateTo('reminders')}>
            View all reminders
          </button>
        {/if}
      </div>

      <div class="panel">
        <h3>Compliance Alerts</h3>
        {#if flags.length === 0}
          <p class="empty-state">No compliance issues detected</p>
        {:else}
          <div class="flag-list">
            {#each flags.slice(0, 8) as f}
              <div class="flag-item">
                <span class="badge {getSeverityClass(f.severity)}">{f.severity}</span>
                <div>
                  <div class="flag-accession">{f.accession_number}</div>
                  <div class="flag-message">{f.message}</div>
                </div>
              </div>
            {/each}
          </div>
          <button class="btn btn-sm" style="margin-top:12px" onclick={() => navigateTo('compliance')}>
            View compliance
          </button>
        {/if}
      </div>

      <div class="panel">
        <h3>Specimens by Stage</h3>
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
        <h3>Specimens by Species</h3>
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

      <div class="panel">
        <h3>Inventory Alerts</h3>
        {#if lowStock.length === 0}
          <p class="empty-state">All stock levels OK</p>
        {:else}
          <div class="flag-list">
            {#each lowStock.slice(0, 8) as item}
              <div class="flag-item">
                <span class="badge badge-yellow">Low</span>
                <div>
                  <div class="flag-accession">{item.name}</div>
                  <div class="flag-message">
                    {item.current_stock} {item.unit} remaining (min: {item.minimum_stock})
                  </div>
                </div>
              </div>
            {/each}
          </div>
          <button class="btn btn-sm" style="margin-top:12px" onclick={() => navigateTo('inventory')}>
            View inventory
          </button>
        {/if}
      </div>

      <div class="panel">
        <h3>Database Backup</h3>
        <p style="font-size:13px; color:#6b7280; margin-bottom:12px;">
          Create a backup of the database to the default backup directory.
        </p>
        {#if $currentUser?.role === 'admin' || $currentUser?.role === 'supervisor'}
          <button class="btn btn-primary" onclick={handleBackup} disabled={backingUp}>
            {backingUp ? 'Backing up...' : 'Backup Now'}
          </button>
        {:else}
          <p style="font-size:12px; color:#6b7280;">Supervisor or admin access required.</p>
        {/if}
      </div>

      {#if $currentUser?.role === 'admin'}
        <div class="panel danger-panel">
          <h3 style="color:#dc2626;">⚠ Dev Tools — Developer Mode</h3>
          <p style="font-size:13px; color:#6b7280; margin-bottom:12px;">
            Enables in-app editing of passages and other protected records.
            Visible only to admins. Toggle persists across sessions.
          </p>
          <label style="display:inline-flex; align-items:center; gap:10px; cursor:pointer; font-size:14px;">
            <input type="checkbox" bind:checked={$devMode} style="width:auto; accent-color:#dc2626;" />
            <span style="font-weight:600; color:{$devMode ? '#dc2626' : '#6b7280'};">
              Dev Mode {$devMode ? 'ON' : 'OFF'}
            </span>
          </label>
        </div>

        <div class="panel danger-panel">
          <h3 style="color:#dc2626;">⚠ Dev Tools — Reset Database</h3>
          <p style="font-size:13px; color:#6b7280; margin-bottom:12px;">
            Permanently deletes all specimens, media batches, subcultures, compliance records,
            inventory, and audit logs. Users and species definitions are preserved.
            <strong>This cannot be undone.</strong>
          </p>
          {#if !showResetPanel}
            <button class="btn btn-danger btn-sm" onclick={() => showResetPanel = true}>
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
              />
              <div style="display:flex; gap:8px;">
                <button class="btn btn-sm" onclick={() => { showResetPanel = false; resetPhrase = ''; }}>
                  Cancel
                </button>
                <button
                  class="btn btn-danger btn-sm"
                  onclick={handleReset}
                  disabled={resetting || resetPhrase !== 'RESET DATABASE'}
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
    gap: 16px;
    margin-bottom: 24px;
  }
  .stat-card {
    background: white;
    border: 1px solid #e2e8f0;
    border-radius: 8px;
    padding: 20px;
    text-align: center;
  }
  :global(.dark) .stat-card { background: #1e293b; border-color: #334155; }
  .stat-card.warn { border-left: 4px solid #f59e0b; }
  .stat-card.alert { border-left: 4px solid #dc2626; }
  .stat-value { font-size: 32px; font-weight: 800; color: #1e293b; }
  :global(.dark) .stat-value { color: #f1f5f9; }
  .stat-label { font-size: 12px; color: #6b7280; text-transform: uppercase; letter-spacing: 0.5px; margin-top: 4px; }

  .dashboard-panels {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 20px;
  }
  .panel {
    background: white;
    border: 1px solid #e2e8f0;
    border-radius: 8px;
    padding: 20px;
  }
  :global(.dark) .panel { background: #1e293b; border-color: #334155; }
  .panel h3 { font-size: 15px; font-weight: 700; margin-bottom: 16px; }

  .reminder-list, .flag-list { display: flex; flex-direction: column; gap: 10px; }
  .reminder-item, .flag-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px;
    border-radius: 6px;
    background: #f8fafc;
  }
  :global(.dark) .reminder-item, :global(.dark) .flag-item { background: #0f172a; }
  .flag-item { gap: 12px; justify-content: flex-start; }
  .reminder-title { font-weight: 600; font-size: 13px; }
  .reminder-meta { font-size: 11px; color: #6b7280; margin-top: 2px; }
  .flag-accession { font-weight: 600; font-size: 13px; }
  .flag-message { font-size: 12px; color: #6b7280; }

  .chart-bars { display: flex; flex-direction: column; gap: 8px; }
  .bar-row { display: flex; align-items: center; gap: 10px; }
  .bar-label { width: 100px; font-size: 12px; font-weight: 600; text-transform: capitalize; }
  .bar-track { flex: 1; height: 20px; background: #f1f5f9; border-radius: 4px; overflow: hidden; }
  :global(.dark) .bar-track { background: #0f172a; }
  .bar-fill { height: 100%; background: #2563eb; border-radius: 4px; transition: width 0.3s; }
  .species-fill { background: #059669; }
  .bar-value { width: 40px; text-align: right; font-size: 13px; font-weight: 700; }

  .danger-panel {
    border: 1px solid rgba(220, 38, 38, 0.35);
    background: rgba(220, 38, 38, 0.04);
  }
  :global(.dark) .danger-panel { background: rgba(220, 38, 38, 0.08); }

  .reset-confirm {
    display: flex;
    flex-direction: column;
  }
</style>
