use std::collections::BTreeMap;

use zeta_app_server_protocol::protocol::config::{
    ApprovalReviewModelSelectionDto, ConfigReadResult, LanguageServerConfigDto,
    LanguageServerModeDto,
};
use zeta_language_server_catalog::{JSON_LANGUAGE_SERVER_ID, RUST_ANALYZER_SERVER_ID};
use zeta_language_service::LanguageServerState;
use zeta_ui::TextInputCommand;
use zeta_ui::{CaretVisibility, Rect, TextInputLayoutEngine};
use zui::{AccessibilityRole, InteractionFrame, UiDispatch};

use super::{
    LANGUAGE_SERVER_EXECUTABLE_INPUT, LANGUAGE_SERVER_SETTINGS_SAVE, LanguageServerSettings,
    LanguageServerSettingsState, LanguageServerSettingsTarget,
};
use crate::shell_style::SHELL_PALETTE;

#[test]
fn authoritative_configuration_populates_the_editable_draft() {
    let mut settings = LanguageServerSettingsState::default();
    let configuration = configuration(
        4,
        7,
        Some(LanguageServerConfigDto {
            mode: LanguageServerModeDto::Enabled,
            executable: Some("/opt/bin/rust-analyzer".to_owned()),
        }),
    );

    settings.synchronize(&configuration);

    assert_eq!(settings.mode(), LanguageServerModeDto::Enabled);
    assert_eq!(settings.executable_input().text(), "/opt/bin/rust-analyzer");
    assert!(settings.can_reset());
    assert_eq!(settings.configuration().unwrap().0, 4);
    assert_eq!(settings.configuration().unwrap().1, RUST_ANALYZER_SERVER_ID);
}

#[test]
fn missing_preference_projects_the_automatic_product_default() {
    let mut settings = LanguageServerSettingsState::default();

    settings.synchronize(&configuration(2, 3, None));

    assert_eq!(settings.mode(), LanguageServerModeDto::Automatic);
    assert!(settings.executable_input().text().is_empty());
    assert!(!settings.can_reset());
}

#[test]
fn authoritative_removal_clears_a_previous_executable() {
    let mut settings = LanguageServerSettingsState::default();
    settings.synchronize(&configuration(
        1,
        1,
        Some(LanguageServerConfigDto {
            mode: LanguageServerModeDto::Enabled,
            executable: Some("/opt/bin/rust-analyzer".into()),
        }),
    ));

    settings.synchronize(&configuration(2, 2, None));

    assert!(settings.executable_input().text().is_empty());
    assert_eq!(settings.mode(), LanguageServerModeDto::Automatic);
}

#[test]
fn executable_override_requires_an_absolute_path() {
    let mut settings = LanguageServerSettingsState::default();
    settings.synchronize(&configuration(1, 1, None));
    settings.apply_executable(TextInputCommand::Insert("bin/rust-analyzer".to_owned()));

    assert_eq!(
        settings.configuration(),
        Err("Executable override must be an absolute path")
    );
}

#[test]
fn unrelated_authoritative_update_preserves_an_open_dirty_draft() {
    let mut settings = LanguageServerSettingsState::default();
    settings.synchronize(&configuration(1, 1, None));
    settings.open();
    settings.select_mode(LanguageServerModeDto::Enabled);

    settings.synchronize(&configuration(2, 2, None));

    assert_eq!(settings.mode(), LanguageServerModeDto::Enabled);
    assert_eq!(settings.configuration().unwrap().0, 2);
}

#[test]
fn each_catalog_server_retains_an_independent_unsaved_draft() {
    let mut configuration = configuration(1, 1, None);
    configuration.language_servers.insert(
        JSON_LANGUAGE_SERVER_ID.into(),
        LanguageServerConfigDto {
            mode: LanguageServerModeDto::Enabled,
            executable: Some("/opt/bin/json-server".into()),
        },
    );
    let mut settings = LanguageServerSettingsState::default();
    settings.synchronize(&configuration);
    settings.open();
    settings.select_mode(LanguageServerModeDto::Disabled);

    settings.select_server(LanguageServerSettingsTarget::Json);
    assert_eq!(settings.mode(), LanguageServerModeDto::Enabled);
    assert_eq!(settings.executable_input().text(), "/opt/bin/json-server");
    assert_eq!(settings.configuration().unwrap().1, JSON_LANGUAGE_SERVER_ID);

    settings.select_server(LanguageServerSettingsTarget::RustAnalyzer);
    assert_eq!(settings.mode(), LanguageServerModeDto::Disabled);
}

#[test]
fn visible_settings_register_a_modal_input_and_save_action() {
    let mut settings = LanguageServerSettingsState::default();
    settings.synchronize(&configuration(1, 1, None));
    settings.open();
    let dispatch = UiDispatch::default();
    let mut text_layout = TextInputLayoutEngine::new();
    let view = LanguageServerSettings::new(
        Rect::from_xywh(0.0, 0.0, 1_000.0, 700.0),
        &settings,
        CaretVisibility::Visible,
        SHELL_PALETTE,
        &mut text_layout,
        &dispatch,
    )
    .unwrap();
    let mut frame = InteractionFrame::default();

    view.register_interactions(&mut frame);

    let nodes = frame.accessibility_nodes(&dispatch);
    assert!(nodes.iter().any(|node| {
        node.id == LANGUAGE_SERVER_EXECUTABLE_INPUT && node.role == AccessibilityRole::TextInput
    }));
    assert!(nodes.iter().any(|node| {
        node.id == LANGUAGE_SERVER_SETTINGS_SAVE && node.role == AccessibilityRole::Button
    }));
}

#[test]
fn runtime_crash_loop_is_visible_without_becoming_configuration_state() {
    let mut settings = LanguageServerSettingsState::default();
    settings.synchronize(&configuration(1, 1, None));
    settings.open();
    let dispatch = UiDispatch::default();
    let mut text_layout = TextInputLayoutEngine::new();
    let runtime = LanguageServerState::CrashLoop {
        restart_attempts: 5,
        message: "transport closed".into(),
    };
    let view = LanguageServerSettings::new(
        Rect::from_xywh(0.0, 0.0, 1_000.0, 700.0),
        &settings,
        CaretVisibility::Visible,
        SHELL_PALETTE,
        &mut text_layout,
        &dispatch,
    )
    .unwrap()
    .with_runtime_state(&runtime);

    assert_eq!(
        view.runtime_status().unwrap().0,
        "Crash loop after 5 restarts"
    );
    assert_eq!(settings.mode(), LanguageServerModeDto::Automatic);
}

fn configuration(
    revision: u64,
    generation: u64,
    language_server: Option<LanguageServerConfigDto>,
) -> ConfigReadResult {
    let mut language_servers = BTreeMap::new();
    if let Some(language_server) = language_server {
        language_servers.insert(RUST_ANALYZER_SERVER_ID.to_owned(), language_server);
    }
    ConfigReadResult {
        revision,
        generation,
        preferred_model: None,
        approval_review_model: ApprovalReviewModelSelectionDto::Automatic,
        providers: BTreeMap::new(),
        mcp_servers: BTreeMap::new(),
        skill_sources: BTreeMap::new(),
        plugin_requests: BTreeMap::new(),
        hooks: BTreeMap::new(),
        language_servers,
    }
}
