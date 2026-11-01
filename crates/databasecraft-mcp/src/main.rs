//! L2 DatabaseCraft CLI and MCP Stdio Server.
//!
//! Stdout is 100% pure JSON-RPC in MCP mode. All diagnostics go to stderr.
//! Both CLI subcommands and MCP tools dispatch identically through `databasecraft_core::dispatch`.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use clap::{Parser, Subcommand};
use databasecraft_core::{
    command_registry, describe_command, dispatch, models::ExportTarget, Command, APP_NAME,
    APP_VERSION, CONTROL_PORT,
};
use databasecraft_store::DatabaseStore;
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::path::Path;
use tokio::runtime::Runtime;

#[derive(Parser)]
#[command(
    name = "databasecraft-cli",
    about = "Pure-Rust Relational Database engine for the ArtCraft suite (CLI-first, MCP over stdio)",
    version = APP_VERSION
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show DatabaseCraft status, tables count, and registered control port
    Status {
        #[arg(long)]
        db: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Execute a SQL query and display or return JSON rows
    Query {
        #[arg(long)]
        sql: String,
        #[arg(long)]
        db: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Inspect database schema (all tables or a single table)
    SchemaInspect {
        #[arg(long)]
        table: Option<String>,
        #[arg(long)]
        db: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Insert a record from a JSON string into a table
    RecordInsert {
        #[arg(long)]
        table: String,
        #[arg(long)]
        record: String,
        #[arg(long)]
        db: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Export relational query directly formatted for DesignCraft, DeckCraft, WordCraft, GridCraft, or PdfCraft
    ExportMerge {
        #[arg(long)]
        query: String,
        #[arg(long)]
        target: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        out: Option<String>,
        #[arg(long)]
        db: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Seed a deterministic demo dataset (ecommerce, film, kpi)
    SeedDemo {
        #[arg(long)]
        name: String,
        #[arg(long)]
        db: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Generate a deterministic semantic vector embedding from text
    Embed {
        text: String,
        #[arg(long, default_value = "128")]
        dimensions: usize,
        #[arg(long)]
        json: bool,
    },
    /// Execute natural language semantic search (in-process embedding + vector / hybrid RRF)
    SemanticSearch {
        query: String,
        #[arg(long)]
        vector_table: String,
        #[arg(long)]
        fts_table: Option<String>,
        #[arg(long, default_value = "128")]
        dimensions: usize,
        #[arg(long, default_value = "10")]
        top_k: usize,
        #[arg(long)]
        db: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// List all registered commands in the Craft command bus (id, label, signature)
    #[command(name = "commands")]
    Registry {
        #[arg(long)]
        json: bool,
    },
    /// Inspect parameter signature and description for a specific command ID
    Describe {
        command_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Launch the MCP (Model Context Protocol 2024-11-05) stdio server (Plane 1 headless or Plane 2 --attach)
    Mcp {
        #[arg(long)]
        db: Option<String>,
        /// Attach live to a running DatabaseCraft GUI on control port (Plane 2)
        #[arg(long)]
        attach: Option<u16>,
    },
}

fn open_store(db_path: Option<&str>) -> DatabaseStore {
    match db_path {
        Some(p) => match DatabaseStore::open(Path::new(p)) {
            Ok(store) => store,
            Err(e) => {
                eprintln!("[databasecraft] Failed to open database at '{p}': {e}. Falling back to in-memory.");
                DatabaseStore::open_in_memory().unwrap_or_else(|_| {
                    eprintln!("[databasecraft] Fatal: could not initialize store.");
                    std::process::exit(1);
                })
            }
        },
        None => DatabaseStore::open_in_memory().unwrap_or_else(|_| {
            eprintln!("[databasecraft] Fatal: could not initialize in-memory store.");
            std::process::exit(1);
        }),
    }
}

fn parse_target(s: &str) -> Option<ExportTarget> {
    match s.to_ascii_lowercase().as_str() {
        "design" | "designcraft" => Some(ExportTarget::DesignCraft),
        "deck" | "deckcraft" => Some(ExportTarget::DeckCraft),
        "word" | "wordcraft" => Some(ExportTarget::WordCraft),
        "grid" | "gridcraft" => Some(ExportTarget::GridCraft),
        "pdf" | "pdfcraft" => Some(ExportTarget::PdfCraft),
        _ => None,
    }
}

fn main() {
    let cli = Cli::parse();
    let rt = match Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[databasecraft] Failed to start tokio runtime: {e}");
            std::process::exit(1);
        }
    };

    match cli.command {
        Commands::Status { db, json } => {
            let store = open_store(db.as_deref());
            let res = rt.block_on(dispatch(Command::Status, &store));
            if json {
                println!("{res}");
            } else {
                println!("DatabaseCraft v{APP_VERSION} (Control Port: {CONTROL_PORT})");
                println!("Status: ready");
                println!("Tables count: {}", res["tables_count"]);
                if let Some(tables) = res["tables"].as_array() {
                    for t in tables {
                        println!(" - {}", t.as_str().unwrap_or(""));
                    }
                }
            }
        }
        Commands::Query { sql, db, json } => {
            let store = open_store(db.as_deref());
            let res = rt.block_on(dispatch(Command::Query { sql }, &store));
            if json {
                println!("{res}");
            } else if res["success"] == true {
                println!("Rows: {}", res["row_count"]);
                println!("{res}");
            } else {
                eprintln!("Error: {}", res["error"]);
            }
        }
        Commands::SchemaInspect { table, db, json } => {
            let store = open_store(db.as_deref());
            let res = rt.block_on(dispatch(Command::SchemaInspect { table }, &store));
            if json {
                println!("{res}");
            } else {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&res).unwrap_or_else(|_| res.to_string())
                );
            }
        }
        Commands::RecordInsert {
            table,
            record,
            db,
            json: _,
        } => {
            let store = open_store(db.as_deref());
            let rec_val: Value = match serde_json::from_str(&record) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Invalid record JSON: {e}");
                    return;
                }
            };
            let res = rt.block_on(dispatch(
                Command::RecordInsert {
                    table,
                    record: rec_val,
                },
                &store,
            ));
            println!("{res}");
        }
        Commands::ExportMerge {
            query,
            target,
            name,
            out,
            db,
            json: _,
        } => {
            let store = open_store(db.as_deref());
            let parsed = match parse_target(&target) {
                Some(t) => t,
                None => {
                    eprintln!("Unknown export target '{target}'. Valid targets: designcraft, deckcraft, wordcraft, gridcraft, pdfcraft");
                    return;
                }
            };
            let res = rt.block_on(dispatch(
                Command::ExportMerge {
                    query,
                    target: parsed,
                    name,
                },
                &store,
            ));
            if let Some(out_path) = out {
                let formatted =
                    serde_json::to_string_pretty(&res).unwrap_or_else(|_| "{}".to_string());
                if let Err(e) = std::fs::write(&out_path, formatted) {
                    eprintln!("Failed to write export to {out_path}: {e}");
                } else {
                    println!("Export written successfully to {out_path}");
                }
            } else {
                println!("{res}");
            }
        }
        Commands::SeedDemo { name, db, json } => {
            let store = open_store(db.as_deref());
            let res = rt.block_on(dispatch(Command::SeedDemo { name }, &store));
            if json {
                println!("{res}");
            } else {
                println!("Seeded dataset: {}", res["seeded"]);
                println!("Tables now available: {}", res["tables"]);
            }
        }
        Commands::Embed {
            text,
            dimensions,
            json,
        } => {
            let store = DatabaseStore::open_in_memory().unwrap_or_else(|_| {
                eprintln!("Failed to open in-memory store");
                std::process::exit(1);
            });
            let res = rt.block_on(dispatch(
                Command::RagEmbed {
                    text,
                    dimensions: Some(dimensions),
                },
                &store,
            ));
            if json {
                println!("{res}");
            } else {
                println!("Text:       {}", res["text"]);
                println!("Dimensions: {}", res["dimensions"]);
                if let Some(arr) = res["vector"].as_array() {
                    let preview: Vec<String> = arr
                        .iter()
                        .take(5)
                        .map(|v| format!("{:.4}", v.as_f64().unwrap_or(0.0)))
                        .collect();
                    println!("Vector:     [{}, ...] (length: {})", preview.join(", "), arr.len());
                }
            }
        }
        Commands::SemanticSearch {
            query,
            vector_table,
            fts_table,
            dimensions,
            top_k,
            db,
            json,
        } => {
            let store = open_store(db.as_deref());
            let res = rt.block_on(dispatch(
                Command::RagAsk {
                    vector_table,
                    fts_table,
                    query,
                    dimensions: Some(dimensions),
                    top_k: Some(top_k),
                },
                &store,
            ));
            if json {
                println!("{res}");
            } else {
                println!("Semantic Search Query: {}", res["query"]);
                println!("Mode:  {}", res["mode"]);
                println!("Hits:  {}", res["count"]);
                if let Some(hits) = res["hits"].as_array() {
                    for (i, hit) in hits.iter().enumerate() {
                        println!(
                            "  #{}: ID={} | Score={:.4} | Meta={}",
                            i + 1,
                            hit["id"],
                            hit.get("rrf_score").or_else(|| hit.get("score")).and_then(|v| v.as_f64()).unwrap_or(0.0),
                            hit["metadata"]
                        );
                    }
                }
            }
        }
        Commands::Registry { json } => {
            let registry = command_registry();
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&registry).unwrap_or_else(|_| "[]".to_string())
                );
            } else {
                println!(
                    "DatabaseCraft Command Bus Registry ({} commands):",
                    registry.len()
                );
                for cmd in registry {
                    println!("  {:20} - {}", cmd.id, cmd.description);
                }
            }
        }
        Commands::Describe { command_id, json } => match describe_command(&command_id) {
            Some(desc) => {
                if json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&desc).unwrap_or_else(|_| "{}".to_string())
                    );
                } else {
                    println!("Command: {}", desc.id);
                    println!("Label:   {}", desc.label);
                    println!("Details: {}", desc.description);
                    println!("Params:  {}", desc.params);
                }
            }
            None => {
                eprintln!(
                    "Unknown command ID: '{command_id}'. Run 'databasecraft-cli commands' to list all commands."
                );
                std::process::exit(1);
            }
        },
        Commands::Mcp { db, attach } => {
            if let Some(port) = attach {
                run_mcp_attach(port);
            } else {
                let store = open_store(db.as_deref());
                run_mcp_stdio(&rt, &store);
            }
        }
    }
}

