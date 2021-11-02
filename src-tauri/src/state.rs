use crate::{download::DownloadState, login::LoginState};

pub struct MainState {
    pub login_state: LoginState,
    pub download_state: DownloadState,
}

impl MainState {
    pub fn new() -> MainState {
        MainState {
            login_state: LoginState::new(),
            download_state: DownloadState::new(),
        }
    }
}