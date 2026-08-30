use serde::Deserialize;
use serde::Serialize;
use zeta_file_access::DirId;
use zeta_protocol::ContentDigest;

/// One physical managed directory bound to one immutable contract root.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedRootBinding {
    pub source_dir_id: DirId,
    pub managed_dir_id: DirId,
    pub root_checkpoint_digest: ContentDigest,
    pub binding_manifest_digest: ContentDigest,
}

/// Provisioning result for the isolated root set owned by one WorkAttempt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum WorkAttemptWorkspace {
    Provisioning,
    Ready {
        roots: Vec<ManagedRootBinding>,
        private_output_dir_id: DirId,
    },
    Failed {
        reason: String,
    },
}

impl WorkAttemptWorkspace {
    pub(crate) const fn is_ready(&self) -> bool {
        matches!(self, Self::Ready { .. })
    }
}
