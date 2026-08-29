use crate::{ChangeFile, ChangeFileKind};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct DirectorySnapshotStore {
    root: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DirectoryReplayResult {
    Clean(String),
    Conflict(Vec<PathBuf>),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Manifest {
    entries: BTreeMap<String, ManifestEntry>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ManifestEntry {
    object_id: String,
    mode: String,
    binary: bool,
    lines: u64,
}

impl DirectorySnapshotStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn capture(&self, directory: &Path) -> Result<String, String> {
        let directory = dunce::canonicalize(directory)
            .map_err(|error| format!("cannot resolve snapshot directory: {error}"))?;
        let mut manifest = Manifest {
            entries: BTreeMap::new(),
        };
        self.capture_directory(&directory, &directory, &mut manifest)?;
        self.write_manifest(&manifest)
    }

    pub fn diff(&self, before: &str, after: &str) -> Result<Vec<ChangeFile>, String> {
        let before = self.read_manifest(before)?;
        let after = self.read_manifest(after)?;
        let mut deleted = before
            .entries
            .keys()
            .filter(|path| !after.entries.contains_key(*path))
            .cloned()
            .collect::<BTreeSet<_>>();
        let mut added = after
            .entries
            .keys()
            .filter(|path| !before.entries.contains_key(*path))
            .cloned()
            .collect::<BTreeSet<_>>();
        let mut changes = Vec::new();

        for old_path in deleted.clone() {
            let old = &before.entries[&old_path];
            let Some(new_path) = added
                .iter()
                .find(|path| same_content(old, &after.entries[*path]))
                .cloned()
            else {
                continue;
            };
            deleted.remove(&old_path);
            added.remove(&new_path);
            changes.push(change(
                PathBuf::from(&new_path),
                Some(PathBuf::from(old_path)),
                ChangeFileKind::Renamed,
                Some(old),
                Some(&after.entries[&new_path]),
            ));
        }
        for path in deleted {
            changes.push(change(
                PathBuf::from(&path),
                None,
                ChangeFileKind::Deleted,
                Some(&before.entries[&path]),
                None,
            ));
        }
        for path in added {
            changes.push(change(
                PathBuf::from(&path),
                None,
                ChangeFileKind::Added,
                None,
                Some(&after.entries[&path]),
            ));
        }
        for (path, old) in &before.entries {
            let Some(new) = after.entries.get(path) else {
                continue;
            };
            if old == new {
                continue;
            }
            changes.push(change(
                PathBuf::from(path),
                None,
                if old.mode == new.mode {
                    ChangeFileKind::Modified
                } else {
                    ChangeFileKind::TypeChanged
                },
                Some(old),
                Some(new),
            ));
        }
        changes.sort_by(|left, right| left.path.cmp(&right.path));
        Ok(changes)
    }

    pub fn replay(
        &self,
        before: &str,
        current: &str,
        after: &str,
    ) -> Result<DirectoryReplayResult, String> {
        let before = self.read_manifest(before)?;
        let current = self.read_manifest(current)?;
        let after = self.read_manifest(after)?;
        let paths = before
            .entries
            .keys()
            .chain(current.entries.keys())
            .chain(after.entries.keys())
            .cloned()
            .collect::<BTreeSet<_>>();
        let mut entries = BTreeMap::new();
        let mut conflicts = Vec::new();
        for path in paths {
            let base = before.entries.get(&path);
            let existing = current.entries.get(&path);
            let incoming = after.entries.get(&path);
            let selected = if existing == base {
                incoming
            } else if incoming == base || existing == incoming {
                existing
            } else {
                conflicts.push(PathBuf::from(&path));
                continue;
            };
            if let Some(entry) = selected {
                entries.insert(path, entry.clone());
            }
        }
        if conflicts.is_empty() {
            self.write_manifest(&Manifest { entries })
                .map(DirectoryReplayResult::Clean)
        } else {
            Ok(DirectoryReplayResult::Conflict(conflicts))
        }
    }

    pub fn read_blob(&self, object_id: &str, limit: usize) -> Result<(Vec<u8>, bool), String> {
        validate_id(object_id)?;
        let bytes = fs::read(self.blob_path(object_id))
            .map_err(|error| format!("cannot read directory snapshot blob: {error}"))?;
        let truncated = bytes.len() > limit;
        Ok((bytes.into_iter().take(limit).collect(), truncated))
    }

    pub fn diff_text(
        &self,
        before: &str,
        after: &str,
        limit: usize,
    ) -> Result<(String, bool), String> {
        let changes = self.diff(before, after)?;
        let mut output = String::new();
        let mut truncated = false;
        for file in changes {
            let header = format!("diff --zeta a/{0} b/{0}\n", file.path.display());
            if !push_bounded(&mut output, &header, limit) {
                truncated = true;
                break;
            }
            if file.binary {
                if !push_bounded(&mut output, "Binary contents changed\n", limit) {
                    truncated = true;
                    break;
                }
                continue;
            }
            for (prefix, object_id) in [
                ('-', file.before_object_id.as_deref()),
                ('+', file.after_object_id.as_deref()),
            ] {
                let Some(object_id) = object_id else { continue };
                let (bytes, blob_truncated) =
                    self.read_blob(object_id, limit.saturating_sub(output.len()))?;
                let text = String::from_utf8(bytes)
                    .map_err(|_| "text snapshot blob is not UTF-8".to_string())?;
                for line in text.lines() {
                    if !push_bounded(&mut output, &format!("{prefix}{line}\n"), limit) {
                        truncated = true;
                        break;
                    }
                }
                truncated |= blob_truncated;
                if truncated {
                    break;
                }
            }
            if truncated {
                break;
            }
        }
        Ok((output, truncated))
    }

    pub fn replace_directory(&self, directory: &Path, tree: &str) -> Result<(), String> {
        let manifest = self.read_manifest(tree)?;
        for entry in fs::read_dir(directory)
            .map_err(|error| format!("cannot enumerate managed directory: {error}"))?
        {
            let path = entry
                .map_err(|error| format!("cannot inspect managed directory entry: {error}"))?
                .path();
            let metadata = fs::symlink_metadata(&path)
                .map_err(|error| format!("cannot inspect managed path: {error}"))?;
            if metadata.is_dir() && !metadata.file_type().is_symlink() {
                fs::remove_dir_all(&path)
            } else {
                fs::remove_file(&path)
            }
            .map_err(|error| format!("cannot clear managed directory path: {error}"))?;
        }
        for (relative, entry) in manifest.entries {
            let path = directory.join(&relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| format!("cannot create snapshot parent: {error}"))?;
            }
            let (bytes, _) = self.read_blob(&entry.object_id, usize::MAX)?;
            if entry.mode == "120000" {
                create_symlink(&bytes, &path)?;
            } else {
                fs::write(&path, bytes)
                    .map_err(|error| format!("cannot restore snapshot file: {error}"))?;
                apply_mode(&path, &entry.mode)?;
            }
        }
        Ok(())
    }

    fn capture_directory(
        &self,
        root: &Path,
        directory: &Path,
        manifest: &mut Manifest,
    ) -> Result<(), String> {
        let mut entries = fs::read_dir(directory)
            .map_err(|error| format!("cannot enumerate snapshot directory: {error}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("cannot inspect snapshot entry: {error}"))?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path)
                .map_err(|error| format!("cannot inspect snapshot path: {error}"))?;
            if metadata.is_dir() && !metadata.file_type().is_symlink() {
                self.capture_directory(root, &path, manifest)?;
                continue;
            }
            let relative = path
                .strip_prefix(root)
                .map_err(|_| "snapshot path escaped its root".to_string())?;
            let relative = portable_path(relative)?;
            let (bytes, mode) = if metadata.file_type().is_symlink() {
                (symlink_bytes(&path)?, "120000".to_string())
            } else if metadata.is_file() {
                (
                    fs::read(&path)
                        .map_err(|error| format!("cannot read snapshot file: {error}"))?,
                    file_mode(&metadata),
                )
            } else {
                return Err(format!("unsupported filesystem entry: {}", path.display()));
            };
            let binary = bytes.contains(&0) || std::str::from_utf8(&bytes).is_err();
            let lines = if binary { 0 } else { text_lines(&bytes) };
            let object_id = self.write_blob(&bytes)?;
            manifest.entries.insert(
                relative,
                ManifestEntry {
                    object_id,
                    mode,
                    binary,
                    lines,
                },
            );
        }
        Ok(())
    }

    fn write_blob(&self, bytes: &[u8]) -> Result<String, String> {
        let id = digest(bytes);
        let path = self.blob_path(&id);
        write_once(&path, bytes)?;
        Ok(id)
    }

    fn write_manifest(&self, manifest: &Manifest) -> Result<String, String> {
        let bytes = serde_json::to_vec(manifest)
            .map_err(|error| format!("cannot encode directory snapshot manifest: {error}"))?;
        let id = digest(&bytes);
        write_once(&self.root.join("trees").join(format!("{id}.json")), &bytes)?;
        Ok(id)
    }

    fn read_manifest(&self, id: &str) -> Result<Manifest, String> {
        validate_id(id)?;
        let path = self.root.join("trees").join(format!("{id}.json"));
        serde_json::from_slice(
            &fs::read(&path)
                .map_err(|error| format!("cannot read directory snapshot manifest: {error}"))?,
        )
        .map_err(|error| format!("invalid directory snapshot manifest: {error}"))
    }

    fn blob_path(&self, id: &str) -> PathBuf {
        self.root.join("blobs").join(&id[..2]).join(id)
    }
}

fn same_content(left: &ManifestEntry, right: &ManifestEntry) -> bool {
    left.object_id == right.object_id && left.mode == right.mode
}

fn change(
    path: PathBuf,
    previous_path: Option<PathBuf>,
    kind: ChangeFileKind,
    before: Option<&ManifestEntry>,
    after: Option<&ManifestEntry>,
) -> ChangeFile {
    let binary =
        before.is_some_and(|entry| entry.binary) || after.is_some_and(|entry| entry.binary);
    ChangeFile {
        path,
        previous_path,
        kind,
        before_object_id: before.map(|entry| entry.object_id.clone()),
        after_object_id: after.map(|entry| entry.object_id.clone()),
        before_mode: before.map(|entry| entry.mode.clone()),
        after_mode: after.map(|entry| entry.mode.clone()),
        binary,
        additions: if binary {
            0
        } else {
            after.map_or(0, |entry| entry.lines)
        },
        deletions: if binary {
            0
        } else {
            before.map_or(0, |entry| entry.lines)
        },
    }
}

fn write_once(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if path.exists() {
        return Ok(());
    }
    let parent = path
        .parent()
        .ok_or_else(|| "snapshot object path omitted its parent".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("cannot create snapshot object directory: {error}"))?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)
        .map_err(|error| format!("cannot create snapshot object: {error}"))?;
    temporary
        .write_all(bytes)
        .map_err(|error| format!("cannot write snapshot object: {error}"))?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|error| format!("cannot sync snapshot object: {error}"))?;
    match temporary.persist_noclobber(path) {
        Ok(_) => Ok(()),
        Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(format!("cannot install snapshot object: {}", error.error)),
    }
}

