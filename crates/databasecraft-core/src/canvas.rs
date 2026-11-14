//! Relational Canvas AST, Schema Adjacency Resolver, and SQLite JSON Query Compiler.
//!
//! Implements the foundational data structures and query compilation for Ultorg-grade "Perspectives":
//! - Non-1NF nested relations without Cartesian row duplication
//! - SQLite 3.38+ `json_group_array(json_object(...))` compilation
//! - Context-aware inline aggregations (SUM, COUNT, AVG, MIN, MAX)
//! - Forward (N:1) and reverse (1:N) foreign-key adjacency graph discovery
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use crate::error::{validate_identifier, DatabaseCraftError};
use databasecraft_store::DatabaseStore;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Aggregation functions supported in inline perspective rollups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AggFunc {
    Sum,
    Count,
    Avg,
    Min,
    Max,
}

impl AggFunc {
    pub fn as_sql(&self) -> &'static str {
        match self {
            AggFunc::Sum => "SUM",
            AggFunc::Count => "COUNT",
            AggFunc::Avg => "AVG",
            AggFunc::Min => "MIN",
            AggFunc::Max => "MAX",
        }
    }
}

/// Comparison operators for perspective filters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterOp {
    Eq,
    Neq,
    Gt,
    Gte,
    Lt,
    Lte,
    Like,
    IsNull,
    IsNotNull,
}

/// Filter condition applied to a canvas level.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanvasFilter {
    pub column: String,
    pub op: FilterOp,
    pub value: Value,
}

/// Sort condition applied to a canvas level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanvasSort {
    pub column: String,
    pub descending: bool,
}

/// Column projection in a canvas level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanvasColumn {
    pub name: String,
    pub alias: Option<String>,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default)]
    pub width_px: Option<u32>,
}

fn default_true() -> bool {
    true
}

impl CanvasColumn {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            alias: None,
            visible: true,
            width_px: None,
        }
    }

    pub fn with_alias(name: &str, alias: &str) -> Self {
        Self {
            name: name.to_string(),
            alias: Some(alias.to_string()),
            visible: true,
            width_px: None,
        }
    }

    pub fn with_width(mut self, width_px: u32) -> Self {
        self.width_px = Some(width_px);
        self
    }
}

/// Reorders an element in a mutable slice/vector from `from_idx` to `to_idx`.
/// If `drop_after` is true, inserts after target index; otherwise inserts before target index.
pub fn reorder_slice<T>(list: &mut Vec<T>, from_idx: usize, to_idx: usize, drop_after: bool) {
    if from_idx >= list.len() || to_idx >= list.len() || from_idx == to_idx {
        return;
    }
    let item = list.remove(from_idx);
    let mut target = if from_idx < to_idx {
        if drop_after {
            to_idx
        } else {
            to_idx.saturating_sub(1)
        }
    } else if drop_after {
        to_idx + 1
    } else {
        to_idx
    };
    if target > list.len() {
        target = list.len();
    }
    list.insert(target, item);
}

/// Inline aggregation calculating summary metrics across relations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanvasAggregation {
    /// Optional relation name (e.g. child table alias) if aggregating over child records.
    pub target_relation: Option<String>,
    /// Target column to aggregate (e.g. "price").
    pub target_col: String,
    /// Aggregate function (SUM, COUNT, etc.).
    pub func: AggFunc,
    /// Output alias name.
    pub alias: String,
}

/// Inlined foreign-key lookup (Compact Join) that projects a human-readable display column
/// from a referenced foreign table directly into the current level without sprouting a child hierarchy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanvasInlinedLookup {
    /// Local foreign-key column on this table (e.g. "customer_id").
    pub local_fk_col: String,
    /// Referenced foreign table (e.g. "customers").
    pub foreign_table: String,
    /// Target primary key column on foreign table (e.g. "id").
    pub foreign_pk_col: String,
    /// Target column to display (e.g. "company_name").
    pub display_col: String,
    /// Output alias name (e.g. "customer_name").
    pub alias: String,
    #[serde(default)]
    pub width_px: Option<u32>,
}

