use mc_launcher_core::account::AccountInfo;

use crate::error::{SerializedError, Result};
use crate::runtime::AsyncRuntime;

use tauri::State;
use tokio;

#[tauri::command]
pub async fn login(runtime: State<'_, AsyncRuntime>) -> Result<()> {
    let mut info = AccountInfo::default();
    let mut rx = runtime.login_abort_receiver.lock().await;
    
    tokio::select! {
        r = info.oauth2_login() => {
            println!("I was invoked from js.");
            r.map_err(SerializedError::from)
        },
        _ = rx.recv() => {
            Err(SerializedError::from("Abort!!"))
        }
    }
}

#[tauri::command]
pub async fn login_abort(runtime: State<'_, AsyncRuntime>) -> Result<()> {
    runtime.login_abort_sender.send(()).await.map_err(SerializedError::from)
}