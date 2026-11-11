# Changelog

All notable changes to **DatabaseCraft** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.7.0] - 2026-11-11

### Added
- **Universal Craft 8-Menu Bar & Help Suite Standard**:
  - Implemented the full Storytold Craft Application Menu Bar Standard ([`CRAFT_MENU_BAR_STANDARD.md`](../../mcp-central-docs/standards/rust-artcraft/CRAFT_MENU_BAR_STANDARD.md)) under Pattern 2 (Viewport-Centric Workstation).
  - Pinned `egui::Panel::top("top_menubar")` featuring the universal 8-menu structure:
    - **`File`**: New In-Memory Database (`Ctrl+N`), Open SQLite Database (`Ctrl+O`), Open Recent ▸ (with persistent list & clearing), Save Perspective Snapshot (`Ctrl+S`), Save Perspective As (`Ctrl+Shift+S`), Seed Presets ▸ (`Northwind`, `Ecommerce`, `Film`, `KPI`), Export Delivery Data ▸ (`DeckCraft`, `DesignCraft`, `WordCraft`, `GridCraft`), Close Perspective (`Ctrl+W`), Quit (`Ctrl+Q`).
    - **`Edit`**: Undo Cell Edit (`Ctrl+Z`), Redo Cell Edit (`Ctrl+Y`), Cut (`Ctrl+X`), Copy (`Ctrl+C`), Paste (`Ctrl+V`), Select All (`Ctrl+A`), Fields & Joins Palette (`Ctrl+K`), Preferences & Fences (`Ctrl+,`).
    - **`View`**: 10 direct workspace accelerators (`F2` Canvas, `F3` Tables, `F4` SQL Console, `F5` Schema ERD, `F6` RAG, `F7` Profiler, `F8` Doctor, `F9` Constructor, `F10` Exporter, `F11` Alerts & Watchdog), Canvas Density switcher (`Compact`, `Standard`, `Relaxed`), Live SQL Drawer toggle, Reset Column Widths, and dock visibility toggles (Navigation Rail, Quick Action Toolbar, Status Footer).
    - **`Insert`**: Sprout Reverse Relation (1:N Subtable), Sprout Forward Relation (N:1 Lookup), Inline Foreign Key Label (Compact Join), Add Aggregate Rollup (SUM/COUNT/AVG), Insert Document into Vector RAG, Synthesize Synthetic Table Data.
    - **`Modify`**: Fields & Joins Palette, Reorder Columns & Fields, Reset All Column Widths, Vacuum & Defragment SQLite Database, Diagnose & Auto-Repair Database.
    - **`Commands`**: Compile Canvas to SQLite JSON AST, Execute AI Natural Language Query (NLQ), Run Deduplication Analysis, Run AI Schema Advisor & Index Recommender, Compute Hybrid BM25 + Dense Vector Search, Evaluate Quiescent Table Mutation Churn, Check Disk & WAL Starvation Envelopes.
    - **`Window`**: Reset Panel Layout, direct workspace focus, and modular visibility toggles for Navigation Rail, Top Action Toolbar, Status Footer, and Live SQL Drawer.
    - **`Help`**: Mandatory 5-dialog transparency suite.
  - Implemented the complete 5-dialog modal Help suite with center anchoring and scrollable viewports:
    - `🔍 Command Palette` (`Ctrl+K` / `Ctrl+Shift+P`): Live searchable fuzzy query over all engine actions, navigation targets, seeds, and maintenance routines with instant one-click execution.
    - `⌨ Keyboard Shortcuts Cheat Sheet` (`F1` / `Ctrl+/`): Categorized table of global, workspace, canvas matrix, and cell editing hotkeys.
    - `📄 Project Files & Documentation`: Interactive document browser linking to `PRD.md`, `CFO_RELATIONAL_CANVAS.md`, `CTO_RESIDENT_WATCHDOG.md`, `TIERED_PLAN.md`, `TOOLS.md`, and `CRAFT_MENU_BAR_STANDARD.md`.
    - `📜 Licenses & Open Source Notices`: Pure Rust `#![forbid(unsafe_code)]` compliance, offline sovereignty guarantee, dual MIT/Apache-2.0 notice, and third-party crate attributions.
    - `ℹ About DatabaseCraft`: Application version, architecture overview, registered control port `14113` [Plane 2 Active], and ecosystem role.
  - Added Workstation Preferences dialog (`Ctrl+,`) with theme switcher, density controls, dock visibility flags, and security fence monitors.
  - Global hotkeys handler intercepting function keys (`F1`–`F11`), `Ctrl+K`, `Ctrl+Shift+P`, `Ctrl+,`, `Ctrl+Z`, `Ctrl+Y` with safe cell editing isolation.
