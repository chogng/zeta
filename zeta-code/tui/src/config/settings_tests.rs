use super::TerminalSettings;
use std::collections::BTreeMap;
use zeta_app_server_protocol::protocol::config::FrontendConfigDto;

#[test]
fn tui_table_defaults_missing_terminal_fields() {
    let section = FrontendConfigDto(BTreeMap::from([(
        "theme".into(),
        serde_json::json!("zeta-code-light"),
    )]));

    let settings = TerminalSettings::from_tui(&section).unwrap();

    assert!(settings.mouse_interactions());
    assert!(
        settings
            .dir_permissions()
            .contains(&zeta_app_server_protocol::protocol::environment::PermissionDto::ReadFiles)
    );
}

#[test]
fn terminal_settings_update_preserves_other_tui_fields() {
    let section = FrontendConfigDto(BTreeMap::from([
        ("theme".into(), serde_json::json!("zeta-code-light")),
        ("futureOption".into(), serde_json::json!({"enabled": true})),
    ]));
    let mut settings = TerminalSettings::default();
    settings.set_mouse_interactions(false);

    let updated = settings.write_to_tui(&section).unwrap();

    assert_eq!(updated.0["theme"], serde_json::json!("zeta-code-light"));
    assert_eq!(
        updated.0["futureOption"],
        serde_json::json!({"enabled": true})
    );
    assert_eq!(updated.0["mouseInteractions"], serde_json::json!(false));
}

#[test]
fn invalid_terminal_values_are_rejected_by_the_tui() {
    let section = FrontendConfigDto(BTreeMap::from([(
        "inputMode".into(),
        serde_json::json!("emacs"),
    )]));

    assert!(TerminalSettings::from_tui(&section).is_err());
}
