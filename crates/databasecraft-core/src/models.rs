//! Relational models, schema definitions, and cross-craft export targets.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DataType {
    Integer,
    Real,
    Text,
    Blob,
    Boolean,
    DateTime,
}

impl DataType {
    pub fn to_sqlite_type(&self) -> &'static str {
        match self {
            DataType::Integer => "INTEGER",
            DataType::Real => "REAL",
            DataType::Text => "TEXT",
            DataType::Blob => "BLOB",
            DataType::Boolean => "INTEGER",
            DataType::DateTime => "TEXT",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ForeignKeyRef {
    pub table: String,
    pub column: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ColumnDef {
    pub name: String,
    pub data_type: DataType,
    #[serde(default)]
    pub nullable: bool,
    #[serde(default)]
    pub primary_key: bool,
    #[serde(default)]
    pub foreign_key: Option<ForeignKeyRef>,
    #[serde(default)]
    pub default_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TableDef {
    pub name: String,
    pub columns: Vec<ColumnDef>,
}

impl TableDef {
    pub fn to_create_sql(&self) -> String {
        let mut parts = Vec::new();
        for col in &self.columns {
            let mut s = format!("\"{}\" {}", col.name, col.data_type.to_sqlite_type());
            if col.primary_key {
                s.push_str(" PRIMARY KEY");
            }
            if !col.nullable && !col.primary_key {
                s.push_str(" NOT NULL");
            }
            if let Some(ref dflt) = col.default_value {
                s.push_str(&format!(" DEFAULT {dflt}"));
            }
            if let Some(ref fk) = col.foreign_key {
                s.push_str(&format!(" REFERENCES \"{}\"(\"{}\")", fk.table, fk.column));
            }
            parts.push(s);
        }
        format!(
            "CREATE TABLE IF NOT EXISTS \"{}\" (\n  {}\n);",
            self.name,
            parts.join(",\n  ")
        )
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ExportTarget {
    DesignCraft,
    DeckCraft,
    WordCraft,
    GridCraft,
    PdfCraft,
}
