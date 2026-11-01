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
        let res = self.runtime.block_on(dispatch(
            Command::FtsSearch {
                table: "products_fts".to_string(),
                query: text,
                top_k: Some(10),
            },
            &self.store,
        ));
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
                ui.heading(format!(
                    "Table: {}",
                    self.selected_table.as_deref().unwrap_or("None")
                ));
                ui.separator();

                if let Some(ref res) = self.table_rows {
                    if let (Some(cols), Some(rows)) =
                        (res["columns"].as_array(), res["rows"].as_array())
                    {
                        egui::ScrollArea::both().show(ui, |ui| {
                            egui::Grid::new("table_grid").striped(true).show(ui, |ui| {
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
                ui.heading("Database Schema Inspector");
                ui.separator();

                let full_schema = self.runtime.block_on(dispatch(
                    Command::SchemaInspect { table: None },
                    &self.store,
                ));
                egui::ScrollArea::both().show(ui, |ui| {
                    ui.monospace(
                        serde_json::to_string_pretty(&full_schema)
                            .unwrap_or_else(|_| full_schema.to_string()),
                    );
                });
            }
            ActiveTab::Rag => {
                ui.heading("RAG & Semantic Retrieval Tester");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Search Query:");
                    ui.text_edit_singleline(&mut self.rag_text);
                    if ui.button("FTS Search").clicked() {
                        self.run_rag_search();
                    }
                });

                ui.separator();
                if let Some(ref res) = self.rag_results {
                    egui::ScrollArea::both().show(ui, |ui| {
                        ui.monospace(
                            serde_json::to_string_pretty(res).unwrap_or_else(|_| res.to_string()),
                        );
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
