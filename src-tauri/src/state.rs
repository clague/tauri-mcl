use tokio::sync::{mpsc, Mutex};

use crate::login::LoginState;

pub struct MainState {
    pub login_state: LoginState,
}

impl MainState {
    pub fn new() -> MainState {
        MainState {
            login_state: LoginState::new(),

        }
    }
}