use crate::{
    ConfigCommandDisposition, ConfigCommandError, ConfigCommandRequest, ConfigCommandResult,
    ConfigGeneration, ConfigRevision, PreferencesUpdate, ResolvedConfigSnapshot, UserConfigCommand,
    UserConfigDocument,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use zeta_protocol::{CommandId, Patch};

const CONFIG_AUTHORITY_SCHEMA_VERSION: u32 = 4;

/// Failure while loading, validating, or persisting the user configuration authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigError(pub String);

impl std::fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ConfigError {}

/// Single-file durable authority for ordinary, non-secret user configuration.
///
/// The file contains the desired document, its monotonic revision, and compact command receipts.
/// It is intentionally not a hand-edited projection: callers must use typed commands so replay
/// and revision semantics remain well-defined.
pub struct ConfigStore {
    path: PathBuf,
    lock_path: PathBuf,
    write_lock: Mutex<()>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConfigAuthority {
    schema_version: u32,
    revision: ConfigRevision,
    generation: ConfigGeneration,
    document: UserConfigDocument,
    receipts: BTreeMap<CommandId, ConfigCommandReceipt>,
}

impl Default for ConfigAuthority {
    fn default() -> Self {
        Self {
            schema_version: CONFIG_AUTHORITY_SCHEMA_VERSION,
            revision: ConfigRevision::INITIAL,
            generation: ConfigGeneration::INITIAL,
            document: UserConfigDocument::default(),
            receipts: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConfigCommandReceipt {
    expected_revision: ConfigRevision,
    command: UserConfigCommand,
    result_revision: ConfigRevision,
    result_generation: ConfigGeneration,
}

impl ConfigStore {
    /// Opens a single-file configuration authority at `path`.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, ConfigError> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(io_error)?;
        }
        Ok(Self {
            lock_path: path.with_extension("lock"),
            path,
            write_lock: Mutex::new(()),
        })
    }

    /// Reads the current immutable resolved snapshot.
    pub fn read_snapshot(&self) -> Result<ResolvedConfigSnapshot, ConfigError> {
        self.read_authority().map(|authority| authority.snapshot())
    }

    /// Applies one retry-safe typed command at its expected authority revision.
    pub fn apply(
        &self,
        request: ConfigCommandRequest,
    ) -> Result<ConfigCommandResult, ConfigCommandError> {
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| ConfigError("config write lock poisoned".into()))?;
        let _file_lock = self.acquire_file_lock()?;
        let mut authority = self.read_authority()?;

        if let Some(receipt) = authority.receipts.get(&request.command_id) {
            if receipt.expected_revision != request.expected_revision
                || receipt.command != request.command
            {
                return Err(ConfigCommandError::CommandConflict);
            }
            return Ok(ConfigCommandResult {
                revision: receipt.result_revision,
                generation: receipt.result_generation,
                disposition: ConfigCommandDisposition::Replayed,
            });
        }

        if request.expected_revision != authority.revision {
            return Err(ConfigCommandError::RevisionConflict {
                expected: request.expected_revision,
                actual: authority.revision,
            });
        }

        let document_before = authority.document.clone();
        apply_command(&mut authority.document, &request.command)?;
        authority.document.validate()?;
        if authority.document != document_before {
            authority.revision = authority.revision.next();
            authority.generation = authority.generation.next();
        }
        authority.receipts.insert(
            request.command_id,
            ConfigCommandReceipt {
                expected_revision: request.expected_revision,
                command: request.command,
                result_revision: authority.revision,
                result_generation: authority.generation,
            },
        );
        self.persist(&authority)?;
        Ok(ConfigCommandResult {
            revision: authority.revision,
            generation: authority.generation,
            disposition: ConfigCommandDisposition::Updated,
        })
    }

    /// Returns the single authority file path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    fn read_authority(&self) -> Result<ConfigAuthority, ConfigError> {
        if !self.path.exists() {
            return Ok(ConfigAuthority::default());
        }
        let authority: ConfigAuthority =
            serde_json::from_slice(&fs::read(&self.path).map_err(io_error)?)
                .map_err(|error| ConfigError(error.to_string()))?;
        if authority.schema_version != CONFIG_AUTHORITY_SCHEMA_VERSION {
            return Err(ConfigError(format!(
                "unsupported config authority schema version {}",
                authority.schema_version
            )));
        }
        authority.document.validate()?;
        Ok(authority)
    }

    fn acquire_file_lock(&self) -> Result<File, ConfigError> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&self.lock_path)
            .map_err(io_error)?;
        file.lock().map_err(io_error)?;
        Ok(file)
    }

    fn persist(&self, authority: &ConfigAuthority) -> Result<(), ConfigError> {
        authority.document.validate()?;
        write_json_atomic(&self.path, authority)
    }
}

