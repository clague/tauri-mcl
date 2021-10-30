use mc_launcher_core::account::AccountInfo;

use crate::error::{SerializedError, Result};
use crate::state::MainState;
use crate::statics::LOGIN_MAX_NUM;

use tauri::State;
use tokio::time::{sleep, Duration};
use tokio::sync::broadcast;

#[tauri::command]
pub async fn login(state: tauri::State<'_, MainState>, index: usize) -> Result<()> {
    let mut info = AccountInfo::default();
    let mut abort = state.login_state.login_abort_sender.subscribe();

    tokio::select! {
        r = info.oauth2_login() => {
            println!("I was invoked from js.");
            return r.map_err(SerializedError::from);
        },
        _ = sleep(Duration::from_secs(60)) => {
            println!("Login timeout!");
            return Err(SerializedError::from("Login Timeout!"));
        }
        i = async {
            loop {
                match abort.recv().await {
                    Ok(i) => return i,
                    Err(_) => {}
                }
            }
        } => {
            if i == index {
                println!("Login Aborted!");
                return Err(SerializedError::from("Login Aborted!"));
            }
        }
    }
    Ok(())
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
