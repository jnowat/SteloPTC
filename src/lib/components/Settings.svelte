<script lang="ts">
  import { onMount } from 'svelte';
  import { labProfile, LAB_PROFILE_LABELS, loadLabProfile, type LabProfile } from '../profile';
  import {
    setLabProfile, getSpecimenStats,
    getBackendConfig, setBackendType, testPostgresConnection,
    getSyncStatus, type BackendConfigInfo, type SyncStatusResponse,
    getNotificationPreferences, setNotificationPreference,
    getSmtpConfig, setSmtpConfig, sendTestDesktopNotification, sendTestEmail,
    type NotificationPreference, type SmtpConfig,
  } from '../api';
  import { addNotification } from '../stores/app';
  import { currentUser } from '../stores/auth';
  import PermissionsEditor from './PermissionsEditor.svelte';

  const PROFILES: LabProfile[] = ['plant_tissue_culture', 'cell_culture', 'mycology'];

  let selected = $state<LabProfile>($labProfile);
  let confirmation = $state('');
  let saving = $state(false);
  let hasData = $state(true);   // conservative until proven otherwise
  let loading = $state(true);

  let changed = $derived(selected !== $labProfile);
  // No phrase required when the lab is empty — backend allows the change unconditionally.
  let confirmed = $derived(!hasData || confirmation.trim() === 'CHANGE PROFILE');

  onMount(async () => {
    loading = true;
    try {
      await loadLabProfile();
      selected = $labProfile;
      const stats = await getSpecimenStats();
      hasData = (stats?.total ?? 0) > 0;
    } catch {
      // On any error keep hasData = true so the confirmation phrase is still required.
    } finally {
      loading = false;
    }
  });

  // WP-50/WP-51 — Multi-user backend + LAN sync foundation (preview).
  let backendConfig = $state<BackendConfigInfo | null>(null);
  let syncStatus = $state<SyncStatusResponse | null>(null);
  let multiUserLoading = $state(true);
  let backendSelected = $state<'sqlite' | 'postgres'>('sqlite');
  let connectionStringInput = $state('');
  let testingConnection = $state(false);
  let testResult = $state<{ ok: boolean; message: string } | null>(null);
  let savingBackendType = $state(false);

  onMount(async () => {
    multiUserLoading = true;
    try {
      backendConfig = await getBackendConfig();
      backendSelected = backendConfig.backend_type;
      syncStatus = await getSyncStatus();
    } catch {
      // Leave both null — the section renders a quiet "unavailable" state below.
    } finally {
      multiUserLoading = false;
    }
  });

  async function handleTestConnection() {
    testingConnection = true;
    testResult = null;
    try {
      const message = await testPostgresConnection(connectionStringInput);
      testResult = { ok: true, message };
    } catch (e: any) {
      testResult = { ok: false, message: e.message };
    } finally {
      testingConnection = false;
    }
  }

  async function handleSaveBackendType() {
    savingBackendType = true;
    try {
      await setBackendType(
        backendSelected,
        backendSelected === 'postgres' ? connectionStringInput : undefined,
      );
      backendConfig = await getBackendConfig();
      addNotification(
        `Intended backend recorded as ${backendSelected === 'postgres' ? 'PostgreSQL' : 'SQLite'}. ` +
        `The active database is unchanged — this records a future-migration preference only.`,
        'success',
      );
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      savingBackendType = false;
    }
  }

  function handleCancel() {
    selected = $labProfile;
    confirmation = '';
  }

  async function handleApply() {
    if (!changed) return;
    saving = true;
    try {
      // Pass the phrase only when the backend will actually check it (hasData).
      await setLabProfile(selected, hasData ? confirmation.trim() : undefined);
      labProfile.set(selected);
      confirmation = '';
      addNotification(`Lab profile changed to ${LAB_PROFILE_LABELS[selected]}`, 'success');
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      saving = false;
    }
  }

  // WP-52 — Notification preferences (self-service, every user) + SMTP config (admin only).
  const SEVERITY_OPTIONS = ['normal', 'high', 'critical'] as const;

  let notifPrefs = $state<NotificationPreference[]>([]);
  let notifLoading = $state(true);
  let testingDesktop = $state(false);

  let smtpConfig = $state<SmtpConfig | null>(null);
  let smtpLoading = $state(true);
  let smtpForm = $state({ host: '', port: 587, username: '', password: '', from_address: '', use_tls: true });
  let savingSmtp = $state(false);
  let testEmailAddress = $state('');
  let testingEmail = $state(false);

  onMount(async () => {
    notifLoading = true;
    try {
      notifPrefs = await getNotificationPreferences();
    } catch {
      // Leave empty — the UI falls back to "enabled, normal" defaults per channel.
    } finally {
      notifLoading = false;
    }

    if ($currentUser?.role === 'admin') {
      await loadSmtpConfig();
    } else {
      smtpLoading = false;
    }
  });

  function prefFor(channel: string): { enabled: boolean; min_severity: string } {
    const p = notifPrefs.find((p) => p.channel === channel);
    return p ? { enabled: p.enabled, min_severity: p.min_severity } : { enabled: true, min_severity: 'normal' };
  }

  async function updatePreference(channel: string, enabled: boolean, minSeverity: string) {
    try {
      await setNotificationPreference(channel, enabled, minSeverity);
      const idx = notifPrefs.findIndex((p) => p.channel === channel);
      if (idx >= 0) {
        notifPrefs[idx] = { ...notifPrefs[idx], enabled, min_severity: minSeverity as any };
      } else {
        notifPrefs = [...notifPrefs, { id: channel, user_id: '', channel: channel as any, enabled, min_severity: minSeverity as any }];
      }
    } catch (e: any) {
      addNotification(e.message, 'error');
    }
  }

  async function handleTestDesktopNotification() {
    testingDesktop = true;
    try {
      await sendTestDesktopNotification();
      addNotification('Test notification sent — check your desktop notification tray', 'success');
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      testingDesktop = false;
    }
  }

  async function loadSmtpConfig() {
    smtpLoading = true;
    try {
      smtpConfig = await getSmtpConfig();
      smtpForm = {
        host: smtpConfig.host ?? '',
        port: smtpConfig.port,
        username: smtpConfig.username ?? '',
        password: '',
        from_address: smtpConfig.from_address ?? '',
        use_tls: smtpConfig.use_tls,
      };
    } catch {
      // Leave null — the SMTP card shows its own quiet unavailable state.
    } finally {
      smtpLoading = false;
    }
  }

  async function handleSaveSmtp() {
    savingSmtp = true;
    try {
      await setSmtpConfig({
        host: smtpForm.host || undefined,
        port: smtpForm.port,
        username: smtpForm.username || undefined,
        password: smtpForm.password || undefined,
        from_address: smtpForm.from_address || undefined,
        use_tls: smtpForm.use_tls,
      });
      smtpForm.password = '';
      await loadSmtpConfig();
      addNotification('SMTP configuration saved', 'success');
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      savingSmtp = false;
    }
  }

  async function handleTestEmail() {
    if (!testEmailAddress.trim()) return;
    testingEmail = true;
    try {
      await sendTestEmail(testEmailAddress.trim());
      addNotification(`Test email sent to ${testEmailAddress.trim()}`, 'success');
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      testingEmail = false;
    }
  }