pub fn tools_list() -> Value {
    json!([
        {
            "name": "db_status",
            "description": "Get DatabaseCraft status, table count, and registered control port 14113",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        },
        {
            "name": "db_query",
            "description": "Execute a SQL query against the DatabaseCraft store and retrieve structured records",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "sql": { "type": "string", "description": "SQL statement (SELECT, INSERT, UPDATE, etc.)" }
                },
                "required": ["sql"]
            }
        },
        {
            "name": "db_schema_inspect",
            "description": "Inspect database schema, foreign keys, and column definitions across all tables or a specific table",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "table": { "type": "string", "description": "Optional table name. Omit to inspect entire database schema" }
                }
            }
        },
        {
            "name": "db_record_insert",
            "description": "Insert a structured JSON record into a table",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "table": { "type": "string", "description": "Target table name" },
                    "record": { "type": "object", "description": "Key-value mapping of column values" }
                },
                "required": ["table", "record"]
            }
        },
        {
            "name": "db_export_merge",
            "description": "Query database and format rows for another Craft family member (designcraft, deckcraft, wordcraft, gridcraft, pdfcraft)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "SQL query producing records to format" },
                    "target": { "type": "string", "description": "Target craft: designcraft, deckcraft, wordcraft, gridcraft, or pdfcraft" },
                    "name": { "type": "string", "description": "Dataset or document title identifier" }
                },
                "required": ["query", "target"]
            }
        },
        {
            "name": "db_seed_demo",
            "description": "Seed deterministic demo dataset (ecommerce, film, kpi) for testing multi-app flows",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Demo dataset name: ecommerce, film, or kpi" }
                },
                "required": ["name"]
            }
        },
        {
            "name": "db_vector_search",
            "description": "Execute a dense vector similarity search (cosine, dot_product, l2) over an embedded vector table",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "table": { "type": "string", "description": "Vector table name" },
                    "vector": { "type": "array", "items": { "type": "number" }, "description": "Dense query vector" },
                    "metric": { "type": "string", "description": "Distance metric: cosine (default), dot_product, or l2" },
                    "top_k": { "type": "integer", "description": "Maximum number of results to return (default: 10)" }
                },
                "required": ["table", "vector"]
            }
        },
        {
            "name": "db_hybrid_search",
            "description": "Execute a hybrid RAG search combining dense vectors and sparse BM25 text keywords with Reciprocal Rank Fusion",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "vector_table": { "type": "string", "description": "Dense vector table name" },
                    "fts_table": { "type": "string", "description": "SQLite FTS5 full-text table name" },
                    "query_text": { "type": "string", "description": "Natural language or keyword query string" },
                    "query_vector": { "type": "array", "items": { "type": "number" }, "description": "Dense query vector" },
                    "top_k": { "type": "integer", "description": "Maximum number of fused results to return (default: 10)" }
                },
                "required": ["vector_table", "fts_table", "query_text", "query_vector"]
            }
        },
        {
            "name": "db_rag_embed",
            "description": "Generate deterministic in-process semantic float vector embedding from text string",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "text": { "type": "string", "description": "Text to embed" },
                    "dimensions": { "type": "integer", "description": "Vector dimensionality (default: 128)" }
                },
                "required": ["text"]
            }
        },
        {
            "name": "db_semantic_search",
            "description": "Execute end-to-end semantic search using natural language text query (in-process embedding + hybrid RRF or vector search)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "vector_table": { "type": "string", "description": "Dense vector table name" },
                    "fts_table": { "type": "string", "description": "Optional FTS5 table name for hybrid search" },
                    "query": { "type": "string", "description": "Natural language query text" },
                    "dimensions": { "type": "integer", "description": "Vector dimensionality (default: 128)" },
                    "top_k": { "type": "integer", "description": "Maximum number of results to return (default: 10)" }
                },
                "required": ["vector_table", "query"]
            }
        }
    ])
}

