//! Typed errors for DatabaseCraft. No panics, no unwraps: every failure is a value.
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum DatabaseCraftError {
    /// Input validation failed (hostile input fence).
    InvalidInput(String),
    /// Underlying storage or SQLite error.
    Storage(String),
    /// Schema validation error (e.g. table not found or column missing).
    Schema(String),
    /// Export generation error.
    Export(String),
    /// Execution timeout.
    Timeout(String),
}

impl fmt::Display for DatabaseCraftError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DatabaseCraftError::InvalidInput(msg) => write!(f, "invalid input: {msg}"),
            DatabaseCraftError::Storage(msg) => write!(f, "storage error: {msg}"),
            DatabaseCraftError::Schema(msg) => write!(f, "schema error: {msg}"),
            DatabaseCraftError::Export(msg) => write!(f, "export error: {msg}"),
            DatabaseCraftError::Timeout(msg) => write!(f, "operation timed out: {msg}"),
        }
    }
}

impl std::error::Error for DatabaseCraftError {}

impl From<databasecraft_store::StoreError> for DatabaseCraftError {
    fn from(err: databasecraft_store::StoreError) -> Self {
        DatabaseCraftError::Storage(err.to_string())
    }
}

/// Hostile input fence for table and identifier names.
pub fn validate_identifier(name: &str) -> Result<&str, DatabaseCraftError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(DatabaseCraftError::InvalidInput(
            "identifier cannot be empty".to_string(),
        ));
    }
    if trimmed.len() > 64 {
        return Err(DatabaseCraftError::InvalidInput(
            "identifier exceeds maximum length of 64 characters".to_string(),
        ));
    }
    if !trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(DatabaseCraftError::InvalidInput(format!(
            "identifier '{trimmed}' contains illegal characters (alphanumeric and _ only)"
        )));
    }
    Ok(trimmed)
}
