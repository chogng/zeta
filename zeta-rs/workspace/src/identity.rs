use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use std::fmt;
use std::path::Path;
use std::str::FromStr;

const TRUST_ID_PREFIX: &str = "sha256:";
const SHA256_HEX_LENGTH: usize = 64;

/// Opaque, durable lookup key for one canonical Workspace root.
///
/// Hosts persist this value instead of the filesystem path itself. It intentionally identifies
/// the canonical path boundary rather than Workspace-controlled content, so aliases resolve to
/// the same key and moving the directory requires a new trust decision.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, schemars::JsonSchema, ts_rs::TS)]
#[schemars(transparent)]
#[ts(type = "string")]
pub struct WorkspaceTrustId(String);

impl WorkspaceTrustId {
    pub(crate) fn from_canonical_path(path: &Path) -> Self {
        let mut digest = Sha256::new();
        hash_platform_path(&mut digest, path);
        let encoded = digest
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        Self(format!("{TRUST_ID_PREFIX}{encoded}"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for WorkspaceTrustId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl FromStr for WorkspaceTrustId {
    type Err = WorkspaceTrustIdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let Some(digest) = value.strip_prefix(TRUST_ID_PREFIX) else {
            return Err(WorkspaceTrustIdError);
        };
        if digest.len() != SHA256_HEX_LENGTH || !digest.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(WorkspaceTrustIdError);
        }
        Ok(Self(format!(
            "{TRUST_ID_PREFIX}{}",
            digest.to_ascii_lowercase()
        )))
    }
}

impl Serialize for WorkspaceTrustId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for WorkspaceTrustId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}

/// Invalid serialized Workspace trust identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WorkspaceTrustIdError;

impl fmt::Display for WorkspaceTrustIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Workspace trust identity must be 'sha256:' plus 64 hex characters")
    }
}

impl std::error::Error for WorkspaceTrustIdError {}

#[cfg(unix)]
fn hash_platform_path(digest: &mut Sha256, path: &Path) {
    use std::os::unix::ffi::OsStrExt;

    digest.update(b"unix\0");
    digest.update(path.as_os_str().as_bytes());
}

#[cfg(windows)]
fn hash_platform_path(digest: &mut Sha256, path: &Path) {
    use std::os::windows::ffi::OsStrExt;

    digest.update(b"windows\0");
    for code_unit in path.as_os_str().encode_wide() {
        digest.update(code_unit.to_le_bytes());
    }
}

#[cfg(not(any(unix, windows)))]
fn hash_platform_path(digest: &mut Sha256, path: &Path) {
    digest.update(b"other\0");
    digest.update(path.as_os_str().to_string_lossy().as_bytes());
}

#[cfg(test)]
#[path = "identity_tests.rs"]
mod tests;
