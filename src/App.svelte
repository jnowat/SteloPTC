<script lang="ts">
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import { isLoggedIn, token, currentUser, clearAuth, initializing } from './lib/stores/auth';
  import { currentView, darkMode, navigateTo } from './lib/stores/app';
  import { getCurrentUser, logout as apiLogout } from './lib/api';
  import Login from './lib/components/Login.svelte';
  import Sidebar from './lib/components/Sidebar.svelte';
  import Dashboard from './lib/components/Dashboard.svelte';
  import SpecimenList from './lib/components/SpecimenList.svelte';
  import SpecimenDetail from './lib/components/SpecimenDetail.svelte';
  import MediaList from './lib/components/MediaList.svelte';
  import ReminderList from './lib/components/ReminderList.svelte';
  import ComplianceView from './lib/components/ComplianceView.svelte';
  import SpeciesManager from './lib/components/SpeciesManager.svelte';
  import UserManager from './lib/components/UserManager.svelte';
  import AuditLog from './lib/components/AuditLog.svelte';
  import Notifications from './lib/components/Notifications.svelte';

  let startupError = '';

  // Try to restore session on mount (once only)
  onMount(() => {
    const savedToken = get(token);
    if (savedToken) {
      getCurrentUser().then((user) => {
        currentUser.set(user);
        initializing.set(false);
      }).catch(() => {
        clearAuth();
      });
    } else {
      initializing.set(false);
    }
  });

  async function handleLogout() {
    try {
      await apiLogout();
    } catch (_e) {
      // ignore
    }
    clearAuth();
    navigateTo('dashboard');
  }

  function toggleDark() {
    darkMode.update((d) => !d);
  }

  // Keyboard shortcuts
  function handleKeydown(e: KeyboardEvent) {
    if (e.ctrlKey || e.metaKey) {
      switch (e.key) {
        case '1': e.preventDefault(); navigateTo('dashboard'); break;
        case '2': e.preventDefault(); navigateTo('specimens'); break;
        case '3': e.preventDefault(); navigateTo('media'); break;
        case '4': e.preventDefault(); navigateTo('reminders'); break;
      }
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="app" class:dark={$darkMode}>
  {#if $initializing}
    <div class="init-screen">
      <div class="init-content">
        <h1 class="init-title">SteloPTC</h1>
        <div class="init-spinner"></div>
        <p>Restoring session...</p>
      </div>
    </div>
  {:else if !$isLoggedIn}
    <Login />
  {:else}
    <div class="layout">
      <Sidebar onlogout={handleLogout} ontoggleDark={toggleDark} isDark={$darkMode} />
      <main class="main-content">
        <Notifications />
        {#if $currentView === 'dashboard'}
          <Dashboard />
        {:else if $currentView === 'specimens'}
          <SpecimenList />
        {:else if $currentView === 'specimen-detail'}
          <SpecimenDetail />
        {:else if $currentView === 'media'}
          <MediaList />
        {:else if $currentView === 'reminders'}
          <ReminderList />
        {:else if $currentView === 'compliance'}
          <ComplianceView />
        {:else if $currentView === 'species'}
          <SpeciesManager />
        {:else if $currentView === 'users'}
          <UserManager />
        {:else if $currentView === 'audit'}
          <AuditLog />
        {:else}
          <Dashboard />
        {/if}
      </main>
    </div>
  {/if}
</div>

<style>
  :global(*) {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
  }

  :global(body) {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
    font-size: 14px;
    line-height: 1.5;
    overflow: hidden;
  }

  .app {
    height: 100vh;
    width: 100vw;
    background: #f8fafc;
    color: #1e293b;
  }

  .app.dark {
    background: #0f172a;
    color: #e2e8f0;
  }

  .init-screen {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100vh;
    width: 100vw;
    background: linear-gradient(135deg, #0f172a 0%, #1e3a5f 50%, #0f4c2d 100%);
  }

  .init-content {
    text-align: center;
    color: #94a3b8;
  }

  .init-title {
    font-size: 32px;
    font-weight: 800;
    color: #f1f5f9;
    letter-spacing: -0.5px;
    margin-bottom: 20px;
  }

  .init-spinner {
    width: 36px;
    height: 36px;
    border: 3px solid #334155;
    border-top-color: #2563eb;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    margin: 0 auto 16px;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .layout {
    display: flex;
    height: 100vh;
  }

  .main-content {
    flex: 1;
    overflow-y: auto;
    padding: 24px;
  }

  :global(.btn) {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 8px 16px;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    background: white;
    color: #374151;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s;
  }
  :global(.btn:hover) { background: #f3f4f6; }
  :global(.btn-primary) { background: #2563eb; color: white; border-color: #2563eb; }
  :global(.btn-primary:hover) { background: #1d4ed8; }
  :global(.btn-danger) { background: #dc2626; color: white; border-color: #dc2626; }
  :global(.btn-danger:hover) { background: #b91c1c; }
  :global(.btn-sm) { padding: 4px 10px; font-size: 12px; }

  :global(.dark .btn) { background: #1e293b; color: #e2e8f0; border-color: #334155; }
  :global(.dark .btn:hover) { background: #334155; }
  :global(.dark .btn-primary) { background: #2563eb; color: white; }

  :global(.card) {
    background: white;
    border: 1px solid #e2e8f0;
    border-radius: 8px;
    padding: 20px;
  }
  :global(.dark .card) { background: #1e293b; border-color: #334155; }

  :global(input), :global(select), :global(textarea) {
    padding: 8px 12px;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    font-size: 13px;
    background: white;
    color: #1e293b;
    width: 100%;
  }
  :global(.dark input), :global(.dark select), :global(.dark textarea) {
    background: #0f172a;
    color: #e2e8f0;
    border-color: #475569;
  }
  :global(input:focus), :global(select:focus), :global(textarea:focus) {
    outline: none;
    border-color: #2563eb;
    box-shadow: 0 0 0 3px rgba(37, 99, 235, 0.1);
  }

  :global(label) {
    display: block;
    font-size: 12px;
    font-weight: 600;
    color: #6b7280;
    margin-bottom: 4px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
  :global(.dark label) { color: #94a3b8; }

  :global(.form-group) { margin-bottom: 16px; }
  :global(.form-row) { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }
  :global(.form-row-3) { display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 16px; }

  :global(table) {
    width: 100%;
    border-collapse: collapse;
  }
  :global(th) {
    text-align: left;
    padding: 10px 12px;
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: #6b7280;
    border-bottom: 2px solid #e2e8f0;
  }
  :global(td) {
    padding: 10px 12px;
    border-bottom: 1px solid #f1f5f9;
    font-size: 13px;
  }
  :global(tr:hover td) { background: #f8fafc; }
  :global(.dark th) { color: #94a3b8; border-bottom-color: #334155; }
  :global(.dark td) { border-bottom-color: #1e293b; }
  :global(.dark tr:hover td) { background: #1e293b; }

  :global(.badge) {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 12px;
    font-size: 11px;
    font-weight: 600;
  }
  :global(.badge-green) { background: #dcfce7; color: #166534; }
  :global(.badge-red) { background: #fef2f2; color: #991b1b; }
  :global(.badge-yellow) { background: #fef9c3; color: #854d0e; }
  :global(.badge-blue) { background: #dbeafe; color: #1e40af; }
  :global(.badge-gray) { background: #f1f5f9; color: #475569; }
  :global(.dark .badge-green) { background: #166534; color: #dcfce7; }
  :global(.dark .badge-red) { background: #991b1b; color: #fef2f2; }
  :global(.dark .badge-yellow) { background: #854d0e; color: #fef9c3; }
  :global(.dark .badge-blue) { background: #1e40af; color: #dbeafe; }
  :global(.dark .badge-gray) { background: #334155; color: #94a3b8; }

  :global(.page-header) {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 24px;
  }
  :global(.page-header h1) { font-size: 24px; font-weight: 700; }

  :global(.empty-state) {
    text-align: center;
    padding: 48px 24px;
    color: #9ca3af;
  }
</style>
