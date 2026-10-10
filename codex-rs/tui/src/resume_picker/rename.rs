use codex_protocol::ThreadId;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;

use super::PickerLoadRequest;
use super::PickerState;
use super::SessionSelection;
use super::SessionStatus;
use super::archive::ArchiveState;
use crate::key_hint::KeyBindingListExt;
use crate::keymap::KeymapContext;

/// Inline rename session inside the picker.
///
/// `Editing` owns the draft until the user accepts or cancels; `Saving` is the
/// in-flight `thread/name/set` request, matched by thread id and name so a
/// stale response cannot clobber a newer edit.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) enum RenameState {
    #[default]
    Idle,
    Editing { thread_id: ThreadId, draft: String },
    Saving { thread_id: ThreadId, name: String },
}

fn normalize_name(name: &str) -> Option<String> {
    let trimmed = name.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

impl PickerState {
    /// Whether Ctrl+R may claim the rename shortcut. Archived threads stay
    /// read-only, and a user keymap that already binds Ctrl+R keeps precedence.
    pub(super) fn rename_shortcut_available(&self) -> bool {
        if self.status == SessionStatus::Archived {
            return false;
        }
        let rename_key = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL);
        self.keymap.list.action_for(rename_key).is_none()
            && !self.keymap.chords.bindings.iter().any(|binding| {
                binding.action.context == KeymapContext::List
                    && binding.chord.prefix.is_press(rename_key)
            })
    }

    /// Begin an inline rename for the selected row.
    pub(super) fn begin_rename_selected(&mut self) {
        if !matches!(self.rename_state, RenameState::Idle)
            || !matches!(self.archive_state, ArchiveState::Idle)
        {
            return;
        }
        let Some(row) = self.filtered_rows.get(self.selected) else {
            return;
        };
        let Some(thread_id) = row.thread_id else {
            self.inline_error = Some(String::from("Selected session cannot be renamed."));
            self.request_frame();
            return;
        };
        self.rename_state = RenameState::Editing {
            thread_id,
            draft: String::new(),
        };
        self.request_frame();
    }

    /// Route a key while a rename edit is active. Owns all input except the
    /// global Ctrl+C exit; returns `Some` only when the picker should exit.
    pub(super) fn handle_rename_key(&mut self, key: KeyEvent) -> Option<SessionSelection> {
        if matches!(key,
            KeyEvent {
                code: KeyCode::Char('c'),
                modifiers,
                ..
            } if modifiers.contains(KeyModifiers::CONTROL))
        {
            return Some(SessionSelection::Exit);
        }

        match std::mem::take(&mut self.rename_state) {
            RenameState::Editing {
                thread_id,
                mut draft,
            } => {
                if self.keymap.list.cancel.is_pressed(key) {
                    self.rename_state = RenameState::Idle;
                } else if self.keymap.list.accept.is_pressed(key) {
                    match normalize_name(&draft) {
                        Some(name) => {
                            self.rename_state = RenameState::Saving {
                                thread_id,
                                name: name.clone(),
                            };
                            (self.picker_loader)(PickerLoadRequest::Rename { thread_id, name });
                        }
                        None => {
                            self.rename_state = RenameState::Editing { thread_id, draft };
                            self.inline_error =
                                Some(String::from("Thread name cannot be empty."));
                        }
                    }
                } else {
                    match key {
                        KeyEvent {
                            code: KeyCode::Backspace,
                            ..
                        } => {
                            draft.pop();
                        }
                        KeyEvent {
                            code: KeyCode::Char(c),
                            modifiers,
                            ..
                        } if !modifiers.contains(KeyModifiers::CONTROL)
                            && !modifiers.contains(KeyModifiers::ALT) =>
                        {
                            draft.push(c);
                        }
                        _ => {}
                    }
                    self.rename_state = RenameState::Editing { thread_id, draft };
                }
            }
            RenameState::Saving { thread_id, name } => {
                self.rename_state = if self.keymap.list.cancel.is_pressed(key) {
                    RenameState::Idle
                } else {
                    RenameState::Saving { thread_id, name }
                };
            }
            RenameState::Idle => {}
        }
        self.request_frame();
        None
    }

    /// Append pasted text to the rename draft when editing.
    pub(super) fn handle_rename_paste(&mut self, pasted: &str) -> bool {
        let RenameState::Editing { draft, .. } = &mut self.rename_state else {
            return false;
        };
        let Some(pasted) = crate::clipboard_paste::normalize_pasted_search_query(pasted) else {
            return true;
        };
        if !draft.is_empty() && !draft.ends_with(char::is_whitespace) {
            draft.push(' ');
        }
        draft.push_str(&pasted);
        self.request_frame();
        true
    }

    /// Apply a `thread/name/set` result: keep the row in sync with the server
    /// and reopen the editor on failure so the user can retry.
    pub(super) fn handle_rename_result(
        &mut self,
        thread_id: ThreadId,
        name: String,
        result: std::io::Result<()>,
    ) {
        let was_pending = matches!(
            self.rename_state,
            RenameState::Saving {
                thread_id: pending_id,
                name: ref pending_name,
            } if pending_id == thread_id && *pending_name == name
        );
        match result {
            Ok(()) => {
                self.inline_error = None;
                // Apply even when the user cancelled the wait: the name did
                // change server-side, so the row should reflect it.
                for row in self.all_rows.iter_mut() {
                    if row.thread_id == Some(thread_id) {
                        row.thread_name = Some(name.clone());
                    }
                }
                if was_pending {
                    self.rename_state = RenameState::Idle;
                }
                self.apply_filter();
                if let Some(selected) = self
                    .filtered_rows
                    .iter()
                    .position(|row| row.thread_id == Some(thread_id))
                {
                    self.selected = selected;
                    self.ensure_selected_visible();
                }
            }
            Err(error) => {
                if was_pending {
                    self.rename_state = RenameState::Editing {
                        thread_id,
                        draft: name,
                    };
                }
                self.inline_error = Some(format!("Failed to rename session: {error}"));
            }
        }
        self.request_frame();
    }
}
