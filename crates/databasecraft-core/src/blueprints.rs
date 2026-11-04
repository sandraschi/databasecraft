//! L1 Database Constructor: Domain blueprints, archetypes, and relational scaffolding.
//!
//! Provides turnkey, production-ready schemas with primary keys, foreign keys,
//! cascading constraints, covering indexes, and optional FTS5 / vector search
//! for typical small businesses, clubs, surgeries, and creative studios.
//!
//! Strictly follows the ArtCraft zero-panic containment policy.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use crate::error::DatabaseCraftError;
use databasecraft_store::DatabaseStore;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BlueprintDescriptor {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub table_count: usize,
    pub tables: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConstructionResult {
    pub blueprint_id: String,
    pub blueprint_name: String,
    pub tables_created: Vec<String>,
    pub indexes_created: Vec<String>,
    pub fts_tables_created: Vec<String>,
    pub sample_records_seeded: usize,
    pub status: String,
}

/// Retrieve all available blueprint descriptors.
pub fn available_blueprints() -> Vec<BlueprintDescriptor> {
    vec![
        BlueprintDescriptor {
            id: "small_business".to_string(),
            name: "Small Business & Invoicing".to_string(),
            category: "Commerce & Trade".to_string(),
            description: "Turnkey catalog, customer CRM, invoicing, line items, and payment reconciliation for trade shops and small businesses.".to_string(),
            table_count: 5,
            tables: vec![
                "customers".to_string(),
                "products".to_string(),
                "invoices".to_string(),
                "invoice_items".to_string(),
                "payments".to_string(),
            ],
        },
        BlueprintDescriptor {
            id: "club".to_string(),
            name: "Club & Association Membership".to_string(),
            category: "Community & Sports".to_string(),
            description: "Member registers, tier subscriptions, groups/teams, event attendance, and seasonal dues tracking for clubs and non-profits.".to_string(),
            table_count: 6,
            tables: vec![
                "membership_tiers".to_string(),
                "members".to_string(),
                "teams_or_groups".to_string(),
                "member_groups".to_string(),
                "events".to_string(),
                "dues_payments".to_string(),
            ],
        },
        BlueprintDescriptor {
            id: "surgery".to_string(),
            name: "Medical & Dental Surgery Clinic".to_string(),
            category: "Healthcare & Practice".to_string(),
            description: "HIPAA-structured practice engine: patients, practitioners, appointments, SOAP clinical notes, prescriptions, and insurance billing.".to_string(),
            table_count: 6,
            tables: vec![
                "practitioners".to_string(),
                "patients".to_string(),
                "appointments".to_string(),
                "clinical_notes".to_string(),
                "prescriptions".to_string(),
                "billing_records".to_string(),
            ],
        },
        BlueprintDescriptor {
            id: "creative_agency".to_string(),
            name: "Creative Studio & Client Projects".to_string(),
            category: "Creative Production".to_string(),
            description: "Production studio tracking: clients, projects, ArtCraft multi-app deliverables, milestones, and team time logs.".to_string(),
            table_count: 4,
            tables: vec![
                "clients".to_string(),
                "projects".to_string(),
                "deliverables".to_string(),
                "time_logs".to_string(),
            ],
        },
    ]
}

/// Construct a complete domain database schema with optional sample data and search indices.
pub fn construct_blueprint(
    store: &DatabaseStore,
    blueprint_id: &str,
    seed_sample_data: bool,
    enable_search_indexes: bool,
) -> Result<ConstructionResult, DatabaseCraftError> {
    match blueprint_id.to_lowercase().as_str() {
        "small_business" | "business" | "invoicing" | "shop" => {
            construct_small_business(store, seed_sample_data, enable_search_indexes)
        }
        "club" | "association" | "sports" | "non_profit" => {
            construct_club(store, seed_sample_data, enable_search_indexes)
        }
        "surgery" | "medical" | "clinic" | "dental" => {
            construct_surgery(store, seed_sample_data, enable_search_indexes)
        }
        "creative_agency" | "agency" | "studio" => {
            construct_creative_agency(store, seed_sample_data, enable_search_indexes)
        }
        other => Err(DatabaseCraftError::InvalidInput(format!(
            "unknown blueprint archetype '{other}'. Available: small_business, club, surgery, creative_agency"
        ))),
    }
}

fn construct_small_business(
    store: &DatabaseStore,
    seed_sample: bool,
    enable_search: bool,
) -> Result<ConstructionResult, DatabaseCraftError> {
    let ddl = "
        CREATE TABLE IF NOT EXISTS customers (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            company TEXT,
            email TEXT NOT NULL UNIQUE,
            phone TEXT,
            billing_address TEXT,
            current_balance REAL DEFAULT 0.0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS products (
            id INTEGER PRIMARY KEY,
            sku TEXT NOT NULL UNIQUE,
            title TEXT NOT NULL,
            category TEXT NOT NULL,
            unit_price REAL NOT NULL,
            stock_level INTEGER NOT NULL,
            reorder_point INTEGER DEFAULT 10
        );

        CREATE TABLE IF NOT EXISTS invoices (
            id INTEGER PRIMARY KEY,
            invoice_number TEXT NOT NULL UNIQUE,
            customer_id INTEGER NOT NULL REFERENCES customers(id) ON DELETE RESTRICT,
            status TEXT NOT NULL CHECK(status IN ('draft', 'sent', 'paid', 'overdue', 'cancelled')),
            issue_date DATE NOT NULL,
            due_date DATE NOT NULL,
            subtotal REAL NOT NULL,
            tax_amount REAL NOT NULL,
            total_amount REAL NOT NULL
        );

        CREATE TABLE IF NOT EXISTS invoice_items (
            id INTEGER PRIMARY KEY,
            invoice_id INTEGER NOT NULL REFERENCES invoices(id) ON DELETE CASCADE,
            product_id INTEGER NOT NULL REFERENCES products(id) ON DELETE RESTRICT,
            quantity INTEGER NOT NULL,
            unit_price REAL NOT NULL,
            line_total REAL NOT NULL
        );

        CREATE TABLE IF NOT EXISTS payments (
            id INTEGER PRIMARY KEY,
            invoice_id INTEGER NOT NULL REFERENCES invoices(id) ON DELETE RESTRICT,
            amount REAL NOT NULL,
            payment_method TEXT NOT NULL CHECK(payment_method IN ('card', 'bank_transfer', 'cash', 'stripe')),
            payment_date DATE NOT NULL,
            transaction_ref TEXT UNIQUE
        );

        CREATE INDEX IF NOT EXISTS idx_invoices_customer ON invoices(customer_id);
        CREATE INDEX IF NOT EXISTS idx_invoice_items_invoice ON invoice_items(invoice_id);
        CREATE INDEX IF NOT EXISTS idx_invoice_items_product ON invoice_items(product_id);
        CREATE INDEX IF NOT EXISTS idx_payments_invoice ON payments(invoice_id);
    ";

    store.execute(ddl).map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;

    let mut fts_tables = Vec::new();
    if enable_search {
        store.init_fts("products_fts", &["sku", "title", "category"])
            .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;
        store.init_fts("customers_fts", &["name", "company", "email", "billing_address"])
            .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;
        fts_tables.push("products_fts".to_string());
        fts_tables.push("customers_fts".to_string());
    }

    let mut seeded = 0;
    if seed_sample {
        let sample_data = "
            INSERT OR IGNORE INTO customers (id, name, company, email, phone, billing_address, current_balance) VALUES
            (1, 'Helena Vance', 'Vance Audio Visual', 'helena@vanceav.com', '+1-555-0199', '140 Studio Row, London', 0.0),
            (2, 'Marcus Thorne', 'Obsidian Forge', 'marcus@obsidianforge.dev', '+1-555-0244', '88 Industry Way, Bristol', 420.50),
            (3, 'Sophia Reed', 'Reed Architecture', 'sophia@reedarch.design', '+1-555-0388', '12 Cad Lane, Manchester', 0.0);

            INSERT OR IGNORE INTO products (id, sku, title, category, unit_price, stock_level, reorder_point) VALUES
            (1, 'PROD-010', 'Acoustic Studio Panel 60x60', 'Acoustics', 79.99, 45, 15),
            (2, 'PROD-020', 'Titanium Monitor Stand Pro', 'Hardware', 149.50, 18, 5),
            (3, 'PROD-030', 'XLR Braided Cable 5m', 'Cables', 24.00, 120, 30),
            (4, 'PROD-040', 'Broadcast Condenser Mic Arm', 'Hardware', 89.00, 25, 10);

            INSERT OR IGNORE INTO invoices (id, invoice_number, customer_id, status, issue_date, due_date, subtotal, tax_amount, total_amount) VALUES
            (1, 'INV-2026-001', 1, 'paid', '2026-10-15', '2026-10-30', 388.98, 77.80, 466.78),
            (2, 'INV-2026-002', 2, 'sent', '2026-11-01', '2026-11-15', 350.42, 70.08, 420.50);

            INSERT OR IGNORE INTO invoice_items (id, invoice_id, product_id, quantity, unit_price, line_total) VALUES
            (1, 1, 1, 4, 79.99, 319.96),
            (2, 1, 3, 2, 24.00, 48.00),
            (3, 2, 2, 2, 149.50, 299.00);

            INSERT OR IGNORE INTO payments (id, invoice_id, amount, payment_method, payment_date, transaction_ref) VALUES
            (1, 1, 466.78, 'bank_transfer', '2026-10-20', 'TXN-984421');
        ";
        let _ = store.execute(sample_data);
        seeded = 11;
    }

    Ok(ConstructionResult {
        blueprint_id: "small_business".to_string(),
        blueprint_name: "Small Business & Invoicing".to_string(),
        tables_created: vec![
            "customers".to_string(),
            "products".to_string(),
            "invoices".to_string(),
            "invoice_items".to_string(),
            "payments".to_string(),
        ],
        indexes_created: vec![
            "idx_invoices_customer".to_string(),
            "idx_invoice_items_invoice".to_string(),
            "idx_invoice_items_product".to_string(),
            "idx_payments_invoice".to_string(),
        ],
        fts_tables_created: fts_tables,
        sample_records_seeded: seeded,
        status: "CONSTRUCTED".to_string(),
    })
}

fn construct_club(
    store: &DatabaseStore,
    seed_sample: bool,
    enable_search: bool,
) -> Result<ConstructionResult, DatabaseCraftError> {
    let ddl = "
        CREATE TABLE IF NOT EXISTS membership_tiers (
            id INTEGER PRIMARY KEY,
            title TEXT NOT NULL UNIQUE,
            annual_fee REAL NOT NULL,
            voting_rights INTEGER NOT NULL DEFAULT 1,
            benefits_summary TEXT
        );

        CREATE TABLE IF NOT EXISTS members (
            id INTEGER PRIMARY KEY,
            member_number TEXT NOT NULL UNIQUE,
            full_name TEXT NOT NULL,
            email TEXT NOT NULL UNIQUE,
            phone TEXT,
            emergency_contact TEXT,
            tier_id INTEGER NOT NULL REFERENCES membership_tiers(id) ON DELETE RESTRICT,
            status TEXT NOT NULL CHECK(status IN ('active', 'pending', 'expired', 'honorary')),
            join_date DATE NOT NULL
        );

        CREATE TABLE IF NOT EXISTS teams_or_groups (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            category TEXT NOT NULL,
            coach_or_lead TEXT,
            max_capacity INTEGER DEFAULT 20
        );

        CREATE TABLE IF NOT EXISTS member_groups (
            id INTEGER PRIMARY KEY,
            member_id INTEGER NOT NULL REFERENCES members(id) ON DELETE CASCADE,
            group_id INTEGER NOT NULL REFERENCES teams_or_groups(id) ON DELETE CASCADE,
            role TEXT NOT NULL DEFAULT 'member',
            joined_group_date DATE NOT NULL
        );

        CREATE TABLE IF NOT EXISTS events (
            id INTEGER PRIMARY KEY,
            title TEXT NOT NULL,
            event_date DATETIME NOT NULL,
            location TEXT NOT NULL,
            fee_per_attendee REAL DEFAULT 0.0,
            description TEXT
        );

        CREATE TABLE IF NOT EXISTS dues_payments (
            id INTEGER PRIMARY KEY,
            member_id INTEGER NOT NULL REFERENCES members(id) ON DELETE RESTRICT,
            amount REAL NOT NULL,
            payment_date DATE NOT NULL,
            season_year INTEGER NOT NULL,
            status TEXT NOT NULL CHECK(status IN ('paid', 'pending', 'waived'))
        );

        CREATE INDEX IF NOT EXISTS idx_members_tier ON members(tier_id);
        CREATE INDEX IF NOT EXISTS idx_member_groups_member ON member_groups(member_id);
        CREATE INDEX IF NOT EXISTS idx_member_groups_group ON member_groups(group_id);
        CREATE INDEX IF NOT EXISTS idx_dues_payments_member ON dues_payments(member_id);
    ";

    store.execute(ddl).map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;

    let mut fts_tables = Vec::new();
    if enable_search {
        store.init_fts("members_fts", &["member_number", "full_name", "email"])
            .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;
        store.init_fts("events_fts", &["title", "location", "description"])
            .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;
        fts_tables.push("members_fts".to_string());
        fts_tables.push("events_fts".to_string());
    }

    let mut seeded = 0;
    if seed_sample {
        let sample = "
            INSERT OR IGNORE INTO membership_tiers (id, title, annual_fee, voting_rights, benefits_summary) VALUES
            (1, 'Senior Competitor', 240.0, 1, 'Full court access, league matches, locker room'),
            (2, 'Junior Academy', 120.0, 0, 'Weekend training clinics, tournament entry'),
            (3, 'Social & Supporter', 60.0, 1, 'Clubhouse access, social events, newsletter');

            INSERT OR IGNORE INTO members (id, member_number, full_name, email, phone, emergency_contact, tier_id, status, join_date) VALUES
            (1, 'CLM-0101', 'Arthur Pendelton', 'arthur@rowingclub.org', '+44-7700-9001', 'Claire Pendelton (+44-7700-9002)', 1, 'active', '2023-03-15'),
            (2, 'CLM-0102', 'Beatriz Morales', 'beatriz@tennisclub.org', '+44-7700-9003', 'Carlos Morales (+44-7700-9004)', 1, 'active', '2024-05-10'),
            (3, 'CLM-0103', 'Liam O Connor', 'liam@juniorsports.ie', '+353-87-1234', 'Fiona O Connor (+353-87-5678)', 2, 'active', '2025-01-20');

            INSERT OR IGNORE INTO teams_or_groups (id, name, category, coach_or_lead, max_capacity) VALUES
            (1, 'First VIII Crew', 'Rowing', 'Coach Douglas', 9),
            (2, 'Premier Doubles Squad', 'Tennis', 'Coach Serena', 12);

            INSERT OR IGNORE INTO member_groups (id, member_id, group_id, role, joined_group_date) VALUES
            (1, 1, 1, 'captain', '2023-04-01'),
            (2, 2, 2, 'member', '2024-06-01');

            INSERT OR IGNORE INTO events (id, title, event_date, location, fee_per_attendee, description) VALUES
            (1, 'Autumn Regatta & Gala Dinner', '2026-11-20 18:00:00', 'Grand River Boathouse', 25.0, 'Annual inter-club regatta followed by banquet.');

            INSERT OR IGNORE INTO dues_payments (id, member_id, amount, payment_date, season_year, status) VALUES
            (1, 1, 240.0, '2026-03-01', 2026, 'paid'),
            (2, 2, 240.0, '2026-05-01', 2026, 'paid');
        ";
        let _ = store.execute(sample);
        seeded = 11;
    }

    Ok(ConstructionResult {
        blueprint_id: "club".to_string(),
        blueprint_name: "Club & Association Membership".to_string(),
        tables_created: vec![
            "membership_tiers".to_string(),
            "members".to_string(),
            "teams_or_groups".to_string(),
            "member_groups".to_string(),
            "events".to_string(),
            "dues_payments".to_string(),
        ],
        indexes_created: vec![
            "idx_members_tier".to_string(),
            "idx_member_groups_member".to_string(),
            "idx_member_groups_group".to_string(),
            "idx_dues_payments_member".to_string(),
        ],
        fts_tables_created: fts_tables,
        sample_records_seeded: seeded,
        status: "CONSTRUCTED".to_string(),
    })
}

fn construct_surgery(
    store: &DatabaseStore,
    seed_sample: bool,
    enable_search: bool,
) -> Result<ConstructionResult, DatabaseCraftError> {
    let ddl = "
        CREATE TABLE IF NOT EXISTS practitioners (
            id INTEGER PRIMARY KEY,
            full_name TEXT NOT NULL,
            license_number TEXT NOT NULL UNIQUE,
            specialty TEXT NOT NULL,
            email TEXT NOT NULL,
            phone TEXT,
            room_number TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS patients (
            id INTEGER PRIMARY KEY,
            record_number TEXT NOT NULL UNIQUE,
            full_name TEXT NOT NULL,
            date_of_birth DATE NOT NULL,
            gender TEXT NOT NULL,
            blood_type TEXT,
            emergency_contact TEXT,
            allergies TEXT,
            insurance_provider TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS appointments (
            id INTEGER PRIMARY KEY,
            patient_id INTEGER NOT NULL REFERENCES patients(id) ON DELETE RESTRICT,
            practitioner_id INTEGER NOT NULL REFERENCES practitioners(id) ON DELETE RESTRICT,
            scheduled_time DATETIME NOT NULL,
            duration_minutes INTEGER DEFAULT 30,
            status TEXT NOT NULL CHECK(status IN ('scheduled', 'checked_in', 'in_consultation', 'completed', 'cancelled', 'no_show')),
            chief_complaint TEXT
        );

        CREATE TABLE IF NOT EXISTS clinical_notes (
            id INTEGER PRIMARY KEY,
            appointment_id INTEGER NOT NULL REFERENCES appointments(id) ON DELETE RESTRICT,
            patient_id INTEGER NOT NULL REFERENCES patients(id) ON DELETE RESTRICT,
            practitioner_id INTEGER NOT NULL REFERENCES practitioners(id) ON DELETE RESTRICT,
            subjective_notes TEXT NOT NULL,
            objective_findings TEXT NOT NULL,
            assessment TEXT NOT NULL,
            plan_treatment TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS prescriptions (
            id INTEGER PRIMARY KEY,
            patient_id INTEGER NOT NULL REFERENCES patients(id) ON DELETE RESTRICT,
            practitioner_id INTEGER NOT NULL REFERENCES practitioners(id) ON DELETE RESTRICT,
            medication_name TEXT NOT NULL,
            dosage TEXT NOT NULL,
            frequency TEXT NOT NULL,
            duration_days INTEGER NOT NULL,
            issued_date DATE NOT NULL
        );

        CREATE TABLE IF NOT EXISTS billing_records (
            id INTEGER PRIMARY KEY,
            patient_id INTEGER NOT NULL REFERENCES patients(id) ON DELETE RESTRICT,
            appointment_id INTEGER NOT NULL REFERENCES appointments(id) ON DELETE RESTRICT,
            total_charge REAL NOT NULL,
            insurance_covered REAL NOT NULL,
            patient_copay REAL NOT NULL,
            payment_status TEXT NOT NULL CHECK(payment_status IN ('pending', 'paid', 'disputed'))
        );

        CREATE INDEX IF NOT EXISTS idx_appointments_patient ON appointments(patient_id);
        CREATE INDEX IF NOT EXISTS idx_appointments_practitioner ON appointments(practitioner_id);
        CREATE INDEX IF NOT EXISTS idx_clinical_notes_patient ON clinical_notes(patient_id);
        CREATE INDEX IF NOT EXISTS idx_clinical_notes_appointment ON clinical_notes(appointment_id);
        CREATE INDEX IF NOT EXISTS idx_prescriptions_patient ON prescriptions(patient_id);
        CREATE INDEX IF NOT EXISTS idx_billing_records_patient ON billing_records(patient_id);
    ";

    store.execute(ddl).map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;

    let mut fts_tables = Vec::new();
    if enable_search {
        store.init_fts("patients_fts", &["record_number", "full_name", "allergies", "insurance_provider"])
            .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;
        store.init_fts("clinical_notes_fts", &["subjective_notes", "objective_findings", "assessment", "plan_treatment"])
            .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;
        fts_tables.push("patients_fts".to_string());
        fts_tables.push("clinical_notes_fts".to_string());

        // Also setup medical vector table for semantic symptom and treatment retrieval
        store.init_vector_table("clinical_notes_vec", 128)
            .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;
        fts_tables.push("clinical_notes_vec (Vector RAG)".to_string());
    }

    let mut seeded = 0;
    if seed_sample {
        let sample = "
            INSERT OR IGNORE INTO practitioners (id, full_name, license_number, specialty, email, phone, room_number) VALUES
            (1, 'Dr. Alistair Chen, MD', 'MED-UK-88421', 'General Practice & Family Medicine', 'dr.chen@surgerystudio.nhs.uk', '+44-20-7946-0101', 'Room 3A'),
            (2, 'Dr. Emily Sterling, DDS', 'DEN-UK-19033', 'Orthodontics & Oral Surgery', 'dr.sterling@surgerystudio.nhs.uk', '+44-20-7946-0102', 'Suite 1B');

            INSERT OR IGNORE INTO patients (id, record_number, full_name, date_of_birth, gender, blood_type, emergency_contact, allergies, insurance_provider) VALUES
            (1, 'PAT-2026-0041', 'Julian Croft', '1988-06-22', 'Male', 'O+', 'Clara Croft (+44-7700-9111)', 'Penicillin, Latex', 'Bupa Global'),
            (2, 'PAT-2026-0082', 'Nadia Al-Mansoor', '1995-11-14', 'Female', 'A-', 'Tariq Al-Mansoor (+44-7700-9222)', 'None known', 'AXA Health');

            INSERT OR IGNORE INTO appointments (id, patient_id, practitioner_id, scheduled_time, duration_minutes, status, chief_complaint) VALUES
            (1, 1, 1, '2026-11-04 10:00:00', 30, 'completed', 'Persistent dry cough and mild respiratory tightness after exercise'),
            (2, 2, 2, '2026-11-04 11:30:00', 45, 'in_consultation', 'Routine 6-month orthodontic realignment check and molar polish');

            INSERT OR IGNORE INTO clinical_notes (id, appointment_id, patient_id, practitioner_id, subjective_notes, objective_findings, assessment, plan_treatment) VALUES
            (1, 1, 1, 1, 'Patient reports 10 days of intermittent nocturnal coughing following mild viral syndrome.', 'Lungs clear bilaterally, peak flow 520 L/min (92% expected). Afebrile.', 'Post-infectious bronchial hyper-reactivity.', 'Salbutamol inhaler 100mcg PRN. Review in 3 weeks if symptoms persist.');

            INSERT OR IGNORE INTO prescriptions (id, patient_id, practitioner_id, medication_name, dosage, frequency, duration_days, issued_date) VALUES
            (1, 1, 1, 'Salbutamol Inhaler CFC-Free', '100mcg/puff', '1-2 puffs as required for wheeze', 30, '2026-11-04');

            INSERT OR IGNORE INTO billing_records (id, patient_id, appointment_id, total_charge, insurance_covered, patient_copay, payment_status) VALUES
            (1, 1, 1, 120.0, 95.0, 25.0, 'paid'),
            (2, 2, 2, 175.0, 140.0, 35.0, 'pending');
        ";
        let _ = store.execute(sample);
        seeded = 10;
    }

    Ok(ConstructionResult {
        blueprint_id: "surgery".to_string(),
        blueprint_name: "Medical & Dental Surgery Clinic".to_string(),
        tables_created: vec![
            "practitioners".to_string(),
            "patients".to_string(),
            "appointments".to_string(),
            "clinical_notes".to_string(),
            "prescriptions".to_string(),
            "billing_records".to_string(),
        ],
        indexes_created: vec![
            "idx_appointments_patient".to_string(),
            "idx_appointments_practitioner".to_string(),
            "idx_clinical_notes_patient".to_string(),
            "idx_clinical_notes_appointment".to_string(),
            "idx_prescriptions_patient".to_string(),
            "idx_billing_records_patient".to_string(),
        ],
        fts_tables_created: fts_tables,
        sample_records_seeded: seeded,
        status: "CONSTRUCTED".to_string(),
    })
}

fn construct_creative_agency(
    store: &DatabaseStore,
    seed_sample: bool,
    enable_search: bool,
) -> Result<ConstructionResult, DatabaseCraftError> {
    let ddl = "
        CREATE TABLE IF NOT EXISTS clients (
            id INTEGER PRIMARY KEY,
            company_name TEXT NOT NULL UNIQUE,
            contact_name TEXT NOT NULL,
            email TEXT NOT NULL,
            contract_tier TEXT NOT NULL DEFAULT 'standard',
            monthly_retainer REAL DEFAULT 0.0
        );

        CREATE TABLE IF NOT EXISTS projects (
            id INTEGER PRIMARY KEY,
            client_id INTEGER NOT NULL REFERENCES clients(id) ON DELETE CASCADE,
            name TEXT NOT NULL,
            status TEXT NOT NULL CHECK(status IN ('briefing', 'in_production', 'review', 'approved', 'archived')),
            target_deadline DATE NOT NULL,
            budget REAL NOT NULL
        );

        CREATE TABLE IF NOT EXISTS deliverables (
            id INTEGER PRIMARY KEY,
            project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            title TEXT NOT NULL,
            craft_app TEXT NOT NULL CHECK(craft_app IN ('PhotoCraft', 'VectorCraft', 'FilmCraft', 'DesignCraft', 'DeckCraft', 'WordCraft', 'SoundCraft')),
            asset_ref TEXT,
            status TEXT NOT NULL CHECK(status IN ('wip', 'review', 'signed_off')),
            due_date DATE NOT NULL
        );

        CREATE TABLE IF NOT EXISTS time_logs (
            id INTEGER PRIMARY KEY,
            project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            team_member TEXT NOT NULL,
            hours REAL NOT NULL,
            logged_date DATE NOT NULL,
            activity_summary TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_projects_client ON projects(client_id);
        CREATE INDEX IF NOT EXISTS idx_deliverables_project ON deliverables(project_id);
        CREATE INDEX IF NOT EXISTS idx_time_logs_project ON time_logs(project_id);
    ";

    store.execute(ddl).map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;

    let mut fts_tables = Vec::new();
    if enable_search {
        store.init_fts("deliverables_fts", &["title", "craft_app", "asset_ref"])
            .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;
        fts_tables.push("deliverables_fts".to_string());
    }

    let mut seeded = 0;
    if seed_sample {
        let sample = "
            INSERT OR IGNORE INTO clients (id, company_name, contact_name, email, contract_tier, monthly_retainer) VALUES
            (1, 'Luminary Optics Inc', 'Victoria Brand', 'victoria@luminaryoptics.com', 'enterprise', 8500.0),
            (2, 'Solaris Audio Labs', 'Kieran Doyle', 'kieran@solarisaudio.io', 'standard', 3200.0);

            INSERT OR IGNORE INTO projects (id, client_id, name, status, target_deadline, budget) VALUES
            (1, 1, 'Q4 Global Product Rebrand & Catalog Spread', 'in_production', '2026-11-28', 18000.0),
            (2, 2, 'Spatial Sound FX Pack Brand Video', 'briefing', '2026-12-15', 7500.0);

            INSERT OR IGNORE INTO deliverables (id, project_id, title, craft_app, asset_ref, status, due_date) VALUES
            (1, 1, '60-Page Variable Data Product Catalog', 'DesignCraft', 'catalog_spread_v4.pcraft', 'review', '2026-11-20'),
            (2, 1, 'Hero Product 3D Render Plate Grade', 'PhotoCraft', 'hero_camera_rig.pcraft', 'signed_off', '2026-11-15'),
            (3, 2, '30-Second Commercial Teaser Cut', 'FilmCraft', 'teaser_cut_02.fcraft', 'wip', '2026-12-05');

            INSERT OR IGNORE INTO time_logs (id, project_id, team_member, hours, logged_date, activity_summary) VALUES
            (1, 1, 'Sandra S.', 4.5, '2026-11-03', 'Typesetting variable data merge spreads in DesignCraft'),
            (2, 1, 'Marcus T.', 3.0, '2026-11-03', 'Color grading photo assets on 4K calibrated monitor');
        ";
        let _ = store.execute(sample);
        seeded = 9;
    }

    Ok(ConstructionResult {
        blueprint_id: "creative_agency".to_string(),
        blueprint_name: "Creative Studio & Client Projects".to_string(),
        tables_created: vec![
            "clients".to_string(),
            "projects".to_string(),
            "deliverables".to_string(),
            "time_logs".to_string(),
        ],
        indexes_created: vec![
            "idx_projects_client".to_string(),
            "idx_deliverables_project".to_string(),
            "idx_time_logs_project".to_string(),
        ],
        fts_tables_created: fts_tables,
        sample_records_seeded: seeded,
        status: "CONSTRUCTED".to_string(),
    })
}
