//! L1 DatabaseCraft Core: Schema engines, cross-craft exports, and Universal Command Bus.
//!
//! Follows the ArtCraft containment harness:
//! - Zero panics, zero unwraps (`clippy::deny(...)`)
//! - Strict Command::dispatch bus
//! - Complete separation from UI (egui) and CLI (clap) runtimes
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

pub mod error;
pub mod export;
pub mod models;
pub mod seed;

use databasecraft_store::DatabaseStore;
use error::{validate_identifier, DatabaseCraftError};
use models::{ExportTarget, TableDef};
use seed::{seed_demo_dataset, DemoKind};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Standard registered control port for DatabaseCraft in the ArtCraft family.
pub const CONTROL_PORT: u16 = 14113;
pub const APP_NAME: &str = "databasecraft";
pub const APP_VERSION: &str = "0.5.0";

/// Unique string IDs exposed over MCP `tools/list` and the CLI.
pub const CMD_STATUS: &str = "status";
pub const CMD_SCHEMA_INSPECT: &str = "db.schema_inspect";
pub const CMD_SCHEMA_CREATE: &str = "db.schema_create";
pub const CMD_QUERY: &str = "db.query";
pub const CMD_RECORD_INSERT: &str = "db.record_insert";
pub const CMD_RECORD_UPDATE: &str = "db.record_update";
pub const CMD_RECORD_DELETE: &str = "db.record_delete";
pub const CMD_EXPORT_MERGE: &str = "db.export_merge";
pub const CMD_SEED_DEMO: &str = "db.seed_demo";
pub const CMD_EXECUTE: &str = "db.execute";
pub const CMD_VECTOR_INIT: &str = "rag.vector_init";
pub const CMD_VECTOR_INSERT: &str = "rag.vector_insert";
pub const CMD_VECTOR_SEARCH: &str = "rag.vector_search";
pub const CMD_FTS_INIT: &str = "rag.fts_init";
pub const CMD_FTS_SEARCH: &str = "rag.fts_search";
pub const CMD_HYBRID_SEARCH: &str = "rag.hybrid_search";
pub const CMD_RAG_EMBED: &str = "rag.embed_text";
pub const CMD_RAG_ASK: &str = "rag.semantic_search";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "command", content = "params")]
pub enum Command {
    Status,
    SchemaInspect {
        table: Option<String>,
    },
    SchemaCreate {
        table_def: TableDef,
    },
    Query {
        sql: String,
    },
    RecordInsert {
        table: String,
        record: Value,
    },
    RecordUpdate {
        table: String,
        id: i64,
        changes: Value,
    },
    RecordDelete {
        table: String,
        id: i64,
    },
    ExportMerge {
        query: String,
        target: ExportTarget,
        name: Option<String>,
    },
    SeedDemo {
        name: String,
    },
    Execute {
        sql: String,
    },
    VectorInit {
        table: String,
        dimensions: usize,
    },
    VectorInsert {
        table: String,
        id: i64,
        vector: Vec<f32>,
        metadata: Value,
    },
    VectorSearch {
        table: String,
        vector: Vec<f32>,
        metric: Option<databasecraft_store::VectorMetric>,
        top_k: Option<usize>,
    },
    FtsInit {
        table: String,
        columns: Vec<String>,
    },
    FtsSearch {
        table: String,
        query: String,
        top_k: Option<usize>,
    },
    HybridSearch {
        vector_table: String,
        fts_table: String,
        query_text: String,
        query_vector: Vec<f32>,
        top_k: Option<usize>,
    },
    RagEmbed {
        text: String,
        dimensions: Option<usize>,
    },
    RagAsk {
        vector_table: String,
        fts_table: Option<String>,
        query: String,
        dimensions: Option<usize>,
        top_k: Option<usize>,
    },
}

/// The Universal Command Bus: dispatches any Command variant into the database store.
/// Never panics: always returns a structured JSON `Value` (with an `"error"` field on failure).
pub async fn dispatch(cmd: Command, store: &DatabaseStore) -> Value {
    match execute_dispatch(cmd, store) {
        Ok(val) => val,
        Err(err) => json!({
            "success": false,
            "error": err.to_string(),
        }),
    }
}

