# DatabaseCraft

**DatabaseCraft** is the pure-Rust relational database, schema engine, and dynamic data-merge platform built for the **Storytold ArtCraft** creative ecosystem and the **24/7 Executive/CFO Relational Cockpit**.

```
                           [ DatabaseCraft ] (Control Port 14113)
                           /        |        \               \
                          v         v         v               v
                    DesignCraft  DeckCraft  WordCraft     CFO 24/7 Cockpit
                  (Catalog Spread) (Live Deck) (Mail Merge) (Relational Canvas)
```

---

## What is DatabaseCraft?

DatabaseCraft solves the classic chasm between **spreadsheets** (which cannot handle plural $1:N$ relationships without VLOOKUP spaghetti) and **traditional SQL tools** (which duplicate rows via Cartesian join explosion).

Inspired by the MIT CSAIL *Relational Canvas* research (*SIEUFERD*) and commercialized by tools like Ultorg ($35/mo), DatabaseCraft brings **Non-1NF hierarchical perspectives** directly to desktop Rust:
* **Nested Subqueries Without Row Duplication**: View Parent Entities, nested Invoices, and nested Line Items in a single continuous visual spreadsheet.
* **1-Click Relation Sprouting**: Discover forward ($N:1$) and reverse ($1:N$) foreign keys and sprout them into child tables with one click.
* **Contextual Inline Rollups**: Real-time atomic metrics (`SUM`, `COUNT`, `AVG`) calculated directly within parent headers.
* **24/7 Executive Cockpit**: Ultra-lightweight (sub-30MB RAM), zero-panic Rust engine running continuously on a CFO's second monitor.
* **Cross-Craft Dynamic Publishing**: Stream verified hierarchical data directly into **DesignCraft** catalogs, **DeckCraft** board slides, and **WordCraft** contracts.

---

## Features

- **Pure-Rust & Zero-Panic Policy**: Workspace-wide `#![deny(clippy::unwrap_used)]`. Every error is a strongly typed `Result<T, DatabaseCraftError>`.
- **Relational Canvas & SQLite JSON Compiler**: Compiles nested visual perspective ASTs into high-performance SQLite 3.38+ `json_group_array(json_object(...))` queries in sub-milliseconds.
- **Embedded & Local-First**: SQLite-backed `.dbcraft` storage with WAL mode, foreign key enforcement, and dense vector embeddings (`Cosine`, `DotProduct`, `L2`).
- **Universal Command Bus (`Command::dispatch`)**: 100% of CLI subcommands, MCP `tools/call`, and desktop UI actions dispatch through the exact same typed Rust bus.
- **Agent-Ready MCP Server**: Built-in line-delimited stdio Model Context Protocol (`MCP 2024-11-05`) server on registered control port `14113`.
- **Turnkey Domain Constructor**: Instantly construct battle-tested schemas for `small_business`, `billing` (Stripe mirror), `club`, `surgery`, and `creative_agency`.

---

## Quickstart

### 1. Status & Schema Discovery
```powershell
# Show status and registered control port 14113
cargo run -p databasecraft-mcp -- status

# Run CTO resident watchdog check (detects disk starvation, quiescence churn anomalies)
cargo run -p databasecraft-mcp -- watchdog

# List active notifications from the Craft Notification Bus
cargo run -p databasecraft-mcp -- notifications

# Seed deterministic ecommerce catalog
cargo run -p databasecraft-mcp -- seed-demo --name ecommerce

# Discover forward (N:1) and reverse (1:N) relations for a table
cargo run -p databasecraft-mcp -- schema-relations --table categories
```

### 2. Execute Relational Canvas Perspectives
```powershell
# Query raw table
cargo run -p databasecraft-mcp -- query --sql "SELECT id, name FROM categories;"

# Export formatted multi-app merge for DesignCraft
cargo run -p databasecraft-mcp -- export-merge --query "SELECT sku, title, price, hero_image FROM products;" --target designcraft

# Run MCP server over stdio for AI agents
cargo run -p databasecraft-mcp -- mcp
```

### 3. Launch Desktop GUI (or Headless CI Verification)
```powershell
# Run desktop app in headless smoke verification mode
cargo run -p databasecraft-app -- --smoke

# Launch native GPU desktop application on control port 14113
cargo run -p databasecraft-app -- --control 14113
```

---

## Deep Documentation

* 📄 [**The CFO & Executive 24/7 Relational Cockpit**](docs/CFO_RELATIONAL_CANVAS.md) — Operational and architectural guide for running continuous financial perspectives.
* 📄 [**Product Requirements Document (PRD)**](docs/PRD.md) — 5-tier system architecture, functional specs (F1–F23), and containment rules.
* 📄 [**Cross-Craft Multi-App Integration**](docs/CROSS_CRAFT_FLOWS.md) — Export payload specifications for DesignCraft, DeckCraft, WordCraft, GridCraft, and PdfCraft.
* 📄 [**Hybrid RAG & Vector Facility**](docs/RAG_FACILITY.md) — In-process dense vector embeddings and SQLite FTS5 Reciprocal Rank Fusion (RRF).

---

## Status & Lineage

**Does today:** 
- Core/Store/MCP/App crates compiling under strict `#![deny(clippy::unwrap_used)]`.
- Relational Canvas AST, SQLite JSON query compiler, and relation sprouter.
- Continuous Spanned Spreadsheet Matrix with multi-tier table ownership headers, 1px hairline borders, and density control (Compact, Standard, Relaxed).
- Smart cell auto-formatter: currency commas (`$1,850.00`), delivery/order status pill badges, tabular numeric alignment, and monospace dates.
- Realistic commercial demo datasets: Northwind Traders (`customers -> orders -> order_items`), Ecommerce, Film Production, and Executive KPIs.
- Generalized Craft Notification Bus with severity levels (`info`, `warning`, `critical`, `disaster`) and ring buffer.
- Autonomous CTO Watchdog with quiescent table mutation anomaly detection (e.g. `employees`, `roles`).
- Desktop GUI with 11 integrated tabs: `Table Data`, `✨ Relational Canvas`, `SQL & NLQ Console`, `Schema ERD`, `AI Copilot`, `RAG Vectors`, `Profiler`, `Doctor`, `Domain Constructor`, `Cross-Craft Export`, `🚨 Alerts & Watchdog`.
- 27 automated tests passing with zero warnings.
- MCP stdio framing (`MCP 2024-11-05`) + TCP Plane 2 on control port `14113`.

**Lineage:**  
Storytold ArtCraft ([storytold/artcraft](https://github.com/storytold/artcraft) — *"the IDE for artists"*) was the starting point. The 16 craft apps in the Storytold stable each ship one binary that is both human CLI and agent MCP server (stdio), headless-first, with `run.json` and report artifacts. DatabaseCraft provides the relational keystone for this ecosystem.
