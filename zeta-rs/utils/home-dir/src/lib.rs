//! Resolution and validation of the host-wide Zeta profile root.

use std::ffi::OsStr;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use zeta_utils_absolute_path::AbsolutePathBuf;

const PROFILE_ROOT_ENV: &str = "ZETA_PROFILE_ROOT";

/// Returns the host-wide Zeta profile root.
///
/// `ZETA_PROFILE_ROOT` overrides the default `<home>/.zeta` location. An explicit override must
/// already exist as a directory and is canonicalized. The default location does not need to exist.
pub fn find_zeta_home() -> io::Result<AbsolutePathBuf> {
    let configured = std::env::var_os(PROFILE_ROOT_ENV);
    find_zeta_home_from(configured.as_deref(), dirs::home_dir())
}

fn find_zeta_home_from(
    configured: Option<&OsStr>,
    user_home: Option<PathBuf>,
) -> io::Result<AbsolutePathBuf> {
    let configured = configured.filter(|value| !value.is_empty());
    let Some(configured) = configured else {
        let user_home = user_home.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "could not find user home directory",
            )
        })?;
        return AbsolutePathBuf::from_absolute(user_home.join(".zeta"));
    };

    let path = Path::new(configured);
    let metadata = path.metadata().map_err(|error| match error.kind() {
        io::ErrorKind::NotFound => io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "{PROFILE_ROOT_ENV} points to {:?}, but that path does not exist",
                path
            ),
        ),
        _ => io::Error::new(
            error.kind(),
            format!("failed to read {PROFILE_ROOT_ENV} {:?}: {error}", path),
        ),
    })?;

    if !metadata.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "{PROFILE_ROOT_ENV} points to {:?}, but that path is not a directory",
                path
            ),
        ));
    }

    let canonical = path.canonicalize().map_err(|error| {
        io::Error::new(
            error.kind(),
            format!(
                "failed to canonicalize {PROFILE_ROOT_ENV} {:?}: {error}",
                path
            ),
        )
    })?;
    AbsolutePathBuf::from_absolute(canonical)
}

#[cfg(test)]
#[path = "home_dir_tests.rs"]
mod tests;
