//! L3 DatabaseCraft Native Desktop GUI.
//!
//! Powered by egui / eframe immediate-mode rendering (60-144 FPS).
//! Calls `databasecraft_core::dispatch` directly in-process with zero WebView2.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use databasecraft_core::{dispatch, models::ExportTarget, Command, APP_VERSION, CONTROL_PORT};
use databasecraft_store::DatabaseStore;
use eframe::egui;
use serde_json::Value;
use std::sync::Arc;
use tokio::runtime::Runtime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    Tables,
    Query,
    Schema,
    Rag,
    Export,
}

pub struct DatabaseCraftGui {
    pub store: Arc<DatabaseStore>,
    pub runtime: Arc<Runtime>,
    pub active_tab: ActiveTab,
    pub tables: Vec<String>,
    pub selected_table: Option<String>,
    pub table_rows: Option<Value>,
    pub filter_text: String,
    pub sql_query: String,
    pub query_result: Option<Value>,
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
            tables,
            selected_table: first_table.clone(),
            table_rows: None,
            filter_text: String::new(),
            sql_query: "SELECT * FROM sqlite_master;".to_string(),
            query_result: None,
            rag_text: "Obsidian headphones".to_string(),
            rag_results: None,
            export_target: ExportTarget::DesignCraft,
            export_preview: None,
            status_message: format!(
                "DatabaseCraft v{APP_VERSION} initialized on Port {CONTROL_PORT}"
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
        let sql = format!("SELECT * FROM \"{table}\" LIMIT 50;");
        let res = self
            .runtime
            .block_on(dispatch(Command::Query { sql }, &self.store));
        self.table_rows = Some(res);
    }

