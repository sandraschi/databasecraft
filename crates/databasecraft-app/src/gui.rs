//! L3 DatabaseCraft Native Desktop GUI.
//!
//! Powered by egui / eframe immediate-mode rendering (60-144 FPS).
//! Calls `databasecraft_core::dispatch` directly in-process with zero WebView2.
//! Strict ArtCraft zero-panic containment: no unwraps, no expects, no panics.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use databasecraft_core::canvas::{
    suggest_display_column, AggFunc, CanvasAggregation, CanvasColumn, CanvasInlinedLookup,
    CanvasNestedRelation, CanvasSort, CanvasSpec, TableRelations,
};
use databasecraft_core::notifications::{CraftNotification, NotificationSeverity};
use databasecraft_core::{dispatch, models::ExportTarget, Command, APP_VERSION, CONTROL_PORT};
use databasecraft_store::DatabaseStore;
use eframe::egui::{self, Color32, Vec2};
use serde_json::Value;
use std::sync::Arc;
use std::time::Instant;
use tokio::runtime::Runtime;

#[derive(Debug, Clone, PartialEq)]
pub struct ActiveCellEdit {
    pub table: String,
    pub pk_col: String,
    pub pk_val: Value,
    pub col: String,
    pub initial_str: String,
    pub edit_str: String,
}

#[derive(Debug, Clone)]
pub struct CellMutation {
    pub table: String,
    pub pk_col: String,
    pub pk_val: Value,
    pub col: String,
    pub old_val: Value,
    pub new_val: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    Tables,
    Canvas,
    Query,
    Schema,
    AiWorkflows,
    Rag,
    Profiler,
    Doctor,
    Constructor,
    Export,
    Alerts,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiSubTab {
    Nlq,
    Dedupe,
    Synthetic,
    Advisor,
    Ingest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasDensity {
    Compact,
    Standard,
    Relaxed,
}

impl CanvasDensity {
    pub fn row_height(&self) -> f32 {
        match self {
            CanvasDensity::Compact => 22.0,
            CanvasDensity::Standard => 28.0,
            CanvasDensity::Relaxed => 34.0,
        }
    }

    pub fn font_size(&self) -> f32 {
        match self {
            CanvasDensity::Compact => 11.5,
            CanvasDensity::Standard => 13.0,
            CanvasDensity::Relaxed => 14.5,
        }
    }

    pub fn cell_padding(&self) -> (i8, i8) {
        match self {
            CanvasDensity::Compact => (8, 3),
            CanvasDensity::Standard => (12, 5),
            CanvasDensity::Relaxed => (16, 8),
        }
    }
}

fn format_currency(val: f64) -> String {
    let sign = if val < 0.0 { "-" } else { "" };
    let abs = val.abs();
    let int_part = abs.trunc() as u64;
    let frac_part = (abs.fract() * 100.0).round() as u64;

    // Add thousands commas
    let s = int_part.to_string();
    let mut formatted_int = String::new();
    let len = s.len();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (len - i).is_multiple_of(3) {
            formatted_int.push(',');
        }
        formatted_int.push(ch);
    }
    format!("{sign}${formatted_int}.{frac_part:02}")
}

fn render_status_pill(
    ui: &mut egui::Ui,
    text: &str,
    bg: Color32,
    border: Color32,
    text_color: Color32,
    font_size: f32,
) {
    egui::Frame::new()
        .fill(bg)
        .stroke(egui::Stroke::new(1.0, border))
        .corner_radius(3)
        .inner_margin(egui::Margin::symmetric(6, 2))
        .show(ui, |ui| {
            let font_id = egui::FontId::new(font_size - 1.0, egui::FontFamily::Proportional);
            ui.label(
                egui::RichText::new(text.to_uppercase())
                    .font(font_id)
                    .color(text_color)
                    .strong(),
            );
        });
}

fn render_formatted_cell(
    ui: &mut egui::Ui,
    col_name: &str,
    val: &Value,
    density: CanvasDensity,
) {
    let font_size = density.font_size();
    if val.is_null() {
        ui.colored_label(Color32::from_rgb(107, 114, 128), "—");
        return;
    }

    if let Some(s) = val.as_str() {
        let s_lower = s.to_lowercase();
        if s_lower == "shipped"
            || s_lower == "delivered"
            || s_lower == "print"
            || s_lower == "active"
            || s_lower == "completed"
        {
            render_status_pill(
                ui,
                s,
                Color32::from_rgba_unmultiplied(16, 185, 129, 45),
                Color32::from_rgb(52, 211, 153),
                Color32::from_rgb(167, 243, 208),
                font_size,
            );
        } else if s_lower == "processing" || s_lower == "hold" || s_lower == "in_progress" {
            render_status_pill(
                ui,
                s,
                Color32::from_rgba_unmultiplied(14, 165, 233, 45),
                Color32::from_rgb(56, 189, 248),
                Color32::from_rgb(186, 230, 253),
                font_size,
            );
        } else if s_lower == "pending" || s_lower == "review" {
            render_status_pill(
                ui,
                s,
                Color32::from_rgba_unmultiplied(245, 158, 11, 45),
                Color32::from_rgb(251, 191, 36),
                Color32::from_rgb(254, 243, 199),
                font_size,
            );
        } else if s_lower == "ng"
            || s_lower == "cancelled"
            || s_lower == "failed"
            || s_lower == "inactive"
        {
            render_status_pill(
                ui,
                s,
                Color32::from_rgba_unmultiplied(239, 68, 68, 45),
                Color32::from_rgb(248, 113, 113),
                Color32::from_rgb(254, 202, 202),
                font_size,
            );
        } else if col_name.contains("date") || col_name.contains("time") {
            let font_id = egui::FontId::new(font_size - 1.0, egui::FontFamily::Monospace);
            ui.label(
                egui::RichText::new(s)
                    .font(font_id)
                    .color(Color32::from_rgb(203, 213, 225)),
            );
        } else {
            ui.label(
                egui::RichText::new(s)
                    .size(font_size)
                    .color(Color32::from_rgb(241, 245, 249))
                    .strong(),
            );
        }
    } else if let Some(n) = val.as_f64() {
        let col_lower = col_name.to_lowercase();
        if col_lower.contains("price")
            || col_lower.contains("freight")
            || col_lower.contains("cost")
            || col_lower.contains("total")
            || col_lower.contains("usd")
            || col_lower.contains("arr")
            || col_lower.contains("mrr")
            || col_lower.contains("value")
        {
            let curr = format_currency(n);
            let font_id = egui::FontId::new(font_size, egui::FontFamily::Monospace);
            ui.label(
                egui::RichText::new(curr)
                    .font(font_id)
                    .color(Color32::from_rgb(52, 211, 153))
                    .strong(),
            );
        } else if col_lower.contains("pct")
            || col_lower.contains("discount")
            || col_lower.contains("margin")
            || col_lower.contains("churn")
        {
            let pct_val = if n <= 1.0 && n > 0.0 && col_lower.contains("discount") {
                n * 100.0
            } else {
                n
            };
            let font_id = egui::FontId::new(font_size, egui::FontFamily::Monospace);
            ui.label(
                egui::RichText::new(format!("{:.1}%", pct_val))
                    .font(font_id)
                    .color(Color32::from_rgb(56, 189, 248)),
            );
        } else if col_lower.contains("quantity")
            || col_lower.contains("inventory")
            || col_lower.contains("count")
            || col_lower.contains("seats")
        {
            let font_id = egui::FontId::new(font_size, egui::FontFamily::Monospace);
            ui.label(
                egui::RichText::new(format!("{}", n as i64))
                    .font(font_id)
                    .color(Color32::from_rgb(186, 230, 253))
                    .strong(),
            );
        } else {
            let font_id = egui::FontId::new(font_size, egui::FontFamily::Monospace);
            ui.label(
                egui::RichText::new(format!("{n}"))
                    .font(font_id)
                    .color(Color32::from_rgb(226, 232, 240)),
            );
        }
    } else if let Some(b) = val.as_bool() {
        let text = if b { "TRUE" } else { "FALSE" };
        let color = if b {
            Color32::from_rgb(52, 211, 153)
        } else {
            Color32::from_rgb(148, 163, 184)
        };
        ui.label(egui::RichText::new(text).size(font_size).color(color));
    } else {
        ui.label(egui::RichText::new(val.to_string()).size(font_size));
    }
}

pub struct DatabaseCraftGui {
    pub store: Arc<DatabaseStore>,
    pub runtime: Arc<Runtime>,
    pub active_tab: ActiveTab,
    pub ai_subtab: AiSubTab,
    pub tables: Vec<String>,
    pub selected_table: Option<String>,
    pub table_search: String,
    pub table_rows: Option<Value>,
    pub filter_text: String,
    pub sql_query: String,
    pub query_result: Option<Value>,
    pub query_latency_ms: Option<u128>,
    pub nlq_prompt: String,
    pub nlq_result: Option<Value>,
    pub dedupe_column: String,
    pub dedupe_threshold: f32,
    pub dedupe_result: Option<Value>,
    pub synthetic_count: usize,
    pub synthetic_insert: bool,
    pub synthetic_result: Option<Value>,
    pub advisor_result: Option<Value>,
    pub ingest_title: String,
    pub ingest_content: String,
    pub ingest_tags: String,
    pub ingest_result: Option<Value>,
    pub profiler_result: Option<Value>,
    pub diagnostic_report: Option<Value>,
    pub repair_report: Option<Value>,
    pub salvage_path: String,
    pub constructor_blueprint: String,
    pub constructor_seed: bool,
    pub constructor_search: bool,
    pub constructor_result: Option<Value>,
    pub constructor_blueprints_list: Option<Value>,
    pub rag_text: String,
    pub rag_results: Option<Value>,
    pub export_target: ExportTarget,
    pub export_preview: Option<Value>,
    pub status_message: String,
    // Relational Canvas (Ultorg-Style) perspective state
    pub canvas_root_table: Option<String>,
    pub canvas_spec: Option<CanvasSpec>,
    pub canvas_result: Option<Value>,
    pub canvas_relations: Option<TableRelations>,
    pub canvas_latency_ms: Option<u128>,
    pub canvas_field_selector_open: bool,
    pub canvas_show_sql_drawer: bool,
    pub canvas_rollup_col: String,
    pub canvas_rollup_func: AggFunc,
    pub canvas_rollup_alias: String,
    pub canvas_density: CanvasDensity,
    // CTO Operations Watchdog and Notification Bus state
    pub notifications: Vec<CraftNotification>,
    pub unread_alerts_count: usize,
    pub watchdog_last_check: Option<Value>,
    pub alert_filter_severity: Option<NotificationSeverity>,
    pub alert_filter_unack_only: bool,
    // In-place cell editing & undo stack
    pub active_cell_edit: Option<ActiveCellEdit>,
    pub cell_undo_stack: Vec<CellMutation>,
    pub cell_redo_stack: Vec<CellMutation>,
}

impl DatabaseCraftGui {
    pub fn new(store: Arc<DatabaseStore>, runtime: Arc<Runtime>) -> Self {
        let initial_status = runtime.block_on(dispatch(Command::Status, &store));
        let mut tables = Vec::new();
        if let Some(arr) = initial_status["tables"].as_array() {
            for t in arr {
                if let Some(s) = t.as_str() {
                    tables.push(s.to_string());
                }
            }
        }
        let first_table = tables.first().cloned();

        let mut app = Self {
            store,
            runtime,
            active_tab: ActiveTab::Tables,
            ai_subtab: AiSubTab::Nlq,
            tables,
            selected_table: first_table.clone(),
            table_search: String::new(),
            table_rows: None,
            filter_text: String::new(),
            sql_query: "SELECT * FROM sqlite_master;".to_string(),
            query_result: None,
            query_latency_ms: None,
            nlq_prompt: "find products with price > 100 limit 5".to_string(),
            nlq_result: None,
            dedupe_column: "title".to_string(),
            dedupe_threshold: 0.75,
            dedupe_result: None,
            synthetic_count: 5,
            synthetic_insert: false,
            synthetic_result: None,
            advisor_result: None,
            ingest_title: "Film Scene 1 Shot Log".to_string(),
            ingest_content: "# Scene 1\nNeon alley opening tracking shot with 50mm lens.\n# Scene 2\nActor enters antique shop and discovers golden artifact.".to_string(),
            ingest_tags: "film, production, shot-log".to_string(),
            ingest_result: None,
            profiler_result: None,
            diagnostic_report: None,
            repair_report: None,
            salvage_path: "salvaged_database.dbcraft".to_string(),
            constructor_blueprint: "small_business".to_string(),
            constructor_seed: true,
            constructor_search: true,
            constructor_result: None,
            constructor_blueprints_list: None,
            rag_text: "Obsidian headphones".to_string(),
            rag_results: None,
            export_target: ExportTarget::DesignCraft,
            export_preview: None,
            status_message: format!(
                "DatabaseCraft v{APP_VERSION} initialized on Port {CONTROL_PORT} [Ready]"
            ),
            canvas_root_table: first_table.clone(),
            canvas_spec: None,
            canvas_result: None,
            canvas_relations: None,
            canvas_latency_ms: None,
            canvas_field_selector_open: false,
            canvas_show_sql_drawer: false,
            canvas_rollup_col: "price".to_string(),
            canvas_rollup_func: AggFunc::Sum,
            canvas_rollup_alias: "total_value".to_string(),
            canvas_density: CanvasDensity::Standard,
            notifications: Vec::new(),
            unread_alerts_count: 0,
            watchdog_last_check: None,
            alert_filter_severity: None,
            alert_filter_unack_only: false,
            active_cell_edit: None,
            cell_undo_stack: Vec::new(),
            cell_redo_stack: Vec::new(),
        };

        app.refresh_notifications();

        if let Some(ref t) = first_table {
            app.load_table_data(t);
            app.init_canvas_for_table(t);
        }

        app
    }

    pub fn refresh_tables(&mut self) {
        let status = self
            .runtime
            .block_on(dispatch(Command::Status, &self.store));
        let mut tables = Vec::new();
        if let Some(arr) = status["tables"].as_array() {
            for t in arr {
                if let Some(s) = t.as_str() {
                    tables.push(s.to_string());
                }
            }
        }
        self.tables = tables;
        if self.selected_table.is_none() {
            self.selected_table = self.tables.first().cloned();
        }
    }