- **Interactive Column Resizing & Pixel-Perfect 2D Grid Alignment**:
  - Implemented real-time drag divider handles (6px interactive hit zone) between column headers in Tier 2 with visual color highlights (emerald on hover, electric cyan while dragging).
  - Synchronized pixel-perfect widths enforced from Tier 2 column headers down to every data cell in the continuous spreadsheet matrix.
  - Double-clicking any column resize divider handle instantly auto-resets the column to density default width.
  - Added `CanvasColumn::width_px` and `CanvasInlinedLookup::width_px` preserving persistent layout geometries in the Canvas perspective AST.
  - Added top-bar quick action `⇲ Reset Widths` clearing manual overrides and restoring standard density metrics.
- **Drag-and-Drop Column Reordering (Ultorg-Style)**:
  - Column headers in Tier 2 support intuitive click-and-drag reordering with `egui::CursorIcon::Grabbing`.
  - Dynamic floating drag badge following the mouse pointer showing `⇄ Reordering Column: [NAME]`.
  - Target drop zone feedback with an animated 3px vertical sky-blue landing line indicating exact insertion placement (before or after target column).
  - Reordering immediately updates `CanvasSpec::columns`, reorders projections without table schema mutation, and triggers live AST recompilation.
  - Added `CanvasSpec::reorder_columns` and `reorder_slice` AST helper algorithms.
  - Enhanced Fields & Joins Palette (`Ctrl+K`) with `[▲]` / `[▼]` manual reordering controls alongside visual drag-and-drop.
- **In-Place Cell Editing & Multi-Step Undo/Redo Stack**:
  - Double-clicking any cell in the Canvas continuous matrix grid switches into an active `egui::TextEdit` input with `[✓ Commit]` (`Enter`) and `[✖ Cancel]` (`Esc`).
  - Atomic primary key write-back via `Command::UpdateCell` executing parameterized `UPDATE <table> SET <col> = ? WHERE <pk> = ?`.
  - Multi-step Undo (`Ctrl+Z`) and Redo (`Ctrl+Y`) stack integrated directly into the Canvas top toolbar and hotkeys.
  - Cell mutations automatically log to the quiescent `MutationTracker`, triggering the CTO Watchdog and emitting `NotificationCategory::MutationAudit` events on the generalized Craft Notification Bus.
  - Added CLI subcommand `update-cell` and MCP tool `db_update_cell`.
- **Compact Joins (Foreign Key Inlining)**:
  - Added `CanvasInlinedLookup` to `CanvasSpec` and `CanvasNestedRelation`.
  - Allows inlining referenced human-readable labels (e.g. `company_name` for `customer_id` in orders) as scalar subqueries without sprouting entire child/parent table column hierarchies.
  - Auto-suggests candidate human-readable display columns (`name`, `title`, `company_name`, `label`).
  - UI offers 1-click `+ 🏷️ Inline: <rel>` buttons in the Relations Bar, rendering inlined lookups in a dedicated Lilac Header Band with removal pills (`✖`).
  - Evaluates in sub-milliseconds with zero Cartesian row inflation.
- **Interactive Tri-State Column Header Sorting**:
  - Tri-state sorting (`ASC` -> `DESC` -> `None`) triggered by clicking column headers in the 2D Canvas Matrix.
  - Visual glyph indicators (`▲` for ascending, `▼` for descending) directly in column headers.
  - Recompiles the perspective AST and refetches data dynamically.
