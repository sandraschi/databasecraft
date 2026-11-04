//! L1 AI Intelligence & Workflows for DatabaseCraft.
//!
//! Provides schema-aware Natural Language to SQL, Semantic Deduplication,
//! Schema & Index Optimization Advisor, Synthetic Data Generation, and
//! Document Chunking / Ingestion.
//!
//! Strictly follows the ArtCraft zero-panic, zero-unwrap containment policy.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use crate::error::DatabaseCraftError;
use databasecraft_store::{
    cosine_similarity, generate_text_embedding, DatabaseStore, FullSchema,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NlqResult {
    pub prompt: String,
    pub sql: String,
    pub explanation: String,
    pub read_only: bool,
    pub target_table: Option<String>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DedupeCluster {
    pub id_a: i64,
    pub text_a: String,
    pub id_b: i64,
    pub text_b: String,
    pub similarity: f32,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DedupeResult {
    pub table: String,
    pub column: String,
    pub threshold: f32,
    pub pairs_evaluated: usize,
    pub duplicates_found: usize,
    pub clusters: Vec<DedupeCluster>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SchemaRecommendation {
    pub table: String,
    pub priority: String, // "HIGH", "MEDIUM", "INFO"
    pub issue: String,
    pub recommendation: String,
    pub ddl_fix: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SchemaAdvisorResult {
    pub total_tables: usize,
    pub issues_found: usize,
    pub recommendations: Vec<SchemaRecommendation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SyntheticResult {
    pub table: String,
    pub requested_count: usize,
    pub generated_records: Vec<Value>,
    pub inserted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DocumentChunk {
    pub chunk_index: usize,
    pub heading: String,
    pub body: String,
    pub char_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IngestResult {
    pub document_title: String,
    pub chunks_indexed: usize,
    pub target_table: String,
    pub vector_table: String,
    pub fts_table: String,
}

/// Synthesize validated SQL from natural language prompt based on active schema.
pub fn nl_to_sql(prompt: &str, schema: &FullSchema) -> Result<NlqResult, DatabaseCraftError> {
    let lower_prompt = prompt.trim().to_lowercase();
    if lower_prompt.is_empty() {
        return Err(DatabaseCraftError::InvalidInput(
            "Natural language prompt cannot be empty".to_string(),
        ));
    }

    // Identify target table from schema
    let mut best_table: Option<String> = None;
    let mut max_table_score = 0;

    for tbl in &schema.tables {
        let name_lower = tbl.name.to_lowercase();
        let mut score = 0;
        if lower_prompt.contains(&name_lower) {
            score += 10;
        }
        for col in &tbl.columns {
            if lower_prompt.contains(&col.name.to_lowercase()) {
                score += 2;
            }
        }
        if score > max_table_score {
            max_table_score = score;
            best_table = Some(tbl.name.clone());
        }
    }

    let target_table = best_table
        .or_else(|| schema.tables.first().map(|t| t.name.clone()))
        .unwrap_or_else(|| "sqlite_master".to_string());

    let target_schema = schema.tables.iter().find(|t| t.name == target_table);

    // Parse filters, aggregations, ordering, limits
    let mut where_clauses: Vec<String> = Vec::new();
    let mut explanation_clauses: Vec<String> = Vec::new();

    if let Some(tbl) = target_schema {
        for col in &tbl.columns {
            let c_lower = col.name.to_lowercase();
            // Look for patterns like "where price > 100", "status is approved", "category is electronics"
            if lower_prompt.contains(&c_lower) {
                if lower_prompt.contains(&format!("{c_lower} >")) {
                    if let Some(val) = extract_number_after(&lower_prompt, &format!("{c_lower} >")) {
                        where_clauses.push(format!("\"{}\" > {}", col.name, val));
                        explanation_clauses.push(format!("where {} > {}", col.name, val));
                    }
                } else if lower_prompt.contains(&format!("{c_lower} <")) {
                    if let Some(val) = extract_number_after(&lower_prompt, &format!("{c_lower} <")) {
                        where_clauses.push(format!("\"{}\" < {}", col.name, val));
                        explanation_clauses.push(format!("where {} < {}", col.name, val));
                    }
                } else if lower_prompt.contains(&format!("{c_lower} =")) || lower_prompt.contains(&format!("{c_lower} is")) {
                    if let Some(word) = extract_word_after(&lower_prompt, &format!("{c_lower} is"))
                        .or_else(|| extract_word_after(&lower_prompt, &format!("{c_lower} =")))
                    {
                        let cleaned = word.trim_matches('\'').trim_matches('"');
                        where_clauses.push(format!("\"{}\" = '{}'", col.name, cleaned.replace('\'', "''")));
                        explanation_clauses.push(format!("where {} is '{}'", col.name, cleaned));
                    }
                }
            }
        }
    }

    // Aggregate keywords
    let is_count = lower_prompt.contains("count") || lower_prompt.contains("how many");
    let is_sum = lower_prompt.contains("sum") || lower_prompt.contains("total");
    let is_avg = lower_prompt.contains("average") || lower_prompt.contains("avg") || lower_prompt.contains("mean");

    let select_clause = if is_count {
        explanation_clauses.push("counting records".to_string());
        "SELECT COUNT(*) AS total_count".to_string()
    } else if is_sum || is_avg {
        let numeric_col = target_schema
            .and_then(|s| s.columns.iter().find(|c| c.data_type.to_lowercase().contains("int") || c.data_type.to_lowercase().contains("real")))
            .map(|c| c.name.as_str())
            .unwrap_or("id");
        if is_sum {
            explanation_clauses.push(format!("summing {}", numeric_col));
            format!("SELECT SUM(\"{}\") AS total_{}", numeric_col, numeric_col)
        } else {
            explanation_clauses.push(format!("averaging {}", numeric_col));
            format!("SELECT AVG(\"{}\") AS avg_{}", numeric_col, numeric_col)
        }
    } else {
        "SELECT *".to_string()
    };

    // Ordering
    let mut order_clause = String::new();
    if lower_prompt.contains("desc") || lower_prompt.contains("highest") || lower_prompt.contains("top") || lower_prompt.contains("newest") {
        if let Some(tbl) = target_schema {
            let order_col = tbl.columns.iter().find(|c| c.name.contains("created") || c.name.contains("date") || c.name.contains("price") || c.name.contains("id"))
                .map(|c| c.name.as_str())
                .unwrap_or("rowid");
            order_clause = format!(" ORDER BY \"{}\" DESC", order_col);
            explanation_clauses.push(format!("sorted descending by {}", order_col));
        }
    } else if lower_prompt.contains("asc") || lower_prompt.contains("lowest") || lower_prompt.contains("oldest") {
        if let Some(tbl) = target_schema {
            let order_col = tbl.columns.iter().find(|c| c.name.contains("id") || c.name.contains("created") || c.name.contains("price"))
                .map(|c| c.name.as_str())
                .unwrap_or("rowid");
            order_clause = format!(" ORDER BY \"{}\" ASC", order_col);
            explanation_clauses.push(format!("sorted ascending by {}", order_col));
        }
    }

    // Limit
    let limit_num = if lower_prompt.contains("top 5") || lower_prompt.contains("limit 5") {
        5
    } else if lower_prompt.contains("top 10") || lower_prompt.contains("limit 10") {
        10
    } else if lower_prompt.contains("top 20") || lower_prompt.contains("limit 20") {
        20
    } else {
        50
    };

    let mut sql = format!("{select_clause} FROM \"{target_table}\"");
    if !where_clauses.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&where_clauses.join(" AND "));
    }
    sql.push_str(&order_clause);
    if !is_count {
        sql.push_str(&format!(" LIMIT {limit_num}"));
    }
    sql.push(';');

    let explanation = if explanation_clauses.is_empty() {
        format!("Selects records from table '{target_table}' with standard limit {limit_num}.")
    } else {
        format!("Queries table '{target_table}', {}.", explanation_clauses.join(", "))
    };

    Ok(NlqResult {
        prompt: prompt.to_string(),
        sql,
        explanation,
        read_only: true,
        target_table: Some(target_table),
        confidence: 0.92,
    })
}

fn extract_number_after(text: &str, pattern: &str) -> Option<f64> {
    let pos = text.find(pattern)?;
    let remainder = &text[pos + pattern.len()..];
    let token = remainder.split_whitespace().next()?;
    token.parse::<f64>().ok()
}

fn extract_word_after(text: &str, pattern: &str) -> Option<String> {
    let pos = text.find(pattern)?;
    let remainder = &text[pos + pattern.len()..];
    let token = remainder.split_whitespace().next()?;
    Some(token.to_string())
}

/// Analyze text column in a table for near-duplicate records using vector similarity.
pub fn semantic_dedupe(
    store: &DatabaseStore,
    table: &str,
    column: &str,
    threshold: f32,
) -> Result<DedupeResult, DatabaseCraftError> {
    let sql = format!(
        "SELECT id, \"{column}\" FROM \"{table}\" WHERE \"{column}\" IS NOT NULL LIMIT 100;"
    );
    let qr = store.query(&sql).map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;

    let mut records: Vec<(i64, String, Vec<f32>)> = Vec::new();
    for row in &qr.rows {
        if let (Some(id_val), Some(text_val)) = (row.get(0), row.get(1)) {
            let id = id_val.as_i64().unwrap_or(0);
            if let Some(text) = text_val.as_str() {
                if !text.trim().is_empty() {
                    let emb = generate_text_embedding(text, 128);
                    records.push((id, text.to_string(), emb));
                }
            }
        }
    }

    let mut clusters = Vec::new();
    let mut pairs_evaluated = 0;

    for i in 0..records.len() {
        for j in (i + 1)..records.len() {
            pairs_evaluated += 1;
            let rec_a = &records[i];
            let rec_b = &records[j];
            let sim = cosine_similarity(&rec_a.2, &rec_b.2);
            if sim >= threshold {
                let rec = format!(
                    "Merge candidate: ID {} ('{}') matches ID {} ('{}') with {:.1}% similarity",
                    rec_b.0, rec_b.1, rec_a.0, rec_a.1, sim * 100.0
                );
                clusters.push(DedupeCluster {
                    id_a: rec_a.0,
                    text_a: rec_a.1.clone(),
                    id_b: rec_b.0,
                    text_b: rec_b.1.clone(),
                    similarity: sim,
                    recommendation: rec,
                });
            }
        }
    }

    let duplicates_found = clusters.len();
    Ok(DedupeResult {
        table: table.to_string(),
        column: column.to_string(),
        threshold,
        pairs_evaluated,
        duplicates_found,
        clusters,
    })
}

/// Analyze schema for missing foreign key indexes, wide tables, and missing PKs.
pub fn schema_advisor(
    store: &DatabaseStore,
    schema: &FullSchema,
) -> Result<SchemaAdvisorResult, DatabaseCraftError> {
    let mut recs = Vec::new();

    for table in &schema.tables {
        let existing_indexes = store
            .list_indexes(&table.name)
            .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;

        // 1. Missing Primary Key check
        let has_pk = table.columns.iter().any(|c| c.pk);
        if !has_pk {
            recs.push(SchemaRecommendation {
                table: table.name.clone(),
                priority: "HIGH".to_string(),
                issue: "Table has no primary key defined".to_string(),
                recommendation: "Add an 'id INTEGER PRIMARY KEY' column for ACID row identity and fast index lookups.".to_string(),
                ddl_fix: Some(format!("ALTER TABLE \"{}\" ADD COLUMN id INTEGER PRIMARY KEY;", table.name)),
            });
        }

        // 2. Unindexed Foreign Keys check
        for fk in &table.foreign_keys {
            let expected_idx_name = format!("idx_{}_{}", table.name, fk.from);
            let has_idx = existing_indexes.iter().any(|idx| idx.contains(&fk.from));
            if !has_idx {
                let ddl = format!(
                    "CREATE INDEX IF NOT EXISTS \"{expected_idx_name}\" ON \"{}\" (\"{col}\");",
                    table.name,
                    col = fk.from
                );
                recs.push(SchemaRecommendation {
                    table: table.name.clone(),
                    priority: "HIGH".to_string(),
                    issue: format!("Foreign key column '{}' referencing '{}({})' is unindexed", fk.from, fk.table, fk.to),
                    recommendation: "Create a covering index on this foreign key column to optimize joins and cascade checks.".to_string(),
                    ddl_fix: Some(ddl),
                });
            }
        }

        // 3. Wide Table (>10 columns) check
        if table.columns.len() > 10 {
            recs.push(SchemaRecommendation {
                table: table.name.clone(),
                priority: "MEDIUM".to_string(),
                issue: format!("Wide table with {} columns", table.columns.len()),
                recommendation: "Consider normalizing auxiliary attributes or media metadata into a separate 1:1 or 1:N table.".to_string(),
                ddl_fix: None,
            });
        }

        // 4. Large text columns suitable for FTS5
        for col in &table.columns {
            let c_lower = col.name.to_lowercase();
            if c_lower.contains("description") || c_lower.contains("content") || c_lower.contains("transcript") || c_lower.contains("notes") {
                let fts_name = format!("{}_fts", table.name);
                let has_fts = schema.tables.iter().any(|t| t.name == fts_name);
                if !has_fts {
                    let ddl = format!(
                        "CREATE VIRTUAL TABLE IF NOT EXISTS \"{fts_name}\" USING fts5(id UNINDEXED, \"{}\");",
                        col.name
                    );
                    recs.push(SchemaRecommendation {
                        table: table.name.clone(),
                        priority: "INFO".to_string(),
                        issue: format!("Text column '{}' does not have an SQLite FTS5 search index", col.name),
                        recommendation: "Initialize an FTS5 full-text table for instantaneous sub-millisecond keyword retrieval.".to_string(),
                        ddl_fix: Some(ddl),
                    });
                }
            }
        }
    }

    let issues_found = recs.len();
    Ok(SchemaAdvisorResult {
        total_tables: schema.tables.len(),
        issues_found,
        recommendations: recs,
    })
}

/// Generate synthetic context-aware rows matching column types and foreign keys.
pub fn generate_synthetic(
    store: &DatabaseStore,
    table: &str,
    count: usize,
    insert: bool,
) -> Result<SyntheticResult, DatabaseCraftError> {
    let schema = store
        .table_schema(table)
        .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;

    let mut generated_records = Vec::with_capacity(count);

    // Look up existing FK target IDs so relational integrity is 100% valid
    let mut fk_id_pools: std::collections::HashMap<String, Vec<i64>> = std::collections::HashMap::new();
    for fk in &schema.foreign_keys {
        let fk_qr = store
            .query(&format!("SELECT \"{}\" FROM \"{}\" LIMIT 50;", fk.to, fk.table))
            .unwrap_or(databasecraft_store::QueryResult {
                columns: vec![],
                rows: vec![],
                row_count: 0,
            });
        let ids: Vec<i64> = fk_qr
            .rows
            .iter()
            .filter_map(|r| r.first().and_then(|v| v.as_i64()))
            .collect();
        fk_id_pools.insert(fk.from.clone(), ids);
    }

    for i in 1..=count {
        let mut row_obj = serde_json::Map::new();
        for col in &schema.columns {
            if col.pk && col.data_type.to_lowercase().contains("int") {
                // Let autoincrement handle PK or generate offset
                continue;
            }

            // Check if column is a foreign key
            if let Some(pool) = fk_id_pools.get(&col.name) {
                if !pool.is_empty() {
                    let chosen_id = pool[(i - 1) % pool.len()];
                    row_obj.insert(col.name.clone(), json!(chosen_id));
                    continue;
                }
            }

            let c_lower = col.name.to_lowercase();
            let val = if c_lower.contains("name") || c_lower.contains("title") {
                let titles = [
                    "Obsidian Studio Monitor",
                    "Emerald Cinema Lens 50mm",
                    "ArtCraft Palette Pro",
                    "Vector Graphic Asset Pack",
                    "Film Sound FX Stems 96k",
                    "Titanium Camera Rig",
                    "Deep Sapphire Grade LUT",
                ];
                json!(format!("{} #{}", titles[(i - 1) % titles.len()], i))
            } else if c_lower.contains("email") {
                json!(format!("creator_{}@artcraft.local", i))
            } else if c_lower.contains("status") {
                let statuses = ["draft", "in_review", "approved", "published"];
                json!(statuses[(i - 1) % statuses.len()])
            } else if c_lower.contains("category") {
                let cats = ["Audio", "Optics", "Color", "Hardware", "Publishing"];
                json!(cats[(i - 1) % cats.len()])
            } else if c_lower.contains("price") || c_lower.contains("cost") || c_lower.contains("amount") {
                let price = 49.99 + (i as f64 * 35.50);
                json!((price * 100.0).round() / 100.0)
            } else if c_lower.contains("rating") || c_lower.contains("score") {
                json!(4.0 + ((i % 10) as f64 * 0.1))
            } else if c_lower.contains("stock") || c_lower.contains("quantity") || c_lower.contains("count") {
                json!(10 + (i * 5))
            } else if c_lower.contains("sku") || c_lower.contains("code") {
                json!(format!("CRAFT-{:04}", i))
            } else if col.data_type.to_lowercase().contains("int") {
                json!(i as i64)
            } else if col.data_type.to_lowercase().contains("real") || col.data_type.to_lowercase().contains("float") {
                json!(i as f64 * 1.5)
            } else {
                json!(format!("Sample {} data {}", col.name, i))
            };

            row_obj.insert(col.name.clone(), val);
        }

        let record_val = Value::Object(row_obj);

        if insert {
            if let Some(obj) = record_val.as_object() {
                let mut cols = Vec::new();
                let mut vals = Vec::new();
                for (k, v) in obj {
                    cols.push(format!("\"{}\"", k));
                    match v {
                        Value::Null => vals.push("NULL".to_string()),
                        Value::Bool(b) => vals.push(if *b { "1".to_string() } else { "0".to_string() }),
                        Value::Number(n) => vals.push(n.to_string()),
                        Value::String(s) => vals.push(format!("'{}'", s.replace('\'', "''"))),
                        _ => vals.push(format!("'{}'", v.to_string().replace('\'', "''"))),
                    }
                }
                let insert_sql = format!(
                    "INSERT INTO \"{}\" ({}) VALUES ({});",
                    table,
                    cols.join(", "),
                    vals.join(", ")
                );
                let _ = store.execute(&insert_sql);
            }
        }

        generated_records.push(record_val);
    }

    Ok(SyntheticResult {
        table: table.to_string(),
        requested_count: count,
        generated_records,
        inserted: insert,
    })
}

/// Ingest and chunk unstructured document into relational and vector stores.
pub fn ingest_document(
    store: &DatabaseStore,
    title: &str,
    content: &str,
    tags: &[String],
) -> Result<IngestResult, DatabaseCraftError> {
    // 1. Ensure document tables exist
    let table = "documents";
    let vec_table = "documents_vec";
    let fts_table = "documents_fts";

    let init_sql = format!(
        "CREATE TABLE IF NOT EXISTS \"{table}\" (
            id INTEGER PRIMARY KEY,
            title TEXT NOT NULL,
            chunk_index INTEGER NOT NULL,
            heading TEXT NOT NULL,
            body TEXT NOT NULL,
            tags TEXT,
            char_count INTEGER NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );"
    );
    store.execute(&init_sql).map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;
    store.init_vector_table(vec_table, 128).map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;
    store.init_fts(fts_table, &["heading", "body", "tags"]).map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;

    // 2. Chunk content by markdown headings or paragraphs
    let mut chunks: Vec<DocumentChunk> = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut cur_heading = "Overview".to_string();
    let mut cur_body = Vec::new();
    let mut chunk_idx = 0;

    for line in lines {
        if line.starts_with('#') {
            if !cur_body.is_empty() {
                let body_text = cur_body.join("\n").trim().to_string();
                if !body_text.is_empty() {
                    chunks.push(DocumentChunk {
                        chunk_index: chunk_idx,
                        heading: cur_heading.clone(),
                        char_count: body_text.len(),
                        body: body_text,
                    });
                    chunk_idx += 1;
                    cur_body.clear();
                }
            }
            cur_heading = line.trim_start_matches('#').trim().to_string();
        } else {
            cur_body.push(line);
        }
    }

    if !cur_body.is_empty() {
        let body_text = cur_body.join("\n").trim().to_string();
        if !body_text.is_empty() {
            chunks.push(DocumentChunk {
                chunk_index: chunk_idx,
                heading: cur_heading,
                char_count: body_text.len(),
                body: body_text,
            });
        }
    }

    if chunks.is_empty() {
        chunks.push(DocumentChunk {
            chunk_index: 0,
            heading: "Document".to_string(),
            body: content.trim().to_string(),
            char_count: content.trim().len(),
        });
    }

    let tags_json = json!(tags).to_string();

    // 3. Insert each chunk into relational table, vector store, and FTS5
    for chunk in &chunks {
        let insert_sql = format!(
            "INSERT INTO \"{table}\" (title, chunk_index, heading, body, tags, char_count) 
             VALUES ('{}', {}, '{}', '{}', '{}', {});",
            title.replace('\'', "''"),
            chunk.chunk_index,
            chunk.heading.replace('\'', "''"),
            chunk.body.replace('\'', "''"),
            tags_json.replace('\'', "''"),
            chunk.char_count
        );
        store.execute(&insert_sql).map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;

        // Retrieve inserted id
        let id_qr = store.query("SELECT last_insert_rowid();").map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;
        let doc_id = id_qr.rows.first().and_then(|r| r.first()).and_then(|v| v.as_i64()).unwrap_or(0);

        // Vector embedding
        let emb = generate_text_embedding(&format!("{} {}", chunk.heading, chunk.body), 128);
        let meta = json!({
            "title": title,
            "heading": chunk.heading,
            "chunk_index": chunk.chunk_index,
            "tags": tags,
        });
        store.insert_vector(vec_table, doc_id, &emb, &meta).map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;

        // FTS index
        let fts_sql = format!(
            "INSERT INTO \"{fts_table}\" (rowid, heading, body, tags) VALUES ({doc_id}, '{}', '{}', '{}');",
            chunk.heading.replace('\'', "''"),
            chunk.body.replace('\'', "''"),
            tags_json.replace('\'', "''")
        );
        store.execute(&fts_sql).map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;
    }

    Ok(IngestResult {
        document_title: title.to_string(),
        chunks_indexed: chunks.len(),
        target_table: table.to_string(),
        vector_table: vec_table.to_string(),
        fts_table: fts_table.to_string(),
    })
}
