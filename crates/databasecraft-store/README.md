# `databasecraft-store` (L1 SQLite & Dense Vector Store)

Local-first `.dbcraft` persistence layer built on `rusqlite`:

- **Relational Storage (`lib.rs`)**: SQLite WAL mode, foreign-key enforcement, schema introspection (`schema-relations`), primary-key write-back, and quiescent mutation auditing.
- **Hybrid RAG & Vector Search (`vector.rs`)**: In-process dense vector embeddings (`Cosine`, `DotProduct`, `L2`) paired with SQLite FTS5 Reciprocal Rank Fusion (RRF).