/// Nested child relation in a perspective (e.g. `Orders` under `Customers`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanvasNestedRelation {
    /// Visual alias name for this relation (e.g. "orders").
    pub relation_name: String,
    /// Target database table name (e.g. "orders").
    pub target_table: String,
    /// Key column in parent table (e.g. "id").
    pub parent_key_col: String,
    /// Foreign key column in target table (e.g. "customer_id").
    pub foreign_key_col: String,
    /// True if 1:N reverse relation; false if N:1 forward relation.
    #[serde(default = "default_true")]
    pub is_reverse: bool,
    /// Projected columns from child table.
    pub columns: Vec<CanvasColumn>,
    /// Inlined foreign-key lookups for this relation level.
    #[serde(default)]
    pub inlined_lookups: Vec<CanvasInlinedLookup>,
    /// Recursive nested relations (e.g. `OrderItems` under `Orders`).
    #[serde(default)]
    pub nested_relations: Vec<CanvasNestedRelation>,
    /// Aggregations computed within this relation level.
    #[serde(default)]
    pub aggregations: Vec<CanvasAggregation>,
    /// Filter predicates for this relation level.
    #[serde(default)]
    pub filters: Vec<CanvasFilter>,
    /// Optional limit on child rows.
    pub limit: Option<usize>,
}

/// Full specification of a Relational Canvas Perspective.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanvasSpec {
    /// Root table name (e.g. "categories" or "customers").
    pub root_table: String,
    /// Projected columns on root table.
    pub columns: Vec<CanvasColumn>,
    /// Root-level inlined foreign-key lookups (Compact Joins).
    #[serde(default)]
    pub inlined_lookups: Vec<CanvasInlinedLookup>,
    /// Nested child relations.
    #[serde(default)]
    pub nested_relations: Vec<CanvasNestedRelation>,
    /// Root-level inline rollups/aggregations.
    #[serde(default)]
    pub aggregations: Vec<CanvasAggregation>,
    /// Root-level filters.
    #[serde(default)]
    pub filters: Vec<CanvasFilter>,
    /// Root-level sort order.
    #[serde(default)]
    pub sort: Vec<CanvasSort>,
    /// Pagination limit.
    pub limit: Option<usize>,
    /// Pagination offset.
    pub offset: Option<usize>,
}

impl CanvasNestedRelation {
    /// Reorders child columns by moving `from_col` to `to_col`.
    pub fn reorder_columns(&mut self, from_col: &str, to_col: &str, drop_after: bool) -> bool {
        let from_idx = self.columns.iter().position(|c| {
            c.name.eq_ignore_ascii_case(from_col)
                || c.alias
                    .as_deref()
                    .unwrap_or("")
                    .eq_ignore_ascii_case(from_col)
        });
        let to_idx = self.columns.iter().position(|c| {
            c.name.eq_ignore_ascii_case(to_col)
                || c.alias
                    .as_deref()
                    .unwrap_or("")
                    .eq_ignore_ascii_case(to_col)
        });

        if let (Some(f), Some(t)) = (from_idx, to_idx) {
            reorder_slice(&mut self.columns, f, t, drop_after);
            true
        } else {
            false
        }
    }

    /// Sets the persistent pixel width of a child column.
    pub fn set_column_width(&mut self, col_name: &str, width_px: u32) -> bool {
        if let Some(col) = self.columns.iter_mut().find(|c| {
            c.name.eq_ignore_ascii_case(col_name)
                || c.alias
                    .as_deref()
                    .unwrap_or("")
                    .eq_ignore_ascii_case(col_name)
        }) {
            col.width_px = Some(width_px);
            true
        } else if let Some(lookup) = self
            .inlined_lookups
            .iter_mut()
            .find(|l| l.alias.eq_ignore_ascii_case(col_name))
        {
            lookup.width_px = Some(width_px);
            true
        } else {
            false
        }
    }
}

impl CanvasSpec {
    pub fn new(root_table: &str) -> Self {
        Self {
            root_table: root_table.to_string(),
            columns: Vec::new(),
            inlined_lookups: Vec::new(),
            nested_relations: Vec::new(),
            aggregations: Vec::new(),
            filters: Vec::new(),
            sort: Vec::new(),
            limit: None,
            offset: None,
        }
    }

