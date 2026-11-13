//! L1 DatabaseCraft Store: SQLite & local-first relational storage engine.
//!
//! Enforces zero-panic, zero-unwrap containment over rusqlite.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

pub mod vector;

pub use vector::{
    cosine_similarity, dot_product, generate_text_embedding, l2_distance, pack_vector,
    unpack_vector, FtsMatch, HybridMatch, VectorMatch, VectorMetric,
};

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ColumnProfile {
    pub name: String,
    pub data_type: String,
    pub null_count: usize,
    pub distinct_count: usize,
    pub min_value: Option<String>,
    pub max_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TableProfile {
    pub table: String,
    pub total_rows: usize,
    pub columns: Vec<ColumnProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FkViolation {
    pub table: String,
    pub rowid: i64,
    pub target_table: String,
    pub fkid: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiagnosticReport {
    pub is_healthy: bool,
    pub integrity_ok: bool,
    pub integrity_errors: Vec<String>,
    pub fk_violations: Vec<FkViolation>,
    pub total_tables: usize,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RepairReport {
    pub action_taken: String,
    pub indexes_rebuilt: bool,
    pub orphans_purged: usize,
    pub vacuum_succeeded: bool,
    pub salvage_path: Option<String>,
    pub final_health_status: String,
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

    /// Update a single cell identifying the target row by its primary key column.
    pub fn update_cell(
        &self,
        table: &str,
        pk_col: &str,
        pk_val: &serde_json::Value,
        col: &str,
        new_val: &serde_json::Value,
    ) -> Result<usize, StoreError> {
        if !table.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(StoreError::InvalidInput(format!(
                "invalid table name '{table}'"
            )));
        }
        if !pk_col
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(StoreError::InvalidInput(format!(
                "invalid primary key column '{pk_col}'"
            )));
        }
        if !col.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(StoreError::InvalidInput(format!(
                "invalid target column '{col}'"
            )));
        }

        let conn = self
            .conn
            .lock()
            .map_err(|e| StoreError::Lock(e.to_string()))?;

        let sql = format!("UPDATE \"{table}\" SET \"{col}\" = ?1 WHERE \"{pk_col}\" = ?2;");
        let mut stmt = conn.prepare(&sql)?;
        let val_param = json_to_sqlite_value(new_val);
        let pk_param = json_to_sqlite_value(pk_val);

        let rows = stmt.execute(rusqlite::params![val_param, pk_param])?;
        Ok(rows)
    }

    /// Discover the primary key column name for a given table, if one exists.
    pub fn get_primary_key(&self, table_name: &str) -> Result<Option<String>, StoreError> {
        let schema = self.table_schema(table_name)?;
        Ok(schema.columns.into_iter().find(|c| c.pk).map(|c| c.name))
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

    /// Initialize a vector table for dense vector embeddings.
    pub fn init_vector_table(
        &self,
        table_name: &str,
        _dimensions: usize,
    ) -> Result<(), StoreError> {
        if !table_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(StoreError::InvalidInput(format!(
                "table name '{table_name}' contains illegal characters"
            )));
        }
        let sql = format!(
            "CREATE TABLE IF NOT EXISTS \"{table_name}\" (
                id INTEGER PRIMARY KEY,
                vector BLOB NOT NULL,
                dimensions INTEGER NOT NULL,
                metadata TEXT
            );"
        );
        self.execute(&sql)?;
        Ok(())
    }

    /// Insert or replace a vector embedding with associated JSON metadata.
    pub fn insert_vector(
        &self,
        table_name: &str,
        id: i64,
        vector: &[f32],
        metadata: &serde_json::Value,
    ) -> Result<(), StoreError> {
        if !table_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(StoreError::InvalidInput(format!(
                "table name '{table_name}' contains illegal characters"
            )));
        }
        let packed = pack_vector(vector);
        let meta_str = metadata.to_string();
        let dims = vector.len() as i64;

        let conn = self
            .conn
            .lock()
            .map_err(|e| StoreError::Lock(e.to_string()))?;
        let sql = format!("INSERT OR REPLACE INTO \"{table_name}\" (id, vector, dimensions, metadata) VALUES (?, ?, ?, ?);");
        let mut stmt = conn.prepare(&sql)?;
        stmt.execute(rusqlite::params![id, packed, dims, meta_str])?;
        Ok(())
    }

    /// Execute a dense vector similarity search across a vector table.
    pub fn search_vector(
        &self,
        table_name: &str,
        query_vec: &[f32],
        metric: VectorMetric,
        top_k: usize,
    ) -> Result<Vec<VectorMatch>, StoreError> {
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
        let sql = format!("SELECT id, vector, metadata FROM \"{table_name}\";");
        let mut stmt = conn.prepare(&sql)?;

        let mut matches = Vec::new();
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let id: i64 = row.get(0)?;
            let blob: Vec<u8> = row.get(1)?;
            let meta_str: Option<String> = row.get(2)?;
            let candidate_vec = unpack_vector(&blob);

            let score = match metric {
                VectorMetric::Cosine => cosine_similarity(query_vec, &candidate_vec),
                VectorMetric::DotProduct => dot_product(query_vec, &candidate_vec),
                VectorMetric::L2 => {
                    let dist = l2_distance(query_vec, &candidate_vec);
                    1.0 / (1.0 + dist) // normalize distance to [0, 1] similarity
                }
            };

            let metadata = meta_str
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or(serde_json::Value::Null);

            matches.push(VectorMatch {
                id,
                score,
                metadata,
            });
        }

        // Sort descending by score
        matches.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        matches.truncate(top_k);
        Ok(matches)
    }

    /// Initialize an SQLite FTS5 full-text search table for sparse keyword indexing.
    pub fn init_fts(&self, fts_table: &str, columns: &[&str]) -> Result<(), StoreError> {
        if !fts_table
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(StoreError::InvalidInput(format!(
                "fts table name '{fts_table}' contains illegal characters"
            )));
        }
        let cols_sql = columns
            .iter()
            .map(|c| format!("\"{}\"", c))
            .collect::<Vec<_>>()
            .join(", ");
        let sql =
            format!("CREATE VIRTUAL TABLE IF NOT EXISTS \"{fts_table}\" USING fts5({cols_sql});");
        self.execute(&sql)?;
        Ok(())
    }

    /// Execute an SQLite FTS5 full-text search returning matches ranked by BM25.
    pub fn search_fts(
        &self,
        fts_table: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<FtsMatch>, StoreError> {
        if !fts_table
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(StoreError::InvalidInput(format!(
                "fts table name '{fts_table}' contains illegal characters"
            )));
        }
        let conn = self
            .conn
            .lock()
            .map_err(|e| StoreError::Lock(e.to_string()))?;
        let sql = format!(
            "SELECT rowid, rank FROM \"{fts_table}\" WHERE \"{fts_table}\" MATCH ? ORDER BY rank LIMIT ?;"
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params![query, top_k as i64], |row| {
            Ok(FtsMatch {
                id: row.get(0)?,
                rank: row.get(1)?,
                snippet: String::new(),
            })
        })?;

        let mut matches = Vec::new();
        for r in rows {
            matches.push(r?);
        }
        Ok(matches)
    }

    /// Execute a Hybrid RAG search: combines dense vector similarity with sparse BM25 keyword matching via Reciprocal Rank Fusion (RRF).
    pub fn hybrid_search(
        &self,
        vector_table: &str,
        fts_table: &str,
        query_text: &str,
        query_vec: &[f32],
        top_k: usize,
        rrf_k: f64,
    ) -> Result<Vec<HybridMatch>, StoreError> {
        let vector_hits =
            self.search_vector(vector_table, query_vec, VectorMetric::Cosine, top_k * 2)?;
        let fts_hits = self
            .search_fts(fts_table, query_text, top_k * 2)
            .unwrap_or_default();

        let mut rrf_scores: std::collections::HashMap<
            i64,
            (f64, Option<f32>, Option<f64>, serde_json::Value),
        > = std::collections::HashMap::new();

        for (rank, hit) in vector_hits.iter().enumerate() {
            let rrf_inc = 1.0 / (rrf_k + (rank as f64 + 1.0));
            let entry = rrf_scores
                .entry(hit.id)
                .or_insert((0.0, None, None, hit.metadata.clone()));
            entry.0 += rrf_inc;
            entry.1 = Some(hit.score);
        }

        for (rank, hit) in fts_hits.iter().enumerate() {
            let rrf_inc = 1.0 / (rrf_k + (rank as f64 + 1.0));
            let entry =
                rrf_scores
                    .entry(hit.id)
                    .or_insert((0.0, None, None, serde_json::Value::Null));
            entry.0 += rrf_inc;
            entry.2 = Some(hit.rank);
        }

        let mut hybrid_matches = Vec::new();
        for (id, (score, vec_score, fts_rank, meta)) in rrf_scores {
            hybrid_matches.push(HybridMatch {
                id,
                rrf_score: score,
                vector_score: vec_score,
                fts_rank,
                metadata: meta,
            });
        }

        hybrid_matches.sort_by(|a, b| {
            b.rrf_score
                .partial_cmp(&a.rrf_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        hybrid_matches.truncate(top_k);
        Ok(hybrid_matches)
    }

    /// Vacuum the database to reclaim space and defragment.
    pub fn vacuum(&self) -> Result<(), StoreError> {
        self.execute("VACUUM;")?;
        Ok(())
    }

    /// Backup the database into another file using SQLite VACUUM INTO.
    pub fn backup_into(&self, target_path: &str) -> Result<(), StoreError> {
        let escaped = target_path.replace('\'', "''");
        let sql = format!("VACUUM INTO '{escaped}';");
        self.execute(&sql)?;
        Ok(())
    }

    /// List existing indexes for a given table.
    pub fn list_indexes(&self, table_name: &str) -> Result<Vec<String>, StoreError> {
        if !table_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(StoreError::InvalidInput(format!(
                "table name '{table_name}' contains illegal characters"
            )));
        }
        let qr = self.query(&format!("PRAGMA index_list(\"{table_name}\");"))?;
        let mut idxs = Vec::new();
        for row in qr.rows {
            if let Some(val) = row.get(1) {
                if let Some(s) = val.as_str() {
                    idxs.push(s.to_string());
                }
            }
        }
        Ok(idxs)
    }

    /// Profile table columns for distribution, null counts, and distinct cardinality.
    pub fn profile_table(&self, table_name: &str) -> Result<TableProfile, StoreError> {
        let schema = self.table_schema(table_name)?;
        let count_qr = self.query(&format!("SELECT COUNT(*) FROM \"{table_name}\";"))?;
        let total_rows = count_qr
            .rows
            .first()
            .and_then(|r| r.first())
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        let mut columns = Vec::with_capacity(schema.columns.len());
        for col in &schema.columns {
            let col_name = &col.name;
            let stat_sql = format!(
                "SELECT \
                    COUNT(CASE WHEN \"{col_name}\" IS NULL THEN 1 END) AS null_cnt, \
                    COUNT(DISTINCT \"{col_name}\") AS dist_cnt, \
                    MIN(\"{col_name}\") AS min_val, \
                    MAX(\"{col_name}\") AS max_val \
                 FROM \"{table_name}\";"
            );
            let stat_qr = self.query(&stat_sql)?;
            let (null_count, distinct_count, min_val, max_val) =
                if let Some(row) = stat_qr.rows.first() {
                    let nulls = row.first().and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                    let dist = row.get(1).and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                    let min_str = row.get(2).filter(|v| !v.is_null()).map(|v| v.to_string());
                    let max_str = row.get(3).filter(|v| !v.is_null()).map(|v| v.to_string());
                    (nulls, dist, min_str, max_str)
                } else {
                    (0, 0, None, None)
                };

            columns.push(ColumnProfile {
                name: col_name.clone(),
                data_type: col.data_type.clone(),
                null_count,
                distinct_count,
                min_value: min_val,
                max_value: max_val,
            });
        }

        Ok(TableProfile {
            table: table_name.to_string(),
            total_rows,
            columns,
        })
    }

    /// Run comprehensive PRAGMA integrity_check and PRAGMA foreign_key_check.
    pub fn diagnose(&self) -> Result<DiagnosticReport, StoreError> {
        // 1. Run PRAGMA integrity_check
        let integrity_qr = self.query("PRAGMA integrity_check;")?;
        let mut integrity_errors = Vec::new();
        let mut integrity_ok = true;

        for row in &integrity_qr.rows {
            if let Some(val) = row.first() {
                if let Some(msg) = val.as_str() {
                    if msg.to_lowercase() != "ok" {
                        integrity_ok = false;
                        integrity_errors.push(msg.to_string());
                    }
                }
            }
        }

        // 2. Run PRAGMA foreign_key_check
        let fk_qr = self.query("PRAGMA foreign_key_check;")?;
        let mut fk_violations = Vec::new();

        for row in &fk_qr.rows {
            if let (Some(tbl), Some(rowid_val), Some(target_tbl), Some(fkid_val)) =
                (row.first(), row.get(1), row.get(2), row.get(3))
            {
                let table = tbl.as_str().unwrap_or("unknown").to_string();
                let rowid = rowid_val.as_i64().unwrap_or(0);
                let target_table = target_tbl.as_str().unwrap_or("unknown").to_string();
                let fkid = fkid_val.as_i64().unwrap_or(0);

                fk_violations.push(FkViolation {
                    table,
                    rowid,
                    target_table,
                    fkid,
                });
            }
        }

        let tables = self.list_tables()?;
        let is_healthy = integrity_ok && fk_violations.is_empty();
        let status = if !integrity_ok {
            "CORRUPTED".to_string()
        } else if !fk_violations.is_empty() {
            "DEGRADED".to_string()
        } else {
            "HEALTHY".to_string()
        };

        Ok(DiagnosticReport {
            is_healthy,
            integrity_ok,
            integrity_errors,
            fk_violations,
            total_tables: tables.len(),
            status,
        })
    }

    /// Rebuild all indexes in the database.
    pub fn reindex_all(&self) -> Result<(), StoreError> {
        self.execute("REINDEX;")?;
        Ok(())
    }

    /// Purge orphaned foreign key rows that violate relational constraints.
    pub fn purge_foreign_key_orphans(&self) -> Result<usize, StoreError> {
        let diag = self.diagnose()?;
        let mut total_purged = 0;

        for violation in &diag.fk_violations {
            if !violation
                .table
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
            {
                continue;
            }
            let delete_sql = format!(
                "DELETE FROM \"{}\" WHERE rowid = {};",
                violation.table, violation.rowid
            );
            if let Ok(affected) = self.execute(&delete_sql) {
                total_purged += affected;
            }
        }

        Ok(total_purged)
    }

    /// Execute automated database repair sequence (REINDEX + purge orphans + VACUUM).
    pub fn auto_repair(&self) -> Result<RepairReport, StoreError> {
        // Step 1: Reindex
        let reindex_ok = self.reindex_all().is_ok();

        // Step 2: Purge orphaned FK rows
        let orphans_purged = self.purge_foreign_key_orphans().unwrap_or(0);

        // Step 3: Vacuum defragmentation
        let vacuum_ok = self.vacuum().is_ok();

        // Verify final state
        let final_diag = self.diagnose()?;

        Ok(RepairReport {
            action_taken: "auto_repair (REINDEX + PURGE_ORPHANS + VACUUM)".to_string(),
            indexes_rebuilt: reindex_ok,
            orphans_purged,
            vacuum_succeeded: vacuum_ok,
            salvage_path: None,
            final_health_status: final_diag.status,
        })
    }

    /// Salvage database: dump surviving table definitions and records into a fresh clean database file.
    pub fn salvage_to(&self, target_path: &str) -> Result<RepairReport, StoreError> {
        let clean_store = DatabaseStore::open(target_path)?;
        let full_schema = self.inspect_all()?;
        let mut total_recovered_rows = 0;

        for tbl in &full_schema.tables {
            // Re-create table schema in clean store
            let mut col_defs = Vec::new();
            for col in &tbl.columns {
                let pk_str = if col.pk { " PRIMARY KEY" } else { "" };
                let notnull_str = if col.notnull { " NOT NULL" } else { "" };
                col_defs.push(format!(
                    "\"{}\" {}{}{}",
                    col.name, col.data_type, pk_str, notnull_str
                ));
            }
            let ddl = format!(
                "CREATE TABLE IF NOT EXISTS \"{}\" ({});",
                tbl.name,
                col_defs.join(", ")
            );
            let _ = clean_store.execute(&ddl);

            // Copy rows safely row-by-row
            if let Ok(qr) = self.query(&format!("SELECT * FROM \"{}\";", tbl.name)) {
                for row in &qr.rows {
                    let mut col_names = Vec::new();
                    let mut val_strs = Vec::new();
                    for (col_name, val) in qr.columns.iter().zip(row.iter()) {
                        col_names.push(format!("\"{}\"", col_name));
                        match val {
                            serde_json::Value::Null => val_strs.push("NULL".to_string()),
                            serde_json::Value::Bool(b) => {
                                val_strs.push(if *b { "1".to_string() } else { "0".to_string() })
                            }
                            serde_json::Value::Number(n) => val_strs.push(n.to_string()),
                            serde_json::Value::String(s) => {
                                val_strs.push(format!("'{}'", s.replace('\'', "''")))
                            }
                            _ => {
                                val_strs.push(format!("'{}'", val.to_string().replace('\'', "''")))
                            }
                        }
                    }
                    if !col_names.is_empty() {
                        let ins = format!(
                            "INSERT OR IGNORE INTO \"{}\" ({}) VALUES ({});",
                            tbl.name,
                            col_names.join(", "),
                            val_strs.join(", ")
                        );
                        if clean_store.execute(&ins).is_ok() {
                            total_recovered_rows += 1;
                        }
                    }
                }
            }
        }

        let final_diag = clean_store.diagnose()?;

        Ok(RepairReport {
            action_taken: format!(
                "salvage_dump (recovered {total_recovered_rows} rows to new file)"
            ),
            indexes_rebuilt: true,
            orphans_purged: 0,
            vacuum_succeeded: true,
            salvage_path: Some(target_path.to_string()),
            final_health_status: final_diag.status,
        })
    }
}

/// Convert a serde_json::Value to a rusqlite::types::Value.
pub fn json_to_sqlite_value(val: &serde_json::Value) -> rusqlite::types::Value {
    match val {
        serde_json::Value::Null => rusqlite::types::Value::Null,
        serde_json::Value::Bool(b) => rusqlite::types::Value::Integer(if *b { 1 } else { 0 }),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                rusqlite::types::Value::Integer(i)
            } else if let Some(f) = n.as_f64() {
                rusqlite::types::Value::Real(f)
            } else {
                rusqlite::types::Value::Text(n.to_string())
            }
        }
        serde_json::Value::String(s) => rusqlite::types::Value::Text(s.clone()),
        _ => rusqlite::types::Value::Text(val.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_cell_and_get_primary_key() -> Result<(), Box<dyn std::error::Error>> {
        let store = DatabaseStore::open_in_memory()?;
        store.execute(
            "CREATE TABLE users (id INTEGER PRIMARY KEY, username TEXT, balance REAL, active INTEGER);
             INSERT INTO users (id, username, balance, active) VALUES (1, 'alice', 100.50, 1);",
        )?;

        let pk = store.get_primary_key("users")?;
        assert_eq!(pk, Some("id".to_string()));

        let affected = store.update_cell(
            "users",
            "id",
            &serde_json::json!(1),
            "username",
            &serde_json::json!("alice_updated"),
        )?;
        assert_eq!(affected, 1);

        let qr = store.query("SELECT username FROM users WHERE id = 1;")?;
        assert_eq!(qr.rows.len(), 1);
        assert_eq!(
            qr.rows.first().and_then(|r| r.first()),
            Some(&serde_json::Value::String("alice_updated".to_string()))
        );

        Ok(())
    }

    #[test]
    fn test_text_embedding_properties() -> Result<(), Box<dyn std::error::Error>> {
        let v1 = generate_text_embedding("wireless bluetooth headphones", 128);
        assert_eq!(v1.len(), 128);

        // Unit length check: norm ~ 1.0
        let norm_sq: f32 = v1.iter().map(|&x| x * x).sum();
        assert!((norm_sq.sqrt() - 1.0).abs() < 1e-4);

        // Deterministic check
        let v1_again = generate_text_embedding("wireless bluetooth headphones", 128);
        assert_eq!(v1, v1_again);

        // Semantic proximity: "wireless headphone" should have high cosine similarity with "wireless bluetooth headphones"
        let v2 = generate_text_embedding("wireless headphone", 128);
        let sim = cosine_similarity(&v1, &v2);
        assert!(sim > 0.4);

        // Empty string gives zero vector
        let empty = generate_text_embedding("", 128);
        assert!(empty.iter().all(|&x| x == 0.0));

        Ok(())
    }
}
