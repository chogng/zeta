//! Credential-store boundary. No credential is persisted in Zeta configuration or rollout logs.

use std::process::Command;

/// Obtains sensitive values from an operating-system credential service or another secure host.
///
/// Implementations must not write secrets to Zeta's ordinary configuration, rollout, or logs.
pub trait CredentialStore: Send + Sync {
    fn read_secret(&self, account: &str) -> Result<Option<String>, CredentialError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CredentialError(pub String);

/// Reads account-scoped secrets from the macOS Keychain item identified by `service`.
///
/// The adapter passes each account as a process argument to the OS `security` utility but never
/// writes a secret to a command line, configuration file, event log, or diagnostic message.
pub struct MacosKeychainCredentialStore {
    service: String,
}

impl MacosKeychainCredentialStore {
    pub fn new(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }
}

impl CredentialStore for MacosKeychainCredentialStore {
    fn read_secret(&self, account: &str) -> Result<Option<String>, CredentialError> {
        if !cfg!(target_os = "macos") {
            return Err(CredentialError(
                "macOS Keychain is unavailable on this host".into(),
            ));
        }
        let output = Command::new("security")
            .args([
                "find-generic-password",
                "-s",
                &self.service,
                "-a",
                account,
                "-w",
            ])
            .output()
            .map_err(|error| CredentialError(error.to_string()))?;
        if output.status.success() {
            return String::from_utf8(output.stdout)
                .map(|secret| Some(secret.trim_end_matches(['\r', '\n']).into()))
                .map_err(|error| CredentialError(error.to_string()));
        }
        if output.status.code() == Some(44) {
            Ok(None)
        } else {
            Err(CredentialError("Keychain lookup failed".into()))
        }
    }
}

/// Explicitly disables credential access in hosts that do not provide a secure secret facility.
pub struct UnavailableCredentialStore;
impl CredentialStore for UnavailableCredentialStore {
    fn read_secret(&self, _: &str) -> Result<Option<String>, CredentialError> {
        Err(CredentialError("credential store unavailable".into()))
    }
}
