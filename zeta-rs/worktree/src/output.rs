use crate::ManagedDirCleanupEligibility;
use crate::WorktreeManager;
use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use serde::Deserialize;
use serde::Serialize;
use sha2::Digest;
use sha2::Sha256;
use std::fs;
use std::io::ErrorKind;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use tempfile::NamedTempFile;
use zeta_file_access::Dir;
use zeta_file_access::DirId;
use zeta_protocol::ContentDigest;
use zeta_turn_changes::DirectorySnapshotStore;

const OUTPUT_BINDING_FILENAME: &str = "zeta-work-attempt-output.json";
const OUTPUT_BINDING_VERSION: u8 = 1;

/// Exact execution whose private build and test output survives process restart.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ManagedOutputOwner {
    WorkAttempt(WorkAttemptOutputOwner),
    Verification(VerificationOutputOwner),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkAttemptOutputOwner {
    work_run_id: String,
    attempt_id: String,
    thread_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VerificationOutputOwner {
    work_run_id: String,
    verification_key: String,
}

impl ManagedOutputOwner {
    pub fn work_attempt(
        work_run_id: impl Into<String>,
        attempt_id: impl Into<String>,
        thread_id: impl Into<String>,
    ) -> Self {
        Self::WorkAttempt(WorkAttemptOutputOwner {
            work_run_id: work_run_id.into(),
            attempt_id: attempt_id.into(),
            thread_id: thread_id.into(),
        })
    }

    pub fn verification(
        work_run_id: impl Into<String>,
        verification_key: impl Into<String>,
    ) -> Self {
        Self::Verification(VerificationOutputOwner {
            work_run_id: work_run_id.into(),
            verification_key: verification_key.into(),
        })
    }

    fn validate(&self) -> Result<()> {
        let valid = match self {
            Self::WorkAttempt(WorkAttemptOutputOwner {
                work_run_id,
                attempt_id,
                thread_id,
            }) => {
                !work_run_id.trim().is_empty()
                    && !attempt_id.trim().is_empty()
                    && !thread_id.trim().is_empty()
            }
            Self::Verification(VerificationOutputOwner {
                work_run_id,
                verification_key,
            }) => !work_run_id.trim().is_empty() && !verification_key.trim().is_empty(),
        };
        if !valid {
            bail!("managed output owner is invalid");
        }
        Ok(())
    }

    fn digest(&self) -> String {
        let mut hasher = Sha256::new();
        let values = match self {
            Self::WorkAttempt(WorkAttemptOutputOwner {
                work_run_id,
                attempt_id,
                thread_id,
            }) => vec![
                "work-attempt-output",
                work_run_id.as_str(),
                attempt_id.as_str(),
                thread_id.as_str(),
            ],
            Self::Verification(VerificationOutputOwner {
                work_run_id,
                verification_key,
            }) => vec![
                "work-verification-output",
                work_run_id.as_str(),
                verification_key.as_str(),
            ],
        };
        for value in values {
            hasher.update(value.as_bytes());
            hasher.update([0]);
        }
        format!("{:x}", hasher.finalize())
    }
}

/// Durable private directory that an exact WorkAttempt may use for generated output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedOutputBinding {
    owner: ManagedOutputOwner,
    root: PathBuf,
    dir_id: DirId,
    manifest_digest: ContentDigest,
}

