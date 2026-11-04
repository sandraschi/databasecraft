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

use databasecraft_core::{
    dispatch, models::ExportTarget, Command, APP_VERSION, CONTROL_PORT,
};
use databasecraft_store::DatabaseStore;
use eframe::egui::{self, Color32, Vec2};
use serde_json::Value;
use std::sync::Arc;
use std::time::Instant;
use tokio::runtime::Runtime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    Tables,
    Query,
    Schema,
    AiWorkflows,
    Rag,
    Profiler,
    Export,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiSubTab {
    Nlq,
    Dedupe,
    Synthetic,
    Advisor,
    Ingest,
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
    pub rag_text: String,
    pub rag_results: Option<Value>,
    pub export_target: ExportTarget,
    pub export_preview: Option<Value>,
    pub status_message: String,
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
            rag_text: "Obsidian headphones".to_string(),
            rag_results: None,
            export_target: ExportTarget::DesignCraft,
            export_preview: None,
            status_message: format!(
                "DatabaseCraft v{APP_VERSION} initialized on Port {CONTROL_PORT} [Ready]"
            ),
        };

        if let Some(t) = first_table {
            app.load_table_data(&t);
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
            self.status_message = format!("Generated and inserted {count} synthetic rows into '{table}'");
        } else {
            self.status_message = format!("Generated preview of {count} synthetic rows for '{table}'");
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
        self.status_message = "Document chunked and ingested into relational, vector, and FTS tables".to_string();
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
                ui.colored_label(emerald_bright, format!("Port {CONTROL_PORT} [Plane 2 Active]"));
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

                if ui.button("🧹 Vacuum").clicked() {
                    let _ = self.runtime.block_on(dispatch(Command::Vacuum, &self.store));
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
                    let text = egui::RichText::new(label)
                        .color(if active { emerald_bright } else { Color32::WHITE });
                    ui.selectable_label(active, text).clicked()
                };

                if nav_button(ui, self.active_tab == ActiveTab::Tables, "📊 Table Data") {
                    self.active_tab = ActiveTab::Tables;
                }
                if nav_button(ui, self.active_tab == ActiveTab::Query, "⚡ SQL & NLQ Console") {
                    self.active_tab = ActiveTab::Query;
                }
                if nav_button(ui, self.active_tab == ActiveTab::Schema, "🗺️ Schema & ERD") {
                    self.active_tab = ActiveTab::Schema;
                }
                if nav_button(ui, self.active_tab == ActiveTab::AiWorkflows, "🤖 AI Copilot & Workflows") {
                    self.active_tab = ActiveTab::AiWorkflows;
                }
                if nav_button(ui, self.active_tab == ActiveTab::Rag, "🎯 RAG & Vectors") {
                    self.active_tab = ActiveTab::Rag;
                }
                if nav_button(ui, self.active_tab == ActiveTab::Profiler, "📈 Data Profiler") {
                    self.active_tab = ActiveTab::Profiler;
                    self.run_profiler();
                }
                if nav_button(ui, self.active_tab == ActiveTab::Export, "📦 Cross-Craft Merge") {
                    self.active_tab = ActiveTab::Export;
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
        });
    }
}
