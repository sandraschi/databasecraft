# DatabaseCraft — Tiered Execution Plan

**Parent Standard:** [`RUST_ARTCRAFT_STANDARD.md`](../../mcp-central-docs/standards/RUST_ARTCRAFT_STANDARD.md)  
**Menu Bar Standard:** [`CRAFT_MENU_BAR_STANDARD.md`](../../mcp-central-docs/standards/rust-artcraft/CRAFT_MENU_BAR_STANDARD.md)  
**Architecture Model:** 4-Tier ArtCraft Incremental Delivery  
**Status:** Active  

---

## Architecture Flow

```
Tier 1: Core Engine ──> Tier 2: MCP / CLI ──> Tier 3: Desktop UI ──> Tier 4: Menu Bar, Help & Packaging
(databasecraft-core)   (databasecraft-mcp)   (databasecraft-app)   (Hardened Workstation)
```

---

## Tier Specifications

### Tier 1 — Core Relational Engine & Storage (`databasecraft-core`, `databasecraft-store`)
- [x] Embedded SQLite engine with WAL mode, foreign keys, and busy timeout.
- [x] Universal `Command::dispatch` execution bus.
- [x] Hostile input sanitization fences (`validate_identifier`).
- [x] Deterministic seed datasets (`ecommerce`, `film`, `kpi`).
- [x] Dense vector math (Cosine, DotProduct, L2) and SQLite FTS5 BM25 hybrid RAG.
- [x] Non-1NF Relational Perspective AST compiler (`json_group_array`).
- [x] Bidirectional Foreign Key relation sprouting engine.
- [x] `#![forbid(unsafe_code)]` and zero-panic clippy lints.

### Tier 2 — Headless Driver & FastMCP (`databasecraft-mcp`)
- [x] Single CLI binary `databasecraft-cli` with `--json` output.
- [x] FastMCP stdio server communicating over JSON-RPC 2.0.
- [x] Complete tool catalog mirroring `Command::dispatch` operations.
- [x] Cross-craft export payloads (DesignCraft, DeckCraft, WordCraft, GridCraft, PdfCraft).
- [x] Headless `--smoke` verification without window initialization.

### Tier 3 — Desktop Workstation UI (`databasecraft-app`)
- [x] Immediate-mode `egui` 0.37 / `wgpu` GPU rendering at 60–144 FPS.
- [x] 11 integrated workspaces: Table Grid, Relational Canvas, SQL & NLQ Console, Schema ERD, RAG & Vectors, Alerts & Watchdog, etc.
- [x] Continuous spanned spreadsheet matrix with smart typographic cell formatters.
- [x] Double-click atomic in-place cell editing with multi-step Undo/Redo.
- [x] 24/7 Executive CFO Cockpit and CTO Quiescence Watchdog.

### Tier 4 — Craft Menu Bar Standard, Help Suite & Hardened Verification
- [x] Universal 8-menu bar (`File`, `Edit`, `View`, `Insert`, `Modify`, `Commands`, `Window`, `Help`) with `switch_on_hover`.
- [x] Mandatory 5-dialog Help suite (`Command Palette...`, `Keyboard Shortcuts...`, `Project Files & Docs...`, `Licenses & Notices...`, `About DatabaseCraft`).
- [x] Fleet UI/UX Pattern 2 adherence: viewport-centric navigation, top button row mapped to View toggles, panel visibility managed via `View` / `Window`.
- [x] Tofu Guard verification (100% font glyph coverage, zero tofu placeholder boxes).
- [x] Headless `--smoke` parity validation.
- [x] Standalone release binary packaging and NSIS installer.
