//! Discovery and validation of trusted language-server launch definitions.
//!
//! This crate owns built-in server identities, user intent, executable candidate resolution, and
//! availability reporting. It never starts a process, reads an editor document, or runs LSP.

mod css;
mod definition;
mod error;
mod node_runtime;
mod provider;
mod resolver;

/// Stable built-in identity for the Rust language server.
pub const RUST_ANALYZER_SERVER_ID: &str = "rust-analyzer";
/// Stable built-in identity for the VS Code JSON/JSONC language server.
pub const JSON_LANGUAGE_SERVER_ID: &str = "vscode-json-language-server";
/// Stable built-in identity for the Bash and shell-script language server.
pub const BASH_LANGUAGE_SERVER_ID: &str = "bash-language-server";
/// Stable built-in identity for JavaScript and TypeScript language intelligence.
pub const TYPESCRIPT_LANGUAGE_SERVER_ID: &str = "typescript-language-server";
/// Stable provider identity for the Marketplace CSS/SCSS/Less language server.
pub const CSS_LANGUAGE_SERVER_ID: &str = "css-language-server";

pub use css::CssLanguageServerProvider;
pub use definition::LanguageServerDefinition;
pub use error::LanguageServerCatalogError;
pub use node_runtime::ManagedNodeRuntime;
pub use node_runtime::ManagedNodeRuntimeSource;
pub use provider::LanguageServerProvider;
pub use provider::LanguageServerProviderError;
pub use provider::LanguageServerProviderLaunch;
pub use provider::LanguageServerProviderRegistry;
pub use resolver::{
    LanguageServerCatalog, LanguageServerCatalogEntry, LanguageServerCatalogResolution,
    LanguageServerCatalogState, LanguageServerExecutableCandidates, LanguageServerExecutionPolicy,
    LanguageServerMode, LanguageServerPreference,
};
#[cfg(test)]
#[path = "provider_tests.rs"]
mod provider_tests;
