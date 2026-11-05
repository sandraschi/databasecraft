# DatabaseCraft

**DatabaseCraft** is the pure-Rust relational database, schema engine, and dynamic data-merge platform built for the **Storytold ArtCraft** creative ecosystem.

```
       [ DatabaseCraft ] (Control Port 14113)
        /       |       \
       v        v        v
 DesignCraft DeckCraft WordCraft
(Catalog Spread) (Live Deck) (Mail Merge)
```

## Features

- **Pure-Rust & Zero-Panic**: Enforces `deny(clippy::unwrap_used)` throughout.
- **Embedded & Local-First**: SQLite-backed `.dbcraft` storage with WAL mode, ACID transactions, and foreign key enforcement.
- **Universal Command Bus (`Command::dispatch`)**: CLI subcommands, MCP `tools/call`, and desktop UI execute the exact same typed commands.
- **Cross-Craft Data Merge**: Export query results directly formatted for:
  - **DesignCraft**: Multi-page catalog and spread data merge.
  - **DeckCraft**: Slide-ready KPI tables and charts.
  - **WordCraft**: Mail-merge dictionaries and report variables.
  - **GridCraft**: Seamless CSV/TSV table interchange.
  - **PdfCraft**: Direct report and ledger layout tables.
- **Agent-Ready MCP Server**: Built-in line-delimited stdio Model Context Protocol (MCP `2024-11-05`) server.

## Quickstart

```powershell
# Show status and control port
cargo run -p databasecraft-mcp -- status

# Seed deterministic ecommerce demo data
cargo run -p databasecraft-mcp -- seed-demo --name ecommerce

# Query products table
cargo run -p databasecraft-mcp -- query --sql "SELECT sku, title, price FROM products;"

# Export formatted data merge for DesignCraft
cargo run -p databasecraft-mcp -- export-merge --query "SELECT sku, title, price, hero_image FROM products;" --target designcraft

# Run Model Context Protocol (MCP) server
cargo run -p databasecraft-mcp -- mcp

# Run desktop app in headless smoke verification mode
cargo run -p databasecraft-app -- --smoke
```

## Status — does today / next week / non-goals

**Does today:** core/store/mcp/app with workspace clippy denies; real CLI
(status/query/schema/record/seed/embed/search); 17 test fns; docs (4);
`status` reports its control port.

**Next week:** password-URL redaction test; binary-name decision
(`dbcraft-cli.exe`); 3 PLAN recipes green + compat baseline.

**Non-goals:** backbone, not app — blueprints compile in AppCraft;
Postgres/MySQL paths need dockerized CI or stay sqlite-labeled.

## Lineage — the Storytold craft-app pattern

Storytold ArtCraft ([storytold/artcraft](https://github.com/storytold/artcraft) — "the IDE for artists") was the starting point. The 16 craft apps in the Storytold stable each ship one binary that is both the human CLI and the agent MCP server (stdio), headless-first, with `run.json` + report artifacts. That modus operandi and harness is what this repo implements; the code here is independent, the pattern is shared.
