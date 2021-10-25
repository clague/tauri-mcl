use tokio::sync::{mpsc, Mutex};

pub struct AsyncRuntime {
    pub login_abort_sender: mpsc::Sender<()>,
    pub login_abort_receiver: Mutex<mpsc::Receiver<()>>,
}

impl AsyncRuntime {
    pub fn new() -> AsyncRuntime {
        let (tx, rx) = mpsc::channel::<()>(10);
        AsyncRuntime {
            login_abort_sender: tx,
            login_abort_receiver: Mutex::new(rx),
        }
    }
}