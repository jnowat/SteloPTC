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

    if current < 26 {
        migration_026_biosafety_level(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (26)", [])?;
    }

    if current < 27 {
        migration_027_mycology_vocabulary(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (27)", [])?;
    }

    if current < 28 {
        migration_028_colonization_contaminant(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (28)", [])?;
    }

    if current < 29 {
        migration_029_genetic_lineage_markers(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (29)", [])?;
    }

    if current < 30 {
        migration_030_fruiting_records(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (30)", [])?;
    }

    if current < 31 {
        migration_031_taxon_hash_chain(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (31)", [])?;
    }

    if current < 32 {
        migration_032_domain_column(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (32)", [])?;
    }

    if current < 33 {
        migration_033_breeding_programs(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (33)", [])?;
    }

    if current < 34 {
        migration_034_provisional_taxa(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (34)", [])?;
    }

    if current < 35 {
        migration_035_multiuser_foundation(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (35)", [])?;
    }

    if current < 36 {
        migration_036_field_permissions(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (36)", [])?;
    }

    if current < 37 {
        migration_037_environmental_readings(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (37)", [])?;
    }

    if current < 38 {
        migration_038_notifications(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (38)", [])?;
    }

    if current < 39 {
        migration_039_perf_indexes_v2(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (39)", [])?;
    }

    if current < 40 {
        migration_040_locations(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (40)", [])?;
    }

    if current < 41 {
        migration_041_ai_suggestions(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (41)", [])?;
    }

    if current < 42 {
        migration_042_backup_targets(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (42)", [])?;
    }

    if current < 43 {
        migration_043_reanchor_events(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (43)", [])?;
    }

    if current < 44 {
        migration_044_signing_keys(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (44)", [])?;
    }

    if current < 45 {
        migration_045_installed_plugins(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (45)", [])?;
    }

    if current < 46 {
        migration_046_checkpoint_anchors(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (46)", [])?;
    }

    if current < 47 {
        migration_047_signed_events(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (47)", [])?;
    }

    if current < 48 {
        migration_048_regulatory_submissions(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (48)", [])?;
    }

    if current < 49 {
        migration_049_specimen_passports(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (49)", [])?;
    }

    if current < 50 {
        migration_050_taxonomy_registries(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (50)", [])?;
    }

    Ok(())
}

fn migration_050_taxonomy_registries(conn: &Connection) -> DbResult<()> {
    // WP-71: Shared taxonomy registry — federated, signed reference-data exchange.
    //
    // `taxonomy_registries` registers both directions of a registry bundle's life:
    //   - `issued`   : a signed registry this lab exported (kept so it can be
    //                  re-exported and to record that we vouched for it).
    //   - `imported` : a registry received from another lab, verified, and folded
    //                  into this lab's own audit chain (`audit_entry` links the
    //                  `registry_imported` audit row committing to `content_hash`).
    // `registry_json` is the full signed document. `UNIQUE(direction, registry_id)`
    // makes importing the same registry twice a no-op error rather than a silent
    // duplicate.
    //
    // `registry_record_dispositions` records, per imported registry, the
    // operator's per-record decision (`accept` / `override` / `fork`) and where a
    // record was reconciled locally — the audit trail of a federated merge. Import
    // is additive and non-destructive (see the registry module docs).
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS taxonomy_registries (
            id                    TEXT PRIMARY KEY,
            registry_id           TEXT NOT NULL,
            direction             TEXT NOT NULL
                                  CHECK (direction IN ('issued','imported')),
            issuer_lab            TEXT NOT NULL,
            issuer_public_key     TEXT NOT NULL,
            content_hash          TEXT NOT NULL,
            record_count          INTEGER NOT NULL DEFAULT 0,
            taxon_count           INTEGER NOT NULL DEFAULT 0,
            species_count         INTEGER NOT NULL DEFAULT 0,
            strain_count          INTEGER NOT NULL DEFAULT 0,
            verified              INTEGER NOT NULL DEFAULT 0,
            audit_entry           TEXT,
            registry_json         TEXT NOT NULL,
            created_by            TEXT REFERENCES users(id),
            created_at            TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(direction, registry_id)
        );
        CREATE INDEX IF NOT EXISTS idx_taxonomy_registries_direction
            ON taxonomy_registries(direction);

        CREATE TABLE IF NOT EXISTS registry_record_dispositions (
            id                    TEXT PRIMARY KEY,
            registry_row_id       TEXT NOT NULL REFERENCES taxonomy_registries(id),
            source_key            TEXT NOT NULL,
            record_type           TEXT NOT NULL,
            local_status          TEXT NOT NULL,
            disposition           TEXT NOT NULL
                                  CHECK (disposition IN ('accept','override','fork')),
            action_taken          TEXT NOT NULL,
            local_record_id       TEXT,
            created_at            TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_registry_dispositions_registry
            ON registry_record_dispositions(registry_row_id);",
    )?;
    Ok(())
}

fn migration_049_specimen_passports(conn: &Connection) -> DbResult<()> {
    // WP-70: Federated identity & inter-lab specimen transfer — the specimen
    // passport. A single table records both directions of a passport's life:
    //   - `issued`   : a signed passport this lab produced for one of its own
    //                  specimens, kept so it can be re-exported and to record that
    //                  we vouched for it.
    //   - `imported` : a passport received from another lab, verified, and folded
    //                  into this lab's own audit chain (`audit_entry` links the
    //                  `passport_imported` audit row that commits to `content_hash`).
    // `passport_json` is the full signed document. `UNIQUE(direction, passport_id)`
    // makes importing the same passport twice a no-op error rather than a silent
    // duplicate. SteloPTC does not transport passports over any network — see the
    // module docs for the disclosed scope boundary.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS specimen_passports (
            id                    TEXT PRIMARY KEY,
            passport_id           TEXT NOT NULL,
            direction             TEXT NOT NULL
                                  CHECK (direction IN ('issued','imported')),
            specimen_id           TEXT,
            issuer_lab            TEXT NOT NULL,
            issuer_public_key     TEXT NOT NULL,
            subject_accession     TEXT NOT NULL,
            subject_scientific_name TEXT,
            content_hash          TEXT NOT NULL,
            entry_count           INTEGER NOT NULL DEFAULT 0,
            verified              INTEGER NOT NULL DEFAULT 0,
            audit_entry           TEXT,
            passport_json         TEXT NOT NULL,
            created_by            TEXT REFERENCES users(id),
            created_at            TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(direction, passport_id)
        );
        CREATE INDEX IF NOT EXISTS idx_specimen_passports_direction
            ON specimen_passports(direction);
        CREATE INDEX IF NOT EXISTS idx_specimen_passports_specimen
            ON specimen_passports(specimen_id);",
    )?;
    Ok(())
}

fn migration_048_regulatory_submissions(conn: &Connection) -> DbResult<()> {
    // WP-68: Regulatory submission pipeline (advanced).
    //
    // Tracks the lifecycle of a regulatory submission (built on the WP-60 export
    // bundles): `ready`/`blocked` reflect the last readiness evaluation against
    // live compliance state; `generated` means the signed package was produced;
    // `submitted` means the operator submitted it through the official channel and
    // recorded the returned reference; `acknowledged` is reserved for a later
    // confirmation step. `scope` and `readiness` are JSON snapshots. SteloPTC does
    // not electronically submit to a government portal — see the module docs for
    // the disclosed scope boundary.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS regulatory_submissions (
            id                   TEXT PRIMARY KEY,
            kind                 TEXT NOT NULL,
            title                TEXT NOT NULL,
            scope                TEXT NOT NULL,
            status               TEXT NOT NULL DEFAULT 'draft'
                                 CHECK (status IN ('draft','ready','blocked','generated','submitted','acknowledged')),
            readiness            TEXT,
            package_path         TEXT,
            package_signature    TEXT,
            submission_reference TEXT,
            auto_generate        INTEGER NOT NULL DEFAULT 0,
            created_by           TEXT REFERENCES users(id),
            created_at           TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at           TEXT NOT NULL DEFAULT (datetime('now')),
            submitted_at         TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_regulatory_submissions_status
            ON regulatory_submissions(status);",
    )?;
    Ok(())
}

fn migration_047_signed_events(conn: &Connection) -> DbResult<()> {
    // WP-67: Trust Layer Phase 3 — specimen lifecycle events as signed
    // transactions.
    //
    // `user_signing_keys` holds one Ed25519 keypair per user (distinct from the
    // single lab-wide WP-60 export key in `signing_keys`), so a signed ledger
    // entry is attributable to the individual who authorized it. `signed_events`
    // is a monotonic, hash-chained ledger (`seq` gapless and UNIQUE; `prev_hash`
    // links each entry to the previous one) where every row additionally carries
    // a detached Ed25519 signature over its `event_hash` plus the `public_key`
    // that signed it. Content edits break `event_hash`; deletions break the `seq`
    // gap; forged attribution fails signature verification. See
    // db::queries::compute_entry_hash for the shared hashing primitive.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS user_signing_keys (
            user_id         TEXT PRIMARY KEY REFERENCES users(id),
            public_key_b64  TEXT NOT NULL,
            private_key_b64 TEXT NOT NULL,
            created_at      TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE TABLE IF NOT EXISTS signed_events (
            id          TEXT PRIMARY KEY,
            seq         INTEGER NOT NULL UNIQUE,
            event_type  TEXT NOT NULL,
            entity_type TEXT NOT NULL,
            entity_id   TEXT,
            user_id     TEXT REFERENCES users(id),
            payload     TEXT NOT NULL,
            prev_hash   TEXT NOT NULL,
            event_hash  TEXT NOT NULL,
            signature   TEXT NOT NULL,
            public_key  TEXT NOT NULL,
            created_at  TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_signed_events_entity
            ON signed_events(entity_type, entity_id);
        CREATE INDEX IF NOT EXISTS idx_signed_events_seq
            ON signed_events(seq);",
    )?;
    Ok(())
}

fn migration_046_checkpoint_anchors(conn: &Connection) -> DbResult<()> {
    // WP-66: Trust Layer Phase 2 — on-chain anchoring (Dogecoin OP_RETURN).
    //
    // Each row records the lifecycle of anchoring one audit-checkpoint's Merkle
    // root to a public chain: `prepared` (the OP_RETURN payload was generated),
    // `submitted` (the operator broadcast it externally and recorded the txid),
    // and `confirmed` (the on-chain data was independently verified to commit to
    // this exact root). `merkle_root` and `op_return_hex` are snapshotted at
    // prepare time so the anchor record stays interpretable even if the covering
    // checkpoint is later re-examined. When an anchor reaches `submitted`, the
    // txid is also written back to audit_checkpoints.anchored_txid (the Phase-2
    // hook reserved since migration 013) so the existing checkpoint row surfaces
    // its anchor directly.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS checkpoint_anchors (
            id            TEXT PRIMARY KEY,
            checkpoint_id TEXT NOT NULL REFERENCES audit_checkpoints(id),
            chain_name    TEXT NOT NULL DEFAULT 'dogecoin',
            merkle_root   TEXT NOT NULL,
            op_return_hex TEXT NOT NULL,
            txid          TEXT,
            status        TEXT NOT NULL DEFAULT 'prepared'
                          CHECK (status IN ('prepared','submitted','confirmed')),
            verified_at   TEXT,
            created_by    TEXT REFERENCES users(id),
            created_at    TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at    TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_checkpoint_anchors_checkpoint
            ON checkpoint_anchors(checkpoint_id);
        CREATE INDEX IF NOT EXISTS idx_checkpoint_anchors_txid
            ON checkpoint_anchors(txid);",
    )?;
    Ok(())
}

fn migration_036_field_permissions(conn: &Connection) -> DbResult<()> {
    // WP-55: Field-level permissions foundation for shared lab use.
    //
    // `field_permissions` is a plain (role, entity_type, field_name) -> visible
    // lookup. Absence of a row means "visible" (see db::permissions::is_field_visible),
    // so this table only needs rows for fields that are actually gated — it seeds
    // permissive defaults (visible = 1) for the three fields currently wired into
    // the masking layer: strains.genomic_fingerprint and
    // breeding_programs.{goal,target_traits}. Existing deployments see no change
    // in behavior until an admin explicitly restricts a role via PermissionsEditor.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS field_permissions (
            id          TEXT PRIMARY KEY,
            role        TEXT NOT NULL CHECK (role IN ('admin','supervisor','tech','guest')),
            entity_type TEXT NOT NULL,
            field_name  TEXT NOT NULL,
            visible     INTEGER NOT NULL DEFAULT 1,
            UNIQUE (role, entity_type, field_name)
        );
        CREATE INDEX IF NOT EXISTS idx_field_permissions_entity ON field_permissions(entity_type, field_name);",
    )?;

    let roles = ["admin", "supervisor", "tech", "guest"];
    let seeds: [(&str, &str); 3] = [
        ("strain", "genomic_fingerprint"),
        ("breeding_program", "goal"),
        ("breeding_program", "target_traits"),
    ];
    for (entity_type, field_name) in seeds {
        for role in roles {
            conn.execute(
                "INSERT OR IGNORE INTO field_permissions (id, role, entity_type, field_name, visible) \
                 VALUES (?1, ?2, ?3, ?4, 1)",
                rusqlite::params![uuid::Uuid::new_v4().to_string(), role, entity_type, field_name],
            )?;
        }
    }

    Ok(())
}

fn migration_037_environmental_readings(conn: &Connection) -> DbResult<()> {
    // WP-54: Environmental sensor integration foundation.
    //
    // A reading always belongs to a specimen and/or a subculture (at least one
    // must be set — enforced by CHECK). `source` records where the value came
    // from; `manual` is fully supported today, the other three values are
    // recorded for forward-compatibility with a future hardware transport
    // layer (see db::sensors for the parsing/validation foundation that
    // already exists independent of any live hardware connection).
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS environmental_readings (
            id            TEXT PRIMARY KEY,
            specimen_id   TEXT REFERENCES specimens(id) ON DELETE CASCADE,
            subculture_id TEXT REFERENCES subcultures(id) ON DELETE CASCADE,
            reading_type  TEXT NOT NULL CHECK (reading_type IN ('temp_c','humidity_pct','co2_ppm','light_lux','ph','custom')),
            value         REAL NOT NULL,
            unit          TEXT,
            source        TEXT NOT NULL DEFAULT 'manual' CHECK (source IN ('manual','usb_serial','bluetooth','mqtt')),
            recorded_at   TEXT NOT NULL DEFAULT (datetime('now')),
            notes         TEXT,
            created_by    TEXT,
            created_at    TEXT NOT NULL DEFAULT (datetime('now')),
            CHECK (specimen_id IS NOT NULL OR subculture_id IS NOT NULL)
        );
        CREATE INDEX IF NOT EXISTS idx_environmental_readings_specimen ON environmental_readings(specimen_id, recorded_at);
        CREATE INDEX IF NOT EXISTS idx_environmental_readings_subculture ON environmental_readings(subculture_id, recorded_at);",
    )?;
    Ok(())
}

fn migration_038_notifications(conn: &Connection) -> DbResult<()> {
    // WP-52: Email/push notification foundation.
    //
    // `notification_preferences` is per-user, per-channel (one row per user
    // per channel they've configured; absence of a row means "use the
    // built-in default" — see db::notifications::effective_preference).
    // `smtp_config` is a single-row table (id = 1) for the lab's outbound
    // mail server. The `password` column is stored as given by the admin —
    // see the WP-52 "As built" note in ROADMAP.md for the disclosed
    // trade-off (no OS-keychain integration in this packet, unlike the
    // zero-knowledge design used for WP-59 cloud-backup credentials).
    // `commands::backup::create_backup` redacts this column (to NULL) in the
    // backup file it produces, so the plaintext password lives only in the
    // live database, never in a copy that could leave the machine.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS notification_preferences (
            id           TEXT PRIMARY KEY,
            user_id      TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            channel      TEXT NOT NULL CHECK (channel IN ('desktop','email','mobile_push')),
            enabled      INTEGER NOT NULL DEFAULT 1,
            min_severity TEXT NOT NULL DEFAULT 'normal' CHECK (min_severity IN ('normal','high','critical')),
            UNIQUE (user_id, channel)
        );
        CREATE INDEX IF NOT EXISTS idx_notification_preferences_user ON notification_preferences(user_id);

        CREATE TABLE IF NOT EXISTS smtp_config (
            id           INTEGER PRIMARY KEY CHECK (id = 1),
            host         TEXT,
            port         INTEGER NOT NULL DEFAULT 587,
            username     TEXT,
            password     TEXT,
            from_address TEXT,
            use_tls      INTEGER NOT NULL DEFAULT 1,
            updated_at   TEXT NOT NULL DEFAULT (datetime('now'))
        );
        INSERT OR IGNORE INTO smtp_config (id) VALUES (1);

        INSERT OR IGNORE INTO app_settings (key, value) VALUES ('notification_check_interval_minutes', '15');",
    )?;
    Ok(())
}

fn migration_039_perf_indexes_v2(conn: &Connection) -> DbResult<()> {
    // WP-63: Performance & scalability hardening — exhaustive index audit follow-up
    // to migration_007. Each index below resolves a specific `EXPLAIN QUERY PLAN`
    // scan identified against the hot paths at 100k-specimen / 1M-subculture scale:
    //
    //   - specimens(is_archived, stage, species_id, created_at DESC): covers the
    //     SpecimenList filter+sort query (WHERE is_archived = ? AND stage = ? AND
    //     species_id = ? ORDER BY created_at DESC) without a secondary sort step.
    //     idx_specimens_archived_created (migration_007) already covers the
    //     2-column case; this is the full covering index for the 3-predicate case.
    //   - subcultures(specimen_id, created_at DESC): the passage timeline query
    //     orders by created_at, not passage_number — idx_subcultures_specimen_passage
    //     (migration_007) doesn't serve an ORDER BY created_at DESC.
    //   - subcultures(event_type, created_at DESC): compliance queries filter by
    //     event_type and want the most recent first; idx_subcultures_event_type
    //     (migration_015) is a single-column index with no sort component.
    //   - fruiting_records(specimen_id, flush_number): fruiting history is always
    //     read per-specimen ordered by flush number; the existing index
    //     (migration_030) is specimen_id-only.
    //   - breeding_records(program_id, generation_number): generational summaries
    //     group by program and order by generation; the existing indexes
    //     (migration_033) are single-column.
    //
    // audit_log(lineage_id, chain_seq) and audit_log(entity_type, entity_id) are
    // already covered by idx_audit_lineage (migration_009) and idx_audit_entity
    // (migration_001) respectively — no new index needed there.
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_specimens_archived_stage_species_created
            ON specimens(is_archived, stage, species_id, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_subcultures_specimen_created
            ON subcultures(specimen_id, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_subcultures_event_type_created
            ON subcultures(event_type, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_fruiting_records_specimen_flush
            ON fruiting_records(specimen_id, flush_number);
        CREATE INDEX IF NOT EXISTS idx_breeding_records_program_generation
            ON breeding_records(program_id, generation_number);

        INSERT OR IGNORE INTO app_settings (key, value) VALUES ('pedigree_max_depth', '10');",
    )?;
    Ok(())
}

fn migration_040_locations(conn: &Connection) -> DbResult<()> {
    // WP-57: Interactive lab map — optional location entity with floor-plan
    // coordinates. Purely additive: the existing free-text `specimens.location` /
    // `location_details` columns are completely unchanged and remain the
    // default way to record where a specimen lives. `location_id` is an
    // optional pin placement used only by the new map view; specimens with no
    // `location_id` simply don't appear on the map.
    //
    // Floor-plan images are stored inline as base64 (matching the existing
    // `attachments` inline-storage convention) rather than as a filesystem path,
    // so a location row travels intact inside a database backup. Coordinates are
    // fractional (0.0–1.0) positions on the floor-plan image; the frontend's
    // Leaflet `CRS.Simple` overlay derives pixel coordinates from the image's
    // natural dimensions at render time, so no DPI/scale metadata needs to be
    // stored here.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS locations (
            id               TEXT PRIMARY KEY,
            name             TEXT NOT NULL UNIQUE,
            description      TEXT,
            floor_plan_image TEXT,
            floor_plan_x     REAL,
            floor_plan_y     REAL,
            created_at       TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at       TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_locations_name ON locations(name);",
    )?;

    let _ = conn.execute("ALTER TABLE specimens ADD COLUMN location_id TEXT REFERENCES locations(id)", []);
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_specimens_location_id ON specimens(location_id)",
        [],
    )?;

    Ok(())
}

fn migration_041_ai_suggestions(conn: &Connection) -> DbResult<()> {
    // WP-56: Local AI analysis — draft suggestions requiring explicit approval.
    //
    // An AI suggestion is never written directly into a `notes` field. It is
    // stored here as a standalone draft with full attribution (which model,
    // which exact prompt) and a `status` gate; only `approve_ai_suggestion`
    // (commands::ai) copies its text into the real notes column, and that copy
    // goes through the normal update command's own audit-log write — so the
    // audit trail always shows a human-attributed update, never a synthetic
    // "AI wrote this" entry pretending to be unattended.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS ai_suggestions (
            id          TEXT PRIMARY KEY,
            entity_type TEXT NOT NULL CHECK (entity_type IN ('specimen','subculture','attachment')),
            entity_id   TEXT NOT NULL,
            kind        TEXT NOT NULL CHECK (kind IN ('summarize_notes','suggest_passage_comment','analyze_photo')),
            model_name  TEXT NOT NULL,
            prompt      TEXT NOT NULL,
            suggestion  TEXT NOT NULL,
            status      TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending','approved','rejected')),
            created_by  TEXT REFERENCES users(id),
            reviewed_by TEXT REFERENCES users(id),
            reviewed_at TEXT,
            created_at  TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_ai_suggestions_entity ON ai_suggestions(entity_type, entity_id);
        CREATE INDEX IF NOT EXISTS idx_ai_suggestions_status ON ai_suggestions(status);",
    )?;
    Ok(())
}

fn migration_042_backup_targets(conn: &Connection) -> DbResult<()> {
    // WP-59: Cloud backup & multi-device sync with end-to-end encryption.
    //
    // `config_encrypted` holds an AES-256-GCM-encrypted JSON blob (endpoint,
    // credentials, bucket/path) — the master key is derived from a
    // user-supplied passphrase via Argon2id and is never itself persisted to
    // disk (see src-tauri/src/cloud/crypto.rs). No CHECK constraint on `type`
    // so future target kinds can be added without another migration.
    //
    // `cloud_sync_segments` records which per-device WAL/audit segments
    // (identified by their chain_seq range, the audit chain's authoritative
    // ordering — see WP-51's sync_conflicts precedent) have already been
    // applied from a given peer for a given target, so `reconcile_cloud_sync`
    // never re-applies a segment it has already merged.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS backup_targets (
            id                     TEXT PRIMARY KEY,
            name                   TEXT NOT NULL,
            type                   TEXT NOT NULL,
            config_encrypted       TEXT NOT NULL,
            schedule_cron          TEXT,
            last_backup_at         TEXT,
            last_backup_size_bytes INTEGER,
            last_status            TEXT CHECK (last_status IN ('ok','failed','pending')),
            last_error             TEXT,
            is_enabled             INTEGER NOT NULL DEFAULT 1,
            created_at             TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_backup_targets_enabled ON backup_targets(is_enabled);

        CREATE TABLE IF NOT EXISTS cloud_sync_segments (
            id              TEXT PRIMARY KEY,
            target_id       TEXT NOT NULL REFERENCES backup_targets(id) ON DELETE CASCADE,
            device_id       TEXT NOT NULL,
            chain_seq_start INTEGER NOT NULL,
            chain_seq_end   INTEGER NOT NULL,
            applied_at      TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_cloud_sync_segments_target ON cloud_sync_segments(target_id, device_id);",
    )?;
    Ok(())
}

fn migration_043_reanchor_events(conn: &Connection) -> DbResult<()> {
    // WP-64: Taxon chain re-anchoring tool (WP-45 production-safety follow-up).
    //
    // A permanent, queryable record of every re-anchoring event, kept alongside
    // (never replacing) the audit hash chain itself — see
    // db::queries::reanchor_taxon_chain for how this bridges old and new
    // genesis entries after a taxon reclassification.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS reanchor_events (
            id                       TEXT PRIMARY KEY,
            taxon_id                 TEXT NOT NULL REFERENCES taxa(id),
            performed_by             TEXT NOT NULL,
            reason                   TEXT NOT NULL,
            affected_taxa_count      INTEGER NOT NULL DEFAULT 0,
            affected_species_count  INTEGER NOT NULL DEFAULT 0,
            affected_strains_count   INTEGER NOT NULL DEFAULT 0,
            affected_specimens_count INTEGER NOT NULL DEFAULT 0,
            created_at               TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_reanchor_events_taxon ON reanchor_events(taxon_id);",
    )?;
    Ok(())
}

fn migration_044_signing_keys(conn: &Connection) -> DbResult<()> {
    // WP-60: Regulatory compliance export modules — signing key storage.
    //
    // Single-row table (id = 1, like smtp_config) holding the lab's Ed25519
    // signing keypair used to sign FDA 21 CFR Part 11 attestation bundles.
    // Ed25519 is used in place of the RSA-4096 originally sketched in the
    // ROADMAP — a deliberate, documented deviation (see ROADMAP.md WP-60
    // "As built"): it gives the same sign/verify/public-certificate guarantee
    // with a far smaller, audited pure-Rust dependency and no PEM/ASN.1
    // certificate-authority machinery, which is unnecessary for a
    // self-attested lab signature that an inspector verifies against the
    // bundled public key directly (not a certificate chain).
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS signing_keys (
            id              INTEGER PRIMARY KEY CHECK (id = 1),
            public_key_b64  TEXT NOT NULL,
            private_key_b64 TEXT NOT NULL,
            created_at      TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )?;
    Ok(())
}

fn migration_045_installed_plugins(conn: &Connection) -> DbResult<()> {
    // WP-61: Plugin / extension system for new verticals.
    //
    // Tracks installed plugins in the database (rather than relying solely on
    // a filesystem scan) so install/uninstall state survives independently of
    // the plugins/ directory and can be listed without re-parsing every
    // manifest on every app start. `vocabulary_seeded` records whether this
    // plugin's vocabulary rows have already been applied — seeding is
    // idempotent-by-design (INSERT OR IGNORE) but this flag lets the UI show
    // accurate status without re-running the seed step every launch.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS installed_plugins (
            id                TEXT PRIMARY KEY,
            plugin_name       TEXT NOT NULL,
            version           TEXT NOT NULL,
            profile           TEXT,
            manifest_json     TEXT NOT NULL,
            vocabulary_seeded INTEGER NOT NULL DEFAULT 0,
            installed_at      TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE (plugin_name)
        );",
    )?;
    Ok(())
}

fn migration_035_multiuser_foundation(conn: &Connection) -> DbResult<()> {
    // WP-50/WP-51: Multi-user backend + LAN sync foundation, implemented together
    // for architectural coherence (see ROADMAP.md Phase F).
    //
    // WP-50: records the lab's intended database backend ('sqlite' | 'postgres')
    // in the existing app_settings key/value table. SQLite remains the only
    // backend actually wired into AppState/the query layer — this setting is
    // forward-looking metadata for a future full backend switch, not a live
    // toggle. Deliberately does NOT store a connection string here: connection
    // strings may embed credentials and must never be persisted in plaintext
    // (consistent with the zero-knowledge posture already established for
    // WP-59 cloud backup targets). Callers supply the connection string fresh
    // on each `test_postgres_connection` / `bootstrap_postgres_schema` call.
    //
    // WP-51: two new tables provide the data-model foundation for LAN sync
    // change-detection, reusing the existing hash-chain columns
    // (lineage_id, chain_seq, entry_hash) already on audit_log rather than
    // introducing a parallel change-tracking mechanism.
    //   - sync_peers: known LAN peer devices. Populated by a future discovery
    //     layer; for now, peers are registered explicitly by an admin.
    //   - sync_conflicts: durable record of any (lineage_id, chain_seq) position
    //     where a local entry and an incoming entry disagree on entry_hash — a
    //     genuine fork that must never be silently discarded or auto-merged.
    conn.execute(
        "INSERT OR IGNORE INTO app_settings (key, value) VALUES ('backend_type', 'sqlite')",
        [],
    )?;

    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS sync_peers (
            id            TEXT PRIMARY KEY,
            device_id     TEXT NOT NULL UNIQUE,
            device_name   TEXT NOT NULL,
            last_seen_at  TEXT,
            last_sync_at  TEXT,
            created_at    TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_sync_peers_device ON sync_peers(device_id);

        CREATE TABLE IF NOT EXISTS sync_conflicts (
            id                        TEXT PRIMARY KEY,
            lineage_id                TEXT NOT NULL,
            chain_seq                 INTEGER NOT NULL,
            local_entry_hash          TEXT,
            incoming_entry_hash       TEXT,
            incoming_source_device_id TEXT,
            reason                    TEXT NOT NULL,
            resolved                  INTEGER NOT NULL DEFAULT 0,
            resolved_by               TEXT,
            resolved_at               TEXT,
            detected_at               TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_sync_conflicts_lineage  ON sync_conflicts(lineage_id, chain_seq);
        CREATE INDEX IF NOT EXISTS idx_sync_conflicts_resolved ON sync_conflicts(resolved);
    ")?;

    Ok(())
}

fn migration_034_provisional_taxa(conn: &Connection) -> DbResult<()> {
    // WP-49: Custom/provisional taxa and Darwin Core export support.
    //
    // Adds a `status` column to `taxa` (default 'accepted'; no CHECK constraint so
    // future statuses can be added without another migration).  Also adds a
    // `provisional_notes` TEXT column for lab-internal commentary.
    //
    // Creates a `taxon_mappings` table linking provisional taxa to their accepted
    // NCBI taxa once published.  The link is advisory only — it does not affect
    // the main taxa hierarchy or any hash-chain entries.

    // Ignore DUPLICATE_COLUMN errors in case this migration is re-run against a
    // database that already has the column (defensive idempotency).
    let _ = conn.execute(
        "ALTER TABLE taxa ADD COLUMN status TEXT NOT NULL DEFAULT 'accepted'",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE taxa ADD COLUMN provisional_notes TEXT",
        [],
    );

    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS taxon_mappings (
            id                  TEXT PRIMARY KEY,
            provisional_taxon_id TEXT NOT NULL REFERENCES taxa(id) ON DELETE CASCADE,
            accepted_taxon_id    TEXT REFERENCES taxa(id) ON DELETE SET NULL,
            accepted_ncbi_id     INTEGER,
            accepted_name        TEXT,
            notes                TEXT,
            mapped_by            TEXT,
            mapped_at            TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_taxon_mappings_provisional ON taxon_mappings(provisional_taxon_id);
        CREATE INDEX IF NOT EXISTS idx_taxon_mappings_accepted    ON taxon_mappings(accepted_taxon_id);
    ")?;

    Ok(())
}

fn migration_033_breeding_programs(conn: &Connection) -> DbResult<()> {
    // WP-47: Breeding programs and multi-generational selection tracking.
    //
    // `breeding_programs` is the top-level container: a user-named program with a goal,
    // start date, free-text target traits, and an optional JSON array of founder strain IDs.
    //
    // `breeding_records` links a strain to a program and captures a selection event:
    // generation number, selection notes, a numeric fitness score, and selection date.
    // Deleting a program cascades to its records; the strain FK does not cascade so
    // strains remain intact if a program is removed.
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS breeding_programs (
            id                 TEXT    PRIMARY KEY,
            name               TEXT    NOT NULL,
            goal               TEXT,
            start_date         TEXT,
            target_traits      TEXT,
            founder_strain_ids TEXT,
            notes              TEXT,
            created_at         TEXT    NOT NULL DEFAULT (datetime('now')),
            created_by         TEXT
        );

        CREATE TABLE IF NOT EXISTS breeding_records (
            id                TEXT    PRIMARY KEY,
            program_id        TEXT    NOT NULL REFERENCES breeding_programs(id) ON DELETE CASCADE,
            strain_id         TEXT    NOT NULL REFERENCES strains(id),
            generation_number INTEGER NOT NULL DEFAULT 1,
            selection_notes   TEXT,
            fitness_score     REAL,
            selection_date    TEXT,
            selected_by       TEXT,
            notes             TEXT,
            created_at        TEXT    NOT NULL DEFAULT (datetime('now'))
        );

        CREATE INDEX IF NOT EXISTS idx_breeding_records_program_id
            ON breeding_records(program_id);
        CREATE INDEX IF NOT EXISTS idx_breeding_records_strain_id
            ON breeding_records(strain_id);
    ")?;
    Ok(())
}

fn migration_032_domain_column(conn: &Connection) -> DbResult<()> {
    // WP-46: Add a `domain` TEXT column to app_config that records the biological
    // domain (kingdom-level grouping) for the active lab profile.
    //
    // Known mappings:
    //   plant_tissue_culture → 'Plantae'
    //   cell_culture         → 'Animalia'
    //   mycology             → 'Fungi'
    //
    // No CHECK constraint is added so that future domains (e.g. 'Bacteria', 'Archaea')
    // can be stored without another schema migration.
    //
    // The UPDATE assigns the domain for every known profile; rows with an
    // unrecognised lab_profile fall back to 'Plantae' for safety.
    conn.execute_batch("
        ALTER TABLE app_config ADD COLUMN domain TEXT NOT NULL DEFAULT 'Plantae';
        UPDATE app_config SET domain = CASE lab_profile
            WHEN 'plant_tissue_culture' THEN 'Plantae'
            WHEN 'cell_culture'         THEN 'Animalia'
            WHEN 'mycology'             THEN 'Fungi'
            ELSE 'Plantae'
        END
        WHERE id = 1;
    ")?;
    Ok(())
}

fn migration_031_taxon_hash_chain(conn: &Connection) -> DbResult<()> {
    // WP-45: EXPERIMENTAL — full taxonomic hash chain (Kingdom → Strain → Specimen).
    //
    // No schema changes are required: the `audit_log` table already carries the
    // necessary hash chain columns (`chain_seq`, `prev_hash`, `entry_hash`, `lineage_id`).
    // This migration backfills genesis audit entries for all existing `taxa` rows so that
    // the chain Kingdom → Phylum → Class → Order → Family → Genus is established for
    // any data that pre-dates this migration.
    //
    // Safe to re-run: `backfill_taxa_genesis` skips taxa that already have a genesis entry.
    //
    // RECLASSIFICATION WARNING: Once a taxon has a genesis entry, reclassifying it (renaming,
    // re-parenting, or changing rank) will NOT automatically re-anchor descendant chains.
    // All strains and specimens whose genesis prev_hash was derived from this taxon's
    // entry_hash will remain bound to the original classification. There is currently no
    // automated re-anchoring tool. This is an OPTIONAL/EXPERIMENTAL feature — see ROADMAP.md §WP-45.
    backfill_taxa_genesis(conn)
}

/// EXPERIMENTAL (WP-45): Write genesis audit entries for all existing taxa that do not
/// yet have one, processing ranks from kingdom down to genus so each child taxon can
/// inherit the parent's already-written entry_hash.
///
/// Idempotent: taxa with a pre-existing genesis entry (entity_type = 'taxon', chain_seq = 0)
/// are skipped.
///
/// The hash chain columns are on `audit_log`; the `taxa` table itself is unchanged.
pub fn backfill_taxa_genesis(conn: &Connection) -> DbResult<()> {
    // Rank order guarantees parents are seeded before children.
    let rank_order = ["kingdom", "phylum", "class", "order", "family", "genus"];

    for rank in &rank_order {
        // Collect taxa at this rank that lack a genesis audit entry.
        let taxa: Vec<(String, Option<String>)> = {
            let mut stmt = conn.prepare(
                "SELECT id, parent_id FROM taxa \
                 WHERE rank = ?1 \
                 AND id NOT IN ( \
                     SELECT entity_id FROM audit_log \
                     WHERE entity_id IS NOT NULL \
                       AND entity_type = 'taxon' \
                       AND chain_seq = 0 \
                 )",
            )?;
            let rows = stmt.query_map(rusqlite::params![rank], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
            })?;
            rows.filter_map(|r| r.ok()).collect()
        };

        for (taxon_id, parent_id) in taxa {
            super::queries::log_audit_taxon_genesis(
                conn,
                None,
                "create",
                "taxon",
                Some(&taxon_id),
                None,
                None,
                None,
                parent_id.as_deref(),
            )?;
        }
    }
    Ok(())
}

fn migration_030_fruiting_records(conn: &Connection) -> DbResult<()> {
    // WP-43: fruiting conditions and yield tracking for mycology cultures.
    // A dedicated `fruiting_records` table keeps harvest events separate from
    // the general subculture log so mycology-specific fields don't pollute
    // the passage record for other profiles.
    conn.execute_batch(
        "CREATE TABLE fruiting_records (
             id                  TEXT PRIMARY KEY,
             specimen_id         TEXT NOT NULL REFERENCES specimens(id),
             flush_number        INTEGER NOT NULL DEFAULT 1,
             harvest_date        TEXT NOT NULL,
             fresh_weight_g      REAL,
             dry_weight_g        REAL,
             fruiting_temp_c     REAL,
             fruiting_rh_percent REAL,
             fae_rate            REAL,
             light_hours_per_day REAL,
             notes               TEXT,
             created_by          TEXT,
             created_at          TEXT NOT NULL DEFAULT (datetime('now')),
             updated_at          TEXT NOT NULL DEFAULT (datetime('now'))
         );
         CREATE INDEX idx_fruiting_records_specimen_id
             ON fruiting_records(specimen_id);",
    )?;
    Ok(())
}

fn migration_029_genetic_lineage_markers(conn: &Connection) -> DbResult<()> {
    // WP-42: genetic lineage & strain isolation markers on specimens.
    // origin_type: tracks whether a culture was started from multi-spore inoculation,
    //   an isolated dikaryon (single pairing), or a vegetative tissue clone.
    //   NULL means not specified or not applicable (non-mycology profiles).
    //   A CHECK constraint enforces the allowed vocabulary.
    // is_best_performer: lightweight selection flag (0/1) for strain selection workflows.
    //   Defaults to 0 for all existing rows.
    //   On split, children inherit origin_type but is_best_performer resets to 0.
    conn.execute_batch(
        "ALTER TABLE specimens ADD COLUMN origin_type TEXT \
             CHECK(origin_type IS NULL \
                   OR origin_type IN ('multi_spore','isolated_dikaryon','tissue_clone'));
         ALTER TABLE specimens ADD COLUMN is_best_performer INTEGER NOT NULL DEFAULT 0;",
    )?;
    Ok(())
}

fn migration_028_colonization_contaminant(conn: &Connection) -> DbResult<()> {
    // WP-41: colonization progress and typed contaminant support.
    // colonization_pct: percentage (0-100) of substrate colonized by mycelium.
    // contaminant_type: categorical label for the contaminant (wet rot, trich, cobweb, etc.).
    // Both columns are nullable so existing rows are unaffected.
    conn.execute_batch(
        "ALTER TABLE subcultures ADD COLUMN colonization_pct REAL
             CHECK(colonization_pct IS NULL OR (colonization_pct >= 0.0 AND colonization_pct <= 100.0));
         ALTER TABLE subcultures ADD COLUMN contaminant_type TEXT;",
    )?;
    Ok(())
}

fn migration_027_mycology_vocabulary(conn: &Connection) -> DbResult<()> {
    // WP-40: seed vocabulary for the mycology profile.
    // All six lookup tables already exist from migrations 016/017.
    // This migration is purely additive — INSERT OR IGNORE keeps it idempotent
    // and leaves all plant_tissue_culture and cell_culture rows untouched.
    conn.execute_batch("
        BEGIN;

        -- Mushroom/fungal cultivation lifecycle stages.
        -- contaminated and discarded are terminal; all others are selectable.
        INSERT OR IGNORE INTO stages (profile, code, label, sort_order, is_terminal) VALUES
            ('mycology', 'spore_clone',    'Spore / Clone',    1,  0),
            ('mycology', 'agar',           'Agar Culture',     2,  0),
            ('mycology', 'liquid_culture', 'Liquid Culture',   3,  0),
            ('mycology', 'grain_spawn',    'Grain Spawn',      4,  0),
            ('mycology', 'bulk_substrate', 'Bulk Substrate',   5,  0),
            ('mycology', 'colonizing',     'Colonizing',       6,  0),
            ('mycology', 'fruiting',       'Fruiting',         7,  0),
            ('mycology', 'senescent',      'Senescent',        8,  0),
            ('mycology', 'contaminated',   'Contaminated',     9,  1),
            ('mycology', 'discarded',      'Discarded',        10, 1);

        -- Common transfer/inoculation methods used in fungal cultivation.
        INSERT OR IGNORE INTO propagation_methods (profile, code, label, sort_order) VALUES
            ('mycology', 'agar_to_agar',       'Agar to Agar',       1),
            ('mycology', 'agar_to_grain',      'Agar to Grain',      2),
            ('mycology', 'grain_to_grain',     'Grain to Grain',     3),
            ('mycology', 'grain_to_bulk',      'Grain to Bulk',      4),
            ('mycology', 'liquid_inoculation', 'Liquid Inoculation', 5),
            ('mycology', 'spore_syringe',      'Spore Syringe',      6),
            ('mycology', 'culture_restart',    'Culture Restart',    7),
            ('mycology', 'other',              'Other',              8);

        -- Substrate supplements (reusing hormone_types table for supplement tracking).
        INSERT OR IGNORE INTO hormone_types (profile, code, label, sort_order) VALUES
            ('mycology', 'gypsum',            'Gypsum',             1),
            ('mycology', 'bran',              'Bran (wheat/oat)',   2),
            ('mycology', 'calcium_carbonate', 'Calcium Carbonate',  3),
            ('mycology', 'activated_carbon',  'Activated Carbon',   4),
            ('mycology', 'coconut_coir',      'Coconut Coir',       5),
            ('mycology', 'vermiculite',       'Vermiculite',        6),
            ('mycology', 'other',             'Other',              7);

        -- Compliance record types relevant to mushroom cultivation.
        INSERT OR IGNORE INTO compliance_record_types (profile, code, label, sort_order) VALUES
            ('mycology', 'cultivation_permit',  'Cultivation Permit',   1),
            ('mycology', 'grow_log',            'Grow Log',             2),
            ('mycology', 'contamination_record','Contamination Record', 3),
            ('mycology', 'species_id',          'Species ID Report',    4),
            ('mycology', 'mushroom_permit',     'Mushroom/Fungi Permit',5),
            ('mycology', 'other',               'Other',                6);

        -- Agencies that may have jurisdiction over fungal cultivation operations.
        INSERT OR IGNORE INTO compliance_agencies (profile, code, label, sort_order) VALUES
            ('mycology', 'USDA_APHIS',     'USDA APHIS',            1),
            ('mycology', 'state_ag_dept',  'State Dept. of Ag.',    2),
            ('mycology', 'local_authority','Local Authority',        3),
            ('mycology', 'other',          'Other',                  4);

        -- Inventory categories for a mushroom / fungal cultivation lab.
        INSERT OR IGNORE INTO inventory_categories (profile, code, label, sort_order) VALUES
            ('mycology', 'agar_media',         'Agar Media',           1),
            ('mycology', 'grain_spawn',        'Grain Spawn',          2),
            ('mycology', 'bulk_substrate',     'Bulk Substrate',       3),
            ('mycology', 'liquid_culture',     'Liquid Culture',       4),
            ('mycology', 'substrate_amendment','Substrate Amendment',  5),
            ('mycology', 'syringes_needles',   'Syringes & Needles',   6),
            ('mycology', 'vessel',             'Vessel / Container',   7),
            ('mycology', 'consumable',         'Consumable',           8),
            ('mycology', 'equipment',          'Equipment',            9),
            ('mycology', 'other',              'Other',                10);

        COMMIT;
    ")?;
    Ok(())
}

fn migration_026_biosafety_level(conn: &Connection) -> DbResult<()> {
    // WP-33: mycoplasma & contamination testing compliance.
    // Adds a nullable biosafety_level column to specimens so that cell culture
    // lines can be classified as BSL-1 through BSL-3.  A CHECK constraint
    // enforces the allowed values; NULL means "not classified".
    conn.execute_batch(
        "ALTER TABLE specimens ADD COLUMN biosafety_level TEXT
         CHECK(biosafety_level IN ('BSL-1','BSL-2','BSL-2+','BSL-3'));",
    )?;
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

    // ── migration 026 (WP-33) ─────────────────────────────────────────────────

    #[test]
    fn biosafety_level_column_exists_after_migration_026() {
        let conn = migrated_db();
        conn.execute(
            "INSERT INTO species (id, genus, species_name, species_code) \
             VALUES ('sp1', 'Homo', 'sapiens', 'HEK')",
            [],
        )
        .unwrap();
        let now = "2026-01-01";
        conn.execute(
            "INSERT INTO specimens \
             (id, accession_number, species_id, stage, initiation_date, \
              quarantine_flag, ip_flag, subculture_count, is_archived, contamination_flag, \
              generation, lineage_passage_offset, biosafety_level, created_at, updated_at) \
             VALUES ('s1', '2026-01-01-HEK-001', 'sp1', 'culture', ?1, \
                     0, 0, 0, 0, 0, 0, 0, 'BSL-2', ?1, ?1)",
            [now],
        )
        .expect("biosafety_level column must accept valid BSL value after migration 026");
        let bsl: Option<String> = conn
            .query_row(
                "SELECT biosafety_level FROM specimens WHERE id = 's1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(bsl.as_deref(), Some("BSL-2"));
    }

    #[test]
    fn biosafety_level_rejects_invalid_value() {
        let conn = migrated_db();
        conn.execute(
            "INSERT INTO species (id, genus, species_name, species_code) \
             VALUES ('sp1', 'Homo', 'sapiens', 'HEK')",
            [],
        )
        .unwrap();
        let now = "2026-01-01";
        let result = conn.execute(
            "INSERT INTO specimens \
             (id, accession_number, species_id, stage, initiation_date, \
              quarantine_flag, ip_flag, subculture_count, is_archived, contamination_flag, \
              generation, lineage_passage_offset, biosafety_level, created_at, updated_at) \
             VALUES ('s2', '2026-01-01-HEK-002', 'sp1', 'culture', ?1, \
                     0, 0, 0, 0, 0, 0, 0, 'BSL-99', ?1, ?1)",
            [now],
        );
        assert!(result.is_err(), "invalid biosafety_level must be rejected by CHECK constraint");
    }

    #[test]
    fn biosafety_level_defaults_to_null() {
        let conn = migrated_db();
        conn.execute(
            "INSERT INTO species (id, genus, species_name, species_code) \
             VALUES ('sp1', 'Homo', 'sapiens', 'HEK')",
            [],
        )
        .unwrap();
        let now = "2026-01-01";
        conn.execute(
            "INSERT INTO specimens \
             (id, accession_number, species_id, stage, initiation_date, \
              quarantine_flag, ip_flag, subculture_count, is_archived, contamination_flag, \
              generation, lineage_passage_offset, created_at, updated_at) \
             VALUES ('s3', '2026-01-01-HEK-003', 'sp1', 'culture', ?1, \
                     0, 0, 0, 0, 0, 0, 0, ?1, ?1)",
            [now],
        )
        .unwrap();
        let bsl: Option<String> = conn
            .query_row(
                "SELECT biosafety_level FROM specimens WHERE id = 's3'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(bsl.is_none(), "biosafety_level must default to NULL for existing specimens");
    }

    // ── migration 027 (WP-40) — mycology vocabulary ───────────────────────────

    #[test]
    fn mycology_stages_seeded() {
        let conn = migrated_db();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM stages WHERE profile = 'mycology'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 10, "expected 10 mycology stage entries from seed data");
    }

    #[test]
    fn mycology_terminal_stages_are_contaminated_and_discarded() {
        let conn = migrated_db();
        let mut terminal: Vec<String> = {
            let mut stmt = conn
                .prepare(
                    "SELECT code FROM stages \
                     WHERE profile = 'mycology' AND is_terminal = 1 \
                     ORDER BY code",
                )
                .unwrap();
            stmt.query_map([], |r| r.get(0))
                .unwrap()
                .filter_map(|r| r.ok())
                .collect()
        };
        terminal.sort();
        assert_eq!(
            terminal,
            vec!["contaminated", "discarded"],
            "mycology terminal stages must be contaminated and discarded"
        );
    }

    #[test]
    fn mycology_non_terminal_stages_count() {
        let conn = migrated_db();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM stages \
                 WHERE profile = 'mycology' AND is_terminal = 0",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 8, "8 non-terminal mycology stages must be present");
    }

    #[test]
    fn mycology_stages_expected_codes_present() {
        let conn = migrated_db();
        for code in &[
            "spore_clone", "agar", "liquid_culture", "grain_spawn",
            "bulk_substrate", "colonizing", "fruiting", "senescent",
            "contaminated", "discarded",
        ] {
            let found: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM stages WHERE profile = 'mycology' AND code = ?1",
                    rusqlite::params![code],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(found, 1, "mycology stage '{}' must be present", code);
        }
    }

    #[test]
    fn mycology_propagation_methods_seeded() {
        let conn = migrated_db();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM propagation_methods WHERE profile = 'mycology'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 8, "expected 8 mycology propagation methods");
    }

    #[test]
    fn mycology_propagation_methods_expected_codes_present() {
        let conn = migrated_db();
        for code in &[
            "agar_to_agar", "agar_to_grain", "grain_to_grain", "grain_to_bulk",
            "liquid_inoculation", "spore_syringe", "culture_restart", "other",
        ] {
            let found: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM propagation_methods \
                     WHERE profile = 'mycology' AND code = ?1",
                    rusqlite::params![code],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(found, 1, "mycology propagation method '{}' must be present", code);
        }
    }

    #[test]
    fn mycology_hormone_types_seeded() {
        let conn = migrated_db();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM hormone_types WHERE profile = 'mycology'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 7, "expected 7 mycology supplement types");
    }

    #[test]
    fn mycology_compliance_record_types_seeded() {
        let conn = migrated_db();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM compliance_record_types WHERE profile = 'mycology'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 6, "expected 6 mycology compliance record types");
    }

    #[test]
    fn mycology_compliance_agencies_seeded() {
        let conn = migrated_db();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM compliance_agencies WHERE profile = 'mycology'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 4, "expected 4 mycology compliance agencies");
    }

    #[test]
    fn mycology_inventory_categories_seeded() {
        let conn = migrated_db();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM inventory_categories WHERE profile = 'mycology'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 10, "expected 10 mycology inventory categories");
    }

    #[test]
    fn mycology_vocabulary_does_not_affect_ptc_or_cell_culture() {
        let conn = migrated_db();
        let ptc_stages: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM stages WHERE profile = 'plant_tissue_culture'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(ptc_stages, 15,
            "PTC stage count must be unchanged after mycology seeding");

        let cc_stages: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM stages WHERE profile = 'cell_culture'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(cc_stages, 20,
            "cell_culture stage count must be unchanged after mycology seeding");

        let ptc_props: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM propagation_methods \
                 WHERE profile = 'plant_tissue_culture'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(ptc_props, 7,
            "PTC propagation method count must be unchanged after mycology seeding");

        let cc_props: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM propagation_methods WHERE profile = 'cell_culture'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(cc_props, 11,
            "cell_culture propagation method count must be unchanged after mycology seeding");
    }

    #[test]
    fn mycology_vocabulary_027_idempotent() {
        let conn = migrated_db();
        migration_027_mycology_vocabulary(&conn)
            .expect("re-running migration 027 must succeed");
        let stage_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM stages WHERE profile = 'mycology'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(stage_count, 10, "re-run must not duplicate mycology stage rows");
    }

    // ── Migration 028 tests ────────────────────────────────────────────────

    fn seed_species_028(conn: &Connection) {
        conn.execute(
            "INSERT INTO species (id, genus, species_name, species_code) \
             VALUES ('sp028-species', 'Pleurotus', 'ostreatus', 'PLEU-OS')",
            [],
        )
        .unwrap();
    }

    #[test]
    fn migration_028_adds_colonization_pct_column() {
        let conn = migrated_db();
        seed_species_028(&conn);
        conn.execute_batch(
            "INSERT INTO specimens (id, accession_number, species_id, stage, initiation_date,
                generation, lineage_passage_offset, subculture_count, is_archived,
                contamination_flag, ip_flag, quarantine_flag, created_at, updated_at)
             VALUES ('sp-028', 'ACC-028', 'sp028-species', 'colonizing', '2026-01-01',
                0, 0, 0, 0, 0, 0, 0, '2026-01-01', '2026-01-01');
             INSERT INTO subcultures (id, specimen_id, passage_number, date, colonization_pct,
                created_at, updated_at, event_type)
             VALUES ('sc-028', 'sp-028', 1, '2026-01-01', 55.0,
                '2026-01-01', '2026-01-01', 'passage');",
        )
        .expect("inserting subculture with colonization_pct must succeed after migration 028");
        let pct: f64 = conn
            .query_row(
                "SELECT colonization_pct FROM subcultures WHERE id = 'sc-028'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!((pct - 55.0).abs() < f64::EPSILON);
    }

    #[test]
    fn migration_028_colonization_pct_check_rejects_out_of_range() {
        let conn = migrated_db();
        seed_species_028(&conn);
        conn.execute_batch(
            "INSERT INTO specimens (id, accession_number, species_id, stage, initiation_date,
                generation, lineage_passage_offset, subculture_count, is_archived,
                contamination_flag, ip_flag, quarantine_flag, created_at, updated_at)
             VALUES ('sp-028b', 'ACC-028B', 'sp028-species', 'colonizing', '2026-01-01',
                0, 0, 0, 0, 0, 0, 0, '2026-01-01', '2026-01-01');",
        )
        .unwrap();
        let result = conn.execute(
            "INSERT INTO subcultures (id, specimen_id, passage_number, date,
                colonization_pct, created_at, updated_at, event_type)
             VALUES ('sc-028b', 'sp-028b', 1, '2026-01-01',
                150.0, '2026-01-01', '2026-01-01', 'passage')",
            [],
        );
        assert!(result.is_err(), "colonization_pct > 100 must be rejected by CHECK constraint");
    }

    #[test]
    fn migration_028_adds_contaminant_type_column() {
        let conn = migrated_db();
        seed_species_028(&conn);
        conn.execute_batch(
            "INSERT INTO specimens (id, accession_number, species_id, stage, initiation_date,
                generation, lineage_passage_offset, subculture_count, is_archived,
                contamination_flag, ip_flag, quarantine_flag, created_at, updated_at)
             VALUES ('sp-028c', 'ACC-028C', 'sp028-species', 'colonizing', '2026-01-01',
                0, 0, 0, 0, 0, 0, 0, '2026-01-01', '2026-01-01');
             INSERT INTO subcultures (id, specimen_id, passage_number, date,
                contamination_flag, contaminant_type, created_at, updated_at, event_type)
             VALUES ('sc-028c', 'sp-028c', 1, '2026-01-01',
                1, 'trich', '2026-01-01', '2026-01-01', 'passage');",
        )
        .expect("inserting subculture with contaminant_type must succeed after migration 028");
        let ct: String = conn
            .query_row(
                "SELECT contaminant_type FROM subcultures WHERE id = 'sc-028c'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(ct, "trich");
    }

    #[test]
    fn migration_028_existing_rows_get_null_new_columns() {
        let conn = migrated_db();
        seed_species_028(&conn);
        conn.execute_batch(
            "INSERT INTO specimens (id, accession_number, species_id, stage, initiation_date,
                generation, lineage_passage_offset, subculture_count, is_archived,
                contamination_flag, ip_flag, quarantine_flag, created_at, updated_at)
             VALUES ('sp-028d', 'ACC-028D', 'sp028-species', 'colonizing', '2026-01-01',
                0, 0, 0, 0, 0, 0, 0, '2026-01-01', '2026-01-01');
             INSERT INTO subcultures (id, specimen_id, passage_number, date,
                created_at, updated_at, event_type)
             VALUES ('sc-028d', 'sp-028d', 1, '2026-01-01',
                '2026-01-01', '2026-01-01', 'passage');",
        )
        .unwrap();
        let (cpct, ctype): (Option<f64>, Option<String>) = conn
            .query_row(
                "SELECT colonization_pct, contaminant_type FROM subcultures WHERE id = 'sc-028d'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert!(cpct.is_none(), "colonization_pct must default to NULL");
        assert!(ctype.is_none(), "contaminant_type must default to NULL");
    }

    // ── Migration 029 tests ────────────────────────────────────────────────

    #[test]
    fn migration_029_origin_type_column_exists_and_accepts_valid_values() {
        let conn = migrated_db();
        seed_species_028(&conn);
        for ot in &["multi_spore", "isolated_dikaryon", "tissue_clone"] {
            conn.execute(
                "INSERT INTO specimens (id, accession_number, species_id, stage, initiation_date,
                    generation, lineage_passage_offset, subculture_count, is_archived,
                    contamination_flag, ip_flag, quarantine_flag, created_at, updated_at, origin_type)
                 VALUES (?1, ?2, 'sp028-species', 'colonizing', '2026-01-01',
                    0, 0, 0, 0, 0, 0, 0, '2026-01-01', '2026-01-01', ?3)",
                rusqlite::params![format!("sp-029-{}", ot), format!("ACC-029-{}", ot), ot],
            )
            .unwrap_or_else(|e| panic!("origin_type '{}' must be accepted: {}", ot, e));
        }
    }

    #[test]
    fn migration_029_origin_type_check_rejects_invalid_value() {
        let conn = migrated_db();
        seed_species_028(&conn);
        let result = conn.execute(
            "INSERT INTO specimens (id, accession_number, species_id, stage, initiation_date,
                generation, lineage_passage_offset, subculture_count, is_archived,
                contamination_flag, ip_flag, quarantine_flag, created_at, updated_at, origin_type)
             VALUES ('sp-029-bad', 'ACC-029-BAD', 'sp028-species', 'colonizing', '2026-01-01',
                0, 0, 0, 0, 0, 0, 0, '2026-01-01', '2026-01-01', 'invalid_value')",
            [],
        );
        assert!(result.is_err(), "origin_type CHECK must reject 'invalid_value'");
    }

    #[test]
    fn migration_029_is_best_performer_defaults_to_zero() {
        let conn = migrated_db();
        seed_species_028(&conn);
        conn.execute(
            "INSERT INTO specimens (id, accession_number, species_id, stage, initiation_date,
                generation, lineage_passage_offset, subculture_count, is_archived,
                contamination_flag, ip_flag, quarantine_flag, created_at, updated_at)
             VALUES ('sp-029-perf', 'ACC-029-PERF', 'sp028-species', 'colonizing', '2026-01-01',
                0, 0, 0, 0, 0, 0, 0, '2026-01-01', '2026-01-01')",
            [],
        )
        .unwrap();
        let flag: i32 = conn
            .query_row(
                "SELECT is_best_performer FROM specimens WHERE id = 'sp-029-perf'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(flag, 0, "is_best_performer must default to 0");
    }

    #[test]
    fn migration_029_is_best_performer_can_be_set_to_one() {
        let conn = migrated_db();
        seed_species_028(&conn);
        conn.execute(
            "INSERT INTO specimens (id, accession_number, species_id, stage, initiation_date,
                generation, lineage_passage_offset, subculture_count, is_archived,
                contamination_flag, ip_flag, quarantine_flag, created_at, updated_at,
                is_best_performer)
             VALUES ('sp-029-star', 'ACC-029-STAR', 'sp028-species', 'colonizing', '2026-01-01',
                0, 0, 0, 0, 0, 0, 0, '2026-01-01', '2026-01-01', 1)",
            [],
        )
        .unwrap();
        let flag: i32 = conn
            .query_row(
                "SELECT is_best_performer FROM specimens WHERE id = 'sp-029-star'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(flag, 1, "is_best_performer must be storable as 1");
    }

    #[test]
    fn migration_029_origin_type_nullable_for_non_mycology() {
        let conn = migrated_db();
        seed_species_028(&conn);
        conn.execute(
            "INSERT INTO specimens (id, accession_number, species_id, stage, initiation_date,
                generation, lineage_passage_offset, subculture_count, is_archived,
                contamination_flag, ip_flag, quarantine_flag, created_at, updated_at)
             VALUES ('sp-029-null', 'ACC-029-NULL', 'sp028-species', 'explant', '2026-01-01',
                0, 0, 0, 0, 0, 0, 0, '2026-01-01', '2026-01-01')",
            [],
        )
        .expect("origin_type must default to NULL for non-mycology specimens");
        let ot: Option<String> = conn
            .query_row(
                "SELECT origin_type FROM specimens WHERE id = 'sp-029-null'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(ot.is_none(), "origin_type must be NULL when not specified");
    }

    // ── Migration 030: fruiting_records ──────────────────────────────────────

    #[test]
    fn migration_030_fruiting_records_table_exists() {
        let conn = migrated_db();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='fruiting_records'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "fruiting_records table must exist after migration 030");
    }

    #[test]
    fn migration_030_fruiting_records_index_exists() {
        let conn = migrated_db();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' \
                 AND name='idx_fruiting_records_specimen_id'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "index on fruiting_records.specimen_id must exist");
    }

    #[test]
    fn migration_030_fruiting_record_fk_rejects_unknown_specimen() {
        let conn = migrated_db();
        conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();
        let result = conn.execute(
            "INSERT INTO fruiting_records (id, specimen_id, flush_number, harvest_date) \
             VALUES ('fr1','does-not-exist',1,'2026-06-01')",
            [],
        );
        assert!(result.is_err(), "FK constraint must reject unknown specimen_id");
    }

    #[test]
    fn migration_030_fruiting_record_flush_number_defaults_to_1() {
        let conn = migrated_db();
        seed_species_028(&conn);
        conn.execute(
            "INSERT INTO specimens (id, accession_number, species_id, initiation_date) \
             VALUES ('spec-030','ACC-030','sp028-species','2026-01-01')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO fruiting_records (id, specimen_id, harvest_date) \
             VALUES ('fr-030','spec-030','2026-06-01')",
            [],
        )
        .unwrap();
        let flush: i32 = conn
            .query_row(
                "SELECT flush_number FROM fruiting_records WHERE id = 'fr-030'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(flush, 1, "flush_number must default to 1");
    }

    // ── Migration 032: domain column ─────────────────────────────────────────

    #[test]
    fn migration_032_domain_column_exists() {
        let conn = migrated_db();
        let domain: String = conn
            .query_row("SELECT domain FROM app_config WHERE id = 1", [], |r| r.get(0))
            .expect("domain column must exist in app_config after migration 032");
        assert!(!domain.is_empty(), "domain must not be empty");
    }

    #[test]
    fn migration_032_plant_tissue_culture_maps_to_plantae() {
        let conn = migrated_db();
        // Default lab_profile is 'plant_tissue_culture' — domain must be 'Plantae'.
        let domain: String = conn
            .query_row("SELECT domain FROM app_config WHERE id = 1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(domain, "Plantae");
    }

    #[test]
    fn migration_032_cell_culture_maps_to_animalia() {
        let conn = migrated_db();
        conn.execute(
            "UPDATE app_config SET lab_profile = 'cell_culture', domain = 'Animalia' WHERE id = 1",
            [],
        )
        .unwrap();
        let domain: String = conn
            .query_row("SELECT domain FROM app_config WHERE id = 1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(domain, "Animalia");
    }

    #[test]
    fn migration_032_mycology_maps_to_fungi() {
        let conn = migrated_db();
        conn.execute(
            "UPDATE app_config SET lab_profile = 'mycology', domain = 'Fungi' WHERE id = 1",
            [],
        )
        .unwrap();
        let domain: String = conn
            .query_row("SELECT domain FROM app_config WHERE id = 1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(domain, "Fungi");
    }

    // ── Migration 033: breeding_programs & breeding_records ──────────────────

    #[test]
    fn migration_033_breeding_programs_table_exists() {
        let conn = migrated_db();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='breeding_programs'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "breeding_programs table must exist after migration 033");
    }

    #[test]
    fn migration_033_breeding_records_table_exists() {
        let conn = migrated_db();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='breeding_records'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "breeding_records table must exist after migration 033");
    }

    #[test]
    fn migration_033_breeding_records_index_exists() {
        let conn = migrated_db();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' \
                 AND name='idx_breeding_records_program_id'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "index on breeding_records.program_id must exist");
    }

    #[test]
    fn migration_033_breeding_program_cascade_deletes_records() {
        let conn = migrated_db();
        conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();
        conn.execute(
            "INSERT INTO breeding_programs (id, name) VALUES ('bp1', 'Test Program')",
            [],
        ).unwrap();
        // Insert a record without a real strain FK (FK enforcement is only on strains
        // which requires seeding; test just checks cascade on program delete).
        conn.execute_batch("PRAGMA foreign_keys = OFF").unwrap();
        conn.execute(
            "INSERT INTO breeding_records (id, program_id, strain_id) VALUES ('br1','bp1','s1')",
            [],
        ).unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();
        conn.execute("DELETE FROM breeding_programs WHERE id = 'bp1'", []).unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM breeding_records WHERE program_id = 'bp1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 0, "cascade delete must remove breeding_records when program is deleted");
    }

    // ── Migration 034: provisional taxa & taxon_mappings ─────────────────────

    #[test]
    fn migration_034_taxa_status_column_exists() {
        let conn = migrated_db();
        // Inserting a row with an explicit status proves the column exists.
        conn.execute(
            "INSERT INTO taxa (id, rank, name, status) VALUES ('t-prov', 'genus', 'Provisia', 'provisional')",
            [],
        ).unwrap();
        let status: String = conn
            .query_row("SELECT status FROM taxa WHERE id = 't-prov'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(status, "provisional");
    }

    #[test]
    fn migration_034_taxa_status_defaults_to_accepted() {
        let conn = migrated_db();
        conn.execute(
            "INSERT INTO taxa (id, rank, name) VALUES ('t-acc', 'genus', 'Acceptia')",
            [],
        ).unwrap();
        let status: String = conn
            .query_row("SELECT status FROM taxa WHERE id = 't-acc'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(status, "accepted");
    }

    #[test]
    fn migration_034_taxa_provisional_notes_column_exists() {
        let conn = migrated_db();
        conn.execute(
            "INSERT INTO taxa (id, rank, name, provisional_notes) VALUES ('t-n', 'genus', 'Notia', 'test note')",
            [],
        ).unwrap();
        let notes: String = conn
            .query_row("SELECT provisional_notes FROM taxa WHERE id = 't-n'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(notes, "test note");
    }

    #[test]
    fn migration_034_taxon_mappings_table_exists() {
        let conn = migrated_db();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='taxon_mappings'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "taxon_mappings table must exist after migration 034");
    }

    #[test]
    fn migration_034_taxon_mappings_cascade_deletes() {
        let conn = migrated_db();
        conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();
        conn.execute(
            "INSERT INTO taxa (id, rank, name, status) VALUES ('tp1', 'genus', 'Provisia', 'provisional')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO taxon_mappings (id, provisional_taxon_id, accepted_name) \
             VALUES ('tm1', 'tp1', 'Acceptia')",
            [],
        ).unwrap();
        conn.execute("DELETE FROM taxa WHERE id = 'tp1'", []).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM taxon_mappings WHERE id = 'tm1'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0, "taxon_mappings must cascade-delete when provisional taxon is removed");
    }

    // ── Migration 035: multi-user backend + LAN sync foundation ──────────────

    #[test]
    fn migration_035_backend_type_defaults_to_sqlite() {
        let conn = migrated_db();
        let value: String = conn
            .query_row(
                "SELECT value FROM app_settings WHERE key = 'backend_type'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(value, "sqlite");
    }

    #[test]
    fn migration_035_sync_peers_table_exists() {
        let conn = migrated_db();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='sync_peers'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "sync_peers table must exist after migration 035");
    }

    #[test]
    fn migration_035_sync_peers_device_id_unique() {
        let conn = migrated_db();
        conn.execute(
            "INSERT INTO sync_peers (id, device_id, device_name) VALUES ('p1', 'dev-1', 'Lab PC 1')",
            [],
        ).unwrap();
        let dup = conn.execute(
            "INSERT INTO sync_peers (id, device_id, device_name) VALUES ('p2', 'dev-1', 'Lab PC 1 (dup)')",
            [],
        );
        assert!(dup.is_err(), "duplicate device_id must be rejected by the UNIQUE constraint");
    }

    #[test]
    fn migration_035_sync_conflicts_table_exists() {
        let conn = migrated_db();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='sync_conflicts'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "sync_conflicts table must exist after migration 035");
    }

    #[test]
    fn migration_035_sync_conflicts_resolved_defaults_to_zero() {
        let conn = migrated_db();
        conn.execute(
            "INSERT INTO sync_conflicts (id, lineage_id, chain_seq, reason) \
             VALUES ('c1', 'sp-1', 3, 'fork detected')",
            [],
        ).unwrap();
        let resolved: i64 = conn
            .query_row("SELECT resolved FROM sync_conflicts WHERE id = 'c1'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(resolved, 0);
    }

    #[test]
    fn migration_035_is_idempotent_alongside_full_run() {
        let conn = migrated_db();
        run_all(&conn).expect("re-running all migrations including 035 must not error");
        let value: String = conn
            .query_row(
                "SELECT value FROM app_settings WHERE key = 'backend_type'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(value, "sqlite", "second run must not duplicate or corrupt the seeded setting");
    }

    // ── Migration 036: field_permissions ──────────────────────────────────────

    #[test]
    fn migration_036_field_permissions_table_exists() {
        let conn = migrated_db();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='field_permissions'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn migration_036_seeds_twelve_permissive_rows() {
        let conn = migrated_db();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM field_permissions WHERE visible = 1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 12, "4 roles x 3 seeded fields = 12 rows, all visible");
    }

    #[test]
    fn migration_036_rejects_unknown_role() {
        let conn = migrated_db();
        let result = conn.execute(
            "INSERT INTO field_permissions (id, role, entity_type, field_name) VALUES ('x', 'superadmin', 'strain', 'foo')",
            [],
        );
        assert!(result.is_err(), "CHECK constraint must reject roles outside admin/supervisor/tech/guest");
    }

    // ── Migration 037: environmental_readings ─────────────────────────────────

    #[test]
    fn migration_037_environmental_readings_table_exists() {
        let conn = migrated_db();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='environmental_readings'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn migration_037_rejects_row_with_neither_specimen_nor_subculture() {
        let conn = migrated_db();
        let result = conn.execute(
            "INSERT INTO environmental_readings (id, reading_type, value) VALUES ('r1', 'temp_c', 24.0)",
            [],
        );
        assert!(result.is_err(), "CHECK must require at least one of specimen_id/subculture_id");
    }

    #[test]
    fn migration_037_rejects_unknown_reading_type() {
        let conn = migrated_db();
        let result = conn.execute(
            "INSERT INTO environmental_readings (id, specimen_id, reading_type, value) VALUES ('r1', 'sp-1', 'bogus_type', 1.0)",
            [],
        );
        assert!(result.is_err());
    }

    // ── Migration 038: notification_preferences + smtp_config ────────────────

    #[test]
    fn migration_038_notification_preferences_table_exists() {
        let conn = migrated_db();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='notification_preferences'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn migration_038_smtp_config_seeded_single_row() {
        let conn = migrated_db();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM smtp_config", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1, "smtp_config must always have exactly one row (id = 1)");
    }

    #[test]
    fn migration_038_check_interval_default_seeded() {
        let conn = migrated_db();
        let value: String = conn
            .query_row(
                "SELECT value FROM app_settings WHERE key = 'notification_check_interval_minutes'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(value, "15");
    }

    #[test]
    fn migration_038_rejects_duplicate_preference_per_channel() {
        let conn = migrated_db();
        conn.execute(
            "INSERT INTO users (id, username, password_hash, display_name, role) VALUES ('u1','tester','x','Tester','tech')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO notification_preferences (id, user_id, channel) VALUES ('p1', 'u1', 'desktop')",
            [],
        ).unwrap();
        let dup = conn.execute(
            "INSERT INTO notification_preferences (id, user_id, channel) VALUES ('p2', 'u1', 'desktop')",
            [],
        );
        assert!(dup.is_err(), "UNIQUE(user_id, channel) must reject a second row for the same channel");
    }
}
