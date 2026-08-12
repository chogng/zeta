use crate::{
    ProtocolToolAdapterError, ToolContent, ToolDefinition, ToolInvocationKind, ToolOutput,
    ToolOutputStatus,
};
use zeta_protocol::{ContentPart, ToolCallId, ToolName};

/// Converts the canonical model function contract into a validated host definition.
///
/// The caller selects loading because protocol definitions intentionally do not own host exposure
/// policy. Execution provenance and binding identity are attached later by the registry builder.
pub fn from_protocol_tool_definition(
    definition: &zeta_protocol::ToolDefinition,
    loading: crate::ToolLoading,
) -> Result<ToolDefinition, ProtocolToolAdapterError> {
    let input_schema = crate::ToolInputSchema::parse(definition.parameters.clone())
        .map_err(ProtocolToolAdapterError::Schema)?;
    ToolDefinition::function(
        definition.name.clone(),
        definition.description.clone(),
        input_schema,
        crate::ToolOutputSchema::Unspecified,
        if definition.strict {
            crate::ToolSchemaMode::Strict
        } else {
            crate::ToolSchemaMode::ProviderDefault
        },
        loading,
    )
    .map_err(ProtocolToolAdapterError::Definition)
}

/// Converts a host function definition into the current canonical model-tool contract.
pub fn to_protocol_tool_definition(
    definition: &ToolDefinition,
) -> Result<zeta_protocol::ToolDefinition, ProtocolToolAdapterError> {
    let ToolInvocationKind::Function { input_schema } = definition.invocation() else {
        return Err(ProtocolToolAdapterError::FreeformToolsUnsupported);
    };
    Ok(zeta_protocol::ToolDefinition {
        name: definition.name().clone(),
        description: definition.description().to_owned(),
        parameters: input_schema.as_value().clone(),
        strict: matches!(definition.schema_mode(), crate::ToolSchemaMode::Strict),
    })
}

/// Converts a trustworthy host output into the current canonical model tool result.
pub fn to_protocol_tool_result(
    output: &ToolOutput,
    call_id: ToolCallId,
    name: ToolName,
) -> zeta_protocol::ToolResult {
    let content = output
        .content()
        .iter()
        .map(|content| match content {
            ToolContent::Text(text) => ContentPart::Text(text.clone()),
            ToolContent::Image { url, detail } => ContentPart::ImageUrl {
                url: url.clone(),
                detail: *detail,
            },
        })
        .collect();
    zeta_protocol::ToolResult {
        call_id,
        name,
        content,
        is_error: matches!(output.status(), ToolOutputStatus::Error),
    }
}

#[cfg(test)]
#[path = "protocol_adapter_tests.rs"]
mod tests;