- **Automated Tests**:
  - Expanded test suite to 34 tests covering the Craft Menu Bar standard, modal help dialog transitions, command palette query filtering, dock panel toggles, interactive column resizing, drag-and-drop column reordering, AST column permutations, in-place cell editing, compact joins, primary key extraction, mutation auditing, and GUI undo/redo stacks.

## [0.6.0] - 2026-11-11

### Added
- **Relational Canvas AST & SQLite JSON Query Compiler (SIEUFERD / Ultorg Paradigm)**:
  - Implemented `crates/databasecraft-core/src/canvas.rs`: AST types `CanvasSpec`, `CanvasColumn`, `CanvasNestedRelation`, `CanvasAggregation`, `CanvasFilter`, `CanvasSort`.
  - Implemented `CanvasQueryCompiler` generating high-performance SQLite 3.38+ `json_group_array(json_object(...))` queries executing in sub-milliseconds with zero Cartesian join explosion.
  - Implemented bidirectional schema relationship resolver discovering forward ($N:1$) and reverse ($1:N$) foreign keys.
  - Added Command Bus variants `Command::CanvasQuery`, `Command::CanvasCompileSql`, and `Command::SchemaRelations`.
  - Added CLI subcommands `canvas-query`, `canvas-compile`, and `schema-relations`.
  - Added MCP tools `db_canvas_query`, `db_canvas_compile_sql`, and `db_schema_relations`.
- **Continuous Spanned Spreadsheet Matrix Grid (`egui 0.37`)**:
  - Transformed `ActiveTab::Canvas` in `databasecraft-app` from nested cards into a true 2D continuous matrix grid.
  - Multi-tier table ownership headers: Emerald root band (`🏛️ CUSTOMERS`), Navy subquery bands (`↳ ORDERS`), and Amber rollup bands (`∑ ROLLUPS`).
  - 1px hairline cell dividers (`#262A32`) with zebra striping (`#121418` / `#181B21`).
  - 1-click relation sprouting pills (`+ ⤹ Sprout (1:N)`, `+ 🔗 Sprout (N:1)`).
  - Contextual inline rollups (`SUM`, `COUNT`, `AVG`, `MIN`, `MAX`) calculated in parent headers without row duplication.
  - Visual density controller: `Compact` (22px row height), `Standard` (28px row height), and `Relaxed` (34px row height).
  - Global `🎛 Fields & Joins Palette` (`Ctrl+K`) and live `⚡ SQL Drawer`.
- **Smart Typographic Cell Auto-Formatter**:
  - Automatic currency formatting with thousand commas and dollar symbols (`$1,850.00`) in green monospace tabular digits.
  - Status pill badges with soft filled backgrounds and colored outlines (`[DELIVERED]`, `[SHIPPED]`, `[PROCESSING]`, `[PENDING]`, `[NG]`).
  - Tabular numeric alignment, quantity formatting, and monospace date pills.
- **Realistic Commercial Benchmark Dataset: Northwind Traders**:
  - Added `northwind` dataset with 3-tier connected entities: `customers -> orders -> order_items`.
  - Added top command bar one-click launcher `🚢 Seed: Northwind Traders`.
- **CTO Resident Watchdog & Generalized Craft Notification Bus**:
  - Implemented `crates/databasecraft-core/src/notifications.rs`: thread-safe bounded ring buffer with severity ratings (`info`, `warning`, `critical`, `disaster`).
  - Added `MutationTracker` tracking sliding 60-second mutation velocity on static tables (`employees`, `roles`, `categories`), flagging unexpected high-frequency churn.
  - Added proactive disk starvation detection for database file and WAL size envelopes with automated `VACUUM` suggestion.
  - Added commands `notifications.list`, `notifications.ack`, `watchdog.check`, `watchdog.rules`, `watchdog.record_mutation`.
  - Added CLI subcommands `notifications` and `watchdog`, plus MCP tools `db_notifications_list`, `db_notification_ack`, `db_watchdog_check`.
  - Added top-bar dynamic alert badge (`🔔 Alerts` / `🚨 X Alerts`) and 11th desktop workspace tab: `🚨 Alerts & Watchdog` with 3-column diagnostic metrics and 1-click remediation actions.