impl ManagedOutputBinding {
    pub fn owner(&self) -> &ManagedOutputOwner {
        &self.owner
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn dir_id(&self) -> &DirId {
        &self.dir_id
    }

    pub fn manifest_digest(&self) -> &ContentDigest {
        &self.manifest_digest
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ManagedOutputRecord {
    version: u8,
    owner: ManagedOutputOwner,
    dir_id: DirId,
}

impl ManagedOutputRecord {
    fn digest(&self) -> Result<ContentDigest> {
        let encoded = serde_json::to_vec(self)?;
        Ok(ContentDigest::sha256(&encoded))
    }
}

impl WorktreeManager {
    /// Creates a private output root whose ownership survives process restart.
    pub fn provision_output(&self, owner: &ManagedOutputOwner) -> Result<ManagedOutputBinding> {
        owner.validate()?;
        let digest = owner.digest();
        let container = self
            .settings()
            .root
            .join("outputs")
            .join(&digest[..4])
            .join(&digest);
        match fs::create_dir_all(
            container
                .parent()
                .context("managed output omitted its parent")?,
        ) {
            Ok(()) => {}
            Err(error) => return Err(error.into()),
        }
        match fs::create_dir(&container) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                return self.recover_output(owner);
            }
            Err(error) => return Err(error.into()),
        }
        let result = (|| {
            let output = container.join("output");
            fs::create_dir(&output)?;
            let output = Dir::open_local(&output)?;
            let record = ManagedOutputRecord {
                version: OUTPUT_BINDING_VERSION,
                owner: owner.clone(),
                dir_id: output.id(),
            };
            write_output_record(&container, &record)?;
            Ok(ManagedOutputBinding {
                owner: owner.clone(),
                root: output.canonical_path().to_path_buf(),
                dir_id: output.id(),
                manifest_digest: record.digest()?,
            })
        })();
        if result.is_err() {
            let _ = fs::remove_dir_all(&container);
        }
        result
    }

    /// Recovers only the deterministic output root owned by the exact WorkAttempt.
    pub fn recover_output(&self, owner: &ManagedOutputOwner) -> Result<ManagedOutputBinding> {
        owner.validate()?;
        let digest = owner.digest();
        let container = self
            .settings()
            .root
            .join("outputs")
            .join(&digest[..4])
            .join(&digest);
        let container = dunce::canonicalize(&container)
            .with_context(|| format!("cannot resolve managed output {}", container.display()))?;
        let managed_root = managed_output_root(self)?;
        if !container.starts_with(&managed_root)
            || container
                .strip_prefix(&managed_root)
                .ok()
                .is_none_or(|relative| relative.components().count() != 2)
        {
            bail!("managed output has an invalid layout");
        }
        let record = read_output_record(&container)?;
        if record.version != OUTPUT_BINDING_VERSION || &record.owner != owner {
            bail!("managed output binding owner does not match its durable owner");
        }
        let output = Dir::open_local(container.join("output"))?;
        if output.id() != record.dir_id {
            bail!("managed output directory identity changed after provisioning");
        }
        Ok(ManagedOutputBinding {
            owner: owner.clone(),
            root: output.canonical_path().to_path_buf(),
            dir_id: output.id(),
            manifest_digest: record.digest()?,
        })
    }

    /// Deletes an output root only after the owning ledger proves it is no longer needed.
    pub fn cleanup_output(
        &self,
        binding: &ManagedOutputBinding,
        eligibility: ManagedDirCleanupEligibility,
    ) -> Result<()> {
        match eligibility {
            ManagedDirCleanupEligibility::AllChangeSetsSettled => {}
        }
        let recovered = self.recover_output(binding.owner())?;
        if &recovered != binding {
            bail!("managed output binding changed before cleanup");
        }
        let container = recovered
            .root
            .parent()
            .context("managed output omitted its owner container")?;
        fs::remove_dir_all(container)?;
        Ok(())
    }

    /// Captures the exact private output contents into an owner-private content-addressed store.
    pub fn capture_output(&self, binding: &ManagedOutputBinding) -> Result<ContentDigest> {
        let recovered = self.recover_output(binding.owner())?;
        if &recovered != binding {
            bail!("managed output binding changed before evidence capture");
        }
        let container = recovered
            .root
            .parent()
            .context("managed output omitted its owner container")?;
        let snapshot = DirectorySnapshotStore::new(container.join("evidence"))
            .capture(recovered.root())
            .map_err(anyhow::Error::msg)?;
        ContentDigest::new(format!("sha256:{snapshot}")).map_err(Into::into)
    }
}

fn managed_output_root(manager: &WorktreeManager) -> Result<PathBuf> {
    let path = manager.settings().root.join("outputs");
    fs::create_dir_all(&path)?;
    dunce::canonicalize(&path).map_err(Into::into)
}

fn read_output_record(container: &Path) -> Result<ManagedOutputRecord> {
    let path = container.join(OUTPUT_BINDING_FILENAME);
    let record = serde_json::from_slice::<ManagedOutputRecord>(
        &fs::read(&path).with_context(|| format!("cannot read {}", path.display()))?,
    )
    .with_context(|| format!("invalid managed output binding at {}", path.display()))?;
    record.owner.validate()?;
    Ok(record)
}

fn write_output_record(container: &Path, record: &ManagedOutputRecord) -> Result<()> {
    let path = container.join(OUTPUT_BINDING_FILENAME);
    let mut temporary = NamedTempFile::new_in(container)?;
    serde_json::to_writer(&mut temporary, record)?;
    temporary.flush()?;
    temporary
        .persist_noclobber(&path)
        .map(|_| ())
        .map_err(|error| error.error)
        .with_context(|| format!("cannot write {}", path.display()))
}
