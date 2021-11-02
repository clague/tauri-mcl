#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

mod login;
mod error;
mod state;
mod download;
mod statics;

use login::{login, login_abort};
use download::download_json;
use state::MainState;

fn main() {
    let state = MainState::new();
    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            login,
            login_abort,
            download_json,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
