# DatabaseCraft — Handover Document

**Last Updated**: 2026-11-11  
**Current State**: Production-Ready Relational Canvas & CTO Watchdog (v0.6.0)  
**Primary Artifact**: [sandraschi/databasecraft](https://github.com/sandraschi/databasecraft)  
**Registered Port**: `14113`  
**Automated Tests**: 27 unit & integration tests passing (100% green, 0 warnings)  

---

## 1. What Was Done (v0.6.0 Release)

1. **Relational Canvas AST & SQLite JSON Query Compiler (MIT CSAIL SIEUFERD / Ultorg Model)**:
   - Built `crates/databasecraft-core/src/canvas.rs` (620 lines) modeling Non-1NF perspectives (`CanvasSpec`).
   - Compiler converts hierarchical AST into SQLite 3.38+ `json_group_array(json_object(...))` queries in sub-milliseconds without Cartesian join explosion.
   - Bidirectional relationship resolver (`SchemaRelationsResolver`) auto-detects forward ($N:1$) and reverse ($1:N$) foreign keys.
   - Commands added: `canvas.query`, `canvas.compile_sql`, `db.schema_relations`.
   - Exposed over CLI subcommands and MCP stdio tools (`db_canvas_query`, `db_canvas_compile_sql`, `db_schema_relations`).

2. **Continuous Spanned Spreadsheet Matrix Grid (`egui 0.37`)**:
   - Implemented `ActiveTab::Canvas` in `databasecraft-app`.
   - Replaced card-based layout with a true 2D spreadsheet matrix:
     - Tier 1: Table ownership header bands (Emerald root, Navy subqueries, Amber rollups).
     - Tier 2: Field name headers with 1px hairline cell dividers (`#262A32`) and zebra striping (`#121418` / `#181B21`).
     - Aligned child rows flowing directly alongside parent records without duplicating parent cells.
   - 1-click relation sprouting pills (`+ ⤹ Sprout (1:N)`, `+ 🔗 Sprout (N:1)`).
   - Contextual inline rollups (`SUM`, `COUNT`, `AVG`, `MIN`, `MAX`) calculated in parent headers.
   - Field Selector palette modal (`Ctrl+K`) and live `⚡ SQL Drawer`.
   - Visual density switcher: `Compact` (22px), `Standard` (28px), and `Relaxed` (34px).

3. **Typographic Smart Cell Auto-Formatter**:
   - Currency auto-formatting with thousand separators and dollar symbols (`$1,850.00`) in green monospace tabular digits.
   - Status pill badges with soft filled backgrounds and colored outlines (`[DELIVERED]`, `[SHIPPED]`, `[PROCESSING]`, `[PENDING]`, `[NG]`).
   - Tabular numeric alignment, quantity formatting, and monospace date pills.

4. **Realistic Commercial Benchmark Dataset: Northwind Traders**:
   - Added `northwind` dataset with 3-tier deep relations: `customers -> orders -> order_items`.
   - One-click seed launcher in top command bar: `🚢 Seed: Northwind Traders`.

5. **CTO Resident Watchdog & Generalized Craft Notification Bus**:
   - Built `crates/databasecraft-core/src/notifications.rs`: thread-safe bounded ring buffer `global_notification_bus`.
   - `MutationTracker`: sliding 60-second window monitoring quiescent tables (`employees`, `roles`, `categories`), detecting unexpected high-frequency churn (>10 mut/min).
   - Disk starvation monitor checking DB file and WAL growth envelopes.
   - Commands: `notifications.list`, `notifications.ack`, `watchdog.check`, `watchdog.rules`, `watchdog.record_mutation`.
   - Desktop UI: dynamic top-bar alert badge (`🔔 Alerts` / `🚨 X Alerts`) and 11th workspace tab `🚨 Alerts & Watchdog` with 3-column diagnostic metrics and 1-click remediation actions.

---

## 2. Immediate Next Steps

1. **In-Place Cell Editing with Primary Key Write-Back**:
   - Double-clicking a cell in the Canvas grid triggers inline editing, executing `UPDATE <table> SET <col> = <val> WHERE <pk> = <id>;` with multi-step undo/redo.
2. **Compact Joins (Foreign Key Inlining)**:
   - For single-column foreign keys, inline the referenced display label (e.g., showing `customer.company_name` in place of `orders.customer_id`) without requiring an extra sprouted column hierarchy.
3. **DuckDB / Parquet External Data Connectors**:
   - Allow the Relational Canvas compiler to target external DuckDB and Parquet files for analytical perspectives over gigabyte-scale datasets.

---

## 3. Deliberately NOT Done (Do Not Rediscover as Surprises)

- **No WebView2 / Electron / Node Runtime**: All desktop views are 100% native Rust GPU immediate-mode rendering via `egui 0.37` and `eframe`. Never introduce web view dependencies.
- **No Unsafe / Panic Code**: The entire workspace is bound by `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]`. Any unwrap will break CI.
- **No Cartesian Join Multi-Table Queries**: The Relational Canvas AST strictly compiles to SQLite JSON group arrays and correlated subqueries rather than flat SQL joins to prevent row multiplication.
- **No Direct Port Hardcoding**: Use registered control port `14113` per `WEBAPP_PORTS.md`.

---

## 4. Verification Commands

```powershell
# Run all 27 automated tests
cargo test --manifest-path "d:\Dev\repos\databasecraft\Cargo.toml"

# Run clippy across all targets
cargo clippy --manifest-path "d:\Dev\repos\databasecraft\Cargo.toml" --all-targets

# Headless smoke verification
cargo run --manifest-path "d:\Dev\repos\databasecraft\Cargo.toml" -p databasecraft-app -- --smoke
```