    /// Reorders root projected columns by moving `from_col` to `to_col`.
    pub fn reorder_columns(&mut self, from_col: &str, to_col: &str, drop_after: bool) -> bool {
        let from_idx = self.columns.iter().position(|c| {
            c.name.eq_ignore_ascii_case(from_col)
                || c.alias
                    .as_deref()
                    .unwrap_or("")
                    .eq_ignore_ascii_case(from_col)
        });
        let to_idx = self.columns.iter().position(|c| {
            c.name.eq_ignore_ascii_case(to_col)
                || c.alias
                    .as_deref()
                    .unwrap_or("")
                    .eq_ignore_ascii_case(to_col)
        });

        if let (Some(f), Some(t)) = (from_idx, to_idx) {
            reorder_slice(&mut self.columns, f, t, drop_after);
            true
        } else {
            false
        }
    }

    /// Reorders inlined lookups (Compact Joins) by alias.
    pub fn reorder_inlined_lookups(
        &mut self,
        from_alias: &str,
        to_alias: &str,
        drop_after: bool,
    ) -> bool {
        let from_idx = self
            .inlined_lookups
            .iter()
            .position(|l| l.alias.eq_ignore_ascii_case(from_alias));
        let to_idx = self
            .inlined_lookups
            .iter()
            .position(|l| l.alias.eq_ignore_ascii_case(to_alias));
        if let (Some(f), Some(t)) = (from_idx, to_idx) {
            reorder_slice(&mut self.inlined_lookups, f, t, drop_after);
            true
        } else {
            false
        }
    }

    /// Sets the persistent pixel width of a specified column or inlined lookup.
    pub fn set_column_width(&mut self, col_name: &str, width_px: u32) -> bool {
        if let Some(col) = self.columns.iter_mut().find(|c| {
            c.name.eq_ignore_ascii_case(col_name)
                || c.alias
                    .as_deref()
                    .unwrap_or("")
                    .eq_ignore_ascii_case(col_name)
        }) {
            col.width_px = Some(width_px);
            true
        } else if let Some(lookup) = self
            .inlined_lookups
            .iter_mut()
            .find(|l| l.alias.eq_ignore_ascii_case(col_name))
        {
            lookup.width_px = Some(width_px);
            true
        } else {
            false
        }
    }
}

/// Discovered relational link (Forward or Reverse) for Field Selector sprouting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaRelation {
    pub relation_name: String,
    pub table: String,
    pub source_col: String,
    pub target_col: String,
    pub is_reverse: bool,
    pub label: String,
}

/// Summary of all forward and reverse relations connected to a table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableRelations {
    pub table: String,
    pub forward_relations: Vec<SchemaRelation>,
    pub reverse_relations: Vec<SchemaRelation>,
}

/// Inspect all forward (N:1) and reverse (1:N) foreign-key links connected to `table_name`.
pub fn resolve_table_relations(
    store: &DatabaseStore,
    table_name: &str,
) -> Result<TableRelations, DatabaseCraftError> {
    validate_identifier(table_name)?;
    let full = store.inspect_all()?;

    let current_table = full
        .tables
        .iter()
        .find(|t| t.name.eq_ignore_ascii_case(table_name))
        .ok_or_else(|| DatabaseCraftError::Schema(format!("table '{table_name}' not found")))?;

    let mut forward_relations = Vec::new();
    let mut reverse_relations = Vec::new();

    // 1. Forward relations: Foreign keys declared on current table pointing to another table
    for fk in &current_table.foreign_keys {
        validate_identifier(&fk.table)?;
        validate_identifier(&fk.from)?;
        validate_identifier(&fk.to)?;
        forward_relations.push(SchemaRelation {
            relation_name: fk.table.clone(),
            table: fk.table.clone(),
            source_col: fk.from.clone(),
            target_col: fk.to.clone(),
            is_reverse: false,
            label: format!("{} (N:1 via {})", fk.table, fk.from),
        });
    }

    // 2. Reverse relations: Foreign keys declared on OTHER tables pointing to current table
    for other in &full.tables {
        for fk in &other.foreign_keys {
            if fk.table.eq_ignore_ascii_case(table_name) {
                validate_identifier(&other.name)?;
                validate_identifier(&fk.from)?;
                validate_identifier(&fk.to)?;
                reverse_relations.push(SchemaRelation {
                    relation_name: other.name.clone(),
                    table: other.name.clone(),
                    source_col: fk.to.clone(),
                    target_col: fk.from.clone(),
                    is_reverse: true,
                    label: format!("{} (1:N via {})", other.name, fk.from),
                });
            }
        }
    }

    Ok(TableRelations {
        table: table_name.to_string(),
        forward_relations,
        reverse_relations,
    })
}

