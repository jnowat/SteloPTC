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
    'Name', 'Batch Code', 'Type', 'Prepared By', 'Date Prepared',
    'Expiry Date', 'pH', 'Volume mL', 'Sterilization Method', 'Notes',
  ];
  const rows = items.map((m: any) => [
    m.name ?? '',
    m.batch_code ?? '',
    m.media_type ?? '',
    m.prepared_by_name ?? m.prepared_by ?? '',
    m.date_prepared ?? '',
    m.expiry_date ?? '',
    m.ph ?? '',
    m.volume_ml ?? '',
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
  const rows = items.map((i: any) => [
    i.name ?? '',
    i.category ?? '',
    i.unit ?? '',
    i.current_stock ?? 0,
    i.min_stock ?? '',
    i.supplier ?? '',
    i.catalog_number ?? '',
    i.location ?? '',
    i.notes ?? '',
  ]);
  return [headers, ...rows];
}

export function complianceRows(items: any[]): any[][] {
  const headers = [
    'Specimen ID', 'Record Type', 'Status', 'Authority',
    'Issue Date', 'Expiry Date', 'Notes',
  ];
  const rows = items.map((c: any) => [
    c.specimen_id ?? '',
    c.record_type ?? '',
    c.status ?? '',
    c.authority ?? '',
    c.issue_date ?? '',
    c.expiry_date ?? '',
    c.notes ?? '',
  ]);
  return [headers, ...rows];
}

export function prepSolutionRows(items: any[]): any[][] {
  const headers = [
    'Name', 'Concentration', 'Solvent', 'Prepared By', 'Date Prepared',
    'Expiry Date', 'Volume mL', 'Storage Condition', 'Notes',
  ];
  const rows = items.map((p: any) => [
    p.name ?? '',
    p.concentration ?? '',
    p.solvent ?? '',
    p.prepared_by_name ?? p.prepared_by ?? '',
    p.date_prepared ?? '',
    p.expiry_date ?? '',
    p.volume_ml ?? '',
    p.storage_condition ?? '',
    p.notes ?? '',
  ]);
  return [headers, ...rows];
}
