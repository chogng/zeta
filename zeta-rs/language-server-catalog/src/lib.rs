//! Discovery and validation of trusted language-server launch definitions.
//!
//! This crate owns built-in server identities, user intent, executable candidate resolution, and
//! availability reporting. It never starts a process, reads an editor document, or runs LSP.

mod definition;
mod error;
mod resolver;

/// Stable built-in identity for the Rust language server.
pub const RUST_ANALYZER_SERVER_ID: &str = "rust-analyzer";
/// Stable built-in identity for the VS Code JSON/JSONC language server.
pub const JSON_LANGUAGE_SERVER_ID: &str = "vscode-json-language-server";
/// Stable built-in identity for the Bash and shell-script language server.
pub const BASH_LANGUAGE_SERVER_ID: &str = "bash-language-server";

pub use definition::LanguageServerDefinition;
pub use error::LanguageServerCatalogError;
pub use resolver::{
    LanguageServerCatalog, LanguageServerCatalogEntry, LanguageServerCatalogResolution,
    LanguageServerCatalogState, LanguageServerExecutableCandidates, LanguageServerExecutionPolicy,
    LanguageServerMode, LanguageServerPreference,
};
