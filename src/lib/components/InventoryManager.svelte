<script lang="ts">
  import { onMount } from 'svelte';
  import { listInventory, createInventoryItem, updateInventoryItem, deleteInventoryItem, adjustStock } from '../api';
  import { addNotification } from '../stores/app';
  import { currentUser } from '../stores/auth';

  let items = $state<any[]>([]);
  let loading = $state(true);
  let showForm = $state(false);
  let editingId = $state<string | null>(null);
  let showAdjust = $state<string | null>(null);
  let filterCategory = $state('');
  let filterLowStock = $state(false);
  let searchQuery = $state('');

  let form = $state({
    name: '', category: 'consumable', unit: 'units',
    current_stock: '0', minimum_stock: '0', reorder_point: '',
    supplier: '', catalog_number: '', lot_number: '',
    storage_location: '', expiration_date: '', cost_per_unit: '', notes: '',
  });

  let adjustForm = $state({ amount: '', reason: '' });

  const categories = [
    { value: 'media_ingredient', label: 'Media Ingredient' },
    { value: 'vessel', label: 'Vessel' },
    { value: 'hormone', label: 'Hormone' },
    { value: 'chemical', label: 'Chemical' },
    { value: 'consumable', label: 'Consumable' },
    { value: 'equipment', label: 'Equipment' },
    { value: 'other', label: 'Other' },
  ];

  onMount(() => { load(); });

  async function load() {
    loading = true;
    try { items = await listInventory(); }
    catch (e: any) { addNotification(e.message, 'error'); }
    finally { loading = false; }
  }

  function resetForm() {
    form = {
      name: '', category: 'consumable', unit: 'units',
      current_stock: '0', minimum_stock: '0', reorder_point: '',
      supplier: '', catalog_number: '', lot_number: '',
      storage_location: '', expiration_date: '', cost_per_unit: '', notes: '',
    };
    editingId = null;
  }

  function startEdit(item: any) {
    form = {
      name: item.name,
      category: item.category,
      unit: item.unit,
      current_stock: String(item.current_stock),
      minimum_stock: String(item.minimum_stock),
      reorder_point: item.reorder_point != null ? String(item.reorder_point) : '',
      supplier: item.supplier || '',
      catalog_number: item.catalog_number || '',
      lot_number: item.lot_number || '',
      storage_location: item.storage_location || '',
      expiration_date: item.expiration_date || '',
      cost_per_unit: item.cost_per_unit != null ? String(item.cost_per_unit) : '',
      notes: item.notes || '',
    };
    editingId = item.id;
    showForm = true;
  }

  async function handleSubmit(e: Event) {
    e.preventDefault();
    try {
      if (editingId) {
        await updateInventoryItem({
          id: editingId,
          name: form.name || undefined,
          category: form.category || undefined,
          unit: form.unit || undefined,
          current_stock: form.current_stock ? parseFloat(form.current_stock) : undefined,
          minimum_stock: form.minimum_stock ? parseFloat(form.minimum_stock) : undefined,
          reorder_point: form.reorder_point ? parseFloat(form.reorder_point) : undefined,
          supplier: form.supplier || undefined,
          catalog_number: form.catalog_number || undefined,
          lot_number: form.lot_number || undefined,
          storage_location: form.storage_location || undefined,
          expiration_date: form.expiration_date || undefined,
          cost_per_unit: form.cost_per_unit ? parseFloat(form.cost_per_unit) : undefined,
          notes: form.notes || undefined,
        });
        addNotification('Inventory item updated', 'success');
      } else {
        await createInventoryItem({
          name: form.name,
          category: form.category,
          unit: form.unit,
          current_stock: form.current_stock ? parseFloat(form.current_stock) : undefined,
          minimum_stock: form.minimum_stock ? parseFloat(form.minimum_stock) : undefined,
          reorder_point: form.reorder_point ? parseFloat(form.reorder_point) : undefined,
          supplier: form.supplier || undefined,
          catalog_number: form.catalog_number || undefined,
          lot_number: form.lot_number || undefined,
          storage_location: form.storage_location || undefined,
          expiration_date: form.expiration_date || undefined,
          cost_per_unit: form.cost_per_unit ? parseFloat(form.cost_per_unit) : undefined,
          notes: form.notes || undefined,
        });
        addNotification('Inventory item created', 'success');
      }
      showForm = false;
      resetForm();
      load();
    } catch (e: any) { addNotification(e.message, 'error'); }
  }

  async function handleDelete(id: string) {
    if (!confirm('Delete this inventory item?')) return;
    try {
      await deleteInventoryItem(id);
      addNotification('Item deleted', 'success');
      load();
    } catch (e: any) { addNotification(e.message, 'error'); }
  }

  async function handleAdjust(e: Event) {
    e.preventDefault();
    if (!showAdjust || !adjustForm.amount) return;
    try {
      await adjustStock(showAdjust, parseFloat(adjustForm.amount), adjustForm.reason || undefined);
      addNotification('Stock adjusted', 'success');
      showAdjust = null;
      adjustForm = { amount: '', reason: '' };
      load();
    } catch (e: any) { addNotification(e.message, 'error'); }
  }

  function isLowStock(item: any): boolean {
    if (item.current_stock <= item.minimum_stock) return true;
    if (item.reorder_point != null && item.current_stock <= item.reorder_point) return true;
    return false;
  }

  function isExpired(expDate: string | null): boolean {
    if (!expDate) return false;
    return new Date(expDate) < new Date();
  }

  function getCategoryLabel(val: string): string {
    return categories.find(c => c.value === val)?.label || val;
  }

  function getFilteredItems(): any[] {
    let result = items;
    if (filterCategory) result = result.filter(i => i.category === filterCategory);
    if (filterLowStock) result = result.filter(i => isLowStock(i));
    if (searchQuery) {
      const q = searchQuery.toLowerCase();
      result = result.filter(i =>
        i.name.toLowerCase().includes(q) ||
        (i.supplier && i.supplier.toLowerCase().includes(q)) ||
        (i.catalog_number && i.catalog_number.toLowerCase().includes(q))
      );
    }
    return result;
  }

  let filtered = $derived(getFilteredItems());
  let lowStockCount = $derived(items.filter(isLowStock).length);
