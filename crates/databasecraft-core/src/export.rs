//! Cross-Craft Export Engine: Formats relational query results directly
//! for consumption by DesignCraft, DeckCraft, WordCraft, GridCraft, and PdfCraft.
use crate::error::DatabaseCraftError;
use crate::models::ExportTarget;
use databasecraft_store::QueryResult;
use serde_json::{json, Value};

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
