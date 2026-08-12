use std::fmt;

const MAX_ID_BYTES: usize = 256;

/// Stable host identity for one account-connectable external product declaration.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ConnectorId(String);

impl ConnectorId {
    pub fn new(value: impl Into<String>) -> Result<Self, ConnectorIdentityError> {
        let value = value.into();
        if value.trim().is_empty()
            || value.len() > MAX_ID_BYTES
            || value.chars().any(char::is_control)
        {
            return Err(ConnectorIdentityError(
                "connector ID must be bounded non-empty plain text".into(),
            ));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ConnectorId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Runtime binding selected for a connector independently from its account identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConnectorBinding {
    McpServer { server_id: String },
}

/// Non-secret account projection for one ready connector.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConnectedAccount {
    pub account_id: String,
    pub display_name: String,
    pub credential_reference: String,
    pub connection_generation: u64,
}

/// Current connection lifecycle without storing credential values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConnectorConnectionState {
    Disconnected,
    Connecting,
    Connected(ConnectedAccount),
    Unavailable { reason: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConnectorIdentityError(String);

impl fmt::Display for ConnectorIdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ConnectorIdentityError {}
