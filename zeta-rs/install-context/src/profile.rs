use std::ffi::OsString;
use std::path::PathBuf;

/// Resolves the host-wide profile root shared by local App Server clients.
///
/// `ZETA_PROFILE_ROOT` is authoritative when present. Otherwise every local Zeta product uses
/// `<home>/.zeta`, independent of the active directory and operating system.
pub fn local_profile_root() -> PathBuf {
    resolve_profile_root(
        std::env::var_os("ZETA_PROFILE_ROOT"),
        platform_home_directory(),
    )
}

#[cfg(target_os = "windows")]
fn platform_home_directory() -> Option<OsString> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .or_else(|| {
            let mut root = std::env::var_os("HOMEDRIVE")?;
            root.push(std::env::var_os("HOMEPATH")?);
            Some(root)
        })
}

#[cfg(unix)]
fn platform_home_directory() -> Option<OsString> {
    std::env::var_os("HOME")
}

#[cfg(not(any(unix, target_os = "windows")))]
fn platform_home_directory() -> Option<OsString> {
    std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))
}

fn resolve_profile_root(configured: Option<OsString>, home: Option<OsString>) -> PathBuf {
    configured
        .map(PathBuf::from)
        .or_else(|| home.map(PathBuf::from).map(|root| root.join(".zeta")))
        .unwrap_or_else(|| PathBuf::from(".zeta"))
}

#[cfg(test)]
#[path = "profile_tests.rs"]
mod tests;