    pub fn load_table_data(&mut self, table: &str) {
        let sql = format!("SELECT * FROM \"{table}\" LIMIT 100;");
        let start = Instant::now();
        let res = self
            .runtime
            .block_on(dispatch(Command::Query { sql }, &self.store));
        self.query_latency_ms = Some(start.elapsed().as_millis());
        self.table_rows = Some(res);
    }

    pub fn execute_sql(&mut self) {
        let sql = self.sql_query.clone();
        let start = Instant::now();
        let res = self
            .runtime
            .block_on(dispatch(Command::Query { sql }, &self.store));
        let elapsed = start.elapsed().as_millis();
        self.query_latency_ms = Some(elapsed);

        if res["success"] == true {
            self.status_message = format!(
                "Query executed in {} ms: {} rows returned",
                elapsed, res["row_count"]
            );
        } else {
            self.status_message = format!("SQL error: {}", res["error"]);
        }
        self.query_result = Some(res);
        self.refresh_tables();
    }

    pub fn run_nlq(&mut self) {
        let prompt = self.nlq_prompt.clone();
        let start = Instant::now();
        let res = self
            .runtime
            .block_on(dispatch(Command::AiNlToSql { prompt }, &self.store));
        self.query_latency_ms = Some(start.elapsed().as_millis());
        if res["success"] == true {
            if let Some(sql_str) = res["sql"].as_str() {
                self.sql_query = sql_str.to_string();
            }
            self.status_message = format!(
                "Synthesized SQL: confidence {:.0}%",
                res["confidence"].as_f64().unwrap_or(0.0) * 100.0
            );
        } else {
            self.status_message = format!("NLQ synthesis error: {}", res["error"]);
        }
        self.nlq_result = Some(res);
    }

    pub fn run_dedupe(&mut self) {
        let table = match self.selected_table.as_deref() {
            Some(t) => t.to_string(),
            None => {
                self.status_message = "No table selected for deduplication".to_string();
                return;
            }
        };
        let column = self.dedupe_column.clone();
        let threshold = self.dedupe_threshold;
        let res = self.runtime.block_on(dispatch(
            Command::AiSemanticDedupe {
                table,
                column,
                threshold: Some(threshold),
            },
            &self.store,
        ));
        self.dedupe_result = Some(res);
        self.status_message = "Semantic deduplication analysis complete".to_string();
    }

    pub fn run_advisor(&mut self) {
        let res = self
            .runtime
            .block_on(dispatch(Command::AiSchemaAdvisor, &self.store));
        self.advisor_result = Some(res);
        self.status_message = "Schema and Index Advisor scan complete".to_string();
    }

    pub fn run_synthetic(&mut self) {
        let table = match self.selected_table.as_deref() {
            Some(t) => t.to_string(),
            None => {
                self.status_message = "No table selected for synthetic rows".to_string();
                return;
            }
        };
        let count = self.synthetic_count;
        let insert = self.synthetic_insert;
        let res = self.runtime.block_on(dispatch(
            Command::AiGenerateSynthetic {
                table: table.clone(),
                count,
                insert: Some(insert),
            },
            &self.store,
        ));
        self.synthetic_result = Some(res);
        if insert {
            self.load_table_data(&table);
            self.status_message =
                format!("Generated and inserted {count} synthetic rows into '{table}'");
        } else {
            self.status_message =
                format!("Generated preview of {count} synthetic rows for '{table}'");
        }
    }

    pub fn run_ingest(&mut self) {
        let title = self.ingest_title.clone();
        let content = self.ingest_content.clone();
        let tags: Vec<String> = self
            .ingest_tags
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let res = self.runtime.block_on(dispatch(
            Command::AiIngestDocument {
                title,
                content,
                tags: Some(tags),
            },
            &self.store,
        ));
        self.ingest_result = Some(res);
        self.refresh_tables();
        self.status_message =
            "Document chunked and ingested into relational, vector, and FTS tables".to_string();
    }

    pub fn run_profiler(&mut self) {
        let table = match self.selected_table.as_deref() {
            Some(t) => t.to_string(),
            None => {
                self.status_message = "No table selected for profiling".to_string();
                return;
            }
        };
        let res = self
            .runtime
            .block_on(dispatch(Command::DataProfile { table }, &self.store));
        self.profiler_result = Some(res);
        self.status_message = "Table profiling metrics computed".to_string();
    }

    pub fn run_diagnostics(&mut self) {
        let res = self
            .runtime
            .block_on(dispatch(Command::Diagnose, &self.store));
        let status = res["report"]["status"].as_str().unwrap_or("UNKNOWN");
        self.status_message = format!("Integrity check finished: [{status}]");
        self.diagnostic_report = Some(res);
    }

    pub fn load_blueprints(&mut self) {
        let res = self
            .runtime
            .block_on(dispatch(Command::ListBlueprints, &self.store));
        self.constructor_blueprints_list = Some(res);
    }

    pub fn construct_database(&mut self) {
        let blueprint = self.constructor_blueprint.clone();
        let seed = self.constructor_seed;
        let search = self.constructor_search;
        let res = self.runtime.block_on(dispatch(
            Command::Construct {
                blueprint: blueprint.clone(),
                seed_sample_data: Some(seed),
                enable_search_indexes: Some(search),
            },
            &self.store,
        ));
        self.constructor_result = Some(res);
        self.refresh_tables();
        self.status_message =
            format!("Constructed turnkey domain database blueprint '{blueprint}'");
    }

    pub fn run_repair(&mut self, action: &str) {
        let salvage = if action == "salvage" {
            Some(self.salvage_path.clone())
        } else {
            None
        };
        let res = self.runtime.block_on(dispatch(
            Command::Repair {
                action: action.to_string(),
                salvage_path: salvage,
            },
            &self.store,
        ));
        self.repair_report = Some(res);
        self.refresh_tables();
        self.run_diagnostics();
    }

    pub fn init_canvas_for_table(&mut self, table_name: &str) {
        self.canvas_root_table = Some(table_name.to_string());

        let schema_res = self.runtime.block_on(dispatch(
            Command::SchemaInspect {
                table: Some(table_name.to_string()),
            },
            &self.store,
        ));

        let mut cols = Vec::new();
        if let Some(tbl) = schema_res.get("table") {
            if let Some(arr) = tbl.get("columns").and_then(|v| v.as_array()) {
                for c in arr {
                    if let Some(name) = c.get("name").and_then(|v| v.as_str()) {
                        cols.push(CanvasColumn::new(name));
                    }
                }
            }
        }
        if cols.is_empty() {
            cols.push(CanvasColumn::new("id"));
        }

        let rel_res = self.runtime.block_on(dispatch(
            Command::SchemaRelations {
                table: table_name.to_string(),
            },
            &self.store,
        ));

        let mut forward_relations = Vec::new();
        let mut reverse_relations = Vec::new();
        if rel_res["success"] == true {
            if let Some(arr) = rel_res["forward_relations"].as_array() {
                for r in arr {
                    if let Ok(sr) = serde_json::from_value::<databasecraft_core::canvas::SchemaRelation>(r.clone()) {
                        forward_relations.push(sr);
                    }
                }
            }
            if let Some(arr) = rel_res["reverse_relations"].as_array() {
                for r in arr {
                    if let Ok(sr) = serde_json::from_value::<databasecraft_core::canvas::SchemaRelation>(r.clone()) {
                        reverse_relations.push(sr);
                    }
                }
            }
        }
        self.canvas_relations = Some(TableRelations {
            table: table_name.to_string(),
            forward_relations,
            reverse_relations,
        });

        let spec = CanvasSpec {
            root_table: table_name.to_string(),
            columns: cols,
            inlined_lookups: Vec::new(),
            nested_relations: Vec::new(),
            aggregations: Vec::new(),
            filters: Vec::new(),
            sort: Vec::new(),
            limit: Some(50),
            offset: None,
        };

        self.canvas_spec = Some(spec);
        self.run_canvas_query();
    }

    pub fn run_canvas_query(&mut self) {
        if let Some(ref spec) = self.canvas_spec.clone() {
            let start = Instant::now();
            let res = self.runtime.block_on(dispatch(
                Command::CanvasQuery {
                    spec: spec.clone(),
                },
                &self.store,
            ));
            self.canvas_latency_ms = Some(start.elapsed().as_millis());
            self.canvas_result = Some(res);
        }
    }

    pub fn sprout_relation(&mut self, rel_name: &str, is_reverse: bool) {
        if let (Some(ref mut spec), Some(ref relations)) =
            (&mut self.canvas_spec, &self.canvas_relations)
        {
            let candidate = if is_reverse {
                relations
                    .reverse_relations
                    .iter()
                    .find(|r| r.relation_name.eq_ignore_ascii_case(rel_name))
            } else {
                relations
                    .forward_relations
                    .iter()
                    .find(|r| r.relation_name.eq_ignore_ascii_case(rel_name))
            };

            if let Some(r) = candidate {
                let schema_res = self.runtime.block_on(dispatch(
                    Command::SchemaInspect {
                        table: Some(r.table.clone()),
                    },
                    &self.store,
                ));
                let mut child_cols = Vec::new();
                if let Some(tbl) = schema_res.get("table") {
                    if let Some(arr) = tbl.get("columns").and_then(|v| v.as_array()) {
                        for c in arr {
                            if let Some(name) = c.get("name").and_then(|v| v.as_str()) {
                                child_cols.push(CanvasColumn::new(name));
                            }
                        }
                    }
                }
                if child_cols.is_empty() {
                    child_cols.push(CanvasColumn::new("id"));
                }

                let nested = CanvasNestedRelation {
                    relation_name: r.relation_name.clone(),
                    target_table: r.table.clone(),
                    parent_key_col: r.source_col.clone(),
                    foreign_key_col: r.target_col.clone(),
                    is_reverse,
                    columns: child_cols,
                    inlined_lookups: Vec::new(),
                    nested_relations: Vec::new(),
                    aggregations: Vec::new(),
                    filters: Vec::new(),
                    limit: None,
                };

                spec.nested_relations.retain(|existing| {
                    !existing.relation_name.eq_ignore_ascii_case(&r.relation_name)
                });
                spec.nested_relations.push(nested);
            }
        }
        self.run_canvas_query();
    }

    pub fn inline_foreign_key(&mut self, rel_name: &str) {
        if let (Some(ref mut spec), Some(ref relations)) =
            (&mut self.canvas_spec, &self.canvas_relations)
        {
            if let Some(r) = relations
                .forward_relations
                .iter()
                .find(|r| r.relation_name.eq_ignore_ascii_case(rel_name))
            {
                let display_col = suggest_display_column(&self.store, &r.table)
                    .unwrap_or_else(|_| "name".to_string());
                let alias = format!("{}_{}", r.relation_name, display_col);

                let lookup = CanvasInlinedLookup {
                    local_fk_col: r.source_col.clone(),
                    foreign_table: r.table.clone(),
                    foreign_pk_col: r.target_col.clone(),
                    display_col,
                    alias: alias.clone(),
                };

                spec.inlined_lookups
                    .retain(|existing| !existing.alias.eq_ignore_ascii_case(&alias));
                spec.inlined_lookups.push(lookup);
            }
        }
        self.run_canvas_query();
    }

    pub fn remove_inlined_lookup(&mut self, alias: &str) {
        if let Some(ref mut spec) = self.canvas_spec {
            spec.inlined_lookups
                .retain(|l| !l.alias.eq_ignore_ascii_case(alias));
        }
        self.run_canvas_query();
    }

    pub fn toggle_canvas_sort(&mut self, col_name: &str) {
        if let Some(ref mut spec) = self.canvas_spec {
            let existing_idx = spec
                .sort
                .iter()
                .position(|s| s.column.eq_ignore_ascii_case(col_name));
            if let Some(idx) = existing_idx {
                if !spec.sort[idx].descending {
                    // Switch ASC -> DESC
                    spec.sort[idx].descending = true;
                } else {
                    // Switch DESC -> None (remove sort)
                    spec.sort.remove(idx);
                }
            } else {
                // Add ASC
                spec.sort.push(CanvasSort {
                    column: col_name.to_string(),
                    descending: false,
                });
            }
        }
        self.run_canvas_query();
    }

    pub fn commit_cell_edit(&mut self) {
        if let Some(edit) = self.active_cell_edit.take() {
            let parsed_val: Value = if edit.edit_str.eq_ignore_ascii_case("null") {
                Value::Null
            } else if let Ok(n) = edit.edit_str.parse::<i64>() {
                Value::Number(n.into())
            } else if let Ok(f) = edit.edit_str.parse::<f64>() {
                serde_json::Number::from_f64(f)
                    .map(Value::Number)
                    .unwrap_or_else(|| Value::String(edit.edit_str.clone()))
            } else if let Ok(b) = edit.edit_str.parse::<bool>() {
                Value::Bool(b)
            } else {
                Value::String(edit.edit_str.clone())
            };

            let old_val: Value = if let Ok(n) = edit.initial_str.parse::<i64>() {
                Value::Number(n.into())
            } else if let Ok(f) = edit.initial_str.parse::<f64>() {
                serde_json::Number::from_f64(f)
                    .map(Value::Number)
                    .unwrap_or_else(|| Value::String(edit.initial_str.clone()))
            } else if let Ok(b) = edit.initial_str.parse::<bool>() {
                Value::Bool(b)
            } else {
                Value::String(edit.initial_str.clone())
            };

            let res = self.runtime.block_on(dispatch(
                Command::UpdateCell {
                    table: edit.table.clone(),
                    pk_col: edit.pk_col.clone(),
                    pk_val: edit.pk_val.clone(),
                    col: edit.col.clone(),
                    new_val: parsed_val.clone(),
                },
                &self.store,
            ));

            if res["success"] == true {
                self.cell_undo_stack.push(CellMutation {
                    table: edit.table,
                    pk_col: edit.pk_col,
                    pk_val: edit.pk_val,
                    col: edit.col,
                    old_val,
                    new_val: parsed_val,
                });
                self.cell_redo_stack.clear();
                self.status_message = "Cell updated successfully".to_string();
                self.run_canvas_query();
                if let Some(ref t) = self.selected_table.clone() {
                    self.load_table_data(t);
                }
            } else {
                self.status_message = format!("Cell update failed: {}", res["error"]);
            }
        }
    }

    pub fn cancel_cell_edit(&mut self) {
        self.active_cell_edit = None;
    }

