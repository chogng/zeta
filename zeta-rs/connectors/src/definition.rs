use crate::ConnectorError;
use crate::ConnectorErrorKind;
use crate::ConnectorId;

const MAX_DISPLAY_TEXT_BYTES: usize = 4 * 1024;
const MAX_RUNTIME_ID_BYTES: usize = 1024;

/// Runtime declaration selected for a Connector independently from its account state.
///
/// Consumers materialize this declaration only after the Connector is connected. The binding
/// contains no live MCP session, transport, credential bytes, or execution authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConnectorRuntimeBinding {
    McpServer { server_id: String },
}

impl ConnectorRuntimeBinding {
    pub fn mcp_server(server_id: impl Into<String>) -> Result<Self, ConnectorError> {
        let server_id = server_id.into();
        validate_text("MCP server ID", &server_id, MAX_RUNTIME_ID_BYTES)?;
        Ok(Self::McpServer { server_id })
    }

    pub fn mcp_server_id(&self) -> &str {
        match self {
            Self::McpServer { server_id } => server_id,
        }
    }
}

/// Runtime-free definition of one connectable external product surface.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConnectorDefinition {
    id: ConnectorId,
    display_name: String,
    description: String,
    runtime_binding: ConnectorRuntimeBinding,
}

impl ConnectorDefinition {
    pub fn new(
        id: ConnectorId,
        display_name: impl Into<String>,
        description: impl Into<String>,
        runtime_binding: ConnectorRuntimeBinding,
    ) -> Result<Self, ConnectorError> {
        let display_name = display_name.into();
        let description = description.into();
        validate_text(
            "connector display name",
            &display_name,
            MAX_DISPLAY_TEXT_BYTES,
        )?;
        validate_text(
            "connector description",
            &description,
            MAX_DISPLAY_TEXT_BYTES,
        )?;
        Ok(Self {
            id,
            display_name,
            description,
            runtime_binding,
        })
    }

    pub fn id(&self) -> &ConnectorId {
        &self.id
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn runtime_binding(&self) -> &ConnectorRuntimeBinding {
        &self.runtime_binding
    }
}

pub(crate) fn validate_text(
    label: &str,
    value: &str,
    maximum_bytes: usize,
) -> Result<(), ConnectorError> {
    if value.trim().is_empty()
        || value.trim() != value
        || value.len() > maximum_bytes
        || value.chars().any(char::is_control)
    {
        return Err(ConnectorError::new(
            ConnectorErrorKind::InvalidDefinition,
            format!("{label} must be bounded non-empty plain text without surrounding whitespace"),
        ));
    }
    Ok(())
}
