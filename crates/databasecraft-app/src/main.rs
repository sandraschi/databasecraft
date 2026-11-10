//! L3 DatabaseCraft Native Desktop Application.
//!
//! Follows the ArtCraft in-process architecture:
//! - Direct in-process calls to `databasecraft_core::dispatch`
//! - Zero WebView2 / HTML / JS / CSS cruft
//! - Headless `--smoke` CI verification mode
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

mod gui;

use clap::Parser;
use databasecraft_core::{dispatch, Command, APP_VERSION, CONTROL_PORT};
use databasecraft_store::DatabaseStore;
use eframe::egui;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::runtime::Runtime;

#[derive(Parser, Debug)]
#[command(
    name = "databasecraft",
    about = "DatabaseCraft: Pure-Rust Relational Database and Form Designer",
    version = APP_VERSION
)]
struct Args {
    /// Path to a .dbcraft or .sqlite database file
    #[arg(short, long)]
    file: Option<PathBuf>,

    /// Run smoke test without opening an interactive GUI window (CI verification)
    #[arg(long)]
    smoke: bool,

    /// Bind control port for cross-craft orchestration (default: 14113)
    #[arg(long, default_value_t = CONTROL_PORT)]
    control: u16,
}

pub struct AppState {
    pub store: DatabaseStore,
}

impl AppState {
    pub fn new_in_memory() -> Result<Self, databasecraft_store::StoreError> {
        let store = DatabaseStore::open_in_memory()?;
        Ok(Self { store })
    }

    pub fn new_with_file(path: &std::path::Path) -> Result<Self, databasecraft_store::StoreError> {
        let store = DatabaseStore::open(path)?;
        Ok(Self { store })
    }

    pub async fn execute_command(&self, cmd: Command) -> Value {
        dispatch(cmd, &self.store).await
    }
}

