<script lang="ts">
  import { onMount } from 'svelte';
  import { open as shellOpen } from '@tauri-apps/plugin-shell';
  import { listErrorLogs, markErrorsRead, clearErrorLogs } from '../api';
  import { unreadErrorCount, navigateTo } from '../stores/app';
  import { currentUser } from '../stores/auth';

  const GITHUB_ISSUES = 'https://github.com/jnowat/SteloPTC/issues/new';

  let logs = $state<any[]>([]);
  let totalLogs = $state(0);
  let loading = $state(true);
  let expandedId = $state<string | null>(null);

  // Filters
  let filterSeverity = $state('');
  let filterModule = $state('');
  let filterUnreadOnly = $state(false);
  let currentPage = $state(1);
  const PER_PAGE = 25;

  let copyFeedback = $state<Record<string, string>>({});

  const severityOrder: Record<string, number> = { critical: 0, error: 1, warning: 2, info: 3 };

  async function load() {
    loading = true;
    try {
      const result = await listErrorLogs({
        severity: filterSeverity || undefined,
        module: filterModule || undefined,
        unread_only: filterUnreadOnly || undefined,
        page: currentPage,
        per_page: PER_PAGE,
      });
      logs = result.items;
      totalLogs = result.total;
    } catch (_e) {
      // silently ignore — don't recurse into error logger
    } finally {
      loading = false;
    }
  }

  async function handleMarkAllRead() {
    await markErrorsRead().catch(() => {});
    unreadErrorCount.set(0);
    await load();
  }

  async function handleClearAll() {
    if (!confirm('Clear all error logs? This cannot be undone.')) return;
    await clearErrorLogs().catch(() => {});
    await load();
    unreadErrorCount.set(0);
  }

  function toggleExpand(id: string) {
    expandedId = expandedId === id ? null : id;
  }

  async function copyToClipboard(text: string, key: string) {
    try {
      await navigator.clipboard.writeText(text);
      copyFeedback = { ...copyFeedback, [key]: 'Copied!' };
      setTimeout(() => {
        copyFeedback = { ...copyFeedback, [key]: '' };
      }, 1800);
    } catch {
      copyFeedback = { ...copyFeedback, [key]: 'Failed' };
    }
  }

  function buildCopyText(log: any): string {
    const lines = [
      `SteloPTC Error Log`,
      `==================`,
      `ID:        ${log.id}`,
      `Timestamp: ${log.timestamp}`,
      `Severity:  ${log.severity.toUpperCase()}`,
      `Title:     ${log.title}`,
      `Module:    ${log.module ?? '—'}`,
      `User:      ${log.username ?? '—'}`,
      ``,
      `Message:`,
      log.message,
    ];
    if (log.form_payload) {
      lines.push('', 'Form Payload:', log.form_payload);
    }
    if (log.stack_trace) {
      lines.push('', 'Stack Trace:', log.stack_trace);
    }
    return lines.join('\n');
  }

  async function reportOnGitHub(log: any) {
    const body = encodeURIComponent(
      `**Error Report from SteloPTC v0.1.10**\n\n` +
      `**Title:** ${log.title}\n` +
      `**Module:** ${log.module ?? '—'}\n` +
      `**Severity:** ${log.severity}\n` +
      `**Timestamp:** ${log.timestamp}\n\n` +
      `**Message:**\n\`\`\`\n${log.message}\n\`\`\`\n\n` +
      (log.form_payload ? `**Form Payload:**\n\`\`\`json\n${log.form_payload}\n\`\`\`\n\n` : '') +
      `*Please describe what you were doing when this occurred.*`
    );
    const title = encodeURIComponent(`[Bug] ${log.title}`);
    const url = `${GITHUB_ISSUES}?title=${title}&body=${body}&labels=bug`;
    await shellOpen(url).catch(() => {});
  }

  function severityClass(sev: string): string {
    switch (sev) {
      case 'critical': return 'sev-critical';
      case 'error':    return 'sev-error';
      case 'warning':  return 'sev-warning';
      default:         return 'sev-info';
    }
  }

  function formatTime(ts: string): string {
    try {
      return new Date(ts + (ts.includes('T') ? 'Z' : ' UTC')).toLocaleString();
    } catch {
      return ts;
    }
  }

  onMount(async () => {
    await load();
    // Mark as read when the page is opened
    await markErrorsRead().catch(() => {});
    unreadErrorCount.set(0);
  });

  $effect(() => {
    // Re-load when filters change
    currentPage = 1;
    load();
  });