fn portable_path(path: &Path) -> Result<String, String> {
    path.components()
        .map(|component| {
            component
                .as_os_str()
                .to_str()
                .ok_or_else(|| "snapshot path is not UTF-8".to_string())
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|components| components.join("/"))
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn validate_id(id: &str) -> Result<(), String> {
    if id.len() == 64 && id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err("invalid directory snapshot object ID".into())
    }
}

fn text_lines(bytes: &[u8]) -> u64 {
    if bytes.is_empty() {
        0
    } else {
        bytes.iter().filter(|byte| **byte == b'\n').count() as u64
            + u64::from(!bytes.ends_with(b"\n"))
    }
}

fn push_bounded(output: &mut String, value: &str, limit: usize) -> bool {
    if output.len().saturating_add(value.len()) > limit {
        return false;
    }
    output.push_str(value);
    true
}

#[cfg(unix)]
fn file_mode(metadata: &fs::Metadata) -> String {
    use std::os::unix::fs::PermissionsExt;
    if metadata.permissions().mode() & 0o111 == 0 {
        "100644"
    } else {
        "100755"
    }
    .into()
}

#[cfg(not(unix))]
fn file_mode(_: &fs::Metadata) -> String {
    "100644".into()
}

#[cfg(unix)]
fn apply_mode(path: &Path, mode: &str) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let permissions = fs::Permissions::from_mode(if mode == "100755" { 0o755 } else { 0o644 });
    fs::set_permissions(path, permissions)
        .map_err(|error| format!("cannot restore snapshot mode: {error}"))
}

