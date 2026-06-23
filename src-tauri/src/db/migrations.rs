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
        assert_eq!(count, 12, "expected 12 cell_culture stage entries from seed data");
    }

    #[test]
    fn cell_culture_only_archived_is_terminal() {
        let conn = migrated_db();
        let terminal: Vec<String> = {
            let mut stmt = conn
                .prepare(
                    "SELECT code FROM stages \
                     WHERE profile = 'cell_culture' AND is_terminal = 1",
                )
                .unwrap();
            stmt.query_map([], |r| r.get(0))
                .unwrap()
                .filter_map(|r| r.ok())
                .collect()
        };
        assert_eq!(terminal, vec!["archived"],
            "exactly one cell_culture stage should be terminal, and it must be 'archived'");
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
        assert_eq!(count, 11,
            "11 non-terminal cell_culture stages should be available for selection");
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
        assert_eq!(count, 7);
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
        assert_eq!(count, 4);
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
        assert_eq!(count, 9);
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
        assert_eq!(count, 4);
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
        assert_eq!(count, 7);
    }

    #[test]
    fn cell_culture_vocabulary_does_not_affect_ptc() {
        let conn = migrated_db();
        // PTC stage count must remain exactly 15 after migration 018.
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
}
