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
pub const APP_VERSION: &str = "0.1.0";

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
    }
}
