use std::collections::BTreeMap;

use serde_json::json;
use zeta_app_server_protocol::protocol::config::FrontendConfigDto;

use super::StatusLineItem;
use super::StatusLineSettings;

#[test]
fn missing_status_line_uses_every_item_in_default_order() {
    let settings = StatusLineSettings::from_tui(&FrontendConfigDto::default()).unwrap();

    assert_eq!(settings.items().collect::<Vec<_>>(), StatusLineItem::ALL);
}

#[test]
fn configured_order_controls_status_line_order() {
    let settings = StatusLineSettings::from_tui(&FrontendConfigDto(BTreeMap::from([(
        "statusLine".into(),
        json!(["model", "permissions"]),
    )])))
    .unwrap();

    assert_eq!(
        settings.items().collect::<Vec<_>>(),
        vec![StatusLineItem::Model, StatusLineItem::Permissions]
    );
}

#[test]
fn writing_status_line_preserves_other_tui_values() {
    let section = FrontendConfigDto(BTreeMap::from([("theme".into(), json!("system"))]));
    let mut settings = StatusLineSettings::default();
    settings.set(StatusLineItem::GitChanges, false);

    let written = settings.write_to_tui(&section);

    assert_eq!(written.0.get("theme"), Some(&json!("system")));
    assert_eq!(
        written.0.get("statusLine"),
        Some(&json!(["permissions", "model", "git-branch"]))
    );
}

#[test]
fn unknown_and_duplicate_items_are_rejected() {
    for status_line in [json!(["future"]), json!(["model", "model"])] {
        let section = FrontendConfigDto(BTreeMap::from([("statusLine".into(), status_line)]));
        assert!(StatusLineSettings::from_tui(&section).is_err());
    }
}
