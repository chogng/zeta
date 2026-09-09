use super::TerminalSettings;
use crate::nls::Language;
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
    assert!(!settings.copy_on_select());
    assert!(!settings.memory_diagnostics());
    assert_eq!(settings.auto_update(), crate::UpdatePolicy::Latest);
    assert_eq!(settings.language(), Language::English);
}

#[test]
fn terminal_settings_update_removes_legacy_fields() {
    let section = FrontendConfigDto(BTreeMap::from([
        (
            "dirPermissions".into(),
            serde_json::json!({"readFiles": true, "writeFiles": true}),
        ),
        ("followUpMode".into(), serde_json::json!("steer")),
    ]));

    let settings = TerminalSettings::from_tui(&section).unwrap();
    let updated = settings.write_to_tui(&section).unwrap();

    assert!(!updated.0.contains_key("dirPermissions"));
    assert!(!updated.0.contains_key("followUpMode"));
    assert!(!settings.copy_on_select());
}

#[test]
fn copy_on_select_round_trips_and_rejects_non_boolean_values() {
    for enabled in [false, true] {
        let section = FrontendConfigDto(BTreeMap::from([
            ("copyOnSelect".into(), serde_json::json!(enabled)),
            ("mouseInteractions".into(), serde_json::json!(!enabled)),
        ]));
        let settings = TerminalSettings::from_tui(&section).unwrap();
        assert_eq!(settings.copy_on_select(), enabled);
        assert_eq!(settings.mouse_interactions(), !enabled);
        let updated = settings.write_to_tui(&section).unwrap();
        assert_eq!(updated.0["copyOnSelect"], serde_json::json!(enabled));
        assert_eq!(updated.0["mouseInteractions"], serde_json::json!(!enabled));
    }
    let section = FrontendConfigDto(BTreeMap::from([(
        "copyOnSelect".into(),
        serde_json::json!("true"),
    )]));
    assert!(TerminalSettings::from_tui(&section).is_err());
}

#[test]
fn automatic_update_policy_round_trips_and_rejects_unknown_values() {
    for (value, expected) in [
        ("latest", crate::UpdatePolicy::Latest),
        ("stable", crate::UpdatePolicy::Stable),
        ("never", crate::UpdatePolicy::Never),
    ] {
        let section = FrontendConfigDto(BTreeMap::from([(
            "autoUpdate".into(),
            serde_json::json!(value),
        )]));
        let settings = TerminalSettings::from_tui(&section).unwrap();
        assert_eq!(settings.auto_update(), expected);
        assert_eq!(
            settings.write_to_tui(&section).unwrap().0["autoUpdate"],
            serde_json::json!(value)
        );
    }
    let section = FrontendConfigDto(BTreeMap::from([(
        "autoUpdate".into(),
        serde_json::json!(true),
    )]));
    assert!(TerminalSettings::from_tui(&section).is_err());
}

#[test]
fn terminal_settings_update_preserves_other_tui_fields() {
    let section = FrontendConfigDto(BTreeMap::from([
        ("theme".into(), serde_json::json!("zeta-code-light")),
        ("futureOption".into(), serde_json::json!({"enabled": true})),
    ]));
    let mut settings = TerminalSettings::default();
    settings.set_mouse_interactions(false);
    settings.set_memory_diagnostics(true);
    settings.set_auto_update(crate::UpdatePolicy::Never);
    settings.set_language(Language::French);

    let updated = settings.write_to_tui(&section).unwrap();

    assert_eq!(updated.0["theme"], serde_json::json!("zeta-code-light"));
    assert_eq!(
        updated.0["futureOption"],
        serde_json::json!({"enabled": true})
    );
    assert_eq!(updated.0["mouseInteractions"], serde_json::json!(false));
    assert_eq!(updated.0["copyOnSelect"], serde_json::json!(false));
    assert_eq!(updated.0["memoryDiagnostics"], serde_json::json!(true));
    assert_eq!(updated.0["autoUpdate"], serde_json::json!("never"));
    assert_eq!(updated.0["language"], serde_json::json!("fr"));
}

#[test]
fn invalid_terminal_values_are_rejected_by_the_tui() {
    let section = FrontendConfigDto(BTreeMap::from([(
        "inputMode".into(),
        serde_json::json!("emacs"),
    )]));

    assert!(TerminalSettings::from_tui(&section).is_err());
}

#[test]
fn supported_languages_round_trip_from_the_tui_table() {
    for (value, expected) in [
        ("en", Language::English),
        ("ja", Language::Japanese),
        ("zh-CN", Language::Chinese),
        ("fr", Language::French),
    ] {
        let section = FrontendConfigDto(BTreeMap::from([(
            "language".into(),
            serde_json::json!(value),
        )]));

        let settings = TerminalSettings::from_tui(&section).unwrap();

        assert_eq!(settings.language(), expected);
        assert_eq!(
            settings.write_to_tui(&section).unwrap().0["language"],
            serde_json::json!(value)
        );
    }
}

#[test]
fn unsupported_language_is_rejected() {
    let section = FrontendConfigDto(BTreeMap::from([(
        "language".into(),
        serde_json::json!("de"),
    )]));

    assert!(TerminalSettings::from_tui(&section).is_err());
}
