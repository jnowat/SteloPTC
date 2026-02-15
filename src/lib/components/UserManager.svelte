<script lang="ts">
  import { listUsers, createUser, updateUserRole } from '../api';
  import { addNotification } from '../stores/app';

  let users = $state<any[]>([]);
  let loading = $state(true);
  let showForm = $state(false);
  let form = $state({ username: '', password: '', display_name: '', email: '', role: 'tech' });

  const roles = ['admin', 'supervisor', 'tech', 'guest'];

  $effect(() => { load(); });

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
    <button class="btn btn-primary" onclick={() => showForm = !showForm}>
      {showForm ? 'Cancel' : '+ New User'}
    </button>
  </div>

  {#if showForm}
    <div class="card" style="margin-bottom:16px;">
      <form onsubmit={handleCreate}>
        <h3 style="margin-bottom:16px;">Create User</h3>
        <div class="form-row">
          <div class="form-group">
            <label>Username *</label>
            <input type="text" bind:value={form.username} required />
          </div>
          <div class="form-group">
            <label>Password *</label>
            <input type="password" bind:value={form.password} required />
          </div>
        </div>
        <div class="form-row">
          <div class="form-group">
            <label>Display Name *</label>
            <input type="text" bind:value={form.display_name} required />
          </div>
          <div class="form-group">
            <label>Email</label>
            <input type="email" bind:value={form.email} />
          </div>
        </div>
        <div class="form-group">
          <label>Role</label>
          <select bind:value={form.role}>
            {#each roles as r}
              <option value={r}>{r}</option>
            {/each}
          </select>
        </div>
        <div style="text-align:right;">
          <button type="submit" class="btn btn-primary">Create User</button>
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
            <th>Username</th>
            <th>Display Name</th>
            <th>Email</th>
            <th>Role</th>
            <th>Status</th>
          </tr>
        </thead>
        <tbody>
          {#each users as u}
            <tr>
              <td><strong>{u.username}</strong></td>
              <td>{u.display_name}</td>
              <td>{u.email || '—'}</td>
              <td>
                <select value={u.role} onchange={(e) => handleRoleChange(u.id, (e.target as HTMLSelectElement).value)} style="width:auto;">
                  {#each roles as r}
                    <option value={r}>{r}</option>
                  {/each}
                </select>
              </td>
              <td>
                {#if u.is_active}
                  <span class="badge badge-green">Active</span>
                {:else}
                  <span class="badge badge-gray">Inactive</span>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>
