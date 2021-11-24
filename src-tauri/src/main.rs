#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

pub mod login;
pub mod error;
pub mod state;
pub mod download;
pub mod statics;

use crate::login::{login, login_abort, get_logged, get_logging, get_active, set_active, delete_account};
use crate::download::download_json;
use crate::state::MainState;
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
            set_active,
            delete_account,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
