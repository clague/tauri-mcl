use mc_launcher_core::account::AccountInfo;
use serde::Serialize;

use crate::error::{SerializedError, Result};
use crate::state::MainState;
use crate::statics::LOGIN_MAX_NUM;


use std::collections::HashMap;
use serde_json::Map;
use serde_json::Value;
use tokio::time::{sleep, Duration};
use tokio::sync::broadcast;
use parking_lot::Mutex;
use notify_rust::Notification;

#[tauri::command]
pub async fn login(state: tauri::State<'_, Mutex<MainState>>, index: usize) -> Result<Logged> {
    let mut info = AccountInfo::default();
    let mut abort;
    {
        let mut lock = state.lock();
        abort = lock.login_state.login_abort_sender.subscribe();

        lock.login_state.logging.insert(index, Logging {
            index,
            err_message: String::new(),
        });
        drop(lock);
    }
    if let Err(e) = tokio::select! {
        r = info.oauth2_login() => {
            if let Ok(_) = r {
                println!("Log in successfully");
                Ok(())
            }
            else if let Err(e) = r {
                Err(SerializedError::from(e))
            }
            else { Ok(()) }
        },
        _ = sleep(Duration::from_secs(120)) => {
            println!("Login timeout!");
            Err(SerializedError::from("Login Timeout!"))
        },
        _ = async {
            loop {
                if let Ok(i) = abort.recv().await {
                    if i == index {
                        break;
                    }
                }
            }
        } => Err(SerializedError::from("Login Aborted!"))
    } {
        let mut lock = state.lock();
        lock.login_state.logging.insert(index, Logging {
            index,
            err_message: e.to_string(),
        }); // refresh error message
        drop(lock);
        Notification::new()
            .body(&format!("Failed to log in! Reason: {}", e.to_string()))
            .show()?;
        Err(e)
    }
    else {
        let res = Logged {
            name: info.name.clone(),
            uuid: info.uuid.clone(),
        };
        let mut lock = state.lock();
        lock.login_state.logging.remove(&index);

        if lock.login_state.accounts.is_empty() {
            lock.login_state.active_account = info.uuid.clone();
        }
        lock.login_state.accounts.insert(info.uuid.clone(), info);
        drop(lock);
        Notification::new()
            .body(&format!("User {} has been logged in!", res.name))
            .show()?;
        Ok(res)
    }
}

#[tauri::command]
pub async fn login_abort(state: tauri::State<'_, Mutex<MainState>>, index: usize) -> Result<usize> {
    let mut lock = state.lock();
    lock.login_state.logging.remove(&index);
    lock.login_state.login_abort_sender.send(index).map_err(SerializedError::from)
}

#[tauri::command]
pub async fn get_logged(state: tauri::State<'_, Mutex<MainState>>) -> Result<Map<String, Value>> {
    let mut res = Map::new();
    let lock = state.lock();
    for (key, value) in &lock.login_state.accounts {
        res.insert(key.to_owned(), serde_json::to_value(Logged {
            name: value.name.clone(),
            uuid: value.uuid.clone(),
        })?);
    }
    drop(lock);
    Ok(res)
}

#[tauri::command]
pub async fn get_logging(state: tauri::State<'_, Mutex<MainState>>) -> Result<Map<String, Value>> {
    let mut res = Map::new();
    let lock = state.lock();
    for (key, value) in &lock.login_state.logging {
        res.insert(key.to_string(), serde_json::to_value(value.clone())?);
    }
    drop(lock);
    Ok(res)
}

#[tauri::command]
pub async fn get_active(state: tauri::State<'_, Mutex<MainState>>) -> Result<String> {
    let lock = state.lock();
    Ok(lock.login_state.active_account.clone())
}

#[derive(Clone, Serialize)]
pub struct Logging {
    pub index: usize, // used to recognize
    pub err_message: String,
}

#[derive(Clone, Serialize)]
pub struct Logged {
    pub name: String,
    pub uuid: String,
}

pub struct LoginState {
    pub login_abort_sender: broadcast::Sender<usize>,
    pub login_abort_receiver: broadcast::Receiver<usize>,
    pub logging: HashMap<usize, Logging>,
    pub accounts: HashMap<String, AccountInfo>,
    pub active_account: String, // uuid
}

impl LoginState {
    pub fn new() -> LoginState {
        let (tx, rx) = broadcast::channel::<usize>(*LOGIN_MAX_NUM);
        LoginState {
            login_abort_sender: tx,
            login_abort_receiver: rx,
            logging: HashMap::with_capacity(5),
            accounts: HashMap::with_capacity(5),
            active_account: String::new(),
        }
    }
}
