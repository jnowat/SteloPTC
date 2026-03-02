<script lang="ts">
  import { onMount } from 'svelte';
  import { listUsers, createUser, updateUserRole } from '../api';
  import { addNotification } from '../stores/app';

  let users = $state<any[]>([]);
  let loading = $state(true);
  let showForm = $state(false);
  let form = $state({ username: '', password: '', display_name: '', email: '', role: 'tech' });

  const roles = ['admin', 'supervisor', 'tech', 'guest'];

  onMount(() => { load(); });

  async function load() {
    loading = true;
    try { users = await listUsers(); }
    catch (e: any) { addNotification(e.message, 'error'); }
    finally { loading = false; }
  }

  async function handleCreate(e: Event) {
    e.preventDefault();
    try {
      await createUser({
        username: form.username,
        password: form.password,
        display_name: form.display_name,
        email: form.email || undefined,
        role: form.role,
      });
      addNotification('User created', 'success');
      showForm = false;
      form = { username: '', password: '', display_name: '', email: '', role: 'tech' };
      load();
    } catch (e: any) { addNotification(e.message, 'error'); }
  }

  async function handleRoleChange(userId: string, newRole: string) {
    try {
      await updateUserRole(userId, newRole);
      addNotification('Role updated', 'success');
      load();
    } catch (e: any) { addNotification(e.message, 'error'); }
  }
</script>

<div>
  <div class="page-header">
    <h1>User Management</h1>
    <button class="btn btn-primary" title={showForm ? 'Cancel and close the form' : 'Open form to create a new user'} onclick={() => showForm = !showForm}>
      {showForm ? 'Cancel' : '+ New User'}
    </button>
  </div>

  {#if showForm}
    <div class="card" style="margin-bottom:16px;">
      <form onsubmit={handleCreate}>
        <h3 style="margin-bottom:16px;">Create User</h3>
        <div class="form-row">
          <div class="form-group">
            <label title="Unique login name for this user">Username *</label>
            <input type="text" title="Enter a unique username for login" bind:value={form.username} required />
          </div>
          <div class="form-group">
            <label title="Password for this user account">Password *</label>
            <input type="password" title="Enter a secure password for this user" bind:value={form.password} required />
          </div>
        </div>
        <div class="form-row">
          <div class="form-group">
            <label title="Full name shown in the interface for this user">Display Name *</label>
            <input type="text" title="Enter the user's full display name" bind:value={form.display_name} required />
          </div>
          <div class="form-group">
            <label title="Optional email address for notifications and contact">Email</label>
            <input type="email" title="Enter the user's email address (optional)" bind:value={form.email} />
          </div>
        </div>
        <div class="form-group">
          <label title="Permission level assigned to this user">Role</label>
          <select title="Select the role that determines user permissions" bind:value={form.role}>
            {#each roles as r}
              <option value={r}>{r}</option>
            {/each}
          </select>
        </div>
        <div style="text-align:right;">
          <button type="submit" class="btn btn-primary" title="Save and create this new user account">Create User</button>
        </div>
      </form>
    </div>
  {/if}

  {#if loading}
    <div class="empty-state">Loading...</div>
  {:else}
    <div class="card">
      <table>
        <thead>
          <tr>
            <th title="User's unique login name">Username</th>
            <th title="User's full name shown in the interface">Display Name</th>
            <th title="User's email address">Email</th>
            <th title="User's permission level">Role</th>
            <th title="Whether the user account is currently active">Status</th>
          </tr>
        </thead>
        <tbody>
          {#each users as u}
            <tr>
              <td><strong>{u.username}</strong></td>
              <td>{u.display_name}</td>
              <td>{u.email || '—'}</td>
              <td>
                <select title="Change this user's role and permissions" value={u.role} onchange={(e) => handleRoleChange(u.id, (e.target as HTMLSelectElement).value)} style="width:auto;">
                  {#each roles as r}
                    <option value={r}>{r}</option>
                  {/each}
                </select>
              </td>
              <td>
                {#if u.is_active}
                  <span class="badge badge-green" title="This user account is active and can log in">Active</span>
                {:else}
                  <span class="badge badge-gray" title="This user account is inactive and cannot log in">Inactive</span>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>
