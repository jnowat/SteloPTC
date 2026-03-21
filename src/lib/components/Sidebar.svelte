<script lang="ts">
  import { currentView, navigateTo, unreadErrorCount, type View } from '../stores/app';
  import { currentUser } from '../stores/auth';

  let { onlogout, ontoggleDark, isDark }: { onlogout: () => void; ontoggleDark: () => void; isDark: boolean } = $props();

  let mobileOpen = $state(false);

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
    { id: 'error-log', label: 'Error Log', icon: '&#9888;' },
    { id: 'export', label: 'Export Data', icon: '&#8659;' },
  ];

  function canSee(item: NavItem): boolean {
    if (!item.roles) return true;
    const role = $currentUser?.role || 'guest';
    return item.roles.includes(role);
  }

  function handleNavTap(id: View) {
    navigateTo(id);
    mobileOpen = false;
  }
</script>

<!-- Mobile hamburger button -->
<button
  class="hamburger"
  aria-label="Open navigation menu"
  title="Open navigation menu"
  onclick={() => (mobileOpen = true)}
>
  <span></span>
  <span></span>
  <span></span>
</button>

<!-- Mobile overlay -->
{#if mobileOpen}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="mobile-overlay" onclick={() => (mobileOpen = false)}></div>
{/if}

<aside class="sidebar" class:mobile-open={mobileOpen}>
  <div class="sidebar-header">
    <h2 title="SteloPTC — Sterilized Tissue/Plant Tissue Culture management system">SteloPTC</h2>
    <span class="version" title="Application version">v0.1.18</span>
    <!-- Mobile close button inside drawer -->
    <button class="drawer-close" aria-label="Close menu" title="Close navigation menu" onclick={() => (mobileOpen = false)}>&#10005;</button>
  </div>

  <nav class="nav">
    {#each navItems as item}
      {#if canSee(item)}
        <button
          class="nav-item"
          class:active={$currentView === item.id}
          onclick={() => handleNavTap(item.id)}
          title={
            item.id === 'dashboard' ? 'Go to Dashboard — overview of all key metrics' :
            item.id === 'specimens' ? 'Go to Specimens — manage and view all tissue culture specimens' :
            item.id === 'media' ? 'Go to Media Logs — track media preparation and usage records' :
            item.id === 'reminders' ? 'Go to Reminders — view and manage scheduled tasks and alerts' :
            item.id === 'compliance' ? 'Go to Compliance — review compliance flags and regulatory records' :
            item.id === 'species' ? 'Go to Species — manage species definitions and subculture intervals' :
            item.id === 'inventory' ? 'Go to Inventory — track stock levels and supply usage' :
            item.id === 'users' ? 'Go to Users — manage user accounts and roles (admin only)' :
            item.id === 'audit' ? 'Go to Audit Log — view system-wide change history (admin/supervisor)' :
            item.id === 'error-log' ? 'Go to Error Log — review application errors and warnings' :
            item.id === 'export' ? 'Go to Export — download data as Excel, CSV, or JSON' :
            `Navigate to ${item.label}`
          }
        >
          <span class="nav-icon">{@html item.icon}</span>
          <span class="nav-label">{item.label}</span>
          {#if item.id === 'error-log' && $unreadErrorCount > 0}
            <span class="error-badge" title="{$unreadErrorCount} unread error{$unreadErrorCount === 1 ? '' : 's'}">{$unreadErrorCount > 99 ? '99+' : $unreadErrorCount}</span>
          {/if}
        </button>
      {/if}
    {/each}
  </nav>

  <div class="sidebar-footer">
    <div class="user-info" title="Logged in as {$currentUser?.display_name || 'User'} ({$currentUser?.role || 'unknown role'})">
      <div class="user-name">{$currentUser?.display_name || 'User'}</div>
      <div class="user-role" title="Your account role determines which features and data you can access">{$currentUser?.role || ''}</div>
    </div>
    <div class="footer-actions">
      <button class="icon-btn" onclick={ontoggleDark} title="Toggle dark/light theme">
        {@html isDark ? '&#9728;' : '&#127769;'}
      </button>
      <button class="icon-btn" onclick={onlogout} title="Log out of the current session">
        {@html '&#10140;'}
      </button>
    </div>
  </div>
</aside>

<style>
  /* ── Hamburger (hidden on desktop) ─────────────────────────── */
  .hamburger {
    display: none;
    flex-direction: column;
    justify-content: center;
    gap: 5px;
    position: fixed;
    /* Shift down by the status-bar / notch safe area so it's never hidden */
    top: calc(14px + env(safe-area-inset-top, 0px));
    left: calc(14px + env(safe-area-inset-left, 0px));
    z-index: 1100;
    width: 48px;
    height: 48px;
    min-height: 48px;
    padding: 12px;
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 10px;
    cursor: pointer;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
    transition: background 0.15s;
  }
  .hamburger:hover { background: #334155; }
  .hamburger span {
    display: block;
    width: 100%;
    height: 2px;
    background: #94a3b8;
    border-radius: 2px;
  }

  /* ── Mobile overlay ─────────────────────────────────────────── */
  .mobile-overlay {
    display: none;
    position: fixed;
    inset: 0;
    z-index: 1050;
    background: rgba(0, 0, 0, 0.55);
  }

  /* ── Sidebar ────────────────────────────────────────────────── */
  .sidebar {
    width: 220px;
    height: 100vh;
    height: 100dvh; /* dynamic viewport height for mobile */
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
    display: flex;
    align-items: baseline;
    gap: 8px;
    flex-wrap: wrap;
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
  .drawer-close {
    display: none;
    margin-left: auto;
    background: none;
    border: none;
    color: #64748b;
    font-size: 16px;
    cursor: pointer;
    padding: 4px;
    line-height: 1;
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
    position: relative;
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
  .nav-label { flex: 1; }

  .error-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 18px;
    height: 18px;
    padding: 0 5px;
    border-radius: 9px;
    background: #dc2626;
    color: #fff;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0;
    line-height: 1;
    animation: badgePop 0.3s cubic-bezier(0.34,1.56,0.64,1);
  }
  @keyframes badgePop {
    from { transform: scale(0); opacity: 0; }
    to   { transform: scale(1); opacity: 1; }
  }

  .sidebar-footer {
    padding: 16px;
    /* Reserve home-indicator height at the bottom */
    padding-bottom: calc(16px + env(safe-area-inset-bottom, 0px));
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

  /* ── Sidebar header safe-area top padding on mobile/tablet ───── */
  @media (max-width: 1024px) {
    .sidebar-header {
      padding-top: calc(20px + env(safe-area-inset-top, 0px));
    }
  }

  /* ── Mobile/tablet breakpoint (< 1024px) ───────────────────── */
  @media (max-width: 1024px) {
    .hamburger {
      display: flex;
    }

    .mobile-overlay {
      display: block;
    }

    /* Full-screen slide-out drawer */
    .sidebar {
      position: fixed;
      top: 0;
      left: 0;
      width: min(280px, 85vw);
      height: 100dvh;
      z-index: 1100;
      transform: translateX(-100%);
      transition: transform 0.28s cubic-bezier(0.4, 0, 0.2, 1);
      box-shadow: 4px 0 32px rgba(0, 0, 0, 0.5);
    }
    .sidebar.mobile-open {
      transform: translateX(0);
    }

    .drawer-close {
      display: flex;
    }

    /* Larger touch targets on mobile/tablet (48px = WCAG 2.5.5 / Apple HIG) */
    .nav-item {
      padding: 14px 16px;
      font-size: 15px;
      min-height: 52px;
      border-radius: 8px;
    }
    .nav-icon { font-size: 18px; }
    .icon-btn {
      width: 48px;
      height: 48px;
      min-height: 48px;
      font-size: 18px;
    }
    .drawer-close {
      min-height: 48px;
      min-width: 48px;
      align-items: center;
      justify-content: center;
      font-size: 20px;
    }
  }
</style>
