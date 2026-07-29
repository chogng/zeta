use crate::is_wsl;
use std::path::{Path, PathBuf};

/// Canonicalizes a path for host-filesystem identity comparison.
///
/// The path must exist. WSL paths on mounted Windows drives are ASCII-lowercased
/// after canonicalization because those mounts are case-insensitive.
pub fn normalize_for_path_comparison(path: impl AsRef<Path>) -> std::io::Result<PathBuf> {
    let canonical = path.as_ref().canonicalize()?;
    Ok(normalize_for_wsl(canonical))
}

/// Compares two paths after applying host-filesystem normalization.
///
/// When either path cannot be canonicalized, direct path equality is used so
/// two identical not-yet-created paths still compare equal.
pub fn paths_match_after_normalization(left: impl AsRef<Path>, right: impl AsRef<Path>) -> bool {
    if let (Ok(left), Ok(right)) = (
        normalize_for_path_comparison(left.as_ref()),
        normalize_for_path_comparison(right.as_ref()),
    ) {
        return left == right;
    }
    left.as_ref() == right.as_ref()
}

/// Removes host-specific syntax that must not cross a native working-directory boundary.
pub fn normalize_for_native_workdir(path: impl AsRef<Path>) -> PathBuf {
    normalize_for_native_workdir_on(path.as_ref().to_path_buf(), cfg!(windows))
}

fn normalize_for_wsl(path: PathBuf) -> PathBuf {
    normalize_for_wsl_on(path, is_wsl())
}

pub(super) fn normalize_for_native_workdir_on(path: PathBuf, is_windows: bool) -> PathBuf {
    if is_windows {
        dunce::simplified(&path).to_path_buf()
    } else {
        path
    }
}

pub(super) fn normalize_for_wsl_on(path: PathBuf, is_wsl: bool) -> PathBuf {
    if !is_wsl || !is_wsl_case_insensitive_path(&path) {
        return path;
    }
    lower_ascii_path(path)
}

fn is_wsl_case_insensitive_path(path: &Path) -> bool {
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::ffi::OsStrExt;
        use std::path::Component;

        let mut components = path.components();
        let Some(Component::RootDir) = components.next() else {
            return false;
        };
        let Some(Component::Normal(mount)) = components.next() else {
            return false;
        };
        let Some(Component::Normal(drive)) = components.next() else {
            return false;
        };
        let drive = drive.as_bytes();
        mount.as_bytes().eq_ignore_ascii_case(b"mnt")
            && drive.len() == 1
            && drive[0].is_ascii_alphabetic()
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = path;
        false
    }
}

#[cfg(target_os = "linux")]
fn lower_ascii_path(path: PathBuf) -> PathBuf {
    use std::ffi::OsString;
    use std::os::unix::ffi::{OsStrExt, OsStringExt};

    let lowered = path
        .as_os_str()
        .as_bytes()
        .iter()
        .map(u8::to_ascii_lowercase)
        .collect();
    PathBuf::from(OsString::from_vec(lowered))
}

#[cfg(not(target_os = "linux"))]
fn lower_ascii_path(path: PathBuf) -> PathBuf {
    path
}