impl ConfigAuthority {
    fn snapshot(&self) -> ResolvedConfigSnapshot {
        ResolvedConfigSnapshot::from_document(self.revision, self.generation, &self.document)
    }
}

fn apply_command(
    document: &mut UserConfigDocument,
    command: &UserConfigCommand,
) -> Result<(), ConfigError> {
    match command {
        UserConfigCommand::UpdatePreferences(update) => apply_preferences(document, update),
        UserConfigCommand::ConfigureProvider { provider, config } => {
            if &config.provider != provider {
                return Err(ConfigError(format!(
                    "provider command key '{}' does not match configuration provider '{}'",
                    provider, config.provider
                )));
            }
            document.providers.insert(provider.clone(), config.clone());
        }
        UserConfigCommand::RemoveProvider { provider } => {
            if document
                .agent
                .preferred_model
                .as_ref()
                .is_some_and(|model| model.provider == *provider)
            {
                return Err(ConfigError(format!(
                    "cannot remove provider '{}' while it is the preferred model provider",
                    provider
                )));
            }
            if document
                .agent
                .approval_review_model
                .explicit_model()
                .is_some_and(|model| model.provider == *provider)
            {
                return Err(ConfigError(format!(
                    "cannot remove provider '{}' while it is the approval review model provider",
                    provider
                )));
            }
            document.providers.remove(provider);
        }
        UserConfigCommand::UpsertMcpServer { server } => {
            document
                .mcp
                .servers
                .insert(server.id.clone(), server.clone());
        }
        UserConfigCommand::RemoveMcpServer { server_id } => {
            document.mcp.servers.remove(server_id);
        }
        UserConfigCommand::SetMcpServerEnablement {
            server_id,
            enablement,
        } => {
            let server = document.mcp.servers.get_mut(server_id).ok_or_else(|| {
                ConfigError(format!("MCP server '{}' is not configured", server_id))
            })?;
            server.enablement = *enablement;
        }
        UserConfigCommand::AddSkillSource { source } => {
            document
                .skills
                .sources
                .insert(source.id.clone(), source.clone());
        }
        UserConfigCommand::RemoveSkillSource { source_id } => {
            document.skills.sources.remove(source_id);
        }
        UserConfigCommand::SetSkillSourceEnablement {
            source_id,
            enablement,
        } => {
            let source = document.skills.sources.get_mut(source_id).ok_or_else(|| {
                ConfigError(format!("Skill source '{}' is not configured", source_id))
            })?;
            source.enablement = *enablement;
        }
    }
    Ok(())
}

fn apply_preferences(document: &mut UserConfigDocument, update: &PreferencesUpdate) {
    match &update.preferred_model {
        Patch::Missing => {}
        Patch::Null => document.agent.preferred_model = None,
        Patch::Value(model) => document.agent.preferred_model = Some(model.clone()),
    }
    match &update.approval_review_model {
        Patch::Missing => {}
        Patch::Null => {
            document.agent.approval_review_model = crate::ApprovalReviewModelSelection::Automatic;
        }
        Patch::Value(selection) => {
            document.agent.approval_review_model = selection.clone();
        }
    }
    match update.theme {
        Patch::Missing => {}
        Patch::Null => document.ui.theme = None,
        Patch::Value(theme) => document.ui.theme = Some(theme),
    }
}

fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), ConfigError> {
    let temporary = path.with_extension("tmp");
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| ConfigError(error.to_string()))?;
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&temporary)
        .map_err(io_error)?;
    file.write_all(&bytes).map_err(io_error)?;
    file.sync_all().map_err(io_error)?;
    fs::rename(&temporary, path).map_err(io_error)?;
    if let Some(parent) = path.parent() {
        File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(io_error)?;
    }
    Ok(())
}

fn io_error(error: impl std::fmt::Display) -> ConfigError {
    ConfigError(error.to_string())
}
