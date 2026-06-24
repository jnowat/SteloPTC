use rusqlite::Connection;
use super::DbResult;

pub fn run_all(conn: &Connection) -> DbResult<()> {
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
    ")?;

    let current: i64 = conn
        .query_row("SELECT COALESCE(MAX(version), 0) FROM schema_version", [], |r| r.get(0))
        .unwrap_or(0);

    if current < 1 {
        migration_001_initial(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (1)", [])?;
    }

    if current < 2 {
        migration_002_v019(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (2)", [])?;
    }

    if current < 3 {
        migration_003_v0110(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (3)", [])?;
    }

    if current < 4 {
        migration_004_v0114(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (4)", [])?;
    }

    if current < 5 {
        migration_005_contamination_schedule(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (5)", [])?;
    }

    if current < 6 {
        migration_006_force_password_change(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (6)", [])?;
    }

    if current < 7 {
        migration_007_perf_indexes(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (7)", [])?;
    }

    if current < 8 {
        migration_008_audit_hash_chain(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (8)", [])?;
    }

    if current < 9 {
        migration_009_audit_lineage(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (9)", [])?;
    }

    if current < 10 {
        migration_010_specimen_genealogy(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (10)", [])?;
    }

    if current < 11 {
        migration_011_media_draft(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (11)", [])?;
    }

    if current < 12 {
        migration_012_specimen_contamination(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (12)", [])?;
    }

    if current < 13 {
        migration_013_audit_checkpoints(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (13)", [])?;
    }

    if current < 14 {
        migration_014_checkpoint_auto_and_settings(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (14)", [])?;
    }

    if current < 15 {
        migration_015_death_events_and_lab_profile(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (15)", [])?;
    }

    if current < 16 {
        migration_016_vocabulary_tables(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (16)", [])?;
    }

    if current < 17 {
        migration_017_remaining_vocabularies(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (17)", [])?;
    }

    if current < 18 {
        migration_018_cell_culture_vocabulary(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (18)", [])?;
    }

    if current < 19 {
        migration_019_strain_model(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (19)", [])?;
    }

    if current < 20 {
        migration_020_expanded_taxonomy(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (20)", [])?;
    }

    if current < 21 {
        migration_021_ncbi_sync_log(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (21)", [])?;
    }

    if current < 22 {
        migration_022_hybrid_generation_labels(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (22)", [])?;
    }

    if current < 23 {
        migration_023_cell_culture_vocabulary(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (23)", [])?;
    }

    if current < 24 {
        migration_024_pdl_fields(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (24)", [])?;
    }

    if current < 25 {
        migration_025_frozen_vials(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (25)", [])?;
    }

    Ok(())
}

fn migration_025_frozen_vials(conn: &Connection) -> DbResult<()> {
    // WP-32: cryopreservation & LN2 inventory.
    // Adds a first-class table for frozen vial lots with location, freeze details,
    // and status.  Vial counts have a CHECK >= 0 to prevent negative inventory.
    // Location fields mirror the Room/Rack/Shelf/Tray structure used on specimens,
    // renamed to Freezer/Tower/Box/Position for cryo context.
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS frozen_vials (
            id                TEXT    PRIMARY KEY,
            specimen_id       TEXT    REFERENCES specimens(id),
            species_id        TEXT    NOT NULL REFERENCES species(id),
            passage_number    INTEGER NOT NULL DEFAULT 0,
            cumulative_pdl    REAL,
            vial_count        INTEGER NOT NULL DEFAULT 1 CHECK(vial_count >= 0),
            freeze_date       TEXT    NOT NULL,
            freeze_medium     TEXT    NOT NULL,
            location          TEXT,
            location_freezer  TEXT,
            location_tower    TEXT,
            location_box      TEXT,
            location_position TEXT,
            status            TEXT    NOT NULL DEFAULT 'active'
                                      CHECK(status IN ('active','depleted','discarded')),
            notes             TEXT,
            created_by        TEXT,
            created_at        TEXT    NOT NULL DEFAULT (datetime('now')),
            updated_at        TEXT    NOT NULL DEFAULT (datetime('now'))
        );

        CREATE INDEX IF NOT EXISTS idx_frozen_vials_species
            ON frozen_vials(species_id);
        CREATE INDEX IF NOT EXISTS idx_frozen_vials_specimen
            ON frozen_vials(specimen_id);
        CREATE INDEX IF NOT EXISTS idx_frozen_vials_status
            ON frozen_vials(status);
    ")?;
    Ok(())
}

fn migration_024_pdl_fields(conn: &Connection) -> DbResult<()> {
    // WP-31: passage-number lineage & doubling time.
    // Adds cumulative PDL tracking on specimens and per-passage cell count /
    // doubling time columns on subcultures.  All columns are nullable so
    // existing rows continue to work without backfill.
    conn.execute_batch("
        ALTER TABLE specimens  ADD COLUMN cumulative_pdl       REAL;
        ALTER TABLE subcultures ADD COLUMN seed_cell_count     REAL;
        ALTER TABLE subcultures ADD COLUMN harvest_cell_count  REAL;
        ALTER TABLE subcultures ADD COLUMN split_ratio         REAL;
        ALTER TABLE subcultures ADD COLUMN pdl_gained          REAL;
        ALTER TABLE subcultures ADD COLUMN doubling_time_hours REAL;
    ")?;
    Ok(())
}

fn migration_016_vocabulary_tables(conn: &Connection) -> DbResult<()> {
    // WP-23: stages lookup table replaces the CHECK constraint on specimens.stage.
    // WP-24 (partial): propagation_methods lookup table replaces CHECK on specimens.propagation_method.
    //
    // Codes exactly match the values from the existing CHECK constraints so all
    // existing specimen rows remain valid after the rebuild.  After this migration,
    // adding a stage or propagation method only requires a row insert — no DDL.
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS stages (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            profile     TEXT    NOT NULL,
            code        TEXT    NOT NULL,
            label       TEXT    NOT NULL,
            sort_order  INTEGER NOT NULL DEFAULT 0,
            is_terminal INTEGER NOT NULL DEFAULT 0,
            UNIQUE(profile, code)
        );

        CREATE INDEX IF NOT EXISTS idx_stages_profile
            ON stages(profile, sort_order);

        INSERT OR IGNORE INTO stages (profile, code, label, sort_order, is_terminal) VALUES
            ('plant_tissue_culture', 'explant',         'Explant',         1,  0),
            ('plant_tissue_culture', 'callus',          'Callus',          2,  0),
            ('plant_tissue_culture', 'suspension',      'Suspension',      3,  0),
            ('plant_tissue_culture', 'protoplast',      'Protoplast',      4,  0),
            ('plant_tissue_culture', 'shoot',           'Shoot',           5,  0),
            ('plant_tissue_culture', 'shoot_meristem',  'Shoot Meristem',  6,  0),
            ('plant_tissue_culture', 'apical_meristem', 'Apical Meristem', 7,  0),
            ('plant_tissue_culture', 'root',            'Root',            8,  0),
            ('plant_tissue_culture', 'root_meristem',   'Root Meristem',   9,  0),
            ('plant_tissue_culture', 'embryogenic',     'Embryogenic',     10, 0),
            ('plant_tissue_culture', 'plantlet',        'Plantlet',        11, 0),
            ('plant_tissue_culture', 'acclimatized',    'Acclimatized',    12, 0),
            ('plant_tissue_culture', 'stock',           'Stock',           13, 0),
            ('plant_tissue_culture', 'archived',        'Archived',        14, 1),
            ('plant_tissue_culture', 'custom',          'Custom',          15, 0);

        CREATE TABLE IF NOT EXISTS propagation_methods (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            profile     TEXT    NOT NULL,
            code        TEXT    NOT NULL,
            label       TEXT    NOT NULL,
            sort_order  INTEGER NOT NULL DEFAULT 0,
            UNIQUE(profile, code)
        );

        CREATE INDEX IF NOT EXISTS idx_propagation_methods_profile
            ON propagation_methods(profile, sort_order);

        INSERT OR IGNORE INTO propagation_methods (profile, code, label, sort_order) VALUES
            ('plant_tissue_culture', 'microprop',             'Micropropagation',      1),
            ('plant_tissue_culture', 'somatic_embryogenesis', 'Somatic Embryogenesis', 2),
            ('plant_tissue_culture', 'organogenesis',         'Organogenesis',         3),
            ('plant_tissue_culture', 'meristem_culture',      'Meristem Culture',      4),
            ('plant_tissue_culture', 'anther_culture',        'Anther Culture',        5),
            ('plant_tissue_culture', 'protoplast_fusion',     'Protoplast Fusion',     6),
            ('plant_tissue_culture', 'other',                 'Other',                 7);
    ")?;

    // Rebuild specimens to drop CHECK constraints on stage and propagation_method.
    // acclimatization_status keeps its CHECK — it is not in the WP-23/24 scope.
    // All columns from migrations 002–012 (genealogy from 010, contamination from 012)
    // are preserved exactly.
    conn.execute("PRAGMA foreign_keys = OFF", [])?;
    let result = conn.execute_batch("
        BEGIN;
        CREATE TABLE IF NOT EXISTS specimens_v16 (
            id                      TEXT    PRIMARY KEY,
            accession_number        TEXT    NOT NULL UNIQUE,
            species_id              TEXT    NOT NULL REFERENCES species(id),
            project_id              TEXT    REFERENCES projects(id),
            stage                   TEXT    NOT NULL DEFAULT 'explant',
            custom_stage            TEXT,
            provenance              TEXT,
            source_plant            TEXT,
            initiation_date         TEXT    NOT NULL,
            location                TEXT,
            location_details        TEXT,
            propagation_method      TEXT,
            acclimatization_status  TEXT    CHECK(acclimatization_status IN (
                                        'not_applicable','in_vitro','hardening',
                                        'greenhouse','field','completed'
                                    )),
            health_status           TEXT    DEFAULT 'healthy',
            disease_status          TEXT,
            quarantine_flag         INTEGER NOT NULL DEFAULT 0,
            quarantine_release_date TEXT,
            permit_number           TEXT,
            permit_expiry           TEXT,
            ip_flag                 INTEGER NOT NULL DEFAULT 0,
            ip_notes                TEXT,
            environmental_notes     TEXT,
            subculture_count        INTEGER NOT NULL DEFAULT 0,
            parent_specimen_id      TEXT    REFERENCES specimens_v16(id),
            qr_code_data            TEXT,
            notes                   TEXT,
            is_archived             INTEGER NOT NULL DEFAULT 0,
            archived_at             TEXT,
            employee_id             TEXT,
            created_by              TEXT    REFERENCES users(id),
            created_at              TEXT    NOT NULL DEFAULT (datetime('now')),
            updated_at              TEXT    NOT NULL DEFAULT (datetime('now')),
            generation              INTEGER NOT NULL DEFAULT 0,
            lineage_passage_offset  INTEGER NOT NULL DEFAULT 0,
            root_specimen_id        TEXT    REFERENCES specimens_v16(id),
            contamination_flag      INTEGER NOT NULL DEFAULT 0,
            contamination_notes     TEXT
        );

        INSERT INTO specimens_v16 (
            id, accession_number, species_id, project_id, stage, custom_stage,
            provenance, source_plant, initiation_date, location, location_details,
            propagation_method, acclimatization_status, health_status, disease_status,
            quarantine_flag, quarantine_release_date, permit_number, permit_expiry,
            ip_flag, ip_notes, environmental_notes, subculture_count, parent_specimen_id,
            qr_code_data, notes, is_archived, archived_at, employee_id, created_by,
            created_at, updated_at, generation, lineage_passage_offset, root_specimen_id,
            contamination_flag, contamination_notes
        )
        SELECT
            id, accession_number, species_id, project_id, stage, custom_stage,
            provenance, source_plant, initiation_date, location, location_details,
            propagation_method, acclimatization_status, health_status, disease_status,
            quarantine_flag, quarantine_release_date, permit_number, permit_expiry,
            ip_flag, ip_notes, environmental_notes, subculture_count, parent_specimen_id,
            qr_code_data, notes, is_archived, archived_at, employee_id, created_by,
            created_at, updated_at,
            COALESCE(generation, 0),
            COALESCE(lineage_passage_offset, 0),
            root_specimen_id,
            COALESCE(contamination_flag, 0),
            contamination_notes
        FROM specimens;

        DROP TABLE specimens;
        ALTER TABLE specimens_v16 RENAME TO specimens;

        CREATE INDEX IF NOT EXISTS idx_specimens_accession
            ON specimens(accession_number);
        CREATE INDEX IF NOT EXISTS idx_specimens_species
            ON specimens(species_id);
        CREATE INDEX IF NOT EXISTS idx_specimens_project
            ON specimens(project_id);
        CREATE INDEX IF NOT EXISTS idx_specimens_stage
            ON specimens(stage);
        CREATE INDEX IF NOT EXISTS idx_specimens_quarantine
            ON specimens(quarantine_flag);
        CREATE INDEX IF NOT EXISTS idx_specimens_archived
            ON specimens(is_archived);
        CREATE INDEX IF NOT EXISTS idx_specimens_created_at
            ON specimens(created_at);
        CREATE INDEX IF NOT EXISTS idx_specimens_parent
            ON specimens(parent_specimen_id);
        CREATE INDEX IF NOT EXISTS idx_specimens_archived_created
            ON specimens(is_archived, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_specimens_root
            ON specimens(root_specimen_id);
        COMMIT;
    ");
    conn.execute("PRAGMA foreign_keys = ON", [])?;
    result?;

    Ok(())
}

fn migration_017_remaining_vocabularies(conn: &Connection) -> DbResult<()> {
    // WP-24: remaining vocabulary tables — hormone_types, compliance_record_types,
    // compliance_agencies, inventory_categories — all profile-scoped and seeded with
    // plant_tissue_culture values.  Then rebuilds media_hormones, compliance_records,
    // and inventory_items to drop their respective CHECK constraints in one pass.
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS hormone_types (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            profile    TEXT    NOT NULL,
            code       TEXT    NOT NULL,
            label      TEXT    NOT NULL,
            sort_order INTEGER NOT NULL DEFAULT 0,
            UNIQUE(profile, code)
        );

        CREATE INDEX IF NOT EXISTS idx_hormone_types_profile
            ON hormone_types(profile, sort_order);

        INSERT OR IGNORE INTO hormone_types (profile, code, label, sort_order) VALUES
            ('plant_tissue_culture', 'auxin',       'Auxin',       1),
            ('plant_tissue_culture', 'cytokinin',   'Cytokinin',   2),
            ('plant_tissue_culture', 'gibberellin', 'Gibberellin', 3),
            ('plant_tissue_culture', 'other',       'Other',       4);

        CREATE TABLE IF NOT EXISTS compliance_record_types (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            profile    TEXT    NOT NULL,
            code       TEXT    NOT NULL,
            label      TEXT    NOT NULL,
            sort_order INTEGER NOT NULL DEFAULT 0,
            UNIQUE(profile, code)
        );

        CREATE INDEX IF NOT EXISTS idx_compliance_record_types_profile
            ON compliance_record_types(profile, sort_order);

        INSERT OR IGNORE INTO compliance_record_types (profile, code, label, sort_order) VALUES
            ('plant_tissue_culture', 'disease_test',       'Disease Test',         1),
            ('plant_tissue_culture', 'permit',             'Permit',               2),
            ('plant_tissue_culture', 'phytosanitary_cert', 'Phytosanitary Cert.',  3),
            ('plant_tissue_culture', 'inspection',         'Inspection',           4),
            ('plant_tissue_culture', 'quarantine',         'Quarantine',           5),
            ('plant_tissue_culture', 'movement_permit',    'Movement Permit',      6),
            ('plant_tissue_culture', 'pest_risk',          'Pest Risk Assessment', 7),
            ('plant_tissue_culture', 'export_cert',        'Export Certificate',   8),
            ('plant_tissue_culture', 'other',              'Other',                9);

        CREATE TABLE IF NOT EXISTS compliance_agencies (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            profile    TEXT    NOT NULL,
            code       TEXT    NOT NULL,
            label      TEXT    NOT NULL,
            sort_order INTEGER NOT NULL DEFAULT 0,
            UNIQUE(profile, code)
        );

        CREATE INDEX IF NOT EXISTS idx_compliance_agencies_profile
            ON compliance_agencies(profile, sort_order);

        INSERT OR IGNORE INTO compliance_agencies (profile, code, label, sort_order) VALUES
            ('plant_tissue_culture', 'USDA_APHIS', 'USDA APHIS',               1),
            ('plant_tissue_culture', 'TX_AG',      'TX Dept. of Agriculture',  2),
            ('plant_tissue_culture', 'FL_FDACS',   'FL FDACS',                 3),
            ('plant_tissue_culture', 'other',      'Other',                    4);

        CREATE TABLE IF NOT EXISTS inventory_categories (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            profile    TEXT    NOT NULL,
            code       TEXT    NOT NULL,
            label      TEXT    NOT NULL,
            sort_order INTEGER NOT NULL DEFAULT 0,
            UNIQUE(profile, code)
        );

        CREATE INDEX IF NOT EXISTS idx_inventory_categories_profile
            ON inventory_categories(profile, sort_order);

        INSERT OR IGNORE INTO inventory_categories (profile, code, label, sort_order) VALUES
            ('plant_tissue_culture', 'media_ingredient', 'Media Ingredient', 1),
            ('plant_tissue_culture', 'vessel',           'Vessel',           2),
            ('plant_tissue_culture', 'hormone',          'Hormone',          3),
            ('plant_tissue_culture', 'chemical',         'Chemical',         4),
            ('plant_tissue_culture', 'consumable',       'Consumable',       5),
            ('plant_tissue_culture', 'equipment',        'Equipment',        6),
            ('plant_tissue_culture', 'other',            'Other',            7);
    ")?;

    // Rebuild three tables to drop their CHECK constraints in one PRAGMA OFF/ON window.
    conn.execute("PRAGMA foreign_keys = OFF", [])?;
    let result = conn.execute_batch("
        BEGIN;
        -- media_hormones: drop CHECK on hormone_type.
        -- amount_used and amount_unit (added by ALTER in migration_002) are included.
        CREATE TABLE IF NOT EXISTS media_hormones_v17 (
            id                     TEXT PRIMARY KEY,
            media_batch_id         TEXT NOT NULL REFERENCES media_batches(id) ON DELETE CASCADE,
            hormone_name           TEXT NOT NULL,
            hormone_type           TEXT,
            concentration_mg_per_l REAL NOT NULL,
            supplier               TEXT,
            lot_number             TEXT,
            reagent_batch_id       TEXT,
            amount_used            REAL,
            amount_unit            TEXT
        );

        INSERT INTO media_hormones_v17
        SELECT id, media_batch_id, hormone_name, hormone_type,
               concentration_mg_per_l, supplier, lot_number, reagent_batch_id,
               amount_used, amount_unit
        FROM media_hormones;

        DROP TABLE media_hormones;
        ALTER TABLE media_hormones_v17 RENAME TO media_hormones;

        -- compliance_records: drop CHECK on record_type and agency.
        -- test_result and status CHECKs are kept (operational, not vocabulary-driven).
        CREATE TABLE IF NOT EXISTS compliance_records_v17 (
            id               TEXT PRIMARY KEY,
            specimen_id      TEXT NOT NULL REFERENCES specimens(id) ON DELETE CASCADE,
            record_type      TEXT NOT NULL,
            agency           TEXT,
            permit_number    TEXT,
            permit_expiry    TEXT,
            test_type        TEXT,
            test_method      TEXT,
            test_date        TEXT,
            test_lab         TEXT,
            test_result      TEXT CHECK(test_result IN (
                                 'positive','negative','inconclusive','pending',NULL
                             )),
            status           TEXT NOT NULL DEFAULT 'valid' CHECK(status IN (
                                 'valid','expired','pending','flagged','revoked'
                             )),
            flag_reason      TEXT,
            chain_of_custody TEXT,
            notes            TEXT,
            document_path    TEXT,
            created_by       TEXT REFERENCES users(id),
            created_at       TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at       TEXT NOT NULL DEFAULT (datetime('now'))
        );

        INSERT INTO compliance_records_v17
        SELECT id, specimen_id, record_type, agency, permit_number, permit_expiry,
               test_type, test_method, test_date, test_lab, test_result, status,
               flag_reason, chain_of_custody, notes, document_path,
               created_by, created_at, updated_at
        FROM compliance_records;

        DROP TABLE compliance_records;
        ALTER TABLE compliance_records_v17 RENAME TO compliance_records;

        CREATE INDEX IF NOT EXISTS idx_compliance_specimen
            ON compliance_records(specimen_id);
        CREATE INDEX IF NOT EXISTS idx_compliance_type
            ON compliance_records(record_type);
        CREATE INDEX IF NOT EXISTS idx_compliance_status
            ON compliance_records(status);

        -- inventory_items: drop CHECK on category.
        -- physical_state, concentration, concentration_unit (added by ALTER in migration_002)
        -- are included in the rebuild.
        CREATE TABLE IF NOT EXISTS inventory_items_v17 (
            id                TEXT PRIMARY KEY,
            name              TEXT NOT NULL,
            category          TEXT NOT NULL,
            unit              TEXT NOT NULL,
            current_stock     REAL NOT NULL DEFAULT 0,
            minimum_stock     REAL NOT NULL DEFAULT 0,
            reorder_point     REAL,
            supplier          TEXT,
            catalog_number    TEXT,
            lot_number        TEXT,
            storage_location  TEXT,
            expiration_date   TEXT,
            cost_per_unit     REAL,
            notes             TEXT,
            created_at        TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at        TEXT NOT NULL DEFAULT (datetime('now')),
            physical_state    TEXT DEFAULT 'solid',
            concentration     REAL,
            concentration_unit TEXT
        );

        INSERT INTO inventory_items_v17
        SELECT id, name, category, unit, current_stock, minimum_stock, reorder_point,
               supplier, catalog_number, lot_number, storage_location, expiration_date,
               cost_per_unit, notes, created_at, updated_at,
               COALESCE(physical_state, 'solid'), concentration, concentration_unit
        FROM inventory_items;

        DROP TABLE inventory_items;
        ALTER TABLE inventory_items_v17 RENAME TO inventory_items;
        COMMIT;
    ");
    conn.execute("PRAGMA foreign_keys = ON", [])?;
    result?;

    Ok(())
}

fn migration_018_cell_culture_vocabulary(conn: &Connection) -> DbResult<()> {
    // WP-27: seed minimal vocabulary for the cell_culture profile.
    // All tables already exist from migrations 016/017; this is purely additive.
    // INSERT OR IGNORE keeps re-runs safe with no duplicates.
    conn.execute_batch("
        BEGIN;

        INSERT OR IGNORE INTO stages (profile, code, label, sort_order, is_terminal) VALUES
            ('cell_culture', 'primary',          'Primary Culture',  1,  0),
            ('cell_culture', 'subculture',       'Subculture',       2,  0),
            ('cell_culture', 'expansion',        'Expansion',        3,  0),
            ('cell_culture', 'maintenance',      'Maintenance',      4,  0),
            ('cell_culture', 'differentiation',  'Differentiation',  5,  0),
            ('cell_culture', 'characterization', 'Characterization', 6,  0),
            ('cell_culture', 'selection',        'Selection',        7,  0),
            ('cell_culture', 'stable_line',      'Stable Cell Line', 8,  0),
            ('cell_culture', 'cryo_stock',       'Cryo Stock',       9,  0),
            ('cell_culture', 'thaw_recovery',    'Thaw Recovery',    10, 0),
            ('cell_culture', 'archived',         'Archived',         11, 1),
            ('cell_culture', 'custom',           'Custom',           12, 0);

        INSERT OR IGNORE INTO propagation_methods (profile, code, label, sort_order) VALUES
            ('cell_culture', 'trypsin_passage',     'Trypsin Passage',     1),
            ('cell_culture', 'mechanical_passage',  'Mechanical Passage',  2),
            ('cell_culture', 'suspension_dilution', 'Suspension Dilution', 3),
            ('cell_culture', 'feeder_free',         'Feeder-Free',         4),
            ('cell_culture', 'feeder_dependent',    'Feeder-Dependent',    5),
            ('cell_culture', 'spin_out',            'Spin-out & Reseed',   6),
            ('cell_culture', 'other',               'Other',               7);

        INSERT OR IGNORE INTO hormone_types (profile, code, label, sort_order) VALUES
            ('cell_culture', 'growth_factor', 'Growth Factor', 1),
            ('cell_culture', 'cytokine',      'Cytokine',      2),
            ('cell_culture', 'steroid',       'Steroid',       3),
            ('cell_culture', 'other',         'Other',         4);

        INSERT OR IGNORE INTO compliance_record_types (profile, code, label, sort_order) VALUES
            ('cell_culture', 'mycoplasma_test',   'Mycoplasma Test',    1),
            ('cell_culture', 'sterility_test',    'Sterility Test',     2),
            ('cell_culture', 'identity_test',     'Identity Test',      3),
            ('cell_culture', 'bsl_review',        'BSL Review',         4),
            ('cell_culture', 'irb_approval',      'IRB Approval',       5),
            ('cell_culture', 'material_transfer', 'Material Transfer',  6),
            ('cell_culture', 'coa',               'Cert. of Analysis',  7),
            ('cell_culture', 'permit',            'Permit',             8),
            ('cell_culture', 'other',             'Other',              9);

        INSERT OR IGNORE INTO compliance_agencies (profile, code, label, sort_order) VALUES
            ('cell_culture', 'CDC_NIH',    'CDC / NIH',  1),
            ('cell_culture', 'FDA_CBER',   'FDA CBER',   2),
            ('cell_culture', 'USDA_APHIS', 'USDA APHIS', 3),
            ('cell_culture', 'other',      'Other',      4);

        INSERT OR IGNORE INTO inventory_categories (profile, code, label, sort_order) VALUES
            ('cell_culture', 'media',          'Cell Culture Media', 1),
            ('cell_culture', 'serum',          'Serum / Serum-Free', 2),
            ('cell_culture', 'enzyme',         'Enzyme',             3),
            ('cell_culture', 'supplement',     'Growth Supplement',  4),
            ('cell_culture', 'vessel',         'Vessel',             5),
            ('cell_culture', 'cryoprotectant', 'Cryoprotectant',     6),
            ('cell_culture', 'other',          'Other',              7);

        COMMIT;
    ")?;
    Ok(())
}

fn migration_015_death_events_and_lab_profile(conn: &Connection) -> DbResult<()> {
    // WP-22: two additive additions.
    //
    // 1. event_type on subcultures distinguishes normal passages ('passage') from terminal
    //    death recordings ('death').  Death events do not increment subculture_count on the
    //    specimen; they archive it instead.  DEFAULT 'passage' keeps all existing rows valid.
    //
    // 2. app_config is a guaranteed-single-row table (id = 1 enforced by CHECK) that holds
    //    app-level settings that differ from app_settings (which is key/value).  Starting
    //    with lab_profile — the discipline this installation is configured for.
    //    Allowed: 'plant_tissue_culture' | 'cell_culture' | 'mycology'.
    conn.execute_batch("
        ALTER TABLE subcultures ADD COLUMN event_type TEXT NOT NULL DEFAULT 'passage';
        CREATE INDEX IF NOT EXISTS idx_subcultures_event_type ON subcultures(event_type);

        CREATE TABLE IF NOT EXISTS app_config (
            id          INTEGER PRIMARY KEY CHECK (id = 1),
            lab_profile TEXT NOT NULL DEFAULT 'plant_tissue_culture'
                        CHECK (lab_profile IN ('plant_tissue_culture','cell_culture','mycology')),
            updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
        );
        INSERT OR IGNORE INTO app_config (id, lab_profile) VALUES (1, 'plant_tissue_culture');
    ")?;
    Ok(())
}

fn migration_014_checkpoint_auto_and_settings(conn: &Connection) -> DbResult<()> {
    // WP-21: add auto-checkpoint provenance columns and a minimal app_settings table.
    //
    // is_auto = 1 for checkpoints created automatically (pre-backup or entry-count trigger).
    // auto_source: "backup" | "entry_count" | NULL (manual).
    //
    // app_settings holds simple key/value pairs for app-level configuration; starting
    // with the auto-checkpoint knobs only. Keys are inserted with sensible defaults and
    // only written when the user explicitly changes them via set_auto_checkpoint_config.
    conn.execute_batch("
        ALTER TABLE audit_checkpoints ADD COLUMN is_auto     INTEGER NOT NULL DEFAULT 0;
        ALTER TABLE audit_checkpoints ADD COLUMN auto_source TEXT;

        CREATE TABLE IF NOT EXISTS app_settings (
            key        TEXT PRIMARY KEY,
            value      TEXT NOT NULL,
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        INSERT OR IGNORE INTO app_settings (key, value) VALUES
            ('auto_checkpoint_enabled',  '1'),
            ('auto_checkpoint_interval', '100'),
            ('auto_checkpoint_on_backup','1');
    ")?;
    Ok(())
}

fn migration_013_audit_checkpoints(conn: &Connection) -> DbResult<()> {
    // Merkle checkpoint table (WP-20). Each row seals a range of a lineage's audit chain
    // into a single Merkle root so the sealed history can be re-verified at any time.
    //
    // anchored_txid is the Phase-2 hook (WP-65+): once on-chain anchoring is added,
    // the published Dogecoin txid for this root will be stored here. Left NULL for now.
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS audit_checkpoints (
            id            TEXT PRIMARY KEY,
            lineage_id    TEXT NOT NULL,
            start_seq     INTEGER NOT NULL,
            end_seq       INTEGER NOT NULL,
            entry_count   INTEGER NOT NULL,
            merkle_root   TEXT NOT NULL,
            created_at    TEXT NOT NULL DEFAULT (datetime('now')),
            created_by    TEXT REFERENCES users(id),
            anchored_txid TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_audit_checkpoints_lineage
            ON audit_checkpoints(lineage_id);
        CREATE INDEX IF NOT EXISTS idx_audit_checkpoints_created
            ON audit_checkpoints(created_at);
    ")?;
    Ok(())
}

fn migration_012_specimen_contamination(conn: &Connection) -> DbResult<()> {
    // Add structured contamination columns to specimens.
    // These capture the final contamination state of a specimen when it is archived
    // (typically via a split), keeping the data separate from free-text notes.
    conn.execute_batch("
        ALTER TABLE specimens ADD COLUMN contamination_flag  INTEGER NOT NULL DEFAULT 0;
        ALTER TABLE specimens ADD COLUMN contamination_notes TEXT;
    ")?;
    Ok(())
}

fn migration_011_media_draft(conn: &Connection) -> DbResult<()> {
    // Add is_draft flag to media_batches.  Draft batches are created as
    // lightweight placeholders during a split when the full formulation is
    // not yet known; they are marked needs_review=1 and completed later in
    // the Media Management screen.
    conn.execute_batch("
        ALTER TABLE media_batches ADD COLUMN is_draft INTEGER NOT NULL DEFAULT 0;
        CREATE INDEX IF NOT EXISTS idx_media_batches_draft ON media_batches(is_draft);
    ")?;
    Ok(())
}

fn migration_010_specimen_genealogy(conn: &Connection) -> DbResult<()> {
    // Track generational depth and cumulative passage history for split lineages.
    //
    // generation:             0 for root specimens, increments by 1 on each split.
    // lineage_passage_offset: total subculture passages accumulated across ALL ancestor
    //                         specimens at the moment this specimen was created.
    //                         Enables "passages from root" = offset + subculture_count.
    // root_specimen_id:       NULL for root specimens; ID of the absolute root ancestor
    //                         for all derived specimens. Enables efficient family queries.
    conn.execute_batch("
        ALTER TABLE specimens ADD COLUMN generation               INTEGER NOT NULL DEFAULT 0;
        ALTER TABLE specimens ADD COLUMN lineage_passage_offset   INTEGER NOT NULL DEFAULT 0;
        ALTER TABLE specimens ADD COLUMN root_specimen_id         TEXT REFERENCES specimens(id);

        CREATE INDEX IF NOT EXISTS idx_specimens_root ON specimens(root_specimen_id);
    ")?;
    Ok(())
}

fn migration_009_audit_lineage(conn: &Connection) -> DbResult<()> {
    // Switch the hash chain from a single global sequence to per-lineage sequences.
    //
    // lineage_id groups all audit entries that form a single verifiable chain.
    // For entity events it equals entity_id; for system events it is "system".
    // When a specimen is split, the child starts a new lineage (lineage_id = child id)
    // but inherits prev_hash from the parent's last entry, creating a visible fork.
    //
    // chain_seq is now per-lineage, not global.
    // The (lineage_id, chain_seq) pair uniquely identifies a position in any lineage.
    //
    // Rows from migration_008 (global chain) are considered legacy: they keep their
    // chain_seq / prev_hash / entry_hash values but lineage_id is back-filled from
    // entity_id so verification can still be attempted per-entity.
    conn.execute_batch("
        ALTER TABLE audit_log ADD COLUMN lineage_id TEXT;

        -- Back-fill: assign lineage_id for any rows that already have chain data.
        UPDATE audit_log
           SET lineage_id = COALESCE(entity_id, 'system')
         WHERE chain_seq IS NOT NULL;

        -- Composite index used by chain-head lookups and verification queries.
        CREATE INDEX IF NOT EXISTS idx_audit_lineage ON audit_log(lineage_id, chain_seq);
    ")?;
    Ok(())
}

fn migration_008_audit_hash_chain(conn: &Connection) -> DbResult<()> {
    // Add three columns that make the audit_log a tamper-evident hash chain.
    // Existing rows keep NULL in all three — only rows inserted after this
    // migration carry chain values.
    conn.execute_batch("
        ALTER TABLE audit_log ADD COLUMN chain_seq  INTEGER;
        ALTER TABLE audit_log ADD COLUMN prev_hash  TEXT;
        ALTER TABLE audit_log ADD COLUMN entry_hash TEXT;

        CREATE INDEX IF NOT EXISTS idx_audit_chain_seq ON audit_log(chain_seq);
    ")?;
    Ok(())
}

fn migration_007_perf_indexes(conn: &Connection) -> DbResult<()> {
    conn.execute_batch("
        -- specimens: ORDER BY created_at DESC (list/search views)
        CREATE INDEX IF NOT EXISTS idx_specimens_created_at ON specimens(created_at);

        -- specimens: parent_specimen_id used in lineage lookups
        CREATE INDEX IF NOT EXISTS idx_specimens_parent ON specimens(parent_specimen_id);

        -- specimens: composite covering common list filter + sort
        CREATE INDEX IF NOT EXISTS idx_specimens_archived_created ON specimens(is_archived, created_at DESC);

        -- subcultures: composite for per-specimen history queries (specimen_id + ORDER BY passage_number)
        CREATE INDEX IF NOT EXISTS idx_subcultures_specimen_passage ON subcultures(specimen_id, passage_number);

        -- subcultures: date used in schedule and recent-subculture stats
        CREATE INDEX IF NOT EXISTS idx_subcultures_created_at ON subcultures(created_at);

        -- subcultures: composite for contamination stats join
        CREATE INDEX IF NOT EXISTS idx_subcultures_contamination_specimen ON subcultures(contamination_flag, specimen_id);
    ")?;
    Ok(())
}

fn migration_006_force_password_change(conn: &Connection) -> DbResult<()> {
    conn.execute_batch("
        ALTER TABLE users ADD COLUMN must_change_password INTEGER NOT NULL DEFAULT 0;
        UPDATE users SET must_change_password = 1 WHERE username = 'admin';
    ")?;
    Ok(())
}

fn migration_005_contamination_schedule(conn: &Connection) -> DbResult<()> {
    conn.execute_batch("
        ALTER TABLE subcultures ADD COLUMN contamination_flag INTEGER NOT NULL DEFAULT 0;
        ALTER TABLE subcultures ADD COLUMN contamination_notes TEXT;
        CREATE INDEX IF NOT EXISTS idx_subcultures_contamination ON subcultures(contamination_flag);
    ")?;
    Ok(())
}

fn migration_004_v0114(conn: &Connection) -> DbResult<()> {
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS qr_scans (
            id TEXT PRIMARY KEY,
            raw_data TEXT NOT NULL,
            accession_number TEXT,
            scanned_by TEXT REFERENCES users(id),
            scanned_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_qr_scans_accession ON qr_scans(accession_number);
        CREATE INDEX IF NOT EXISTS idx_qr_scans_at ON qr_scans(scanned_at);
    ")?;
    Ok(())
}

fn migration_001_initial(conn: &Connection) -> DbResult<()> {
    conn.execute_batch("
        -- Users and authentication
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            username TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            display_name TEXT NOT NULL,
            email TEXT,
            role TEXT NOT NULL DEFAULT 'tech' CHECK(role IN ('admin','supervisor','tech','guest')),
            is_active INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL REFERENCES users(id),
            token TEXT NOT NULL UNIQUE,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            expires_at TEXT NOT NULL
        );

        -- Species master table
        CREATE TABLE IF NOT EXISTS species (
            id TEXT PRIMARY KEY,
            genus TEXT NOT NULL,
            species_name TEXT NOT NULL,
            common_name TEXT,
            species_code TEXT NOT NULL UNIQUE,
            default_subculture_interval_days INTEGER DEFAULT 28,
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        -- Projects
        CREATE TABLE IF NOT EXISTS projects (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            lead_user_id TEXT REFERENCES users(id),
            status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active','paused','completed','archived')),
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        -- Specimens
        CREATE TABLE IF NOT EXISTS specimens (
            id TEXT PRIMARY KEY,
            accession_number TEXT NOT NULL UNIQUE,
            species_id TEXT NOT NULL REFERENCES species(id),
            project_id TEXT REFERENCES projects(id),
            stage TEXT NOT NULL DEFAULT 'explant' CHECK(stage IN (
                'explant','callus','suspension','protoplast','shoot','root',
                'embryogenic','plantlet','acclimatized','stock','archived','custom'
            )),
            custom_stage TEXT,
            provenance TEXT,
            source_plant TEXT,
            initiation_date TEXT NOT NULL,
            location TEXT,
            location_details TEXT,
            propagation_method TEXT CHECK(propagation_method IN (
                'microprop','somatic_embryogenesis','organogenesis',
                'meristem_culture','anther_culture','protoplast_fusion','other'
            )),
            acclimatization_status TEXT CHECK(acclimatization_status IN (
                'not_applicable','in_vitro','hardening','greenhouse','field','completed'
            )),
            health_status TEXT DEFAULT 'healthy',
            disease_status TEXT,
            quarantine_flag INTEGER NOT NULL DEFAULT 0,
            quarantine_release_date TEXT,
            permit_number TEXT,
            permit_expiry TEXT,
            ip_flag INTEGER NOT NULL DEFAULT 0,
            ip_notes TEXT,
            environmental_notes TEXT,
            subculture_count INTEGER NOT NULL DEFAULT 0,
            parent_specimen_id TEXT REFERENCES specimens(id),
            qr_code_data TEXT,
            notes TEXT,
            is_archived INTEGER NOT NULL DEFAULT 0,
            archived_at TEXT,
            created_by TEXT REFERENCES users(id),
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE INDEX IF NOT EXISTS idx_specimens_accession ON specimens(accession_number);
        CREATE INDEX IF NOT EXISTS idx_specimens_species ON specimens(species_id);
        CREATE INDEX IF NOT EXISTS idx_specimens_project ON specimens(project_id);
        CREATE INDEX IF NOT EXISTS idx_specimens_stage ON specimens(stage);
        CREATE INDEX IF NOT EXISTS idx_specimens_quarantine ON specimens(quarantine_flag);
        CREATE INDEX IF NOT EXISTS idx_specimens_archived ON specimens(is_archived);

        -- Specimen tags (hierarchical)
        CREATE TABLE IF NOT EXISTS tags (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            category TEXT NOT NULL,
            parent_tag_id TEXT REFERENCES tags(id),
            color TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS specimen_tags (
            specimen_id TEXT NOT NULL REFERENCES specimens(id) ON DELETE CASCADE,
            tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
            value TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            PRIMARY KEY (specimen_id, tag_id)
        );

        -- Media batches
        CREATE TABLE IF NOT EXISTS media_batches (
            id TEXT PRIMARY KEY,
            batch_id TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL,
            preparation_date TEXT NOT NULL,
            expiration_date TEXT,
            basal_salts TEXT DEFAULT 'MS',
            basal_salts_concentration REAL DEFAULT 1.0,
            vitamins TEXT,
            sucrose_g_per_l REAL,
            agar_g_per_l REAL,
            gelling_agent TEXT,
            ph_before_autoclave REAL,
            ph_after_autoclave REAL,
            sterilization_method TEXT DEFAULT 'autoclave',
            volume_prepared_ml REAL,
            volume_used_ml REAL DEFAULT 0,
            volume_remaining_ml REAL,
            storage_conditions TEXT,
            qc_notes TEXT,
            supplier_info TEXT,
            cost_per_batch REAL,
            osmolarity REAL,
            conductivity REAL,
            is_custom INTEGER NOT NULL DEFAULT 0,
            needs_review INTEGER NOT NULL DEFAULT 0,
            notes TEXT,
            created_by TEXT REFERENCES users(id),
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        -- Media hormones (many-to-many)
        CREATE TABLE IF NOT EXISTS media_hormones (
            id TEXT PRIMARY KEY,
            media_batch_id TEXT NOT NULL REFERENCES media_batches(id) ON DELETE CASCADE,
            hormone_name TEXT NOT NULL,
            hormone_type TEXT CHECK(hormone_type IN ('auxin','cytokinin','gibberellin','other')),
            concentration_mg_per_l REAL NOT NULL,
            supplier TEXT,
            lot_number TEXT,
            reagent_batch_id TEXT
        );

        -- Subculture history
        CREATE TABLE IF NOT EXISTS subcultures (
            id TEXT PRIMARY KEY,
            specimen_id TEXT NOT NULL REFERENCES specimens(id) ON DELETE CASCADE,
            passage_number INTEGER NOT NULL,
            date TEXT NOT NULL,
            media_batch_id TEXT REFERENCES media_batches(id),
            ph REAL,
            temperature_c REAL,
            light_cycle TEXT,
            light_intensity_lux REAL,
            experimental_treatment TEXT,
            vessel_type TEXT,
            vessel_size TEXT,
            vessel_material TEXT,
            vessel_lid_type TEXT,
            location_from TEXT,
            location_to TEXT,
            temp_before REAL,
            temp_after REAL,
            humidity_before REAL,
            humidity_after REAL,
            light_before TEXT,
            light_after TEXT,
            exposure_duration_hours REAL,
            notes TEXT,
            observations TEXT,
            performed_by TEXT REFERENCES users(id),
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE INDEX IF NOT EXISTS idx_subcultures_specimen ON subcultures(specimen_id);
        CREATE INDEX IF NOT EXISTS idx_subcultures_date ON subcultures(date);

        -- Attachments
        CREATE TABLE IF NOT EXISTS attachments (
            id TEXT PRIMARY KEY,
            entity_type TEXT NOT NULL CHECK(entity_type IN ('specimen','subculture','media_batch','compliance')),
            entity_id TEXT NOT NULL,
            file_name TEXT NOT NULL,
            file_path TEXT NOT NULL,
            file_size_bytes INTEGER,
            mime_type TEXT,
            description TEXT,
            uploaded_by TEXT REFERENCES users(id),
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE INDEX IF NOT EXISTS idx_attachments_entity ON attachments(entity_type, entity_id);

        -- Reminders
        CREATE TABLE IF NOT EXISTS reminders (
            id TEXT PRIMARY KEY,
            specimen_id TEXT REFERENCES specimens(id) ON DELETE CASCADE,
            title TEXT NOT NULL,
            description TEXT,
            reminder_type TEXT NOT NULL CHECK(reminder_type IN (
                'subculture_due','media_expiry','disease_test','permit_expiry',
                'quarantine_review','custom'
            )),
            due_date TEXT NOT NULL,
            is_recurring INTEGER NOT NULL DEFAULT 0,
            recurrence_days INTEGER,
            recurrence_rule TEXT,
            status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active','snoozed','dismissed','completed')),
            snooze_count INTEGER NOT NULL DEFAULT 0,
            urgency TEXT NOT NULL DEFAULT 'normal' CHECK(urgency IN ('low','normal','high','critical')),
            assigned_to TEXT REFERENCES users(id),
            created_by TEXT REFERENCES users(id),
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE INDEX IF NOT EXISTS idx_reminders_due ON reminders(due_date);
        CREATE INDEX IF NOT EXISTS idx_reminders_status ON reminders(status);
        CREATE INDEX IF NOT EXISTS idx_reminders_specimen ON reminders(specimen_id);

        -- Compliance records
        CREATE TABLE IF NOT EXISTS compliance_records (
            id TEXT PRIMARY KEY,
            specimen_id TEXT NOT NULL REFERENCES specimens(id) ON DELETE CASCADE,
            record_type TEXT NOT NULL CHECK(record_type IN (
                'disease_test','permit','phytosanitary_cert','inspection',
                'quarantine','movement_permit','pest_risk','export_cert','other'
            )),
            agency TEXT CHECK(agency IN ('USDA_APHIS','TX_AG','FL_FDACS','other')),
            permit_number TEXT,
            permit_expiry TEXT,
            test_type TEXT,
            test_method TEXT,
            test_date TEXT,
            test_lab TEXT,
            test_result TEXT CHECK(test_result IN ('positive','negative','inconclusive','pending', NULL)),
            status TEXT NOT NULL DEFAULT 'valid' CHECK(status IN ('valid','expired','pending','flagged','revoked')),
            flag_reason TEXT,
            chain_of_custody TEXT,
            notes TEXT,
            document_path TEXT,
            created_by TEXT REFERENCES users(id),
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE INDEX IF NOT EXISTS idx_compliance_specimen ON compliance_records(specimen_id);
        CREATE INDEX IF NOT EXISTS idx_compliance_type ON compliance_records(record_type);
        CREATE INDEX IF NOT EXISTS idx_compliance_status ON compliance_records(status);

        -- Inventory (basic supplies tracking)
        CREATE TABLE IF NOT EXISTS inventory_items (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            category TEXT NOT NULL CHECK(category IN (
                'media_ingredient','vessel','hormone','chemical','consumable','equipment','other'
            )),
            unit TEXT NOT NULL,
            current_stock REAL NOT NULL DEFAULT 0,
            minimum_stock REAL NOT NULL DEFAULT 0,
            reorder_point REAL,
            supplier TEXT,
            catalog_number TEXT,
            lot_number TEXT,
            storage_location TEXT,
            expiration_date TEXT,
            cost_per_unit REAL,
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        -- Audit log
        CREATE TABLE IF NOT EXISTS audit_log (
            id TEXT PRIMARY KEY,
            user_id TEXT REFERENCES users(id),
            action TEXT NOT NULL,
            entity_type TEXT NOT NULL,
            entity_id TEXT,
            old_value TEXT,
            new_value TEXT,
            ip_address TEXT,
            details TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE INDEX IF NOT EXISTS idx_audit_user ON audit_log(user_id);
        CREATE INDEX IF NOT EXISTS idx_audit_entity ON audit_log(entity_type, entity_id);
        CREATE INDEX IF NOT EXISTS idx_audit_created ON audit_log(created_at);
    ")?;

    Ok(())
}

fn migration_002_v019(conn: &Connection) -> DbResult<()> {
    // Step 1: Disable FK enforcement for table recreation
    conn.execute("PRAGMA foreign_keys = OFF", [])?;

    // Step 2: Recreate specimens table with expanded stage constraint + employee_id
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS specimens_v2 (
            id TEXT PRIMARY KEY,
            accession_number TEXT NOT NULL UNIQUE,
            species_id TEXT NOT NULL REFERENCES species(id),
            project_id TEXT REFERENCES projects(id),
            stage TEXT NOT NULL DEFAULT 'explant' CHECK(stage IN (
                'explant','callus','suspension','protoplast',
                'shoot','shoot_meristem','apical_meristem',
                'root','root_meristem',
                'embryogenic','plantlet','acclimatized','stock','archived','custom'
            )),
            custom_stage TEXT,
            provenance TEXT,
            source_plant TEXT,
            initiation_date TEXT NOT NULL,
            location TEXT,
            location_details TEXT,
            propagation_method TEXT CHECK(propagation_method IN (
                'microprop','somatic_embryogenesis','organogenesis',
                'meristem_culture','anther_culture','protoplast_fusion','other'
            )),
            acclimatization_status TEXT CHECK(acclimatization_status IN (
                'not_applicable','in_vitro','hardening','greenhouse','field','completed'
            )),
            health_status TEXT DEFAULT 'healthy',
            disease_status TEXT,
            quarantine_flag INTEGER NOT NULL DEFAULT 0,
            quarantine_release_date TEXT,
            permit_number TEXT,
            permit_expiry TEXT,
            ip_flag INTEGER NOT NULL DEFAULT 0,
            ip_notes TEXT,
            environmental_notes TEXT,
            subculture_count INTEGER NOT NULL DEFAULT 0,
            parent_specimen_id TEXT REFERENCES specimens_v2(id),
            qr_code_data TEXT,
            notes TEXT,
            is_archived INTEGER NOT NULL DEFAULT 0,
            archived_at TEXT,
            employee_id TEXT,
            created_by TEXT REFERENCES users(id),
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        INSERT INTO specimens_v2 (
            id, accession_number, species_id, project_id, stage, custom_stage,
            provenance, source_plant, initiation_date, location, location_details,
            propagation_method, acclimatization_status, health_status, disease_status,
            quarantine_flag, quarantine_release_date, permit_number, permit_expiry,
            ip_flag, ip_notes, environmental_notes, subculture_count, parent_specimen_id,
            qr_code_data, notes, is_archived, archived_at, created_by, created_at, updated_at
        )
        SELECT
            id, accession_number, species_id, project_id,
            CASE WHEN stage NOT IN (
                'explant','callus','suspension','protoplast',
                'shoot','shoot_meristem','apical_meristem',
                'root','root_meristem',
                'embryogenic','plantlet','acclimatized','stock','archived','custom'
            ) THEN 'custom' ELSE stage END,
            custom_stage, provenance, source_plant, initiation_date, location, location_details,
            propagation_method, acclimatization_status, health_status, disease_status,
            quarantine_flag, quarantine_release_date, permit_number, permit_expiry,
            ip_flag, ip_notes, environmental_notes, subculture_count, parent_specimen_id,
            qr_code_data, notes, is_archived, archived_at, created_by, created_at, updated_at
        FROM specimens;

        DROP TABLE specimens;
        ALTER TABLE specimens_v2 RENAME TO specimens;

        CREATE INDEX IF NOT EXISTS idx_specimens_accession ON specimens(accession_number);
        CREATE INDEX IF NOT EXISTS idx_specimens_species ON specimens(species_id);
        CREATE INDEX IF NOT EXISTS idx_specimens_project ON specimens(project_id);
        CREATE INDEX IF NOT EXISTS idx_specimens_stage ON specimens(stage);
        CREATE INDEX IF NOT EXISTS idx_specimens_quarantine ON specimens(quarantine_flag);
        CREATE INDEX IF NOT EXISTS idx_specimens_archived ON specimens(is_archived);
    ")?;

    // Step 3: Re-enable FK enforcement
    conn.execute("PRAGMA foreign_keys = ON", [])?;

    // Step 4: Add new columns via ALTER TABLE (safe, non-destructive)
    conn.execute_batch("
        ALTER TABLE inventory_items ADD COLUMN physical_state TEXT DEFAULT 'solid';
        ALTER TABLE inventory_items ADD COLUMN concentration REAL;
        ALTER TABLE inventory_items ADD COLUMN concentration_unit TEXT;

        ALTER TABLE media_batches ADD COLUMN employee_id TEXT;

        ALTER TABLE subcultures ADD COLUMN employee_id TEXT;
        ALTER TABLE subcultures ADD COLUMN health_status TEXT;

        ALTER TABLE media_hormones ADD COLUMN amount_used REAL;
        ALTER TABLE media_hormones ADD COLUMN amount_unit TEXT;

        CREATE TABLE IF NOT EXISTS prepared_solutions (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            source_item_id TEXT REFERENCES inventory_items(id),
            source_item_name TEXT,
            concentration REAL NOT NULL,
            concentration_unit TEXT NOT NULL,
            solvent TEXT,
            volume_ml REAL NOT NULL,
            volume_remaining_ml REAL NOT NULL,
            prepared_by TEXT,
            preparation_date TEXT NOT NULL,
            expiration_date TEXT,
            storage_conditions TEXT,
            lot_number TEXT,
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
    ")?;

    Ok(())
}

fn migration_003_v0110(conn: &Connection) -> DbResult<()> {
    // Step 1: Check if the specimens table has the expanded stage constraint.
    // Some users may have migration_002 fail silently (e.g., PRAGMA FK interaction),
    // leaving the old v1 CHECK constraint that rejects 'shoot_meristem' etc.
    let schema_sql: String = conn.query_row(
        "SELECT COALESCE(sql, '') FROM sqlite_master WHERE type='table' AND name='specimens'",
        [],
        |row| row.get(0),
    ).unwrap_or_default();

    if !schema_sql.contains("shoot_meristem") {
        // The specimens table still has the old constraint — rebuild it.
        conn.execute("PRAGMA foreign_keys = OFF", [])?;
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS specimens_v3 (
                id TEXT PRIMARY KEY,
                accession_number TEXT NOT NULL UNIQUE,
                species_id TEXT NOT NULL REFERENCES species(id),
                project_id TEXT REFERENCES projects(id),
                stage TEXT NOT NULL DEFAULT 'explant' CHECK(stage IN (
                    'explant','callus','suspension','protoplast',
                    'shoot','shoot_meristem','apical_meristem',
                    'root','root_meristem',
                    'embryogenic','plantlet','acclimatized','stock','archived','custom'
                )),
                custom_stage TEXT,
                provenance TEXT,
                source_plant TEXT,
                initiation_date TEXT NOT NULL,
                location TEXT,
                location_details TEXT,
                propagation_method TEXT CHECK(propagation_method IN (
                    'microprop','somatic_embryogenesis','organogenesis',
                    'meristem_culture','anther_culture','protoplast_fusion','other'
                )),
                acclimatization_status TEXT CHECK(acclimatization_status IN (
                    'not_applicable','in_vitro','hardening','greenhouse','field','completed'
                )),
                health_status TEXT DEFAULT 'healthy',
                disease_status TEXT,
                quarantine_flag INTEGER NOT NULL DEFAULT 0,
                quarantine_release_date TEXT,
                permit_number TEXT,
                permit_expiry TEXT,
                ip_flag INTEGER NOT NULL DEFAULT 0,
                ip_notes TEXT,
                environmental_notes TEXT,
                subculture_count INTEGER NOT NULL DEFAULT 0,
                parent_specimen_id TEXT REFERENCES specimens_v3(id),
                qr_code_data TEXT,
                notes TEXT,
                is_archived INTEGER NOT NULL DEFAULT 0,
                archived_at TEXT,
                employee_id TEXT,
                created_by TEXT REFERENCES users(id),
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            INSERT INTO specimens_v3 (
                id, accession_number, species_id, project_id, stage, custom_stage,
                provenance, source_plant, initiation_date, location, location_details,
                propagation_method, acclimatization_status, health_status, disease_status,
                quarantine_flag, quarantine_release_date, permit_number, permit_expiry,
                ip_flag, ip_notes, environmental_notes, subculture_count, parent_specimen_id,
                qr_code_data, notes, is_archived, archived_at, employee_id, created_by,
                created_at, updated_at
            )
            SELECT
                id, accession_number, species_id, project_id,
                CASE WHEN stage NOT IN (
                    'explant','callus','suspension','protoplast',
                    'shoot','shoot_meristem','apical_meristem',
                    'root','root_meristem',
                    'embryogenic','plantlet','acclimatized','stock','archived','custom'
                ) THEN 'custom' ELSE stage END,
                custom_stage, provenance, source_plant, initiation_date, location,
                location_details, propagation_method, acclimatization_status, health_status,
                disease_status, quarantine_flag, quarantine_release_date, permit_number,
                permit_expiry, ip_flag, ip_notes, environmental_notes, subculture_count,
                parent_specimen_id, qr_code_data, notes, is_archived, archived_at,
                CASE WHEN typeof(employee_id) = 'text' THEN employee_id ELSE NULL END,
                created_by, created_at, updated_at
            FROM specimens;

            DROP TABLE specimens;
            ALTER TABLE specimens_v3 RENAME TO specimens;

            CREATE INDEX IF NOT EXISTS idx_specimens_accession ON specimens(accession_number);
            CREATE INDEX IF NOT EXISTS idx_specimens_species ON specimens(species_id);
            CREATE INDEX IF NOT EXISTS idx_specimens_project ON specimens(project_id);
            CREATE INDEX IF NOT EXISTS idx_specimens_stage ON specimens(stage);
            CREATE INDEX IF NOT EXISTS idx_specimens_quarantine ON specimens(quarantine_flag);
            CREATE INDEX IF NOT EXISTS idx_specimens_archived ON specimens(is_archived);
        ")?;
        conn.execute("PRAGMA foreign_keys = ON", [])?;
    }

    // Step 2: Create the error_logs table (always safe with IF NOT EXISTS).
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS error_logs (
            id TEXT PRIMARY KEY,
            timestamp TEXT NOT NULL DEFAULT (datetime('now')),
            title TEXT NOT NULL,
            message TEXT NOT NULL,
            module TEXT,
            severity TEXT NOT NULL DEFAULT 'error' CHECK(severity IN ('info','warning','error','critical')),
            user_id TEXT REFERENCES users(id),
            username TEXT,
            form_payload TEXT,
            stack_trace TEXT,
            is_read INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE INDEX IF NOT EXISTS idx_error_logs_timestamp ON error_logs(timestamp);
        CREATE INDEX IF NOT EXISTS idx_error_logs_severity ON error_logs(severity);
        CREATE INDEX IF NOT EXISTS idx_error_logs_module ON error_logs(module);
        CREATE INDEX IF NOT EXISTS idx_error_logs_is_read ON error_logs(is_read);
    ")?;

    Ok(())
}

pub fn seed_defaults(conn: &Connection) -> DbResult<()> {
    // Check if already seeded
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))
        .unwrap_or(0);

    if count > 0 {
        return Ok(());
    }

    let admin_id = uuid::Uuid::new_v4().to_string();
    let password_hash = bcrypt::hash("admin", bcrypt::DEFAULT_COST).unwrap();

    conn.execute(
        "INSERT INTO users (id, username, password_hash, display_name, email, role, must_change_password)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1)",
        rusqlite::params![admin_id, "admin", password_hash, "Administrator", "admin@stelolab.local", "admin"],
    )?;

    // Seed default species
    let species_data = vec![
        ("Asparagus", "officinalis", "Asparagus", "ASP-OFF", 28),
        ("Nandina", "domestica", "Heavenly Bamboo", "NAN-DOM", 35),
        ("Citrus", "sinensis", "Sweet Orange", "CIT-SIN", 42),
        ("Citrus", "limon", "Lemon", "CIT-LIM", 42),
        ("Citrus", "paradisi", "Grapefruit", "CIT-PAR", 42),
        ("Citrus", "reticulata", "Mandarin", "CIT-RET", 42),
    ];

    for (genus, species, common, code, interval) in species_data {
        let id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT OR IGNORE INTO species (id, genus, species_name, common_name, species_code, default_subculture_interval_days)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![id, genus, species, common, code, interval],
        )?;
    }

    // Seed default tags
    let tag_categories = vec![
        ("Health", vec![
            ("Vigor 1 - Poor", None),
            ("Vigor 2 - Fair", None),
            ("Vigor 3 - Good", None),
            ("Vigor 4 - Very Good", None),
            ("Vigor 5 - Excellent", None),
            ("Green", Some("#22c55e")),
            ("Yellow", Some("#eab308")),
            ("Brown", Some("#92400e")),
            ("Orange", Some("#f97316")),
            ("Purple", Some("#a855f7")),
            ("Black", Some("#1c1917")),
            ("Necrosis", Some("#dc2626")),
        ]),
        ("Disease", vec![
            ("Bacterial", Some("#ef4444")),
            ("Fungal", Some("#f59e0b")),
            ("Viral", Some("#8b5cf6")),
            ("Viroid", Some("#ec4899")),
            ("Unknown Pathogen", Some("#6b7280")),
        ]),
        ("Growth", vec![
            ("Callus Formation", Some("#84cc16")),
            ("Shoot Formation", Some("#22d3ee")),
            ("Root Formation", Some("#a78bfa")),
            ("Embryogenic", Some("#fb923c")),
        ]),
        ("Issue", vec![
            ("Contamination", Some("#dc2626")),
            ("Hyperhydricity", Some("#3b82f6")),
            ("Browning", Some("#92400e")),
        ]),
        ("Contamination Type", vec![
            ("Bacterial Contam.", Some("#ef4444")),
            ("Fungal Contam.", Some("#f59e0b")),
            ("Yeast Contam.", Some("#fbbf24")),
            ("Endogenous Contam.", Some("#d946ef")),
        ]),
        ("Action Needed", vec![
            ("Subculture Due", Some("#3b82f6")),
            ("Quarantine", Some("#dc2626")),
            ("Discard", Some("#1c1917")),
            ("Acclimatize", Some("#22c55e")),
        ]),
    ];

    for (category, tags) in tag_categories {
        let cat_id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT OR IGNORE INTO tags (id, name, category, color) VALUES (?1, ?2, ?3, NULL)",
            rusqlite::params![cat_id, category, category],
        )?;
        for (tag_name, color) in tags {
            let tag_id = uuid::Uuid::new_v4().to_string();
            conn.execute(
                "INSERT OR IGNORE INTO tags (id, name, category, parent_tag_id, color) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![tag_id, tag_name, category, cat_id, color],
            )?;
        }
    }

    Ok(())
}

fn migration_019_strain_model(conn: &Connection) -> DbResult<()> {
    // WP-28: strains as first-class entities between species and specimens.
    // Purely additive — no existing tables are altered beyond two nullable
    // columns on specimens.
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS strains (
            id                   TEXT    PRIMARY KEY,
            species_id           TEXT    NOT NULL REFERENCES species(id),
            name                 TEXT    NOT NULL,
            code                 TEXT    NOT NULL,
            strain_type          TEXT    NOT NULL DEFAULT 'wildtype',
            status               TEXT    NOT NULL DEFAULT 'unverified'
                                     CHECK(status IN ('unverified','claimed',
                                                      'confirmed_manual','confirmed_genomic')),
            claimed_by           TEXT,
            claimed_at           TEXT,
            confirmation_basis   TEXT,
            genomic_fingerprint  TEXT,
            is_hybrid            INTEGER NOT NULL DEFAULT 0,
            is_archived          INTEGER NOT NULL DEFAULT 0,
            archived_at          TEXT,
            created_by           TEXT    REFERENCES users(id),
            created_at           TEXT    NOT NULL DEFAULT (datetime('now')),
            updated_at           TEXT    NOT NULL DEFAULT (datetime('now')),
            UNIQUE(species_id, code)
        );

        CREATE INDEX IF NOT EXISTS idx_strains_species
            ON strains(species_id);
        CREATE INDEX IF NOT EXISTS idx_strains_status
            ON strains(status);

        CREATE TABLE IF NOT EXISTS strain_parents (
            id                          TEXT    PRIMARY KEY,
            strain_id                   TEXT    NOT NULL REFERENCES strains(id),
            parent_strain_id            TEXT    NOT NULL REFERENCES strains(id),
            parent_role                 TEXT,
            parent_chain_seq_at_creation INTEGER
        );

        CREATE INDEX IF NOT EXISTS idx_strain_parents_strain
            ON strain_parents(strain_id);
        CREATE INDEX IF NOT EXISTS idx_strain_parents_parent
            ON strain_parents(parent_strain_id);

        CREATE TABLE IF NOT EXISTS hybridization_events (
            id                TEXT    PRIMARY KEY,
            hybrid_strain_id  TEXT    NOT NULL REFERENCES strains(id),
            parent_a_strain_id TEXT   NOT NULL REFERENCES strains(id),
            parent_b_strain_id TEXT   NOT NULL REFERENCES strains(id),
            parent_a_chain_seq INTEGER NOT NULL,
            parent_b_chain_seq INTEGER NOT NULL,
            notes             TEXT,
            created_by        TEXT    REFERENCES users(id),
            created_at        TEXT    NOT NULL DEFAULT (datetime('now'))
        );

        CREATE INDEX IF NOT EXISTS idx_hybridization_events_hybrid
            ON hybridization_events(hybrid_strain_id);
    ")?;

    // Additive columns on specimens — nullable so existing rows are unaffected.
    // SQLite does not support ADD COLUMN with a REFERENCES clause in older
    // versions, so we omit the FK keyword and rely on application-level checks.
    conn.execute_batch("
        ALTER TABLE specimens ADD COLUMN strain_id TEXT;
        ALTER TABLE specimens ADD COLUMN strain_chain_seq INTEGER;
        CREATE INDEX IF NOT EXISTS idx_specimens_strain ON specimens(strain_id);
    ")?;

    Ok(())
}

fn migration_020_expanded_taxonomy(conn: &Connection) -> DbResult<()> {
    // WP-35: hierarchical taxonomy backbone (Genus → Kingdom).
    // Taxa records are classification-only — no hash chain involvement.
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS taxa (
            id              TEXT    PRIMARY KEY,
            rank            TEXT    NOT NULL
                                CHECK(rank IN ('kingdom','phylum','class','order','family','genus')),
            name            TEXT    NOT NULL,
            parent_id       TEXT    REFERENCES taxa(id),
            ncbi_taxon_id   INTEGER,
            ncbi_updated_at TEXT,
            local_override  INTEGER NOT NULL DEFAULT 0,
            taxon_path      TEXT,
            created_at      TEXT    NOT NULL DEFAULT (datetime('now')),
            updated_at      TEXT    NOT NULL DEFAULT (datetime('now'))
        );

        CREATE INDEX IF NOT EXISTS idx_taxa_parent ON taxa(parent_id);
        CREATE INDEX IF NOT EXISTS idx_taxa_rank   ON taxa(rank);
        CREATE INDEX IF NOT EXISTS idx_taxa_name   ON taxa(name);
    ")?;

    // Additive columns on species — nullable so existing rows are unaffected.
    // Safe to run on any database at schema version 1–19.
    conn.execute_batch("
        ALTER TABLE species ADD COLUMN taxon_path    TEXT;
        ALTER TABLE species ADD COLUMN ncbi_taxon_id INTEGER;
    ")?;

    // Back-fill genus taxa from existing species data.
    backfill_genus_taxa(conn)?;

    Ok(())
}

/// Extracts distinct genus values from the species table, creates a `taxa` row
/// for each genus that does not already have one, and then updates
/// `species.taxon_path` for any species whose path is not yet set.
///
/// Safe to call multiple times — idempotent by design.
pub fn backfill_genus_taxa(conn: &Connection) -> DbResult<()> {
    let genera: Vec<String> = {
        let mut stmt = conn.prepare("SELECT DISTINCT genus FROM species ORDER BY genus")?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        rows.filter_map(|r| r.ok()).collect()
    };

    for genus in genera {
        // Check for an existing genus taxon so re-runs are safe.
        let existing_id: Option<String> = conn
            .query_row(
                "SELECT id FROM taxa WHERE rank = 'genus' AND name = ?1",
                rusqlite::params![genus],
                |r| r.get(0),
            )
            .ok();

        let taxon_id = if let Some(id) = existing_id {
            id
        } else {
            let id = uuid::Uuid::new_v4().to_string();
            // taxon_path for a root genus (no ancestors yet) is a JSON array
            // containing only its own ID: ["<id>"].  UUID chars are safe in JSON.
            let path = format!("[\"{}\"]", id);
            conn.execute(
                "INSERT INTO taxa (id, rank, name, parent_id, local_override, taxon_path)
                 VALUES (?1, 'genus', ?2, NULL, 0, ?3)",
                rusqlite::params![id, genus, path],
            )?;
            id
        };

        // Update species that do not yet have a taxon_path.
        let path = format!("[\"{}\"]", taxon_id);
        conn.execute(
            "UPDATE species SET taxon_path = ?1 WHERE genus = ?2 AND taxon_path IS NULL",
            rusqlite::params![path, genus],
        )?;
    }

    Ok(())
}

fn migration_021_ncbi_sync_log(conn: &Connection) -> DbResult<()> {
    // WP-36: NCBI Taxonomy import and ongoing sync support.
    // ncbi_sync_log records every import, update, and conflict detected during
    // NCBI sync operations, and tracks admin-driven conflict resolutions.
    // Resolution CHECK uses IS NULL so that unresolved rows (NULL) are always
    // valid without needing NULL in the IN list.
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS ncbi_sync_log (
            id               TEXT    PRIMARY KEY,
            sync_type        TEXT    NOT NULL
                                 CHECK(sync_type IN ('import','update','conflict')),
            taxon_id         TEXT,
            ncbi_taxon_id    INTEGER,
            conflict_details TEXT,
            resolved_at      TEXT,
            resolved_by      TEXT,
            resolution       TEXT
                                 CHECK(resolution IS NULL OR
                                       resolution IN ('kept_local','accepted_ncbi','merged')),
            created_at       TEXT    NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_ncbi_sync_log_taxon
            ON ncbi_sync_log(taxon_id);
        CREATE INDEX IF NOT EXISTS idx_ncbi_sync_log_ncbi_id
            ON ncbi_sync_log(ncbi_taxon_id);
        CREATE INDEX IF NOT EXISTS idx_ncbi_sync_log_type
            ON ncbi_sync_log(sync_type);
        CREATE INDEX IF NOT EXISTS idx_ncbi_sync_log_created
            ON ncbi_sync_log(created_at DESC);
    ")?;
    Ok(())
}

fn migration_022_hybrid_generation_labels(conn: &Connection) -> DbResult<()> {
    // WP-38: additive columns for generation labeling, backcross depth, and
    // cross-species override flag.  All nullable / have defaults so existing
    // rows are unaffected; no table rebuild required.
    conn.execute_batch(
        "ALTER TABLE hybridization_events ADD COLUMN generation_label TEXT;
         ALTER TABLE hybridization_events ADD COLUMN backcross_depth INTEGER;
         ALTER TABLE strains ADD COLUMN is_cross_species INTEGER NOT NULL DEFAULT 0;",
    )?;
    Ok(())
}

fn migration_023_cell_culture_vocabulary(conn: &Connection) -> DbResult<()> {
    // WP-30: expand the cell_culture vocabulary with additional lifecycle-state
    // stages, common propagation techniques, supplement types, and biomanufacturing
    // compliance / inventory terms.  Migration 018 seeded a minimal base set;
    // this migration is purely additive on top of it.
    // INSERT OR IGNORE keeps the migration idempotent and leaves all
    // plant_tissue_culture rows completely untouched.
    conn.execute_batch("
        BEGIN;

        -- Lifecycle-state stages that complement the phase-based stages from migration 018.
        -- contaminated and discarded are terminal: cells in these states cannot be
        -- progressed further without remediation or replacement.
        INSERT OR IGNORE INTO stages (profile, code, label, sort_order, is_terminal) VALUES
            ('cell_culture', 'thawed',        'Thawed',             13, 0),
            ('cell_culture', 'adherent',      'Adherent',           14, 0),
            ('cell_culture', 'suspension',    'Suspension Culture', 15, 0),
            ('cell_culture', 'confluent',     'Confluent',          16, 0),
            ('cell_culture', 'passaged',      'Passaged',           17, 0),
            ('cell_culture', 'cryopreserved', 'Cryopreserved',      18, 0),
            ('cell_culture', 'contaminated',  'Contaminated',       19, 1),
            ('cell_culture', 'discarded',     'Discarded',          20, 1);

        -- Common propagation terminology used in standard cell culture protocols.
        INSERT OR IGNORE INTO propagation_methods (profile, code, label, sort_order) VALUES
            ('cell_culture', 'trypsinization',         'Trypsinization',          8),
            ('cell_culture', 'mechanical_dissociation','Mechanical Dissociation', 9),
            ('cell_culture', 'dilution',               'Dilution Passaging',      10),
            ('cell_culture', 'subculturing',           'Subculturing',            11);

        -- Media supplement types not covered by the growth-factor / cytokine categories.
        INSERT OR IGNORE INTO hormone_types (profile, code, label, sort_order) VALUES
            ('cell_culture', 'serum_supplement',  'Serum Supplement',  5),
            ('cell_culture', 'vitamin_supplement','Vitamin Supplement', 6);

        -- Biomanufacturing compliance record types.
        INSERT OR IGNORE INTO compliance_record_types (profile, code, label, sort_order) VALUES
            ('cell_culture', 'gmp_batch_record',  'GMP Batch Record',           10),
            ('cell_culture', 'cell_line_identity','Cell Line Identity Report',  11);

        -- International regulatory agencies relevant to cell culture / biomanufacturing.
        INSERT OR IGNORE INTO compliance_agencies (profile, code, label, sort_order) VALUES
            ('cell_culture', 'EMA', 'EMA (European Medicines Agency)', 5),
            ('cell_culture', 'ICH', 'ICH Guidelines',                  6);

        -- Additional consumable categories for cell culture labs.
        INSERT OR IGNORE INTO inventory_categories (profile, code, label, sort_order) VALUES
            ('cell_culture', 'disposables', 'Plasticware & Disposables',  8),
            ('cell_culture', 'antibiotics', 'Antibiotics & Antimycotics', 9);

        COMMIT;
    ")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn migrated_db() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory DB");
        run_all(&conn).expect("all migrations must succeed on a fresh in-memory DB");
        conn
    }

    #[test]
    fn all_migrations_run_on_empty_db() {
        let _ = migrated_db();
    }

    #[test]
    fn migrations_are_idempotent() {
        // Running run_all a second time on an already-migrated DB should be a no-op.
        let conn = migrated_db();
        run_all(&conn).expect("second run of migrations must not error");
    }

    #[test]
    fn stages_has_fifteen_ptc_entries() {
        let conn = migrated_db();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM stages WHERE profile = 'plant_tissue_culture'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 15, "expected 15 PTC stage entries from seed data");
    }

    #[test]
    fn only_archived_stage_is_terminal() {
        let conn = migrated_db();
        let terminal: Vec<String> = {
            let mut stmt = conn
                .prepare(
                    "SELECT code FROM stages \
                     WHERE profile = 'plant_tissue_culture' AND is_terminal = 1",
                )
                .unwrap();
            stmt.query_map([], |r| r.get(0))
                .unwrap()
                .filter_map(|r| r.ok())
                .collect()
        };
        assert_eq!(terminal, vec!["archived"],
            "exactly one stage should be terminal, and it must be 'archived'");
    }

    #[test]
    fn non_terminal_stages_count_is_fourteen() {
        let conn = migrated_db();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM stages \
                 WHERE profile = 'plant_tissue_culture' AND is_terminal = 0",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 14,
            "14 non-terminal stages should be available for selection in the UI");
    }

    #[test]
    fn propagation_methods_seeded_correctly() {
        let conn = migrated_db();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM propagation_methods WHERE profile = 'plant_tissue_culture'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 7);
    }

    #[test]
    fn compliance_record_types_seeded_correctly() {
        let conn = migrated_db();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM compliance_record_types WHERE profile = 'plant_tissue_culture'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 9);
    }

    #[test]
    fn inventory_categories_seeded_correctly() {
        let conn = migrated_db();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM inventory_categories WHERE profile = 'plant_tissue_culture'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 7);
    }

    #[test]
    fn hormone_types_seeded_correctly() {
        let conn = migrated_db();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM hormone_types WHERE profile = 'plant_tissue_culture'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 4);
    }

    // ── cell_culture vocabulary tests (WP-27) ──────────────────────────────

    #[test]
    fn cell_culture_stages_seeded() {
        let conn = migrated_db();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM stages WHERE profile = 'cell_culture'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        // 12 from migration 018 + 8 lifecycle-state stages from migration 023.
        assert_eq!(count, 20, "expected 20 cell_culture stage entries after migrations 018+023");
    }

    #[test]
    fn cell_culture_only_archived_is_terminal() {
        let conn = migrated_db();
        let mut terminal: Vec<String> = {
            let mut stmt = conn
                .prepare(
                    "SELECT code FROM stages \
                     WHERE profile = 'cell_culture' AND is_terminal = 1 \
                     ORDER BY sort_order",
                )
                .unwrap();
            stmt.query_map([], |r| r.get(0))
                .unwrap()
                .filter_map(|r| r.ok())
                .collect()
        };
        terminal.sort();
        // archived (migration 018) + contaminated + discarded (migration 023).
        assert_eq!(
            terminal,
            vec!["archived", "contaminated", "discarded"],
            "cell_culture terminal stages must be archived, contaminated, and discarded"
        );
    }

    #[test]
    fn cell_culture_non_terminal_stages_count() {
        let conn = migrated_db();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM stages \
                 WHERE profile = 'cell_culture' AND is_terminal = 0",
                [],
                |r| r.get(0),
            )
            .unwrap();
        // 11 from migration 018 + 6 non-terminal lifecycle stages from migration 023.
        assert_eq!(count, 17,
            "17 non-terminal cell_culture stages should be available for selection");
    }

    #[test]
    fn cell_culture_propagation_methods_seeded() {
        let conn = migrated_db();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM propagation_methods WHERE profile = 'cell_culture'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        // 7 from migration 018 + 4 from migration 023.
        assert_eq!(count, 11);
    }

    #[test]
    fn cell_culture_hormone_types_seeded() {
        let conn = migrated_db();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM hormone_types WHERE profile = 'cell_culture'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        // 4 from migration 018 + 2 from migration 023.
        assert_eq!(count, 6);
    }

    #[test]
    fn cell_culture_compliance_record_types_seeded() {
        let conn = migrated_db();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM compliance_record_types WHERE profile = 'cell_culture'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        // 9 from migration 018 + 2 from migration 023.
        assert_eq!(count, 11);
    }

    #[test]
    fn cell_culture_compliance_agencies_seeded() {
        let conn = migrated_db();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM compliance_agencies WHERE profile = 'cell_culture'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        // 4 from migration 018 + 2 from migration 023.
        assert_eq!(count, 6);
    }

    #[test]
    fn cell_culture_inventory_categories_seeded() {
        let conn = migrated_db();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM inventory_categories WHERE profile = 'cell_culture'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        // 7 from migration 018 + 2 from migration 023.
        assert_eq!(count, 9);
    }

    #[test]
    fn cell_culture_vocabulary_does_not_affect_ptc() {
        let conn = migrated_db();
        // PTC stage count must remain exactly 15 after migrations 018 and 023.
        let ptc_stages: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM stages WHERE profile = 'plant_tissue_culture'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(ptc_stages, 15,
            "PTC stage count must be unchanged after cell_culture seeding");

        // PTC propagation methods must still be 7.
        let ptc_props: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM propagation_methods \
                 WHERE profile = 'plant_tissue_culture'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(ptc_props, 7,
            "PTC propagation method count must be unchanged after cell_culture seeding");
    }

    // ── migration 023 (WP-30) ──────────────────────────────────────────────────

    #[test]
    fn cell_culture_lifecycle_state_stages_present() {
        let conn = migrated_db();
        let codes_expected = [
            "thawed", "adherent", "suspension", "confluent",
            "passaged", "cryopreserved", "contaminated", "discarded",
        ];
        for code in &codes_expected {
            let found: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM stages \
                     WHERE profile = 'cell_culture' AND code = ?1",
                    rusqlite::params![code],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(found, 1, "cell_culture stage '{}' must be present", code);
        }
    }

    #[test]
    fn cell_culture_contaminated_and_discarded_are_terminal() {
        let conn = migrated_db();
        for code in &["contaminated", "discarded"] {
            let is_terminal: i64 = conn
                .query_row(
                    "SELECT is_terminal FROM stages \
                     WHERE profile = 'cell_culture' AND code = ?1",
                    rusqlite::params![code],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(is_terminal, 1, "cell_culture stage '{}' must be terminal", code);
        }
    }

    #[test]
    fn cell_culture_lifecycle_stages_are_non_terminal() {
        let conn = migrated_db();
        for code in &["thawed", "adherent", "suspension", "confluent", "passaged", "cryopreserved"] {
            let is_terminal: i64 = conn
                .query_row(
                    "SELECT is_terminal FROM stages \
                     WHERE profile = 'cell_culture' AND code = ?1",
                    rusqlite::params![code],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(is_terminal, 0, "cell_culture stage '{}' must be non-terminal", code);
        }
    }

    #[test]
    fn cell_culture_propagation_methods_023_present() {
        let conn = migrated_db();
        let codes_expected = [
            "trypsinization", "mechanical_dissociation", "dilution", "subculturing",
        ];
        for code in &codes_expected {
            let found: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM propagation_methods \
                     WHERE profile = 'cell_culture' AND code = ?1",
                    rusqlite::params![code],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(found, 1, "cell_culture propagation method '{}' must be present", code);
        }
    }

    #[test]
    fn cell_culture_hormone_types_023_present() {
        let conn = migrated_db();
        for code in &["serum_supplement", "vitamin_supplement"] {
            let found: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM hormone_types \
                     WHERE profile = 'cell_culture' AND code = ?1",
                    rusqlite::params![code],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(found, 1, "cell_culture hormone_type '{}' must be present", code);
        }
    }

    #[test]
    fn cell_culture_compliance_023_present() {
        let conn = migrated_db();
        for code in &["gmp_batch_record", "cell_line_identity"] {
            let found: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM compliance_record_types \
                     WHERE profile = 'cell_culture' AND code = ?1",
                    rusqlite::params![code],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(found, 1, "cell_culture compliance_record_type '{}' must be present", code);
        }
        for code in &["EMA", "ICH"] {
            let found: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM compliance_agencies \
                     WHERE profile = 'cell_culture' AND code = ?1",
                    rusqlite::params![code],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(found, 1, "cell_culture compliance_agency '{}' must be present", code);
        }
    }

    #[test]
    fn cell_culture_inventory_categories_023_present() {
        let conn = migrated_db();
        for code in &["disposables", "antibiotics"] {
            let found: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM inventory_categories \
                     WHERE profile = 'cell_culture' AND code = ?1",
                    rusqlite::params![code],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(found, 1, "cell_culture inventory_category '{}' must be present", code);
        }
    }

    #[test]
    fn cell_culture_vocabulary_023_idempotent() {
        let conn = migrated_db();
        // Running the migration a second time must not add duplicate rows.
        migration_023_cell_culture_vocabulary(&conn)
            .expect("re-running migration 023 must succeed");
        let stage_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM stages WHERE profile = 'cell_culture'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(stage_count, 20, "re-run must not duplicate cell_culture stage rows");
    }

    #[test]
    fn cell_culture_vocabulary_023_ptc_unchanged() {
        let conn = migrated_db();
        let ptc_stages: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM stages WHERE profile = 'plant_tissue_culture'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(ptc_stages, 15,
            "migration 023 must not alter plant_tissue_culture stage count");
    }

    #[test]
    fn specimen_with_valid_stage_persists_after_migration() {
        let conn = migrated_db();
        // Insert a species first (required FK).
        conn.execute(
            "INSERT INTO species (id, genus, species_name, species_code) \
             VALUES ('sp1', 'Citrus', 'sinensis', 'CIT-01')",
            [],
        )
        .unwrap();
        let now = "2026-01-01";
        conn.execute(
            "INSERT INTO specimens \
             (id, accession_number, species_id, stage, initiation_date, \
              quarantine_flag, ip_flag, subculture_count, is_archived, contamination_flag, \
              generation, lineage_passage_offset, created_at, updated_at) \
             VALUES ('s1', '2026-01-01-CIT-01-001', 'sp1', 'explant', ?1, \
                     0, 0, 0, 0, 0, 0, 0, ?1, ?1)",
            [now],
        )
        .unwrap();
        let stage: String = conn
            .query_row("SELECT stage FROM specimens WHERE id = 's1'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(stage, "explant");
    }

    // ── migration 019 (WP-28) ──────────────────────────────────────────────────

    #[test]
    fn strains_table_exists_after_migration_019() {
        let conn = migrated_db();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM strains", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0, "strains table must exist and be empty on fresh DB");
    }

    #[test]
    fn strain_parents_table_exists_after_migration_019() {
        let conn = migrated_db();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM strain_parents", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn hybridization_events_table_exists_after_migration_019() {
        let conn = migrated_db();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM hybridization_events", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn specimens_has_strain_id_column_after_migration_019() {
        let conn = migrated_db();
        // Insert a species and specimen; strain_id must default to NULL.
        conn.execute(
            "INSERT INTO species (id, genus, species_name, species_code) \
             VALUES ('sp1', 'Citrus', 'sinensis', 'CIT-01')",
            [],
        ).unwrap();
        let now = "2026-01-01";
        conn.execute(
            "INSERT INTO specimens \
             (id, accession_number, species_id, stage, initiation_date, \
              quarantine_flag, ip_flag, subculture_count, is_archived, contamination_flag, \
              generation, lineage_passage_offset, created_at, updated_at) \
             VALUES ('s1', '2026-01-01-CIT-01-001', 'sp1', 'explant', ?1, \
                     0, 0, 0, 0, 0, 0, 0, ?1, ?1)",
            [now],
        ).unwrap();
        let strain_id: Option<String> = conn
            .query_row(
                "SELECT strain_id FROM specimens WHERE id = 's1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(strain_id.is_none(), "strain_id must default to NULL for existing specimens");
    }

    // ── migration 020 (WP-35) ──────────────────────────────────────────────────

    #[test]
    fn taxa_table_exists_after_migration_020() {
        let conn = migrated_db();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM taxa", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0, "taxa table must exist and be empty on a fresh DB");
    }

    #[test]
    fn taxa_rank_check_constraint_rejects_invalid_rank() {
        let conn = migrated_db();
        let result = conn.execute(
            "INSERT INTO taxa (id, rank, name) VALUES ('t1', 'species', 'Test')",
            [],
        );
        assert!(result.is_err(), "rank 'species' must be rejected by the CHECK constraint");
    }

    #[test]
    fn taxa_rank_check_constraint_accepts_valid_ranks() {
        let conn = migrated_db();
        for (i, rank) in ["kingdom", "phylum", "class", "order", "family", "genus"]
            .iter()
            .enumerate()
        {
            conn.execute(
                "INSERT INTO taxa (id, rank, name) VALUES (?1, ?2, ?3)",
                rusqlite::params![format!("t{}", i), rank, format!("Test {}", rank)],
            )
            .unwrap_or_else(|e| panic!("rank '{}' should be accepted: {}", rank, e));
        }
    }

    #[test]
    fn species_has_taxon_path_and_ncbi_columns_after_migration_020() {
        let conn = migrated_db();
        // Columns must exist and accept NULL values.
        conn.execute(
            "INSERT INTO species (id, genus, species_name, species_code, taxon_path, ncbi_taxon_id) \
             VALUES ('sp1', 'Testus', 'exampleus', 'TST-01', NULL, NULL)",
            [],
        )
        .expect("species insert with new nullable columns must succeed");
        let tp: Option<String> = conn
            .query_row("SELECT taxon_path FROM species WHERE id = 'sp1'", [], |r| r.get(0))
            .unwrap();
        assert!(tp.is_none(), "taxon_path must default to NULL");
    }

    #[test]
    fn backfill_creates_genus_taxon_for_existing_species() {
        let conn = migrated_db();
        conn.execute(
            "INSERT INTO species (id, genus, species_name, species_code) \
             VALUES ('sp1', 'Citrus', 'sinensis', 'CIT-TST')",
            [],
        )
        .unwrap();

        backfill_genus_taxa(&conn).expect("backfill must succeed");

        let taxon_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM taxa WHERE rank = 'genus' AND name = 'Citrus'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(taxon_count, 1, "one genus taxon must be created for 'Citrus'");

        let taxon_path: Option<String> = conn
            .query_row(
                "SELECT taxon_path FROM species WHERE id = 'sp1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(
            taxon_path.is_some(),
            "species.taxon_path must be populated after backfill"
        );
    }

    #[test]
    fn backfill_is_idempotent() {
        let conn = migrated_db();
        conn.execute(
            "INSERT INTO species (id, genus, species_name, species_code) \
             VALUES ('sp1', 'Nandina', 'domestica', 'NAN-TST')",
            [],
        )
        .unwrap();

        backfill_genus_taxa(&conn).expect("first backfill must succeed");
        backfill_genus_taxa(&conn).expect("second backfill must succeed");

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM taxa WHERE rank = 'genus' AND name = 'Nandina'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "backfill must not create duplicate taxa on re-run");
    }

    #[test]
    fn backfill_groups_multiple_species_under_same_genus_taxon() {
        let conn = migrated_db();
        for (id, sp) in [("sp1", "sinensis"), ("sp2", "limon"), ("sp3", "paradisi")] {
            conn.execute(
                "INSERT INTO species (id, genus, species_name, species_code) \
                 VALUES (?1, 'Citrus', ?2, ?3)",
                rusqlite::params![id, sp, format!("CIT-{}", sp)],
            )
            .unwrap();
        }

        backfill_genus_taxa(&conn).expect("backfill must succeed");

        let taxon_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM taxa WHERE rank = 'genus' AND name = 'Citrus'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(taxon_count, 1, "three Citrus species must share a single genus taxon");

        let species_with_path: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM species WHERE genus = 'Citrus' AND taxon_path IS NOT NULL",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            species_with_path, 3,
            "all three Citrus species must have taxon_path set"
        );
    }

    // ── migration 021 (WP-36) ──────────────────────────────────────────────────

    #[test]
    fn ncbi_sync_log_table_exists_after_migration_021() {
        let conn = migrated_db();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM ncbi_sync_log", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0, "ncbi_sync_log must exist and be empty on fresh DB");
    }

    #[test]
    fn ncbi_sync_log_type_check_accepts_valid_values() {
        let conn = migrated_db();
        let now = "2026-01-01T00:00:00.000Z";
        for sync_type in ["import", "update", "conflict"] {
            let id = uuid::Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO ncbi_sync_log (id, sync_type, created_at) VALUES (?1, ?2, ?3)",
                rusqlite::params![id, sync_type, now],
            )
            .unwrap_or_else(|e| panic!("sync_type '{}' should be accepted: {}", sync_type, e));
        }
    }

    #[test]
    fn ncbi_sync_log_type_check_rejects_invalid_value() {
        let conn = migrated_db();
        let result = conn.execute(
            "INSERT INTO ncbi_sync_log (id, sync_type, created_at) \
             VALUES ('bad', 'delete', '2026-01-01')",
            [],
        );
        assert!(result.is_err(), "sync_type 'delete' must be rejected by CHECK constraint");
    }

    #[test]
    fn ncbi_sync_log_resolution_check_accepts_valid_values() {
        let conn = migrated_db();
        let now = "2026-01-01T00:00:00.000Z";
        for (i, res) in ["kept_local", "accepted_ncbi", "merged"].iter().enumerate() {
            let id = format!("log-{}", i);
            conn.execute(
                "INSERT INTO ncbi_sync_log (id, sync_type, resolution, created_at) \
                 VALUES (?1, 'conflict', ?2, ?3)",
                rusqlite::params![id, res, now],
            )
            .unwrap_or_else(|e| panic!("resolution '{}' should be accepted: {}", res, e));
        }
    }

    #[test]
    fn ncbi_sync_log_resolution_check_rejects_invalid_value() {
        let conn = migrated_db();
        let result = conn.execute(
            "INSERT INTO ncbi_sync_log (id, sync_type, resolution, created_at) \
             VALUES ('bad', 'conflict', 'delete_local', '2026-01-01')",
            [],
        );
        assert!(result.is_err(), "resolution 'delete_local' must be rejected by CHECK");
    }

    #[test]
    fn ncbi_sync_log_resolution_null_is_always_valid() {
        let conn = migrated_db();
        conn.execute(
            "INSERT INTO ncbi_sync_log (id, sync_type, resolution, created_at) \
             VALUES ('log-null', 'conflict', NULL, '2026-01-01')",
            [],
        )
        .expect("NULL resolution must always be valid for unresolved conflicts");
    }

    #[test]
    fn ncbi_sync_log_stores_conflict_details_json() {
        let conn = migrated_db();
        let details = r#"{"name":{"local":"Foo","ncbi":"Bar"}}"#;
        conn.execute(
            "INSERT INTO ncbi_sync_log (id, sync_type, taxon_id, ncbi_taxon_id, \
             conflict_details, created_at) \
             VALUES ('log-cd', 'conflict', 'taxon-1', 12345, ?1, '2026-01-01')",
            rusqlite::params![details],
        )
        .unwrap();

        let stored: String = conn
            .query_row(
                "SELECT conflict_details FROM ncbi_sync_log WHERE id = 'log-cd'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(stored, details, "conflict_details JSON must round-trip correctly");
    }

    // ── migration 022 (WP-38) ──────────────────────────────────────────────────

    #[test]
    fn hybridization_events_has_generation_label_after_migration_022() {
        let conn = migrated_db();
        conn.execute(
            "INSERT INTO species (id, genus, species_name, species_code) \
             VALUES ('sp1', 'Testus', 'exampleus', 'TST-01')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO strains (id, species_id, name, code) VALUES ('s1', 'sp1', 'Parent A', 'PA')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO strains (id, species_id, name, code) VALUES ('s2', 'sp1', 'Parent B', 'PB')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO strains (id, species_id, name, code, is_hybrid) \
             VALUES ('h1', 'sp1', 'Hybrid', 'HY', 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO hybridization_events \
             (id, hybrid_strain_id, parent_a_strain_id, parent_b_strain_id, \
              parent_a_chain_seq, parent_b_chain_seq, generation_label, backcross_depth) \
             VALUES ('evt1', 'h1', 's1', 's2', 0, 0, 'F1', NULL)",
            [],
        )
        .expect("hybridization_events must accept generation_label and backcross_depth");
        let label: Option<String> = conn
            .query_row(
                "SELECT generation_label FROM hybridization_events WHERE id = 'evt1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(label, Some("F1".to_string()), "generation_label must round-trip");
    }

    #[test]
    fn strains_has_is_cross_species_after_migration_022() {
        let conn = migrated_db();
        conn.execute(
            "INSERT INTO species (id, genus, species_name, species_code) \
             VALUES ('sp1', 'Testus', 'exampleus', 'TST-01')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO strains (id, species_id, name, code, is_cross_species) \
             VALUES ('s1', 'sp1', 'CrossHybrid', 'CH', 1)",
            [],
        )
        .expect("strains must accept is_cross_species = 1");
        let flag: i32 = conn
            .query_row(
                "SELECT is_cross_species FROM strains WHERE id = 's1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(flag, 1, "is_cross_species must round-trip");
    }

    #[test]
    fn strains_is_cross_species_defaults_to_zero() {
        let conn = migrated_db();
        conn.execute(
            "INSERT INTO species (id, genus, species_name, species_code) \
             VALUES ('sp1', 'Testus', 'exampleus', 'TST-01')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO strains (id, species_id, name, code) \
             VALUES ('s2', 'sp1', 'Normal', 'NM')",
            [],
        )
        .unwrap();
        let flag: i32 = conn
            .query_row(
                "SELECT is_cross_species FROM strains WHERE id = 's2'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(flag, 0, "is_cross_species must default to 0 for existing/new rows");
    }

    #[test]
    fn frozen_vials_table_exists_after_migration_025() {
        let conn = migrated_db();
        // Table must exist.
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM frozen_vials",
                [],
                |r| r.get(0),
            )
            .expect("frozen_vials table must exist after migration 025");
        assert_eq!(count, 0);
    }

    #[test]
    fn frozen_vials_rejects_negative_count() {
        let conn = migrated_db();
        conn.execute(
            "INSERT INTO species (id, genus, species_name, species_code) \
             VALUES ('sp1', 'Homo', 'sapiens', 'HEK')",
            [],
        )
        .unwrap();
        let result = conn.execute(
            "INSERT INTO frozen_vials \
             (id, species_id, passage_number, vial_count, freeze_date, freeze_medium) \
             VALUES ('v1', 'sp1', 0, -1, '2026-01-01', '10% DMSO')",
            [],
        );
        assert!(result.is_err(), "negative vial_count must be rejected by CHECK constraint");
    }
}
