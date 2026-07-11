// Pure row-builder functions for Excel export sheets. Extracted from ExportManager.svelte
// so they can be unit-tested in isolation.

export function specimenRows(jsonStr: string): any[][] {
  let items: any[];
  try { items = JSON.parse(jsonStr); } catch { return []; }
  const headers = [
    'Accession', 'Species Code', 'Species', 'Stage', 'Provenance',
    'Initiation Date', 'Location', 'Health Status', 'Quarantine',
    'Subculture Count', 'Notes',
  ];
  const rows = items.map((s: any) => [
    s.accession_number ?? '',
    s.species_code ?? '',
    s.species_name ?? '',
    s.stage ?? '',
    s.provenance ?? '',
    s.initiation_date ?? '',
    s.location ?? '',
    s.health_status ?? '',
    s.quarantine_flag ? 'Yes' : 'No',
    s.subculture_count ?? 0,
    s.notes ?? '',
  ]);
  return [headers, ...rows];
}

export function subcultureRows(items: any[]): any[][] {
  const headers = [
    'Specimen ID', 'Passage #', 'Date', 'Media Batch', 'Vessel Type',
    'Vessel Size', 'Health Status', 'Contamination', 'Contamination Notes',
    'pH', 'Temp °C', 'Light Cycle', 'Performed By', 'Notes', 'Observations',
  ];
  const rows = items.map((sc: any) => [
    sc.specimen_id ?? '',
    sc.passage_number ?? '',
    sc.date ?? '',
    sc.media_batch_name ?? '',
    sc.vessel_type ?? '',
    sc.vessel_size ?? '',
    sc.health_status ?? '',
    sc.contamination_flag ? 'Yes' : 'No',
    sc.contamination_notes ?? '',
    sc.ph ?? '',
    sc.temperature_c ?? '',
    sc.light_cycle ?? '',
    sc.performer_name ?? '',
    sc.notes ?? '',
    sc.observations ?? '',
  ]);
  return [headers, ...rows];
}

export function mediaRows(items: any[]): any[][] {
  const headers = [
    'Name', 'Batch Code', 'Base', 'Prepared By', 'Date Prepared',
    'Expiry Date', 'pH', 'Volume mL', 'Sterilization Method', 'Notes',
  ];
  // Field names below must match the serialized MediaBatch (src-tauri/src/models/media.rs):
  // batch_id, basal_salts, preparation_date, expiration_date, ph_before_autoclave,
  // volume_prepared_ml. MediaBatch has no prepared_by field — the preparer is employee_id
  // (falling back to created_by). Reading the wrong names silently blanked these columns.
  const rows = items.map((m: any) => [
    m.name ?? '',
    m.batch_id ?? '',
    m.basal_salts ?? '',
    m.employee_id ?? m.created_by ?? '',
    m.preparation_date ?? '',
    m.expiration_date ?? '',
    m.ph_before_autoclave ?? '',
    m.volume_prepared_ml ?? '',
    m.sterilization_method ?? '',
    m.notes ?? '',
  ]);
  return [headers, ...rows];
}

export function inventoryRows(items: any[]): any[][] {
  const headers = [
    'Name', 'Category', 'Unit', 'Current Stock', 'Min Stock',
    'Supplier', 'Catalog #', 'Location', 'Notes',
  ];
  // minimum_stock / storage_location per InventoryItem (src-tauri/src/models/inventory.rs).
  const rows = items.map((i: any) => [
    i.name ?? '',
    i.category ?? '',
    i.unit ?? '',
    i.current_stock ?? 0,
    i.minimum_stock ?? '',
    i.supplier ?? '',
    i.catalog_number ?? '',
    i.storage_location ?? '',
    i.notes ?? '',
  ]);
  return [headers, ...rows];
}

export function complianceRows(items: any[]): any[][] {
  // Columns map to real ComplianceRecord fields (src-tauri/src/models/compliance.rs):
  // agency, permit_number, permit_expiry. There is no "issue date" on the record, so the
  // permit number is surfaced instead of an always-empty column.
  const headers = [
    'Specimen ID', 'Record Type', 'Status', 'Agency',
    'Permit #', 'Permit Expiry', 'Notes',
  ];
  const rows = items.map((c: any) => [
    c.specimen_id ?? '',
    c.record_type ?? '',
    c.status ?? '',
    c.agency ?? '',
    c.permit_number ?? '',
    c.permit_expiry ?? '',
    c.notes ?? '',
  ]);
  return [headers, ...rows];
}

export function prepSolutionRows(items: any[]): any[][] {
  const headers = [
    'Name', 'Concentration', 'Solvent', 'Prepared By', 'Date Prepared',
    'Expiry Date', 'Volume mL', 'Storage Condition', 'Notes',
  ];
  // preparation_date / expiration_date / storage_conditions per PreparedSolution
  // (src-tauri/src/models/inventory.rs). volume_ml and prepared_by already match.
  const rows = items.map((p: any) => [
    p.name ?? '',
    p.concentration ?? '',
    p.solvent ?? '',
    p.prepared_by_name ?? p.prepared_by ?? '',
    p.preparation_date ?? '',
    p.expiration_date ?? '',
    p.volume_ml ?? '',
    p.storage_conditions ?? '',
    p.notes ?? '',
  ]);
  return [headers, ...rows];
}
