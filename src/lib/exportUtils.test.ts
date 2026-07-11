import { describe, it, expect } from 'vitest';
import {
  specimenRows,
  subcultureRows,
  mediaRows,
  inventoryRows,
  complianceRows,
  prepSolutionRows,
} from './exportUtils';

// ── specimenRows ──────────────────────────────────────────────────────────────

describe('specimenRows', () => {
  it('returns header row + data row for a single specimen', () => {
    const json = JSON.stringify([{
      accession_number: '2026-01-01-CIT-001',
      species_code: 'CIT',
      species_name: 'Citrus sinensis',
      stage: 'shoot',
      provenance: 'Field A',
      initiation_date: '2026-01-01',
      location: 'Room 1 / Rack A',
      health_status: '4',
      quarantine_flag: false,
      subculture_count: 3,
      notes: 'test note',
    }]);
    const rows = specimenRows(json);
    expect(rows).toHaveLength(2);
    expect(rows[0][0]).toBe('Accession');
    expect(rows[1][0]).toBe('2026-01-01-CIT-001');
    expect(rows[1][8]).toBe('No');
  });

  it('maps quarantine_flag true to "Yes"', () => {
    const json = JSON.stringify([{ quarantine_flag: true }]);
    const rows = specimenRows(json);
    expect(rows[1][8]).toBe('Yes');
  });

  it('fills nullish fields with empty string', () => {
    const json = JSON.stringify([{}]);
    const rows = specimenRows(json);
    expect(rows[1][0]).toBe('');
    expect(rows[1][10]).toBe('');
  });

  it('returns only header row for empty array', () => {
    const rows = specimenRows('[]');
    expect(rows).toHaveLength(1);
    expect(rows[0][0]).toBe('Accession');
  });

  it('returns empty array for invalid JSON', () => {
    expect(specimenRows('not-json')).toEqual([]);
  });

  it('has 11 columns in the header', () => {
    const rows = specimenRows('[]');
    expect(rows[0]).toHaveLength(11);
  });
});

// ── subcultureRows ────────────────────────────────────────────────────────────

describe('subcultureRows', () => {
  it('returns header + one data row', () => {
    const rows = subcultureRows([{
      specimen_id: 'abc',
      passage_number: 2,
      date: '2026-06-01',
      contamination_flag: true,
    }]);
    expect(rows).toHaveLength(2);
    expect(rows[0][0]).toBe('Specimen ID');
    expect(rows[1][0]).toBe('abc');
    expect(rows[1][7]).toBe('Yes');
  });

  it('maps contamination_flag false to "No"', () => {
    const rows = subcultureRows([{ contamination_flag: false }]);
    expect(rows[1][7]).toBe('No');
  });

  it('returns only header for empty array', () => {
    expect(subcultureRows([])).toHaveLength(1);
  });

  it('has 15 columns in the header', () => {
    expect(subcultureRows([])[0]).toHaveLength(15);
  });
});

// ── mediaRows ─────────────────────────────────────────────────────────────────

describe('mediaRows', () => {
  // Uses the REAL serialized MediaBatch shape so field-name drift is caught.
  it('maps real MediaBatch fields to every column', () => {
    const rows = mediaRows([{
      name: 'MS Base', batch_id: 'MB-001', basal_salts: 'MS',
      employee_id: 'tech-7', preparation_date: '2026-06-01',
      expiration_date: '2026-12-01', ph_before_autoclave: 5.8,
      volume_prepared_ml: 1000, sterilization_method: 'autoclave', notes: 'ok',
    }]);
    expect(rows[1]).toEqual(
      ['MS Base', 'MB-001', 'MS', 'tech-7', '2026-06-01', '2026-12-01', 5.8, 1000, 'autoclave', 'ok'],
    );
  });

  it('falls back to created_by when employee_id is absent', () => {
    const rows = mediaRows([{ created_by: 'user-uuid' }]);
    expect(rows[1][3]).toBe('user-uuid');
  });

  it('has 10 columns in the header', () => {
    expect(mediaRows([])[0]).toHaveLength(10);
  });
});

// ── inventoryRows ─────────────────────────────────────────────────────────────

describe('inventoryRows', () => {
  // Real InventoryItem shape: minimum_stock + storage_location (not min_stock/location).
  it('maps real InventoryItem fields to every column', () => {
    const rows = inventoryRows([{
      name: 'Sucrose', category: 'reagent', unit: 'g', current_stock: 500,
      minimum_stock: 100, supplier: 'Sigma', catalog_number: 'S-1',
      storage_location: 'Shelf B', notes: 'n',
    }]);
    expect(rows[1]).toEqual(['Sucrose', 'reagent', 'g', 500, 100, 'Sigma', 'S-1', 'Shelf B', 'n']);
  });

  it('defaults current_stock to 0 when absent', () => {
    const rows = inventoryRows([{}]);
    expect(rows[1][3]).toBe(0);
  });

  it('has 9 columns in the header', () => {
    expect(inventoryRows([])[0]).toHaveLength(9);
  });
});

// ── complianceRows ────────────────────────────────────────────────────────────

describe('complianceRows', () => {
  // Real ComplianceRecord shape: agency / permit_number / permit_expiry.
  it('maps real ComplianceRecord fields correctly', () => {
    const rows = complianceRows([{
      specimen_id: 'sp1', record_type: 'permit', status: 'valid',
      agency: 'USDA', permit_number: 'P-123', permit_expiry: '2027-01-01',
      notes: 'ok',
    }]);
    expect(rows[1]).toEqual(['sp1', 'permit', 'valid', 'USDA', 'P-123', '2027-01-01', 'ok']);
  });

  it('has 7 columns in the header', () => {
    expect(complianceRows([])[0]).toHaveLength(7);
  });
});

// ── prepSolutionRows ──────────────────────────────────────────────────────────

describe('prepSolutionRows', () => {
  // Real PreparedSolution shape: preparation_date / expiration_date / storage_conditions.
  it('maps real PreparedSolution fields to every column', () => {
    const rows = prepSolutionRows([{
      name: 'IAA 1mM', concentration: 1, solvent: 'EtOH', prepared_by: 'tech-3',
      preparation_date: '2026-05-01', expiration_date: '2026-08-01',
      volume_ml: 50, storage_conditions: '4C', notes: 'n',
    }]);
    expect(rows[1]).toEqual(
      ['IAA 1mM', 1, 'EtOH', 'tech-3', '2026-05-01', '2026-08-01', 50, '4C', 'n'],
    );
  });

  it('uses prepared_by_name over prepared_by', () => {
    const rows = prepSolutionRows([{ prepared_by_name: 'Carol', prepared_by: 'id' }]);
    expect(rows[1][3]).toBe('Carol');
  });

  it('has 9 columns in the header', () => {
    expect(prepSolutionRows([])[0]).toHaveLength(9);
  });
});
