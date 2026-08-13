use std::path::Path;

use tempfile::TempDir;
use zeta_language_server_catalog::LanguageServerDefinition;
use zeta_language_server_catalog::LanguageServerProvider;
use zeta_language_server_catalog::LanguageServerProviderError;
use zeta_language_server_catalog::LanguageServerProviderLaunch;
use zeta_language_server_catalog::LanguageServerProviderRegistry;
use zeta_language_server_distribution::LanguageServerActivationAuthority;

use super::language_marketplace_runtime::AppServerLanguageMarketplaceRuntime;

#[test]
fn registry_rebuild_preserves_host_injected_providers_without_activation() {
    let root = TempDir::new().unwrap();
    let authority = LanguageServerActivationAuthority::open(root.path()).unwrap();
    let mut base = LanguageServerProviderRegistry::new();
    base.register(BaseProvider).unwrap();
    let runtime = AppServerLanguageMarketplaceRuntime::new(authority, None, base, Vec::new());

    let rebuilt = runtime.registry().unwrap();

    assert!(rebuilt.contains("test-base-language-server"));
    assert!(!rebuilt.activation_enables("test-base-language-server"));
}

struct BaseProvider;

impl LanguageServerProvider for BaseProvider {
    fn id(&self) -> &'static str {
        "test-base-language-server"
    }

    fn languages(&self) -> &'static [&'static str] {
        &["test-language"]
    }

    fn definition(
        &self,
        _workspace_root: &Path,
        _launch: LanguageServerProviderLaunch<'_>,
    ) -> Result<LanguageServerDefinition, LanguageServerProviderError> {
        Err(LanguageServerProviderError::UnsupportedActivatedServer(
            self.id().into(),
        ))
    }
}