    pub fn undo_cell_mutation(&mut self) {
        if let Some(mutation) = self.cell_undo_stack.pop() {
            let res = self.runtime.block_on(dispatch(
                Command::UpdateCell {
                    table: mutation.table.clone(),
                    pk_col: mutation.pk_col.clone(),
                    pk_val: mutation.pk_val.clone(),
                    col: mutation.col.clone(),
                    new_val: mutation.old_val.clone(),
                },
                &self.store,
            ));
            if res["success"] == true {
                self.cell_redo_stack.push(mutation);
                self.status_message = "Undid last cell edit".to_string();
                self.run_canvas_query();
                if let Some(ref t) = self.selected_table.clone() {
                    self.load_table_data(t);
                }
            }
        }
    }

    pub fn redo_cell_mutation(&mut self) {
        if let Some(mutation) = self.cell_redo_stack.pop() {
            let res = self.runtime.block_on(dispatch(
                Command::UpdateCell {
                    table: mutation.table.clone(),
                    pk_col: mutation.pk_col.clone(),
                    pk_val: mutation.pk_val.clone(),
                    col: mutation.col.clone(),
                    new_val: mutation.new_val.clone(),
                },
                &self.store,
            ));
            if res["success"] == true {
                self.cell_undo_stack.push(mutation);
                self.status_message = "Redid cell edit".to_string();
                self.run_canvas_query();
                if let Some(ref t) = self.selected_table.clone() {
                    self.load_table_data(t);
                }
            }
        }
    }

    pub fn remove_sprouted_relation(&mut self, rel_name: &str) {
        if let Some(ref mut spec) = self.canvas_spec {
            spec.nested_relations
                .retain(|r| !r.relation_name.eq_ignore_ascii_case(rel_name));
        }
        self.run_canvas_query();
    }

    pub fn add_canvas_aggregation(
        &mut self,
        target_rel: Option<String>,
        col: String,
        func: AggFunc,
        alias: String,
    ) {
        if let Some(ref mut spec) = self.canvas_spec {
            spec.aggregations
                .retain(|a| !a.alias.eq_ignore_ascii_case(&alias));
            spec.aggregations.push(CanvasAggregation {
                target_relation: target_rel,
                target_col: col,
                func,
                alias,
            });
        }
        self.run_canvas_query();
    }

    pub fn remove_canvas_aggregation(&mut self, alias: &str) {
        if let Some(ref mut spec) = self.canvas_spec {
            spec.aggregations
                .retain(|a| !a.alias.eq_ignore_ascii_case(alias));
        }
        self.run_canvas_query();
    }

    pub fn run_rag_search(&mut self) {
        let text = self.rag_text.clone();
        let vec_table = self.tables.iter().find(|t| t.ends_with("_vec")).cloned();
        let fts_table = self.tables.iter().find(|t| t.ends_with("_fts")).cloned();

        let res = if let Some(v_tbl) = vec_table {
            self.runtime.block_on(dispatch(
                Command::RagAsk {
                    vector_table: v_tbl,
                    fts_table,
                    query: text,
                    dimensions: Some(128),
                    top_k: Some(10),
                },
                &self.store,
            ))
        } else if let Some(f_tbl) = fts_table {
            self.runtime.block_on(dispatch(
                Command::FtsSearch {
                    table: f_tbl,
                    query: text,
                    top_k: Some(10),
                },
                &self.store,
            ))
        } else {
            self.runtime.block_on(dispatch(
                Command::RagEmbed {
                    text,
                    dimensions: Some(128),
                },
                &self.store,
            ))
        };
        self.rag_results = Some(res);
    }

    pub fn generate_export(&mut self) {
        let table = match self.selected_table.as_deref() {
            Some(t) => t,
            None => {
                self.status_message = "No table selected for export".to_string();
                return;
            }
        };
        let query = format!("SELECT * FROM \"{table}\";");
        let res = self.runtime.block_on(dispatch(
            Command::ExportMerge {
                query,
                target: self.export_target,
                name: Some(table.to_string()),
            },
            &self.store,
        ));
        self.export_preview = Some(res);
        self.status_message = format!("Export generated for {:?}", self.export_target);
    }

    pub fn refresh_notifications(&mut self) {
        let res = self.runtime.block_on(dispatch(
            Command::NotificationsList {
                severity: None,
                unacknowledged_only: None,
            },
            &self.store,
        ));
        let mut list = Vec::new();
        let mut unack_cnt = 0;
        if let Some(arr) = res["notifications"].as_array() {
            for v in arr {
                if let Ok(notif) = serde_json::from_value::<CraftNotification>(v.clone()) {
                    if !notif.acknowledged {
                        unack_cnt += 1;
                    }
                    list.push(notif);
                }
            }
        }
        self.notifications = list;
        self.unread_alerts_count = unack_cnt;
    }

    pub fn run_watchdog_check(&mut self) {
        let start = Instant::now();
        let res = self
            .runtime
            .block_on(dispatch(Command::WatchdogCheck, &self.store));
        let elapsed = start.elapsed().as_millis();
        let emitted = res["emitted_count"].as_u64().unwrap_or(0);
        let health = res["health"].as_str().unwrap_or("nominal");
        self.status_message = format!(
            "Watchdog scan complete in {} ms: {} notifications emitted (Health: {})",
            elapsed, emitted, health
        );
        self.watchdog_last_check = Some(res);
        self.refresh_notifications();
    }

    pub fn ack_notification(&mut self, id: &str) {
        let _ = self.runtime.block_on(dispatch(
            Command::NotificationAck {
                id: id.to_string(),
            },
            &self.store,
        ));
        self.refresh_notifications();
        self.status_message = format!("Acknowledged notification {id}");
    }
}

impl eframe::App for DatabaseCraftGui {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Theme Colors
        let emerald = Color32::from_rgb(16, 139, 126);
        let emerald_bright = Color32::from_rgb(52, 211, 153);
        let sky_blue = Color32::from_rgb(56, 189, 248);
        let amber = Color32::from_rgb(245, 158, 11);
        let muted_gray = Color32::from_rgb(156, 163, 175);

