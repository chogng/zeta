use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use zeta_utils_path::write_atomically;

use super::TerminalSettings;

const MAX_RESOURCE_BYTES: u64 = 64 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
enum ResourceSnapshot {
    Missing,
    Contents(Vec<u8>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TerminalSettingsEdit {
    pub(crate) expected_revision: u64,
    pub(crate) mouse_interactions: bool,
}

/// Host-local terminal preferences displayed alongside the App Server configuration snapshot.
pub(crate) struct ConfigResource {
    path: PathBuf,
    observed: Option<ResourceSnapshot>,
    settings: TerminalSettings,
    revision: u64,
}

impl ConfigResource {
    pub(crate) fn new(path: PathBuf) -> Self {
        Self {
            path,
            observed: None,
            settings: TerminalSettings::default(),
            revision: 0,
        }
    }

    pub(crate) fn refresh(&mut self) -> Result<TerminalSettings, String> {
        let snapshot = read_snapshot(&self.path)
            .map_err(|error| format!("could not read {}: {error}", self.path.display()))?;
        if self.observed.as_ref() == Some(&snapshot) {
            return Ok(self.settings);
        }
        let settings = settings_from_snapshot(&snapshot)
            .map_err(|error| format!("rejected {}: {error}", self.path.display()))?;
        self.observed = Some(snapshot);
        self.settings = settings;
        self.revision = self.revision.saturating_add(1);
        Ok(settings)
    }

    pub(crate) const fn revision(&self) -> u64 {
        self.revision
    }

    pub(crate) const fn settings(&self) -> TerminalSettings {
        self.settings
    }

    pub(crate) fn apply_edit(
        &mut self,
        edit: &TerminalSettingsEdit,
    ) -> Result<(TerminalSettings, u64), String> {
        if edit.expected_revision != self.revision {
            return Err(
                "terminal settings changed after Config opened; reopen /config and try again"
                    .into(),
            );
        }
        let current = read_snapshot(&self.path)
            .map_err(|error| format!("could not read {}: {error}", self.path.display()))?;
        if self.observed.as_ref() != Some(&current) {
            return Err("terminal settings changed on disk; reopen /config and try again".into());
        }

        let mut settings = settings_from_snapshot(&current)
            .map_err(|error| format!("rejected {}: {error}", self.path.display()))?;
        settings.set_mouse_interactions(edit.mouse_interactions);
        let mut contents = serde_json::to_vec_pretty(&settings)
            .map_err(|error| format!("could not serialize terminal settings: {error}"))?;
        contents.push(b'\n');
        write_atomically(&self.path, &contents)
            .map_err(|error| format!("could not save {}: {error}", self.path.display()))?;

        self.observed = Some(ResourceSnapshot::Contents(contents));
        self.settings = settings;
        self.revision = self.revision.saturating_add(1);
        Ok((settings, self.revision))
    }
}

fn settings_from_snapshot(snapshot: &ResourceSnapshot) -> Result<TerminalSettings, String> {
    match snapshot {
        ResourceSnapshot::Missing => Ok(TerminalSettings::default()),
        ResourceSnapshot::Contents(contents) => {
            serde_json::from_slice(contents).map_err(|error| format!("invalid JSON: {error}"))
        }
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
#[path = "resource_tests.rs"]
mod tests;
