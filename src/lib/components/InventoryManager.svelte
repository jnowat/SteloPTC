<script lang="ts">
  import { onMount } from 'svelte';
  import { listInventory, createInventoryItem, updateInventoryItem, deleteInventoryItem, adjustStock, listPreparedSolutions, createPreparedSolution, updatePreparedSolution, deletePreparedSolution } from '../api';
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
    physical_state: 'solid',
    concentration: '', concentration_unit: 'mg/L',
  });

  let adjustForm = $state({ amount: '', reason: '' });

  let solutions = $state<any[]>([]);
  let showSolutionForm = $state(false);
  let solutionForm = $state({
    name: '', source_item_id: '', concentration: '', concentration_unit: 'µM',
    solvent: '', volume_ml: '', prepared_by: '', preparation_date: new Date().toISOString().split('T')[0],
    expiration_date: '', storage_conditions: '', lot_number: '', notes: '',
    source_amount_used: '',
  });
  let editingSolutionId = $state<string | null>(null);

  const categories = [
    { value: 'media_ingredient', label: 'Media Ingredient' },
    { value: 'vessel', label: 'Vessel' },
    { value: 'hormone', label: 'Hormone' },
    { value: 'chemical', label: 'Chemical' },
    { value: 'consumable', label: 'Consumable' },
    { value: 'equipment', label: 'Equipment' },
    { value: 'other', label: 'Other' },
  ];

  onMount(() => {
    load();
    listPreparedSolutions().then(s => solutions = s).catch((e: any) => addNotification(e.message, 'error'));
  });

  async function load() {
    loading = true;
    try { items = await listInventory(); }
    catch (e: any) { addNotification(e.message, 'error'); }
    finally { loading = false; }
    listPreparedSolutions().then(s => solutions = s).catch((e: any) => addNotification(e.message, 'error'));
  }

  function resetForm() {
    form = {
      name: '', category: 'consumable', unit: 'units',
      current_stock: '0', minimum_stock: '0', reorder_point: '',
      supplier: '', catalog_number: '', lot_number: '',
      storage_location: '', expiration_date: '', cost_per_unit: '', notes: '',
      physical_state: 'solid',
      concentration: '', concentration_unit: 'mg/L',
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
      physical_state: item.physical_state || 'solid',
      concentration: item.concentration != null ? String(item.concentration) : '',
      concentration_unit: item.concentration_unit || 'mg/L',
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
          physical_state: form.physical_state,
          concentration: form.physical_state === 'liquid' && form.concentration ? parseFloat(form.concentration) : undefined,
          concentration_unit: form.physical_state === 'liquid' && form.concentration ? form.concentration_unit || undefined : undefined,
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
          physical_state: form.physical_state,
          concentration: form.physical_state === 'liquid' && form.concentration ? parseFloat(form.concentration) : undefined,
          concentration_unit: form.physical_state === 'liquid' && form.concentration ? form.concentration_unit || undefined : undefined,
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

  function resetSolutionForm() {
    solutionForm = {
      name: '', source_item_id: '', concentration: '', concentration_unit: 'µM',
      solvent: '', volume_ml: '', prepared_by: '', preparation_date: new Date().toISOString().split('T')[0],
      expiration_date: '', storage_conditions: '', lot_number: '', notes: '',
      source_amount_used: '',
    };
    editingSolutionId = null;
  }

  async function handleSolutionSubmit(e: Event) {
    e.preventDefault();
    try {
      await createPreparedSolution({
        name: solutionForm.name,
        source_item_id: solutionForm.source_item_id || undefined,
        concentration: solutionForm.concentration ? parseFloat(solutionForm.concentration) : undefined,
        concentration_unit: solutionForm.concentration_unit || undefined,
        solvent: solutionForm.solvent || undefined,
        volume_ml: solutionForm.volume_ml ? parseFloat(solutionForm.volume_ml) : undefined,
        prepared_by: solutionForm.prepared_by || undefined,
        preparation_date: solutionForm.preparation_date || undefined,
        expiration_date: solutionForm.expiration_date || undefined,
        storage_conditions: solutionForm.storage_conditions || undefined,
        lot_number: solutionForm.lot_number || undefined,
        notes: solutionForm.notes || undefined,
        source_amount_used: solutionForm.source_amount_used ? parseFloat(solutionForm.source_amount_used) : undefined,
      });
      addNotification('Prepared solution created', 'success');
      showSolutionForm = false;
      resetSolutionForm();
      listPreparedSolutions().then(s => solutions = s).catch((e: any) => addNotification(e.message, 'error'));
    } catch (e: any) { addNotification(e.message, 'error'); }
  }

  async function handleSolutionDelete(id: string) {
    if (!confirm('Delete this prepared solution?')) return;
    try {
      await deletePreparedSolution(id);
      addNotification('Prepared solution deleted', 'success');
      listPreparedSolutions().then(s => solutions = s).catch((e: any) => addNotification(e.message, 'error'));
    } catch (e: any) { addNotification(e.message, 'error'); }
  }

  async function handleSolutionUpdate(sol: any, newVolumeRemaining: string) {
    try {
      await updatePreparedSolution({
        id: sol.id,
        volume_remaining_ml: newVolumeRemaining ? parseFloat(newVolumeRemaining) : undefined,
      });
      addNotification('Volume updated', 'success');
      listPreparedSolutions().then(s => solutions = s).catch((e: any) => addNotification(e.message, 'error'));
    } catch (e: any) { addNotification(e.message, 'error'); }
  }

  let solutionVolumeInputs = $state<Record<string, string>>({});
</script>

<div>
  <div class="page-header">
    <h1>Inventory</h1>
    {#if $currentUser?.role !== 'guest'}
      <button class="btn btn-primary" title={showForm ? 'Cancel and close the new item form without saving' : 'Open the form to add a new inventory item'} onclick={() => { if (showForm) { showForm = false; resetForm(); } else { resetForm(); showForm = true; } }}>
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
            <label title="The name of this inventory item (e.g., Agar Powder, MS Salts)">Name *</label>
            <input type="text" title="Enter the full name of the inventory item" bind:value={form.name} placeholder="e.g., Agar Powder" required />
          </div>
          <div class="form-group">
            <label title="The category this item belongs to — affects how it is grouped and reported">Category *</label>
            <select title="Select the category: media ingredient, vessel, hormone, chemical, consumable, equipment, or other" bind:value={form.category} required>
              {#each categories as cat}
                <option value={cat.value}>{cat.label}</option>
              {/each}
            </select>
          </div>
          <div class="form-group">
            <label title="Whether this item is a solid (weighed by mass) or liquid (measured by volume)">Physical State</label>
            <div style="display:flex; gap:16px; align-items:center; margin-top:4px;">
              <label title="Select Solid if this item is a powder or other solid measured by mass (g, mg)" style="display:inline-flex; align-items:center; gap:6px; text-transform:none; letter-spacing:0; cursor:pointer; font-weight:normal;">
                <input type="radio" title="This item is a solid (powder, crystals, etc.) measured by mass" bind:group={form.physical_state} value="solid" style="width:auto;" /> Solid
              </label>
              <label title="Select Liquid if this item is a solution measured by volume (mL, µL)" style="display:inline-flex; align-items:center; gap:6px; text-transform:none; letter-spacing:0; cursor:pointer; font-weight:normal;">
                <input type="radio" title="This item is a liquid or solution measured by volume" bind:group={form.physical_state} value="liquid" style="width:auto;" /> Liquid / Solution
              </label>
            </div>
          </div>
        </div>
        <div class="form-row-3">
          <div class="form-group">
            <label title="The unit used to measure stock quantity (e.g., g, mL, units)">Unit *</label>
            <input type="text" title="Type or select the unit of measurement — used for all stock quantities and alerts" list="unit-options" bind:value={form.unit} placeholder="g, mg, mL..." required />
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
          <div class="form-group">
            <label title="The current amount in stock in the selected unit">Current Stock</label>
            <input type="number" title="Enter the current quantity on hand — used to track low-stock alerts" step="0.01" bind:value={form.current_stock} />
          </div>
          <div class="form-group">
            <label title="The minimum stock level below which a low-stock alert is triggered">Minimum Stock</label>
            <input type="number" title="When current stock falls to or below this amount, a low-stock alert is raised" step="0.01" bind:value={form.minimum_stock} />
          </div>
        </div>
        {#if form.physical_state === 'liquid'}
        <div class="form-row">
          <div class="form-group" style="flex:2;">
            <label title="The concentration of this stock solution (numeric value only — select unit separately)">Stock Concentration</label>
            <input type="number" title="Enter the numeric concentration value of this stock solution (e.g., 10 for 10 mM BAP)" step="any" bind:value={form.concentration} placeholder="e.g., 10" />
          </div>
          <div class="form-group" style="flex:1;">
            <label title="The unit of concentration for this liquid item (e.g., mM, mg/L)">Unit</label>
            <select title="Select the concentration unit for this liquid item" bind:value={form.concentration_unit}>
              <option value="nM">nM</option>
              <option value="µM">µM</option>
              <option value="mM">mM</option>
              <option value="M">M</option>
              <option value="ng/mL">ng/mL</option>
              <option value="µg/mL">µg/mL</option>
              <option value="mg/mL">mg/mL</option>
              <option value="mg/L">mg/L</option>
              <option value="g/L">g/L</option>
              <option value="%">% (v/v or w/v)</option>
            </select>
          </div>
        </div>
        {/if}
        <div class="form-row-3">
          <div class="form-group">
            <label title="An optional higher threshold at which you'd want to reorder — shown in low-stock reports">Reorder Point</label>
            <input type="number" title="When stock reaches this level, it appears in reorder alerts (must be above minimum stock)" step="0.01" bind:value={form.reorder_point} placeholder="Optional" />
          </div>
          <div class="form-group">
            <label title="The name of the supplier or manufacturer for this item">Supplier</label>
            <input type="text" title="Enter the supplier or manufacturer (e.g., Sigma-Aldrich, PhytoTech)" bind:value={form.supplier} placeholder="e.g., Sigma-Aldrich" />
          </div>
          <div class="form-group">
            <label title="The supplier's catalog or product number for this item">Catalog Number</label>
            <input type="text" title="Enter the supplier catalog number for quick reordering" bind:value={form.catalog_number} />
          </div>
        </div>
        <div class="form-row-3">
          <div class="form-group">
            <label title="The lot or batch number from the supplier — used for quality traceability">Lot Number</label>
            <input type="text" title="Enter the lot or batch number from the supplier for traceability" bind:value={form.lot_number} />
          </div>
          <div class="form-group">
            <label title="The physical location where this item is stored in the lab">Storage Location</label>
            <input type="text" title="Enter the storage location (e.g., Shelf B-3, Fridge 2, -20°C Freezer)" bind:value={form.storage_location} placeholder="e.g., Shelf B-3" />
          </div>
          <div class="form-group">
            <label title="The date after which this item should no longer be used">Expiration Date</label>
            <input type="date" title="Select the expiration date — expired items will be highlighted in the inventory table" bind:value={form.expiration_date} />
          </div>
        </div>
        <div class="form-row-3">
          <div class="form-group">
            <label title="The cost per unit in USD — used for budget tracking and reports">Cost per Unit ($)</label>
            <input type="number" title="Enter the cost per unit in USD for budget and usage reports" step="0.01" bind:value={form.cost_per_unit} />
          </div>
        </div>
        <div class="form-group">
          <label title="Any additional notes about this inventory item (storage requirements, hazards, preparation tips)">Notes</label>
          <textarea title="Enter any additional notes: special storage requirements, hazards, handling instructions, etc." bind:value={form.notes} rows="2"></textarea>
        </div>
        <div style="text-align:right;">
          <button type="submit" title={editingId ? 'Save changes to this inventory item' : 'Create a new inventory item with the details entered above'} class="btn btn-primary">{editingId ? 'Update Item' : 'Create Item'}</button>
        </div>
      </form>
    </div>
  {/if}

  <div class="filters card" style="margin-bottom:16px; display:flex; gap:12px; align-items:center; flex-wrap:wrap;">
    <input type="text" title="Search inventory items by name, supplier, or catalog number" bind:value={searchQuery} placeholder="Search items..." style="max-width:220px;" />
    <select title="Filter inventory items by category" bind:value={filterCategory} style="max-width:180px;">
      <option value="">All Categories</option>
      {#each categories as cat}
        <option value={cat.value}>{cat.label}</option>
      {/each}
    </select>
    <label title="Show only items that are currently below their minimum or reorder stock threshold" style="display:inline-flex; align-items:center; gap:6px; font-size:13px; text-transform:none; letter-spacing:0; cursor:pointer;">
      <input type="checkbox" title="Toggle to show only low-stock items" bind:checked={filterLowStock} style="width:auto;" />
      Low stock only
    </label>
    <span style="font-size:12px; color:#6b7280;">
      {filtered.length} item{filtered.length !== 1 ? 's' : ''}
      {#if lowStockCount > 0}
        &middot; <span class="low-stock-count" title="{lowStockCount} item{lowStockCount !== 1 ? 's are' : ' is'} at or below their minimum stock or reorder point — check inventory">{lowStockCount} low stock</span>
      {/if}
    </span>
  </div>

  {#if showAdjust}
    <div class="card" style="margin-bottom:16px;">
      <form onsubmit={handleAdjust}>
        <h3 style="margin-bottom:12px;">Adjust Stock</h3>
        <div class="form-row">
          <div class="form-group">
            <label title="The amount to add or subtract from current stock — use negative values to deduct (e.g., -5 to use 5 units)">Amount (+/-)</label>
            <input type="number" title="Enter a positive number to add stock, or a negative number to deduct (e.g., -5 to record usage of 5 units)" step="0.01" bind:value={adjustForm.amount} placeholder="e.g., -5 or 100" required />
          </div>
          <div class="form-group">
            <label title="A brief explanation of why the stock level is being adjusted">Reason</label>
            <input type="text" title="Enter the reason for the adjustment (e.g., Used for media prep, Received new shipment, Waste/spillage)" bind:value={adjustForm.reason} placeholder="e.g., Used for media prep" />
          </div>
        </div>
        <div style="display:flex; gap:8px; justify-content:flex-end;">
          <button type="button" title="Cancel the stock adjustment and close this form" class="btn" onclick={() => { showAdjust = null; }}>Cancel</button>
          <button type="submit" title="Apply the stock adjustment and update the current stock level" class="btn btn-primary">Apply Adjustment</button>
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
            <th title="Item name and supplier catalog number">Name</th>
            <th title="Category grouping: media ingredient, vessel, hormone, chemical, consumable, equipment, or other">Category</th>
            <th title="Physical state: solid (mass-based) or liquid (volume-based), with concentration for liquids">State</th>
            <th title="Current stock quantity in the item's unit — highlighted yellow when below minimum or reorder point">Stock</th>
            <th title="Minimum stock threshold below which a low-stock alert is raised">Min</th>
            <th title="Supplier or manufacturer of this item">Supplier</th>
            <th title="Physical storage location in the lab (e.g., shelf, fridge, freezer)">Location</th>
            <th title="Expiration date — items past this date are highlighted in red">Expires</th>
            <th title="Current status: OK (sufficient stock), Low Stock (below threshold), or Expired">Status</th>
            <th title="Actions: adjust stock level, edit item details, or delete the item"></th>
          </tr>
        </thead>
        <tbody>
          {#each filtered as item}
            <tr>
              <td>
                <strong>{item.name}</strong>
                {#if item.catalog_number}
                  <div style="font-size:11px; color:#6b7280;" title="Supplier catalog number: {item.catalog_number} — use for reordering">{item.catalog_number}</div>
                {/if}
              </td>
              <td><span class="badge badge-gray" title="Category: {getCategoryLabel(item.category)}">{getCategoryLabel(item.category)}</span></td>
              <td>
                <span
                  class="badge {item.physical_state === 'liquid' ? 'badge-blue' : 'badge-gray'}"
                  title={item.physical_state === 'liquid' ? 'Liquid — stock tracked by volume (mL, L, µL)' : 'Solid — stock tracked by mass (g, mg, µg)'}
                >
                  {item.physical_state === 'liquid' ? 'Liquid' : 'Solid'}
                </span>
                {#if item.physical_state === 'liquid' && item.concentration}
                  <div style="font-size:11px; color:#6b7280; margin-top:2px;" title="Stock solution concentration: {item.concentration} {item.concentration_unit || ''}">{item.concentration} {item.concentration_unit || ''}</div>
                {/if}
              </td>
              <td
                class:low-stock={isLowStock(item)}
                title={isLowStock(item) ? `Low stock — current (${item.current_stock} ${item.unit}) is at or below minimum (${item.minimum_stock} ${item.unit})` : `Current stock: ${item.current_stock} ${item.unit}`}
              >
                <strong>{item.current_stock}</strong> {item.unit}
              </td>
              <td title="Minimum stock threshold: {item.minimum_stock} {item.unit} — system warns when stock falls to or below this level">{item.minimum_stock} {item.unit}</td>
              <td title={item.supplier ? `Supplier: ${item.supplier}` : 'No supplier recorded'}>{item.supplier || '—'}</td>
              <td title={item.storage_location ? `Storage location: ${item.storage_location}` : 'No storage location recorded'}>{item.storage_location || '—'}</td>
              <td>
                {#if item.expiration_date}
                  <span
                    class:expired={isExpired(item.expiration_date)}
                    title={isExpired(item.expiration_date) ? `Expired on ${item.expiration_date} — do not use` : `Expires on ${item.expiration_date}`}
                  >
                    {item.expiration_date}
                  </span>
                {:else}
                  —
                {/if}
              </td>
              <td>
                {#if isExpired(item.expiration_date)}
                  <span class="badge badge-red" title="This item has passed its expiration date and should not be used">Expired</span>
                {:else if isLowStock(item)}
                  <span class="badge badge-yellow" title="Current stock is at or below the minimum or reorder threshold — consider restocking">Low Stock</span>
                {:else}
                  <span class="badge badge-green" title="Stock level is sufficient and the item has not expired">OK</span>
                {/if}
              </td>
              <td>
                <div style="display:flex; gap:4px;">
                  {#if $currentUser?.role !== 'guest'}
                    <button class="btn btn-sm" title="Adjust the stock level for {item.name} — enter a positive or negative amount" onclick={() => { showAdjust = item.id; adjustForm = { amount: '', reason: '' }; }}>+/-</button>
                    <button class="btn btn-sm" title="Edit the details for {item.name}" onclick={() => startEdit(item)}>Edit</button>
                  {/if}
                  {#if $currentUser?.role === 'admin' || $currentUser?.role === 'supervisor'}
                    <button class="btn btn-sm btn-danger" title="Permanently delete {item.name} from inventory — this cannot be undone" onclick={() => handleDelete(item.id)}>Del</button>
                  {/if}
                </div>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}

  <!-- Prepared Solutions Section -->
  <div style="margin-top:32px;">
    <div class="page-header" style="margin-bottom:16px;">
      <h2 style="margin:0;">Prepared Stock Solutions</h2>
      {#if $currentUser?.role !== 'guest'}
        <button class="btn btn-primary" title={showSolutionForm ? 'Cancel and close the new solution form without saving' : 'Open the form to record a new prepared stock solution'} onclick={() => { if (showSolutionForm) { showSolutionForm = false; resetSolutionForm(); } else { resetSolutionForm(); showSolutionForm = true; } }}>
          {showSolutionForm ? 'Cancel' : '+ New Solution'}
        </button>
      {/if}
    </div>

    {#if showSolutionForm}
      <div class="card" style="margin-bottom:16px;">
        <form onsubmit={handleSolutionSubmit}>
          <h3 style="margin-bottom:16px;">New Prepared Solution</h3>
          <div class="form-row-3">
            <div class="form-group">
              <label title="The name or description of this prepared solution (e.g., 10 mM BAP stock, 1 mg/mL IBA in DMSO)">Solution Name *</label>
              <input type="text" title="Enter a descriptive name for this prepared solution" bind:value={solutionForm.name} placeholder="e.g., 10 mM BAP stock" required />
            </div>
            <div class="form-group">
              <label title="The inventory item used as the starting material for this solution — links this record to raw stock">Source Inventory Item</label>
              <select title="Select the inventory item this solution was prepared from — the source amount will be deducted from that item's stock" bind:value={solutionForm.source_item_id}>
                <option value="">— None —</option>
                {#each items as item}
                  <option value={item.id}>{item.name}</option>
                {/each}
              </select>
            </div>
            <div class="form-group">
              <label title="The amount of the source inventory item consumed to prepare this solution (deducted from stock)">Source Amount Used</label>
              <input type="number" title="Enter the amount of source material used — this will be subtracted from the selected inventory item's stock" step="any" bind:value={solutionForm.source_amount_used} placeholder="Amount deducted from stock" />
            </div>
          </div>
          <div class="form-row">
            <div class="form-group" style="flex:2;">
              <label title="The concentration of this prepared stock solution (numeric value only — select unit separately)">Concentration</label>
              <input type="number" title="Enter the numeric concentration of the prepared solution (e.g., 10 for 10 mM)" step="any" bind:value={solutionForm.concentration} placeholder="e.g., 10" />
            </div>
            <div class="form-group" style="flex:1;">
              <label title="The unit for the solution concentration (e.g., mM, µM, mg/mL)">Unit</label>
              <select title="Select the concentration unit for this prepared solution" bind:value={solutionForm.concentration_unit}>
                <option value="nM">nM</option>
                <option value="µM">µM</option>
                <option value="mM">mM</option>
                <option value="M">M</option>
                <option value="ng/mL">ng/mL</option>
                <option value="µg/mL">µg/mL</option>
                <option value="mg/mL">mg/mL</option>
                <option value="mg/L">mg/L</option>
                <option value="g/L">g/L</option>
                <option value="%">% (v/v or w/v)</option>
              </select>
            </div>
            <div class="form-group" style="flex:2;">
              <label title="The solvent used to dissolve or dilute the source material (e.g., DMSO, distilled water, ethanol)">Solvent</label>
              <input type="text" title="Enter the solvent used to prepare this solution (e.g., DMSO, dH2O, 70% ethanol)" bind:value={solutionForm.solvent} placeholder="e.g., DMSO, dH2O" />
            </div>
            <div class="form-group" style="flex:1;">
              <label title="The total volume of solution prepared in milliliters">Volume (mL)</label>
              <input type="number" title="Enter the total volume of prepared solution in mL — remaining volume can be updated later" step="any" bind:value={solutionForm.volume_ml} placeholder="e.g., 10" />
            </div>
          </div>
          <div class="form-row-3">
            <div class="form-group">
              <label title="The person who prepared this solution — for traceability and QC purposes">Prepared By</label>
              <input type="text" title="Enter the name or initials of the person who prepared this solution" bind:value={solutionForm.prepared_by} placeholder="Name or initials" />
            </div>
            <div class="form-group">
              <label title="The date this solution was prepared">Preparation Date</label>
              <input type="date" title="Select the date this solution was prepared" bind:value={solutionForm.preparation_date} />
            </div>
            <div class="form-group">
              <label title="The date after which this prepared solution should be discarded">Expiration Date</label>
              <input type="date" title="Select the expiration date — solutions past this date will be flagged in the table" bind:value={solutionForm.expiration_date} />
            </div>
          </div>
          <div class="form-row-3">
            <div class="form-group">
              <label title="The lot number of the source material used to prepare this solution">Lot Number</label>
              <input type="text" title="Enter the lot number from the source reagent for traceability" bind:value={solutionForm.lot_number} />
            </div>
            <div class="form-group">
              <label title="The required storage conditions to maintain solution stability">Storage Conditions</label>
              <input type="text" title="Enter storage requirements (e.g., -20°C in dark, 4°C, aliquot and freeze)" bind:value={solutionForm.storage_conditions} placeholder="e.g., -20°C, dark" />
            </div>
          </div>
          <div class="form-group">
            <label title="Any additional notes about this prepared solution (hazards, protocol reference, pH adjustments, etc.)">Notes</label>
            <textarea title="Enter any additional notes: hazards, preparation protocol details, pH adjustments, known issues, etc." bind:value={solutionForm.notes} rows="2"></textarea>
          </div>
          <div style="text-align:right;">
            <button type="submit" title="Save this prepared solution record to the database" class="btn btn-primary">Create Solution</button>
          </div>
        </form>
      </div>
    {/if}

    {#if solutions.length === 0}
      <div class="empty-state">No prepared solutions yet</div>
    {:else}
      <div class="card" style="overflow-x:auto;">
        <table>
          <thead>
            <tr>
              <th title="The name and lot number of the prepared solution, with storage conditions below">Name</th>
              <th title="The raw inventory item this solution was prepared from">Source Item</th>
              <th title="The concentration of this stock solution (value and unit)">Concentration</th>
              <th title="The current remaining volume in mL — update this field as the solution is consumed">Volume Remaining</th>
              <th title="The person who prepared this solution">Prepared By</th>
              <th title="The date this solution was prepared">Date</th>
              <th title="The expiration date of this solution — expired solutions are highlighted in red">Expires</th>
              <th title="Actions: delete this prepared solution record">Actions</th>
            </tr>
          </thead>
          <tbody>
            {#each solutions as sol}
              <tr>
                <td>
                  <strong>{sol.name}</strong>
                  {#if sol.lot_number}
                    <div style="font-size:11px; color:#6b7280;" title="Lot number of the source reagent used to prepare this solution: {sol.lot_number}">Lot: {sol.lot_number}</div>
                  {/if}
                  {#if sol.storage_conditions}
                    <div style="font-size:11px; color:#6b7280;" title="Storage conditions: {sol.storage_conditions}">{sol.storage_conditions}</div>
                  {/if}
                </td>
                <td title={sol.source_item_id ? 'Raw inventory item used as the starting material for this solution' : 'No source inventory item linked to this solution'}>
                  {#if sol.source_item_id}
                    {items.find(i => i.id === sol.source_item_id)?.name || sol.source_item_id}
                  {:else}
                    —
                  {/if}
                </td>
                <td title={sol.concentration != null ? `Stock solution concentration: ${sol.concentration} ${sol.concentration_unit || ''} — use this to calculate dilutions for media preparation` : 'No concentration recorded for this solution'}>
                  {#if sol.concentration != null}
                    {sol.concentration} {sol.concentration_unit || ''}
                  {:else}
                    —
                  {/if}
                </td>
                <td>
                  <div style="display:flex; gap:4px; align-items:center;">
                    <input
                      type="number"
                      step="any"
                      title="Enter the current remaining volume of {sol.name} in mL"
                      style="width:72px; padding:2px 6px; font-size:13px;"
                      value={solutionVolumeInputs[sol.id] ?? (sol.volume_remaining_ml != null ? String(sol.volume_remaining_ml) : (sol.volume_ml != null ? String(sol.volume_ml) : ''))}
                      oninput={(e) => { solutionVolumeInputs[sol.id] = (e.target as HTMLInputElement).value; }}
                      placeholder="mL"
                    />
                    <span style="font-size:12px; color:#6b7280;">mL</span>
                    {#if $currentUser?.role !== 'guest'}
                      <button class="btn btn-sm" title="Save the updated remaining volume for {sol.name}" onclick={() => handleSolutionUpdate(sol, solutionVolumeInputs[sol.id] ?? '')}>Update</button>
                    {/if}
                  </div>
                </td>
                <td title={sol.prepared_by ? `Prepared by: ${sol.prepared_by}` : 'Preparer not recorded'}>{sol.prepared_by || '—'}</td>
                <td title={sol.preparation_date ? `Preparation date: ${sol.preparation_date}` : 'Preparation date not recorded'}>{sol.preparation_date || '—'}</td>
                <td>
                  {#if sol.expiration_date}
                    <span
                      class:expired={isExpired(sol.expiration_date)}
                      title={isExpired(sol.expiration_date) ? `Expired on ${sol.expiration_date} — this solution should not be used` : `Expires on ${sol.expiration_date}`}
                    >{sol.expiration_date}</span>
                  {:else}
                    —
                  {/if}
                </td>
                <td>
                  {#if $currentUser?.role === 'admin' || $currentUser?.role === 'supervisor'}
                    <button class="btn btn-sm btn-danger" title="Permanently delete the {sol.name} solution record — this cannot be undone" onclick={() => handleSolutionDelete(sol.id)}>Del</button>
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </div>
</div>

<style>
  .low-stock { color: #d97706; font-weight: 700; }
  .low-stock-count { color: #d97706; font-weight: 600; }
  .expired { color: #dc2626; font-weight: 600; }
</style>