/// Helper to sanitize and quote SQL identifiers.
fn quote_ident(name: &str) -> Result<String, DatabaseCraftError> {
    let validated = validate_identifier(name)?;
    Ok(format!("\"{validated}\""))
}

/// Helper to format literal filter values safely.
fn format_filter_value(val: &Value) -> Result<String, DatabaseCraftError> {
    match val {
        Value::Null => Ok("NULL".to_string()),
        Value::Bool(b) => Ok(if *b { "1".to_string() } else { "0".to_string() }),
        Value::Number(n) => Ok(n.to_string()),
        Value::String(s) => {
            // Escape single quotes for SQL string literals
            let escaped = s.replace('\'', "''");
            Ok(format!("'{escaped}'"))
        }
        _ => Err(DatabaseCraftError::InvalidInput(
            "complex arrays/objects not supported as direct filter literals".to_string(),
        )),
    }
}

/// Compiler that transforms a `CanvasSpec` AST into an executable SQLite JSON SQL query.
pub struct CanvasCompiler;

impl CanvasCompiler {
    /// Validate all identifiers and constraints in the canvas specification.
    pub fn validate_spec(spec: &CanvasSpec) -> Result<(), DatabaseCraftError> {
        validate_identifier(&spec.root_table)?;
        for col in &spec.columns {
            validate_identifier(&col.name)?;
            if let Some(ref alias) = col.alias {
                validate_identifier(alias)?;
            }
        }
        for agg in &spec.aggregations {
            validate_identifier(&agg.target_col)?;
            validate_identifier(&agg.alias)?;
            if let Some(ref rel) = agg.target_relation {
                validate_identifier(rel)?;
            }
        }
        for filter in &spec.filters {
            validate_identifier(&filter.column)?;
        }
        for sort in &spec.sort {
            validate_identifier(&sort.column)?;
        }
        for lookup in &spec.inlined_lookups {
            Self::validate_inlined_lookup(lookup)?;
        }
        for rel in &spec.nested_relations {
            Self::validate_nested_relation(rel)?;
        }
        Ok(())
    }

    fn validate_nested_relation(rel: &CanvasNestedRelation) -> Result<(), DatabaseCraftError> {
        validate_identifier(&rel.relation_name)?;
        validate_identifier(&rel.target_table)?;
        validate_identifier(&rel.parent_key_col)?;
        validate_identifier(&rel.foreign_key_col)?;
        for col in &rel.columns {
            validate_identifier(&col.name)?;
            if let Some(ref alias) = col.alias {
                validate_identifier(alias)?;
            }
        }
        for lookup in &rel.inlined_lookups {
            Self::validate_inlined_lookup(lookup)?;
        }
        for agg in &rel.aggregations {
            validate_identifier(&agg.target_col)?;
            validate_identifier(&agg.alias)?;
            if let Some(ref r) = agg.target_relation {
                validate_identifier(r)?;
            }
        }
        for filter in &rel.filters {
            validate_identifier(&filter.column)?;
        }
        for child in &rel.nested_relations {
            Self::validate_nested_relation(child)?;
        }
        Ok(())
    }

    fn validate_inlined_lookup(lookup: &CanvasInlinedLookup) -> Result<(), DatabaseCraftError> {
        validate_identifier(&lookup.local_fk_col)?;
        validate_identifier(&lookup.foreign_table)?;
        validate_identifier(&lookup.foreign_pk_col)?;
        validate_identifier(&lookup.display_col)?;
        validate_identifier(&lookup.alias)?;
        Ok(())
    }

