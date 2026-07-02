<script lang="ts">
  import { currentView, navigateTo, unreadErrorCount, workQueueCount, type View } from '../stores/app';
  import { currentUser } from '../stores/auth';
  import { getVersion } from '@tauri-apps/api/app';
  import { isTauri } from '../isTauri';

  let { onlogout, ontoggleDark, isDark }: { onlogout: () => void; ontoggleDark: () => void; isDark: boolean } = $props();

  let mobileOpen = $state(false);
  let appVersion = $state('…');

  // Inside the Tauri webview, read the running binary's own version. In the
  // PWA/browser build there is no Tauri IPC bridge to answer that call, so
  // fall back to the version baked in at build time (see vite.config.ts).
  if (isTauri()) {
    getVersion().then(v => { appVersion = `v${v}`; }).catch(() => { appVersion = `v${__APP_VERSION__}`; });
  } else {
    appVersion = `v${__APP_VERSION__}`;
  }

  interface NavItem {
    id: View;
    label: string;
    icon: string;
    roles?: string[];
  }

  const navItems: NavItem[] = [
    { id: 'dashboard', label: 'Dashboard', icon: '&#9633;' },
    { id: 'work-queue', label: 'Work Queue', icon: '&#9989;' },
    { id: 'analytics', label: 'Analytics', icon: '&#128200;' },
    { id: 'lab-map', label: 'Lab Map', icon: '&#128506;' },
    { id: 'specimens', label: 'Specimens', icon: '&#127793;' },
    { id: 'media', label: 'Media Logs', icon: '&#129514;' },
    { id: 'reminders', label: 'Reminders', icon: '&#128276;' },
    { id: 'compliance', label: 'Compliance', icon: '&#128203;' },
    { id: 'species', label: 'Species', icon: '&#127807;' },
    { id: 'taxonomy', label: 'Taxonomy', icon: '&#129516;' },
    { id: 'ncbi-sync', label: 'NCBI Sync', icon: '&#128202;', roles: ['admin'] },
    { id: 'inventory', label: 'Inventory', icon: '&#128230;' },
    { id: 'cryo', label: 'Cryostorage', icon: '&#10052;' },
    { id: 'breeding', label: 'Breeding', icon: '&#127812;' },
    { id: 'provisional-taxa', label: 'Prov. Taxa', icon: '&#128300;' },
    { id: 'users', label: 'Users', icon: '&#128101;', roles: ['admin'] },
    { id: 'settings', label: 'Settings', icon: '&#9881;', roles: ['admin'] },
    { id: 'audit', label: 'Audit Log', icon: '&#128220;', roles: ['admin', 'supervisor'] },
    { id: 'error-log', label: 'Error Log', icon: '&#9888;' },
    { id: 'export', label: 'Export Data', icon: '&#8659;' },
    { id: 'import', label: 'Import Data', icon: '&#8657;' },
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
  aria-expanded={mobileOpen}
  aria-controls="sidebar-nav"
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
    <span class="version" title="Application version">{appVersion}</span>
    <!-- Mobile close button inside drawer -->
    <button class="drawer-close" aria-label="Close menu" title="Close navigation menu" onclick={() => (mobileOpen = false)}>&#10005;</button>
  </div>

  <nav id="sidebar-nav" class="nav" aria-label="Main navigation">
    {#each navItems as item}
      {#if canSee(item)}
        <button
          class="nav-item"
          class:active={$currentView === item.id}
          aria-current={$currentView === item.id ? 'page' : undefined}
          onclick={() => handleNavTap(item.id)}
          aria-label={
            item.id === 'dashboard' ? 'Dashboard — overview of all key metrics (Ctrl+1)' :
            item.id === 'work-queue' ? 'Work Queue — specimens needing attention today' :
            item.id === 'specimens' ? 'Specimens — manage and view all tissue culture specimens (Ctrl+2)' :
            item.id === 'media' ? 'Media Logs — track media preparation and usage records (Ctrl+3)' :
            item.id === 'reminders' ? 'Reminders — view and manage scheduled tasks and alerts (Ctrl+4)' :
            item.id === 'compliance' ? 'Compliance — review compliance flags and regulatory records' :
            item.id === 'species' ? 'Species — manage species definitions and subculture intervals' :
            item.id === 'taxonomy' ? 'Taxonomy — browse species and strains, manage strain identities' :
            item.id === 'ncbi-sync' ? 'NCBI Sync — import and sync taxonomy data from NCBI (admin only)' :
            item.id === 'inventory' ? 'Inventory — track stock levels and supply usage' :
            item.id === 'cryo' ? 'Cryostorage — manage frozen vial inventory in LN₂ and −80°C' :
            item.id === 'breeding' ? 'Breeding Programs — track multi-generational selection and fitness' :
            item.id === 'provisional-taxa' ? 'Provisional Taxa — manage lab-internal custom taxa and Darwin Core export' :
            item.id === 'users' ? 'Users — manage user accounts and roles (admin only)' :
            item.id === 'settings' ? 'Settings — configure lab profile and system options (admin only)' :
            item.id === 'audit' ? 'Audit Log — view system-wide change history (admin/supervisor)' :
            item.id === 'error-log' ? 'Error Log — review application errors and warnings (Ctrl+5)' :
            item.id === 'export' ? 'Export Data — download data as Excel, CSV, or JSON' :
            item.id === 'import' ? 'Import Data — restore or bulk-load data from an Excel workbook' :
            item.label
          }
          title={
            item.id === 'dashboard' ? 'Go to Dashboard — overview of all key metrics' :
            item.id === 'work-queue' ? 'Go to Work Queue — specimens needing attention today' :
            item.id === 'specimens' ? 'Go to Specimens — manage and view all tissue culture specimens' :
            item.id === 'media' ? 'Go to Media Logs — track media preparation and usage records' :
            item.id === 'reminders' ? 'Go to Reminders — view and manage scheduled tasks and alerts' :
            item.id === 'compliance' ? 'Go to Compliance — review compliance flags and regulatory records' :
            item.id === 'species' ? 'Go to Species — manage species definitions and subculture intervals' :
            item.id === 'taxonomy' ? 'Go to Taxonomy — browse species and strains, manage strain identities' :
            item.id === 'ncbi-sync' ? 'Go to NCBI Sync — import and sync taxonomy data from NCBI' :
            item.id === 'inventory' ? 'Go to Inventory — track stock levels and supply usage' :
            item.id === 'cryo' ? 'Go to Cryostorage — manage frozen vial inventory in LN₂ and −80°C' :
            item.id === 'breeding' ? 'Go to Breeding Programs — track multi-generational selection and fitness' :
            item.id === 'provisional-taxa' ? 'Go to Provisional Taxa — manage lab-internal custom taxa and Darwin Core export' :
            item.id === 'users' ? 'Go to Users — manage user accounts and roles (admin only)' :
            item.id === 'settings' ? 'Go to Settings — configure lab profile and system options' :
            item.id === 'audit' ? 'Go to Audit Log — view system-wide change history (admin/supervisor)' :
            item.id === 'error-log' ? 'Go to Error Log — review application errors and warnings' :
            item.id === 'export' ? 'Go to Export — download data as Excel, CSV, or JSON' :
            item.id === 'import' ? 'Go to Import — restore or bulk-load data from an Excel workbook' :
            `Navigate to ${item.label}`
          }
        >
          <span class="nav-icon">{@html item.icon}</span>
          <span class="nav-label">{item.label}</span>
          {#if item.id === 'work-queue' && $workQueueCount > 0}
            <span class="queue-badge" title="{$workQueueCount} item{$workQueueCount === 1 ? '' : 's'} need attention">{$workQueueCount > 99 ? '99+' : $workQueueCount}</span>
          {/if}
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
      <button class="icon-btn" onclick={ontoggleDark} title="Toggle dark/light theme" aria-label={isDark ? 'Switch to light theme' : 'Switch to dark theme'}>
        {@html isDark ? '&#9728;' : '&#127769;'}
      </button>
      <button class="icon-btn" onclick={onlogout} title="Log out of the current session" aria-label="Log out">
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
    z-index: var(--z-sidebar);
    width: 48px;
    height: 48px;
    min-height: 48px;
    padding: var(--space-3);
    background: var(--color-sidebar-bg);
    border: 1px solid var(--color-sidebar-border);
    border-radius: var(--radius-xl);
    cursor: pointer;
    box-shadow: var(--shadow-hamburger);
    transition: background var(--transition-fast);
  }
  .hamburger:hover { background: var(--color-sidebar-hover); }
  .hamburger span {
    display: block;
    width: 100%;
    height: 2px;
    background: var(--color-sidebar-text);
    border-radius: 2px;
  }

  /* ── Mobile overlay ─────────────────────────────────────────── */
  .mobile-overlay {
    display: none;
    position: fixed;
    inset: 0;
    z-index: var(--z-overlay);
    background: rgba(0, 0, 0, 0.55);
  }

  /* ── Sidebar ────────────────────────────────────────────────── */
  .sidebar {
    width: 220px;
    height: 100vh;
    height: 100dvh; /* dynamic viewport height for mobile */
    display: flex;
    flex-direction: column;
    background: var(--color-sidebar-bg);
    color: var(--color-sidebar-text);
    border-right: 1px solid var(--color-sidebar-border);
    flex-shrink: 0;
  }
  .sidebar-header {
    padding: var(--space-5);
    border-bottom: 1px solid var(--color-sidebar-border);
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
    flex-wrap: wrap;
  }
  .sidebar-header h2 {
    color: var(--color-sidebar-bright);
    font-size: var(--font-size-xl);
    font-weight: 800;
    letter-spacing: -0.5px;
  }
  .version {
    font-size: var(--font-size-xs);
    color: var(--color-sidebar-muted);
  }
  .drawer-close {
    display: none;
    margin-left: auto;
    background: none;
    border: none;
    color: var(--color-sidebar-muted);
    font-size: var(--font-size-md);
    cursor: pointer;
    padding: var(--space-1);
    line-height: 1;
  }
  .nav {
    flex: 1;
    padding: var(--space-3) var(--space-2);
    overflow-y: auto;
  }
  .nav-item {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 10px var(--space-3);
    border: none;
    background: none;
    color: var(--color-sidebar-text);
    font-size: var(--font-size-base);
    font-weight: 500;
    cursor: pointer;
    border-radius: var(--radius-md);
    transition: all var(--transition-fast);
    text-align: left;
    position: relative;
  }
  .nav-item:hover {
    background: var(--color-sidebar-hover);
    color: var(--color-text);
  }
  .nav-item.active {
    background: var(--color-sidebar-active);
    color: white;
  }
  .nav-icon { font-size: var(--font-size-md); width: 20px; text-align: center; }
  .nav-label { flex: 1; }

  .queue-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 18px;
    height: 18px;
    padding: 0 5px;
    border-radius: var(--radius-full);
    background: #d97706;
    color: #fff;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0;
    line-height: 1;
    animation: badgePop 0.3s cubic-bezier(0.34,1.56,0.64,1);
  }

  .error-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 18px;
    height: 18px;
    padding: 0 5px;
    border-radius: var(--radius-full);
    background: var(--color-danger);
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
    padding: var(--space-4);
    /* Reserve home-indicator height at the bottom */
    padding-bottom: calc(var(--space-4) + env(safe-area-inset-bottom, 0px));
    border-top: 1px solid var(--color-sidebar-border);
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .user-name { font-size: var(--font-size-base); color: var(--color-text); font-weight: 600; }
  .user-role { font-size: var(--font-size-xs); color: var(--color-sidebar-muted); text-transform: capitalize; }
  .footer-actions { display: flex; gap: var(--space-1); }
  .icon-btn {
    background: none;
    border: 1px solid var(--color-sidebar-icon-btn);
    color: var(--color-sidebar-text);
    width: 32px;
    height: 32px;
    border-radius: var(--radius-md);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: var(--font-size-md);
  }
  .icon-btn:hover { background: var(--color-sidebar-hover); color: var(--color-text); }

  /* ── Sidebar header safe-area top padding on mobile/tablet ───── */
  @media (max-width: 1024px) {
    .sidebar-header {
      padding-top: calc(var(--space-5) + env(safe-area-inset-top, 0px));
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
      z-index: var(--z-sidebar);
      transform: translateX(-100%);
      transition: transform var(--transition-drawer);
      box-shadow: var(--shadow-sidebar);
    }
    .sidebar.mobile-open {
      transform: translateX(0);
    }

    .drawer-close {
      display: flex;
    }

    /* Larger touch targets on mobile/tablet (48px = WCAG 2.5.5 / Apple HIG) */
    .nav-item {
      padding: 14px var(--space-4);
      font-size: var(--font-size-lg);
      min-height: 52px;
      border-radius: var(--radius-lg);
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
