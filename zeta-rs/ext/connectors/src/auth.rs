use std::fmt;
use std::fmt::Write;
use std::sync::Arc;

use sha2::Digest;
use sha2::Sha256;
use zeta_connectors::ConnectorAccount;
use zeta_connectors::ConnectorAccountId;
use zeta_connectors::ConnectorConnectionGeneration;
use zeta_connectors::ConnectorConnectionState;
use zeta_connectors::ConnectorCredentialRef;
use zeta_connectors::ConnectorError;
use zeta_connectors::ConnectorId;
use zeta_connectors::ConnectorSnapshotGeneration;
use zeta_secrets::DeleteSecretOutcome;
use zeta_secrets::SecretKey;
use zeta_secrets::SecretStore;
use zeta_secrets::SecretStoreError;
use zeta_secrets::SecretValue;

use crate::ConnectorAuthority;
use crate::ConnectorAuthorityCommand;
use crate::ConnectorAuthorityError;
use crate::ConnectorCommandDisposition;
use crate::ConnectorCommandId;
use crate::ConnectorCommandRequest;
use crate::ConnectorCommandResult;

const AUTH_COMMAND_DOMAIN: &[u8] = b"zeta-connector-auth-command-v1\0";
const CREDENTIAL_KEY_DOMAIN: &[u8] = b"zeta-connector-credential-key-v1\0";

/// Ephemeral API-token connection request. The token is never cloneable or persisted in authority.
pub struct ConnectorApiTokenConnectRequest {
    pub command_id: ConnectorCommandId,
    pub expected_generation: ConnectorSnapshotGeneration,
    pub connector_id: ConnectorId,
    pub connection_generation: ConnectorConnectionGeneration,
    pub account_id: ConnectorAccountId,
    pub account_display_name: String,
    pub token: SecretValue,
}

/// Result of best-effort credential cleanup after runtime readiness has been revoked.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConnectorCredentialCleanup {
    Deleted,
    AlreadyAbsent,
    RetryRequired,
}

/// Disconnect result separates authoritative revocation from secret cleanup status.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConnectorDisconnectResult {
    pub command: ConnectorCommandResult,
    pub credential_cleanup: ConnectorCredentialCleanup,
}

/// Coordinates one Connector authority with its opaque secret persistence backend.
pub struct ConnectorCredentialService {
    authority: ConnectorAuthority,
    secrets: Arc<dyn SecretStore>,
}

impl ConnectorCredentialService {
    pub fn new(authority: ConnectorAuthority, secrets: Arc<dyn SecretStore>) -> Self {
        Self { authority, secrets }
    }

    pub fn authority(&self) -> &ConnectorAuthority {
        &self.authority
    }

    /// Stores an API token and publishes a connected account under two retry-safe authority steps.
    pub fn connect_api_token(
        &self,
        request: ConnectorApiTokenConnectRequest,
    ) -> Result<ConnectorCommandResult, ConnectorCredentialServiceError> {
        let snapshot = self.authority.snapshot();
        snapshot.entry(&request.connector_id).ok_or_else(|| {
            ConnectorCredentialServiceError::new(
                ConnectorCredentialServiceErrorKind::Authority,
                "connector is unavailable",
            )
        })?;
        let secret_key = credential_key(&request.connector_id)?;
        let credential_reference = ConnectorCredentialRef::new(secret_key.as_str().to_string())?;
        let account = ConnectorAccount::new(
            request.account_id,
            request.account_display_name,
            credential_reference,
            request.connection_generation,
        )?;
        let begin = self.authority.apply(ConnectorCommandRequest {
            command_id: phase_command_id(&request.command_id, "begin")?,
            expected_generation: request.expected_generation,
            connector_id: request.connector_id.clone(),
            command: ConnectorAuthorityCommand::BeginConnect {
                generation: request.connection_generation,
            },
        })?;
        let complete_command_id = phase_command_id(&request.command_id, "complete")?;
        if begin.disposition == ConnectorCommandDisposition::Replayed {
            let current = self.authority.snapshot();
            let still_in_exact_attempt =
                current.entry(&request.connector_id).is_some_and(|entry| {
                    entry.connection().generation() == request.connection_generation
                        && matches!(
                            entry.connection().state(),
                            ConnectorConnectionState::Connecting
                        )
                });
            if !still_in_exact_attempt {
                return self
                    .authority
                    .apply(ConnectorCommandRequest {
                        command_id: complete_command_id,
                        expected_generation: begin.generation,
                        connector_id: request.connector_id,
                        command: ConnectorAuthorityCommand::CompleteConnect { account },
                    })
                    .map_err(Into::into);
            }
        }
        self.secrets.store(&secret_key, &request.token)?;
        match self.authority.apply(ConnectorCommandRequest {
            command_id: complete_command_id,
            expected_generation: begin.generation,
            connector_id: request.connector_id,
            command: ConnectorAuthorityCommand::CompleteConnect { account },
        }) {
            Ok(result) => Ok(result),
            Err(error) => {
                let _ = self.secrets.delete(&secret_key);
                Err(error.into())
            }
        }
    }

