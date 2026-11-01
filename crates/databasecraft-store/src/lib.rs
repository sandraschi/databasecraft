//! L1 DatabaseCraft Store: SQLite & local-first relational storage engine.
//!
//! Enforces zero-panic, zero-unwrap containment over rusqlite.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use rusqlite::{types::ValueRef, Connection};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;
use std::sync::Mutex;

#[derive(Debug)]
pub enum StoreError {
    Sqlite(String),
    Lock(String),
    InvalidInput(String),
    NotFound(String),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::Sqlite(msg) => write!(f, "sqlite error: {msg}"),
            StoreError::Lock(msg) => write!(f, "store lock poisoned: {msg}"),
            StoreError::InvalidInput(msg) => write!(f, "invalid input: {msg}"),
            StoreError::NotFound(msg) => write!(f, "not found: {msg}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<rusqlite::Error> for StoreError {
    fn from(err: rusqlite::Error) -> Self {
        StoreError::Sqlite(err.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ColumnInfo {
    pub cid: i64,
    pub name: String,
    pub data_type: String,
    pub notnull: bool,
    pub dflt_value: Option<String>,
    pub pk: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ForeignKeyInfo {
    pub id: i64,
    pub seq: i64,
    pub table: String,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TableSchema {
    pub name: String,
    pub columns: Vec<ColumnInfo>,
    pub foreign_keys: Vec<ForeignKeyInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FullSchema {
    pub tables: Vec<TableSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub row_count: usize,
}

pub struct DatabaseStore {
    conn: Mutex<Connection>,
}

impl DatabaseStore {
    /// Open an in-memory SQLite database.
    pub fn open_in_memory() -> Result<Self, StoreError> {
        let conn = Connection::open_in_memory()?;
        // Enable WAL-equivalent pragmas for integrity
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;",
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Open a disk-backed SQLite database (.dbcraft, .sqlite, .db).
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;",
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Execute raw DDL or mutation SQL without returning rows.
    pub fn execute(&self, sql: &str) -> Result<usize, StoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StoreError::Lock(e.to_string()))?;
        conn.execute_batch(sql)?;
        Ok(conn.changes() as usize)
    }

    /// Execute a SELECT query and return structured JSON rows and columns.
    pub fn query(&self, sql: &str) -> Result<QueryResult, StoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StoreError::Lock(e.to_string()))?;

        let mut stmt = conn.prepare(sql)?;
        let col_count = stmt.column_count();
        let mut columns = Vec::with_capacity(col_count);
        for col in stmt.column_names() {
            columns.push(col.to_string());
        }

        let mut rows = Vec::new();
        let mut query_rows = stmt.query([])?;

        while let Some(row) = query_rows.next()? {
            let mut row_values = Vec::with_capacity(col_count);
            for i in 0..col_count {
                let val_ref = row.get_ref(i)?;
                let json_val = match val_ref {
                    ValueRef::Null => serde_json::Value::Null,
                    ValueRef::Integer(n) => serde_json::Value::Number(n.into()),
                    ValueRef::Real(f) => serde_json::Number::from_f64(f)
                        .map(serde_json::Value::Number)
                        .unwrap_or(serde_json::Value::Null),
                    ValueRef::Text(s) => {
                        let text = String::from_utf8_lossy(s).to_string();
                        serde_json::Value::String(text)
                    }
                    ValueRef::Blob(b) => {
                        let hex = b
                            .iter()
                            .map(|byte| format!("{:02x}", byte))
                            .collect::<String>();
                        serde_json::Value::String(format!("0x{hex}"))
                    }
                };
                row_values.push(json_val);
            }
            rows.push(row_values);
        }

        let row_count = rows.len();
        Ok(QueryResult {
            columns,
            rows,
            row_count,
        })
    }

    /// List all non-internal user tables in the database.
    pub fn list_tables(&self) -> Result<Vec<String>, StoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StoreError::Lock(e.to_string()))?;
        let mut stmt = conn.prepare(
            "SELECT name FROM sqlite_master 
             WHERE type='table' AND name NOT LIKE 'sqlite_%' 
             ORDER BY name ASC;",
        )?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut tables = Vec::new();
        for r in rows {
            tables.push(r?);
        }
        Ok(tables)
    }

    /// Inspect columns and foreign keys for a specific table.
    pub fn table_schema(&self, table_name: &str) -> Result<TableSchema, StoreError> {
        // Sanitize table name (prevent SQL injection into PRAGMA)
        if !table_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(StoreError::InvalidInput(format!(
                "table name '{table_name}' contains illegal characters"
            )));
        }

        let conn = self
            .conn
            .lock()
            .map_err(|e| StoreError::Lock(e.to_string()))?;

        // 1. Columns
        let mut col_stmt = conn.prepare(&format!("PRAGMA table_info(\"{table_name}\");"))?;
        let col_rows = col_stmt.query_map([], |row| {
            Ok(ColumnInfo {
                cid: row.get(0)?,
                name: row.get(1)?,
                data_type: row.get(2)?,
                notnull: row.get::<_, i64>(3)? != 0,
                dflt_value: row.get(4)?,
                pk: row.get::<_, i64>(5)? != 0,
            })
        })?;

        let mut columns = Vec::new();
        for col in col_rows {
            columns.push(col?);
        }

        // 2. Foreign keys
        let mut fk_stmt = conn.prepare(&format!("PRAGMA foreign_key_list(\"{table_name}\");"))?;
        let fk_rows = fk_stmt.query_map([], |row| {
            Ok(ForeignKeyInfo {
                id: row.get(0)?,
                seq: row.get(1)?,
                table: row.get(2)?,
                from: row.get(3)?,
                to: row.get(4)?,
            })
        })?;

        let mut foreign_keys = Vec::new();
        for fk in fk_rows {
            foreign_keys.push(fk?);
        }

        Ok(TableSchema {
            name: table_name.to_string(),
            columns,
            foreign_keys,
        })
    }

    /// Inspect the entire database schema (all tables, columns, foreign keys).
    pub fn inspect_all(&self) -> Result<FullSchema, StoreError> {
        let tables = self.list_tables()?;
        let mut schemas = Vec::with_capacity(tables.len());
        for table in tables {
            schemas.push(self.table_schema(&table)?);
        }
        Ok(FullSchema { tables: schemas })
    }
}
