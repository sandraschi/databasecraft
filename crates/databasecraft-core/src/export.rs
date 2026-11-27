//! Cross-Craft Export Engine: Formats relational query results directly
//! for consumption by DesignCraft, DeckCraft, WordCraft, GridCraft, and PdfCraft,
//! and exports self-contained Datasette (`datasette.io`) bundles.
use crate::canvas::{CanvasCompiler, CanvasSpec};
use crate::error::DatabaseCraftError;
use crate::models::ExportTarget;
use databasecraft_store::{DatabaseStore, QueryResult};
use serde_json::{json, Map, Value};
use std::path::Path;

pub fn format_for_craft(
    query_result: &QueryResult,
    target: ExportTarget,
    dataset_name: &str,
) -> Result<Value, DatabaseCraftError> {
    match target {
        ExportTarget::DesignCraft => format_designcraft(query_result, dataset_name),
        ExportTarget::DeckCraft => format_deckcraft(query_result, dataset_name),
        ExportTarget::WordCraft => format_wordcraft(query_result, dataset_name),
        ExportTarget::GridCraft => format_gridcraft(query_result),
        ExportTarget::PdfCraft => format_pdfcraft(query_result, dataset_name),
    }
}

/// DesignCraft Data Merge format (spread & catalog variable data)
fn format_designcraft(qr: &QueryResult, name: &str) -> Result<Value, DatabaseCraftError> {
    let mut records = Vec::with_capacity(qr.row_count);
    for row in &qr.rows {
        let mut map = serde_json::Map::new();
        for (idx, col_name) in qr.columns.iter().enumerate() {
            let val = row.get(idx).cloned().unwrap_or(Value::Null);
            map.insert(col_name.clone(), val);
        }
        records.push(Value::Object(map));
    }

    Ok(json!({
        "app": "designcraft",
        "format": "data_merge",
        "dataset": name,
        "fields": qr.columns,
        "record_count": qr.row_count,
        "records": records,
    }))
}

/// DeckCraft slides presentation format (slide tables and charts)
fn format_deckcraft(qr: &QueryResult, title: &str) -> Result<Value, DatabaseCraftError> {
    Ok(json!({
        "app": "deckcraft",
        "format": "slide_table",
        "title": title,
        "slide_count": 1,
        "slides": [{
            "header": title,
            "table": {
                "columns": qr.columns,
                "rows": qr.rows,
                "row_count": qr.row_count,
            }
        }]
    }))
}

/// WordCraft Mail-Merge dictionary format
fn format_wordcraft(qr: &QueryResult, name: &str) -> Result<Value, DatabaseCraftError> {
    let mut merge_entries = Vec::with_capacity(qr.row_count);
    for row in &qr.rows {
        let mut map = serde_json::Map::new();
        for (idx, col_name) in qr.columns.iter().enumerate() {
            let val = row.get(idx).cloned().unwrap_or(Value::Null);
            map.insert(col_name.clone(), val);
        }
        merge_entries.push(Value::Object(map));
    }

    Ok(json!({
        "app": "wordcraft",
        "format": "mail_merge",
        "template_dataset": name,
        "entries": merge_entries,
    }))
}

/// GridCraft tabular CSV string representation
fn format_gridcraft(qr: &QueryResult) -> Result<Value, DatabaseCraftError> {
    let mut csv_lines = Vec::new();
    csv_lines.push(qr.columns.join(","));

    for row in &qr.rows {
        let line = row
            .iter()
            .map(|val| match val {
                Value::Null => "".to_string(),
                Value::Bool(b) => b.to_string(),
                Value::Number(n) => n.to_string(),
                Value::String(s) => {
                    if s.contains(',') || s.contains('"') || s.contains('\n') {
                        format!("\"{}\"", s.replace('"', "\"\""))
                    } else {
                        s.clone()
                    }
                }
                Value::Array(a) => format!("\"[array:{}]\"", a.len()),
                Value::Object(_) => "\"[object]\"".to_string(),
            })
            .collect::<Vec<_>>()
            .join(",");
        csv_lines.push(line);
    }

    Ok(json!({
        "app": "gridcraft",
        "format": "csv",
        "row_count": qr.row_count,
        "csv_text": csv_lines.join("\n"),
    }))
}

