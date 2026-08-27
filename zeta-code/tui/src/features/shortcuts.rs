use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;

use serde_json::Value;
use zeta_keybinding::HostPlatform;
use zeta_keybinding::user_binding_diagnostics;
use zeta_utils_path::write_atomically;

use crate::keymap::AppKeymap;
use crate::keymap::compile_app_user_bindings;

mod view;

pub(crate) use view::ShortcutAction;
pub(crate) use view::ShortcutCaptureOutcome;
pub(crate) use view::ShortcutCaptureState;
pub(crate) use view::ShortcutView;
pub(crate) use view::action_menu;
pub(crate) use view::capture_view;
pub(crate) use view::shortcut_view;

const MAX_RESOURCE_BYTES: u64 = 1024 * 1024;
const POLL_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Clone, Debug, Eq, PartialEq)]
enum ResourceSnapshot {
    Missing,
    Contents(Vec<u8>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ShortcutCaptureMode {
    SingleKey,
    Chord,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ShortcutEditIntent {
    ReplaceUser,
    AddAlternate,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ShortcutEditKind {
    Set {
        key: String,
        intent: ShortcutEditIntent,
    },
    ClearUser,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ShortcutEdit {
    pub(crate) expected_revision: u64,
    pub(crate) command_id: String,
    pub(crate) kind: ShortcutEditKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ShortcutResourcePoll {
    Unchanged,
    Updated,
    Rejected(String),
}

/// Host-local, product-scoped user keybindings for the Zeta Code TUI.
pub(crate) struct ShortcutResource {
    path: PathBuf,
    platform: HostPlatform,
    observed: Option<ResourceSnapshot>,
    diagnostics: Vec<String>,
    revision: u64,
    next_poll: Instant,
}

impl ShortcutResource {
    pub(crate) fn new(path: PathBuf, now: Instant) -> Self {
        Self {
            path,
            platform: HostPlatform::current(),
            observed: None,
            diagnostics: Vec::new(),
            revision: 0,
            next_poll: now,
        }
    }

    pub(crate) fn poll(&mut self, now: Instant, keymap: &mut AppKeymap) -> ShortcutResourcePoll {
        if now < self.next_poll {
            return ShortcutResourcePoll::Unchanged;
        }
        self.next_poll = now + POLL_INTERVAL;
        let snapshot = match read_snapshot(&self.path) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                return self.reject(format!("could not read {}: {error}", self.path.display()));
            }
        };
        if self.observed.as_ref() == Some(&snapshot) {
            return ShortcutResourcePoll::Unchanged;
        }
        self.observed = Some(snapshot.clone());
        self.revision = self.revision.saturating_add(1);
        let rules = match snapshot {
            ResourceSnapshot::Missing => Vec::new(),
            ResourceSnapshot::Contents(contents) => {
                match compile_app_user_bindings(&contents, self.platform) {
                    Ok(rules) => rules,
                    Err(error) => {
                        return self.reject(format!("rejected {}: {error}", self.path.display()));
                    }
                }
            }
        };
        let diagnostics = user_binding_diagnostics(&rules, self.platform);
        if let Err(error) = keymap.replace_user_bindings(rules) {
            return self.reject(format!("rejected {}: {error}", self.path.display()));
        }
        self.diagnostics = diagnostics;
        ShortcutResourcePoll::Updated
    }

    pub(crate) fn diagnostics(&self) -> &[String] {
        &self.diagnostics
    }

    pub(crate) fn setup_view(&self, keymap: &AppKeymap) -> ShortcutView {
        shortcut_view(
            keymap.setup_actions(),
            &self.path,
            &self.diagnostics,
            self.revision,
        )
    }

    pub(crate) fn apply_edit(
        &mut self,
        edit: &ShortcutEdit,
        keymap: &mut AppKeymap,
        now: Instant,
    ) -> Result<String, String> {
        if edit.expected_revision != self.revision {
            return Err(
                "keybindings changed after the editor opened; reopen /shortcuts and try again"
                    .to_owned(),
            );
        }
        let current = read_snapshot(&self.path)
            .map_err(|error| format!("could not read {}: {error}", self.path.display()))?;
        if self.observed.as_ref() != Some(&current) {
            return Err("keybindings changed on disk; reopen /shortcuts and try again".to_owned());
        }
        let (contents, notice) = edited_document(&current, edit)?;
        let rules = compile_app_user_bindings(&contents, self.platform)
            .map_err(|error| format!("rejected shortcut edit: {error}"))?;
        let diagnostics = user_binding_diagnostics(&rules, self.platform);
        let mut next_keymap = AppKeymap::default();
        next_keymap.replace_user_bindings(rules)?;
        write_atomically(&self.path, &contents)
            .map_err(|error| format!("could not save {}: {error}", self.path.display()))?;
        self.observed = Some(ResourceSnapshot::Contents(contents));
        self.diagnostics = diagnostics;
        self.revision = self.revision.saturating_add(1);
        self.next_poll = now + POLL_INTERVAL;
        *keymap = next_keymap;
        Ok(notice)
    }

    fn reject(&mut self, diagnostic: String) -> ShortcutResourcePoll {
        if self.diagnostics.as_slice() == [diagnostic.as_str()] {
            return ShortcutResourcePoll::Unchanged;
        }
        self.diagnostics = vec![diagnostic.clone()];
        ShortcutResourcePoll::Rejected(diagnostic)
    }
}

fn edited_document(
    snapshot: &ResourceSnapshot,
    edit: &ShortcutEdit,
) -> Result<(Vec<u8>, String), String> {
    let mut document = match snapshot {
        ResourceSnapshot::Missing => Value::Array(Vec::new()),
        ResourceSnapshot::Contents(contents) => {
            serde_json::from_slice(contents).map_err(|error| format!("invalid JSON: {error}"))?
        }
    };
    let entries = document
        .as_array_mut()
        .ok_or_else(|| "the keybindings resource root must be an array".to_owned())?;
    let command_matches = |entry: &Value| {
        entry.get("command").and_then(Value::as_str) == Some(edit.command_id.as_str())
    };

    let notice = match &edit.kind {
        ShortcutEditKind::Set { key, intent } => {
            if matches!(intent, ShortcutEditIntent::ReplaceUser) {
                entries.retain(|entry| !command_matches(entry));
            } else if entries.iter().any(|entry| {
                command_matches(entry)
                    && entry.get("key").and_then(Value::as_str) == Some(key.as_str())
                    && entry.get("when").is_none()
            }) {
                return Ok((
                    snapshot_contents(snapshot),
                    format!("No change: `{}` already uses `{key}`.", edit.command_id),
                ));
            }
            entries.push(serde_json::json!({
                "key": key,
                "command": edit.command_id,
            }));
            match intent {
                ShortcutEditIntent::ReplaceUser => format!("Set user shortcut to `{key}`."),
                ShortcutEditIntent::AddAlternate => {
                    format!("Added user shortcut `{key}`.")
                }
            }
        }
        ShortcutEditKind::ClearUser => {
            let before = entries.len();
            entries.retain(|entry| !command_matches(entry));
            if entries.len() == before {
                return Ok((
                    snapshot_contents(snapshot),
                    "No change: this action has no user shortcuts.".to_owned(),
                ));
            }
            "Cleared user shortcuts.".to_owned()
        }
    };

    let mut contents = serde_json::to_vec_pretty(&document)
        .map_err(|error| format!("could not serialize keybindings: {error}"))?;
    contents.push(b'\n');
    Ok((contents, notice))
}

fn snapshot_contents(snapshot: &ResourceSnapshot) -> Vec<u8> {
    match snapshot {
        ResourceSnapshot::Missing => b"[]\n".to_vec(),
        ResourceSnapshot::Contents(contents) => contents.clone(),
    }
}

fn read_snapshot(path: &Path) -> io::Result<ResourceSnapshot> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(ResourceSnapshot::Missing);
        }
        Err(error) => return Err(error),
    };
    if metadata.len() > MAX_RESOURCE_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("resource exceeds {MAX_RESOURCE_BYTES} bytes"),
        ));
    }
    let contents = fs::read(path)?;
    if contents.len() as u64 > MAX_RESOURCE_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("resource exceeds {MAX_RESOURCE_BYTES} bytes"),
        ));
    }
    Ok(ResourceSnapshot::Contents(contents))
}

#[cfg(test)]
#[path = "shortcuts_tests.rs"]
mod tests;