    pub fn execute_sql(&mut self) {
        let sql = self.sql_query.clone();
        let res = self
            .runtime
            .block_on(dispatch(Command::Query { sql }, &self.store));
        if res["success"] == true {
            self.status_message = format!("Query executed: {} rows", res["row_count"]);
        } else {
            self.status_message = format!("SQL error: {}", res["error"]);
        }
        self.query_result = Some(res);
        self.refresh_tables();
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
        // 1. Top Command Bar
        egui::Panel::top("top_bar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading(format!("DatabaseCraft v{APP_VERSION}"));
                ui.separator();
                ui.label(format!("Port: {CONTROL_PORT} [Ready]"));
                ui.separator();

                if ui.button("Seed: Ecommerce").clicked() {
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

                if ui.button("Seed: Film").clicked() {
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

                if ui.button("Seed: KPI").clicked() {
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

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::widgets::global_theme_preference_buttons(ui);
                });
            });
        });

        // 2. Bottom Status Bar
        egui::Panel::bottom("status_bar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status_message);
            });
        });

        // 3. Left Navigation Rail
        egui::Panel::left("nav_rail")
            .min_size(180.0)
            .show(ui, |ui| {
                ui.add_space(6.0);
                ui.heading("Navigation");
                ui.add_space(4.0);

                if ui
                    .selectable_label(self.active_tab == ActiveTab::Tables, "Table Data")
                    .clicked()
                {
                    self.active_tab = ActiveTab::Tables;
                }
                if ui
                    .selectable_label(self.active_tab == ActiveTab::Query, "SQL Console")
                    .clicked()
                {
                    self.active_tab = ActiveTab::Query;
                }
                if ui
                    .selectable_label(self.active_tab == ActiveTab::Schema, "Schema & ERD")
                    .clicked()
                {
                    self.active_tab = ActiveTab::Schema;
                }
                if ui
                    .selectable_label(self.active_tab == ActiveTab::Rag, "RAG & Vector")
                    .clicked()
                {
                    self.active_tab = ActiveTab::Rag;
                }
                if ui
                    .selectable_label(self.active_tab == ActiveTab::Export, "Cross-Craft Export")
                    .clicked()
                {
                    self.active_tab = ActiveTab::Export;
                }

                ui.separator();
                ui.heading(format!("Tables ({})", self.tables.len()));
                ui.add_space(4.0);

                let mut table_to_select = None;
                for t in &self.tables {
                    let is_selected = self.selected_table.as_deref() == Some(t);
                    if ui.selectable_label(is_selected, t).clicked() {
                        table_to_select = Some(t.clone());
                    }
                }
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
                    ui.label("Filter:");
                    ui.text_edit_singleline(&mut self.filter_text);
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

                        ui.label(format!("Showing {} of {} rows", filtered_rows.len(), rows.len()));

                        egui::ScrollArea::both().show(ui, |ui| {
                            egui::Grid::new("table_grid").striped(true).show(ui, |ui| {
                                for col in cols {
                                    ui.strong(col.as_str().unwrap_or(""));
                                }
                                ui.end_row();

                                for row in filtered_rows {
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
                    ui.label("Select a table or click Seed above to populate demo data.");
                }
            }
            ActiveTab::Query => {
                ui.heading("SQL Query Console");
                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("Run Query (Ctrl+Enter)").clicked() {
                        self.execute_sql();
                    }
                });

                ui.add(
                    egui::TextEdit::multiline(&mut self.sql_query)
                        .font(egui::TextStyle::Monospace)
                        .desired_rows(4)
                        .desired_width(f32::INFINITY),
                );

                ui.separator();
                if let Some(ref res) = self.query_result {
                    egui::ScrollArea::both().show(ui, |ui| {
                        ui.monospace(
                            serde_json::to_string_pretty(res).unwrap_or_else(|_| res.to_string()),
                        );
                    });
                }
            }
            ActiveTab::Schema => {
                ui.heading("Database Schema Inspector & Visual ERD");
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
                                    ui.set_min_width(260.0);
                                    let table_name = table["name"].as_str().unwrap_or("unnamed");
                                    ui.horizontal(|ui| {
                                        ui.strong(format!("📋 {table_name}"));
                                        if let Some(cols) = table["columns"].as_array() {
                                            ui.weak(format!("({} cols)", cols.len()));
                                        }
                                    });
                                    ui.separator();

                                    if let Some(cols) = table["columns"].as_array() {
                                        for col in cols {
                                            ui.horizontal(|ui| {
                                                if col["pk"] == true {
                                                    ui.colored_label(egui::Color32::from_rgb(16, 185, 129), "[PK]");
                                                }
                                                ui.label(col["name"].as_str().unwrap_or(""));
                                                ui.weak(col["data_type"].as_str().unwrap_or(""));
                                                if col["notnull"] == true {
                                                    ui.weak("*");
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
                                                ui.colored_label(
                                                    egui::Color32::from_rgb(56, 189, 248),
                                                    format!("↳ {from} → {target_tbl}({to})"),
                                                );
                                            }
                                        }
                                    }
                                });
                                ui.add_space(8.0);
                            }
                        });
                    } else {
                        ui.monospace(
                            serde_json::to_string_pretty(&full_schema)
                                .unwrap_or_else(|_| full_schema.to_string()),
                        );
                    }
                });
            }
            ActiveTab::Rag => {
                ui.heading("RAG & In-Process Semantic Retrieval Workbench");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Semantic Query:");
                    ui.text_edit_singleline(&mut self.rag_text);
                    if ui.button("⚡ Semantic Search").clicked() {
                        self.run_rag_search();
                    }
                });

                ui.separator();
                if let Some(ref res) = self.rag_results {
                    egui::ScrollArea::both().show(ui, |ui| {
                        if let Some(hits) = res["hits"].as_array() {
                            ui.label(format!("Retrieved {} semantic hits (mode: {}):", hits.len(), res["mode"].as_str().unwrap_or("hybrid")));
                            for (i, hit) in hits.iter().enumerate() {
                                egui::Frame::group(ui.style()).show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.strong(format!("#{}: ID {}", i + 1, hit["id"]));
                                        let score = hit.get("rrf_score")
                                            .or_else(|| hit.get("score"))
                                            .and_then(|v| v.as_f64())
                                            .unwrap_or(0.0);
                                        ui.colored_label(egui::Color32::from_rgb(16, 185, 129), format!("Score: {:.4}", score));
                                    });
                                    if !hit["metadata"].is_null() {
                                        ui.monospace(serde_json::to_string_pretty(&hit["metadata"]).unwrap_or_default());
                                    }
                                });
                                ui.add_space(4.0);
                            }
                        } else {
                            ui.monospace(
                                serde_json::to_string_pretty(res).unwrap_or_else(|_| res.to_string()),
                            );
                        }
                    });
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

                    if ui.button("Generate Merge Payload").clicked() {
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