- **Comprehensive Executive Documentation**:
  - Created `docs/CFO_RELATIONAL_CANVAS.md`: 24/7 executive cockpit operational guide.
- **Test Suite**:
  - Expanded automated test suite from 17 to 27 tests (100% passing, 0 warnings, strict zero-panic `#![deny(clippy::unwrap_used)]`).

## [0.5.0] - 2026-11-01

### Added
- **In-Process Semantic Embeddings (`KBZ-002`)**:
  - Implemented pure-Rust deterministic subword character n-gram feature projection with unit L2 normalization (`generate_text_embedding`).
  - Zero external ONNX/C++ runtimes or cloud calls; runs in microseconds entirely offline.
  - Added Command Bus entries `rag.embed_text` (`CMD_RAG_EMBED`) and high-level `rag.semantic_search` (`CMD_RAG_ASK`) accepting natural language query text directly.
  - Added CLI subcommands `databasecraft-cli embed <text>` and `databasecraft-cli semantic-search <query>`.
  - Added MCP tools `db_rag_embed` and `db_semantic_search`.
- **Visual ERD Schema Inspector**:
  - Transformed the Schema workspace into an interactive visual card canvas displaying column types, `[PK]` badges, NOT NULL indicators, and directional foreign-key connector links (`↳ from → to(col)`).
- **Interactive Grid Filtering**:
  - Added instant case-insensitive text filtering in the Table Data Grid tab.
- **RAG & Semantic Retrieval Workbench UI**:
  - Added interactive semantic query executor displaying fused RRF and vector similarity scores alongside hit metadata.
- **Expanded Command Bus**:
  - Expanded the registered ArtCraft command catalog from 15 to 17 commands.

## [0.4.0] - 2026-11-01

### Added
- **Craft Two-Plane Live Control Architecture**:
  - **Plane 1 (In-Process Headless MCP)**: Default `databasecraft-cli mcp` operates as a stand-alone JSON-RPC stdio server over direct SQLite storage.
  - **Plane 2 (Live GUI Attachment via TCP)**: `databasecraft.exe` launches an asynchronous `tokio` TCP control listener on `127.0.0.1:14113`. `databasecraft-cli mcp --attach 14113` bridges external agent stdio into the live GUI runtime.
- **Craft Command Bus Registry & Introspection**:
  - Implemented `databasecraft-cli commands [--json]` displaying all 15 registered bus commands, IDs, human-readable labels, and operational descriptions.
  - Implemented `databasecraft-cli describe <command_id> [--json]` providing machine-readable parameter signatures and parameter type definitions.
- **Automated Regression & Parity Tests**:
  - Added `test_command_registry_and_describe` in `databasecraft-core` asserting signature integrity across all 15 commands.
  - Added `test_control_channel_tcp_loopback` in `databasecraft-app` asserting live JSON-RPC `initialize` and `tools/call` over loopback TCP sockets.
- **Network Stack Capabilities**:
  - Configured `tokio` with `net` and `io-util` features for clean async TCP transport.

## [0.3.0] - 2026-11-01

### Added
- **Full GPU-Accelerated Desktop UI (`databasecraft-app`)**:
  - Implemented 5-view native immediate-mode desktop canvas using `egui 0.37` and `eframe` running at 60–144 FPS with zero WebView2.
  - **Top Bar**: Live control port indicator (`Port 14113 [Ready]`), quick deterministic demo seed triggers (`Ecommerce`, `Film`, `KPI`), and global Dark/Light theme toggle.
  - **Table Data Grid**: Virtualized high-fps cell grid rendering records, columns, and data types directly from SQLite.
  - **SQL Query Console**: Interactive query editor with execution telemetry, error reporting, and pretty-printed results.
  - **Schema & ERD**: Complete relational schema and foreign-key relation inspector.
  - **RAG & Vector Workbench**: Interactive semantic retrieval and BM25 testing workbench.
  - **Cross-Craft Data Merge**: Dynamic merge payload generator and preview for DesignCraft, DeckCraft, WordCraft, GridCraft, and PdfCraft.
  - Retained instantaneous `--smoke` headless CI verification mode.

