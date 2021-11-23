#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

mod login;
mod error;
mod state;
mod download;
mod statics;

use std::sync::Arc;
use login::{login, login_abort, get_logged, get_logging, get_active};
use download::download_json;
use state::MainState;
use parking_lot::Mutex;

fn main() {
    let state = Mutex::new(MainState::new());
    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            login,
            login_abort,
            download_json,
            get_logged,
            get_logging,
            get_active,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
