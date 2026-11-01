# DatabaseCraft — Kibitz Inbox

**Purpose**: The **Kibitz Inbox** is the project's asynchronous ledger for capturing observations, unsolicited advice, agent-to-agent critiques, ecosystem notes, and architectural suggestions.

Following the principle that *the best critique arrives from an outside vantage point*, any agent, reviewer, or collaborator can file an entry here without waiting for an official planning cycle.

---

## Triage Protocol

Each entry moves through five states:
* `[PROPOSED]`: Unreviewed observation or advice.
* `[TRIAGED]`: Acknowledged by maintainers; feasibility evaluated.
* `[ACCEPTED]`: Scheduled for implementation in an upcoming release tier.
* `[REJECTED]`: Declined with technical rationale preserved in notes.
* `[IMPLEMENTED]`: Shipped and verified with a regression test.

---

## Active Ledger

### KBZ-001: Lance Columnar Vector Container Evaluation for Large Catalogs
* **Date**: 2026-11-01
* **Source**: Architecture Review (Creative DAM Evaluation)
* **Status**: `[ACCEPTED]` (Target: v0.3)
* **Observation**: SQLite BLOB storage with in-memory cosine scans works effortlessly for catalogs up to ~50,000 items (< 15 ms). However, for commercial film asset libraries or 500k-item PhotoCraft RAW archives, an in-memory scan causes memory spikes.
* **Advice**: Integrate the pure-Rust **`lance`** crate as an optional columnar storage partition inside `.dbcraft` bundles. Lance provides zero-copy Apache Arrow buffers and disk-based inverted vector indexing (IVF-PQ) with zero C++ dependencies.

---

### KBZ-002: In-Process Local Embeddings via `fastembed-rs`
* **Date**: 2026-11-01
* **Source**: Agent MCP Optimization Review
* **Status**: `[ACCEPTED]` (Target: v0.2.1)
* **Observation**: Currently, generating query vectors requires callers (or agents) to compute the embedding upstream and pass raw float arrays (`query_vector: [0.1, ...]`).
* **Advice**: Add `fastembed-rs` (backed by `ort` / ONNX Runtime) to `databasecraft-core`. This allows DatabaseCraft to expose a high-level `db_rag_ask { text: "sunset landscape", collection: "assets" }` command that generates 384-dim BGE embeddings locally in ~12 ms with zero Python or cloud API calls.

---

### KBZ-003: Virtualized 100k-Row Grid Performance in `egui`
* **Date**: 2026-11-01
* **Source**: UI/UX Review
* **Status**: `[ACCEPTED]` (Target: v0.3)
* **Observation**: Direct iteration of 10,000+ UI cells causes frame drops in immediate-mode GUIs if not strictly virtualized.
* **Advice**: Implement `egui_extras::TableBuilder` with explicit row-height caching in `databasecraft-app`. Only render the ~40 rows visible in the viewport, maintaining a locked 120 FPS frame rate regardless of whether the table contains 10 rows or 1,000,000 rows.

---

### KBZ-004: Live Document Push over Control Port `14113`
* **Date**: 2026-11-01
* **Source**: Multi-App Workflow Testing
* **Status**: `[ACCEPTED]` (Target: v0.3)
* **Observation**: In Flow 13 (Product Catalog), DesignCraft currently pulls exported JSON files. If a user edits a price in DatabaseCraft while DesignCraft is open, the spread does not refresh automatically.
* **Advice**: Expose a lightweight event bus over port `14113` broadcasting `{"event": "table_mutated", "table": "products"}` so attached apps can trigger live canvas re-renders without polling.

---

### KBZ-005: Porter Stemming & Trigram Support in SQLite FTS5
* **Date**: 2026-11-01
* **Source**: Search Precision Audit
* **Status**: `[IMPLEMENTED]` (v0.2.0-dev)
* **Observation**: Raw keyword matching fails on plurals (e.g. searching "lens" misses "lenses").
* **Advice**: Configure SQLite FTS5 tables with `tokenize = 'porter unicode61 remove_diacritics 2'` to ensure inflectional stem matching across English and European character sets.
