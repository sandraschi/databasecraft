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
}
