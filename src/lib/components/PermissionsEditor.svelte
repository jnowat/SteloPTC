<script lang="ts">
  import { onMount } from 'svelte';
  import { listFieldPermissions, setFieldPermission, type FieldPermission } from '../api';
  import { addNotification } from '../stores/app';

  const ROLES: Array<FieldPermission['role']> = ['admin', 'supervisor', 'tech', 'guest'];
  const ROLE_LABELS: Record<string, string> = {
    admin: 'Admin', supervisor: 'Supervisor', tech: 'Tech', guest: 'Guest',
  };

  let permissions = $state<FieldPermission[]>([]);
  let loading = $state(true);
  let saving = $state<string | null>(null); // "role:entity:field" key currently in flight

  // Rows: unique (entity_type, field_name) pairs, in seed order.
  let rows = $derived.by(() => {
    const seen = new Set<string>();
    const out: Array<{ entity_type: string; field_name: string }> = [];
    for (const p of permissions) {
      const key = `${p.entity_type}.${p.field_name}`;
      if (!seen.has(key)) {
        seen.add(key);
        out.push({ entity_type: p.entity_type, field_name: p.field_name });
      }
    }
    return out;
  });

  function isVisible(entityType: string, fieldName: string, role: string): boolean {
    const perm = permissions.find(
      (p) => p.entity_type === entityType && p.field_name === fieldName && p.role === role,
    );
    return perm?.visible ?? true;
  }

  onMount(load);

  async function load() {
    loading = true;
    try {
      permissions = await listFieldPermissions();
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      loading = false;
    }
  }

  async function toggle(entityType: string, fieldName: string, role: string) {
    const key = `${role}:${entityType}:${fieldName}`;
    const newValue = !isVisible(entityType, fieldName, role);
    saving = key;
    try {
      await setFieldPermission(role, entityType, fieldName, newValue);
      const idx = permissions.findIndex(
        (p) => p.entity_type === entityType && p.field_name === fieldName && p.role === role,
      );
      if (idx >= 0) {
        permissions[idx] = { ...permissions[idx], visible: newValue };
      } else {
        permissions = [...permissions, { id: key, role: role as FieldPermission['role'], entity_type: entityType, field_name: fieldName, visible: newValue }];
      }
      addNotification(
        `${entityType}.${fieldName} is now ${newValue ? 'visible' : 'restricted'} for ${ROLE_LABELS[role]}`,
        'success',
      );
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      saving = null;
    }
  }
</script>

<div class="pe-wrap">
  <p style="font-size:13px;color:#6b7280;margin-bottom:14px;">
    Controls which roles can see sensitive fields. A field hidden from a role shows a
    <strong>🔒 Restricted</strong> indicator there instead of its real value — it is never omitted, and it
    never affects what gets written to the audit trail. Changes take effect immediately.
  </p>

  {#if loading}
    <div class="pe-loading" aria-busy="true" aria-label="Loading field permissions"></div>
  {:else if rows.length === 0}
    <p style="font-size:13px;color:#6b7280;">No gated fields are configured.</p>
  {:else}
    <table class="pe-table">
      <thead>
        <tr>
          <th>Field</th>
          {#each ROLES as role}
            <th>{ROLE_LABELS[role]}</th>
          {/each}
        </tr>
      </thead>
      <tbody>
        {#each rows as row}
          <tr>
            <td class="pe-field-name">{row.entity_type}.{row.field_name}</td>
            {#each ROLES as role}
              {@const key = `${role}:${row.entity_type}:${row.field_name}`}
              {@const visible = isVisible(row.entity_type, row.field_name, role)}
              <td class="pe-cell">
                <label class="pe-checkbox-label" title="{visible ? 'Visible' : 'Restricted'} — click to {visible ? 'restrict' : 'allow'} {ROLE_LABELS[role]} from viewing {row.entity_type}.{row.field_name}">
                  <input
                    type="checkbox"
                    checked={visible}
                    disabled={saving === key}
                    onchange={() => toggle(row.entity_type, row.field_name, role)}
                  />
                  <span class="visually-hidden">{ROLE_LABELS[role]} can view {row.entity_type}.{row.field_name}</span>
                </label>
              </td>
            {/each}
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

<style>
  .pe-loading {
    height: 100px;
    border-radius: 6px;
    background: linear-gradient(90deg, #e2e8f0 25%, #f1f5f9 50%, #e2e8f0 75%);
    background-size: 200% 100%;
    animation: pe-shimmer 1.4s ease-in-out infinite;
  }
  @keyframes pe-shimmer {
    0% { background-position: 200% 0; }
    100% { background-position: -200% 0; }
  }
  .pe-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }
  .pe-table th, .pe-table td {
    padding: 8px 12px;
    text-align: center;
    border-bottom: 1px solid #e5e7eb;
  }
  :global(.dark) .pe-table th, :global(.dark) .pe-table td {
    border-bottom-color: #334155;
  }
  .pe-table th {
    font-weight: 700;
    color: #374151;
  }
  :global(.dark) .pe-table th {
    color: #cbd5e1;
  }
  .pe-field-name {
    text-align: left !important;
    font-family: 'Courier New', monospace;
    font-size: 12px;
  }
  .pe-checkbox-label {
    display: inline-flex;
    cursor: pointer;
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
