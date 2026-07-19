<script lang="ts">
  import { addNotification } from '../stores/app';
  import { currentUser } from '../stores/auth';
  import { runDataIntegrityCheck, type IntegrityReport } from '../api';

  // WP-76: Lab data-integrity self-check. Runs a battery of read-only referential
  // and audit-chain invariant checks and reports any corruption (orphaned rows,
  // broken lineage links, audit-chain gaps) the operator should repair.

  let open = $state(false);
  let running = $state(false);
  let report = $state<IntegrityReport | null>(null);

  const isAdmin = $derived($currentUser?.role === 'admin');

  async function toggle() {
    open = !open;
  }

  async function run() {
    running = true;
    try {
      report = await runDataIntegrityCheck();
      addNotification(
        report.ok ? 'Data integrity check passed — no issues found.' : `Found ${report.issues.length} integrity issue(s).`,
        report.ok ? 'success' : 'error',
      );
    } catch (e: any) {
      addNotification(e?.message || 'Integrity check failed', 'error');
    } finally {
      running = false;
    }
  }

  function severityClass(s: string): string {
    return s === 'critical' ? 'di-critical' : s === 'high' ? 'di-high' : 'di-normal';
  }
</script>

<div class="card" style="margin-bottom:16px;">
  <div class="di-header">
    <strong>🩺 Data Integrity Self-Check</strong>
    <button class="btn btn-sm" onclick={toggle}>{open ? 'Hide' : 'Show'}</button>
  </div>

  {#if open}
    <p class="di-intro">
      Scans the database for referential problems SQLite can't retroactively catch —
      orphaned specimens, passages pointing at deleted parents, strains without a
      species — plus audit-lineage sequence gaps (a removed history row). Read-only;
      administrator access required.
    </p>

    {#if !isAdmin}
      <p class="di-empty">Administrator access is required to run the integrity check.</p>
    {:else}
      <div class="di-actions">
        <button class="btn btn-sm btn-primary" disabled={running} onclick={run}>
          {running ? 'Scanning…' : 'Run Integrity Check'}
        </button>
        {#if report}
          <span class={report.ok ? 'di-ok' : 'di-fail'}>
            {report.ok ? `✓ All ${report.checks_run} checks passed` : `✗ ${report.issues.length} of ${report.checks_run} checks failed`}
          </span>
        {/if}
      </div>

      {#if report && !report.ok}
        <div class="di-table-wrap">
          <table class="di-table">
            <thead>
              <tr><th>Severity</th><th>Issue</th><th>Count</th><th>Examples</th></tr>
            </thead>
            <tbody>
              {#each report.issues as issue}
                <tr>
                  <td><span class="di-badge {severityClass(issue.severity)}">{issue.severity}</span></td>
                  <td>{issue.title}</td>
                  <td>{issue.count}</td>
                  <td class="di-examples">{issue.examples.join(', ')}{issue.count > issue.examples.length ? ' …' : ''}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    {/if}
  {/if}
</div>

<style>
  .di-header { display: flex; align-items: center; justify-content: space-between; }
  .di-intro { font-size: 13px; color: #6b7280; margin: 8px 0 12px; }
  :global(.dark) .di-intro { color: #94a3b8; }
  .di-actions { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; margin-bottom: 10px; }
  .di-ok { color: #16a34a; font-weight: 600; font-size: 13px; }
  .di-fail { color: #dc2626; font-weight: 600; font-size: 13px; }
  .di-empty { font-size: 13px; color: #6b7280; }
  .di-table-wrap { overflow-x: auto; }
  .di-table { width: 100%; border-collapse: collapse; font-size: 13px; }
  .di-table th, .di-table td { text-align: left; padding: 6px 10px; border-bottom: 1px solid #e2e8f0; }
  :global(.dark) .di-table th, :global(.dark) .di-table td { border-color: #334155; }
  .di-badge { display: inline-block; padding: 2px 8px; border-radius: 999px; font-size: 11px; font-weight: 600; }
  .di-critical { background: #fee2e2; color: #991b1b; }
  .di-high { background: #fef3c7; color: #92400e; }
  .di-normal { background: #dbeafe; color: #1e40af; }
  .di-examples { font-family: ui-monospace, monospace; font-size: 12px; color: #6b7280; }
</style>
