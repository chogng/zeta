use std::env;
use std::path::PathBuf;

/// Resolves the host-wide profile root shared by local App Server clients.
///
/// `ZETA_PROFILE_ROOT` is authoritative when present. Otherwise the path follows the host's
/// conventional per-user state location so changing the active workspace does not select a
/// different Config, Session, or Thread authority.
pub fn local_profile_root() -> PathBuf {
    if let Some(configured) = env::var_os("ZETA_PROFILE_ROOT") {
        return PathBuf::from(configured);
    }
    default_profile_root().unwrap_or_else(|| PathBuf::from(".zeta"))
}

#[cfg(target_os = "windows")]
fn default_profile_root() -> Option<PathBuf> {
    env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .map(|root| root.join("Zeta").join("state"))
}

#[cfg(target_os = "macos")]
fn default_profile_root() -> Option<PathBuf> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .map(|root| root.join("Library/Application Support/Zeta/state"))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn default_profile_root() -> Option<PathBuf> {
    env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .map(|root| root.join("zeta"))
        .or_else(|| {
            env::var_os("HOME")
                .map(PathBuf::from)
                .map(|root| root.join(".local/state/zeta"))
        })
}

#[cfg(not(any(unix, target_os = "windows")))]
fn default_profile_root() -> Option<PathBuf> {
    None
}