fn execute_dispatch(cmd: Command, store: &DatabaseStore) -> Result<Value, DatabaseCraftError> {
    match cmd {
        Command::Status => {
            let tables = store.list_tables()?;
            Ok(json!({
                "app": APP_NAME,
                "version": APP_VERSION,
                "control_port": CONTROL_PORT,
                "status": "ready",
                "tables_count": tables.len(),
                "tables": tables,
            }))
        }
        Command::SchemaInspect { table } => {
            if let Some(t) = table {
                let schema = store.table_schema(&t)?;
                Ok(json!({
                    "success": true,
                    "table": schema,
                }))
            } else {
                let full = store.inspect_all()?;
                Ok(json!({
                    "success": true,
                    "schema": full,
                }))
            }
        }
        Command::SchemaCreate { table_def } => {
            validate_identifier(&table_def.name)?;
            for col in &table_def.columns {
                validate_identifier(&col.name)?;
            }
            let sql = table_def.to_create_sql();
            store.execute(&sql)?;
            Ok(json!({
                "success": true,
                "table": table_def.name,
                "created_sql": sql,
            }))
        }
        Command::Query { sql } => {
            let qr = store.query(&sql)?;
            Ok(json!({
                "success": true,
                "columns": qr.columns,
                "rows": qr.rows,
                "row_count": qr.row_count,
            }))
        }
        Command::RecordInsert { table, record } => {
            validate_identifier(&table)?;
            let obj = record.as_object().ok_or_else(|| {
                DatabaseCraftError::InvalidInput("record must be a JSON object".to_string())
            })?;

            let mut col_names = Vec::new();
            let mut val_placeholders = Vec::new();
            for (k, v) in obj {
                validate_identifier(k)?;
                col_names.push(format!("\"{}\"", k));
                match v {
                    Value::Null => val_placeholders.push("NULL".to_string()),
                    Value::Bool(b) => {
                        val_placeholders.push(if *b { "1".to_string() } else { "0".to_string() })
                    }
                    Value::Number(n) => val_placeholders.push(n.to_string()),
                    Value::String(s) => {
                        val_placeholders.push(format!("'{}'", s.replace('\'', "''")))
                    }
                    _ => val_placeholders.push(format!("'{}'", v.to_string().replace('\'', "''"))),
                }
            }

            let sql = format!(
                "INSERT INTO \"{}\" ({}) VALUES ({});",
                table,
                col_names.join(", "),
                val_placeholders.join(", ")
            );
            store.execute(&sql)?;

            // Retrieve last insert id
            let id_qr = store.query("SELECT last_insert_rowid() AS id;")?;
            let inserted_id = id_qr
                .rows
                .first()
                .and_then(|r| r.first())
                .cloned()
                .unwrap_or(Value::Null);

            Ok(json!({
                "success": true,
                "table": table,
                "inserted_id": inserted_id,
            }))
        }
        Command::RecordUpdate { table, id, changes } => {
            validate_identifier(&table)?;
            let obj = changes.as_object().ok_or_else(|| {
                DatabaseCraftError::InvalidInput("changes must be a JSON object".to_string())
            })?;

            let mut set_clauses = Vec::new();
            for (k, v) in obj {
                validate_identifier(k)?;
                let formatted_val = match v {
                    Value::Null => "NULL".to_string(),
                    Value::Bool(b) => {
                        if *b {
                            "1".to_string()
                        } else {
                            "0".to_string()
                        }
                    }
                    Value::Number(n) => n.to_string(),
                    Value::String(s) => format!("'{}'", s.replace('\'', "''")),
                    _ => format!("'{}'", v.to_string().replace('\'', "''")),
                };
                set_clauses.push(format!("\"{}\" = {}", k, formatted_val));
            }

            let sql = format!(
                "UPDATE \"{}\" SET {} WHERE id = {};",
                table,
                set_clauses.join(", "),
                id
            );
            let affected = store.execute(&sql)?;

            Ok(json!({
                "success": true,
                "table": table,
                "id": id,
                "rows_affected": affected,
            }))
        }
        Command::RecordDelete { table, id } => {
            validate_identifier(&table)?;
            let sql = format!("DELETE FROM \"{}\" WHERE id = {};", table, id);
            let affected = store.execute(&sql)?;
            Ok(json!({
                "success": true,
                "table": table,
                "id": id,
                "rows_affected": affected,
            }))
        }
        Command::ExportMerge {
            query,
            target,
            name,
        } => {
            let qr = store.query(&query)?;
            let dataset_name = name.unwrap_or_else(|| "export_data".to_string());
            let exported_value = export::format_for_craft(&qr, target, &dataset_name)?;
            Ok(json!({
                "success": true,
                "target": target,
                "export": exported_value,
            }))
        }
        Command::SeedDemo { name } => {
            let kind = DemoKind::parse(&name)?;
            seed_demo_dataset(store, kind)?;
            let tables = store.list_tables()?;
            Ok(json!({
                "success": true,
                "seeded": name,
                "tables": tables,
            }))
        }
        Command::Execute { sql } => {
            let affected = store.execute(&sql)?;
            Ok(json!({
                "success": true,
                "rows_affected": affected,
            }))
        }
        Command::VectorInit { table, dimensions } => {
            validate_identifier(&table)?;
            store.init_vector_table(&table, dimensions)?;
            Ok(json!({
                "success": true,
                "table": table,
                "dimensions": dimensions,
            }))
        }
        Command::VectorInsert {
            table,
            id,
            vector,
            metadata,
        } => {
            validate_identifier(&table)?;
            store.insert_vector(&table, id, &vector, &metadata)?;
            Ok(json!({
                "success": true,
                "table": table,
                "id": id,
                "dimensions": vector.len(),
            }))
        }
        Command::VectorSearch {
            table,
            vector,
            metric,
            top_k,
        } => {
            validate_identifier(&table)?;
            let chosen_metric = metric.unwrap_or(databasecraft_store::VectorMetric::Cosine);
            let k = top_k.unwrap_or(10);
            let hits = store.search_vector(&table, &vector, chosen_metric, k)?;
            Ok(json!({
                "success": true,
                "table": table,
                "count": hits.len(),
                "hits": hits,
            }))
        }
        Command::FtsInit { table, columns } => {
            validate_identifier(&table)?;
            for col in &columns {
                validate_identifier(col)?;
            }
            let col_refs = columns.iter().map(|s| s.as_str()).collect::<Vec<_>>();
            store.init_fts(&table, &col_refs)?;
            Ok(json!({
                "success": true,
                "fts_table": table,
                "columns": columns,
            }))
        }
        Command::FtsSearch {
            table,
            query,
            top_k,
        } => {
            validate_identifier(&table)?;
            let k = top_k.unwrap_or(10);
            let hits = store.search_fts(&table, &query, k)?;
            Ok(json!({
                "success": true,
                "table": table,
                "count": hits.len(),
                "hits": hits,
            }))
        }
        Command::HybridSearch {
            vector_table,
            fts_table,
            query_text,
            query_vector,
            top_k,
        } => {
            validate_identifier(&vector_table)?;
            validate_identifier(&fts_table)?;
            let k = top_k.unwrap_or(10);
            let hits = store.hybrid_search(
                &vector_table,
                &fts_table,
                &query_text,
                &query_vector,
                k,
                60.0,
            )?;
            Ok(json!({
                "success": true,
                "count": hits.len(),
                "hits": hits,
            }))
        }
        Command::RagEmbed { text, dimensions } => {
            let dim = dimensions.unwrap_or(128);
            let vec = databasecraft_store::generate_text_embedding(&text, dim);
            Ok(json!({
                "success": true,
                "text": text,
                "dimensions": dim,
                "vector": vec,
            }))
        }
        Command::RagAsk {
            vector_table,
            fts_table,
            query,
            dimensions,
            top_k,
        } => {
            validate_identifier(&vector_table)?;
            let dim = dimensions.unwrap_or(128);
            let query_vec = databasecraft_store::generate_text_embedding(&query, dim);
            let k = top_k.unwrap_or(10);

            if let Some(fts) = fts_table {
                validate_identifier(&fts)?;
                let hits = store.hybrid_search(&vector_table, &fts, &query, &query_vec, k, 60.0)?;
                Ok(json!({
                    "success": true,
                    "mode": "hybrid",
                    "query": query,
                    "count": hits.len(),
                    "hits": hits,
                }))
            } else {
                let hits = store.search_vector(
                    &vector_table,
                    &query_vec,
                    databasecraft_store::VectorMetric::Cosine,
                    k,
                )?;
                Ok(json!({
                    "success": true,
                    "mode": "vector",
                    "query": query,
                    "count": hits.len(),
                    "hits": hits,
                }))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommandDescriptor {
    pub id: String,
    pub label: String,
    pub description: String,
    pub params: Value,
}

/// The machine-readable Craft Command Registry.
/// Provides introspection for `<craft>-cli commands --json`.
pub fn command_registry() -> Vec<CommandDescriptor> {
    vec![
        CommandDescriptor {
            id: CMD_STATUS.to_string(),
            label: "App Status".to_string(),
            description: "Show DatabaseCraft status, tables count, and registered control port 14113".to_string(),
            params: json!({}),
        },
        CommandDescriptor {
            id: CMD_QUERY.to_string(),
            label: "Execute SQL".to_string(),
            description: "Run SQL query against SQLite/OLTP and return structured rows and columns".to_string(),
            params: json!({ "sql": "string" }),
        },
        CommandDescriptor {
            id: CMD_SCHEMA_INSPECT.to_string(),
            label: "Inspect Schema".to_string(),
            description: "Inspect table columns, data types, primary keys, and foreign keys".to_string(),
            params: json!({ "table": "string (optional)" }),
        },
        CommandDescriptor {
            id: CMD_SCHEMA_CREATE.to_string(),
            label: "Create Table".to_string(),
            description: "Create new relational table with typed columns and constraints".to_string(),
            params: json!({ "table_def": "object" }),
        },
        CommandDescriptor {
            id: CMD_RECORD_INSERT.to_string(),
            label: "Insert Record".to_string(),
            description: "Insert JSON record object into specified table".to_string(),
            params: json!({ "table": "string", "record": "object" }),
        },
        CommandDescriptor {
            id: CMD_RECORD_UPDATE.to_string(),
            label: "Update Record".to_string(),
            description: "Update columns for record matching ID in specified table".to_string(),
            params: json!({ "table": "string", "id": "number", "changes": "object" }),
        },
        CommandDescriptor {
            id: CMD_RECORD_DELETE.to_string(),
            label: "Delete Record".to_string(),
            description: "Delete record matching ID from specified table".to_string(),
            params: json!({ "table": "string", "id": "number" }),
        },
        CommandDescriptor {
            id: CMD_EXPORT_MERGE.to_string(),
            label: "Export Cross-Craft Merge".to_string(),
            description: "Format SQL query rows for DesignCraft, DeckCraft, WordCraft, GridCraft, or PdfCraft".to_string(),
            params: json!({ "query": "string", "target": "string (designcraft|deckcraft|wordcraft|gridcraft|pdfcraft)", "name": "string (optional)" }),
        },
        CommandDescriptor {
            id: CMD_SEED_DEMO.to_string(),
            label: "Seed Deterministic Demo".to_string(),
            description: "Populate deterministic dataset: ecommerce, film, or kpi".to_string(),
            params: json!({ "name": "string (ecommerce|film|kpi)" }),
        },
        CommandDescriptor {
            id: CMD_VECTOR_INIT.to_string(),
            label: "Initialize Vector Table".to_string(),
            description: "Create vector table for dense embeddings with specified dimension".to_string(),
            params: json!({ "table": "string", "dimensions": "number" }),
        },
        CommandDescriptor {
            id: CMD_VECTOR_INSERT.to_string(),
            label: "Insert Vector Embedding".to_string(),
            description: "Store dense float vector with ID and associated JSON metadata".to_string(),
            params: json!({ "table": "string", "id": "number", "vector": "array of numbers", "metadata": "object" }),
        },
        CommandDescriptor {
            id: CMD_VECTOR_SEARCH.to_string(),
            label: "Vector Similarity Search".to_string(),
            description: "Dense vector nearest neighbor search using Cosine, DotProduct, or L2 metric".to_string(),
            params: json!({ "table": "string", "vector": "array of numbers", "metric": "string (cosine|dot_product|l2)", "top_k": "number" }),
        },
        CommandDescriptor {
            id: CMD_FTS_INIT.to_string(),
            label: "Initialize FTS5 Full-Text Table".to_string(),
            description: "Create SQLite FTS5 virtual table for sparse keyword indexing".to_string(),
            params: json!({ "table": "string", "columns": "array of strings" }),
        },
        CommandDescriptor {
            id: CMD_FTS_SEARCH.to_string(),
            label: "FTS5 BM25 Search".to_string(),
            description: "Execute sparse BM25 text query returning ranked matching rows".to_string(),
            params: json!({ "table": "string", "query": "string", "top_k": "number" }),
        },
        CommandDescriptor {
            id: CMD_HYBRID_SEARCH.to_string(),
            label: "Hybrid RAG Search".to_string(),
            description: "Execute dense vector + sparse FTS5 search with Reciprocal Rank Fusion".to_string(),
            params: json!({ "vector_table": "string", "fts_table": "string", "query_text": "string", "query_vector": "array of numbers", "top_k": "number" }),
        },
        CommandDescriptor {
            id: CMD_RAG_EMBED.to_string(),
            label: "Generate Text Embedding".to_string(),
            description: "Deterministic in-process subword semantic vector projection for raw text".to_string(),
            params: json!({ "text": "string", "dimensions": "number (optional, default 128)" }),
        },
        CommandDescriptor {
            id: CMD_RAG_ASK.to_string(),
            label: "Semantic Text Search".to_string(),
            description: "High-level semantic text search executing in-process embedding + hybrid RRF or vector ranking".to_string(),
            params: json!({ "vector_table": "string", "fts_table": "string (optional)", "query": "string", "dimensions": "number (optional)", "top_k": "number (optional)" }),
        },
    ]
}

/// Retrieve the descriptor for a specific command ID.
pub fn describe_command(id: &str) -> Option<CommandDescriptor> {
    command_registry().into_iter().find(|cmd| cmd.id == id)
}
