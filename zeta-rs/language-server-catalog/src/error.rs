/// Static catalog-definition or executable-name validation failure.
#[derive(Debug, thiserror::Error)]
pub enum LanguageServerCatalogError {
    #[error(transparent)]
    InvalidExecutableName(#[from] zeta_install_context::InvalidHostExecutableName),
    #[error(transparent)]
    InvalidDefinition(#[from] zeta_lsp::LanguageServerRouterError),
}