    /// Compile a `CanvasSpec` AST into an ANSI/SQLite 3.38+ SQL query.
    pub fn compile_sql(spec: &CanvasSpec) -> Result<String, DatabaseCraftError> {
        Self::validate_spec(spec)?;

        let root_alias = "root";
        let mut select_clauses = Vec::new();

        // 1. Primitive columns
        for col in &spec.columns {
            if !col.visible {
                continue;
            }
            let quoted_col = quote_ident(&col.name)?;
            let alias = col.alias.as_deref().unwrap_or(&col.name);
            let quoted_alias = quote_ident(alias)?;
            select_clauses.push(format!("{root_alias}.{quoted_col} AS {quoted_alias}"));
        }

        // 1b. Inlined foreign-key lookups (Compact Joins)
        for (idx, lookup) in spec.inlined_lookups.iter().enumerate() {
            let lookup_sql = Self::compile_inlined_lookup(lookup, root_alias, idx)?;
            select_clauses.push(lookup_sql);
        }

        // If no columns were explicitly chosen, default to selecting all columns via root.*
        if select_clauses.is_empty() && spec.nested_relations.is_empty() {
            select_clauses.push(format!("{root_alias}.*"));
        }

        // 2. Root-level inline aggregations
        for agg in &spec.aggregations {
            let agg_sql = Self::compile_aggregation(agg, root_alias, &spec.nested_relations)?;
            select_clauses.push(agg_sql);
        }

        // 3. Nested relations via SQLite json_group_array(json_object(...))
        for (idx, rel) in spec.nested_relations.iter().enumerate() {
            let child_alias = format!("sub_{idx}");
            let rel_sql = Self::compile_nested_relation(rel, root_alias, &child_alias)?;
            select_clauses.push(rel_sql);
        }

        let quoted_root_table = quote_ident(&spec.root_table)?;
        let mut sql = format!(
            "SELECT\n  {}\nFROM {} AS {}",
            select_clauses.join(",\n  "),
            quoted_root_table,
            root_alias
        );

        // 4. Filters
        if !spec.filters.is_empty() {
            let mut filter_clauses = Vec::new();
            for f in &spec.filters {
                let quoted_col = quote_ident(&f.column)?;
                let clause = match f.op {
                    FilterOp::Eq => format!(
                        "{root_alias}.{quoted_col} = {}",
                        format_filter_value(&f.value)?
                    ),
                    FilterOp::Neq => format!(
                        "{root_alias}.{quoted_col} != {}",
                        format_filter_value(&f.value)?
                    ),
                    FilterOp::Gt => format!(
                        "{root_alias}.{quoted_col} > {}",
                        format_filter_value(&f.value)?
                    ),
                    FilterOp::Gte => format!(
                        "{root_alias}.{quoted_col} >= {}",
                        format_filter_value(&f.value)?
                    ),
                    FilterOp::Lt => format!(
                        "{root_alias}.{quoted_col} < {}",
                        format_filter_value(&f.value)?
                    ),
                    FilterOp::Lte => format!(
                        "{root_alias}.{quoted_col} <= {}",
                        format_filter_value(&f.value)?
                    ),
                    FilterOp::Like => format!(
                        "{root_alias}.{quoted_col} LIKE {}",
                        format_filter_value(&f.value)?
                    ),
                    FilterOp::IsNull => format!("{root_alias}.{quoted_col} IS NULL"),
                    FilterOp::IsNotNull => format!("{root_alias}.{quoted_col} IS NOT NULL"),
                };
                filter_clauses.push(clause);
            }
            sql.push_str(&format!("\nWHERE {}", filter_clauses.join(" AND ")));
        }

        // 5. Sort
        if !spec.sort.is_empty() {
            let mut sort_clauses = Vec::new();
            for s in &spec.sort {
                let quoted_col = quote_ident(&s.column)?;
                let dir = if s.descending { "DESC" } else { "ASC" };
                sort_clauses.push(format!("{root_alias}.{quoted_col} {dir}"));
            }
            sql.push_str(&format!("\nORDER BY {}", sort_clauses.join(", ")));
        }

        // 6. Pagination
        if let Some(limit) = spec.limit {
            sql.push_str(&format!("\nLIMIT {limit}"));
            if let Some(offset) = spec.offset {
                sql.push_str(&format!(" OFFSET {offset}"));
            }
        }

        sql.push(';');
        Ok(sql)
    }