        // 1. Top Command Bar
        egui::Panel::top("top_bar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.colored_label(emerald, "◆");
                ui.strong(format!("DatabaseCraft v{APP_VERSION}"));
                ui.separator();
                ui.colored_label(
                    emerald_bright,
                    format!("Port {CONTROL_PORT} [Plane 2 Active]"),
                );
                ui.separator();

                let alert_badge_color = if self.unread_alerts_count > 0 {
                    Color32::from_rgb(239, 68, 68)
                } else {
                    muted_gray
                };
                let alert_label = if self.unread_alerts_count > 0 {
                    format!("🚨 {} Alerts", self.unread_alerts_count)
                } else {
                    "🔔 Alerts".to_string()
                };
                if ui
                    .button(
                        egui::RichText::new(alert_label)
                            .color(alert_badge_color)
                            .strong(),
                    )
                    .clicked()
                {
                    self.active_tab = ActiveTab::Alerts;
                    self.refresh_notifications();
                }
                ui.separator();

                if ui.button("🌱 Seed: Ecommerce").clicked() {
                    let _ = self.runtime.block_on(dispatch(
                        Command::SeedDemo {
                            name: "ecommerce".to_string(),
                        },
                        &self.store,
                    ));
                    self.refresh_tables();
                    if let Some(t) = self.tables.first().cloned() {
                        self.selected_table = Some(t.clone());
                        self.load_table_data(&t);
                    }
                    self.status_message = "Seeded Ecommerce Catalog".to_string();
                }

                if ui.button("🎬 Seed: Film").clicked() {
                    let _ = self.runtime.block_on(dispatch(
                        Command::SeedDemo {
                            name: "film".to_string(),
                        },
                        &self.store,
                    ));
                    self.refresh_tables();
                    if let Some(t) = self.tables.first().cloned() {
                        self.selected_table = Some(t.clone());
                        self.load_table_data(&t);
                    }
                    self.status_message = "Seeded Film Production Takes".to_string();
                }

                if ui.button("📊 Seed: KPI").clicked() {
                    let _ = self.runtime.block_on(dispatch(
                        Command::SeedDemo {
                            name: "kpi".to_string(),
                        },
                        &self.store,
                    ));
                    self.refresh_tables();
                    if let Some(t) = self.tables.first().cloned() {
                        self.selected_table = Some(t.clone());
                        self.load_table_data(&t);
                    }
                    self.status_message = "Seeded Executive KPI Metrics".to_string();
                }

                if ui
                    .button(
                        egui::RichText::new("🚢 Seed: Northwind Traders")
                            .color(Color32::from_rgb(52, 211, 153))
                            .strong(),
                    )
                    .clicked()
                {
                    let _ = self.runtime.block_on(dispatch(
                        Command::SeedDemo {
                            name: "northwind".to_string(),
                        },
                        &self.store,
                    ));
                    self.refresh_tables();
                    self.selected_table = Some("customers".to_string());
                    self.load_table_data("customers");
                    self.init_canvas_for_table("customers");
                    self.sprout_relation("orders", true);
                    self.active_tab = ActiveTab::Canvas;
                    self.status_message =
                        "Seeded Northwind Traders Commercial Dataset: Customers -> Orders (1:N)"
                            .to_string();
                }

                if ui.button("🧹 Vacuum").clicked() {
                    let _ = self
                        .runtime
                        .block_on(dispatch(Command::Vacuum, &self.store));
                    self.status_message = "Database vacuumed and defragmented".to_string();
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::widgets::global_theme_preference_buttons(ui);
                });
            });
        });

        // 2. Bottom Status Bar
        egui::Panel::bottom("status_bar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.colored_label(emerald, "●");
                ui.label(&self.status_message);
                if let Some(ms) = self.query_latency_ms {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.colored_label(muted_gray, format!("Latency: {} ms", ms));
                    });
                }
            });
        });

        // 3. Left Navigation Rail
        egui::Panel::left("nav_rail")
            .min_size(210.0)
            .show(ui, |ui| {
                ui.add_space(6.0);
                ui.strong("WORKSPACES");
                ui.add_space(4.0);

                let nav_button = |ui: &mut egui::Ui, active: bool, label: &str| -> bool {
                    let text = egui::RichText::new(label).color(if active {
                        emerald_bright
                    } else {
                        ui.visuals().text_color()
                    });
                    ui.selectable_label(active, text).clicked()
                };

                if nav_button(ui, self.active_tab == ActiveTab::Tables, "📊 Table Data") {
                    self.active_tab = ActiveTab::Tables;
                }
                if nav_button(
                    ui,
                    self.active_tab == ActiveTab::Canvas,
                    "✨ Relational Canvas",
                ) {
                    self.active_tab = ActiveTab::Canvas;
                    if self.canvas_spec.is_none() {
                        if let Some(ref t) = self.selected_table.clone() {
                            self.init_canvas_for_table(t);
                        }
                    }
                }
                if nav_button(
                    ui,
                    self.active_tab == ActiveTab::Query,
                    "⚡ SQL & NLQ Console",
                ) {
                    self.active_tab = ActiveTab::Query;
                }
                if nav_button(ui, self.active_tab == ActiveTab::Schema, "🗺️ Schema & ERD") {
                    self.active_tab = ActiveTab::Schema;
                }
                if nav_button(
                    ui,
                    self.active_tab == ActiveTab::AiWorkflows,
                    "🤖 AI Copilot & Workflows",
                ) {
                    self.active_tab = ActiveTab::AiWorkflows;
                }
                if nav_button(ui, self.active_tab == ActiveTab::Rag, "🎯 RAG & Vectors") {
                    self.active_tab = ActiveTab::Rag;
                }
                if nav_button(
                    ui,
                    self.active_tab == ActiveTab::Profiler,
                    "📈 Data Profiler",
                ) {
                    self.active_tab = ActiveTab::Profiler;
                    self.run_profiler();
                }
                if nav_button(
                    ui,
                    self.active_tab == ActiveTab::Doctor,
                    "🩺 Doctor & Repair",
                ) {
                    self.active_tab = ActiveTab::Doctor;
                    self.run_diagnostics();
                }
                if nav_button(
                    ui,
                    self.active_tab == ActiveTab::Constructor,
                    "🏗️ Domain Constructor",
                ) {
                    self.active_tab = ActiveTab::Constructor;
                    self.load_blueprints();
                }
                if nav_button(
                    ui,
                    self.active_tab == ActiveTab::Export,
                    "📦 Cross-Craft Merge",
                ) {
                    self.active_tab = ActiveTab::Export;
                }
                let nav_alert_label = if self.unread_alerts_count > 0 {
                    format!("🚨 Alerts & Watchdog ({})", self.unread_alerts_count)
                } else {
                    "🔔 Alerts & Watchdog".to_string()
                };
                if nav_button(ui, self.active_tab == ActiveTab::Alerts, &nav_alert_label) {
                    self.active_tab = ActiveTab::Alerts;
                    self.refresh_notifications();
                }

                ui.separator();
                ui.horizontal(|ui| {
                    ui.strong(format!("TABLES ({})", self.tables.len()));
                    if ui.small_button("⟳").clicked() {
                        self.refresh_tables();
                    }
                });
                ui.add_space(2.0);

                ui.text_edit_singleline(&mut self.table_search);
                let search_lower = self.table_search.trim().to_lowercase();

                let mut table_to_select = None;
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for t in &self.tables {
                        if !search_lower.is_empty() && !t.to_lowercase().contains(&search_lower) {
                            continue;
                        }
                        let is_selected = self.selected_table.as_deref() == Some(t);
                        let icon = if t.ends_with("_vec") {
                            "🎯"
                        } else if t.ends_with("_fts") {
                            "🔍"
                        } else {
                            "📋"
                        };
                        let label = format!("{icon} {t}");
                        if ui.selectable_label(is_selected, label).clicked() {
                            table_to_select = Some(t.clone());
                        }
                    }
                });

                if let Some(selected) = table_to_select {
                    self.selected_table = Some(selected.clone());
                    self.load_table_data(&selected);
                    if self.active_tab == ActiveTab::Canvas {
                        self.init_canvas_for_table(&selected);
                    }
                }
            });

        // 4. Central Workspace Area
        egui::CentralPanel::default().show(ui, |ui| match self.active_tab {
            ActiveTab::Tables => {
                ui.horizontal(|ui| {
                    ui.heading(format!(
                        "Table: {}",
                        self.selected_table.as_deref().unwrap_or("None")
                    ));
                    ui.add_space(8.0);
                    if ui
                        .button(
                            egui::RichText::new("✨ Open in Relational Canvas")
                                .color(emerald_bright)
                                .strong(),
                        )
                        .clicked()
                    {
                        if let Some(ref t) = self.selected_table.clone() {
                            self.init_canvas_for_table(t);
                            self.active_tab = ActiveTab::Canvas;
                        }
                    }
                    ui.add_space(16.0);
                    ui.label("Search filter:");
                    ui.text_edit_singleline(&mut self.filter_text);
                    if ui.button("⟳ Reload").clicked() {
                        if let Some(ref t) = self.selected_table.clone() {
                            self.load_table_data(t);
                        }
                    }
                });
                ui.separator();

                if let Some(ref res) = self.table_rows {
                    if let (Some(cols), Some(rows)) =
                        (res["columns"].as_array(), res["rows"].as_array())
                    {
                        let filter_lower = self.filter_text.trim().to_lowercase();
                        let filtered_rows: Vec<&Value> = if filter_lower.is_empty() {
                            rows.iter().collect()
                        } else {
                            rows.iter()
                                .filter(|r| {
                                    r.as_array().is_some_and(|cells| {
                                        cells.iter().any(|c| {
                                            c.to_string().to_lowercase().contains(&filter_lower)
                                        })
                                    })
                                })
                                .collect()
                        };

                        ui.horizontal(|ui| {
                            ui.colored_label(emerald_bright, format!("Showing {} of {} rows", filtered_rows.len(), rows.len()));
                            ui.colored_label(muted_gray, format!("• {} columns", cols.len()));
                        });

                        egui::ScrollArea::both().show(ui, |ui| {
                            egui::Grid::new("table_grid").striped(true).spacing(Vec2::new(14.0, 6.0)).show(ui, |ui| {
                                ui.strong("#");
                                for col in cols {
                                    ui.colored_label(emerald_bright, col.as_str().unwrap_or(""));
                                }
                                ui.end_row();

                                for (row_idx, row) in filtered_rows.iter().enumerate() {
                                    ui.weak(format!("{}", row_idx + 1));
                                    if let Some(cells) = row.as_array() {
                                        for cell in cells {
                                            if cell.is_null() {
                                                ui.colored_label(muted_gray, "[NULL]");
                                            } else if let Some(n) = cell.as_f64() {
                                                ui.colored_label(sky_blue, format!("{n}"));
                                            } else if let Some(s) = cell.as_str() {
                                                ui.label(s);
                                            } else {
                                                ui.label(cell.to_string());
                                            }
                                        }
                                        ui.end_row();
                                    }
                                }
                            });
                        });
                    }
                } else {
                    ui.label("Select a table from the left navigation rail or click Seed above to populate data.");
                }
            }
            ActiveTab::Canvas => {
                let current_table = self
                    .canvas_root_table
                    .clone()
                    .or_else(|| self.selected_table.clone())
                    .unwrap_or_else(|| "None".to_string());

                // Top control bar
                ui.horizontal(|ui| {
                    ui.colored_label(emerald_bright, "✨ Relational Perspective Canvas");
                    ui.separator();
                    ui.label("Root Table:");
                    ui.colored_label(Color32::WHITE, egui::RichText::new(&current_table).strong());

                    ui.add_space(8.0);
                    ui.label("Filter:");
                    ui.add(egui::TextEdit::singleline(&mut self.filter_text).desired_width(140.0));

                    if ui.button("🎛 Fields & Joins").clicked() {
                        self.canvas_field_selector_open = !self.canvas_field_selector_open;
                    }

                    if ui
                        .button(if self.canvas_show_sql_drawer {
                            "⚡ Hide SQL"
                        } else {
                            "⚡ View SQL"
                        })
                        .clicked()
                    {
                        self.canvas_show_sql_drawer = !self.canvas_show_sql_drawer;
                    }

                    if ui.button("⟳ Reload").clicked() {
                        self.run_canvas_query();
                    }

                    ui.separator();
                    ui.label("Density:");
                    ui.selectable_value(
                        &mut self.canvas_density,
                        CanvasDensity::Compact,
                        "Compact",
                    );
                    ui.selectable_value(
                        &mut self.canvas_density,
                        CanvasDensity::Standard,
                        "Standard",
                    );
                    ui.selectable_value(
                        &mut self.canvas_density,
                        CanvasDensity::Relaxed,
                        "Relaxed",
                    );

                    ui.separator();
                    if !self.cell_undo_stack.is_empty()
                        && ui
                            .button(format!("↩ Undo ({})", self.cell_undo_stack.len()))
                            .clicked()
                    {
                        self.undo_cell_mutation();
                    }
                    if !self.cell_redo_stack.is_empty()
                        && ui
                            .button(format!("↪ Redo ({})", self.cell_redo_stack.len()))
                            .clicked()
                    {
                        self.redo_cell_mutation();
                    }
                });

                // Sprout bar: Forward and reverse relationships discovered
                let mut sprout_action: Option<(String, bool)> = None;
                let mut inline_action: Option<String> = None;
                let mut remove_sprout_action: Option<String> = None;
                let mut remove_inline_action: Option<String> = None;
                let mut remove_agg_alias: Option<String> = None;

                if let Some(ref rels) = self.canvas_relations.clone() {
                    ui.horizontal_wrapped(|ui| {
                        ui.weak("Relations:");

                        // Reverse relations (1:N)
                        for r in &rels.reverse_relations {
                            let is_sprouted = self.canvas_spec.as_ref().is_some_and(|s| {
                                s.nested_relations
                                    .iter()
                                    .any(|nr| nr.relation_name.eq_ignore_ascii_case(&r.relation_name))
                            });

                            if is_sprouted {
                                if ui
                                    .button(
                                        egui::RichText::new(format!("✓ {} (1:N) ✖", r.relation_name))
                                            .color(sky_blue)
                                            .strong(),
                                    )
                                    .clicked()
                                {
                                    remove_sprout_action = Some(r.relation_name.clone());
                                }
                            } else if ui.button(format!("+ ⤹ Sprout: {} (1:N)", r.relation_name)).clicked() {
                                sprout_action = Some((r.relation_name.clone(), true));
                            }
                        }

                        // Forward relations (N:1)
                        for r in &rels.forward_relations {
                            let is_sprouted = self.canvas_spec.as_ref().is_some_and(|s| {
                                s.nested_relations
                                    .iter()
                                    .any(|nr| nr.relation_name.eq_ignore_ascii_case(&r.relation_name))
                            });

                            if is_sprouted {
                                if ui
                                    .button(
                                        egui::RichText::new(format!("✓ {} (N:1) ✖", r.relation_name))
                                            .color(sky_blue)
                                            .strong(),
                                    )
                                    .clicked()
                                {
                                    remove_sprout_action = Some(r.relation_name.clone());
                                }
                            } else {
                                if ui.button(format!("+ 🔗 Sprout: {} (N:1)", r.relation_name)).clicked() {
                                    sprout_action = Some((r.relation_name.clone(), false));
                                }
                                if ui.button(format!("+ 🏷️ Inline: {} (N:1)", r.relation_name)).clicked() {
                                    inline_action = Some(r.relation_name.clone());
                                }
                            }
                        }

                        // Active inlined lookups (Compact Joins)
                        if let Some(ref spec) = self.canvas_spec {
                            for lookup in &spec.inlined_lookups {
                                if ui
                                    .button(
                                        egui::RichText::new(format!("✓ 🏷️ {} ✖", lookup.alias))
                                            .color(Color32::from_rgb(192, 132, 252))
                                            .strong(),
                                    )
                                    .clicked()
                                {
                                    remove_inline_action = Some(lookup.alias.clone());
                                }
                            }
                        }

                        // Active rollups
                        if let Some(ref spec) = self.canvas_spec {
                            for agg in &spec.aggregations {
                                if ui
                                    .button(
                                        egui::RichText::new(format!(
                                             "∑ {}: {:?}({}) ✖",
                                            agg.alias, agg.func, agg.target_col
                                        ))
                                        .color(amber),
                                    )
                                    .clicked()
                                {
                                    remove_agg_alias = Some(agg.alias.clone());
                                }
                            }
                        }
                    });
                }

                if let Some((rel, is_rev)) = sprout_action {
                    self.sprout_relation(&rel, is_rev);
                }
                if let Some(rel) = inline_action {
                    self.inline_foreign_key(&rel);
                }
                if let Some(rel) = remove_sprout_action {
                    self.remove_sprouted_relation(&rel);
                }
                if let Some(alias) = remove_inline_action {
                    self.remove_inlined_lookup(&alias);
                }
                if let Some(alias) = remove_agg_alias {
                    self.remove_canvas_aggregation(&alias);
                }
                ui.separator();

                // SQL Drawer
                if self.canvas_show_sql_drawer {
                    if let Some(ref res) = self.canvas_result {
                        if let Some(sql) = res["result"]["compiled_sql"].as_str() {
                            egui::Frame::group(ui.style())
                                .fill(Color32::from_rgb(22, 24, 29))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.colored_label(
                                            amber,
                                            "Compiled SQLite JSON Aggregation Query:",
                                        );
                                        if let Some(lat) = self.canvas_latency_ms {
                                            ui.colored_label(
                                                muted_gray,
                                                format!("• Execution Latency: {lat}ms"),
                                            );
                                        }
                                    });
                                    egui::ScrollArea::vertical().max_height(100.0).show(ui, |ui| {
                                        ui.add(
                                            egui::TextEdit::multiline(&mut sql.to_string())
                                                .font(egui::TextStyle::Monospace)
                                                .desired_width(f32::INFINITY),
                                        );
                                    });
                                });
                            ui.separator();
                        }
                    }
                }

                // Render Canvas Data Cards
                if let Some(ref res) = self.canvas_result {
                    if res["success"] == true {
                        if let Some(data) = res["result"]["data"].as_array() {
                            let filter_lower = self.filter_text.trim().to_lowercase();
                            let filtered_records: Vec<&Value> = if filter_lower.is_empty() {
                                data.iter().collect()
                            } else {
                                data.iter()
                                    .filter(|rec| {
                                        rec.to_string().to_lowercase().contains(&filter_lower)
                                    })
                                    .collect()
                            };

                            let sprouted_count = self
                                .canvas_spec
                                .as_ref()
                                .map(|s| s.nested_relations.len())
                                .unwrap_or(0);
                            let rollup_count = self
                                .canvas_spec
                                .as_ref()
                                .map(|s| s.aggregations.len())
                                .unwrap_or(0);

                            ui.horizontal(|ui| {
                                ui.colored_label(
                                    emerald_bright,
                                    format!("Perspective: {} records", filtered_records.len()),
                                );
                                ui.colored_label(
                                    sky_blue,
                                    format!("• {sprouted_count} sprouted subqueries"),
                                );
                                ui.colored_label(amber, format!("• {rollup_count} rollups"));
                                if let Some(lat) = self.canvas_latency_ms {
                                    ui.colored_label(muted_gray, format!("• {lat}ms"));
                                }
                            });

                            ui.add_space(2.0);

                            let density = self.canvas_density;
                            let font_size = density.font_size();
                            let pad = density.cell_padding();
                            let min_row_h = density.row_height();

                            // Extract visible root columns
                            let root_cols: Vec<String> = self
                                .canvas_spec
                                .as_ref()
                                .map(|s| {
                                    s.columns
                                        .iter()
                                        .filter(|c| c.visible)
                                        .map(|c| c.alias.clone().unwrap_or_else(|| c.name.clone()))
                                        .collect()
                                })
                                .unwrap_or_default();

                            // Extract inlined lookups (Compact Joins)
                            let inlined_lookups = self
                                .canvas_spec
                                .as_ref()
                                .map(|s| s.inlined_lookups.clone())
                                .unwrap_or_default();

                            // Extract nested relations
                            let nested_rels = self
                                .canvas_spec
                                .as_ref()
                                .map(|s| s.nested_relations.clone())
                                .unwrap_or_default();

                            // Extract aggregations
                            let rollups = self
                                .canvas_spec
                                .as_ref()
                                .map(|s| s.aggregations.clone())
                                .unwrap_or_default();

                            let pk_col = self
                                .store
                                .get_primary_key(&current_table)
                                .ok()
                                .flatten()
                                .unwrap_or_else(|| "id".to_string());

                            let mut commit_cell = false;
                            let mut cancel_cell = false;
                            let mut start_cell_edit: Option<(String, String, Value, String, String)> =
                                None;
                            let mut toggle_sort_col: Option<String> = None;

                            egui::ScrollArea::both().show(ui, |ui| {
                                // Outer container with hairline borders
                                egui::Frame::group(ui.style())
                                    .fill(Color32::from_rgb(17, 19, 23))
                                    .stroke(egui::Stroke::new(1.0, Color32::from_rgb(38, 42, 50)))
                                    .inner_margin(0.0)
                                    .show(ui, |ui| {
                                        // 1. TIER 1: Table Ownership Header Band
                                        ui.horizontal(|ui| {
                                            ui.spacing_mut().item_spacing = Vec2::ZERO;

                                            // Root Table Header Box
                                            let root_title = format!(
                                                "🏛️ {} (Root Entity)",
                                                current_table.to_uppercase()
                                            );
                                            egui::Frame::new()
                                                .fill(Color32::from_rgb(16, 75, 70))
                                                .stroke(egui::Stroke::new(
                                                    1.0,
                                                    Color32::from_rgb(13, 148, 136),
                                                ))
                                                .inner_margin(egui::Margin::symmetric(12, 4))
                                                .show(ui, |ui| {
                                                    ui.strong(
                                                        egui::RichText::new(root_title)
                                                            .size(font_size + 1.0)
                                                            .color(Color32::WHITE),
                                                    );
                                                });

                                            // Inlined Lookups Band
                                            if !inlined_lookups.is_empty() {
                                                egui::Frame::new()
                                                    .fill(Color32::from_rgb(58, 28, 92))
                                                    .stroke(egui::Stroke::new(
                                                        1.0,
                                                        Color32::from_rgb(168, 85, 247),
                                                    ))
                                                    .inner_margin(egui::Margin::symmetric(12, 4))
                                                    .show(ui, |ui| {
                                                        ui.strong(
                                                            egui::RichText::new(format!(
                                                                "🏷️ {} INLINED (Compact)",
                                                                inlined_lookups.len()
                                                            ))
                                                            .size(font_size + 1.0)
                                                            .color(Color32::from_rgb(
                                                                233, 213, 255,
                                                            )),
                                                        );
                                                    });
                                            }

                                            // Nested Subquery Bands
                                            for rel in &nested_rels {
                                                let sub_title = format!(
                                                    "↳ {} (1:N Subquery)",
                                                    rel.relation_name.to_uppercase()
                                                );
                                                egui::Frame::new()
                                                    .fill(Color32::from_rgb(22, 50, 78))
                                                    .stroke(egui::Stroke::new(
                                                        1.0,
                                                        Color32::from_rgb(14, 165, 233),
                                                    ))
                                                    .inner_margin(egui::Margin::symmetric(12, 4))
                                                    .show(ui, |ui| {
                                                        ui.strong(
                                                            egui::RichText::new(sub_title)
                                                                .size(font_size + 1.0)
                                                                .color(Color32::from_rgb(
                                                                    186, 230, 253,
                                                                )),
                                                        );
                                                    });
                                            }

                                            // Rollups Band
                                            if !rollups.is_empty() {
                                                egui::Frame::new()
                                                    .fill(Color32::from_rgb(66, 48, 16))
                                                    .stroke(egui::Stroke::new(
                                                        1.0,
                                                        Color32::from_rgb(245, 158, 11),
                                                    ))
                                                    .inner_margin(egui::Margin::symmetric(12, 4))
                                                    .show(ui, |ui| {
                                                        ui.strong(
                                                            egui::RichText::new("∑ INLINE ROLLUPS")
                                                                .size(font_size + 1.0)
                                                                .color(Color32::from_rgb(
                                                                    254, 243, 199,
                                                                )),
                                                        );
                                                    });
                                            }
                                        });

                                        // 2. TIER 2: Column Headers
                                        ui.horizontal(|ui| {
                                            ui.spacing_mut().item_spacing = Vec2::ZERO;

                                            let header_bg = Color32::from_rgb(24, 27, 33);
                                            let border_stroke = egui::Stroke::new(
                                                1.0,
                                                Color32::from_rgb(38, 42, 50),
                                            );

                                            // Row index column header
                                            egui::Frame::new()
                                                .fill(header_bg)
                                                .stroke(border_stroke)
                                                .inner_margin(egui::Margin::symmetric(8, 5))
                                                .show(ui, |ui| {
                                                    ui.colored_label(muted_gray, "#");
                                                });

                                            // Root table column headers with interactive sorting
                                            for col in &root_cols {
                                                let sort_icon = self
                                                    .canvas_spec
                                                    .as_ref()
                                                    .and_then(|s| {
                                                        s.sort
                                                            .iter()
                                                            .find(|si| {
                                                                si.column.eq_ignore_ascii_case(col)
                                                            })
                                                            .map(|si| {
                                                                if si.descending {
                                                                    " ▼"
                                                                } else {
                                                                    " ▲"
                                                                }
                                                            })
                                                    })
                                                    .unwrap_or("");

                                                egui::Frame::new()
                                                    .fill(header_bg)
                                                    .stroke(border_stroke)
                                                    .inner_margin(egui::Margin::symmetric(
                                                        pad.0, 5,
                                                    ))
                                                    .show(ui, |ui| {
                                                        let btn_text = format!(
                                                            "{}{sort_icon}",
                                                            col.to_uppercase()
                                                        );
                                                        if ui
                                                            .button(
                                                                egui::RichText::new(btn_text)
                                                                    .color(emerald_bright)
                                                                    .strong(),
                                                            )
                                                            .clicked()
                                                        {
                                                            toggle_sort_col = Some(col.clone());
                                                        }
                                                    });
                                            }

                                            // Inlined lookups column headers
                                            for lookup in &inlined_lookups {
                                                egui::Frame::new()
                                                    .fill(Color32::from_rgb(26, 20, 36))
                                                    .stroke(border_stroke)
                                                    .inner_margin(egui::Margin::symmetric(
                                                        pad.0, 5,
                                                    ))
                                                    .show(ui, |ui| {
                                                        ui.colored_label(
                                                            Color32::from_rgb(192, 132, 252),
                                                            format!(
                                                                "🏷️ {}",
                                                                lookup.alias.to_uppercase()
                                                            ),
                                                        );
                                                    });
                                            }

                                            // Subquery column headers
                                            for rel in &nested_rels {
                                                egui::Frame::new()
                                                    .fill(Color32::from_rgb(20, 24, 32))
                                                    .stroke(border_stroke)
                                                    .inner_margin(egui::Margin::symmetric(6, 5))
                                                    .show(ui, |ui| {
                                                        ui.colored_label(sky_blue, "SUB#");
                                                    });

                                                for c in &rel.columns {
                                                    if !c.visible {
                                                        continue;
                                                    }
                                                    let col_name =
                                                        c.alias.as_deref().unwrap_or(&c.name);
                                                    egui::Frame::new()
                                                        .fill(Color32::from_rgb(20, 24, 32))
                                                        .stroke(border_stroke)
                                                        .inner_margin(egui::Margin::symmetric(
                                                            pad.0, 5,
                                                        ))
                                                        .show(ui, |ui| {
                                                            ui.colored_label(
                                                                sky_blue,
                                                                col_name.to_uppercase(),
                                                            );
                                                        });
                                                }
                                            }

                                            // Rollup column headers
                                            for agg in &rollups {
                                                egui::Frame::new()
                                                    .fill(Color32::from_rgb(28, 25, 20))
                                                    .stroke(border_stroke)
                                                    .inner_margin(egui::Margin::symmetric(
                                                        pad.0, 5,
                                                    ))
                                                    .show(ui, |ui| {
                                                        ui.colored_label(
                                                            amber,
                                                            format!(
                                                                "∑ {}",
                                                                agg.alias.to_uppercase()
                                                            ),
                                                        );
                                                    });
                                            }
                                        });

                                        // 3. CONTINUOUS ROW BLOCKS
                                        for (row_idx, record) in filtered_records.iter().enumerate()
                                        {
                                            if let Some(obj) = record.as_object() {
                                                let row_bg = if row_idx % 2 == 0 {
                                                    Color32::from_rgb(18, 20, 24)
                                                } else {
                                                    Color32::from_rgb(24, 27, 33)
                                                };
                                                let border_stroke = egui::Stroke::new(
                                                    1.0,
                                                    Color32::from_rgb(38, 42, 50),
                                                );

                                                egui::Frame::new()
                                                    .fill(row_bg)
                                                    .stroke(border_stroke)
                                                    .show(ui, |ui| {
                                                        ui.horizontal(|ui| {
                                                            ui.spacing_mut().item_spacing =
                                                                Vec2::ZERO;
                                                            ui.spacing_mut().interact_size.y =
                                                                min_row_h;

                                                            // Row number
                                                            egui::Frame::new()
                                                                .stroke(border_stroke)
                                                                .inner_margin(
                                                                    egui::Margin::symmetric(
                                                                        8, pad.1,
                                                                    ),
                                                                )
                                                                .show(ui, |ui| {
                                                                    ui.colored_label(
                                                                        emerald_bright,
                                                                        format!("#{}", row_idx + 1),
                                                                    );
                                                                });

                                                            // Root columns with interactive cell editing
                                                            let row_pk_val = obj
                                                                .get(&pk_col)
                                                                .cloned()
                                                                .unwrap_or(Value::Number(
                                                                    (row_idx as i64).into(),
                                                                ));

                                                            for col in &root_cols {
                                                                let val = obj
                                                                    .get(col)
                                                                    .unwrap_or(&Value::Null);

                                                                let is_editing = self
                                                                    .active_cell_edit
                                                                    .as_ref()
                                                                    .is_some_and(|e| {
                                                                        e.table == current_table
                                                                            && e.pk_val
                                                                                == row_pk_val
                                                                            && e.col == *col
                                                                    });

                                                                egui::Frame::new()
                                                                    .stroke(border_stroke)
                                                                    .inner_margin(
                                                                        egui::Margin::symmetric(
                                                                            pad.0, pad.1,
                                                                        ),
                                                                    )
                                                                    .show(ui, |ui| {
                                                                        if is_editing {
                                                                            ui.horizontal(|ui| {
                                                                                if let Some(
                                                                                    ref mut edit,
                                                                                ) = self
                                                                                    .active_cell_edit
                                                                                {
                                                                                    let text_resp = ui.add(
                                                                                        egui::TextEdit::singleline(
                                                                                            &mut edit.edit_str,
                                                                                        )
                                                                                        .desired_width(85.0),
                                                                                    );
                                                                                    if text_resp.lost_focus()
                                                                                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                                                                    {
                                                                                        commit_cell = true;
                                                                                    }
                                                                                }
                                                                                if ui.button("✓").clicked() {
                                                                                    commit_cell = true;
                                                                                }
                                                                                if ui.button("✖").clicked() {
                                                                                    cancel_cell = true;
                                                                                }
                                                                            });
                                                                        } else {
                                                                            let cell_resp = ui.scope(|ui| {
                                                                                render_formatted_cell(
                                                                                    ui, col, val, density,
                                                                                );
                                                                            }).response;

                                                                            if cell_resp.double_clicked() {
                                                                                let val_str = match val {
                                                                                    Value::String(s) => s.clone(),
                                                                                    Value::Null => "".to_string(),
                                                                                    _ => val.to_string(),
                                                                                };
                                                                                start_cell_edit = Some((
                                                                                    current_table.clone(),
                                                                                    pk_col.clone(),
                                                                                    row_pk_val.clone(),
                                                                                    col.clone(),
                                                                                    val_str,
                                                                                ));
                                                                            }
                                                                        }
                                                                    });
                                                            }

                                                            // Inlined lookups (Compact Joins)
                                                            for lookup in &inlined_lookups {
                                                                let val = obj
                                                                    .get(&lookup.alias)
                                                                    .unwrap_or(&Value::Null);
                                                                egui::Frame::new()
                                                                    .stroke(border_stroke)
                                                                    .inner_margin(
                                                                        egui::Margin::symmetric(
                                                                            pad.0, pad.1,
                                                                        ),
                                                                    )
                                                                    .show(ui, |ui| {
                                                                        render_formatted_cell(
                                                                            ui,
                                                                            &lookup.alias,
                                                                            val,
                                                                            density,
                                                                        );
                                                                    });
                                                            }

                                                            // Sprouted subquery tables
                                                            for rel in &nested_rels {
                                                                let sub_val = obj
                                                                    .get(&rel.relation_name)
                                                                    .unwrap_or(&Value::Null);
                                                                let sub_rows = sub_val.as_array();

                                                                if let Some(rows) = sub_rows {
                                                                    if rows.is_empty() {
                                                                        egui::Frame::new()
                                                                            .stroke(border_stroke)
                                                                            .inner_margin(
                                                                                egui::Margin::symmetric(
                                                                                    pad.0, pad.1,
                                                                                ),
                                                                            )
                                                                            .show(ui, |ui| {
                                                                                ui.colored_label(
                                                                                    muted_gray,
                                                                                    "— (empty) —",
                                                                                );
                                                                            });
                                                                    } else {
                                                                        // Render nested aligned sub-table with hairline borders
                                                                        ui.vertical(|ui| {
                                                                            ui.spacing_mut()
                                                                                .item_spacing =
                                                                                Vec2::ZERO;
                                                                            for (c_idx, c_row) in
                                                                                rows.iter()
                                                                                    .enumerate()
                                                                            {
                                                                                let sub_row_bg = if c_idx
                                                                                    % 2
                                                                                    == 0
                                                                                {
                                                                                    Color32::from_rgb(
                                                                                        20, 23, 29,
                                                                                    )
                                                                                } else {
                                                                                    Color32::from_rgb(
                                                                                        26, 30, 38,
                                                                                    )
                                                                                };
                                                                                egui::Frame::new()
                                                                                    .fill(
                                                                                        sub_row_bg,
                                                                                    )
                                                                                    .stroke(
                                                                                        border_stroke,
                                                                                    )
                                                                                    .show(
                                                                                        ui,
                                                                                        |ui| {
                                                                                            ui.horizontal(|ui| {
                                                                                    ui.spacing_mut().item_spacing = Vec2::ZERO;
                                                                                    // Sub-index
                                                                                    egui::Frame::new()
                                                                                        .stroke(border_stroke)
                                                                                        .inner_margin(egui::Margin::symmetric(6, pad.1))
                                                                                        .show(ui, |ui| {
                                                                                            ui.weak(format!("{}.", c_idx + 1));
                                                                                        });

                                                                                    if let Some(c_obj) = c_row.as_object() {
                                                                                        for c in &rel.columns {
                                                                                            if !c.visible {
                                                                                                continue;
                                                                                            }
                                                                                            let key = c.alias.as_deref().unwrap_or(&c.name);
                                                                                            let val = c_obj.get(key).unwrap_or(&Value::Null);
                                                                                            egui::Frame::new()
                                                                                                .stroke(border_stroke)
                                                                                                .inner_margin(egui::Margin::symmetric(pad.0, pad.1))
                                                                                                .show(ui, |ui| {
                                                                                                    render_formatted_cell(ui, &c.name, val, density);
                                                                                                });
                                                                                        }
                                                                                    }
                                                                                });
                                                                                        },
                                                                                    );
                                                                            }
                                                                        });
                                                                    }
                                                                } else {
                                                                    egui::Frame::new()
                                                                        .stroke(border_stroke)
                                                                        .inner_margin(
                                                                            egui::Margin::symmetric(
                                                                                pad.0, pad.1,
                                                                            ),
                                                                        )
                                                                        .show(ui, |ui| {
                                                                            ui.colored_label(
                                                                                muted_gray, "—",
                                                                            );
                                                                        });
                                                                }
                                                            }

                                                            // Inline Rollups
                                                            for agg in &rollups {
                                                                let val = obj
                                                                    .get(&agg.alias)
                                                                    .unwrap_or(&Value::Null);
                                                                egui::Frame::new()
                                                                    .stroke(border_stroke)
                                                                    .inner_margin(
                                                                        egui::Margin::symmetric(
                                                                            pad.0, pad.1,
                                                                        ),
                                                                    )
                                                                    .show(ui, |ui| {
                                                                        if let Some(n) =
                                                                            val.as_f64()
                                                                        {
                                                                            let formatted = if agg
                                                                                .func
                                                                                == AggFunc::Sum
                                                                                && (agg
                                                                                    .target_col
                                                                                    .contains(
                                                                                        "price",
                                                                                    )
                                                                                    || agg
                                                                                        .target_col
                                                                                        .contains(
                                                                                            "freight",
                                                                                        )
                                                                                    || agg
                                                                                        .target_col
                                                                                        .contains(
                                                                                            "total",
                                                                                        )) {
                                                                                format_currency(n)
                                                                            } else {
                                                                                format!("{n}")
                                                                            };
                                                                            let font_id =
                                                                                egui::FontId::new(
                                                                                    font_size,
                                                                                    egui::FontFamily::Monospace,
                                                                                );
                                                                            ui.label(
                                                                                egui::RichText::new(
                                                                                    formatted,
                                                                                )
                                                                                .font(font_id)
                                                                                .color(amber)
                                                                                .strong(),
                                                                            );
                                                                        } else {
                                                                            ui.label(
                                                                                egui::RichText::new(
                                                                                    val.to_string(),
                                                                                )
                                                                                .size(font_size)
                                                                                .color(amber),
                                                                            );
                                                                        }
                                                                    });
                                                            }
                                                        });
                                                    });
                                            }
                                        }
                                    });
                            });

                            if commit_cell {
                                self.commit_cell_edit();
                            }
                            if cancel_cell {
                                self.cancel_cell_edit();
                            }
                            if let Some((table, pk_col, pk_val, col, val_str)) = start_cell_edit {
                                self.active_cell_edit = Some(ActiveCellEdit {
                                    table,
                                    pk_col,
                                    pk_val,
                                    col,
                                    initial_str: val_str.clone(),
                                    edit_str: val_str,
                                });
                            }
                            if let Some(col) = toggle_sort_col {
                                self.toggle_canvas_sort(&col);
                            }
                        }
                    } else {
                        ui.colored_label(
                            Color32::RED,
                            format!(
                                "Canvas Error: {}",
                                res["error"].as_str().unwrap_or("Unknown")
                            ),
                        );
                    }
                } else {
                    ui.label("Select a table from the left navigation rail to initialize the Relational Canvas.");
                }

                // Field Selector Modal Palette Window
                if self.canvas_field_selector_open {
                    let mut is_open = true;
                    egui::Window::new("🎛 Fields & Joins Palette (Ultorg-Style)")
                        .open(&mut is_open)
                        .default_width(380.0)
                        .show(ui.ctx(), |ui| {
                            ui.heading("Field Selector");
                            ui.separator();

                            ui.strong("Root Table Attributes:");
                            if let Some(ref mut spec) = self.canvas_spec {
                                for col in &mut spec.columns {
                                    ui.checkbox(&mut col.visible, &col.name);
                                }
                            }

                            ui.separator();
                            ui.strong("Add Inline Rollup Metric:");
                            ui.horizontal(|ui| {
                                ui.label("Target Col:");
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.canvas_rollup_col)
                                        .desired_width(70.0),
                                );

                                ui.label("Func:");
                                ui.selectable_value(
                                    &mut self.canvas_rollup_func,
                                    AggFunc::Sum,
                                    "SUM",
                                );
                                ui.selectable_value(
                                    &mut self.canvas_rollup_func,
                                    AggFunc::Count,
                                    "COUNT",
                                );
                                ui.selectable_value(
                                    &mut self.canvas_rollup_func,
                                    AggFunc::Avg,
                                    "AVG",
                                );
                                ui.selectable_value(
                                    &mut self.canvas_rollup_func,
                                    AggFunc::Min,
                                    "MIN",
                                );
                                ui.selectable_value(
                                    &mut self.canvas_rollup_func,
                                    AggFunc::Max,
                                    "MAX",
                                );

                                ui.label("Alias:");
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.canvas_rollup_alias)
                                        .desired_width(80.0),
                                );
                            });

                            let sprout_options: Vec<String> = self
                                .canvas_spec
                                .as_ref()
                                .map(|s| {
                                    s.nested_relations
                                        .iter()
                                        .map(|nr| nr.relation_name.clone())
                                        .collect()
                                })
                                .unwrap_or_default();

                            let target_relation: Option<String> = sprout_options.first().cloned();

                            if ui.button("➕ Add Rollup").clicked() {
                                let col = self.canvas_rollup_col.clone();
                                let func = self.canvas_rollup_func;
                                let alias = self.canvas_rollup_alias.clone();
                                self.add_canvas_aggregation(target_relation, col, func, alias);
                            }

                            ui.add_space(8.0);
                            if ui.button("Close Palette").clicked() {
                                self.canvas_field_selector_open = false;
                            }
                        });
                    if !is_open {
                        self.canvas_field_selector_open = false;
                    }
                }
            }
            ActiveTab::Query => {
                ui.heading("SQL & Natural Language Query Console");
                ui.separator();

                // NLQ Assist Bar
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.strong("🤖 NLQ Query Synthesizer:");
                        ui.add(egui::TextEdit::singleline(&mut self.nlq_prompt).desired_width(450.0));
                        if ui.button("⚡ Translate to SQL").clicked() {
                            self.run_nlq();
                        }
                    });
                    if let Some(ref res) = self.nlq_result {
                        if res["success"] == true {
                            ui.horizontal(|ui| {
                                ui.colored_label(emerald_bright, format!("Target: {}", res["target_table"].as_str().unwrap_or("")));
                                ui.colored_label(muted_gray, format!("• {}", res["explanation"].as_str().unwrap_or("")));
                            });
                        }
                    }
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if ui.button("▶ Run Query (Ctrl+Enter)").clicked() {
                        self.execute_sql();
                    }
                    ui.separator();
                    ui.weak("Templates:");
                    if ui.small_button("SELECT *").clicked() {
                        if let Some(ref t) = self.selected_table {
                            self.sql_query = format!("SELECT * FROM \"{t}\" LIMIT 50;");
                        }
                    }
                    if ui.small_button("COUNT(*)").clicked() {
                        if let Some(ref t) = self.selected_table {
                            self.sql_query = format!("SELECT COUNT(*) AS total FROM \"{t}\";");
                        }
                    }
                    if ui.small_button("GROUP BY").clicked() {
                        if let Some(ref t) = self.selected_table {
                            self.sql_query = format!("SELECT * FROM \"{t}\" GROUP BY 1 LIMIT 20;");
                        }
                    }
                });

                ui.add(
                    egui::TextEdit::multiline(&mut self.sql_query)
                        .font(egui::TextStyle::Monospace)
                        .desired_rows(5)
                        .desired_width(f32::INFINITY),
                );

                ui.separator();
                if let Some(ref res) = self.query_result {
                    if res["success"] == true {
                        if let (Some(cols), Some(rows)) =
                            (res["columns"].as_array(), res["rows"].as_array())
                        {
                            ui.horizontal(|ui| {
                                ui.colored_label(emerald_bright, format!("Result: {} rows", rows.len()));
                                ui.colored_label(muted_gray, format!("• {} columns", cols.len()));
                            });

                            egui::ScrollArea::both().show(ui, |ui| {
                                egui::Grid::new("query_result_grid").striped(true).spacing(Vec2::new(12.0, 4.0)).show(ui, |ui| {
                                    for col in cols {
                                        ui.strong(col.as_str().unwrap_or(""));
                                    }
                                    ui.end_row();

                                    for row in rows {
                                        if let Some(cells) = row.as_array() {
                                            for cell in cells {
                                                ui.label(cell.to_string());
                                            }
                                            ui.end_row();
                                        }
                                    }
                                });
                            });
                        }
                    } else {
                        ui.colored_label(Color32::from_rgb(239, 68, 68), format!("Query Error: {}", res["error"]));
                    }
                }
            }
            ActiveTab::Schema => {
                ui.horizontal(|ui| {
                    ui.heading("Database Schema Inspector & Visual ERD");
                    ui.separator();
                    if ui.button("🛡️ Run Index Advisor").clicked() {
                        self.active_tab = ActiveTab::AiWorkflows;
                        self.ai_subtab = AiSubTab::Advisor;
                        self.run_advisor();
                    }
                });
                ui.separator();

                let full_schema = self.runtime.block_on(dispatch(
                    Command::SchemaInspect { table: None },
                    &self.store,
                ));

                egui::ScrollArea::both().show(ui, |ui| {
                    if let Some(tables_arr) = full_schema["schema"]["tables"].as_array() {
                        ui.horizontal_wrapped(|ui| {
                            for table in tables_arr {
                                egui::Frame::group(ui.style()).show(ui, |ui| {
                                    ui.set_min_width(280.0);
                                    let table_name = table["name"].as_str().unwrap_or("unnamed");
                                    ui.horizontal(|ui| {
                                        ui.colored_label(emerald_bright, "📋");
                                        ui.strong(table_name);
                                        if let Some(cols) = table["columns"].as_array() {
                                            ui.weak(format!("({} cols)", cols.len()));
                                        }
                                    });
                                    ui.separator();

                                    if let Some(cols) = table["columns"].as_array() {
                                        for col in cols {
                                            ui.horizontal(|ui| {
                                                if col["pk"] == true {
                                                    ui.colored_label(amber, "🔑 PK");
                                                }
                                                ui.label(col["name"].as_str().unwrap_or(""));
                                                ui.colored_label(muted_gray, col["data_type"].as_str().unwrap_or(""));
                                                if col["notnull"] == true {
                                                    ui.colored_label(amber, "*");
                                                }
                                            });
                                        }
                                    }

                                    if let Some(fks) = table["foreign_keys"].as_array() {
                                        if !fks.is_empty() {
                                            ui.separator();
                                            for fk in fks {
                                                let from = fk["from"].as_str().unwrap_or("");
                                                let target_tbl = fk["table"].as_str().unwrap_or("");
                                                let to = fk["to"].as_str().unwrap_or("");
                                                ui.colored_label(sky_blue, format!("↳ {from} → {target_tbl}({to})"));
                                            }
                                        }
                                    }
                                });
                                ui.add_space(8.0);
                            }
                        });
                    }
                });
            }
            ActiveTab::AiWorkflows => {
                ui.heading("AI Copilot & Workflow Automation Engine");
                ui.separator();

                ui.horizontal(|ui| {
                    if ui.selectable_label(self.ai_subtab == AiSubTab::Nlq, "💬 NL to SQL").clicked() {
                        self.ai_subtab = AiSubTab::Nlq;
                    }
                    if ui.selectable_label(self.ai_subtab == AiSubTab::Dedupe, "🔍 Semantic Dedupe").clicked() {
                        self.ai_subtab = AiSubTab::Dedupe;
                    }
                    if ui.selectable_label(self.ai_subtab == AiSubTab::Synthetic, "🧪 Synthetic Data").clicked() {
                        self.ai_subtab = AiSubTab::Synthetic;
                    }
                    if ui.selectable_label(self.ai_subtab == AiSubTab::Advisor, "🛡️ Schema Advisor").clicked() {
                        self.ai_subtab = AiSubTab::Advisor;
                    }
                    if ui.selectable_label(self.ai_subtab == AiSubTab::Ingest, "📄 Document Ingest").clicked() {
                        self.ai_subtab = AiSubTab::Ingest;
                    }
                });
                ui.separator();

                match self.ai_subtab {
                    AiSubTab::Nlq => {
                        ui.label("Ask questions or query requirements in plain English:");
                        ui.horizontal(|ui| {
                            ui.add(egui::TextEdit::singleline(&mut self.nlq_prompt).desired_width(500.0));
                            if ui.button("⚡ Synthesize SQL").clicked() {
                                self.run_nlq();
                            }
                        });
                        ui.add_space(6.0);

                        let mut run_exec = false;
                        if let Some(ref res) = self.nlq_result {
                            egui::Frame::group(ui.style()).show(ui, |ui| {
                                ui.strong("Generated SQL Statement:");
                                ui.monospace(res["sql"].as_str().unwrap_or(""));
                                ui.separator();
                                ui.label(format!("Explanation: {}", res["explanation"].as_str().unwrap_or("")));
                                ui.label(format!("Target Table: {}", res["target_table"].as_str().unwrap_or("")));
                                ui.label(format!("Confidence: {:.1}%", res["confidence"].as_f64().unwrap_or(0.0) * 100.0));
                                if ui.button("▶ Execute Generated Query").clicked() {
                                    run_exec = true;
                                }
                            });
                        }
                        if run_exec {
                            self.active_tab = ActiveTab::Query;
                            self.execute_sql();
                        }
                    }
                    AiSubTab::Dedupe => {
                        ui.label("Detect near-duplicate records using dense vector embedding similarity:");
                        ui.horizontal(|ui| {
                            ui.label(format!("Target Table: {}", self.selected_table.as_deref().unwrap_or("None")));
                            ui.separator();
                            ui.label("Column:");
                            ui.text_edit_singleline(&mut self.dedupe_column);
                            ui.separator();
                            ui.label("Similarity Threshold:");
                            ui.add(egui::Slider::new(&mut self.dedupe_threshold, 0.5..=0.99).text("sim"));
                            if ui.button("🔍 Find Duplicates").clicked() {
                                self.run_dedupe();
                            }
                        });
                        ui.separator();

                        if let Some(ref res) = self.dedupe_result {
                            if res["success"] == true {
                                ui.colored_label(emerald_bright, format!(
                                    "Evaluated {} pairs • Found {} duplicate clusters",
                                    res["pairs_evaluated"], res["duplicates_found"]
                                ));
                                egui::ScrollArea::both().show(ui, |ui| {
                                    if let Some(clusters) = res["clusters"].as_array() {
                                        for c in clusters {
                                            egui::Frame::group(ui.style()).show(ui, |ui| {
                                                let sim = c["similarity"].as_f64().unwrap_or(0.0);
                                                ui.horizontal(|ui| {
                                                    ui.colored_label(amber, format!("{:.1}% Match", sim * 100.0));
                                                    ui.strong(format!("Record #{} vs #{}", c["id_a"], c["id_b"]));
                                                });
                                                ui.label(format!("A: '{}'", c["text_a"].as_str().unwrap_or("")));
                                                ui.label(format!("B: '{}'", c["text_b"].as_str().unwrap_or("")));
                                                ui.colored_label(sky_blue, c["recommendation"].as_str().unwrap_or(""));
                                            });
                                            ui.add_space(4.0);
                                        }
                                    }
                                });
                            }
                        }
                    }
                    AiSubTab::Synthetic => {
                        ui.label("Generate realistic, context-aware synthetic rows with valid foreign keys:");
                        ui.horizontal(|ui| {
                            ui.label(format!("Target Table: {}", self.selected_table.as_deref().unwrap_or("None")));
                            ui.separator();
                            ui.label("Row Count:");
                            ui.add(egui::Slider::new(&mut self.synthetic_count, 1..=50));
                            ui.checkbox(&mut self.synthetic_insert, "Insert directly into database");
                            if ui.button("🧪 Generate Rows").clicked() {
                                self.run_synthetic();
                            }
                        });
                        ui.separator();

                        if let Some(ref res) = self.synthetic_result {
                            if res["success"] == true {
                                ui.colored_label(emerald_bright, format!(
                                    "Generated {} records (inserted: {})",
                                    res["requested_count"], res["inserted"]
                                ));
                                egui::ScrollArea::both().show(ui, |ui| {
                                    ui.monospace(serde_json::to_string_pretty(&res["records"]).unwrap_or_default());
                                });
                            }
                        }
                    }
                    AiSubTab::Advisor => {
                        ui.horizontal(|ui| {
                            ui.label("Scan schema for missing indexes, primary keys, and performance bottlenecks:");
                            if ui.button("🛡️ Scan Schema").clicked() {
                                self.run_advisor();
                            }
                        });
                        ui.separator();

                        if let Some(ref res) = self.advisor_result {
                            if res["success"] == true {
                                ui.colored_label(emerald_bright, format!(
                                    "Inspected {} tables • Found {} optimization recommendations",
                                    res["total_tables"], res["issues_found"]
                                ));
                                egui::ScrollArea::both().show(ui, |ui| {
                                    if let Some(recs) = res["recommendations"].as_array() {
                                        for r in recs {
                                            egui::Frame::group(ui.style()).show(ui, |ui| {
                                                let priority = r["priority"].as_str().unwrap_or("INFO");
                                                let color = match priority {
                                                    "HIGH" => Color32::from_rgb(239, 68, 68),
                                                    "MEDIUM" => amber,
                                                    _ => sky_blue,
                                                };
                                                ui.horizontal(|ui| {
                                                    ui.colored_label(color, format!("[{priority}]"));
                                                    ui.strong(format!("Table '{}': {}", r["table"].as_str().unwrap_or(""), r["issue"].as_str().unwrap_or("")));
                                                });
                                                ui.label(r["recommendation"].as_str().unwrap_or(""));
                                                if let Some(ddl) = r["ddl_fix"].as_str() {
                                                    ui.monospace(format!("Fix DDL: {ddl}"));
                                                }
                                            });
                                            ui.add_space(4.0);
                                        }
                                    }
                                });
                            }
                        }
                    }
                    AiSubTab::Ingest => {
                        ui.label("Ingest unstructured text or notes with automatic section chunking and vector indexing:");
                        ui.horizontal(|ui| {
                            ui.label("Title:");
                            ui.text_edit_singleline(&mut self.ingest_title);
                            ui.label("Tags (comma separated):");
                            ui.text_edit_singleline(&mut self.ingest_tags);
                            if ui.button("📄 Ingest & Chunk").clicked() {
                                self.run_ingest();
                            }
                        });
                        ui.label("Document Content (Markdown):");
                        ui.add(egui::TextEdit::multiline(&mut self.ingest_content).desired_rows(6).desired_width(f32::INFINITY));

                        if let Some(ref res) = self.ingest_result {
                            if res["success"] == true {
                                ui.colored_label(emerald_bright, format!(
                                    "Successfully indexed {} chunks into tables: '{}', '{}', '{}'",
                                    res["chunks_indexed"], res["target_table"], res["vector_table"], res["fts_table"]
                                ));
                            }
                        }
                    }
                }
            }
            ActiveTab::Rag => {
                ui.heading("RAG & In-Process Semantic Retrieval Workbench");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Semantic Query:");
                    ui.add(egui::TextEdit::singleline(&mut self.rag_text).desired_width(400.0));
                    if ui.button("⚡ Semantic Search").clicked() {
                        self.run_rag_search();
                    }
                });

                ui.separator();
                if let Some(ref res) = self.rag_results {
                    egui::ScrollArea::both().show(ui, |ui| {
                        if let Some(hits) = res["hits"].as_array() {
                            ui.colored_label(emerald_bright, format!(
                                "Retrieved {} semantic hits (retrieval mode: {}):",
                                hits.len(),
                                res["mode"].as_str().unwrap_or("hybrid")
                            ));
                            for (i, hit) in hits.iter().enumerate() {
                                egui::Frame::group(ui.style()).show(ui, |ui| {
                                    let score = hit.get("rrf_score")
                                        .or_else(|| hit.get("score"))
                                        .and_then(|v| v.as_f64())
                                        .unwrap_or(0.0);
                                    ui.horizontal(|ui| {
                                        ui.strong(format!("#{}: ID {}", i + 1, hit["id"]));
                                        ui.colored_label(emerald_bright, format!("Similarity Score: {:.4}", score));
                                    });
                                    if !hit["metadata"].is_null() {
                                        ui.monospace(serde_json::to_string_pretty(&hit["metadata"]).unwrap_or_default());
                                    }
                                });
                                ui.add_space(4.0);
                            }
                        } else {
                            ui.monospace(serde_json::to_string_pretty(res).unwrap_or_default());
                        }
                    });
                }
            }
            ActiveTab::Profiler => {
                ui.horizontal(|ui| {
                    ui.heading(format!(
                        "Data Profiler & Quality Metrics: {}",
                        self.selected_table.as_deref().unwrap_or("None")
                    ));
                    ui.separator();
                    if ui.button("⟳ Re-profile").clicked() {
                        self.run_profiler();
                    }
                });
                ui.separator();

                if let Some(ref res) = self.profiler_result {
                    if res["success"] == true {
                        let prof = &res["profile"];
                        ui.colored_label(emerald_bright, format!(
                            "Total Records: {} • Table: {}",
                            prof["total_rows"], prof["table"]
                        ));

                        egui::ScrollArea::both().show(ui, |ui| {
                            egui::Grid::new("profiler_grid").striped(true).spacing(Vec2::new(16.0, 6.0)).show(ui, |ui| {
                                ui.strong("Column");
                                ui.strong("Type");
                                ui.strong("Nulls");
                                ui.strong("Distinct Values");
                                ui.strong("Min Bound");
                                ui.strong("Max Bound");
                                ui.end_row();

                                if let Some(cols) = prof["columns"].as_array() {
                                    for col in cols {
                                        ui.colored_label(emerald_bright, col["name"].as_str().unwrap_or(""));
                                        ui.colored_label(muted_gray, col["data_type"].as_str().unwrap_or(""));
                                        let nulls = col["null_count"].as_u64().unwrap_or(0);
                                        if nulls > 0 {
                                            ui.colored_label(amber, format!("{nulls}"));
                                        } else {
                                            ui.label("0");
                                        }
                                        ui.colored_label(sky_blue, format!("{}", col["distinct_count"]));
                                        ui.label(col["min_value"].as_str().unwrap_or("-"));
                                        ui.label(col["max_value"].as_str().unwrap_or("-"));
                                        ui.end_row();
                                    }
                                }
                            });
                        });
                    }
                }
            }
            ActiveTab::Doctor => {
                ui.horizontal(|ui| {
                    ui.heading("Database Health Inspector, Integrity Doctor & Repair Suite");
                    ui.separator();
                    if ui.button("🩺 Run Full Diagnostic Scan").clicked() {
                        self.run_diagnostics();
                    }
                });
                ui.separator();

                let mut action_to_run = None;

                if let Some(ref res) = self.diagnostic_report {
                    if res["success"] == true {
                        let rep = &res["report"];
                        let status = rep["status"].as_str().unwrap_or("HEALTHY");
                        let is_healthy = rep["is_healthy"].as_bool().unwrap_or(false);
                        let integrity_ok = rep["integrity_ok"].as_bool().unwrap_or(false);
                        let fks = rep["fk_violations"].as_array();
                        let fk_count = fks.map(|a| a.len()).unwrap_or(0);

                        let (status_color, status_icon, status_desc) = match status {
                            "CORRUPTED" => (
                                Color32::from_rgb(239, 68, 68),
                                "🚨",
                                "CRITICAL: B-Tree page corruption or invalid cell pointers detected! Immediate reindex or salvage required.",
                            ),
                            "DEGRADED" => (
                                amber,
                                "⚠️",
                                "DEGRADED: Relational inconsistency detected. Orphan child rows violate foreign-key constraints.",
                            ),
                            _ => (
                                emerald_bright,
                                "✅",
                                "HEALTHY: All SQLite B-Tree pages, indexes, and foreign-key constraints verified 100% consistent.",
                            ),
                        };

                        // Health Status Card
                        egui::Frame::group(ui.style()).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.heading(format!("{status_icon} Database Status:"));
                                ui.colored_label(status_color, egui::RichText::new(format!("[{status}]")).size(20.0).strong());
                            });
                            ui.colored_label(status_color, status_desc);
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.label(format!("B-Tree Integrity: {}", if integrity_ok { "PASS (ok)" } else { "FAIL" }));
                                ui.separator();
                                ui.label(format!("Foreign Key Violations: {} orphans", fk_count));
                                ui.separator();
                                ui.label(format!("Inspected Tables: {}", rep["total_tables"]));
                            });
                        });

                        ui.add_space(6.0);

                        // If issues found, show details
                        if !is_healthy {
                            ui.strong("Detected Anomalies & Inconsistencies:");
                            if let Some(errs) = rep["integrity_errors"].as_array() {
                                for err in errs {
                                    ui.colored_label(Color32::from_rgb(239, 68, 68), format!("• [B-Tree Corruption] {}", err.as_str().unwrap_or("")));
                                }
                            }
                            if let Some(violations) = fks {
                                for v in violations {
                                    ui.colored_label(amber, format!(
                                        "• [FK Orphan Violation] Table '{}' rowid {} -> non-existent row in '{}' (constraint fkid {})",
                                        v["table"].as_str().unwrap_or(""),
                                        v["rowid"],
                                        v["target_table"].as_str().unwrap_or(""),
                                        v["fkid"]
                                    ));
                                }
                            }
                            ui.separator();
                        }

                        // Automated Repair Action Center
                        ui.heading("Automated Repair & Salvage Center");
                        ui.horizontal_wrapped(|ui| {
                            egui::Frame::group(ui.style()).show(ui, |ui| {
                                ui.set_min_width(260.0);
                                ui.colored_label(emerald_bright, "⚡ Auto-Repair Sequence");
                                ui.weak("Runs Reindex + Purge FK Orphans + Vacuum defrag in safe sequence.");
                                ui.add_space(4.0);
                                if ui.button("🔧 Execute Auto-Repair").clicked() {
                                    action_to_run = Some("auto");
                                }
                            });

                            egui::Frame::group(ui.style()).show(ui, |ui| {
                                ui.set_min_width(260.0);
                                ui.colored_label(sky_blue, "🔨 Rebuild B-Tree Indexes");
                                ui.weak("Rebuilds all database indexes from table data without modifying rows.");
                                ui.add_space(4.0);
                                if ui.button("🔨 Execute REINDEX").clicked() {
                                    action_to_run = Some("reindex");
                                }
                            });

                            egui::Frame::group(ui.style()).show(ui, |ui| {
                                ui.set_min_width(260.0);
                                ui.colored_label(amber, "🧹 Purge Orphaned FK Rows");
                                ui.weak("Deletes orphaned child records that violate foreign key integrity.");
                                ui.add_space(4.0);
                                if ui.button("🧹 Purge FK Orphans").clicked() {
                                    action_to_run = Some("purge_orphans");
                                }
                            });

                            egui::Frame::group(ui.style()).show(ui, |ui| {
                                ui.set_min_width(260.0);
                                ui.colored_label(Color32::from_rgb(244, 63, 94), "🛡️ Salvage & Clean Clone");
                                ui.weak("Recovers surviving schemas and rows row-by-row into a brand new clean file.");
                                ui.add_space(2.0);
                                ui.horizontal(|ui| {
                                    ui.label("Path:");
                                    ui.text_edit_singleline(&mut self.salvage_path);
                                });
                                if ui.button("🛡️ Salvage to Clean Clone").clicked() {
                                    action_to_run = Some("salvage");
                                }
                            });
                        });

                        // Show last repair output
                        if let Some(ref rep_res) = self.repair_report {
                            ui.separator();
                            ui.strong("Last Repair Execution Log:");
                            egui::Frame::group(ui.style()).show(ui, |ui| {
                                ui.monospace(serde_json::to_string_pretty(rep_res).unwrap_or_default());
                            });
                        }
                    }
                } else {
                    ui.label("Click 'Run Full Diagnostic Scan' above to inspect database B-tree pages and relational integrity.");
                }

                if let Some(act) = action_to_run {
                    self.run_repair(act);
                }
            }
            ActiveTab::Constructor => {
                let mut do_construct = false;
                ui.horizontal(|ui| {
                    ui.heading("🏗️ Turnkey Domain Database Constructor");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("⟳ Refresh Blueprints").clicked() {
                            self.load_blueprints();
                        }
                    });
                });
                ui.label(
                    "Instantly construct battle-tested, relational schemas for common organizational patterns \
                     with foreign keys, multi-column indexes, full-text search (FTS5), vector embeddings, and realistic starter seeds."
                );
                ui.separator();

                ui.columns(5, |cols| {
                    let archetypes_row1 = [
                        (
                            "small_business",
                            "🏢 Small Business",
                            "Commerce & Trade",
                            "Customers, Products, Invoices, Items, Payments + FTS5",
                            Color32::from_rgb(52, 211, 153),
                        ),
                        (
                            "club",
                            "🏆 Club & Society",
                            "Community & Sports",
                            "Tiers, Members, Teams, Dues, Events + FTS5",
                            Color32::from_rgb(96, 165, 250),
                        ),
                        (
                            "surgery",
                            "🩺 Medical Clinic",
                            "Healthcare & Practice",
                            "Practitioners, Patients, Appointments, Notes, Vectors",
                            Color32::from_rgb(248, 113, 113),
                        ),
                        (
                            "creative_agency",
                            "🎨 Creative Studio",
                            "Production & Media",
                            "Clients, Projects, Deliverables, Time Logs + FTS5",
                            Color32::from_rgb(251, 191, 36),
                        ),
                        (
                            "real_estate",
                            "🏡 Real Estate",
                            "Property & Leasing",
                            "Properties, Units, Tenants, Leases, Tickets + FTS5",
                            Color32::from_rgb(167, 139, 250),
                        ),
                    ];

                    for (i, (id, name, cat, desc, tint)) in archetypes_row1.iter().enumerate() {
                        cols[i].group(|ui| {
                            let is_sel = self.constructor_blueprint == *id;
                            if is_sel {
                                ui.colored_label(*tint, format!("● ACTIVE: {name}"));
                            } else {
                                ui.colored_label(ui.visuals().text_color(), *name);
                            }
                            ui.colored_label(Color32::from_rgb(148, 163, 184), *cat);
                            ui.add_space(3.0);
                            ui.label(*desc);
                            ui.add_space(4.0);
                            if ui.selectable_label(is_sel, if is_sel { "Selected" } else { "Choose" }).clicked() {
                                self.constructor_blueprint = id.to_string();
                            }
                        });
                    }
                });

                ui.add_space(6.0);

                ui.columns(5, |cols| {
                    let archetypes_row2 = [
                        (
                            "field_service",
                            "🔧 Field Service",
                            "Contractors & Trades",
                            "Accounts, Technicians, Work Orders, Parts, Invoices",
                            Color32::from_rgb(249, 115, 22),
                        ),
                        (
                            "hospitality",
                            "🍽️ Restaurant Dining",
                            "Hospitality & Tables",
                            "Sections, Tables, Guests, Reservations, Allergies",
                            Color32::from_rgb(236, 72, 153),
                        ),
                        (
                            "legal",
                            "⚖️ Legal Practice",
                            "Law & Case Docket",
                            "Clients, Matters, Filings, Deadlines, Time Logs",
                            Color32::from_rgb(129, 140, 248),
                        ),
                        (
                            "automotive",
                            "🚗 Auto Garage",
                            "Workshop & Fleet",
                            "Owners, Vehicles, Job Cards, Parts, MOT Tests",
                            Color32::from_rgb(45, 212, 191),
                        ),
                        (
                            "education",
                            "🎓 Academy & School",
                            "Education & Tutoring",
                            "Instructors, Students, Courses, Cohorts, Roll Call",
                            Color32::from_rgb(163, 230, 53),
                        ),
                    ];

                    for (i, (id, name, cat, desc, tint)) in archetypes_row2.iter().enumerate() {
                        cols[i].group(|ui| {
                            let is_sel = self.constructor_blueprint == *id;
                            if is_sel {
                                ui.colored_label(*tint, format!("● ACTIVE: {name}"));
                            } else {
                                ui.colored_label(ui.visuals().text_color(), *name);
                            }
                            ui.colored_label(Color32::from_rgb(148, 163, 184), *cat);
                            ui.add_space(3.0);
                            ui.label(*desc);
                            ui.add_space(4.0);
                            if ui.selectable_label(is_sel, if is_sel { "Selected" } else { "Choose" }).clicked() {
                                self.constructor_blueprint = id.to_string();
                            }
                        });
                    }
                });

                ui.add_space(8.0);
                ui.separator();
                ui.horizontal(|ui| {
                    ui.strong("Configuration Options:");
                    ui.checkbox(&mut self.constructor_seed, "Seed Realistic Starter Records");
                    ui.checkbox(&mut self.constructor_search, "Enable FTS5 Full-Text & Vector Search Tables");
                    ui.add_space(20.0);
                    if ui.button(egui::RichText::new("🚀 Construct Blueprint Now").color(Color32::from_rgb(16, 185, 129)).strong()).clicked() {
                        do_construct = true;
                    }
                });
                ui.separator();

                if let Some(ref res) = self.constructor_result {
                    if res["success"] == true {
                        let c = &res["construction"];
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.colored_label(Color32::from_rgb(52, 211, 153), "✔ SUCCESS:");
                                ui.strong(format!("Constructed Blueprint '{}' ({})", c["blueprint_name"].as_str().unwrap_or(""), c["blueprint_id"].as_str().unwrap_or("")));
                            });
                            ui.horizontal(|ui| {
                                ui.label(format!("• Tables Created: {}", c["tables_created"].as_array().map(|a| a.len()).unwrap_or(0)));
                                ui.label(format!("• Indexes Created: {}", c["indexes_created"].as_array().map(|a| a.len()).unwrap_or(0)));
                                ui.label(format!("• Starter Records Seeded: {}", c["sample_records_seeded"]));
                            });
                            if let Some(tbls) = c["tables_created"].as_array() {
                                let names: Vec<&str> = tbls.iter().filter_map(|v| v.as_str()).collect();
                                ui.colored_label(Color32::from_rgb(148, 163, 184), format!("Tables: {}", names.join(", ")));
                            }
                        });
                    } else {
                        ui.group(|ui| {
                            ui.colored_label(Color32::from_rgb(239, 68, 68), format!("✖ Construction Error: {}", res["error"].as_str().unwrap_or("Unknown")));
                        });
                    }
                } else {
                    ui.label("Select a domain archetype above and click 'Construct Blueprint Now' to create the schema and starter records.");
                }

                if do_construct {
                    self.construct_database();
                }
            }
            ActiveTab::Export => {
                ui.heading("Cross-Craft Multi-App Data Merge");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Target Application:");
                    ui.selectable_value(
                        &mut self.export_target,
                        ExportTarget::DesignCraft,
                        "DesignCraft",
                    );
                    ui.selectable_value(
                        &mut self.export_target,
                        ExportTarget::DeckCraft,
                        "DeckCraft",
                    );
                    ui.selectable_value(
                        &mut self.export_target,
                        ExportTarget::WordCraft,
                        "WordCraft",
                    );
                    ui.selectable_value(
                        &mut self.export_target,
                        ExportTarget::GridCraft,
                        "GridCraft",
                    );
                    ui.selectable_value(
                        &mut self.export_target,
                        ExportTarget::PdfCraft,
                        "PdfCraft",
                    );

                    if ui.button("🚀 Generate Merge Payload").clicked() {
                        self.generate_export();
                    }
                });

                ui.separator();
                if let Some(ref res) = self.export_preview {
                    egui::ScrollArea::both().show(ui, |ui| {
                        ui.monospace(
                            serde_json::to_string_pretty(res).unwrap_or_else(|_| res.to_string()),
                        );
                    });
                }
            }
            ActiveTab::Alerts => {
                ui.heading("🚨 CTO Operations Watchdog & Anomaly Alert Bus");
                ui.colored_label(
                    muted_gray,
                    "Proactive 24/7 monitoring for database disk starvation, quiescent table churn, foreign-key orphans, and structural corruption.",
                );
                ui.separator();

                // Top Action & Filter Row
                ui.horizontal(|ui| {
                    if ui.button("🔍 Run Watchdog Scan Now").clicked() {
                        self.run_watchdog_check();
                    }
                    if ui.button("⟳ Refresh Feed").clicked() {
                        self.refresh_notifications();
                    }
                    ui.separator();
                    ui.checkbox(&mut self.alert_filter_unack_only, "Unacknowledged only");
                    ui.separator();
                    ui.label("Severity:");
                    if ui
                        .selectable_label(self.alert_filter_severity.is_none(), "All")
                        .clicked()
                    {
                        self.alert_filter_severity = None;
                    }
                    if ui
                        .selectable_label(
                            self.alert_filter_severity == Some(NotificationSeverity::Warning),
                            "Warning+",
                        )
                        .clicked()
                    {
                        self.alert_filter_severity = Some(NotificationSeverity::Warning);
                    }
                    if ui
                        .selectable_label(
                            self.alert_filter_severity == Some(NotificationSeverity::Critical),
                            "Critical+",
                        )
                        .clicked()
                    {
                        self.alert_filter_severity = Some(NotificationSeverity::Critical);
                    }
                });

                ui.add_space(8.0);

                // 3-Column Diagnostic Summary Banner
                ui.columns(3, |columns| {
                    columns[0].group(|ui| {
                        ui.strong("💽 Storage & WAL Health");
                        if let Some(ref report) = self.watchdog_last_check {
                            if let Some(disk) = report.get("disk_stats") {
                                let db_b = disk["db_bytes"].as_u64().unwrap_or(0);
                                let wal_b = disk["wal_bytes"].as_u64().unwrap_or(0);
                                ui.label(format!(
                                    "DB Size: {:.2} MB",
                                    db_b as f64 / (1024.0 * 1024.0)
                                ));
                                ui.label(format!("WAL Size: {:.2} KB", wal_b as f64 / 1024.0));
                            } else {
                                ui.label("In-Memory / Default");
                            }
                        } else {
                            ui.colored_label(muted_gray, "Click 'Run Watchdog Scan' to measure disk");
                        }
                    });

                    columns[1].group(|ui| {
                        ui.strong("⚡ Quiescent Table Churn");
                        ui.label("Monitored: employees, roles, config");
                        ui.colored_label(emerald_bright, "Threshold: ≤10 mut/min");
                        if let Some(ref report) = self.watchdog_last_check {
                            if let Some(stats) =
                                report.get("quiescent_stats").and_then(|v| v.as_object())
                            {
                                if stats.is_empty() {
                                    ui.colored_label(
                                        emerald_bright,
                                        "All static tables quiescent (0 mut/min)",
                                    );
                                } else {
                                    for (tbl, rate) in stats {
                                        ui.label(format!("{}: {} mut/min", tbl, rate));
                                    }
                                }
                            } else {
                                ui.colored_label(emerald_bright, "No churn recorded");
                            }
                        } else {
                            ui.colored_label(muted_gray, "Sliding 60-second window active");
                        }
                    });

                    columns[2].group(|ui| {
                        ui.strong("🛡️ System & Integrity Health");
                        let unack = self.unread_alerts_count;
                        if unack == 0 {
                            ui.colored_label(emerald_bright, "✓ Zero Active Anomalies");
                        } else {
                            ui.colored_label(
                                Color32::from_rgb(239, 68, 68),
                                format!("🚨 {} Unacknowledged Alerts", unack),
                            );
                        }
                        if let Some(ref report) = self.watchdog_last_check {
                            let health = report["health"].as_str().unwrap_or("nominal");
                            ui.label(format!("Watchdog Status: {}", health.to_uppercase()));
                        } else {
                            ui.colored_label(muted_gray, "Awaiting initial manual scan");
                        }
                    });
                });

                ui.add_space(8.0);
                ui.strong("NOTIFICATION FEED & REMEDIATION PIPELINE");
                ui.separator();

                let mut to_ack: Option<String> = None;
                let mut switch_to_doctor = false;
                let mut switch_to_tables = false;
                let mut run_vacuum_now = false;

                let filtered_notifications: Vec<&CraftNotification> = self
                    .notifications
                    .iter()
                    .filter(|n| {
                        if self.alert_filter_unack_only && n.acknowledged {
                            return false;
                        }
                        if let Some(min_sev) = self.alert_filter_severity {
                            if n.severity < min_sev {
                                return false;
                            }
                        }
                        true
                    })
                    .collect();

                if filtered_notifications.is_empty() {
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.colored_label(emerald_bright, "✅");
                            ui.strong("Nominal Operating State:");
                            ui.label(
                                "No active operational alerts or threshold violations detected.",
                            );
                        });
                    });
                } else {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for notif in filtered_notifications {
                            let border_color = match notif.severity {
                                NotificationSeverity::Disaster => Color32::from_rgb(185, 28, 28),
                                NotificationSeverity::Critical => Color32::from_rgb(239, 68, 68),
                                NotificationSeverity::Warning => amber,
                                NotificationSeverity::Info => sky_blue,
                            };
                            let sev_badge = match notif.severity {
                                NotificationSeverity::Disaster => "💥 DISASTER",
                                NotificationSeverity::Critical => "🚨 CRITICAL",
                                NotificationSeverity::Warning => "⚠️ WARNING",
                                NotificationSeverity::Info => "ℹ️ INFO",
                            };

                            egui::Frame::group(ui.style())
                                .stroke(egui::Stroke::new(1.5, border_color))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.colored_label(border_color, sev_badge);
                                        ui.separator();
                                        ui.colored_label(
                                            muted_gray,
                                            format!("[{}]", notif.category.as_str()),
                                        );
                                        ui.strong(&notif.title);
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if notif.acknowledged {
                                                    ui.colored_label(muted_gray, "✓ Acknowledged");
                                                } else if ui.button("✓ Acknowledge").clicked() {
                                                    to_ack = Some(notif.id.clone());
                                                }
                                                ui.colored_label(muted_gray, &notif.timestamp);
                                            },
                                        );
                                    });

                                    ui.label(&notif.message);

                                    if let Some(ref entity) = notif.source_entity {
                                        ui.horizontal(|ui| {
                                            ui.colored_label(muted_gray, "Source Entity:");
                                            ui.monospace(entity);
                                        });
                                    }

                                    if notif.metrics != Value::Null
                                        && !notif
                                            .metrics
                                            .as_object()
                                            .is_none_or(|o| o.is_empty())
                                    {
                                        ui.collapsing("View Anomaly Metrics", |ui| {
                                            ui.monospace(
                                                serde_json::to_string_pretty(&notif.metrics)
                                                    .unwrap_or_else(|_| notif.metrics.to_string()),
                                            );
                                        });
                                    }

                                    if let Some(ref action) = notif.suggested_action {
                                        ui.horizontal(|ui| {
                                            ui.colored_label(sky_blue, "Suggested Remediation:");
                                            ui.strong(action);

                                            let act_lower = action.to_lowercase();
                                            if act_lower.contains("vacuum") {
                                                if ui.button("🧹 Run VACUUM Now").clicked() {
                                                    run_vacuum_now = true;
                                                }
                                            } else if act_lower.contains("doctor")
                                                || act_lower.contains("repair")
                                            {
                                                if ui
                                                    .button("🩺 Jump to Doctor & Auto-Repair")
                                                    .clicked()
                                                {
                                                    switch_to_doctor = true;
                                                }
                                            } else if (act_lower.contains("review")
                                                || act_lower.contains("audit"))
                                                && ui.button("📋 Open Table Viewer").clicked()
                                            {
                                                switch_to_tables = true;
                                            }
                                        });
                                    }
                                });
                            ui.add_space(4.0);
                        }
                    });
                }

                if let Some(id) = to_ack {
                    self.ack_notification(&id);
                }
                if run_vacuum_now {
                    let _ = self
                        .runtime
                        .block_on(dispatch(Command::Vacuum, &self.store));
                    self.status_message =
                        "Vacuum executed successfully from notification remediation action"
                            .to_string();
                    self.run_watchdog_check();
                }
                if switch_to_doctor {
                    self.active_tab = ActiveTab::Doctor;
                    self.run_diagnostics();
                }
                if switch_to_tables {
                    self.active_tab = ActiveTab::Tables;
                }
            }
        });
    }
}
