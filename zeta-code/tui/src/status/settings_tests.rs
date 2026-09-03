use std::collections::BTreeMap;

use serde_json::json;
use zeta_app_server_protocol::protocol::config::FrontendConfigDto;

use super::StatusLineItem;
use super::StatusLineSettings;

#[test]
fn missing_status_line_keeps_accounting_items_opt_in() {
    let settings = StatusLineSettings::from_tui(&FrontendConfigDto::default()).unwrap();

    assert_eq!(
        settings.items().collect::<Vec<_>>(),
        vec![
            StatusLineItem::Permissions,
            StatusLineItem::Model,
            StatusLineItem::GitBranch,
            StatusLineItem::GitChanges,
        ]
    );
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
fn accounting_items_round_trip_when_enabled() {
    let section = FrontendConfigDto(BTreeMap::from([(
        "statusLine".into(),
        json!(["cache-hit-rate", "reference-cost"]),
    )]));

    let settings = StatusLineSettings::from_tui(&section).unwrap();

    assert_eq!(
        settings.items().collect::<Vec<_>>(),
        vec![StatusLineItem::CacheHitRate, StatusLineItem::ReferenceCost]
    );
    assert_eq!(
        settings.write_to_tui(&FrontendConfigDto::default()),
        section
    );
}

#[test]
fn unknown_and_duplicate_items_are_rejected() {
    for status_line in [json!(["future"]), json!(["model", "model"])] {
        let section = FrontendConfigDto(BTreeMap::from([("statusLine".into(), status_line)]));
        assert!(StatusLineSettings::from_tui(&section).is_err());
    }
}
