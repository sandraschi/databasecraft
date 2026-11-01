# Changelog

All notable changes to **DatabaseCraft** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

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