pub fn call_tool(rt: &Runtime, store: &DatabaseStore, name: &str, args: &Value) -> Value {
    match name {
        "db_status" => rt.block_on(dispatch(Command::Status, store)),
        "db_query" => {
            let sql = args["sql"].as_str().unwrap_or("").to_string();
            rt.block_on(dispatch(Command::Query { sql }, store))
        }
        "db_schema_inspect" => {
            let table = args["table"].as_str().map(|s| s.to_string());
            rt.block_on(dispatch(Command::SchemaInspect { table }, store))
        }
        "db_record_insert" => {
            let table = args["table"].as_str().unwrap_or("").to_string();
            let record = args["record"].clone();
            rt.block_on(dispatch(Command::RecordInsert { table, record }, store))
        }
        "db_export_merge" => {
            let query = args["query"].as_str().unwrap_or("").to_string();
            let target_str = args["target"].as_str().unwrap_or("");
            let name = args["name"].as_str().map(|s| s.to_string());
            match parse_target(target_str) {
                Some(target) => rt.block_on(dispatch(
                    Command::ExportMerge {
                        query,
                        target,
                        name,
                    },
                    store,
                )),
                None => {
                    json!({ "success": false, "error": format!("Invalid export target '{target_str}'") })
                }
            }
        }
        "db_seed_demo" => {
            let name = args["name"].as_str().unwrap_or("").to_string();
            rt.block_on(dispatch(Command::SeedDemo { name }, store))
        }
        "db_vector_search" => {
            let table = args["table"].as_str().unwrap_or("").to_string();
            let vector = args["vector"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_f64().map(|f| f as f32))
                        .collect()
                })
                .unwrap_or_default();
            let metric_str = args["metric"].as_str().unwrap_or("cosine");
            let metric = match metric_str {
                "dot_product" => Some(databasecraft_store::VectorMetric::DotProduct),
                "l2" => Some(databasecraft_store::VectorMetric::L2),
                _ => Some(databasecraft_store::VectorMetric::Cosine),
            };
            let top_k = args["top_k"].as_u64().map(|n| n as usize);
            rt.block_on(dispatch(
                Command::VectorSearch {
                    table,
                    vector,
                    metric,
                    top_k,
                },
                store,
            ))
        }
        "db_hybrid_search" => {
            let vector_table = args["vector_table"].as_str().unwrap_or("").to_string();
            let fts_table = args["fts_table"].as_str().unwrap_or("").to_string();
            let query_text = args["query_text"].as_str().unwrap_or("").to_string();
            let query_vector = args["query_vector"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_f64().map(|f| f as f32))
                        .collect()
                })
                .unwrap_or_default();
            let top_k = args["top_k"].as_u64().map(|n| n as usize);
            rt.block_on(dispatch(
                Command::HybridSearch {
                    vector_table,
                    fts_table,
                    query_text,
                    query_vector,
                    top_k,
                },
                store,
            ))
        }
        "db_rag_embed" => {
            let text = args["text"].as_str().unwrap_or("").to_string();
            let dimensions = args["dimensions"].as_u64().map(|d| d as usize);
            rt.block_on(dispatch(Command::RagEmbed { text, dimensions }, store))
        }
        "db_semantic_search" => {
            let vector_table = args["vector_table"].as_str().unwrap_or("").to_string();
            let fts_table = args["fts_table"].as_str().map(|s| s.to_string());
            let query = args["query"].as_str().unwrap_or("").to_string();
            let dimensions = args["dimensions"].as_u64().map(|d| d as usize);
            let top_k = args["top_k"].as_u64().map(|k| k as usize);
            rt.block_on(dispatch(
                Command::RagAsk {
                    vector_table,
                    fts_table,
                    query,
                    dimensions,
                    top_k,
                },
                store,
            ))
        }
        unknown => json!({ "success": false, "error": format!("Unknown tool '{unknown}'") }),
    }
}