    fn compile_aggregation(
        agg: &CanvasAggregation,
        parent_alias: &str,
        available_relations: &[CanvasNestedRelation],
    ) -> Result<String, DatabaseCraftError> {
        let quoted_alias = quote_ident(&agg.alias)?;
        let agg_func = agg.func.as_sql();

        if let Some(ref rel_name) = agg.target_relation {
            let rel = available_relations
                .iter()
                .find(|r| r.relation_name.eq_ignore_ascii_case(rel_name))
                .ok_or_else(|| {
                    DatabaseCraftError::InvalidInput(format!(
                        "aggregation references undefined relation '{rel_name}'"
                    ))
                })?;

            let quoted_target_table = quote_ident(&rel.target_table)?;
            let quoted_target_col = quote_ident(&agg.target_col)?;
            let quoted_parent_key = quote_ident(&rel.parent_key_col)?;
            let quoted_foreign_key = quote_ident(&rel.foreign_key_col)?;
            let agg_alias = "agg_child";

            let expr = if agg.func == AggFunc::Count {
                format!("{agg_func}(*)")
            } else {
                format!("COALESCE({agg_func}({agg_alias}.{quoted_target_col}), 0)")
            };

            Ok(format!(
                "(SELECT {expr} FROM {quoted_target_table} AS {agg_alias} WHERE {agg_alias}.{quoted_foreign_key} = {parent_alias}.{quoted_parent_key}) AS {quoted_alias}"
            ))
        } else {
            // Table-level scalar or self-column aggregate
            let quoted_col = quote_ident(&agg.target_col)?;
            Ok(format!(
                "{agg_func}({parent_alias}.{quoted_col}) AS {quoted_alias}"
            ))
        }
    }

    fn compile_inlined_lookup(
        lookup: &CanvasInlinedLookup,
        parent_alias: &str,
        idx: usize,
    ) -> Result<String, DatabaseCraftError> {
        Self::validate_inlined_lookup(lookup)?;
        let quoted_table = quote_ident(&lookup.foreign_table)?;
        let quoted_display = quote_ident(&lookup.display_col)?;
        let quoted_foreign_pk = quote_ident(&lookup.foreign_pk_col)?;
        let quoted_local_fk = quote_ident(&lookup.local_fk_col)?;
        let quoted_alias = quote_ident(&lookup.alias)?;
        let lookup_alias = format!("inline_{idx}");

        Ok(format!(
            "(SELECT {lookup_alias}.{quoted_display} FROM {quoted_table} AS {lookup_alias} WHERE {lookup_alias}.{quoted_foreign_pk} = {parent_alias}.{quoted_local_fk}) AS {quoted_alias}"
        ))
    }

