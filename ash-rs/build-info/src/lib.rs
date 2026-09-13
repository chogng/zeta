//! Version and compile-time provenance shared by every product entrypoint.

use serde::Serialize;
use sha2::Digest;
use sha2::Sha256;

#[derive(
    Clone, Debug, Eq, PartialEq, Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
pub struct BuildInfo {
    pub version: String,
    pub commit: Option<String>,
    pub target: String,
    pub build_id: Option<String>,
}

impl BuildInfo {
    pub fn current() -> Self {
        let commit = option_env!("ASH_COMPILED_COMMIT");
        let target = env!("ASH_COMPILED_TARGET");
        Self {
            version: env!("CARGO_PKG_VERSION").into(),
            commit: commit.map(str::to_owned),
            target: target.into(),
            build_id: option_env!("ASH_BUILD_ID").map(str::to_owned).or_else(|| {
                commit.map(|commit| {
                    format!(
                        "sha256:{:x}",
                        Sha256::digest(format!("git:{commit}:{target}"))
                    )
                })
            }),
        }
    }
}

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
#[path = "build_info_tests.rs"]
mod tests;
