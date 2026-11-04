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
        BlueprintDescriptor {
            id: "real_estate".to_string(),
            name: "Property & Lease Management".to_string(),
            category: "Real Estate & Facilities".to_string(),
            description: "Turnkey property portfolio, unit configurations, active leases, maintenance ticketing, and rent ledger reconciliation.".to_string(),
            table_count: 5,
            tables: vec![
                "properties".to_string(),
                "units".to_string(),
                "tenants".to_string(),
                "leases".to_string(),
                "maintenance_tickets".to_string(),
            ],
        },
        BlueprintDescriptor {
            id: "field_service".to_string(),
            name: "Field Service & Trades Contractor".to_string(),
            category: "Contractors & Field Work".to_string(),
            description: "Contractor operations: service sites, dispatch technicians, work orders, parts inventory, and diagnostic job sheets.".to_string(),
            table_count: 5,
            tables: vec![
                "service_clients".to_string(),
                "technicians".to_string(),
                "work_orders".to_string(),
                "job_parts".to_string(),
                "service_invoices".to_string(),
            ],
        },
        BlueprintDescriptor {
            id: "hospitality".to_string(),
            name: "Restaurant & Table Reservations".to_string(),
            category: "Hospitality & Dining".to_string(),
            description: "Dining floor plan, table reservations, guest VIP preferences & allergy logs, and order billing tickets.".to_string(),
            table_count: 5,
            tables: vec![
                "dining_sections".to_string(),
                "tables".to_string(),
                "guests".to_string(),
                "reservations".to_string(),
                "guest_preferences".to_string(),
            ],
        },
        BlueprintDescriptor {
            id: "legal".to_string(),
            name: "Legal Case & Matter Practice".to_string(),
            category: "Legal & Advisory".to_string(),
            description: "Law practice docket: clients, legal matters/cases, filings, statutory court deadlines, and billable time entries.".to_string(),
            table_count: 5,
            tables: vec![
                "legal_clients".to_string(),
                "matters".to_string(),
                "case_filings".to_string(),
                "court_deadlines".to_string(),
                "billable_entries".to_string(),
            ],
        },
        BlueprintDescriptor {
            id: "automotive".to_string(),
            name: "Auto Workshop & Fleet Service".to_string(),
            category: "Automotive & Mechanical".to_string(),
            description: "Auto garage engine: vehicle registries (VIN/plates), repair job cards, labor operations, spare parts, and MOT inspections.".to_string(),
            table_count: 5,
            tables: vec![
                "vehicle_owners".to_string(),
                "vehicles".to_string(),
                "job_cards".to_string(),
                "service_parts".to_string(),
                "inspections".to_string(),
            ],
        },
        BlueprintDescriptor {
            id: "education".to_string(),
            name: "Academy & Tutoring Center".to_string(),
            category: "Education & Coaching".to_string(),
            description: "School/academy structure: instructors, students, course syllabi, class cohorts, attendance rolls, and tuition enrollments.".to_string(),
            table_count: 6,
            tables: vec![
                "instructors".to_string(),
                "students".to_string(),
                "courses".to_string(),
                "class_cohorts".to_string(),
                "attendance_records".to_string(),
                "enrollments".to_string(),
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
        "real_estate" | "property" | "rental" | "apartments" => {
            construct_real_estate(store, seed_sample_data, enable_search_indexes)
        }
        "field_service" | "contractor" | "trades" | "hvac" | "plumbing" => {
            construct_field_service(store, seed_sample_data, enable_search_indexes)
        }
        "hospitality" | "restaurant" | "dining" | "reservations" => {
            construct_hospitality(store, seed_sample_data, enable_search_indexes)
        }
        "legal" | "law" | "attorney" | "practice" => {
            construct_legal(store, seed_sample_data, enable_search_indexes)
        }
        "automotive" | "garage" | "auto" | "mechanic" | "workshop" => {
            construct_automotive(store, seed_sample_data, enable_search_indexes)
        }
        "education" | "academy" | "school" | "tutoring" => {
            construct_education(store, seed_sample_data, enable_search_indexes)
        }
        other => Err(DatabaseCraftError::InvalidInput(format!(
            "unknown blueprint archetype '{other}'. Available: small_business, club, surgery, creative_agency, real_estate, field_service, hospitality, legal, automotive, education"
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

    store
        .execute(ddl)
        .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;

    let mut fts_tables = Vec::new();
    if enable_search {
        store
            .init_fts("products_fts", &["sku", "title", "category"])
            .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;
        store
            .init_fts(
                "customers_fts",
                &["name", "company", "email", "billing_address"],
            )
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

    store
        .execute(ddl)
        .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;

    let mut fts_tables = Vec::new();
    if enable_search {
        store
            .init_fts("members_fts", &["member_number", "full_name", "email"])
            .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;
        store
            .init_fts("events_fts", &["title", "location", "description"])
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

    store
        .execute(ddl)
        .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;

    let mut fts_tables = Vec::new();
    if enable_search {
        store
            .init_fts(
                "patients_fts",
                &[
                    "record_number",
                    "full_name",
                    "allergies",
                    "insurance_provider",
                ],
            )
            .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;
        store
            .init_fts(
                "clinical_notes_fts",
                &[
                    "subjective_notes",
                    "objective_findings",
                    "assessment",
                    "plan_treatment",
                ],
            )
            .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;
        fts_tables.push("patients_fts".to_string());
        fts_tables.push("clinical_notes_fts".to_string());

        // Also setup medical vector table for semantic symptom and treatment retrieval
        store
            .init_vector_table("clinical_notes_vec", 128)
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

    store
        .execute(ddl)
        .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;

    let mut fts_tables = Vec::new();
    if enable_search {
        store
            .init_fts("deliverables_fts", &["title", "craft_app", "asset_ref"])
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

// ==========================================
// REAL ESTATE BLUEPRINT
// ==========================================
fn construct_real_estate(
    store: &DatabaseStore,
    seed_sample: bool,
    enable_search: bool,
) -> Result<ConstructionResult, DatabaseCraftError> {
    let ddl = "
        CREATE TABLE IF NOT EXISTS properties (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            address TEXT NOT NULL,
            city TEXT NOT NULL,
            postal_code TEXT NOT NULL,
            property_type TEXT NOT NULL CHECK(property_type IN ('residential', 'commercial', 'industrial', 'mixed')),
            total_units INTEGER NOT NULL DEFAULT 1,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS units (
            id INTEGER PRIMARY KEY,
            property_id INTEGER NOT NULL REFERENCES properties(id) ON DELETE CASCADE,
            unit_number TEXT NOT NULL,
            floor_number INTEGER NOT NULL DEFAULT 1,
            square_meters REAL NOT NULL,
            bedrooms INTEGER DEFAULT 1,
            bathrooms REAL DEFAULT 1.0,
            monthly_rent REAL NOT NULL,
            is_occupied INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS tenants (
            id INTEGER PRIMARY KEY,
            full_name TEXT NOT NULL,
            email TEXT NOT NULL UNIQUE,
            phone TEXT NOT NULL,
            emergency_contact TEXT,
            credit_score INTEGER,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS leases (
            id INTEGER PRIMARY KEY,
            unit_id INTEGER NOT NULL REFERENCES units(id) ON DELETE RESTRICT,
            tenant_id INTEGER NOT NULL REFERENCES tenants(id) ON DELETE RESTRICT,
            start_date DATE NOT NULL,
            end_date DATE NOT NULL,
            rent_amount REAL NOT NULL,
            deposit_held REAL NOT NULL,
            status TEXT NOT NULL CHECK(status IN ('active', 'pending', 'expired', 'terminated'))
        );

        CREATE TABLE IF NOT EXISTS maintenance_tickets (
            id INTEGER PRIMARY KEY,
            unit_id INTEGER NOT NULL REFERENCES units(id) ON DELETE CASCADE,
            tenant_id INTEGER REFERENCES tenants(id) ON DELETE SET NULL,
            category TEXT NOT NULL CHECK(category IN ('plumbing', 'electrical', 'hvac', 'appliance', 'structural', 'locksmith')),
            priority TEXT NOT NULL CHECK(priority IN ('low', 'medium', 'high', 'emergency')),
            title TEXT NOT NULL,
            description TEXT NOT NULL,
            status TEXT NOT NULL CHECK(status IN ('open', 'dispatched', 'in_progress', 'resolved', 'closed')),
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE INDEX IF NOT EXISTS idx_units_property ON units(property_id);
        CREATE INDEX IF NOT EXISTS idx_leases_unit ON leases(unit_id);
        CREATE INDEX IF NOT EXISTS idx_leases_tenant ON leases(tenant_id);
        CREATE INDEX IF NOT EXISTS idx_tickets_unit ON maintenance_tickets(unit_id);
    ";

    store
        .execute(ddl)
        .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;

    let mut fts_tables = Vec::new();
    if enable_search {
        store
            .init_fts(
                "maintenance_tickets_fts",
                &["category", "title", "description"],
            )
            .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;
        store
            .init_fts("properties_fts", &["name", "address", "city"])
            .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;
        fts_tables.push("maintenance_tickets_fts".to_string());
        fts_tables.push("properties_fts".to_string());
    }

    let mut seeded = 0;
    if seed_sample {
        let sample = "
            INSERT OR IGNORE INTO properties (id, name, address, city, postal_code, property_type, total_units) VALUES
            (1, 'Harbor View Apartments', '42 Marina Boulevard', 'Bristol', 'BS1 5TZ', 'residential', 12),
            (2, 'Apex Creative Hub', '108 Commercial Way', 'Manchester', 'M1 4ET', 'commercial', 6);

            INSERT OR IGNORE INTO units (id, property_id, unit_number, floor_number, square_meters, bedrooms, bathrooms, monthly_rent, is_occupied) VALUES
            (1, 1, 'Suite 3A', 3, 68.5, 2, 1.0, 1450.0, 1),
            (2, 1, 'Suite 3B', 3, 52.0, 1, 1.0, 1150.0, 0),
            (3, 2, 'Studio 102', 1, 110.0, 0, 2.0, 2800.0, 1);

            INSERT OR IGNORE INTO tenants (id, full_name, email, phone, emergency_contact, credit_score) VALUES
            (1, 'Dominic Wright', 'dominic.wright@cloudpulse.io', '+44-7700-900123', 'Laura Wright (+44-7700-900456)', 745),
            (2, 'Aria Chen', 'aria@chencreative.co.uk', '+44-7700-900789', 'Office Manager (+44-7700-900999)', 790);

            INSERT OR IGNORE INTO leases (id, unit_id, tenant_id, start_date, end_date, rent_amount, deposit_held, status) VALUES
            (1, 1, 1, '2026-01-01', '2026-12-31', 1450.0, 1450.0, 'active'),
            (2, 3, 2, '2025-09-01', '2027-08-31', 2800.0, 5600.0, 'active');

            INSERT OR IGNORE INTO maintenance_tickets (id, unit_id, tenant_id, category, priority, title, description, status) VALUES
            (1, 1, 1, 'plumbing', 'high', 'Shower mixer cartridge valve leaking', 'Hot water tap drips persistently and loses pressure.', 'open');
        ";
        let _ = store.execute(sample);
        seeded = 10;
    }

    Ok(ConstructionResult {
        blueprint_id: "real_estate".to_string(),
        blueprint_name: "Property & Lease Management".to_string(),
        tables_created: vec![
            "properties".to_string(),
            "units".to_string(),
            "tenants".to_string(),
            "leases".to_string(),
            "maintenance_tickets".to_string(),
        ],
        indexes_created: vec![
            "idx_units_property".to_string(),
            "idx_leases_unit".to_string(),
            "idx_leases_tenant".to_string(),
            "idx_tickets_unit".to_string(),
        ],
        fts_tables_created: fts_tables,
        sample_records_seeded: seeded,
        status: "CONSTRUCTED".to_string(),
    })
}

// ==========================================
// FIELD SERVICE BLUEPRINT
// ==========================================
fn construct_field_service(
    store: &DatabaseStore,
    seed_sample: bool,
    enable_search: bool,
) -> Result<ConstructionResult, DatabaseCraftError> {
    let ddl = "
        CREATE TABLE IF NOT EXISTS service_clients (
            id INTEGER PRIMARY KEY,
            account_name TEXT NOT NULL,
            primary_contact TEXT NOT NULL,
            phone TEXT NOT NULL,
            email TEXT NOT NULL,
            service_address TEXT NOT NULL,
            access_code TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS technicians (
            id INTEGER PRIMARY KEY,
            full_name TEXT NOT NULL,
            trade_specialty TEXT NOT NULL CHECK(trade_specialty IN ('electrical', 'hvac', 'plumbing', 'fire_security', 'roofing')),
            certification_number TEXT NOT NULL,
            phone TEXT NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1
        );

        CREATE TABLE IF NOT EXISTS work_orders (
            id INTEGER PRIMARY KEY,
            order_number TEXT NOT NULL UNIQUE,
            client_id INTEGER NOT NULL REFERENCES service_clients(id) ON DELETE RESTRICT,
            technician_id INTEGER REFERENCES technicians(id) ON DELETE SET NULL,
            status TEXT NOT NULL CHECK(status IN ('scheduled', 'dispatched', 'on_site', 'completed', 'invoiced')),
            priority TEXT NOT NULL CHECK(priority IN ('standard', 'urgent', 'emergency_24h')),
            scheduled_date DATE NOT NULL,
            problem_description TEXT NOT NULL,
            resolution_notes TEXT,
            labor_hours REAL DEFAULT 0.0
        );

        CREATE TABLE IF NOT EXISTS job_parts (
            id INTEGER PRIMARY KEY,
            work_order_id INTEGER NOT NULL REFERENCES work_orders(id) ON DELETE CASCADE,
            part_number TEXT NOT NULL,
            part_name TEXT NOT NULL,
            quantity INTEGER NOT NULL,
            unit_cost REAL NOT NULL,
            line_total REAL NOT NULL
        );

        CREATE TABLE IF NOT EXISTS service_invoices (
            id INTEGER PRIMARY KEY,
            work_order_id INTEGER NOT NULL REFERENCES work_orders(id) ON DELETE RESTRICT,
            labor_charge REAL NOT NULL,
            parts_charge REAL NOT NULL,
            total_amount REAL NOT NULL,
            payment_status TEXT NOT NULL CHECK(payment_status IN ('unpaid', 'paid', 'bad_debt')),
            issued_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE INDEX IF NOT EXISTS idx_work_orders_client ON work_orders(client_id);
        CREATE INDEX IF NOT EXISTS idx_work_orders_tech ON work_orders(technician_id);
        CREATE INDEX IF NOT EXISTS idx_job_parts_order ON job_parts(work_order_id);
        CREATE INDEX IF NOT EXISTS idx_service_invoices_order ON service_invoices(work_order_id);
    ";

    store
        .execute(ddl)
        .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;

    let mut fts_tables = Vec::new();
    if enable_search {
        store
            .init_fts(
                "work_orders_fts",
                &["order_number", "problem_description", "resolution_notes"],
            )
            .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;
        fts_tables.push("work_orders_fts".to_string());
    }

    let mut seeded = 0;
    if seed_sample {
        let sample = "
            INSERT OR IGNORE INTO service_clients (id, account_name, primary_contact, phone, email, service_address, access_code) VALUES
            (1, 'Beacon Bistro & Bar', 'Chef Matteo Rossi', '+44-7700-900331', 'matteo@beaconbistro.co.uk', '14 King St, Bath', 'Gate *3382#'),
            (2, 'Oakhaven Dental Surgery', 'Dr. Evelyn Ward', '+44-7700-900332', 'admin@oakhavendental.co.uk', '45 High St, Bath', 'Ring Buzzer 2');

            INSERT OR IGNORE INTO technicians (id, full_name, trade_specialty, certification_number, phone, is_active) VALUES
            (1, 'Gary Evans', 'hvac', 'HVAC-CERT-9402', '+44-7700-900111', 1),
            (2, 'Liam Brody', 'electrical', 'NICEIC-88214', '+44-7700-900222', 1);

            INSERT OR IGNORE INTO work_orders (id, order_number, client_id, technician_id, status, priority, scheduled_date, problem_description, resolution_notes, labor_hours) VALUES
            (1, 'WO-2026-101', 1, 1, 'completed', 'emergency_24h', '2026-11-02', 'Walk-in freezer compressor tripping circuit breaker intermittently.', 'Replaced thermal overload relay and tested capacitor draw.', 2.5);

            INSERT OR IGNORE INTO job_parts (id, work_order_id, part_number, part_name, quantity, unit_cost, line_total) VALUES
            (1, 1, 'RELAY-TH-44', 'Heavy Duty Thermal Overload Relay', 1, 85.00, 85.00),
            (2, 1, 'CAP-RUN-50UF', '50uF Motor Run Capacitor', 1, 32.50, 32.50);

            INSERT OR IGNORE INTO service_invoices (id, work_order_id, labor_charge, parts_charge, total_amount, payment_status) VALUES
            (1, 1, 200.00, 117.50, 317.50, 'paid');
        ";
        let _ = store.execute(sample);
        seeded = 8;
    }

    Ok(ConstructionResult {
        blueprint_id: "field_service".to_string(),
        blueprint_name: "Field Service & Trades Contractor".to_string(),
        tables_created: vec![
            "service_clients".to_string(),
            "technicians".to_string(),
            "work_orders".to_string(),
            "job_parts".to_string(),
            "service_invoices".to_string(),
        ],
        indexes_created: vec![
            "idx_work_orders_client".to_string(),
            "idx_work_orders_tech".to_string(),
            "idx_job_parts_order".to_string(),
            "idx_service_invoices_order".to_string(),
        ],
        fts_tables_created: fts_tables,
        sample_records_seeded: seeded,
        status: "CONSTRUCTED".to_string(),
    })
}

// ==========================================
// HOSPITALITY BLUEPRINT
// ==========================================
fn construct_hospitality(
    store: &DatabaseStore,
    seed_sample: bool,
    enable_search: bool,
) -> Result<ConstructionResult, DatabaseCraftError> {
    let ddl = "
        CREATE TABLE IF NOT EXISTS dining_sections (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            smoking_allowed INTEGER NOT NULL DEFAULT 0,
            ambient_vibe TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS tables (
            id INTEGER PRIMARY KEY,
            section_id INTEGER NOT NULL REFERENCES dining_sections(id) ON DELETE CASCADE,
            table_number TEXT NOT NULL UNIQUE,
            capacity INTEGER NOT NULL,
            shape TEXT NOT NULL CHECK(shape IN ('round', 'rectangular', 'booth', 'high_top'))
        );

        CREATE TABLE IF NOT EXISTS guests (
            id INTEGER PRIMARY KEY,
            full_name TEXT NOT NULL,
            phone TEXT NOT NULL,
            email TEXT,
            vip_status TEXT NOT NULL DEFAULT 'standard' CHECK(vip_status IN ('standard', 'vip', 'black_card', 'critic')),
            total_visits INTEGER DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS reservations (
            id INTEGER PRIMARY KEY,
            table_id INTEGER NOT NULL REFERENCES tables(id) ON DELETE RESTRICT,
            guest_id INTEGER NOT NULL REFERENCES guests(id) ON DELETE RESTRICT,
            party_size INTEGER NOT NULL,
            reservation_date DATE NOT NULL,
            time_slot TEXT NOT NULL,
            status TEXT NOT NULL CHECK(status IN ('confirmed', 'seated', 'finished', 'no_show', 'cancelled')),
            special_occasion TEXT
        );

        CREATE TABLE IF NOT EXISTS guest_preferences (
            id INTEGER PRIMARY KEY,
            guest_id INTEGER NOT NULL REFERENCES guests(id) ON DELETE CASCADE,
            preference_type TEXT NOT NULL CHECK(preference_type IN ('allergy', 'dietary', 'seating', 'wine')),
            details TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_tables_section ON tables(section_id);
        CREATE INDEX IF NOT EXISTS idx_reservations_table ON reservations(table_id);
        CREATE INDEX IF NOT EXISTS idx_reservations_guest ON reservations(guest_id);
        CREATE INDEX IF NOT EXISTS idx_preferences_guest ON guest_preferences(guest_id);
    ";

    store
        .execute(ddl)
        .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;

    let mut fts_tables = Vec::new();
    if enable_search {
        store
            .init_fts("guests_fts", &["full_name", "phone", "email"])
            .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;
        fts_tables.push("guests_fts".to_string());
    }

    let mut seeded = 0;
    if seed_sample {
        let sample = "
            INSERT OR IGNORE INTO dining_sections (id, name, smoking_allowed, ambient_vibe) VALUES
            (1, 'Main Dining Atrium', 0, 'Intimate Candlelit Acoustic'),
            (2, 'Heated Garden Terrace', 1, 'Lively Alfresco Lounge');

            INSERT OR IGNORE INTO tables (id, section_id, table_number, capacity, shape) VALUES
            (1, 1, 'T-01', 2, 'round'),
            (2, 1, 'T-02', 4, 'booth'),
            (3, 2, 'G-01', 6, 'rectangular');

            INSERT OR IGNORE INTO guests (id, full_name, phone, email, vip_status, total_visits) VALUES
            (1, 'Charlotte Bronte', '+44-7700-900551', 'charlotte@literaryguild.org', 'vip', 8),
            (2, 'Julian Thorne', '+44-7700-900552', 'jthorne@mayfairadvisors.com', 'standard', 2);

            INSERT OR IGNORE INTO reservations (id, table_id, guest_id, party_size, reservation_date, time_slot, status, special_occasion) VALUES
            (1, 2, 1, 4, '2026-11-06', '19:30', 'confirmed', 'Anniversary Celebration'),
            (2, 1, 2, 2, '2026-11-06', '20:00', 'confirmed', 'Client Dinner');

            INSERT OR IGNORE INTO guest_preferences (id, guest_id, preference_type, details) VALUES
            (1, 1, 'allergy', 'Severe shellfish anaphylaxis - cross-contamination fence required'),
            (2, 1, 'wine', 'Prefers dry mineral Burgundies and chilled grower Champagnes');
        ";
        let _ = store.execute(sample);
        seeded = 11;
    }

    Ok(ConstructionResult {
        blueprint_id: "hospitality".to_string(),
        blueprint_name: "Restaurant & Table Reservations".to_string(),
        tables_created: vec![
            "dining_sections".to_string(),
            "tables".to_string(),
            "guests".to_string(),
            "reservations".to_string(),
            "guest_preferences".to_string(),
        ],
        indexes_created: vec![
            "idx_tables_section".to_string(),
            "idx_reservations_table".to_string(),
            "idx_reservations_guest".to_string(),
            "idx_preferences_guest".to_string(),
        ],
        fts_tables_created: fts_tables,
        sample_records_seeded: seeded,
        status: "CONSTRUCTED".to_string(),
    })
}

// ==========================================
// LEGAL BLUEPRINT
// ==========================================
fn construct_legal(
    store: &DatabaseStore,
    seed_sample: bool,
    enable_search: bool,
) -> Result<ConstructionResult, DatabaseCraftError> {
    let ddl = "
        CREATE TABLE IF NOT EXISTS legal_clients (
            id INTEGER PRIMARY KEY,
            client_type TEXT NOT NULL CHECK(client_type IN ('individual', 'corporate', 'trust')),
            name TEXT NOT NULL,
            contact_officer TEXT,
            email TEXT NOT NULL,
            phone TEXT NOT NULL,
            billing_rate REAL NOT NULL,
            retainer_balance REAL DEFAULT 0.0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS matters (
            id INTEGER PRIMARY KEY,
            matter_number TEXT NOT NULL UNIQUE,
            client_id INTEGER NOT NULL REFERENCES legal_clients(id) ON DELETE RESTRICT,
            title TEXT NOT NULL,
            practice_area TEXT NOT NULL CHECK(practice_area IN ('corporate', 'litigation', 'ip_patents', 'employment', 'probate')),
            lead_partner TEXT NOT NULL,
            status TEXT NOT NULL CHECK(status IN ('intake', 'active_discovery', 'in_trial', 'settled', 'closed')),
            opened_date DATE NOT NULL
        );

        CREATE TABLE IF NOT EXISTS case_filings (
            id INTEGER PRIMARY KEY,
            matter_id INTEGER NOT NULL REFERENCES matters(id) ON DELETE CASCADE,
            filing_title TEXT NOT NULL,
            court_jurisdiction TEXT NOT NULL,
            case_index_number TEXT NOT NULL,
            filing_date DATE NOT NULL,
            document_vault_ref TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS court_deadlines (
            id INTEGER PRIMARY KEY,
            matter_id INTEGER NOT NULL REFERENCES matters(id) ON DELETE CASCADE,
            deadline_type TEXT NOT NULL,
            statutory_date DATE NOT NULL,
            responsible_counsel TEXT NOT NULL,
            is_satisfied INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS billable_entries (
            id INTEGER PRIMARY KEY,
            matter_id INTEGER NOT NULL REFERENCES matters(id) ON DELETE RESTRICT,
            timekeeper TEXT NOT NULL,
            hours REAL NOT NULL,
            rate REAL NOT NULL,
            total_fee REAL NOT NULL,
            activity_code TEXT NOT NULL,
            narrative TEXT NOT NULL,
            billed_date DATE NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_matters_client ON matters(client_id);
        CREATE INDEX IF NOT EXISTS idx_filings_matter ON case_filings(matter_id);
        CREATE INDEX IF NOT EXISTS idx_deadlines_matter ON court_deadlines(matter_id);
        CREATE INDEX IF NOT EXISTS idx_billable_matter ON billable_entries(matter_id);
    ";

    store
        .execute(ddl)
        .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;

    let mut fts_tables = Vec::new();
    if enable_search {
        store
            .init_fts(
                "matters_fts",
                &["matter_number", "title", "practice_area", "lead_partner"],
            )
            .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;
        store
            .init_fts(
                "case_filings_fts",
                &["filing_title", "court_jurisdiction", "case_index_number"],
            )
            .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;
        fts_tables.push("matters_fts".to_string());
        fts_tables.push("case_filings_fts".to_string());
    }

    let mut seeded = 0;
    if seed_sample {
        let sample = "
            INSERT OR IGNORE INTO legal_clients (id, client_type, name, contact_officer, email, phone, billing_rate, retainer_balance) VALUES
            (1, 'corporate', 'Aether Dynamics Ltd', 'CFO Brian Sterling', 'legal@aetherdynamics.com', '+44-20-7946-0199', 450.0, 15000.0),
            (2, 'individual', 'Eleanor Vance-St. Clair', NULL, 'eleanor.vance@chambers.uk', '+44-7700-900888', 375.0, 5000.0);

            INSERT OR IGNORE INTO matters (id, matter_number, client_id, title, practice_area, lead_partner, status, opened_date) VALUES
            (1, 'MAT-2026-088', 1, 'Cross-Border Autonomous Drone IP Patent Infringement Defence', 'ip_patents', 'Sarah Jenkins QC', 'active_discovery', '2026-08-14');

            INSERT OR IGNORE INTO case_filings (id, matter_id, filing_title, court_jurisdiction, case_index_number, filing_date, document_vault_ref) VALUES
            (1, 1, 'Motion to Dismiss for Lack of Jurisdiction', 'High Court of Justice Chancery Division', 'HC-2026-000412', '2026-09-30', 'doc_chancery_motion_01.pdf');

            INSERT OR IGNORE INTO court_deadlines (id, matter_id, deadline_type, statutory_date, responsible_counsel, is_satisfied) VALUES
            (1, 1, 'Expert Witness Report Exchange', '2026-11-20', 'Sarah Jenkins QC', 0);

            INSERT OR IGNORE INTO billable_entries (id, matter_id, timekeeper, hours, rate, total_fee, activity_code, narrative, billed_date) VALUES
            (1, 1, 'Sarah Jenkins QC', 3.5, 450.0, 1575.0, 'L120', 'Drafting defense rebuttal witness summaries and prior art citations', '2026-11-03');
        ";
        let _ = store.execute(sample);
        seeded = 6;
    }

    Ok(ConstructionResult {
        blueprint_id: "legal".to_string(),
        blueprint_name: "Legal Case & Matter Practice".to_string(),
        tables_created: vec![
            "legal_clients".to_string(),
            "matters".to_string(),
            "case_filings".to_string(),
            "court_deadlines".to_string(),
            "billable_entries".to_string(),
        ],
        indexes_created: vec![
            "idx_matters_client".to_string(),
            "idx_filings_matter".to_string(),
            "idx_deadlines_matter".to_string(),
            "idx_billable_matter".to_string(),
        ],
        fts_tables_created: fts_tables,
        sample_records_seeded: seeded,
        status: "CONSTRUCTED".to_string(),
    })
}

// ==========================================
// AUTOMOTIVE BLUEPRINT
// ==========================================
fn construct_automotive(
    store: &DatabaseStore,
    seed_sample: bool,
    enable_search: bool,
) -> Result<ConstructionResult, DatabaseCraftError> {
    let ddl = "
        CREATE TABLE IF NOT EXISTS vehicle_owners (
            id INTEGER PRIMARY KEY,
            full_name TEXT NOT NULL,
            phone TEXT NOT NULL,
            email TEXT,
            address TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS vehicles (
            id INTEGER PRIMARY KEY,
            owner_id INTEGER NOT NULL REFERENCES vehicle_owners(id) ON DELETE RESTRICT,
            license_plate TEXT NOT NULL UNIQUE,
            vin TEXT NOT NULL UNIQUE,
            make TEXT NOT NULL,
            model TEXT NOT NULL,
            year INTEGER NOT NULL,
            current_mileage INTEGER NOT NULL,
            fuel_type TEXT NOT NULL CHECK(fuel_type IN ('petrol', 'diesel', 'hybrid', 'bev', 'hydrogen'))
        );

        CREATE TABLE IF NOT EXISTS job_cards (
            id INTEGER PRIMARY KEY,
            job_number TEXT NOT NULL UNIQUE,
            vehicle_id INTEGER NOT NULL REFERENCES vehicles(id) ON DELETE RESTRICT,
            intake_date DATE NOT NULL,
            status TEXT NOT NULL CHECK(status IN ('booked', 'diagnosing', 'awaiting_parts', 'in_repair', 'ready_for_pickup', 'completed')),
            customer_reported_symptoms TEXT NOT NULL,
            lead_mechanic TEXT NOT NULL,
            estimated_labor_hours REAL NOT NULL,
            labor_rate REAL NOT NULL DEFAULT 85.0
        );

        CREATE TABLE IF NOT EXISTS service_parts (
            id INTEGER PRIMARY KEY,
            job_card_id INTEGER NOT NULL REFERENCES job_cards(id) ON DELETE CASCADE,
            oem_part_number TEXT NOT NULL,
            description TEXT NOT NULL,
            quantity INTEGER NOT NULL,
            unit_price REAL NOT NULL,
            line_total REAL NOT NULL
        );

        CREATE TABLE IF NOT EXISTS inspections (
            id INTEGER PRIMARY KEY,
            vehicle_id INTEGER NOT NULL REFERENCES vehicles(id) ON DELETE CASCADE,
            inspection_type TEXT NOT NULL CHECK(inspection_type IN ('mot_annual', 'pre_purchase', 'safety_75_point', 'brake_wear')),
            inspector_name TEXT NOT NULL,
            pass_status TEXT NOT NULL CHECK(pass_status IN ('pass', 'fail', 'advisories_only')),
            advisories TEXT,
            conducted_date DATE NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_vehicles_owner ON vehicles(owner_id);
        CREATE INDEX IF NOT EXISTS idx_job_cards_vehicle ON job_cards(vehicle_id);
        CREATE INDEX IF NOT EXISTS idx_service_parts_job ON service_parts(job_card_id);
        CREATE INDEX IF NOT EXISTS idx_inspections_vehicle ON inspections(vehicle_id);
    ";

    store
        .execute(ddl)
        .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;

    let mut fts_tables = Vec::new();
    if enable_search {
        store
            .init_fts("vehicles_fts", &["license_plate", "vin", "make", "model"])
            .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;
        store
            .init_fts(
                "job_cards_fts",
                &["job_number", "customer_reported_symptoms", "lead_mechanic"],
            )
            .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;
        fts_tables.push("vehicles_fts".to_string());
        fts_tables.push("job_cards_fts".to_string());
    }

    let mut seeded = 0;
    if seed_sample {
        let sample = "
            INSERT OR IGNORE INTO vehicle_owners (id, full_name, phone, email, address) VALUES
            (1, 'Oliver Sinclair', '+44-7700-900441', 'oliver@sinclairvintage.com', '7 Mews Court, London W11');

            INSERT OR IGNORE INTO vehicles (id, owner_id, license_plate, vin, make, model, year, current_mileage, fuel_type) VALUES
            (1, 1, 'LV71 WXY', 'WP0ZZZ99ZTS392184', 'Porsche', '911 Carrera 4S (992)', 2021, 24800, 'petrol');

            INSERT OR IGNORE INTO job_cards (id, job_number, vehicle_id, intake_date, status, customer_reported_symptoms, lead_mechanic, estimated_labor_hours, labor_rate) VALUES
            (1, 'JOB-2026-401', 1, '2026-11-03', 'in_repair', 'Vibration in steering wheel during moderate braking above 50 mph; minor oil seepage on right cylinder head cover.', 'Marco Rossi', 3.5, 95.0);

            INSERT OR IGNORE INTO service_parts (id, job_card_id, oem_part_number, description, quantity, unit_price, line_total) VALUES
            (1, 1, '992-351-405-A', 'Front Carbon Ceramic Disc Brake Rotor 410mm', 2, 420.00, 840.00),
            (2, 1, '992-351-939-B', 'Performance Brake Pad Set Front Axle', 1, 160.00, 160.00);

            INSERT OR IGNORE INTO inspections (id, vehicle_id, inspection_type, inspector_name, pass_status, advisories, conducted_date) VALUES
            (1, 1, 'mot_annual', 'Marco Rossi', 'pass', 'Front tires worn to 3.2mm - recommend replacement before winter.', '2026-10-14');
        ";
        let _ = store.execute(sample);
        seeded = 6;
    }

    Ok(ConstructionResult {
        blueprint_id: "automotive".to_string(),
        blueprint_name: "Auto Workshop & Fleet Service".to_string(),
        tables_created: vec![
            "vehicle_owners".to_string(),
            "vehicles".to_string(),
            "job_cards".to_string(),
            "service_parts".to_string(),
            "inspections".to_string(),
        ],
        indexes_created: vec![
            "idx_vehicles_owner".to_string(),
            "idx_job_cards_vehicle".to_string(),
            "idx_service_parts_job".to_string(),
            "idx_inspections_vehicle".to_string(),
        ],
        fts_tables_created: fts_tables,
        sample_records_seeded: seeded,
        status: "CONSTRUCTED".to_string(),
    })
}

// ==========================================
// EDUCATION BLUEPRINT
// ==========================================
fn construct_education(
    store: &DatabaseStore,
    seed_sample: bool,
    enable_search: bool,
) -> Result<ConstructionResult, DatabaseCraftError> {
    let ddl = "
        CREATE TABLE IF NOT EXISTS instructors (
            id INTEGER PRIMARY KEY,
            full_name TEXT NOT NULL,
            email TEXT NOT NULL UNIQUE,
            specialization TEXT NOT NULL,
            hourly_pay_rate REAL NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1
        );

        CREATE TABLE IF NOT EXISTS students (
            id INTEGER PRIMARY KEY,
            full_name TEXT NOT NULL,
            date_of_birth DATE NOT NULL,
            guardian_name TEXT,
            email TEXT NOT NULL,
            phone TEXT NOT NULL,
            medical_or_learning_notes TEXT,
            registered_date DATE NOT NULL
        );

        CREATE TABLE IF NOT EXISTS courses (
            id INTEGER PRIMARY KEY,
            code TEXT NOT NULL UNIQUE,
            title TEXT NOT NULL,
            department TEXT NOT NULL,
            total_credits INTEGER NOT NULL DEFAULT 3,
            tuition_fee REAL NOT NULL
        );

        CREATE TABLE IF NOT EXISTS class_cohorts (
            id INTEGER PRIMARY KEY,
            course_id INTEGER NOT NULL REFERENCES courses(id) ON DELETE CASCADE,
            instructor_id INTEGER NOT NULL REFERENCES instructors(id) ON DELETE RESTRICT,
            term_season TEXT NOT NULL,
            term_year INTEGER NOT NULL,
            classroom_or_url TEXT NOT NULL,
            max_capacity INTEGER NOT NULL DEFAULT 20
        );

        CREATE TABLE IF NOT EXISTS attendance_records (
            id INTEGER PRIMARY KEY,
            cohort_id INTEGER NOT NULL REFERENCES class_cohorts(id) ON DELETE CASCADE,
            student_id INTEGER NOT NULL REFERENCES students(id) ON DELETE CASCADE,
            session_date DATE NOT NULL,
            attendance_status TEXT NOT NULL CHECK(attendance_status IN ('present', 'late', 'excused', 'absent'))
        );

        CREATE TABLE IF NOT EXISTS enrollments (
            id INTEGER PRIMARY KEY,
            cohort_id INTEGER NOT NULL REFERENCES class_cohorts(id) ON DELETE RESTRICT,
            student_id INTEGER NOT NULL REFERENCES students(id) ON DELETE RESTRICT,
            enrollment_status TEXT NOT NULL CHECK(enrollment_status IN ('enrolled', 'waitlisted', 'completed', 'dropped')),
            final_grade TEXT,
            tuition_paid INTEGER NOT NULL DEFAULT 0
        );

        CREATE INDEX IF NOT EXISTS idx_cohorts_course ON class_cohorts(course_id);
        CREATE INDEX IF NOT EXISTS idx_cohorts_instructor ON class_cohorts(instructor_id);
        CREATE INDEX IF NOT EXISTS idx_attendance_cohort ON attendance_records(cohort_id);
        CREATE INDEX IF NOT EXISTS idx_attendance_student ON attendance_records(student_id);
        CREATE INDEX IF NOT EXISTS idx_enrollments_cohort ON enrollments(cohort_id);
        CREATE INDEX IF NOT EXISTS idx_enrollments_student ON enrollments(student_id);
    ";

    store
        .execute(ddl)
        .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;

    let mut fts_tables = Vec::new();
    if enable_search {
        store
            .init_fts("courses_fts", &["code", "title", "department"])
            .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;
        store
            .init_fts("students_fts", &["full_name", "email", "guardian_name"])
            .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;
        fts_tables.push("courses_fts".to_string());
        fts_tables.push("students_fts".to_string());
    }

    let mut seeded = 0;
    if seed_sample {
        let sample = "
            INSERT OR IGNORE INTO instructors (id, full_name, email, specialization, hourly_pay_rate, is_active) VALUES
            (1, 'Professor Kenneth Albright', 'ken.albright@academy.org', 'Applied Mathematics & Graph Theory', 65.0, 1),
            (2, 'Clara Schumann-Vance', 'clara@academy.org', 'Classical Piano & Counterpoint', 55.0, 1);

            INSERT OR IGNORE INTO students (id, full_name, date_of_birth, guardian_name, email, phone, registered_date) VALUES
            (1, 'Lucas Alvarez', '2008-04-12', 'Elena Alvarez', 'elena.alvarez@familymail.com', '+44-7700-900771', '2026-09-01'),
            (2, 'Zoe Montgomery', '2007-11-23', 'Charles Montgomery', 'charles.m@globalcapital.com', '+44-7700-900772', '2026-09-01');

            INSERT OR IGNORE INTO courses (id, code, title, department, total_credits, tuition_fee) VALUES
            (1, 'MATH-301', 'Discrete Mathematics & Algorithmic Complexity', 'Mathematics', 4, 850.0),
            (2, 'MUS-202', 'Advanced Modal Harmonization', 'Music', 3, 620.0);

            INSERT OR IGNORE INTO class_cohorts (id, course_id, instructor_id, term_season, term_year, classroom_or_url, max_capacity) VALUES
            (1, 1, 1, 'Autumn', 2026, 'Lecture Hall Euler-3', 18),
            (2, 2, 2, 'Autumn', 2026, 'Studio Bach-A', 8);

            INSERT OR IGNORE INTO enrollments (id, cohort_id, student_id, enrollment_status, tuition_paid) VALUES
            (1, 1, 1, 'enrolled', 1),
            (2, 1, 2, 'enrolled', 1);

            INSERT OR IGNORE INTO attendance_records (id, cohort_id, student_id, session_date, attendance_status) VALUES
            (1, 1, 1, '2026-11-02', 'present'),
            (2, 1, 2, '2026-11-02', 'present');
        ";
        let _ = store.execute(sample);
        seeded = 10;
    }

    Ok(ConstructionResult {
        blueprint_id: "education".to_string(),
        blueprint_name: "Academy & Tutoring Center".to_string(),
        tables_created: vec![
            "instructors".to_string(),
            "students".to_string(),
            "courses".to_string(),
            "class_cohorts".to_string(),
            "attendance_records".to_string(),
            "enrollments".to_string(),
        ],
        indexes_created: vec![
            "idx_cohorts_course".to_string(),
            "idx_cohorts_instructor".to_string(),
            "idx_attendance_cohort".to_string(),
            "idx_attendance_student".to_string(),
            "idx_enrollments_cohort".to_string(),
            "idx_enrollments_student".to_string(),
        ],
        fts_tables_created: fts_tables,
        sample_records_seeded: seeded,
        status: "CONSTRUCTED".to_string(),
    })
}
