//! Deterministic demo datasets for DatabaseCraft and Cross-Craft Multi-App flows.
use crate::error::DatabaseCraftError;
use databasecraft_store::DatabaseStore;

pub enum DemoKind {
    Ecommerce,
    FilmProduction,
    ExecutiveKpi,
}

impl DemoKind {
    pub fn parse(s: &str) -> Result<Self, DatabaseCraftError> {
        match s.to_ascii_lowercase().as_str() {
            "ecommerce" | "products" | "catalog" => Ok(DemoKind::Ecommerce),
            "film" | "film_production" | "shots" => Ok(DemoKind::FilmProduction),
            "kpi" | "executive" | "metrics" => Ok(DemoKind::ExecutiveKpi),
            other => Err(DatabaseCraftError::InvalidInput(format!(
                "unknown demo dataset '{other}'. Valid options: ecommerce, film, kpi"
            ))),
        }
    }
}

pub fn seed_demo_dataset(
    store: &DatabaseStore,
    kind: DemoKind,
) -> Result<usize, DatabaseCraftError> {
    match kind {
        DemoKind::Ecommerce => seed_ecommerce(store),
        DemoKind::FilmProduction => seed_film_production(store),
        DemoKind::ExecutiveKpi => seed_executive_kpis(store),
    }
}

fn seed_ecommerce(store: &DatabaseStore) -> Result<usize, DatabaseCraftError> {
    let ddl = "
        CREATE TABLE IF NOT EXISTS categories (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            slug TEXT NOT NULL UNIQUE
        );

        CREATE TABLE IF NOT EXISTS products (
            id INTEGER PRIMARY KEY,
            sku TEXT NOT NULL UNIQUE,
            title TEXT NOT NULL,
            category_id INTEGER REFERENCES categories(id),
            price REAL NOT NULL,
            inventory INTEGER NOT NULL,
            hero_image TEXT NOT NULL
        );

        INSERT INTO categories (id, name, slug) VALUES
            (1, 'Audio Hardware', 'audio-hardware'),
            (2, 'Optics & Lenses', 'optics-lenses'),
            (3, 'Studio Furniture', 'studio-furniture');

        INSERT INTO products (id, sku, title, category_id, price, inventory, hero_image) VALUES
            (1, 'AUD-900', 'Obsidian Field Monitor Headphones', 1, 349.00, 48, 'assets/products/aud-900.png'),
            (2, 'MIC-042', 'Reference Boundary Ribbon Microphone', 1, 799.00, 15, 'assets/products/mic-042.png'),
            (3, 'OPT-35A', 'Anamorphic Cinema Prime 35mm T2.0', 2, 1850.00, 8, 'assets/products/opt-35a.png'),
            (4, 'OPT-85A', 'Anamorphic Cinema Prime 85mm T2.2', 2, 2100.00, 6, 'assets/products/opt-85a.png'),
            (5, 'DSK-PRO', 'Motorized Modular Grading Console Desk', 3, 2450.00, 4, 'assets/products/dsk-pro.png');
    ";
    store
        .execute(ddl)
        .map_err(|e| DatabaseCraftError::Storage(e.to_string()))
}

fn seed_film_production(store: &DatabaseStore) -> Result<usize, DatabaseCraftError> {
    let ddl = "
        CREATE TABLE IF NOT EXISTS scenes (
            id INTEGER PRIMARY KEY,
            scene_number TEXT NOT NULL UNIQUE,
            location TEXT NOT NULL,
            time_of_day TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS takes (
            id INTEGER PRIMARY KEY,
            scene_id INTEGER NOT NULL REFERENCES scenes(id),
            shot_number TEXT NOT NULL,
            take_number INTEGER NOT NULL,
            status TEXT NOT NULL,
            sound_take TEXT NOT NULL,
            lens TEXT NOT NULL,
            notes TEXT
        );

        INSERT INTO scenes (id, scene_number, location, time_of_day) VALUES
            (1, 'SC-01', 'EXT. VIENNA ROOFTOP', 'DAWN'),
            (2, 'SC-02', 'INT. LAB COMMAND DESK', 'NIGHT'),
            (3, 'SC-03', 'EXT. OBSERVATORY TERRACE', 'GOLDEN HOUR');

        INSERT INTO takes (id, scene_id, shot_number, take_number, status, sound_take, lens, notes) VALUES
            (1, 1, '1A', 1, 'NG', 'T01_A1.WAV', '35mm', 'Lens flare on camera left'),
            (2, 1, '1A', 2, 'PRINT', 'T01_A2.WAV', '35mm', 'Hero master take, sharp lock'),
            (3, 2, '2B', 1, 'HOLD', 'T02_B1.WAV', '50mm', 'Audio boom dip on frame 120'),
            (4, 2, '2B', 2, 'PRINT', 'T02_B2.WAV', '50mm', 'Flawless performance and sync'),
            (5, 3, '3A', 1, 'PRINT', 'T03_A1.WAV', '85mm', 'Perfect sunset timing, beautiful grade');
    ";
    store
        .execute(ddl)
        .map_err(|e| DatabaseCraftError::Storage(e.to_string()))
}

fn seed_executive_kpis(store: &DatabaseStore) -> Result<usize, DatabaseCraftError> {
    let ddl = "
        CREATE TABLE IF NOT EXISTS quarterly_metrics (
            id INTEGER PRIMARY KEY,
            quarter TEXT NOT NULL UNIQUE,
            arr_usd REAL NOT NULL,
            mrr_usd REAL NOT NULL,
            gross_margin_pct REAL NOT NULL,
            net_churn_pct REAL NOT NULL,
            active_seats INTEGER NOT NULL
        );

        INSERT INTO quarterly_metrics (id, quarter, arr_usd, mrr_usd, gross_margin_pct, net_churn_pct, active_seats) VALUES
            (1, '2025-Q4', 1240000.0, 103333.0, 78.5, 1.2, 4200),
            (2, '2026-Q1', 1580000.0, 131666.0, 81.2, 0.9, 5800),
            (3, '2026-Q2', 2050000.0, 170833.0, 83.0, 0.7, 7900),
            (4, '2026-Q3', 2720000.0, 226666.0, 84.8, 0.5, 11200);
    ";
    store
        .execute(ddl)
        .map_err(|e| DatabaseCraftError::Storage(e.to_string()))
}
