use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::Sha256;
use sha2::digest::Digest;
use std::fmt;

const SHA256_PREFIX: &str = "sha256:";
const SHA256_HEX_BYTES: usize = 64;

/// Exact SHA-256 identity of one `SKILL.md` byte sequence.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ContentDigest(String);

impl ContentDigest {
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidContentDigest> {
        let value = value.into();
        let Some(hex) = value.strip_prefix(SHA256_PREFIX) else {
            return Err(InvalidContentDigest);
        };
        if hex.len() != SHA256_HEX_BYTES
            || !hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(InvalidContentDigest);
        }
        Ok(Self(value))
    }

    pub(crate) fn from_hasher(hasher: Sha256) -> Self {
        Self(format!("{SHA256_PREFIX}{:x}", hasher.finalize()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ContentDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Serialize for ContentDigest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ContentDigest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidContentDigest;

impl fmt::Display for InvalidContentDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("content digest must use 'sha256:' followed by 64 lowercase hex digits")
    }
}

impl std::error::Error for InvalidContentDigest {}

/// Monotonic identity of one immutable, consumer-visible Skill catalog projection.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SkillCatalogGeneration(u64);

impl SkillCatalogGeneration {
    pub const INITIAL: Self = Self(1);

    pub fn get(self) -> u64 {
        self.0
    }

    pub(crate) fn next(self) -> Self {
        Self(
            self.0
                .checked_add(1)
                .expect("skill catalog generation exhausted"),
        )
    }
}

#[cfg(test)]
#[path = "identity_tests.rs"]
mod tests;
