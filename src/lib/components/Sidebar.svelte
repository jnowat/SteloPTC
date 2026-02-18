<script lang="ts">
  import { currentView, navigateTo, type View } from '../stores/app';
  import { currentUser } from '../stores/auth';

  let { onlogout, ontoggleDark, isDark }: { onlogout: () => void; ontoggleDark: () => void; isDark: boolean } = $props();

  interface NavItem {
    id: View;
    label: string;
    icon: string;
    roles?: string[];
  }

  const navItems: NavItem[] = [
    { id: 'dashboard', label: 'Dashboard', icon: '&#9633;' },
    { id: 'specimens', label: 'Specimens', icon: '&#127793;' },
    { id: 'media', label: 'Media Logs', icon: '&#129514;' },
    { id: 'reminders', label: 'Reminders', icon: '&#128276;' },
    { id: 'compliance', label: 'Compliance', icon: '&#128203;' },
    { id: 'species', label: 'Species', icon: '&#127807;' },
    { id: 'inventory', label: 'Inventory', icon: '&#128230;' },
    { id: 'users', label: 'Users', icon: '&#128101;', roles: ['admin'] },
    { id: 'audit', label: 'Audit Log', icon: '&#128220;', roles: ['admin', 'supervisor'] },
  ];

  function canSee(item: NavItem): boolean {
    if (!item.roles) return true;
    const role = $currentUser?.role || 'guest';
    return item.roles.includes(role);
  }
</script>

<aside class="sidebar">
  <div class="sidebar-header">
    <h2>SteloPTC</h2>
    <span class="version">v0.1.7</span>
  </div>

  <nav class="nav">
    {#each navItems as item}
      {#if canSee(item)}
        <button
          class="nav-item"
          class:active={$currentView === item.id}
          onclick={() => navigateTo(item.id)}
        >
          <span class="nav-icon">{@html item.icon}</span>
          <span class="nav-label">{item.label}</span>
        </button>
      {/if}
    {/each}
  </nav>

  <div class="sidebar-footer">
    <div class="user-info">
      <div class="user-name">{$currentUser?.display_name || 'User'}</div>
      <div class="user-role">{$currentUser?.role || ''}</div>
    </div>
    <div class="footer-actions">
      <button class="icon-btn" onclick={ontoggleDark} title="Toggle dark mode">
        {@html isDark ? '&#9728;' : '&#127769;'}
      </button>
      <button class="icon-btn" onclick={onlogout} title="Logout">
        {@html '&#10140;'}
      </button>
    </div>
  </div>
</aside>

<style>
  .sidebar {
    width: 220px;
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: #1e293b;
    color: #94a3b8;
    border-right: 1px solid #334155;
    flex-shrink: 0;
  }
  .sidebar-header {
    padding: 20px;
    border-bottom: 1px solid #334155;
  }
  .sidebar-header h2 {
    color: #f1f5f9;
    font-size: 18px;
    font-weight: 800;
    letter-spacing: -0.5px;
  }
  .version {
    font-size: 11px;
    color: #64748b;
  }
  .nav {
    flex: 1;
    padding: 12px 8px;
    overflow-y: auto;
  }
  .nav-item {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 10px 12px;
    border: none;
    background: none;
    color: #94a3b8;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    border-radius: 6px;
    transition: all 0.15s;
    text-align: left;
  }
  .nav-item:hover {
    background: #334155;
    color: #e2e8f0;
  }
  .nav-item.active {
    background: #2563eb;
    color: white;
  }
  .nav-icon { font-size: 16px; width: 20px; text-align: center; }
  .sidebar-footer {
    padding: 16px;
    border-top: 1px solid #334155;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .user-name { font-size: 13px; color: #e2e8f0; font-weight: 600; }
  .user-role { font-size: 11px; color: #64748b; text-transform: capitalize; }
  .footer-actions { display: flex; gap: 4px; }
  .icon-btn {
    background: none;
    border: 1px solid #475569;
    color: #94a3b8;
    width: 32px;
    height: 32px;
    border-radius: 6px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 14px;
  }
  .icon-btn:hover { background: #334155; color: #e2e8f0; }
</style>
