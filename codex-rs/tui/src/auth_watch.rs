use crate::app_event::AppEvent;
use crate::app_event_sender::AppEventSender;
use notify::Config;
use notify::Event;
use notify::EventKind;
use notify::RecommendedWatcher;
use notify::RecursiveMode;
use notify::Watcher;
use std::ffi::OsStr;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

const AUTH_WATCH_DEBOUNCE: Duration = Duration::from_millis(250);

pub(crate) struct AuthWatch {
    _watcher: RecommendedWatcher,
}

impl AuthWatch {
    pub(crate) fn start(codex_home: &Path, app_event_tx: AppEventSender) -> notify::Result<Self> {
        let auth_path = codex_home.join("auth.json");
        let auth_file_name = auth_path.file_name().map(OsStr::to_os_string);
        let generation = Arc::new(AtomicU64::new(0));

        let mut watcher = notify::recommended_watcher(move |res| match res {
            Ok(event) => {
                if !is_auth_json_event(&event, auth_path.as_path(), auth_file_name.as_deref()) {
                    return;
                }

                // Trailing debounce: only emit after a short quiet window so we avoid
                // reloading from a partially-written auth.json.
                let event_generation = generation.fetch_add(1, Ordering::SeqCst) + 1;
                let generation = Arc::clone(&generation);
                let app_event_tx = app_event_tx.clone();
                thread::spawn(move || {
                    thread::sleep(AUTH_WATCH_DEBOUNCE);
                    if generation.load(Ordering::SeqCst) == event_generation {
                        app_event_tx.send(AppEvent::AuthFileChanged);
                    }
                });
            }
            Err(err) => {
                tracing::warn!(%err, "auth.json watcher error");
            }
        })?;

        watcher.configure(Config::default())?;
        watcher.watch(codex_home, RecursiveMode::NonRecursive)?;

        Ok(Self { _watcher: watcher })
    }
}

fn is_auth_json_event(event: &Event, auth_path: &Path, auth_file_name: Option<&OsStr>) -> bool {
    if !is_relevant_kind(event.kind) {
        return false;
    }
    event.paths.iter().any(|path| {
        path == auth_path || auth_file_name.is_some_and(|name| path.file_name() == Some(name))
    })
}

fn is_relevant_kind(kind: EventKind) -> bool {
    matches!(
        kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    )
}
