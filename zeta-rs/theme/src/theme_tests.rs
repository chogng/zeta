use std::fs;

use serde::Deserialize;

use crate::tokens;
use crate::{
    ColorScheme, Rgba, ThemeCatalog, ThemeDocument, ThemeLoadOptions, ThemeLoader, ThemeSurface,
};

#[test]
fn embedded_catalog_preserves_aliases_when_a_dependency_is_overridden() {
    let catalog = ThemeCatalog::embedded().unwrap();
    let document = ThemeDocument::parse(
        r##"{
            "version": 1,
            "id": "alias-test",
            "label": "Alias Test",
            "colorScheme": "dark",
            "colors": { "sideBar.background": "#123456" }
        }"##,
    )
    .unwrap();

    let snapshot = catalog.resolve_document(&document).unwrap();

    assert_eq!(
        snapshot.required_color("panel.background").unwrap(),
        Rgba::rgb(18, 52, 86)
    );
}

#[test]
fn embedded_entries_keep_one_token_contract_and_select_product_defaults() {
    let catalog = ThemeCatalog::embedded().unwrap();
    let zeta = catalog.built_in_entry("zeta", ColorScheme::Light).unwrap();
    let zeta_code = catalog
        .built_in_entry("zeta-code", ColorScheme::Light)
        .unwrap();
    let zeterm = catalog
        .built_in_entry("zeterm", ColorScheme::Light)
        .unwrap();

    assert_eq!(zeta.id(), "zeta-light");
    assert_eq!(zeta_code.id(), "zeta-code-light");
    assert_eq!(zeterm.id(), "zeterm-light");
    assert_eq!(
        zeta.colors().keys().collect::<Vec<_>>(),
        zeta_code.colors().keys().collect::<Vec<_>>()
    );
    assert_eq!(
        zeta.colors().keys().collect::<Vec<_>>(),
        zeterm.colors().keys().collect::<Vec<_>>()
    );
    assert_eq!(
        zeta.required_color(tokens::LIST_ACTIVE_SELECTION_BACKGROUND)
            .unwrap(),
        Rgba::rgb(0, 96, 192)
    );
    assert_eq!(
        zeterm
            .required_color(tokens::LIST_ACTIVE_SELECTION_BACKGROUND)
            .unwrap(),
        Rgba::rgb(235, 235, 237)
    );
    assert_eq!(
        zeterm
            .required_color("list.activeSelectionForeground")
            .unwrap(),
        zeterm.required_color(tokens::FOREGROUND).unwrap()
    );
}

#[test]
fn loader_uses_the_host_entry_when_device_preference_follows_system() {
    let root = std::env::temp_dir().join(format!("zeta-theme-entry-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();

    let loaded = ThemeLoader::embedded().unwrap().load(
        ThemeLoadOptions::new(&root, ThemeSurface::Graphical, ColorScheme::Light)
            .with_default_entry("zeterm"),
    );

    assert_eq!(loaded.snapshot.id(), "zeterm-light");
    assert!(loaded.follows_system);
    assert!(loaded.diagnostics.is_empty());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn user_theme_transforms_and_legacy_editor_tokens_match_the_shared_contract() {
    let catalog = ThemeCatalog::embedded().unwrap();
    let document = ThemeDocument::parse(
        r##"{
            "$schema": "https://zeta.dev/schemas/color-theme.schema.json",
            "version": 1,
            "id": "transform-test",
            "label": "Transform Test",
            "colorScheme": "dark",
            "colors": {
                "toolbar.hoverBackground": { "op": "transparent", "value": "#ffffff", "factor": 0.2 },
                "editor.semanticToken.functionForeground": "#ff8800"
            }
        }"##,
    )
    .unwrap();

    let snapshot = catalog.resolve_document(&document).unwrap();

    assert_eq!(
        snapshot
            .required_color("toolbar.hoverBackground")
            .unwrap()
            .to_string(),
        "#ffffff33"
    );
    assert_eq!(
        snapshot
            .required_color(tokens::EDITOR_TOKEN_FUNCTION)
            .unwrap(),
        Rgba::rgb(255, 136, 0)
    );
}

#[test]
fn terminal_surface_consumes_its_optional_preference_and_isolates_broken_files() {
    let root = std::env::temp_dir().join(format!("zeta-theme-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("themes")).unwrap();
    fs::write(
        root.join("configuration.json"),
        r#"{"version":1,"values":{"workbench.colorTheme":"zeta-light","tui.colorTheme":"terminal-test"}}"#,
    )
    .unwrap();
    fs::write(root.join("themes/broken.json"), "{").unwrap();
    fs::write(
        root.join("themes/terminal-test.json"),
        r##"{"version":1,"id":"terminal-test","label":"Terminal Test","colorScheme":"dark","colors":{"accent.foreground":"#abcdef"}}"##,
    )
    .unwrap();

    let loaded = ThemeLoader::embedded().unwrap().load(
        ThemeLoadOptions::new(&root, ThemeSurface::Terminal, ColorScheme::Dark)
            .with_default_entry("zeterm"),
    );

    assert_eq!(loaded.snapshot.id(), "terminal-test");
    assert!(!loaded.follows_system);
    assert_eq!(
        loaded
            .snapshot
            .required_color(tokens::ACCENT_FOREGROUND)
            .unwrap(),
        Rgba::rgb(171, 205, 239)
    );
    assert_eq!(loaded.diagnostics.len(), 1);
    fs::remove_dir_all(root).unwrap();
}

#[derive(Deserialize)]
struct ConformanceFixture {
    theme: serde_json::Value,
    expected: std::collections::BTreeMap<String, String>,
}

#[test]
fn rust_resolver_matches_the_shared_cross_runtime_conformance_fixture() {
    let fixture: ConformanceFixture = serde_json::from_str(include_str!(
        "../../../resources/design-tokens/theme-conformance.json"
    ))
    .unwrap();
    let document = ThemeDocument::parse(&fixture.theme.to_string()).unwrap();
    let snapshot = ThemeCatalog::embedded()
        .unwrap()
        .resolve_document(&document)
        .unwrap();

    for (token, expected) in fixture.expected {
        assert_eq!(
            snapshot.required_color(&token).unwrap().to_string(),
            expected,
            "{token}"
        );
    }
}

#[test]
fn user_theme_document_rejects_values_outside_the_shared_json_schema() {
    for source in [
        r#"{"version":1,"id":"invalid","label":"Invalid","colorScheme":"dark","colors":{"foreground":null}}"#,
        r#"{"version":1,"id":"invalid","label":"Invalid","colorScheme":"dark","colors":{"foreground":"not a token"}}"#,
        r##"{"version":1,"id":"invalid","label":"Invalid","colorScheme":"dark","colors":{"foreground":{"op":"transparent","value":"#fff","factor":2}}}"##,
    ] {
        assert!(ThemeDocument::parse(source).is_err(), "{source}");
    }
}
