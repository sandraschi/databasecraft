# DatabaseCraft — FastMCP & CLI Tools Specification

**Protocol Version**: MCP `2024-11-05`  
**Parent Standard**: [`mcp-central-docs/standards/RUST_ARTCRAFT_STANDARD.md`](../../mcp-central-docs/standards/RUST_ARTCRAFT_STANDARD.md)  
**Registered Port**: `14113` [Plane 2 Active]  
**Architecture**: Headless-First Pure-Rust (`databasecraft-mcp`, `databasecraft-core`, `databasecraft-store`)  
**Status**: Active Production Standard (v0.7.0)

---

## 1. Executive Summary & Transport Architecture

DatabaseCraft provides a typed, zero-panic Model Context Protocol (MCP) server designed for autonomous coding agents (Gemini, Claude Opus, Codex) and cross-craft desktop automation. Every MCP tool directly maps 1:1 to the internal typed `Command::dispatch` bus and has an equivalent command-line interface subcommand in `databasecraft-cli`.

### Dual Transport Layers

```
                     ┌──────────────────────────────────────────────┐
                     │           Autonomous Agent / CLI             │
                     └───────┬──────────────────────────────┬───────┘
                             │ Stdio JSON-RPC               │ Localhost TCP (Plane 2)
                             v                              v
                     ┌──────────────────────────────────────────────┐
                     │          databasecraft-mcp (Port 14113)     │
                     └──────────────────────┬───────────────────────┘
                                            │ typed Command::dispatch
                                            v
                     ┌──────────────────────────────────────────────┐
                     │  databasecraft-core & databasecraft-store    │
                     │  (Non-1NF Canvas, rusqlite WAL, Vectors)    │
                     └──────────────────────────────────────────────┘
```

1. **Stdio JSON-RPC 2.0**: Line-delimited JSON-RPC framing over standard input/output (`databasecraft-mcp mcp`). Diagnostic logs, telemetry, and backtraces are strictly directed to `stderr`.
2. **Localhost TCP Plane 2 (`14113`)**: Active bidirectional loopback listener on port 14113 handling tool execution, live database telemetry, and cross-craft payload publishing.

---

## 2. FastMCP Tool Catalog

The DatabaseCraft MCP server exports **29 typed tools** categorized into 8 functional domains.

### Domain 1: Core Relational & Schema Discovery

| Tool Name | Description | Key Parameters |
|:---|:---|:---|
| **`db_status`** | Returns engine status, table counts, database size, and registered control port 14113. | *(None)* |
| **`db_query`** | Executes an ANSI SQL statement against the embedded SQLite store with parameterized protection. | `sql` (string) |
| **`db_schema_inspect`** | Retrieves column definitions, data types, nullability, primary keys, and foreign keys. | `table` (optional string) |
| **`db_record_insert`** | Inserts a structured JSON object as a table row with strict schema validation. | `table` (string), `record` (object) |
| **`db_schema_relations`** | Discovers forward ($N:1$) and reverse ($1:N$) foreign-key links for relation sprouting and compact joins. | `table` (string) |
| **`db_update_cell`** | Atomically updates a single cell identified by its primary key with audit logging and undo tracking. | `table` (string), `pk_col` (string), `pk_val` (any), `col` (string), `new_val` (any) |

### Domain 2: Relational Canvas (Non-1NF Perspective Engine)

| Tool Name | Description | Key Parameters |
|:---|:---|:---|
| **`canvas_query`** | Executes a hierarchical Non-1NF Relational Canvas specification (`CanvasSpec`) compiling directly into SQLite 3.38+ `json_group_array(json_object(...))` queries without Cartesian row multiplication. | `spec` (`CanvasSpec` object) |
| **`canvas_compile`** | Compiles a `CanvasSpec` AST into raw ANSI/SQLite SQL with correlated subqueries and aggregations without executing it. | `spec` (`CanvasSpec` object) |

### Domain 3: Cross-Craft Dynamic Merges & Seeding

| Tool Name | Description | Key Parameters |
|:---|:---|:---|
| **`db_export_merge`** | Queries the database and formats row records into typed payloads for DesignCraft, DeckCraft, WordCraft, GridCraft, or PdfCraft. | `query` (string), `target` (`designcraft` \| `deckcraft` \| `wordcraft` \| `gridcraft` \| `pdfcraft`), `name` (optional string) |
| **`db_seed_demo`** | Seeds deterministic reference datasets for validation and automated workflows. | `name` (`ecommerce` \| `film` \| `kpi` \| `northwind`) |

### Domain 4: Vector Embeddings & Hybrid RAG

| Tool Name | Description | Key Parameters |
|:---|:---|:---|
| **`db_rag_embed`** | Generates a deterministic, in-process 384-dimensional dense float vector from input text without cloud calls. | `text` (string), `dimensions` (optional int) |
| **`db_vector_search`** | Performs SIMD-optimized dense vector distance searches (`cosine`, `dot_product`, `l2`). | `table` (string), `vector` (array of float), `metric` (optional string), `top_k` (optional int) |
| **`db_hybrid_search`** | Combines dense vector distance with sparse SQLite FTS5 BM25 keyword matching via Reciprocal Rank Fusion (RRF). | `vector_table` (string), `fts_table` (string), `query_text` (string), `query_vector` (array of float), `top_k` (optional int) |
| **`db_semantic_search`** | End-to-end natural language search: embeds text in-process and executes hybrid RRF or dense vector search. | `table` (string), `query` (string), `fts_table` (optional string), `top_k` (optional int) |
| **`db_ingest_document`** | Chunks markdown/text files into semantic sections and stores them simultaneously in relational, vector, and FTS5 tables. | `title` (string), `content` (string), `tags` (optional array of string) |

