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
        "INSERT INTO users (id, username, password_hash, display_name, email, role)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
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