</script>

<div>
  <div class="page-header">
    <h1>Inventory</h1>
    {#if $currentUser?.role !== 'guest'}
      <button class="btn btn-primary" onclick={() => { if (showForm) { showForm = false; resetForm(); } else { resetForm(); showForm = true; } }}>
        {showForm ? 'Cancel' : '+ New Item'}
      </button>
    {/if}
  </div>

  {#if showForm}
    <div class="card" style="margin-bottom:16px;">
      <form onsubmit={handleSubmit}>
        <h3 style="margin-bottom:16px;">{editingId ? 'Edit Item' : 'New Inventory Item'}</h3>
        <div class="form-row-3">
          <div class="form-group">
            <label>Name *</label>
            <input type="text" bind:value={form.name} placeholder="e.g., Agar Powder" required />
          </div>
          <div class="form-group">
            <label>Category *</label>
            <select bind:value={form.category} required>
              {#each categories as cat}
                <option value={cat.value}>{cat.label}</option>
              {/each}
            </select>
          </div>
          <div class="form-group">
            <label>Unit *</label>
            <input type="text" list="unit-options" bind:value={form.unit} placeholder="g, mg, mL..." required />
            <datalist id="unit-options">
              <option value="g">g (grams)</option>
              <option value="mg">mg (milligrams)</option>
              <option value="mL">mL (milliliters)</option>
              <option value="L">L (liters)</option>
              <option value="units">units</option>
              <option value="pcs">pcs (pieces)</option>
              <option value="µg">µg (micrograms)</option>
              <option value="µL">µL (microliters)</option>
            </datalist>
          </div>
        </div>
        <div class="form-row-3">
          <div class="form-group">
            <label>Current Stock</label>
            <input type="number" step="0.01" bind:value={form.current_stock} />
          </div>
          <div class="form-group">
            <label>Minimum Stock</label>
            <input type="number" step="0.01" bind:value={form.minimum_stock} />
          </div>
          <div class="form-group">
            <label>Reorder Point</label>
            <input type="number" step="0.01" bind:value={form.reorder_point} placeholder="Optional" />
          </div>
        </div>
        <div class="form-row-3">
          <div class="form-group">
            <label>Supplier</label>
            <input type="text" bind:value={form.supplier} placeholder="e.g., Sigma-Aldrich" />
          </div>
          <div class="form-group">
            <label>Catalog Number</label>
            <input type="text" bind:value={form.catalog_number} />
          </div>
          <div class="form-group">
            <label>Lot Number</label>
            <input type="text" bind:value={form.lot_number} />
          </div>
        </div>
        <div class="form-row-3">
          <div class="form-group">
            <label>Storage Location</label>
            <input type="text" bind:value={form.storage_location} placeholder="e.g., Shelf B-3" />
          </div>
          <div class="form-group">
            <label>Expiration Date</label>
            <input type="date" bind:value={form.expiration_date} />
          </div>
          <div class="form-group">
            <label>Cost per Unit ($)</label>
            <input type="number" step="0.01" bind:value={form.cost_per_unit} />
          </div>
        </div>
        <div class="form-group">
          <label>Notes</label>
          <textarea bind:value={form.notes} rows="2"></textarea>
        </div>
        <div style="text-align:right;">
          <button type="submit" class="btn btn-primary">{editingId ? 'Update Item' : 'Create Item'}</button>
        </div>
      </form>
    </div>
  {/if}

  <div class="filters card" style="margin-bottom:16px; display:flex; gap:12px; align-items:center; flex-wrap:wrap;">
    <input type="text" bind:value={searchQuery} placeholder="Search items..." style="max-width:220px;" />
    <select bind:value={filterCategory} style="max-width:180px;">
      <option value="">All Categories</option>
      {#each categories as cat}
        <option value={cat.value}>{cat.label}</option>
      {/each}
    </select>
    <label style="display:inline-flex; align-items:center; gap:6px; font-size:13px; text-transform:none; letter-spacing:0; cursor:pointer;">
      <input type="checkbox" bind:checked={filterLowStock} style="width:auto;" />
      Low stock only
    </label>
    <span style="font-size:12px; color:#6b7280;">
      {filtered.length} item{filtered.length !== 1 ? 's' : ''}
      {#if lowStockCount > 0}
        &middot; <span class="low-stock-count">{lowStockCount} low stock</span>
      {/if}
    </span>
  </div>

  {#if showAdjust}
    <div class="card" style="margin-bottom:16px;">
      <form onsubmit={handleAdjust}>
        <h3 style="margin-bottom:12px;">Adjust Stock</h3>
        <div class="form-row">
          <div class="form-group">
            <label>Amount (+/-)</label>
            <input type="number" step="0.01" bind:value={adjustForm.amount} placeholder="e.g., -5 or 100" required />
          </div>
          <div class="form-group">
            <label>Reason</label>
            <input type="text" bind:value={adjustForm.reason} placeholder="e.g., Used for media prep" />
          </div>
        </div>
        <div style="display:flex; gap:8px; justify-content:flex-end;">
          <button type="button" class="btn" onclick={() => { showAdjust = null; }}>Cancel</button>
          <button type="submit" class="btn btn-primary">Apply Adjustment</button>
        </div>
      </form>
    </div>
  {/if}

  {#if loading}
    <div class="empty-state">Loading inventory...</div>
  {:else if filtered.length === 0}
    <div class="empty-state">{items.length === 0 ? 'No inventory items yet' : 'No items match filters'}</div>
  {:else}
    <div class="card" style="overflow-x:auto;">
      <table>
        <thead>
          <tr>
            <th>Name</th>
            <th>Category</th>
            <th>Stock</th>
            <th>Min</th>
            <th>Supplier</th>
            <th>Location</th>
            <th>Expires</th>
            <th>Status</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {#each filtered as item}
            <tr>
              <td>
                <strong>{item.name}</strong>
                {#if item.catalog_number}
                  <div style="font-size:11px; color:#6b7280;">{item.catalog_number}</div>
                {/if}
              </td>
              <td><span class="badge badge-gray">{getCategoryLabel(item.category)}</span></td>
              <td class:low-stock={isLowStock(item)}>
                <strong>{item.current_stock}</strong> {item.unit}
              </td>
              <td>{item.minimum_stock} {item.unit}</td>
              <td>{item.supplier || '—'}</td>
              <td>{item.storage_location || '—'}</td>
              <td>
                {#if item.expiration_date}
                  <span class:expired={isExpired(item.expiration_date)}>
                    {item.expiration_date}
                  </span>
                {:else}
                  —
                {/if}
              </td>
              <td>
                {#if isExpired(item.expiration_date)}
                  <span class="badge badge-red">Expired</span>
                {:else if isLowStock(item)}
                  <span class="badge badge-yellow">Low Stock</span>
                {:else}
                  <span class="badge badge-green">OK</span>
                {/if}
              </td>
              <td>
                <div style="display:flex; gap:4px;">
                  {#if $currentUser?.role !== 'guest'}
                    <button class="btn btn-sm" onclick={() => { showAdjust = item.id; adjustForm = { amount: '', reason: '' }; }}>+/-</button>
                    <button class="btn btn-sm" onclick={() => startEdit(item)}>Edit</button>
                  {/if}
                  {#if $currentUser?.role === 'admin' || $currentUser?.role === 'supervisor'}
                    <button class="btn btn-sm btn-danger" onclick={() => handleDelete(item.id)}>Del</button>
                  {/if}
                </div>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

<style>
  .low-stock { color: #d97706; font-weight: 700; }
  .low-stock-count { color: #d97706; font-weight: 600; }
  .expired { color: #dc2626; font-weight: 600; }
</style>