fn main() {
    let args = Args::parse();
    let rt = match Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[databasecraft] Failed to create tokio runtime: {e}");
            std::process::exit(1);
        }
    };

    let app_state = match args.file {
        Some(ref path) => match AppState::new_with_file(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!(
                    "[databasecraft] Could not open database file {:?}: {e}",
                    path
                );
                std::process::exit(1);
            }
        },
        None => match AppState::new_in_memory() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[databasecraft] Could not open in-memory store: {e}");
                std::process::exit(1);
            }
        },
    };

    if args.smoke {
        println!("[databasecraft-app] Running headless smoke verification...");
        let status = rt.block_on(app_state.execute_command(Command::Status));
        println!("[databasecraft-app] Smoke OK: {status}");
        println!(
            "[databasecraft-app] Registered control port: {}",
            args.control
        );
        return;
    }

    println!("============================================================");
    println!("DatabaseCraft v{APP_VERSION} (ArtCraft Suite)");
    println!(
        "Control Port: {} | Launching egui native GUI...",
        args.control
    );
    println!("============================================================");

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(format!("DatabaseCraft v{APP_VERSION}"))
            .with_inner_size([1120.0, 740.0])
            .with_min_inner_size([720.0, 480.0]),
        ..Default::default()
    };

    let rt_arc = Arc::new(rt);
    let store_arc = Arc::new(app_state.store);

    // Spawn Plane 2 Live Control Channel Listener
    let control_port = args.control;
    let control_store = store_arc.clone();
    rt_arc.spawn(async move {
        let addr = format!("127.0.0.1:{control_port}");
        let listener = match tokio::net::TcpListener::bind(&addr).await {
            Ok(l) => {
                eprintln!("[databasecraft-app] Live control channel listening on {addr}");
                l
            }
            Err(e) => {
                eprintln!("[databasecraft-app] Warning: Could not bind control port {addr}: {e}");
                return;
            }
        };

        loop {
            let (mut socket, _) = match listener.accept().await {
                Ok(conn) => conn,
                Err(_) => break,
            };
            let store = control_store.clone();

            tokio::spawn(async move {
                use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
                let (reader, mut writer) = socket.split();
                let mut lines = BufReader::new(reader).lines();

                while let Ok(Some(line)) = lines.next_line().await {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    let resp = if let Ok(req) = serde_json::from_str::<Value>(trimmed) {
                        let method = req["method"].as_str().unwrap_or("");
                        let id = req.get("id").cloned();
                        match method {
                            "initialize" => serde_json::json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "result": {
                                    "protocolVersion": "2024-11-05",
                                    "capabilities": { "tools": { "listChanged": false } },
                                    "serverInfo": { "name": databasecraft_core::APP_NAME, "version": APP_VERSION }
                                }
                            }),
                            "ping" => serde_json::json!({ "jsonrpc": "2.0", "id": id, "result": {} }),
                            "tools/list" => serde_json::json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "result": {
                                    "tools": [
                                        { "name": "db_status", "description": "Status & tables count", "inputSchema": { "type": "object" } },
                                        { "name": "db_query", "description": "Execute SQL", "inputSchema": { "type": "object", "properties": { "sql": { "type": "string" } }, "required": ["sql"] } },
                                        { "name": "db_seed_demo", "description": "Seed demo dataset", "inputSchema": { "type": "object", "properties": { "name": { "type": "string" } }, "required": ["name"] } }
                                    ]
                                }
                            }),
                            "tools/call" => {
                                let params = req.get("params").cloned().unwrap_or(Value::Null);
                                let tool_name = params["name"].as_str().unwrap_or("");
                                let args = params.get("arguments").cloned().unwrap_or(serde_json::json!({}));
                                let cmd = match tool_name {
                                    "db_status" => Some(Command::Status),
                                    "db_query" => Some(Command::Query { sql: args["sql"].as_str().unwrap_or("").to_string() }),
                                    "db_seed_demo" => Some(Command::SeedDemo { name: args["name"].as_str().unwrap_or("").to_string() }),
                                    _ => None,
                                };
                                let res = match cmd {
                                    Some(c) => dispatch(c, &store).await,
                                    None => serde_json::json!({ "success": false, "error": format!("Tool '{tool_name}' not wired on live control socket") }),
                                };
                                serde_json::json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": { "content": [{ "type": "text", "text": res.to_string() }], "isError": false }
                                })
                            }
                            _ => {
                                if let Ok(cmd) = serde_json::from_value::<Command>(req.clone()) {
                                    dispatch(cmd, &store).await
                                } else {
                                    serde_json::json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32601, "message": "Method not found" } })
                                }
                            }
                        }
                    } else {
                        serde_json::json!({ "error": "invalid json" })
                    };

                    let out_line = format!("{}\n", resp);
                    if writer.write_all(out_line.as_bytes()).await.is_err() {
                        break;
                    }
                }
            });
        }
    });

    let res = eframe::run_native(
        "DatabaseCraft",
        native_options,
        Box::new(move |_cc| Ok(Box::new(gui::DatabaseCraftGui::new(store_arc, rt_arc)))),
    );
    if let Err(e) = res {
        eprintln!("[databasecraft] eframe runtime error: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use databasecraft_core::APP_NAME;

    #[tokio::test]
    async fn test_in_process_gui_parity() -> Result<(), Box<dyn std::error::Error>> {
        let state = AppState::new_in_memory()?;
        let gui_res = state.execute_command(Command::Status).await;
        let direct_res = dispatch(Command::Status, &state.store).await;

        assert_eq!(gui_res, direct_res);
        assert_eq!(gui_res["app"], APP_NAME);
        assert_eq!(gui_res["control_port"], CONTROL_PORT);
        Ok(())
    }

    #[test]
    fn test_canvas_gui_integration() -> Result<(), Box<dyn std::error::Error>> {
        use databasecraft_core::canvas::AggFunc;

        let store = Arc::new(DatabaseStore::open_in_memory()?);
        let rt = Arc::new(tokio::runtime::Runtime::new()?);
        rt.block_on(dispatch(
            Command::SeedDemo {
                name: "ecommerce".to_string(),
            },
            &store,
        ));

        let mut gui = gui::DatabaseCraftGui::new(store.clone(), rt.clone());

        assert_eq!(gui.canvas_root_table.as_deref(), Some("categories"));
        assert!(gui.canvas_spec.is_some());
        assert!(gui.canvas_relations.is_some());

        // Sprout products 1:N
        gui.sprout_relation("products", true);
        let sprouted_spec = gui.canvas_spec.as_ref().ok_or("missing canvas_spec")?;
        assert_eq!(sprouted_spec.nested_relations.len(), 1);
        assert_eq!(sprouted_spec.nested_relations[0].relation_name, "products");

        // Add inline rollup: SUM(products.price) as total_value
        gui.add_canvas_aggregation(
            Some("products".to_string()),
            "price".to_string(),
            AggFunc::Sum,
            "total_value".to_string(),
        );

        let res = gui.canvas_result.as_ref().ok_or("missing canvas_result")?;
        assert_eq!(res["success"], true);
        let data = res["result"]["data"].as_array().ok_or("missing data")?;
        assert_eq!(data.len(), 3);

        // Check Category 1: Audio Hardware has total_value $1148 and 2 products
        let cat1 = &data[0];
        assert_eq!(cat1["name"], "Audio Hardware");
        assert_eq!(cat1["total_value"], 1148.0);
        let cat1_products = cat1["products"].as_array().ok_or("missing products")?;
        assert_eq!(cat1_products.len(), 2);

        Ok(())
    }

    #[tokio::test]
    async fn test_control_channel_tcp_loopback() -> Result<(), Box<dyn std::error::Error>> {
        let store = Arc::new(DatabaseStore::open_in_memory()?);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;

        let control_store = store.clone();
        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
                let (reader, mut writer) = socket.split();
                let mut lines = BufReader::new(reader).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    let resp = if let Ok(req) = serde_json::from_str::<Value>(trimmed) {
                        let method = req["method"].as_str().unwrap_or("");
                        let id = req.get("id").cloned();
                        match method {
                            "initialize" => serde_json::json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "result": { "protocolVersion": "2024-11-05" }
                            }),
                            "tools/call" => {
                                let res = dispatch(Command::Status, &control_store).await;
                                serde_json::json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": { "content": [{ "type": "text", "text": res.to_string() }] }
                                })
                            }
                            _ => {
                                serde_json::json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32601, "message": "Method not found" } })
                            }
                        }
                    } else {
                        serde_json::json!({ "error": "invalid json" })
                    };
                    let _ = writer.write_all(format!("{resp}\n").as_bytes()).await;
                }
            }
        });

        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        let mut stream = tokio::net::TcpStream::connect(addr).await?;
        let (reader, mut writer) = stream.split();
        let mut lines = BufReader::new(reader).lines();

        writer
            .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\"}\n")
            .await?;
        let line1 = lines
            .next_line()
            .await?
            .ok_or("no response to initialize")?;
        let resp1: Value = serde_json::from_str(&line1)?;
        assert_eq!(resp1["result"]["protocolVersion"], "2024-11-05");

        writer
            .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/call\",\"params\":{\"name\":\"db_status\"}}\n")
            .await?;
        let line2 = lines
            .next_line()
            .await?
            .ok_or("no response to tools/call")?;
        let resp2: Value = serde_json::from_str(&line2)?;
        assert_eq!(resp2["id"], 2);
        let text = resp2["result"]["content"][0]["text"].as_str().unwrap_or("");
        assert!(text.contains("databasecraft"));
        Ok(())
    }
}