</script>

<div class="el-page">
  <!-- Page header -->
  <div class="el-header">
    <div class="el-title-row">
      <div class="el-title-group">
        <div class="el-icon-wrap">
          <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/>
            <line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/>
          </svg>
        </div>
        <div>
          <h1 class="el-h1">Error Log</h1>
          <p class="el-subtitle">{totalLogs} total event{totalLogs !== 1 ? 's' : ''} · Application diagnostics &amp; error history</p>
        </div>
      </div>
      <div class="el-actions">
        <button class="el-btn el-btn-ghost" onclick={handleMarkAllRead}>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
            <polyline points="20 6 9 17 4 12"/>
          </svg>
          Mark all read
        </button>
        {#if $currentUser?.role === 'admin' || $currentUser?.role === 'supervisor'}
          <button class="el-btn el-btn-danger" onclick={handleClearAll}>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
              <polyline points="3 6 5 6 21 6"/><path d="M19 6l-1 14H6L5 6"/><path d="M10 11v6"/><path d="M14 11v6"/>
            </svg>
            Clear all
          </button>
        {/if}
      </div>
    </div>

    <!-- Filter bar -->
    <div class="el-filters">
      <div class="el-filter-group">
        <label class="el-filter-label">Severity</label>
        <select class="el-select" bind:value={filterSeverity}>
          <option value="">All severities</option>
          <option value="critical">Critical</option>
          <option value="error">Error</option>
          <option value="warning">Warning</option>
          <option value="info">Info</option>
        </select>
      </div>
      <div class="el-filter-group">
        <label class="el-filter-label">Module</label>
        <input class="el-input" type="text" bind:value={filterModule} placeholder="e.g. specimens.create" />
      </div>
      <label class="el-filter-check">
        <input type="checkbox" bind:checked={filterUnreadOnly} />
        <span>Unread only</span>
      </label>
    </div>
  </div>

  <!-- Log table -->
  <div class="el-card">
    {#if loading}
      <div class="el-loading">
        <div class="el-spinner"></div>
        <span>Loading error logs…</span>
      </div>
    {:else if logs.length === 0}
      <div class="el-empty">
        <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" opacity="0.3">
          <circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/>
        </svg>
        <p>No error logs found</p>
        <span>The application is running clean. Any errors captured here will appear in this log.</span>
      </div>
    {:else}
      <div class="el-table-wrap">
        <table class="el-table">
          <thead>
            <tr>
              <th style="width:160px;">Timestamp</th>
              <th style="width:90px;">Severity</th>
              <th>Title</th>
              <th style="width:160px;">Module</th>
              <th style="width:110px;">User</th>
              <th style="width:32px;"></th>
            </tr>
          </thead>
          <tbody>
            {#each logs as log (log.id)}
              <tr
                class="el-row"
                class:el-row-unread={!log.is_read}
                class:el-row-expanded={expandedId === log.id}
                onclick={() => toggleExpand(log.id)}
              >
                <td class="el-td-time">{formatTime(log.timestamp)}</td>
                <td>
                  <span class="el-sev-badge {severityClass(log.severity)}">
                    {log.severity}
                  </span>
                </td>
                <td class="el-td-title">
                  {#if !log.is_read}
                    <span class="el-unread-dot"></span>
                  {/if}
                  {log.title}
                </td>
                <td class="el-td-module">{log.module ?? '—'}</td>
                <td class="el-td-user">{log.username ?? '—'}</td>
                <td class="el-td-chevron">
                  <svg
                    width="14" height="14" viewBox="0 0 24 24" fill="none"
                    stroke="currentColor" stroke-width="2.5"
                    style="transition:transform 0.2s; transform:rotate({expandedId === log.id ? '90deg' : '0deg'})"
                  >
                    <polyline points="9 18 15 12 9 6"/>
                  </svg>
                </td>
              </tr>

              {#if expandedId === log.id}
                <tr class="el-detail-row">
                  <td colspan="6">
                    <div class="el-detail">
                      <!-- Message -->
                      <div class="el-detail-section">
                        <div class="el-detail-label">Error Message</div>
                        <div class="el-detail-msg">{log.message}</div>
                      </div>

                      <!-- Form payload -->
                      {#if log.form_payload}
                        <div class="el-detail-section">
                          <div class="el-detail-label">Form Payload</div>
                          <pre class="el-json">{log.form_payload}</pre>
                        </div>
                      {/if}

                      <!-- Stack trace -->
                      {#if log.stack_trace}
                        <div class="el-detail-section">
                          <div class="el-detail-label">Stack Trace</div>
                          <pre class="el-stack">{log.stack_trace}</pre>
                        </div>
                      {/if}

                      <!-- Meta row -->
                      <div class="el-detail-meta">
                        <span>ID: <code>{log.id}</code></span>
                        <span>User: <strong>{log.username ?? '—'}</strong></span>
                        <span>Module: <strong>{log.module ?? '—'}</strong></span>
                      </div>

                      <!-- Action buttons -->
                      <div class="el-detail-actions">
                        <button
                          class="el-btn el-btn-ghost"
                          onclick={(e) => { e.stopPropagation(); copyToClipboard(buildCopyText(log), log.id); }}
                        >
                          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                            <rect x="9" y="9" width="13" height="13" rx="2" ry="2"/>
                            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
                          </svg>
                          {copyFeedback[log.id] || 'Copy to clipboard'}
                        </button>
                        <button
                          class="el-btn el-btn-github"
                          onclick={(e) => { e.stopPropagation(); reportOnGitHub(log); }}
                        >
                          <svg width="13" height="13" viewBox="0 0 24 24" fill="currentColor">
                            <path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0 0 24 12c0-6.63-5.37-12-12-12z"/>
                          </svg>
                          Report on GitHub
                        </button>
                      </div>
                    </div>
                  </td>
                </tr>
              {/if}
            {/each}
          </tbody>
        </table>
      </div>

      <!-- Pagination -->
      {#if totalLogs > PER_PAGE}
        <div class="el-pagination">
          <button
            class="el-btn el-btn-ghost el-btn-sm"
            disabled={currentPage === 1}
            onclick={() => { currentPage--; load(); }}
          >← Prev</button>
          <span class="el-page-info">
            Page {currentPage} of {Math.ceil(totalLogs / PER_PAGE)}
          </span>
          <button
            class="el-btn el-btn-ghost el-btn-sm"
            disabled={currentPage >= Math.ceil(totalLogs / PER_PAGE)}
            onclick={() => { currentPage++; load(); }}
          >Next →</button>
        </div>
      {/if}
    {/if}
  </div>
</div>

<style>
  .el-page {
    max-width: 1100px;
    margin: 0 auto;
    animation: fadeUp 0.25s ease-out;
  }

  @keyframes fadeUp {
    from { opacity: 0; transform: translateY(10px); }
    to { opacity: 1; transform: translateY(0); }
  }

  /* ─── Header ─────────────────────────────────────────── */
  .el-header {
    margin-bottom: 20px;
  }
  .el-title-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 16px;
    gap: 16px;
  }
  .el-title-group {
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .el-icon-wrap {
    width: 44px;
    height: 44px;
    border-radius: 12px;
    background: linear-gradient(135deg, rgba(239,68,68,0.2), rgba(220,38,38,0.1));
    border: 1px solid rgba(239,68,68,0.3);
    display: flex;
    align-items: center;
    justify-content: center;
    color: #f87171;
    flex-shrink: 0;
  }
  .el-h1 {
    font-size: 22px;
    font-weight: 700;
    color: #f1f5f9;
    letter-spacing: -0.4px;
  }
  .el-subtitle {
    font-size: 12px;
    color: #64748b;
    margin-top: 2px;
  }

  .el-actions {
    display: flex;
    gap: 8px;
    flex-shrink: 0;
  }

  /* ─── Filters ─────────────────────────────────────────── */
  .el-filters {
    display: flex;
    align-items: flex-end;
    gap: 16px;
    flex-wrap: wrap;
  }
  .el-filter-group {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .el-filter-label {
    font-size: 11px;
    font-weight: 600;
    color: #64748b;
    letter-spacing: 0.4px;
    text-transform: uppercase;
  }
  .el-select,
  .el-input {
    height: 34px;
    padding: 0 10px;
    border-radius: 6px;
    border: 1px solid #334155;
    background: #0f172a;
    color: #e2e8f0;
    font-size: 13px;
    min-width: 160px;
  }
  .el-select:focus,
  .el-input:focus {
    outline: none;
    border-color: #3b82f6;
    box-shadow: 0 0 0 3px rgba(59,130,246,0.12);
  }
  .el-filter-check {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 13px;
    color: #94a3b8;
    cursor: pointer;
    padding-bottom: 2px;
  }
  .el-filter-check input {
    width: auto;
    accent-color: #3b82f6;
  }

  /* ─── Card ────────────────────────────────────────────── */
  .el-card {
    background: rgba(30,41,59,0.85);
    backdrop-filter: blur(12px);
    border: 1px solid #334155;
    border-radius: 12px;
    overflow: hidden;
  }

  /* ─── Loading / Empty ─────────────────────────────────── */
  .el-loading, .el-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 60px 24px;
    gap: 12px;
    color: #475569;
  }
  .el-empty p { font-size: 15px; font-weight: 600; color: #64748b; margin: 0; }
  .el-empty span { font-size: 13px; text-align: center; max-width: 360px; line-height: 1.6; }
  .el-spinner {
    width: 28px; height: 28px;
    border: 3px solid #334155;
    border-top-color: #3b82f6;
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }

  /* ─── Table ───────────────────────────────────────────── */
  .el-table-wrap { overflow-x: auto; }
  .el-table {
    width: 100%;
    border-collapse: collapse;
  }
  .el-table th {
    text-align: left;
    padding: 10px 14px;
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: #475569;
    border-bottom: 1px solid #1e293b;
    background: rgba(15,23,42,0.5);
  }
  .el-row {
    cursor: pointer;
    transition: background 0.12s;
  }
  .el-row:hover { background: rgba(51,65,85,0.45); }
  .el-row td {
    padding: 11px 14px;
    font-size: 13px;
    border-bottom: 1px solid #1e293b;
    color: #cbd5e1;
    vertical-align: middle;
  }
  .el-row-unread td { color: #e2e8f0; }
  .el-row-expanded { background: rgba(51,65,85,0.3); }

  .el-td-time { color: #64748b; font-size: 12px; white-space: nowrap; }
  .el-td-title { font-weight: 500; display: flex; align-items: center; gap: 8px; }
  .el-td-module { font-size: 12px; color: #64748b; font-family: 'Fira Code', monospace; }
  .el-td-user { font-size: 12px; }
  .el-td-chevron { color: #475569; }

  .el-unread-dot {
    width: 7px; height: 7px;
    border-radius: 50%;
    background: #3b82f6;
    flex-shrink: 0;
    box-shadow: 0 0 6px rgba(59,130,246,0.6);
  }

  /* ─── Severity badges ──────────────────────────────────── */
  .el-sev-badge {
    display: inline-flex;
    align-items: center;
    padding: 2px 8px;
    border-radius: 20px;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.2px;
    text-transform: uppercase;
  }
  .sev-critical { background: rgba(220,38,38,0.25); color: #fca5a5; border: 1px solid rgba(220,38,38,0.35); }
  .sev-error    { background: rgba(239,68,68,0.15);  color: #f87171; border: 1px solid rgba(239,68,68,0.25); }
  .sev-warning  { background: rgba(245,158,11,0.15); color: #fbbf24; border: 1px solid rgba(245,158,11,0.25); }
  .sev-info     { background: rgba(59,130,246,0.15); color: #93c5fd; border: 1px solid rgba(59,130,246,0.25); }

  /* ─── Expanded detail ──────────────────────────────────── */
  .el-detail-row td {
    padding: 0;
    border-bottom: 1px solid #1e293b;
    background: rgba(15,23,42,0.6);
  }
  .el-detail {
    padding: 20px 24px;
    display: flex;
    flex-direction: column;
    gap: 16px;
    animation: slideDown 0.18s ease-out;
  }
  @keyframes slideDown {
    from { opacity: 0; transform: translateY(-6px); }
    to   { opacity: 1; transform: translateY(0); }
  }
  .el-detail-section { display: flex; flex-direction: column; gap: 6px; }
  .el-detail-label {
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: #475569;
  }
  .el-detail-msg {
    font-size: 13px;
    color: #e2e8f0;
    line-height: 1.6;
    padding: 12px 16px;
    background: rgba(30,41,59,0.7);
    border: 1px solid #334155;
    border-radius: 8px;
    word-break: break-word;
  }
  .el-json, .el-stack {
    font-family: 'Fira Code', 'Cascadia Code', 'JetBrains Mono', 'Courier New', monospace;
    font-size: 12px;
    line-height: 1.6;
    color: #a5f3fc;
    background: rgba(8,145,178,0.06);
    border: 1px solid rgba(8,145,178,0.15);
    border-radius: 8px;
    padding: 14px 16px;
    overflow-x: auto;
    white-space: pre;
    margin: 0;
  }
  .el-stack { color: #fca5a5; background: rgba(220,38,38,0.05); border-color: rgba(220,38,38,0.15); }

  .el-detail-meta {
    display: flex;
    gap: 20px;
    font-size: 12px;
    color: #475569;
    flex-wrap: wrap;
  }
  .el-detail-meta code {
    font-family: monospace;
    background: #1e293b;
    padding: 1px 5px;
    border-radius: 3px;
    font-size: 11px;
    color: #94a3b8;
  }
  .el-detail-meta strong { color: #94a3b8; }

  .el-detail-actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  /* ─── Pagination ───────────────────────────────────────── */
  .el-pagination {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 14px;
    border-top: 1px solid #1e293b;
  }
  .el-page-info { font-size: 13px; color: #64748b; }

  /* ─── Buttons ──────────────────────────────────────────── */
  .el-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 7px 14px;
    border-radius: 7px;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s;
    border: none;
    white-space: nowrap;
  }
  .el-btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .el-btn-sm { padding: 5px 10px; font-size: 12px; }

  .el-btn-ghost {
    background: rgba(51,65,85,0.5);
    color: #94a3b8;
    border: 1px solid #334155;
  }
  .el-btn-ghost:hover:not(:disabled) { background: #334155; color: #e2e8f0; }

  .el-btn-danger {
    background: rgba(220,38,38,0.15);
    color: #f87171;
    border: 1px solid rgba(220,38,38,0.25);
  }
  .el-btn-danger:hover { background: rgba(220,38,38,0.25); color: #fca5a5; }

  .el-btn-github {
    background: rgba(30,41,59,0.8);
    color: #cbd5e1;
    border: 1px solid #334155;
  }
  .el-btn-github:hover { background: #334155; color: #f1f5f9; }
</style>
