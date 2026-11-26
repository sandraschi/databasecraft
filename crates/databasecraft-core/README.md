# `databasecraft-core` (L1 Domain Engine & Relational Canvas Compiler)

Pure-Rust, UI-agnostic core engine for **DatabaseCraft** (`#![deny(clippy::unwrap_used)]`).

## Modules

| Module | Purpose |
|---|---|
| `lib.rs` | Universal `Command::dispatch` bus shared by CLI, MCP, and GUI. |
| `canvas.rs` | Non-1NF Relational Canvas AST, relation sprouter (`1:N` / `N:1`), compact join inliner, and SQLite 3.38+ `json_group_array` query compiler. |
| `blueprints.rs` & `seed.rs` | Turnkey domain constructors (`small_business`, `billing`, `club`, `surgery`, `creative_agency`) and commercial demo datasets (Northwind, Ecommerce, Film, Executive KPIs). |
| `notifications.rs` | Craft Notification Bus (`info`, `warning`, `critical`, `disaster` ring buffer) and autonomous CTO Resident Watchdog. |
| `export.rs` | Cross-craft merge payloads for DesignCraft, DeckCraft, WordCraft, GridCraft, and PdfCraft. |
| `ai.rs` / `models.rs` / `error.rs` | NLQ/AI copilot helpers, schema models, and typed `DatabaseCraftError`. |
