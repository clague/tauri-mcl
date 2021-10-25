#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

mod login;
mod error;
mod runtime;

use crate::runtime::AsyncRuntime;

fn main() {
    let login_runtime = AsyncRuntime::new();

    tauri::Builder::default()
        .manage(login_runtime)
        .invoke_handler(tauri::generate_handler![
            commands::login,
            commands::login_abort,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
