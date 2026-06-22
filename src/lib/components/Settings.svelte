<script lang="ts">
  import { onMount } from 'svelte';
  import { labProfile, LAB_PROFILE_LABELS, loadLabProfile, type LabProfile } from '../profile';
  import { setLabProfile, getSpecimenStats } from '../api';
  import { addNotification } from '../stores/app';
  import { currentUser } from '../stores/auth';

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
</script>

<div>
  <div class="page-header">
    <h1>Settings</h1>
  </div>

  {#if $currentUser?.role !== 'admin'}
    <div class="card">
      <p style="color: var(--color-text-muted, #6b7280);">Only administrators can change lab settings.</p>
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
</style>
