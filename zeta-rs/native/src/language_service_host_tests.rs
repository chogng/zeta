use std::collections::BTreeMap;
use std::path::Path;

use zeta_app_server_protocol::protocol::config::{
    ApprovalReviewModelSelectionDto, ConfigReadResult, LanguageServerConfigDto,
    LanguageServerModeDto,
};
use zeta_language_server_catalog::{LanguageServerCatalogState, LanguageServerExecutionPolicy};
use zeta_text_file::{TextFileAccess, TextFileDiskVersion, TextFileModifiedAt, TextFileSnapshot};

use super::{catalog_from_configuration, editor_diagnostic, language_document, language_id};
use crate::file_editor_host::FileEditorHost;

#[test]
fn native_adapter_maps_editor_language_revision_and_workspace_path_without_lsp_types() {
    let mut host = FileEditorHost::default();
    host.open(TextFileSnapshot::new(
        "src/main.rs".into(),
        "fn main() {}".into(),
        TextFileDiskVersion::new(
            12,
            TextFileModifiedAt::KnownMillis(1),
            TextFileAccess::Writable,
        ),
    ));

    let document = language_document(Path::new("/workspace"), host.active().unwrap())
        .expect("language document");

    assert_eq!(document.path(), Path::new("/workspace/src/main.rs"));
    assert_eq!(document.language_id(), "rust");
    assert_eq!(document.revision().value(), 1);
    assert_eq!(document.text(), "fn main() {}");
    assert_eq!(language_id(zeta_editor::CodeEditorLanguage::Jsonc), "jsonc");
}

#[test]
fn native_configuration_maps_persisted_mode_into_catalog_policy() {
    let mut language_servers = BTreeMap::new();
    language_servers.insert(
        "rust-analyzer".into(),
        LanguageServerConfigDto {
            mode: LanguageServerModeDto::Disabled,
            executable: None,
        },
    );
    let configuration = ConfigReadResult {
        revision: 4,
        generation: 7,
        preferred_model: None,
        approval_review_model: ApprovalReviewModelSelectionDto::Automatic,
        providers: BTreeMap::new(),
        mcp_servers: BTreeMap::new(),
        skill_sources: BTreeMap::new(),
        plugin_requests: BTreeMap::new(),
        hooks: BTreeMap::new(),
        language_servers,
    };

    let catalog = catalog_from_configuration(&configuration);
    let resolution = catalog
        .resolve(
            &zeta_install_context::InstallContext::current(),
            LanguageServerExecutionPolicy::Allowed,
            Path::new("/workspace"),
        )
        .expect("catalog");

    assert!(resolution.definitions().is_empty());
    assert_eq!(
        resolution.entries()[0].state(),
        &LanguageServerCatalogState::Disabled
    );
}

#[test]
fn native_adapter_projects_language_diagnostics_without_lsp_presentation_types() {
    let diagnostic = zeta_language_service::LanguageDiagnostic {
        range: zeta_language_service::LanguageTextRange::new(4..9),
        severity: zeta_language_service::LanguageDiagnosticSeverity::Warning,
        message: "unused value".into(),
        source: Some("rustc".into()),
        code: Some("unused_variables".into()),
    };

    let projected = editor_diagnostic(&diagnostic);

    assert_eq!(projected.range(), 4..9);
    assert_eq!(
        projected.severity(),
        zeta_editor::CodeEditorDiagnosticSeverity::Warning
    );
    assert_eq!(projected.message(), "unused value");
    assert_eq!(projected.source(), Some("rustc"));
    assert_eq!(projected.code(), Some("unused_variables"));
}