/// Hand-rolled MCP 2024-11-05 line-delimited stdio framing loop.
/// Stdio invariant: stdout is 100% JSON-RPC. Diagnostics go to stderr.
fn run_mcp_stdio(rt: &Runtime, store: &DatabaseStore) {
    eprintln!("[databasecraft-mcp] Starting MCP server stdio loop (MCP 2024-11-05)");
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    for line in stdin.lock().lines() {
        let text = match line {
            Ok(t) => t,
            Err(_) => break,
        };
        let trimmed = text.trim();
        if trimmed.is_empty() {
            continue;
        }

        let req: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[databasecraft-mcp] JSON parse error: {e}");
                continue;
            }
        };

        let method = req["method"].as_str().unwrap_or("");
        let id = req.get("id").cloned();

        let response = match method {
            "initialize" => {
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": {
                            "tools": { "listChanged": false }
                        },
                        "serverInfo": {
                            "name": APP_NAME,
                            "version": APP_VERSION
                        }
                    }
                })
            }
            "notifications/initialized" => {
                // Client acknowledgment notification: no response required
                continue;
            }
            "ping" => {
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {}
                })
            }
            "tools/list" => {
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "tools": tools_list()
                    }
                })
            }
            "tools/call" => {
                let params = req.get("params").cloned().unwrap_or(Value::Null);
                let tool_name = params["name"].as_str().unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or(json!({}));
                let tool_res = call_tool(rt, store, tool_name, &args);

                let is_error = tool_res
                    .get("success")
                    .and_then(|v| v.as_bool())
                    .map(|s| !s)
                    .unwrap_or(false);

                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": [
                            {
                                "type": "text",
                                "text": tool_res.to_string()
                            }
                        ],
                        "isError": is_error
                    }
                })
            }
            _ => {
                if id.is_some() {
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {
                            "code": -32601,
                            "message": format!("Method not found: {method}")
                        }
                    })
                } else {
                    continue;
                }
            }
        };

        let json_line = response.to_string();
        if let Err(e) = writeln!(stdout, "{json_line}") {
            eprintln!("[databasecraft-mcp] stdout write error: {e}");
            break;
        }
        let _ = stdout.flush();
    }
}

