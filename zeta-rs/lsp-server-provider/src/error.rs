/// Static server-definition or executable-name validation failure.
#[derive(Debug, thiserror::Error)]
pub enum LspServerResolverError {
    #[error(transparent)]
    InvalidExecutableName(#[from] zeta_install_context::InvalidHostExecutableName),
    #[error(transparent)]
    InvalidDefinition(Box<zeta_lsp::LanguageServerRouterError>),
}

impl From<zeta_lsp::LanguageServerRouterError> for LspServerResolverError {
    fn from(error: zeta_lsp::LanguageServerRouterError) -> Self {
        Self::InvalidDefinition(Box::new(error))
    }
}
