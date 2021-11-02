use mc_launcher_core::account::AccountInfo;

use crate::error::{SerializedError, Result};
use crate::state::MainState;
use crate::statics::LOGIN_MAX_NUM;

use tauri::State;
use tokio::time::{sleep, Duration};
use tokio::sync::broadcast;

#[tauri::command]
pub async fn login(state: tauri::State<'_, MainState>, index: usize) -> Result<[String; 2]> {
    let mut info = AccountInfo::default();
    let mut abort = state.login_state.login_abort_sender.subscribe();

    tokio::select! {
        _ = info.oauth2_login() => {
            println!("I was invoked from js.");
        },
        _ = sleep(Duration::from_secs(60)) => {
            println!("Login timeout!");
            return Err(SerializedError::from("Login Timeout!"));
        },
        _ = async {
            loop {
                if let Ok(i) = abort.recv().await {
                    if i == index {
                        break;
                    }
                }
            }
        } => return Err(SerializedError::from("Login Aborted!"))
    }
    Ok([info.name, info.uuid]) // seems like tauri can't recognize turple, so return array here
}

#[tauri::command]
pub async fn login_abort(state: State<'_, MainState>, index: usize) -> Result<usize> {
    state.login_state.login_abort_sender.send(index).map_err(SerializedError::from)
}

pub struct LoginState {
    pub login_abort_sender: broadcast::Sender<usize>,
    pub login_abort_receiver: broadcast::Receiver<usize>,
}

impl LoginState {
    pub fn new() -> LoginState {
        let (tx, rx) = broadcast::channel::<usize>(*LOGIN_MAX_NUM);
        LoginState {
            login_abort_sender: tx,
            login_abort_receiver: rx,
        }
    }
}