### Domain 5: AI Copilot & Automated Workflows

| Tool Name | Description | Key Parameters |
|:---|:---|:---|
| **`db_nlq`** | Translates natural language inquiries into schema-aware, safe SQL queries with explanatory commentary. | `prompt` (string) |
| **`db_semantic_dedupe`** | Scans text columns for semantic near-duplicates using vector similarity clustering ($0.0 \le \tau \le 1.0$). | `table` (string), `column` (string), `threshold` (optional float) |
| **`db_schema_advisor`** | Audits database schemas for missing foreign key indexes, unindexed high-cardinality search fields, and FTS5 candidates. | *(None)* |
| **`db_generate_synthetic`** | Synthesizes realistic, referentially intact mock records matching column types and foreign key relationships. | `table` (string), `count` (int), `insert` (optional bool) |

### Domain 6: Diagnostics, Repair & Storage Maintenance

| Tool Name | Description | Key Parameters |
|:---|:---|:---|
| **`db_data_profile`** | Computes cardinality, null ratios, min/max bounds, and distinct values for all columns in a table. | `table` (string) |
| **`db_diagnose`** | Performs deep B-Tree page integrity verification and detects foreign-key orphan rows or structural corruption. | *(None)* |
| **`db_repair`** | Executes automatic repair sequences: `reindex` B-Trees, `purge_orphans`, or `salvage` surviving records to a clean database clone. | `action` (`reindex` \| `purge_orphans` \| `salvage`), `salvage_path` (optional string) |
| **`db_vacuum`** | Reclaims unused disk pages and defragments the SQLite database file and WAL. | *(None)* |
| **`db_backup`** | Creates an atomic, online backup snapshot to a target file path. | `path` (string) |

### Domain 7: Turnkey Schema Blueprints

| Tool Name | Description | Key Parameters |
|:---|:---|:---|
| **`db_list_blueprints`** | Lists all production-grade domain schema blueprints available for instant initialization. | *(None)* |
| **`db_construct`** | Instantiates a complete multi-table schema blueprint (`small_business`, `billing` [Stripe mirror], `club`, `surgery`, `creative_agency`) with optional sample data and search indexes. | `blueprint` (string), `seed_sample_data` (optional bool), `enable_search_indexes` (optional bool) |

### Domain 8: CTO Resident Watchdog & Notification Bus

| Tool Name | Description | Key Parameters |
|:---|:---|:---|
| **`db_watchdog_check`** | Evaluates overall database health, disk starvation thresholds, and sliding-window quiescent table mutations. | *(None)* |
| **`db_notifications_list`** | Queries active notifications from the global bounded ring buffer with severity filtering (`info`, `warning`, `critical`, `disaster`). | `severity` (optional string), `unacknowledged_only` (optional bool) |
| **`db_notification_ack`** | Acknowledges an active operational notification by its unique UUID. | `id` (string) |

---

## 3. CLI Subcommand Reference

Every MCP tool is directly accessible from the command line via `databasecraft-cli`:

```powershell
# Engine status & port verification
databasecraft-cli status --json

# Run SQL query
databasecraft-cli query --sql "SELECT * FROM products LIMIT 5;" --json

# Seed commercial Northwind Traders dataset
databasecraft-cli seed-demo --name northwind --json

# Sprout schema relations
databasecraft-cli schema-relations --table customers --json

# In-place cell atomic mutation
databasecraft-cli update-cell --table products --pk-col id --pk-val 1 --col price --new-val 29.99 --json

# Run CTO watchdog health assessment
databasecraft-cli watchdog --json

# List active notifications
databasecraft-cli notifications --unack-only --json

# Construct turnkey billing schema
databasecraft-cli construct --blueprint billing --seed --indexes --json

# Launch line-delimited MCP stdio server
databasecraft-cli mcp
```

---

## 4. Error Handling & Safety Invariants

1. **Zero Panics**: All tools adhere to `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]`. Errors are serialized as standard JSON-RPC error responses with typed codes:
   ```json
   {
     "jsonrpc": "2.0",
     "id": 1,
     "error": {
       "code": -32602,
       "message": "Invalid identifier: table contains illegal characters"
     }
   }
   ```
2. **Identifier Sanitization**: All table and column names pass through strict regex verification (`^[a-zA-Z_][a-zA-Z0-9_]*$`). Arbitrary SQL injection in table names is strictly rejected.
3. **Quiescence Tracking**: Whenever `db_update_cell` or SQL queries mutate sensitive configuration tables (`employees`, `roles`, `categories`), the mutation is audited by the CTO Watchdog and emits events to the Craft Notification Bus.