</script>

<div>
  <div class="page-header">
    <h1>Settings</h1>
  </div>

  <!-- Notification Preferences — every user configures their own (WP-52) -->
  <div class="card" style="max-width: 640px; margin-bottom: 24px;">
    <h2 style="font-size: 16px; font-weight: 700; margin-bottom: 4px;">
      Notification Preferences <span class="new-feature-badge">New</span>
    </h2>
    <p style="font-size: 13px; color: #6b7280; margin-bottom: 20px;">
      Choose which channels notify you about overdue Work Queue items, and the minimum severity
      that should reach you on each one. Desktop and email notifications are now available (v1.39.0).
    </p>

    {#if notifLoading}
      <div class="loading-pulse" aria-busy="true" aria-label="Loading notification preferences"></div>
    {:else}
      {#each ['desktop', 'email'] as channel}
        {@const pref = prefFor(channel)}
        <div class="form-row" style="align-items:center; margin-bottom: 12px;">
          <label class="notif-channel-toggle" for="notif-enabled-{channel}">
            <input
              id="notif-enabled-{channel}"
              type="checkbox"
              checked={pref.enabled}
              onchange={(e) => updatePreference(channel, (e.currentTarget as HTMLInputElement).checked, pref.min_severity)}
            />
            {channel === 'desktop' ? 'Desktop notifications' : 'Email notifications'}
          </label>
          <div class="form-group" style="flex: 0 0 160px; margin: 0 0 0 16px;">
            <label for="notif-severity-{channel}" class="visually-hidden">Minimum severity for {channel}</label>
            <select
              id="notif-severity-{channel}"
              value={pref.min_severity}
              disabled={!pref.enabled}
              onchange={(e) => updatePreference(channel, pref.enabled, (e.currentTarget as HTMLSelectElement).value)}
            >
              {#each SEVERITY_OPTIONS as sev}
                <option value={sev}>{sev} and above</option>
              {/each}
            </select>
          </div>
        </div>
      {/each}
      <button class="btn" onclick={handleTestDesktopNotification} disabled={testingDesktop} title="Send a test desktop notification">
        {testingDesktop ? 'Sending…' : 'Send Test Desktop Notification'}
      </button>
    {/if}
  </div>

  {#if $currentUser?.role !== 'admin'}
    <div class="card">
      <p style="color: var(--color-text-muted, #6b7280);">Only administrators can change lab-wide settings.</p>
    </div>
  {:else}
    <!-- Lab Profile -->
    <div class="card" style="max-width: 640px; margin-bottom: 24px;">
      <h2 style="font-size: 16px; font-weight: 700; margin-bottom: 4px;">Lab Profile</h2>
      <p style="font-size: 13px; color: #6b7280; margin-bottom: 20px;">
        Determines which vocabulary entries (stages, propagation methods, hormones, etc.) are
        available throughout the application. Choose the profile that matches your lab's work.
      </p>

      {#if loading}
        <div class="loading-pulse" aria-busy="true" aria-label="Loading profile settings"></div>
      {:else}
        <div class="form-group">
          <label for="profile-select" title="Select the active lab profile">Active Profile</label>
          <select
            id="profile-select"
            bind:value={selected}
            title="Choose a lab profile"
          >
            {#each PROFILES as p}
              <option value={p}>{LAB_PROFILE_LABELS[p]}</option>
            {/each}
          </select>
        </div>

        {#if changed}
          {#if hasData}
            <!-- Lab has specimens: full warning + phrase confirmation -->
            <div class="warning-box" role="alert">
              <strong>This lab has existing specimen data</strong>
              <ul>
                <li>Vocabulary dropdowns (stages, hormones, propagation methods) will reflect the new profile immediately.</li>
                <li>Existing specimen records are <em>not</em> deleted, but their current stage values may not appear in the new profile's stage list.</li>
                <li>If the new profile has no seeded vocabulary data, some dropdowns will be empty until data is added.</li>
                <li>This change is logged in the audit trail.</li>
              </ul>
            </div>

            <div class="form-group" style="margin-top: 16px;">
              <label for="confirm-input" title="Type CHANGE PROFILE to confirm">
                Type <code>CHANGE PROFILE</code> to confirm
              </label>
              <input
                id="confirm-input"
                type="text"
                placeholder="CHANGE PROFILE"
                bind:value={confirmation}
                title="Type CHANGE PROFILE exactly to enable the Apply button"
                autocomplete="off"
              />
            </div>
          {:else}
            <!-- Empty lab: lighter notice, no phrase required -->
            <div class="info-notice" role="note">
              <strong>No specimen data exists</strong>
              <p>
                The lab is empty, so this profile change takes effect immediately with no
                confirmation phrase required. Vocabulary dropdowns will reflect the new profile
                on next use. If the new profile has no seeded vocabulary data, some dropdowns
                will be empty until data is added.
              </p>
            </div>
          {/if}

          <div class="action-row">
            <button
              class="btn"
              onclick={handleCancel}
              disabled={saving}
              title="Discard the profile change"
            >
              Cancel
            </button>
            <button
              class="btn btn-primary"
              onclick={handleApply}
              disabled={saving || !confirmed}
              title={confirmed
                ? 'Apply the profile change'
                : 'Type CHANGE PROFILE above to enable this button'}
            >
              {saving ? 'Applying…' : 'Apply Profile Change'}
            </button>
          </div>
        {/if}

        {#if !changed}
          <div class="current-badge">
            Current: <strong>{LAB_PROFILE_LABELS[$labProfile]}</strong>
          </div>
        {/if}
      {/if}
    </div>

    <!-- Vocabulary notice -->
    <div class="card info-box" style="max-width: 640px;">
      <strong>Vocabulary notice</strong>
      <p>
        If vocabulary dropdowns appear empty after switching profiles, it means the new profile
        has no seeded entries yet. An administrator can add vocabulary entries via the database
        migration process or by contacting the system owner.
      </p>
    </div>

    <!-- Multi-User Backend (Preview) — WP-50 / WP-51 foundation -->
    <div class="card" style="max-width: 640px; margin-top: 24px;">
      <h2 style="font-size: 16px; font-weight: 700; margin-bottom: 4px;">Multi-User Backend (Preview)</h2>
      <p style="font-size: 13px; color: #6b7280; margin-bottom: 20px;">
        Foundation work for multi-user deployments. SQLite remains the active database for all
        reads and writes — nothing below changes that. This section records a future backend
        preference and previews the LAN sync data model.
      </p>

      {#if multiUserLoading}
        <div class="loading-pulse" aria-busy="true" aria-label="Loading backend settings"></div>
      {:else if !backendConfig}
        <div class="info-box" style="margin-bottom: 12px;">
          <p>Backend configuration is currently unavailable.</p>
        </div>
      {:else}
        <div class="current-badge" style="margin-bottom: 16px;">
          Active database: <strong>SQLite</strong> · Intended backend: <strong>{backendConfig.backend_type === 'postgres' ? 'PostgreSQL' : 'SQLite'}</strong>
        </div>

        {#if !backendConfig.postgres_feature_compiled}
          <div class="info-notice" role="note">
            <strong>PostgreSQL support not compiled in</strong>
            <p>
              This build was compiled without the <code>postgres</code> Cargo feature.
              Connectivity testing and schema bootstrap are unavailable until a build with
              <code>--features postgres</code> is used.
            </p>
          </div>
        {:else}
          <div class="form-group">
            <label for="backend-select" title="Select the intended database backend">Intended Backend</label>
            <select id="backend-select" bind:value={backendSelected} title="Choose the intended backend">
              <option value="sqlite">SQLite (default, active)</option>
              <option value="postgres">PostgreSQL (foundation only — not yet active)</option>
            </select>
          </div>

          {#if backendSelected === 'postgres'}
            <div class="form-group" style="margin-top: 12px;">
              <label for="pg-connection-string" title="PostgreSQL connection string">
                Connection String
              </label>
              <input
                id="pg-connection-string"
                type="text"
                placeholder="postgres://user:password@host:5432/database"
                bind:value={connectionStringInput}
                title="postgres:// or postgresql:// connection string — never saved to disk"
                autocomplete="off"
              />
              <p style="font-size: 12px; color: #6b7280; margin-top: 4px;">
                Never persisted — supplied fresh for each test or save action.
              </p>
            </div>

            <div class="action-row">
              <button
                class="btn"
                onclick={handleTestConnection}
                disabled={testingConnection || !connectionStringInput.trim()}
                title="Test connectivity to the PostgreSQL server"
              >
                {testingConnection ? 'Testing…' : 'Test Connection'}
              </button>
            </div>

            {#if testResult}
              <div class={testResult.ok ? 'info-notice' : 'warning-box'} style="margin-top: 12px;" role={testResult.ok ? 'note' : 'alert'}>
                <p style="margin:0;">{testResult.message}</p>
              </div>
            {/if}
          {/if}

          <div class="action-row">
            <button
              class="btn btn-primary"
              onclick={handleSaveBackendType}
              disabled={savingBackendType || backendSelected === backendConfig.backend_type}
              title="Save the intended backend preference (does not change the active database)"
            >
              {savingBackendType ? 'Saving…' : 'Save Preference'}
            </button>
          </div>
        {/if}

        {#if syncStatus}
          <h3 style="font-size: 14px; font-weight: 700; margin: 24px 0 8px;">Sync Status (Preview)</h3>
          <p style="font-size: 12px; color: #6b7280; margin-bottom: 12px;">
            LAN discovery and networking are not yet implemented. These counts reflect the
            audit-chain data foundation only.
          </p>
          <div class="sync-stats-grid">
            <div class="sync-stat">
              <span class="sync-stat-value">{syncStatus.lineages_tracked}</span>
              <span class="sync-stat-label">Lineages tracked</span>
            </div>
            <div class="sync-stat">
              <span class="sync-stat-value">{syncStatus.unresolved_conflicts}</span>
              <span class="sync-stat-label">Unresolved conflicts</span>
            </div>
            <div class="sync-stat">
              <span class="sync-stat-value">{syncStatus.known_peers}</span>
              <span class="sync-stat-label">Known peers</span>
            </div>
          </div>
        {/if}
      {/if}
    </div>

    <!-- SMTP Configuration (admin only) — WP-52 -->
    <div class="card" style="max-width: 640px; margin-top: 24px;">
      <h2 style="font-size: 16px; font-weight: 700; margin-bottom: 4px;">Email (SMTP) Configuration</h2>
      <p style="font-size: 13px; color: #6b7280; margin-bottom: 20px;">
        Used to send email notifications for overdue Work Queue items. The password is never
        displayed once saved — leave it blank to keep the current one.
      </p>

      {#if smtpLoading}
        <div class="loading-pulse" aria-busy="true" aria-label="Loading SMTP configuration"></div>
      {:else}
        <div class="form-row">
          <div class="form-group" style="flex:2;">
            <label for="smtp-host">Host</label>
            <input id="smtp-host" type="text" placeholder="smtp.example.com" bind:value={smtpForm.host} />
          </div>
          <div class="form-group" style="flex:1;">
            <label for="smtp-port">Port</label>
            <input id="smtp-port" type="number" bind:value={smtpForm.port} />
          </div>
        </div>
        <div class="form-row">
          <div class="form-group" style="flex:1;">
            <label for="smtp-username">Username</label>
            <input id="smtp-username" type="text" bind:value={smtpForm.username} autocomplete="off" />
          </div>
          <div class="form-group" style="flex:1;">
            <label for="smtp-password">Password {smtpConfig?.password_set ? '(currently set)' : ''}</label>
            <input id="smtp-password" type="password" placeholder="Leave blank to keep current" bind:value={smtpForm.password} autocomplete="new-password" />
          </div>
        </div>
        <div class="form-group">
          <label for="smtp-from">From Address</label>
          <input id="smtp-from" type="email" placeholder="lab@example.com" bind:value={smtpForm.from_address} />
        </div>
        <div class="form-group" style="margin-bottom: 16px;">
          <label class="notif-channel-toggle" for="smtp-tls">
            <input id="smtp-tls" type="checkbox" bind:checked={smtpForm.use_tls} />
            Use STARTTLS
          </label>
        </div>
        <div class="action-row">
          <button class="btn btn-primary" onclick={handleSaveSmtp} disabled={savingSmtp} title="Save SMTP configuration">
            {savingSmtp ? 'Saving…' : 'Save SMTP Configuration'}
          </button>
        </div>

        <div class="form-row" style="margin-top: 20px; align-items: flex-end;">
          <div class="form-group" style="flex:1;">
            <label for="smtp-test-address">Send test email to</label>
            <input id="smtp-test-address" type="email" placeholder="you@example.com" bind:value={testEmailAddress} />
          </div>
          <button class="btn" onclick={handleTestEmail} disabled={testingEmail || !testEmailAddress.trim()} title="Send a test email using the saved SMTP configuration">
            {testingEmail ? 'Sending…' : 'Send Test Email'}
          </button>
        </div>
      {/if}
    </div>

    <!-- Field-Level Permissions (admin only) — WP-55 -->
    <div class="card" style="max-width: 900px; margin-top: 24px;">
      <h2 style="font-size: 16px; font-weight: 700; margin-bottom: 4px;">Field-Level Permissions</h2>
      <PermissionsEditor />
    </div>
  {/if}
</div>

<style>
  .loading-pulse {
    height: 36px;
    border-radius: 6px;
    background: linear-gradient(90deg, #e2e8f0 25%, #f1f5f9 50%, #e2e8f0 75%);
    background-size: 200% 100%;
    animation: shimmer 1.4s ease-in-out infinite;
    margin-bottom: 16px;
  }
  :global(.dark) .loading-pulse {
    background: linear-gradient(90deg, #1e293b 25%, #334155 50%, #1e293b 75%);
    background-size: 200% 100%;
  }
  @keyframes shimmer {
    0%   { background-position: 200% 0; }
    100% { background-position: -200% 0; }
  }

  .warning-box {
    background: #fef9c3;
    border: 1px solid #fde047;
    border-radius: 8px;
    padding: 14px 16px;
    font-size: 13px;
    color: #713f12;
    margin-top: 4px;
  }
  .warning-box strong {
    display: block;
    font-weight: 700;
    margin-bottom: 8px;
  }
  .warning-box ul {
    margin: 0;
    padding-left: 18px;
    line-height: 1.7;
  }
  :global(.dark) .warning-box {
    background: #422006;
    border-color: #92400e;
    color: #fef3c7;
  }

  .info-notice {
    background: #eff6ff;
    border: 1px solid #bfdbfe;
    border-radius: 8px;
    padding: 14px 16px;
    font-size: 13px;
    color: #1e40af;
    margin-top: 4px;
  }
  .info-notice strong {
    display: block;
    font-weight: 700;
    margin-bottom: 6px;
  }
  .info-notice p { margin: 0; line-height: 1.6; }
  :global(.dark) .info-notice {
    background: #1e3a5f;
    border-color: #2563eb;
    color: #bfdbfe;
  }

  .info-box {
    font-size: 13px;
    color: #374151;
  }
  .info-box strong {
    display: block;
    font-weight: 700;
    margin-bottom: 6px;
  }
  .info-box p { margin: 0; line-height: 1.6; }
  :global(.dark) .info-box { color: #cbd5e1; }

  .action-row {
    display: flex;
    gap: 10px;
    margin-top: 16px;
  }

  .current-badge {
    display: inline-block;
    margin-top: 12px;
    padding: 6px 14px;
    background: #f1f5f9;
    border-radius: 20px;
    font-size: 13px;
    color: #475569;
  }
  :global(.dark) .current-badge {
    background: #1e293b;
    color: #94a3b8;
  }

  code {
    font-family: 'Courier New', monospace;
    background: #f1f5f9;
    padding: 1px 5px;
    border-radius: 3px;
    font-size: 12px;
  }
  :global(.dark) code {
    background: #0f172a;
    color: #e2e8f0;
  }

  .sync-stats-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 12px;
  }
  .sync-stat {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 12px 8px;
    background: #f8fafc;
    border-radius: 8px;
  }
  :global(.dark) .sync-stat {
    background: #1e293b;
  }
  .sync-stat-value {
    font-size: 20px;
    font-weight: 700;
    color: #1e293b;
  }
  :global(.dark) .sync-stat-value {
    color: #e2e8f0;
  }
  .sync-stat-label {
    font-size: 11px;
    color: #6b7280;
    margin-top: 2px;
    text-align: center;
  }

  .notif-channel-toggle {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: 14px;
    cursor: pointer;
  }
  .new-feature-badge {
    display: inline-block;
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.03em;
    padding: 2px 7px;
    border-radius: 10px;
    background: #dbeafe;
    color: #1e40af;
    vertical-align: middle;
  }
  :global(.dark) .new-feature-badge {
    background: #1e3a5f;
    color: #93c5fd;
  }
  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0,0,0,0);
    white-space: nowrap;
  }
</style>
