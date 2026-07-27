use crate::patch_format::PatchError;
use std::fs::{self, OpenOptions, Permissions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process;

pub(super) enum PreparedChange {
    Replace {
        target: PathBuf,
        output_path: String,
        content: String,
        permissions: Option<Permissions>,
        kind: ChangeKind,
    },
    Delete {
        target: PathBuf,
        output_path: String,
    },
}

pub(super) enum ChangeKind {
    Updated,
    Added,
}

pub(super) struct PatchSummary {
    pub(super) updated: Vec<String>,
    pub(super) added: Vec<String>,
    pub(super) deleted: Vec<String>,
}

pub(super) fn commit(changes: Vec<PreparedChange>) -> Result<PatchSummary, PatchError> {
    let mut summary = PatchSummary {
        updated: Vec::new(),
        added: Vec::new(),
        deleted: Vec::new(),
    };
    for change in changes {
        match change {
            PreparedChange::Replace {
                target,
                output_path,
                content,
                permissions,
                kind,
            } => {
                atomic_write(&target, content.as_bytes(), permissions.as_ref())?;
                match kind {
                    ChangeKind::Updated => summary.updated.push(output_path),
                    ChangeKind::Added => summary.added.push(output_path),
                }
            }
            PreparedChange::Delete {
                target,
                output_path,
            } => {
                fs::remove_file(&target).map_err(PatchError::io)?;
                summary.deleted.push(output_path);
            }
        }
    }
    Ok(summary)
}

fn atomic_write(
    target: &Path,
    content: &[u8],
    permissions: Option<&Permissions>,
) -> Result<(), PatchError> {
    let parent = target.parent().ok_or_else(|| {
        PatchError::Message(format!("write target has no parent: {}", target.display()))
    })?;
    let file_name = target
        .file_name()
        .ok_or_else(|| {
            PatchError::Message(format!("write target has no name: {}", target.display()))
        })?
        .to_string_lossy();
    for attempt in 0..16 {
        let temporary = parent.join(format!(
            ".{file_name}.zeta-patch-{}-{attempt}",
            process::id()
        ));
        let mut file = match OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
        {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(PatchError::io(error)),
        };
        let write_result = (|| {
            file.write_all(content).map_err(PatchError::io)?;
            file.sync_all().map_err(PatchError::io)?;
            if let Some(permissions) = permissions {
                fs::set_permissions(&temporary, permissions.clone()).map_err(PatchError::io)?;
            }
            drop(file);
            fs::rename(&temporary, target).map_err(PatchError::io)
        })();
        if write_result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        return write_result;
    }
    Err(PatchError::Message(format!(
        "could not allocate a temporary patch file for {}",
        target.display()
    )))
}
