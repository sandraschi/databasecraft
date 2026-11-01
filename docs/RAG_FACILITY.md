# DatabaseCraft — Modern RAG & Hybrid Search Facility

**Specification Version**: 0.2.0-dev  
**Module**: [`crates/databasecraft-store/src/vector.rs`](../crates/databasecraft-store/src/vector.rs) & [`crates/databasecraft-core`](../crates/databasecraft-core)

---

## 1. Overview & Rationale

In professional creative pipelines, content discovery is inherently multi-modal and ambiguous:
* An editor in **FilmCraft** remembers: *"that quiet dawn dialogue take on the roof"* rather than `"SC-01-Take-2"`.
* A layout designer in **DesignCraft** looks for: *"high-end acoustic hardware with dark brushed textures"*.
* An art director in **PhotoCraft** searches: *"warm golden-hour landscape with low saturation"*.

Traditional SQL keyword filters fail because they demand exact terminology. Pure vector search fails because it cannot guarantee strict boolean constraints (e.g., `in_stock = 1`, `lens = '35mm'`).

**DatabaseCraft solves this with a Hybrid RAG Architecture**: combining **dense vector embeddings** (semantic understanding) with **SQLite FTS5 BM25** (exact term precision) and **relational SQL filters** (business logic).

---

## 2. Mathematical & Algorithmic Foundations

### 2.1 Vector Representation & Binary Packing
Vectors are stored as packed little-endian binary blobs (`Vec<u8>`), minimizing serialization overhead and database size:
$$\text{bytes} = \text{dimension} \times 4$$
For example, a 384-dimensional embedding (e.g. `bge-small-en-v1.5` or `all-MiniLM-L6-v2`) packs into exactly **1,536 bytes**.

```rust
pub fn pack_vector(v: &[f32]) -> Vec<u8>
pub fn unpack_vector(bytes: &[u8]) -> Vec<f32>
```
Unpacking uses `as_chunks::<4>()` for zero-allocation SIMD translation back to `&[f32]`.

### 2.2 Similarity Metrics
DatabaseCraft provides three distance metrics:

1. **Cosine Similarity**:
   $$\text{cosine}(\mathbf{u}, \mathbf{v}) = \frac{\mathbf{u} \cdot \mathbf{v}}{\|\mathbf{u}\|_2 \|\mathbf{v}\|_2}$$
   Normalized to the interval $[-1.0, 1.0]$. Ideal for normalized text and image embeddings.

2. **Dot Product (Inner Product)**:
   $$\text{dot}(\mathbf{u}, \mathbf{v}) = \sum_{i=1}^n u_i v_i$$
   Optimal when vectors are pre-normalized to unit length ($\|\mathbf{u}\|_2 = 1$).

3. **Euclidean (L2) Distance (Normalized Similarity)**:
   $$\text{dist}_{L2}(\mathbf{u}, \mathbf{v}) = \sqrt{\sum_{i=1}^n (u_i - v_i)^2}$$
   Converted to a bounded similarity score in $[0.0, 1.0]$ via:
   $$\text{score}_{L2} = \frac{1}{1 + \text{dist}_{L2}}$$

### 2.3 Reciprocal Rank Fusion (RRF)
To merge vector search candidates with sparse FTS5 keyword hits without score scale distortion, DatabaseCraft applies **Reciprocal Rank Fusion**:

$$RRF(d) = \sum_{m \in M} \frac{1}{k + r_m(d)}$$

Where:
* $M$ is the set of retrieval models (Dense Vector + Sparse FTS5).
* $r_m(d)$ is the 1-based rank of document $d$ within model $m$.
* $k$ is the smoothing constant (default: **`60.0`**, proven across TREC benchmarks to prevent top-ranked outliers from dominating).

---

## 3. Schema & Table Layout

### 3.1 Vector Table Layout
```sql
CREATE TABLE IF NOT EXISTS "asset_embeddings" (
    id INTEGER PRIMARY KEY,
    vector BLOB NOT NULL,
    dimensions INTEGER NOT NULL,
    metadata TEXT  -- Structured JSON containing title, app, tags, etc.
);
```

### 3.2 FTS5 Full-Text Search Table
```sql
CREATE VIRTUAL TABLE IF NOT EXISTS "asset_fts" USING fts5(
    title,
    notes,
    tokenize = 'porter unicode61 remove_diacritics 2'
);
```

---

## 4. Universal Command Bus API

### 4.1 Initialize Vector Table
```json
{
  "command": "VectorInit",
  "params": {
    "table": "assets_vec",
    "dimensions": 384
  }
}
```

### 4.2 Insert Vector
```json
{
  "command": "VectorInsert",
  "params": {
    "table": "assets_vec",
    "id": 101,
    "vector": [0.042, -0.12, 0.88, ...],
    "metadata": {
      "app": "photocraft",
      "sku": "AUD-900",
      "title": "Obsidian Field Monitor Headphones"
    }
  }
}
```

### 4.3 Hybrid Search
```json
{
  "command": "HybridSearch",
  "params": {
    "vector_table": "assets_vec",
    "fts_table": "assets_fts",
    "query_text": "Obsidian Headphones",
    "query_vector": [0.045, -0.11, 0.85, ...],
    "top_k": 5
  }
}
```

#### Response Format:
```json
{
  "success": true,
  "count": 1,
  "hits": [
    {
      "id": 101,
      "rrf_score": 0.03278688524590164,
      "vector_score": 0.9942,
      "fts_rank": -3.84,
      "metadata": {
        "app": "photocraft",
        "sku": "AUD-900",
        "title": "Obsidian Field Monitor Headphones"
      }
    }
  ]
}
```

---

## 5. Model Context Protocol (MCP) Tools

When running `databasecraft-cli mcp`, agents have access to:

1. `db_vector_search`: Direct dense vector similarity lookups (`table`, `vector`, `metric`, `top_k`).
2. `db_hybrid_search`: Unified dense + sparse search with RRF (`vector_table`, `fts_table`, `query_text`, `query_vector`, `top_k`).
3. `db_fts_search`: Direct BM25 keyword matching (`table`, `query`, `top_k`).