    fn compile_nested_relation(
        rel: &CanvasNestedRelation,
        parent_alias: &str,
        child_alias: &str,
    ) -> Result<String, DatabaseCraftError> {
        let quoted_target_table = quote_ident(&rel.target_table)?;
        let quoted_parent_key = quote_ident(&rel.parent_key_col)?;
        let quoted_foreign_key = quote_ident(&rel.foreign_key_col)?;
        let quoted_rel_name = quote_ident(&rel.relation_name)?;

        let mut json_obj_pairs = Vec::new();

        // 1. Columns
        for col in &rel.columns {
            if !col.visible {
                continue;
            }
            let key = col.alias.as_deref().unwrap_or(&col.name);
            let quoted_col = quote_ident(&col.name)?;
            json_obj_pairs.push(format!("'{key}', {child_alias}.{quoted_col}"));
        }

        // 1b. Inlined lookups for child records
        for (l_idx, lookup) in rel.inlined_lookups.iter().enumerate() {
            let quoted_table = quote_ident(&lookup.foreign_table)?;
            let quoted_display = quote_ident(&lookup.display_col)?;
            let quoted_foreign_pk = quote_ident(&lookup.foreign_pk_col)?;
            let quoted_local_fk = quote_ident(&lookup.local_fk_col)?;
            let l_alias = format!("{child_alias}_inl_{l_idx}");
            let subquery = format!(
                "(SELECT {l_alias}.{quoted_display} FROM {quoted_table} AS {l_alias} WHERE {l_alias}.{quoted_foreign_pk} = {child_alias}.{quoted_local_fk})"
            );
            json_obj_pairs.push(format!("'{}', {subquery}", lookup.alias));
        }

        // 2. Child aggregations
        for agg in &rel.aggregations {
            let _quoted_alias = quote_ident(&agg.alias)?;
            let agg_func = agg.func.as_sql();
            let quoted_target_col = quote_ident(&agg.target_col)?;
            json_obj_pairs.push(format!(
                "'{}', {agg_func}({child_alias}.{quoted_target_col})",
                agg.alias
            ));
        }

        // 3. Child recursive nested relations
        for (idx, sub) in rel.nested_relations.iter().enumerate() {
            let sub_alias = format!("{child_alias}_{idx}");
            let sub_sql = Self::compile_nested_relation_json(sub, child_alias, &sub_alias)?;
            json_obj_pairs.push(format!("'{}', json(({sub_sql}))", sub.relation_name));
        }

        // If no columns are projected in the subquery, include the foreign key as fallback
        if json_obj_pairs.is_empty() {
            json_obj_pairs.push(format!(
                "'{}', {child_alias}.{quoted_foreign_key}",
                rel.foreign_key_col
            ));
        }

        let mut where_clauses = vec![format!(
            "{child_alias}.{quoted_foreign_key} = {parent_alias}.{quoted_parent_key}"
        )];

        for f in &rel.filters {
            let quoted_col = quote_ident(&f.column)?;
            let clause = match f.op {
                FilterOp::Eq => format!(
                    "{child_alias}.{quoted_col} = {}",
                    format_filter_value(&f.value)?
                ),
                FilterOp::Neq => format!(
                    "{child_alias}.{quoted_col} != {}",
                    format_filter_value(&f.value)?
                ),
                FilterOp::Gt => format!(
                    "{child_alias}.{quoted_col} > {}",
                    format_filter_value(&f.value)?
                ),
                FilterOp::Gte => format!(
                    "{child_alias}.{quoted_col} >= {}",
                    format_filter_value(&f.value)?
                ),
                FilterOp::Lt => format!(
                    "{child_alias}.{quoted_col} < {}",
                    format_filter_value(&f.value)?
                ),
                FilterOp::Lte => format!(
                    "{child_alias}.{quoted_col} <= {}",
                    format_filter_value(&f.value)?
                ),
                FilterOp::Like => format!(
                    "{child_alias}.{quoted_col} LIKE {}",
                    format_filter_value(&f.value)?
                ),
                FilterOp::IsNull => format!("{child_alias}.{quoted_col} IS NULL"),
                FilterOp::IsNotNull => format!("{child_alias}.{quoted_col} IS NOT NULL"),
            };
            where_clauses.push(clause);
        }

        let limit_clause = if let Some(lim) = rel.limit {
            format!(" LIMIT {lim}")
        } else {
            String::new()
        };

        // Note: in SQLite, json_group_array over zero rows evaluates to json('[]')
        // Using COALESCE(..., json('[]')) guarantees an empty array rather than null or error.
        let subquery = format!(
            "(SELECT COALESCE(json_group_array(json_object({})), json('[]')) FROM {} AS {} WHERE {}{}) AS {}",
            json_obj_pairs.join(", "),
            quoted_target_table,
            child_alias,
            where_clauses.join(" AND "),
            limit_clause,
            quoted_rel_name
        );

        Ok(subquery)
    }

