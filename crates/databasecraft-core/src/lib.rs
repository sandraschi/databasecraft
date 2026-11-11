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

pub mod ai;
pub mod blueprints;
pub mod canvas;
pub mod error;
pub mod export;
pub mod models;
pub mod notifications;
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
pub const CMD_AI_NL_TO_SQL: &str = "ai.nl_to_sql";
pub const CMD_AI_SEMANTIC_DEDUPE: &str = "ai.semantic_dedupe";
pub const CMD_AI_SCHEMA_ADVISOR: &str = "ai.schema_advisor";
pub const CMD_AI_GENERATE_SYNTHETIC: &str = "ai.generate_synthetic";
pub const CMD_AI_INGEST_DOCUMENT: &str = "ai.ingest_document";
pub const CMD_DB_DATA_PROFILE: &str = "db.data_profile";
pub const CMD_DB_VACUUM: &str = "db.vacuum";
pub const CMD_DB_BACKUP: &str = "db.backup";
pub const CMD_DB_DIAGNOSE: &str = "db.diagnose";
pub const CMD_DB_REPAIR: &str = "db.repair";
pub const CMD_DB_CONSTRUCT: &str = "db.construct";
pub const CMD_DB_BLUEPRINTS: &str = "db.blueprints";
pub const CMD_CANVAS_QUERY: &str = "canvas.query";
pub const CMD_CANVAS_COMPILE: &str = "canvas.compile_sql";
pub const CMD_SCHEMA_RELATIONS: &str = "db.schema_relations";
pub const CMD_DB_UPDATE_CELL: &str = "db.update_cell";
pub const CMD_NOTIFICATIONS_LIST: &str = "notifications.list";
pub const CMD_NOTIFICATION_ACK: &str = "notifications.ack";
pub const CMD_WATCHDOG_CHECK: &str = "watchdog.check";
pub const CMD_WATCHDOG_RULES: &str = "watchdog.rules";

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
    AiNlToSql {
        prompt: String,
    },
    AiSemanticDedupe {
        table: String,
        column: String,
        threshold: Option<f32>,
    },
    AiSchemaAdvisor,
    AiGenerateSynthetic {
        table: String,
        count: usize,
        insert: Option<bool>,
    },
    AiIngestDocument {
        title: String,
        content: String,
        tags: Option<Vec<String>>,
    },
    DataProfile {
        table: String,
    },
    Vacuum,
    Backup {
        path: String,
    },
    Diagnose,
    Repair {
        action: String,
        salvage_path: Option<String>,
    },
    Construct {
        blueprint: String,
        seed_sample_data: Option<bool>,
        enable_search_indexes: Option<bool>,
    },
    ListBlueprints,
    CanvasQuery {
        spec: canvas::CanvasSpec,
    },
    CanvasCompileSql {
        spec: canvas::CanvasSpec,
    },
    SchemaRelations {
        table: String,
    },
    UpdateCell {
        table: String,
        pk_col: String,
        pk_val: Value,
        col: String,
        new_val: Value,
    },
    NotificationsList {
        severity: Option<notifications::NotificationSeverity>,
        unacknowledged_only: Option<bool>,
    },
    NotificationAck {
        id: String,
    },
    WatchdogCheck,
    WatchdogRules,
    WatchdogRecordMutation {
        table: String,
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
            notifications::global_notification_bus().record_mutation(&table);

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
            notifications::global_notification_bus().record_mutation(&table);

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
            notifications::global_notification_bus().record_mutation(&table);
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
        Command::AiNlToSql { prompt } => {
            let full_schema = store
                .inspect_all()
                .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;
            let nlq = ai::nl_to_sql(&prompt, &full_schema)?;
            Ok(json!({
                "success": true,
                "prompt": nlq.prompt,
                "sql": nlq.sql,
                "explanation": nlq.explanation,
                "read_only": nlq.read_only,
                "target_table": nlq.target_table,
                "confidence": nlq.confidence,
            }))
        }
        Command::AiSemanticDedupe {
            table,
            column,
            threshold,
        } => {
            validate_identifier(&table)?;
            validate_identifier(&column)?;
            let t = threshold.unwrap_or(0.75);
            let res = ai::semantic_dedupe(store, &table, &column, t)?;
            Ok(json!({
                "success": true,
                "table": res.table,
                "column": res.column,
                "threshold": res.threshold,
                "pairs_evaluated": res.pairs_evaluated,
                "duplicates_found": res.duplicates_found,
                "clusters": res.clusters,
            }))
        }
        Command::AiSchemaAdvisor => {
            let full_schema = store
                .inspect_all()
                .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;
            let res = ai::schema_advisor(store, &full_schema)?;
            Ok(json!({
                "success": true,
                "total_tables": res.total_tables,
                "issues_found": res.issues_found,
                "recommendations": res.recommendations,
            }))
        }
        Command::AiGenerateSynthetic {
            table,
            count,
            insert,
        } => {
            validate_identifier(&table)?;
            let should_insert = insert.unwrap_or(false);
            let res = ai::generate_synthetic(store, &table, count, should_insert)?;
            Ok(json!({
                "success": true,
                "table": res.table,
                "requested_count": res.requested_count,
                "inserted": res.inserted,
                "records": res.generated_records,
            }))
        }
        Command::AiIngestDocument {
            title,
            content,
            tags,
        } => {
            let t_tags = tags.unwrap_or_default();
            let res = ai::ingest_document(store, &title, &content, &t_tags)?;
            Ok(json!({
                "success": true,
                "document_title": res.document_title,
                "chunks_indexed": res.chunks_indexed,
                "target_table": res.target_table,
                "vector_table": res.vector_table,
                "fts_table": res.fts_table,
            }))
        }
        Command::DataProfile { table } => {
            validate_identifier(&table)?;
            let profile = store
                .profile_table(&table)
                .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;
            Ok(json!({
                "success": true,
                "profile": profile,
            }))
        }
        Command::Vacuum => {
            store
                .vacuum()
                .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;
            Ok(json!({
                "success": true,
                "status": "vacuum completed",
            }))
        }
        Command::Backup { path } => {
            store
                .backup_into(&path)
                .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;
            Ok(json!({
                "success": true,
                "backup_path": path,
            }))
        }
        Command::Diagnose => {
            let diag = store
                .diagnose()
                .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;
            Ok(json!({
                "success": true,
                "report": diag,
            }))
        }
        Command::Repair {
            action,
            salvage_path,
        } => {
            match action.to_lowercase().as_str() {
                "reindex" => {
                    store
                        .reindex_all()
                        .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;
                    let diag = store
                        .diagnose()
                        .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;
                    Ok(json!({
                        "success": true,
                        "action": "reindex",
                        "status": diag.status,
                    }))
                }
                "purge_orphans" => {
                    let purged = store
                        .purge_foreign_key_orphans()
                        .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;
                    let diag = store
                        .diagnose()
                        .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;
                    Ok(json!({
                        "success": true,
                        "action": "purge_orphans",
                        "orphans_purged": purged,
                        "status": diag.status,
                    }))
                }
                "salvage" => {
                    let target =
                        salvage_path.unwrap_or_else(|| "salvaged_database.dbcraft".to_string());
                    let report = store
                        .salvage_to(&target)
                        .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;
                    Ok(json!({
                        "success": true,
                        "action": "salvage",
                        "report": report,
                    }))
                }
                _ => {
                    // Default to auto_repair
                    let report = store
                        .auto_repair()
                        .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;
                    Ok(json!({
                        "success": true,
                        "action": "auto_repair",
                        "report": report,
                    }))
                }
            }
        }
        Command::Construct {
            blueprint,
            seed_sample_data,
            enable_search_indexes,
        } => {
            let seed = seed_sample_data.unwrap_or(true);
            let search = enable_search_indexes.unwrap_or(true);
            let res = blueprints::construct_blueprint(store, &blueprint, seed, search)?;
            Ok(json!({
                "success": true,
                "construction": res,
            }))
        }
        Command::ListBlueprints => {
            let list = blueprints::available_blueprints();
            Ok(json!({
                "success": true,
                "blueprints": list,
            }))
        }
        Command::CanvasQuery { spec } => {
            let result = canvas::CanvasCompiler::execute(store, &spec)?;
            Ok(json!({
                "success": true,
                "result": result,
            }))
        }
        Command::CanvasCompileSql { spec } => {
            let sql = canvas::CanvasCompiler::compile_sql(&spec)?;
            Ok(json!({
                "success": true,
                "sql": sql,
            }))
        }
        Command::SchemaRelations { table } => {
            let relations = canvas::resolve_table_relations(store, &table)?;
            Ok(json!({
                "success": true,
                "table": relations.table,
                "forward_relations": relations.forward_relations,
                "reverse_relations": relations.reverse_relations,
            }))
        }
        Command::UpdateCell {
            table,
            pk_col,
            pk_val,
            col,
            new_val,
        } => {
            validate_identifier(&table)?;
            validate_identifier(&pk_col)?;
            validate_identifier(&col)?;

            let affected = store
                .update_cell(&table, &pk_col, &pk_val, &col, &new_val)
                .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;

            // Record mutation for quiescence tracker
            notifications::global_notification_bus().record_mutation(&table);

            // Log info to Craft Notification Bus
            let now_secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let notif = notifications::CraftNotification {
                id: format!("edit-{table}-{now_secs}"),
                timestamp: format!("{now_secs}"),
                app: APP_NAME.to_string(),
                severity: notifications::NotificationSeverity::Info,
                category: notifications::NotificationCategory::MutationAudit,
                title: format!("Cell Updated in {table}"),
                message: format!("Cell updated in '{table}.{col}' for row {pk_col}={pk_val}"),
                source_entity: Some(format!("table:{table}")),
                metrics: json!({
                    "table": table,
                    "pk_col": pk_col,
                    "pk_val": pk_val,
                    "col": col,
                    "new_val": new_val,
                    "rows_affected": affected,
                }),
                acknowledged: false,
                suggested_action: None,
            };
            notifications::global_notification_bus().emit(notif);

            Ok(json!({
                "success": true,
                "table": table,
                "pk_col": pk_col,
                "pk_val": pk_val,
                "col": col,
                "new_val": new_val,
                "rows_affected": affected,
            }))
        }
        Command::NotificationsList {
            severity,
            unacknowledged_only,
        } => {
            let unack = unacknowledged_only.unwrap_or(false);
            let notifs = notifications::global_notification_bus().list(severity, unack);
            Ok(json!({
                "success": true,
                "count": notifs.len(),
                "notifications": notifs,
            }))
        }
        Command::NotificationAck { id } => {
            let found = notifications::global_notification_bus().acknowledge(&id);
            Ok(json!({
                "success": true,
                "acknowledged": found,
                "id": id,
            }))
        }
        Command::WatchdogCheck => {
            let alerts = notifications::global_notification_bus().evaluate_watchdog(store)?;
            let unack_count =
                notifications::global_notification_bus().unacknowledged_alert_count();
            Ok(json!({
                "success": true,
                "new_alerts_emitted": alerts.len(),
                "total_unacknowledged": unack_count,
                "alerts": alerts,
            }))
        }
        Command::WatchdogRules => {
            let conf = notifications::global_notification_bus().get_config();
            Ok(json!({
                "success": true,
                "config": conf,
            }))
        }
        Command::WatchdogRecordMutation { table } => {
            validate_identifier(&table)?;
            notifications::global_notification_bus().record_mutation(&table);
            Ok(json!({
                "success": true,
                "table": table,
            }))
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
        CommandDescriptor {
            id: CMD_AI_NL_TO_SQL.to_string(),
            label: "Natural Language to SQL".to_string(),
            description: "Translate natural language prompt into schema-aware SQLite query with explanation and safety fences".to_string(),
            params: json!({ "prompt": "string" }),
        },
        CommandDescriptor {
            id: CMD_AI_SEMANTIC_DEDUPE.to_string(),
            label: "Semantic Deduplication".to_string(),
            description: "Detect near-duplicate records in a text column using vector cosine similarity".to_string(),
            params: json!({ "table": "string", "column": "string", "threshold": "number (optional, default 0.75)" }),
        },
        CommandDescriptor {
            id: CMD_AI_SCHEMA_ADVISOR.to_string(),
            label: "Schema & Index Advisor".to_string(),
            description: "Inspect schema to recommend missing foreign key indexes, primary keys, and FTS5 search candidates".to_string(),
            params: json!({}),
        },
        CommandDescriptor {
            id: CMD_AI_GENERATE_SYNTHETIC.to_string(),
            label: "Generate Synthetic Rows".to_string(),
            description: "Generate realistic context-aware synthetic rows matching column types and foreign key integrity".to_string(),
            params: json!({ "table": "string", "count": "number", "insert": "boolean (optional, default false)" }),
        },
        CommandDescriptor {
            id: CMD_AI_INGEST_DOCUMENT.to_string(),
            label: "Ingest & Chunk Document".to_string(),
            description: "Chunk markdown/text document into sections and ingest into relational, vector, and FTS5 tables".to_string(),
            params: json!({ "title": "string", "content": "string", "tags": "array of strings (optional)" }),
        },
        CommandDescriptor {
            id: CMD_DB_DATA_PROFILE.to_string(),
            label: "Profile Table Data".to_string(),
            description: "Compute cardinality, null ratios, min/max bounds, and distinct counts across table columns".to_string(),
            params: json!({ "table": "string" }),
        },
        CommandDescriptor {
            id: CMD_DB_VACUUM.to_string(),
            label: "Vacuum Database".to_string(),
            description: "Rebuild database file to reclaim unused space and defragment".to_string(),
            params: json!({}),
        },
        CommandDescriptor {
            id: CMD_DB_BACKUP.to_string(),
            label: "Backup Database".to_string(),
            description: "Create atomic backup copy of active database into specified file path".to_string(),
            params: json!({ "path": "string" }),
        },
        CommandDescriptor {
            id: CMD_DB_DIAGNOSE.to_string(),
            label: "Diagnose Database Integrity".to_string(),
            description: "Run PRAGMA integrity_check and foreign_key_check to detect B-tree corruption and orphaned rows".to_string(),
            params: json!({}),
        },
        CommandDescriptor {
            id: CMD_DB_REPAIR.to_string(),
            label: "Repair Database".to_string(),
            description: "Execute database repair: reindex corrupted B-trees, purge orphaned FK records, or salvage to clean clone".to_string(),
            params: json!({ "action": "string (auto|reindex|purge_orphans|salvage)", "salvage_path": "string (optional)" }),
        },
        CommandDescriptor {
            id: CMD_DB_CONSTRUCT.to_string(),
            label: "Construct Domain Blueprint".to_string(),
            description: "Construct complete turnkey schema (small business, club, surgery, agency) with tables, foreign keys, and optional FTS5/seed data".to_string(),
            params: json!({ "blueprint": "string (small_business|club|surgery|creative_agency)", "seed_sample_data": "boolean (optional, default true)", "enable_search_indexes": "boolean (optional, default true)" }),
        },
        CommandDescriptor {
            id: CMD_DB_BLUEPRINTS.to_string(),
            label: "List Blueprint Archetypes".to_string(),
            description: "List available domain database blueprint archetypes and included tables".to_string(),
            params: json!({}),
        },
        CommandDescriptor {
            id: CMD_CANVAS_QUERY.to_string(),
            label: "Execute Relational Canvas Query".to_string(),
            description: "Execute hierarchical non-1NF perspective query with nested subqueries and rollups".to_string(),
            params: json!({ "spec": "CanvasSpec object" }),
        },
        CommandDescriptor {
            id: CMD_CANVAS_COMPILE.to_string(),
            label: "Compile Canvas to SQL".to_string(),
            description: "Compile a CanvasSpec AST into ANSI/SQLite JSON group SQL query without executing".to_string(),
            params: json!({ "spec": "CanvasSpec object" }),
        },
        CommandDescriptor {
            id: CMD_SCHEMA_RELATIONS.to_string(),
            label: "Discover Schema Relations".to_string(),
            description: "Inspect forward (N:1) and reverse (1:N) foreign-key links for a table for perspective sprouting".to_string(),
            params: json!({ "table": "string" }),
        },
        CommandDescriptor {
            id: CMD_DB_UPDATE_CELL.to_string(),
            label: "Update Table Cell".to_string(),
            description: "Atomically update a single cell by primary key with validation, mutation tracking, and notification bus event".to_string(),
            params: json!({
                "table": "string",
                "pk_col": "string",
                "pk_val": "any",
                "col": "string",
                "new_val": "any"
            }),
        },
        CommandDescriptor {
            id: CMD_NOTIFICATIONS_LIST.to_string(),
            label: "List Notifications & Alerts".to_string(),
            description: "Query operational health notifications, anomalies, and disk alerts".to_string(),
            params: json!({ "severity": "string (optional)", "unacknowledged_only": "boolean (optional)" }),
        },
        CommandDescriptor {
            id: CMD_NOTIFICATION_ACK.to_string(),
            label: "Acknowledge Notification".to_string(),
            description: "Mark an operational alert as acknowledged by its unique ID".to_string(),
            params: json!({ "id": "string" }),
        },
        CommandDescriptor {
            id: CMD_WATCHDOG_CHECK.to_string(),
            label: "Run Watchdog Anomaly Evaluation".to_string(),
            description: "Evaluate quiescence violation rates, integrity health, and disk thresholds across database".to_string(),
            params: json!({}),
        },
        CommandDescriptor {
            id: CMD_WATCHDOG_RULES.to_string(),
            label: "Get Watchdog Rules & Config".to_string(),
            description: "Inspect quiescent table watchlist and operational alerting thresholds".to_string(),
            params: json!({}),
        },
    ]
}

/// Retrieve the descriptor for a specific command ID.
pub fn describe_command(id: &str) -> Option<CommandDescriptor> {
    command_registry().into_iter().find(|cmd| cmd.id == id)
}