## [0.2.0] - 2026-11-01

### Added
- **Modern RAG & Vector Facility in `databasecraft-store`**:
  - Pure-Rust SIMD-friendly vector metrics: `cosine_similarity`, `dot_product`, and `l2_distance`.
  - Packed little-endian float vector serialization (`pack_vector`, `unpack_vector` with `as_chunks::<4>()`).
  - Dense vector table initialization and vector insertion (`init_vector_table`, `insert_vector`).
  - Vector similarity search (`search_vector`) with top-k truncation and metadata extraction.
  - SQLite **FTS5** sparse keyword indexing and BM25 search (`init_fts`, `search_fts`).
  - **Hybrid Search Engine** combining dense vector similarity with sparse BM25 matches using Reciprocal Rank Fusion (RRF, $k=60.0$).
- **Universal Command Bus RAG Commands**:
  - `Command::VectorInit { table, dimensions }`
  - `Command::VectorInsert { table, id, vector, metadata }`
  - `Command::VectorSearch { table, vector, metric, top_k }`
  - `Command::FtsInit { table, columns }`
  - `Command::FtsSearch { table, query, top_k }`
  - `Command::HybridSearch { vector_table, fts_table, query_text, query_vector, top_k }`
- **MCP RAG Tools (`databasecraft-cli mcp`)**:
  - `db_vector_search`: Query dense vector collections with configurable metrics.
  - `db_hybrid_search`: Unified semantic + keyword search over linked tables.
- **Unit & Oracle Tests**:
  - `test_vector_insert_and_cosine_search`: Verifies near-exact cosine retrieval.
  - `test_hybrid_rag_search`: Verifies RRF fusion across SQLite FTS5 and vector tables.
- **Documentation**:
  - Comprehensive `docs/PRD.md` (Product Requirements Document).
  - `docs/KIBITZ_INBOX.md` for asynchronous observer advice, critiques, and triage.
  - `docs/RAG_FACILITY.md` technical architecture guide.
  - `docs/CROSS_CRAFT_FLOWS.md` inter-app protocol specification.

---

## [0.1.0] - 2026-11-01

### Added
- **Workspace Scaffolding**: Pure-Rust virtual workspace (`Cargo.toml`) enforcing `clippy::deny(unwrap_used, expect_used, panic, unimplemented, todo)`.
- **L1 `databasecraft-store`**: Local-first relational storage engine on `rusqlite 0.40.2` (WAL mode, foreign keys ON, busy timeout).
- **L1 `databasecraft-core`**:
  - Schema definition models (`TableDef`, `ColumnDef`, `DataType`).
  - Hostile input fence (`validate_identifier`).
  - Universal Command Bus (`enum Command` and async `dispatch`).
  - Deterministic demo datasets (`ecommerce`, `film_production`, `executive_kpis`).
  - Cross-craft export transforms for **DesignCraft** (data merge), **DeckCraft** (slides), **WordCraft** (mail-merge), **GridCraft** (CSV), and **PdfCraft** (ledger tables).
- **L2 `databasecraft-mcp`**:
  - CLI binary (`databasecraft-cli`) with subcommands `status`, `query`, `schema-inspect`, `record-insert`, `export-merge`, `seed-demo`, and `mcp`.
  - Line-delimited MCP `2024-11-05` stdio server with strict pure JSON-RPC on `stdout` and diagnostic logs on `stderr`.
- **L3 `databasecraft-app`**:
  - Native desktop shell calling `core::dispatch` in-process with zero WebView2.
  - `--smoke` headless verification flag for rapid CI testing.
  - In-process parity unit test (`test_in_process_gui_parity`).
- **Fleet Registration**:
  - Control port **`14113`** registered in `mcp-central-docs/operations/WEBAPP_PORTS.md`.
  - Multi-app Flows 13, 14, and 15 registered in `mcp-central-docs/patterns/CRAFT_MULTI_APP_FLOWS.md`.
