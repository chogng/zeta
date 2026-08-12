use std::sync::Arc;

use zeta_extension_api::CapabilityToolContribution;
use zeta_extension_api::CapabilityToolContributor;
use zeta_extension_api::ExtensionError;
use zeta_extension_api::ExtensionRegistryBuilder;
use zeta_extension_api::ExtensionToolAuthority;

use crate::WebSearchBackend;
use crate::tool::WebSearchTool;

struct WebSearchExtension {
    backend: Arc<dyn WebSearchBackend>,
}

impl CapabilityToolContributor for WebSearchExtension {
    fn contribute(&self) -> Result<Vec<CapabilityToolContribution>, ExtensionError> {
        let network_scopes = self.backend.network_scopes();
        if network_scopes.is_empty() || network_scopes.iter().any(|scope| scope.trim().is_empty()) {
            return Err(ExtensionError::new(
                "Web Search backend must declare at least one exact network scope",
            ));
        }
        Ok(vec![CapabilityToolContribution::new(
            Arc::new(WebSearchTool::new(Arc::clone(&self.backend))),
            ExtensionToolAuthority::ExternalRead {
                service: self.backend.service_name().to_owned(),
                network_scopes,
                credential_reference: self.backend.credential_reference(),
            },
        )])
    }
}

/// Installs Web Search into the capability-bearing extension registry.
pub fn install(builder: &mut ExtensionRegistryBuilder, backend: Arc<dyn WebSearchBackend>) {
    builder.capability_tool_contributor(Arc::new(WebSearchExtension { backend }));
}
