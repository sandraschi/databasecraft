# DatabaseCraft Help & Operator Guide

Pure-Rust relational database, Non-1NF **Relational Canvas** (MIT CSAIL *SIEUFERD* / Ultorg-style nested perspectives), and cross-craft data-merge platform (`databasecraft-core`, `databasecraft-store`, `databasecraft-mcp`, `databasecraft-app`).

## 1. Get Started

1. **Launch the Desktop App**: Run `cargo run -p databasecraft-app -- --control 14113` (or launch `databasecraft-app.exe` from the Start Menu).
2. **Seed a Realistic Demo Dataset**: Open **Domain Constructor** (or run `databasecraft-mcp seed-demo --name ecommerce` / `northwind` / `film` / `executive`) to populate a local `.dbcraft` SQLite database with multi-table foreign keys.
3. **Explore the Relational Canvas**: Open the **Relational Canvas** tab to inspect parent rows with nested `1:N` child tables (for example `customers -> orders -> order_items`) in one continuous hierarchical spreadsheet without Cartesian row duplication.
4. **Sprout Relations & Inline Rollups**: Click any discovered forward (`N:1`) or reverse (`1:N`) foreign key to sprout child perspectives or inline human-readable compact joins, and add real-time `SUM` / `COUNT` / `AVG` rollups in parent headers.
5. **Edit Safely & Publish Cross-Craft**: Double-click cells for primary-key write-back with multi-step Undo/Redo, then open **Cross-Craft Export** to emit verified hierarchical payloads for **DesignCraft**, **DeckCraft**, **WordCraft**, **GridCraft**, or **PdfCraft**.

## 2. Functions & Architecture

- **Relational Canvas & SQLite JSON Compiler (`databasecraft-core/src/canvas.rs`)**: Compiles nested visual perspective ASTs into SQLite 3.38+ `json_group_array(json_object(...))` queries in sub-milliseconds.
- **Local-First SQLite & Vector Store (`databasecraft-store`)**: `.dbcraft` WAL-mode SQLite storage with foreign key enforcement, FTS5 full-text search, and in-process dense vector embeddings (`Cosine`, `DotProduct`, `L2`) with Reciprocal Rank Fusion (RRF).
- **Turnkey Domain Constructor (`blueprints.rs`, `seed.rs`)**: One-click schemas for `small_business`, `billing`, `club`, `surgery`, `creative_agency`, Northwind Traders, Ecommerce, Film Production, and Executive KPIs.
- **CTO Resident Watchdog & Craft Notification Bus (`notifications.rs`)**: Detects disk starvation and quiescent table mutation anomalies (e.g. `employees`, `roles`) and publishes structured severity events (`info`, `warning`, `critical`, `disaster`).
- **Universal Command Bus & MCP (`databasecraft-mcp`)**: 100% of CLI, MCP stdio (`MCP 2026-07-28`), and desktop UI actions execute through `Command::dispatch` on registered control port `14113`.

## 3. Usage (GUI, CLI & MCP)

- **GUI (`databasecraft-app`)**:
  - Top 8-Menu Bar: `File`, `Edit`, `View`, `Insert`, `Modify`, `Commands`, `Window`, `Help`
  - `Ctrl+K` / `Ctrl+Shift+P` — Command Palette
  - `F1` / `Ctrl+/` — Keyboard Shortcuts
  - Drag column dividers to resize (double-click to reset); drag headers to reorder and recompile the live perspective AST.
- **CLI (`databasecraft-mcp`)**:
  ```powershell
  databasecraft-mcp status
  databasecraft-mcp watchdog
  databasecraft-mcp notifications
  databasecraft-mcp seed-demo --name ecommerce
  databasecraft-mcp schema-relations --table categories
  databasecraft-mcp query --sql "SELECT id, name FROM categories;"
  databasecraft-mcp export-merge --query "SELECT sku, title, price, hero_image FROM products;" --target designcraft
  ```
- **MCP (`stdio`)**: Run `databasecraft-mcp mcp` (pure JSON-RPC 2.0 on `stdout`).

## 4. Tips & Tricks

- **Compact Joins Over Raw Foreign Keys**: Enable compact joins to project `customers.company_name` directly inline instead of opaque numeric `customer_id` columns.
- **Density Control**: Switch between `Compact`, `Standard`, and `Relaxed` matrix density for 24/7 second-monitor executive cockpit viewing.
- **Smart Cell Formatting**: Currency columns (`$1,850.00`), order/delivery status pills, tabular numerals, and monospace timestamps format automatically.

## 5. Common Pitfalls & Safety

- **Zero-Unwrap Policy**: Workspace compiles under `#![deny(clippy::unwrap_used)]`; every failure returns a typed `DatabaseCraftError`.
- **Quiescent Table Auditing**: Mutating low-churn tables (`employees`, `roles`) triggers CTO Watchdog audit notifications automatically.
- **Headless CI Verification**: Always verify desktop builds without a GPU via `cargo run -p databasecraft-app -- --smoke`.

## 6. Deep Docs & Crate Sub-READMEs

- Crate Docs: [databasecraft-core](crates/databasecraft-core/README.md), [databasecraft-store](crates/databasecraft-store/README.md), [databasecraft-mcp](crates/databasecraft-mcp/README.md), [databasecraft-app](crates/databasecraft-app/README.md)
- Architecture Docs: [CFO Relational Canvas](docs/CFO_RELATIONAL_CANVAS.md), [CTO Resident Watchdog](docs/CTO_RESIDENT_WATCHDOG.md), [PRD](docs/PRD.md), [Cross-Craft Flows](docs/CROSS_CRAFT_FLOWS.md), [RAG Facility](docs/RAG_FACILITY.md)