#[cfg(not(unix))]
fn apply_mode(_: &Path, _: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(unix)]
fn symlink_bytes(path: &Path) -> Result<Vec<u8>, String> {
    use std::os::unix::ffi::OsStrExt;
    fs::read_link(path)
        .map(|target| target.as_os_str().as_bytes().to_vec())
        .map_err(|error| format!("cannot read snapshot symlink: {error}"))
}

#[cfg(not(unix))]
fn symlink_bytes(path: &Path) -> Result<Vec<u8>, String> {
    fs::read_link(path)
        .map(|target| target.to_string_lossy().as_bytes().to_vec())
        .map_err(|error| format!("cannot read snapshot symlink: {error}"))
}

#[cfg(unix)]
fn create_symlink(target: &[u8], path: &Path) -> Result<(), String> {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    std::os::unix::fs::symlink(OsStr::from_bytes(target), path)
        .map_err(|error| format!("cannot restore snapshot symlink: {error}"))
}

#[cfg(windows)]
fn create_symlink(target: &[u8], path: &Path) -> Result<(), String> {
    let target = String::from_utf8(target.to_vec())
        .map_err(|_| "snapshot symlink target is not UTF-8".to_string())?;
    std::os::windows::fs::symlink_file(target, path)
        .map_err(|error| format!("cannot restore snapshot symlink: {error}"))
}