/// PdfCraft printable structured report layout format
fn format_pdfcraft(qr: &QueryResult, title: &str) -> Result<Value, DatabaseCraftError> {
    Ok(json!({
        "app": "pdfcraft",
        "format": "report_table",
        "title": title,
        "columns": qr.columns,
        "rows": qr.rows,
        "row_count": qr.row_count,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}

fn sanitize_db_stem(title: Option<&str>) -> String {
    let raw = title.unwrap_or("database").trim();
    let cleaned: String = raw
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c.to_ascii_lowercase()
            } else if c.is_whitespace() {
                '_'
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches(|c| c == '-' || c == '_');
    if trimmed.is_empty() {
        "database".to_string()
    } else {
        trimmed.to_string()
    }
}

fn pick_label_column(columns: &[databasecraft_store::ColumnInfo]) -> Option<String> {
    const CANDIDATES: [&str; 4] = ["name", "title", "label", "sku"];
    for candidate in CANDIDATES {
        if let Some(col) = columns
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(candidate))
        {
            return Some(col.name.clone());
        }
    }
    None
}

/// Export a clean SQLite snapshot and Datasette `metadata.json` (with table
/// metadata and canned queries compiled from saved Canvas perspectives).
pub fn export_datasette(
    store: &DatabaseStore,
    output_dir: &str,
    title: Option<&str>,
    include_internal: bool,
) -> Result<Value, DatabaseCraftError> {
    let out_dir_trimmed = output_dir.trim();
    if out_dir_trimmed.is_empty() {
        return Err(DatabaseCraftError::InvalidInput(
            "output_dir cannot be empty".to_string(),
        ));
    }

    let out_path = Path::new(out_dir_trimmed);
    std::fs::create_dir_all(out_path).map_err(|e| {
        DatabaseCraftError::Export(format!(
            "failed to create output directory '{}': {e}",
            out_path.display()
        ))
    })?;

    let db_stem = sanitize_db_stem(title);
    let sqlite_path = out_path.join(format!("{db_stem}.sqlite"));
    let metadata_path = out_path.join("metadata.json");

    if sqlite_path.exists() {
        std::fs::remove_file(&sqlite_path).map_err(|e| {
            DatabaseCraftError::Export(format!(
                "failed to replace existing sqlite export '{}': {e}",
                sqlite_path.display()
            ))
        })?;
    }

    let sqlite_path_str = sqlite_path.to_string_lossy().to_string();
    store
        .backup_into(&sqlite_path_str)
        .map_err(|e| DatabaseCraftError::Export(format!("sqlite snapshot failed: {e}")))?;

    if !include_internal {
        let exported_store = DatabaseStore::open(&sqlite_path).map_err(|e| {
            DatabaseCraftError::Export(format!("failed to reopen exported sqlite: {e}"))
        })?;
        exported_store.drop_internal_tables().map_err(|e| {
            DatabaseCraftError::Export(format!("failed to strip internal tables: {e}"))
        })?;
    }

    let schema = store.inspect_all()?;
    let mut tables_meta = Map::new();
    let mut tables_exported = Vec::new();

    for table in &schema.tables {
        tables_exported.push(table.name.clone());
        let mut t_obj = Map::new();
        t_obj.insert("title".to_string(), Value::String(table.name.clone()));
        if let Some(label_col) = pick_label_column(&table.columns) {
            t_obj.insert("label_column".to_string(), Value::String(label_col));
        }
        tables_meta.insert(table.name.clone(), Value::Object(t_obj));
    }

    let mut queries_meta = Map::new();
    let mut queries_exported = Vec::new();
    let mut warnings = Vec::new();

    let perspectives = store.list_perspectives()?;
    for (name, _root_table, spec_json) in perspectives {
        match serde_json::from_str::<CanvasSpec>(&spec_json) {
            Ok(spec) => match CanvasCompiler::compile_sql(&spec) {
                Ok(sql) => {
                    queries_exported.push(name.clone());
                    queries_meta.insert(
                        name.clone(),
                        json!({
                            "title": name,
                            "sql": sql,
                        }),
                    );
                }
                Err(e) => warnings.push(format!("perspective '{name}' failed SQL compile: {e}")),
            },
            Err(e) => warnings.push(format!("perspective '{name}' failed to parse: {e}")),
        }
    }

    let display_title = title
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("DatabaseCraft Export");

    let metadata = json!({
        "title": display_title,
        "description": "Exported from DatabaseCraft (.dbcraft)",
        "databases": {
            db_stem.clone(): {
                "title": display_title,
                "tables": Value::Object(tables_meta),
                "queries": Value::Object(queries_meta),
            }
        }
    });

    let metadata_json = serde_json::to_string_pretty(&metadata).map_err(|e| {
        DatabaseCraftError::Export(format!("failed to serialize metadata.json: {e}"))
    })?;
    std::fs::write(&metadata_path, metadata_json).map_err(|e| {
        DatabaseCraftError::Export(format!(
            "failed to write metadata.json '{}': {e}",
            metadata_path.display()
        ))
    })?;

    let metadata_path_str = metadata_path.to_string_lossy().to_string();
    let launch_command = format!("uvx datasette \"{sqlite_path_str}\" -m \"{metadata_path_str}\"");

    Ok(json!({
        "success": true,
        "output_dir": out_path.to_string_lossy(),
        "sqlite_path": sqlite_path_str,
        "metadata_path": metadata_path_str,
        "database_key": db_stem,
        "tables_exported": tables_exported,
        "queries_exported": queries_exported,
        "warnings": warnings,
        "launch_command": launch_command,
    }))
}
