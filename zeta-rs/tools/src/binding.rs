use crate::{
    ToolBindingId, ToolDefinitionDigest, ToolName, ToolRegistryGeneration, ToolRuntimeKey,
};

/// Resolves a model-visible name in one frozen registry snapshot to one concrete host runtime.
///
/// Bindings are immutable for the lifetime of an invocation. Hosts must reject a call that names
/// a binding from another snapshot rather than re-resolving only by `ToolName`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolBinding {
    registry_generation: ToolRegistryGeneration,
    id: ToolBindingId,
    exposed_name: ToolName,
    definition_digest: ToolDefinitionDigest,
    runtime_key: ToolRuntimeKey,
}

impl ToolBinding {
    pub fn new(
        registry_generation: ToolRegistryGeneration,
        id: ToolBindingId,
        exposed_name: ToolName,
        definition_digest: ToolDefinitionDigest,
        runtime_key: ToolRuntimeKey,
    ) -> Self {
        Self {
            registry_generation,
            id,
            exposed_name,
            definition_digest,
            runtime_key,
        }
    }

    pub fn registry_generation(&self) -> ToolRegistryGeneration {
        self.registry_generation
    }

    pub fn id(&self) -> &ToolBindingId {
        &self.id
    }

    pub fn exposed_name(&self) -> &ToolName {
        &self.exposed_name
    }

    pub fn definition_digest(&self) -> &ToolDefinitionDigest {
        &self.definition_digest
    }

    pub fn runtime_key(&self) -> &ToolRuntimeKey {
        &self.runtime_key
    }
}

#[cfg(test)]
#[path = "binding_tests.rs"]
mod tests;