/// Attach over TCP to a live DatabaseCraft GUI on control port (Plane 2).
/// Relays MCP stdio to/from the live app's control socket.
fn run_mcp_attach(port: u16) {
    eprintln!(
        "[databasecraft-mcp] Attaching live to DatabaseCraft on control port {port} (Plane 2)..."
    );
    let mut tcp_stream = match std::net::TcpStream::connect(("127.0.0.1", port)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "[databasecraft-mcp] Failed to connect to control port {port}: {e}. Ensure databasecraft.exe --control {port} is running."
            );
            std::process::exit(1);
        }
    };
    eprintln!("[databasecraft-mcp] Attached to live control channel. Relaying MCP stdio.");

    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let mut tcp_reader = std::io::BufReader::new(match tcp_stream.try_clone() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[databasecraft-mcp] Socket clone error: {e}");
            std::process::exit(1);
        }
    });

    for line in stdin.lock().lines() {
        let text = match line {
            Ok(t) => t,
            Err(_) => break,
        };
        let trimmed = text.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Err(e) = writeln!(tcp_stream, "{trimmed}") {
            eprintln!("[databasecraft-mcp] Failed to send to control port: {e}");
            break;
        }
        let _ = tcp_stream.flush();

        let mut response_line = String::new();
        if let Err(e) = tcp_reader.read_line(&mut response_line) {
            eprintln!("[databasecraft-mcp] Failed to read from control port: {e}");
            break;
        }
        if let Err(e) = write!(stdout, "{response_line}") {
            eprintln!("[databasecraft-mcp] stdout write error: {e}");
            break;
        }
        let _ = stdout.flush();
    }
}
