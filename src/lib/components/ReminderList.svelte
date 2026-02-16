<script lang="ts">
  import { onMount } from 'svelte';
  import { listReminders, createReminder, dismissReminder } from '../api';
  import { addNotification } from '../stores/app';
  import { currentUser } from '../stores/auth';

  let reminders = $state<any[]>([]);
  let loading = $state(true);
  let showForm = $state(false);
  let form = $state({
    title: '', description: '', reminder_type: 'custom',
    due_date: new Date().toISOString().split('T')[0],
    urgency: 'normal', is_recurring: false, recurrence_days: '',
  });

  const types = ['subculture_due', 'media_expiry', 'disease_test', 'permit_expiry', 'quarantine_review', 'custom'];

  onMount(() => { load(); });

  async function load() {
    loading = true;
    try { reminders = await listReminders(); }
    catch (e: any) { addNotification(e.message, 'error'); }
    finally { loading = false; }
  }

  async function handleCreate(e: Event) {
    e.preventDefault();
    try {
      await createReminder({
        title: form.title,
        description: form.description || undefined,
        reminder_type: form.reminder_type,
        due_date: form.due_date,
        urgency: form.urgency,
        is_recurring: form.is_recurring,
        recurrence_days: form.recurrence_days ? parseInt(form.recurrence_days) : undefined,
      });
      addNotification('Reminder created', 'success');
      showForm = false;
      load();
    } catch (e: any) { addNotification(e.message, 'error'); }
  }

  async function handleDismiss(id: string, snooze: boolean) {
    try {
      await dismissReminder(id, snooze);
      addNotification(snooze ? 'Snoozed for 1 day' : 'Dismissed', 'info');
      load();
    } catch (e: any) { addNotification(e.message, 'error'); }
  }

  function getUrgencyClass(u: string) {
    switch (u) {
      case 'critical': return 'badge-red';
      case 'high': return 'badge-yellow';
      case 'normal': return 'badge-blue';
      default: return 'badge-gray';
    }
  }

  function getStatusClass(s: string) {
    switch (s) {
      case 'active': return 'badge-blue';
      case 'snoozed': return 'badge-yellow';
      case 'dismissed': return 'badge-gray';
      case 'completed': return 'badge-green';
      default: return 'badge-gray';
    }
  }
</script>

<div>
  <div class="page-header">
    <h1>Reminders</h1>
    {#if $currentUser?.role !== 'guest'}
      <button class="btn btn-primary" onclick={() => showForm = !showForm}>
        {showForm ? 'Cancel' : '+ New Reminder'}
      </button>
    {/if}
  </div>

  {#if showForm}
    <div class="card" style="margin-bottom:16px;">
      <form onsubmit={handleCreate}>
        <h3 style="margin-bottom:16px;">New Reminder</h3>
        <div class="form-row">
          <div class="form-group">
            <label>Title *</label>
            <input type="text" bind:value={form.title} required placeholder="e.g., Subculture citrus batch" />
          </div>
          <div class="form-group">
            <label>Type</label>
            <select bind:value={form.reminder_type}>
              {#each types as t}
                <option value={t}>{t.replace(/_/g, ' ')}</option>
              {/each}
            </select>
          </div>
        </div>
        <div class="form-row">
          <div class="form-group">
            <label>Due Date *</label>
            <input type="date" bind:value={form.due_date} required />
          </div>
          <div class="form-group">
            <label>Urgency</label>
            <select bind:value={form.urgency}>
              <option value="low">Low</option>
              <option value="normal">Normal</option>
              <option value="high">High</option>
              <option value="critical">Critical</option>
            </select>
          </div>
        </div>
        <div class="form-row">
          <div class="form-group">
            <label>
              <input type="checkbox" bind:checked={form.is_recurring} style="width:auto;margin-right:6px;" />
              Recurring
            </label>
            {#if form.is_recurring}
              <input type="number" bind:value={form.recurrence_days} placeholder="Every N days" style="margin-top:6px;" />
            {/if}
          </div>
          <div class="form-group">
            <label>Description</label>
            <textarea bind:value={form.description} rows="2"></textarea>
          </div>
        </div>
        <div style="text-align:right;">
          <button type="submit" class="btn btn-primary">Create</button>
        </div>
      </form>
    </div>
  {/if}

  {#if loading}
    <div class="empty-state">Loading...</div>
  {:else if reminders.length === 0}
    <div class="empty-state">No reminders</div>
  {:else}
    <div class="card" style="overflow-x:auto;">
      <table>
        <thead>
          <tr>
            <th>Title</th>
            <th>Type</th>
            <th>Specimen</th>
            <th>Due</th>
            <th>Urgency</th>
            <th>Status</th>
            <th>Snoozed</th>
            <th>Actions</th>
          </tr>
        </thead>
        <tbody>
          {#each reminders as r}
            <tr>
              <td><strong>{r.title}</strong></td>
              <td>{r.reminder_type.replace(/_/g, ' ')}</td>
              <td>{r.specimen_accession || '—'}</td>
              <td>{r.due_date}</td>
              <td><span class="badge {getUrgencyClass(r.urgency)}">{r.urgency}</span></td>
              <td><span class="badge {getStatusClass(r.status)}">{r.status}</span></td>
              <td>{r.snooze_count}x</td>
              <td>
                {#if r.status === 'active' || r.status === 'snoozed'}
                  <div style="display:flex;gap:4px;">
                    <button class="btn btn-sm" onclick={() => handleDismiss(r.id, true)}>Snooze</button>
                    <button class="btn btn-sm" onclick={() => handleDismiss(r.id, false)}>Dismiss</button>
                  </div>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>
