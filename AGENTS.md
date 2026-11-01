# DatabaseCraft - AGENTS.md

**Parent Hub:** [`mcp-central-docs/standards/RUST_ARTCRAFT_STANDARD.md`](../mcp-central-docs/standards/RUST_ARTCRAFT_STANDARD.md)
**Containment:** [`mcp-central-docs/standards/rust-artcraft/CONTAINMENT_AND_CRATES.md`](../mcp-central-docs/standards/rust-artcraft/CONTAINMENT_AND_CRATES.md)
**Command Bus:** [`mcp-central-docs/standards/rust-artcraft/CRAFT_COMMAND_BUS.md`](../mcp-central-docs/standards/rust-artcraft/CRAFT_COMMAND_BUS.md)

## Role in the Storytold ArtCraft Suite

DatabaseCraft is the pure-Rust relational database, schema engine, and data merge keystone for the ArtCraft creative suite:
- **Registered Control Port:** `14113`
- **File Container:** `.dbcraft` (SQLite-backed relational bundle with embedded schema metadata, query definitions, and form layouts)
- **Identity:** Relational data, schemas, and dynamic forms for creative pipelines
- **Color:** Deep Emerald / Obsidian Cyan (`#108B7E`)

## 5-Layer Containment Harness

1. **Layer 1 (Zero-Panic Policy):** All crate roots declare `clippy::deny(unwrap_used, expect_used, panic, unimplemented, todo)`. All errors return `Result<T, DatabaseCraftError>`.
2. **Layer 2 (Three-Tier Workspace Seam):**
   - `crates/databasecraft-store`: L1 storage engine wrapping `rusqlite` (WAL mode, foreign keys ON).
   - `crates/databasecraft-core`: L1 models, cross-craft exports, demo seeds, and universal `Command::dispatch`.
   - `crates/databasecraft-mcp`: L2 CLI-first binary (`databasecraft-cli`) + MCP `stdio` server (`MCP 2024-11-05`).
   - `crates/databasecraft-app`: L3 desktop app calling `databasecraft-core::dispatch` in-process with zero WebView2.
3. **Layer 3 (The Universal Command Bus):** CLI subcommands, MCP `tools/call`, and UI actions all route through `Command::dispatch`.
4. **Layer 4 (Deterministic Demo Datasets):** `ecommerce`, `film`, and `kpi` deterministic datasets provide immediate verifiable schemas.
5. **Layer 5 (Definition of Done):** `just check`, `just test`, `just fmt-check` must be green.

## Stdio Invariant for MCP

In `databasecraft-cli mcp` mode:
- `stdout` is **100% pure line-delimited JSON-RPC**.
- Every log line or diagnostic MUST go to `stderr` (`eprintln!`).