    /// Revokes readiness first, then removes the credential without rolling revocation back.
    pub fn disconnect(
        &self,
        command_id: ConnectorCommandId,
        expected_generation: ConnectorSnapshotGeneration,
        connector_id: ConnectorId,
    ) -> Result<ConnectorDisconnectResult, ConnectorCredentialServiceError> {
        let snapshot = self.authority.snapshot();
        let entry = snapshot.entry(&connector_id).ok_or_else(|| {
            ConnectorCredentialServiceError::new(
                ConnectorCredentialServiceErrorKind::Authority,
                "connector is unavailable",
            )
        })?;
        let credential_reference = match entry.connection().state() {
            ConnectorConnectionState::Connected(account)
            | ConnectorConnectionState::ReauthorizationRequired { account, .. } => {
                Some(account.credential_reference().as_str().to_string())
            }
            ConnectorConnectionState::Disconnected
            | ConnectorConnectionState::Connecting
            | ConnectorConnectionState::Unavailable { .. } => credential_key(&connector_id)
                .ok()
                .map(|key| key.as_str().to_string()),
        };
        let connection_generation = entry.connection().generation();
        let next_connection_generation = if snapshot.generation() == expected_generation {
            connection_generation
                .get()
                .checked_add(1)
                .map(ConnectorConnectionGeneration::new)
                .ok_or_else(|| {
                    ConnectorCredentialServiceError::new(
                        ConnectorCredentialServiceErrorKind::Authority,
                        "connector connection generation exhausted",
                    )
                })?
        } else {
            connection_generation
        };
        let command = self.authority.apply(ConnectorCommandRequest {
            command_id,
            expected_generation,
            connector_id: connector_id.clone(),
            command: ConnectorAuthorityCommand::Disconnect {
                generation: next_connection_generation,
            },
        })?;
        let credential_cleanup = match credential_reference {
            Some(reference) => match SecretKey::new(reference) {
                Ok(key) => match self.secrets.delete(&key) {
                    Ok(DeleteSecretOutcome::Deleted) => ConnectorCredentialCleanup::Deleted,
                    Ok(DeleteSecretOutcome::NotFound) => ConnectorCredentialCleanup::AlreadyAbsent,
                    Err(_) => ConnectorCredentialCleanup::RetryRequired,
                },
                Err(_) => ConnectorCredentialCleanup::RetryRequired,
            },
            None => ConnectorCredentialCleanup::AlreadyAbsent,
        };
        Ok(ConnectorDisconnectResult {
            command,
            credential_cleanup,
        })
    }
}

/// Stable classification for credential orchestration failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConnectorCredentialServiceErrorKind {
    Authority,
    CommandConflict,
    GenerationConflict,
    SecretStore,
    InvalidValue,
}

/// Sanitized Connector credential orchestration failure.
#[derive(Debug)]
pub struct ConnectorCredentialServiceError {
    kind: ConnectorCredentialServiceErrorKind,
    message: String,
}

impl ConnectorCredentialServiceError {
    fn new(kind: ConnectorCredentialServiceErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub fn kind(&self) -> ConnectorCredentialServiceErrorKind {
        self.kind
    }
}

impl fmt::Display for ConnectorCredentialServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ConnectorCredentialServiceError {}

impl From<ConnectorAuthorityError> for ConnectorCredentialServiceError {
    fn from(error: ConnectorAuthorityError) -> Self {
        let kind = match error.kind() {
            crate::ConnectorAuthorityErrorKind::CommandConflict => {
                ConnectorCredentialServiceErrorKind::CommandConflict
            }
            crate::ConnectorAuthorityErrorKind::GenerationConflict => {
                ConnectorCredentialServiceErrorKind::GenerationConflict
            }
            crate::ConnectorAuthorityErrorKind::InvalidCommand
            | crate::ConnectorAuthorityErrorKind::Domain
            | crate::ConnectorAuthorityErrorKind::Persistence => {
                ConnectorCredentialServiceErrorKind::Authority
            }
        };
        Self::new(kind, error.to_string())
    }
}

impl From<SecretStoreError> for ConnectorCredentialServiceError {
    fn from(error: SecretStoreError) -> Self {
        Self::new(
            ConnectorCredentialServiceErrorKind::SecretStore,
            error.to_string(),
        )
    }
}

impl From<ConnectorError> for ConnectorCredentialServiceError {
    fn from(error: ConnectorError) -> Self {
        Self::new(
            ConnectorCredentialServiceErrorKind::InvalidValue,
            error.to_string(),
        )
    }
}

fn phase_command_id(
    parent: &ConnectorCommandId,
    phase: &str,
) -> Result<ConnectorCommandId, ConnectorCredentialServiceError> {
    let mut digest = Sha256::new();
    digest.update(AUTH_COMMAND_DOMAIN);
    digest.update((parent.as_str().len() as u64).to_be_bytes());
    digest.update(parent.as_str().as_bytes());
    digest.update(phase.as_bytes());
    ConnectorCommandId::new(format!(
        "connector-auth-{phase}-{}",
        hex_digest(digest.finalize())
    ))
    .map_err(Into::into)
}

fn credential_key(
    connector_id: &ConnectorId,
) -> Result<SecretKey, ConnectorCredentialServiceError> {
    let mut digest = Sha256::new();
    digest.update(CREDENTIAL_KEY_DOMAIN);
    digest.update((connector_id.as_str().len() as u64).to_be_bytes());
    digest.update(connector_id.as_str().as_bytes());
    SecretKey::new(format!("connector/{}", hex_digest(digest.finalize()))).map_err(|error| {
        ConnectorCredentialServiceError::new(
            ConnectorCredentialServiceErrorKind::InvalidValue,
            error.to_string(),
        )
    })
}

fn hex_digest(bytes: impl IntoIterator<Item = u8>) -> String {
    let mut value = String::with_capacity(64);
    for byte in bytes {
        write!(value, "{byte:02x}").expect("writing to a String cannot fail");
    }
    value
}
