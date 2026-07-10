use std::path::Path;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use std::time::SystemTime;

use crate::app_event::AppEvent;
use crate::app_event_sender::AppEventSender;

const POLL_INTERVAL: Duration = Duration::from_millis(200);
const DEBOUNCE_DELAY: Duration = Duration::from_millis(200);

pub(crate) struct AuthWatch {
    stop_tx: Option<mpsc::Sender<()>>,
    join_handle: Option<JoinHandle<()>>,
}

impl AuthWatch {
    pub(crate) fn start(codex_home: &Path, app_event_tx: AppEventSender) -> Self {
        let (stop_tx, stop_rx) = mpsc::channel();
        let auth_path = codex_home.join("auth.json");
        let join_handle = thread::spawn(move || watch_auth_file(auth_path, app_event_tx, stop_rx));
        Self {
            stop_tx: Some(stop_tx),
            join_handle: Some(join_handle),
        }
    }
}

impl Drop for AuthWatch {
    fn drop(&mut self) {
        if let Some(stop_tx) = self.stop_tx.take() {
            let _ = stop_tx.send(());
        }
        if let Some(join_handle) = self.join_handle.take() {
            let _ = join_handle.join();
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AuthFileState {
    is_file: bool,
    len: u64,
    modified: Option<SystemTime>,
}

fn watch_auth_file(auth_path: PathBuf, app_event_tx: AppEventSender, stop_rx: mpsc::Receiver<()>) {
    let mut previous = auth_file_state(&auth_path);
    loop {
        match stop_rx.recv_timeout(POLL_INTERVAL) {
            Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => return,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        let mut next = auth_file_state(&auth_path);
        if next == previous {
            continue;
        }

        // Wait for a quiet period so atomic replacements and partial writes settle first.
        loop {
            match stop_rx.recv_timeout(DEBOUNCE_DELAY) {
                Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => return,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    let settled = auth_file_state(&auth_path);
                    if settled == next {
                        break;
                    }
                    next = settled;
                }
            }
        }
        previous = next;
        app_event_tx.send(AppEvent::AuthFileChanged);
    }
}

fn auth_file_state(auth_path: &Path) -> AuthFileState {
    match std::fs::metadata(auth_path) {
        Ok(metadata) if metadata.is_file() => AuthFileState {
            is_file: true,
            len: metadata.len(),
            modified: metadata.modified().ok(),
        },
        _ => AuthFileState {
            is_file: false,
            len: 0,
            modified: None,
        },
    }
}
