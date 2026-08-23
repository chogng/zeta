use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;

use zeta_keybinding::HostPlatform;
use zeta_keybinding::user_binding_diagnostics;

use super::keymap::AppKeymap;
use super::keymap::compile_app_user_bindings;

const MAX_RESOURCE_BYTES: u64 = 1024 * 1024;
const POLL_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Clone, Debug, Eq, PartialEq)]
enum ResourceSnapshot {
    Missing,
    Contents(Vec<u8>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum AppKeybindingsResourcePoll {
    Unchanged,
    Updated,
    Rejected(String),
}

/// Host-local, product-scoped user keybindings for the Zeta Code TUI.
pub(super) struct AppKeybindingsResource {
    path: PathBuf,
    platform: HostPlatform,
    observed: Option<ResourceSnapshot>,
    diagnostics: Vec<String>,
    next_poll: Instant,
}

impl AppKeybindingsResource {
    pub(super) fn new(path: PathBuf, now: Instant) -> Self {
        Self {
            path,
            platform: HostPlatform::current(),
            observed: None,
            diagnostics: Vec::new(),
            next_poll: now,
        }
    }

    pub(super) fn poll(
        &mut self,
        now: Instant,
        keymap: &mut AppKeymap,
    ) -> AppKeybindingsResourcePoll {
        if now < self.next_poll {
            return AppKeybindingsResourcePoll::Unchanged;
        }
        self.next_poll = now + POLL_INTERVAL;
        let snapshot = match read_snapshot(&self.path) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                return self.reject(format!("could not read {}: {error}", self.path.display()));
            }
        };
        if self.observed.as_ref() == Some(&snapshot) {
            return AppKeybindingsResourcePoll::Unchanged;
        }
        self.observed = Some(snapshot.clone());
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
        AppKeybindingsResourcePoll::Updated
    }

    pub(super) fn diagnostics(&self) -> &[String] {
        &self.diagnostics
    }

    fn reject(&mut self, diagnostic: String) -> AppKeybindingsResourcePoll {
        if self.diagnostics.as_slice() == [diagnostic.as_str()] {
            return AppKeybindingsResourcePoll::Unchanged;
        }
        self.diagnostics = vec![diagnostic.clone()];
        AppKeybindingsResourcePoll::Rejected(diagnostic)
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
#[path = "keybindings_resource_tests.rs"]
mod tests;
