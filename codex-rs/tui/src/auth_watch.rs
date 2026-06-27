use std::path::Path;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use std::time::SystemTime;

use notify::Config;
use notify::EventKind;
use notify::RecursiveMode;
use notify::Watcher;

use crate::app_event::AppEvent;
use crate::app_event_sender::AppEventSender;

const AUTH_POLL_INTERVAL: Duration = Duration::from_secs(1);

pub(crate) struct AuthWatch {
    stop_tx: Option<mpsc::Sender<()>>,
    join_handle: Option<JoinHandle<()>>,
}

impl AuthWatch {
    pub(crate) fn start(codex_home: &Path, app_event_tx: AppEventSender) -> Self {
        let auth_path = codex_home.join("auth.json");
        let (stop_tx, stop_rx) = mpsc::channel();
        let join_handle = thread::spawn(move || poll_auth_file(auth_path, app_event_tx, stop_rx));
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct AuthFileState {
    is_file: bool,
    len: u64,
    modified: Option<SystemTime>,
}

fn poll_auth_file(auth_path: PathBuf, app_event_tx: AppEventSender, stop_rx: mpsc::Receiver<()>) {
    let mut last_state = auth_file_state(auth_path.as_path());
    loop {
        if last_state.is_file {
            let Some(next_state) = watch_existing_auth_file(
                auth_path.as_path(),
                &app_event_tx,
                &stop_rx,
                &last_state,
            ) else {
                break;
            };
            last_state = next_state;
            continue;
        }

        match stop_rx.recv_timeout(AUTH_POLL_INTERVAL) {
            Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }

        let next_state = auth_file_state(auth_path.as_path());
        if next_state != last_state {
            last_state = next_state;
            app_event_tx.send(AppEvent::AuthFileChanged);
        }
    }
}

fn watch_existing_auth_file(
    auth_path: &Path,
    app_event_tx: &AppEventSender,
    stop_rx: &mpsc::Receiver<()>,
    previous_state: &AuthFileState,
) -> Option<AuthFileState> {
    let (watch_tx, watch_rx) = mpsc::channel();
    let mut watcher = match notify::recommended_watcher(move |res| {
        let _ = watch_tx.send(res);
    }) {
        Ok(watcher) => watcher,
        Err(_err) => return poll_for_state_change(auth_path, app_event_tx, stop_rx, previous_state),
    };
    if watcher.configure(Config::default()).is_err() {
        return poll_for_state_change(auth_path, app_event_tx, stop_rx, previous_state);
    }
    if watcher
        .watch(auth_path, RecursiveMode::NonRecursive)
        .is_err()
    {
        return poll_for_state_change(auth_path, app_event_tx, stop_rx, previous_state);
    }

    loop {
        match stop_rx.try_recv() {
            Ok(()) | Err(mpsc::TryRecvError::Disconnected) => return None,
            Err(mpsc::TryRecvError::Empty) => {}
        }

        match watch_rx.recv_timeout(AUTH_POLL_INTERVAL) {
            Ok(Ok(event)) => {
                if !is_relevant_kind(event.kind) {
                    continue;
                }
                let next_state = auth_file_state(auth_path);
                if &next_state != previous_state {
                    app_event_tx.send(AppEvent::AuthFileChanged);
                }
                return Some(next_state);
            }
            Ok(Err(_err)) => return Some(auth_file_state(auth_path)),
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => return Some(auth_file_state(auth_path)),
        }
    }
}

fn poll_for_state_change(
    auth_path: &Path,
    app_event_tx: &AppEventSender,
    stop_rx: &mpsc::Receiver<()>,
    previous_state: &AuthFileState,
) -> Option<AuthFileState> {
    loop {
        match stop_rx.recv_timeout(AUTH_POLL_INTERVAL) {
            Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => return None,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }

        let next_state = auth_file_state(auth_path);
        if &next_state != previous_state {
            app_event_tx.send(AppEvent::AuthFileChanged);
            return Some(next_state);
        }
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

fn is_relevant_kind(kind: EventKind) -> bool {
    matches!(
        kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    )
}