    fn compile_nested_relation_json(
        rel: &CanvasNestedRelation,
        parent_alias: &str,
        child_alias: &str,
    ) -> Result<String, DatabaseCraftError> {
        let quoted_target_table = quote_ident(&rel.target_table)?;
        let quoted_parent_key = quote_ident(&rel.parent_key_col)?;
        let quoted_foreign_key = quote_ident(&rel.foreign_key_col)?;

        let mut json_obj_pairs = Vec::new();
        for col in &rel.columns {
            if !col.visible {
                continue;
            }
            let key = col.alias.as_deref().unwrap_or(&col.name);
            let quoted_col = quote_ident(&col.name)?;
            json_obj_pairs.push(format!("'{key}', {child_alias}.{quoted_col}"));
        }

        // Inlined lookups for recursive JSON object
        for (l_idx, lookup) in rel.inlined_lookups.iter().enumerate() {
            let quoted_table = quote_ident(&lookup.foreign_table)?;
            let quoted_display = quote_ident(&lookup.display_col)?;
            let quoted_foreign_pk = quote_ident(&lookup.foreign_pk_col)?;
            let quoted_local_fk = quote_ident(&lookup.local_fk_col)?;
            let l_alias = format!("{child_alias}_inl_{l_idx}");
            let subquery = format!(
                "(SELECT {l_alias}.{quoted_display} FROM {quoted_table} AS {l_alias} WHERE {l_alias}.{quoted_foreign_pk} = {child_alias}.{quoted_local_fk})"
            );
            json_obj_pairs.push(format!("'{}', {subquery}", lookup.alias));
        }

        for (idx, sub) in rel.nested_relations.iter().enumerate() {
            let sub_alias = format!("{child_alias}_{idx}");
            let sub_sql = Self::compile_nested_relation_json(sub, child_alias, &sub_alias)?;
            json_obj_pairs.push(format!("'{}', json(({sub_sql}))", sub.relation_name));
        }

        if json_obj_pairs.is_empty() {
            json_obj_pairs.push(format!(
                "'{}', {child_alias}.{quoted_foreign_key}",
                rel.foreign_key_col
            ));
        }

        let where_clause =
            format!("{child_alias}.{quoted_foreign_key} = {parent_alias}.{quoted_parent_key}");

        Ok(format!(
            "SELECT COALESCE(json_group_array(json_object({})), json('[]')) FROM {} AS {} WHERE {}",
            json_obj_pairs.join(", "),
            quoted_target_table,
            child_alias,
            where_clause
        ))
    }

    /// Execute a compiled canvas query and parse nested JSON payloads into a rich hierarchical result.
    pub fn execute(store: &DatabaseStore, spec: &CanvasSpec) -> Result<Value, DatabaseCraftError> {
        let sql = Self::compile_sql(spec)?;
        let raw_result = store.query(&sql)?;

        let mut structured_rows = Vec::new();
        for row in &raw_result.rows {
            let mut obj_map = serde_json::Map::new();
            for (idx, col_name) in raw_result.columns.iter().enumerate() {
                let cell_val = &row[idx];
                // Check if cell is a string that represents a nested JSON array or object
                if let Value::String(s) = cell_val {
                    let trimmed = s.trim();
                    if (trimmed.starts_with('[') && trimmed.ends_with(']'))
                        || (trimmed.starts_with('{') && trimmed.ends_with('}'))
                    {
                        if let Ok(parsed_json) = serde_json::from_str::<Value>(trimmed) {
                            obj_map.insert(col_name.clone(), parsed_json);
                            continue;
                        }
                    }
                }
                obj_map.insert(col_name.clone(), cell_val.clone());
            }
            structured_rows.push(Value::Object(obj_map));
        }

        Ok(json!({
            "root_table": spec.root_table,
            "columns": raw_result.columns,
            "rows_count": structured_rows.len(),
            "data": structured_rows,
            "compiled_sql": sql,
        }))
    }
}

/// Heuristic suggesting a human-readable display column for a table (e.g. for inlining foreign keys).
pub fn suggest_display_column(
    store: &DatabaseStore,
    table_name: &str,
) -> Result<String, DatabaseCraftError> {
    validate_identifier(table_name)?;
    let schema = store
        .table_schema(table_name)
        .map_err(|e| DatabaseCraftError::Schema(e.to_string()))?;

    let candidates = [
        "company_name",
        "name",
        "title",
        "label",
        "description",
        "code",
        "username",
        "email",
    ];

    for candidate in candidates {
        if let Some(col) = schema
            .columns
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(candidate))
        {
            return Ok(col.name.clone());
        }
    }

    // Fallback 1: First non-PK TEXT column
    if let Some(col) = schema
        .columns
        .iter()
        .find(|c| !c.pk && c.data_type.to_uppercase().contains("TEXT"))
    {
        return Ok(col.name.clone());
    }

    // Fallback 2: First non-PK column
    if let Some(col) = schema.columns.iter().find(|c| !c.pk) {
        return Ok(col.name.clone());
    }

    // Fallback 3: First column or "id"
    Ok(schema
        .columns
        .first()
        .map(|c| c.name.clone())
        .unwrap_or_else(|| "id".to_string()))
}
