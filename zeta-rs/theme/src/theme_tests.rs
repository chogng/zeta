use std::fs;

use serde::Deserialize;

use crate::ColorScheme;
use crate::Rgba;
use crate::ThemeCatalog;
use crate::ThemeChoiceKind;
use crate::ThemeDocument;
use crate::ThemeLoadOptions;
use crate::ThemeLoader;
use crate::ThemeSizeUnit;
use crate::ThemeSurface;
use crate::loader::resolve_device_root;
use crate::tokens;

#[test]
fn device_preferences_share_the_profile_root_by_default() {
    assert_eq!(
        resolve_device_root(None, None, Some("/home/ada".into())),
        std::path::PathBuf::from("/home/ada/.zeta")
    );
    assert_eq!(
        resolve_device_root(
            None,
            Some("/profiles/zeta".into()),
            Some("/home/ignored".into()),
        ),
        std::path::PathBuf::from("/profiles/zeta")
    );
    assert_eq!(
        resolve_device_root(
            Some("/devices/zeta".into()),
            Some("/profiles/zeta".into()),
            Some("/home/ignored".into()),
        ),
        std::path::PathBuf::from("/devices/zeta")
    );
}

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
    let app = catalog.built_in_entry("app", ColorScheme::Light).unwrap();

    assert_eq!(zeta.id(), "zeta-light");
    assert_eq!(zeta_code.id(), "zeta-code-light");
    assert_eq!(app.id(), "app-light");
    assert_eq!(
        zeta.colors().keys().collect::<Vec<_>>(),
        zeta_code.colors().keys().collect::<Vec<_>>()
    );
    assert_eq!(
        zeta.colors().keys().collect::<Vec<_>>(),
        app.colors().keys().collect::<Vec<_>>()
    );
    assert_eq!(
        zeta.required_color(tokens::LIST_ACTIVE_SELECTION_BACKGROUND)
            .unwrap(),
        Rgba::rgb(0, 96, 192)
    );
    assert_eq!(
        app.required_color(tokens::LIST_ACTIVE_SELECTION_BACKGROUND)
            .unwrap(),
        Rgba::rgb(235, 235, 237)
    );
    assert_eq!(
        app.required_color("list.activeSelectionForeground")
            .unwrap(),
        app.required_color(tokens::FOREGROUND).unwrap()
    );
}

#[test]
fn embedded_snapshots_expose_typed_shared_size_tokens() {
    let snapshot = ThemeCatalog::embedded()
        .unwrap()
        .built_in(ColorScheme::Light)
        .unwrap();

    let body = snapshot.size(tokens::FONT_SIZE_BODY1).unwrap();
    assert_eq!(body.unit(), ThemeSizeUnit::Pixels);
    assert_eq!(body.value(), 13.0);
    assert_eq!(
        snapshot
            .required_pixel_size(tokens::FONT_SIZE_LABEL1)
            .unwrap(),
        12.0
    );
    assert_eq!(
        snapshot
            .required_size("fontWeight.semiBold")
            .unwrap()
            .unit(),
        ThemeSizeUnit::Unitless
    );
    assert_eq!(
        snapshot
            .required_size("animation.durationFast")
            .unwrap()
            .as_milliseconds(),
        Some(120.0)
    );
    assert_eq!(
        snapshot
            .required_pixel_size(tokens::SCROLLBAR_SIZE)
            .unwrap(),
        10.0
    );
    assert!(snapshot.required_pixel_size("fontWeight.semiBold").is_err());
}

#[test]
fn loader_uses_the_host_entry_when_device_preference_follows_system() {
    let root = std::env::temp_dir().join(format!("zeta-theme-entry-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();

    let loaded = ThemeLoader::embedded().unwrap().load(
        ThemeLoadOptions::new(&root, ThemeSurface::Graphical, ColorScheme::Light)
            .with_default_entry("app"),
    );

    assert_eq!(loaded.snapshot.id(), "app-light");
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
            .with_default_entry("app"),
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

#[test]
fn terminal_theme_selection_is_immediate_and_preserves_other_device_values() {
    let root = std::env::temp_dir().join(format!("zeta-theme-selection-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("configuration.json"),
        r#"{"version":1,"values":{"workbench.colorTheme":"zeta-light","window.zoomLevel":2}}"#,
    )
    .unwrap();
    let loader = ThemeLoader::embedded().unwrap();

    let selected = loader
        .select(
            ThemeLoadOptions::new(&root, ThemeSurface::Terminal, ColorScheme::Dark),
            "zeta-dark",
        )
        .unwrap();

    assert_eq!(selected.snapshot.id(), "zeta-dark");
    let persisted: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(root.join("configuration.json")).unwrap())
            .unwrap();
    assert_eq!(persisted["values"]["tui.colorTheme"], "zeta-dark");
    assert_eq!(persisted["values"]["workbench.colorTheme"], "zeta-light");
    assert_eq!(persisted["values"]["window.zoomLevel"], 2);
    assert_eq!(
        loader
            .load(ThemeLoadOptions::new(
                &root,
                ThemeSurface::Terminal,
                ColorScheme::Dark,
            ))
            .snapshot
            .id(),
        "zeta-dark"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn theme_preview_resolves_without_persisting_the_candidate() {
    let root = std::env::temp_dir().join(format!("zeta-theme-preview-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let loader = ThemeLoader::embedded().unwrap();
    let options = ThemeLoadOptions::new(&root, ThemeSurface::Terminal, ColorScheme::Dark)
        .with_default_entry("zeta-code");

    let preview = loader.preview(options, "zeta-code-light").unwrap();

    assert_eq!(preview.snapshot.id(), "zeta-code-light");
    assert!(!root.join("configuration.json").exists());
    assert_eq!(loader.load(options).snapshot.id(), "zeta-code-dark");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn theme_choices_include_system_builtins_and_valid_user_themes() {
    let root = std::env::temp_dir().join(format!("zeta-theme-choices-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("themes")).unwrap();
    fs::write(
        root.join("themes/aurora.json"),
        r##"{"version":1,"id":"aurora","label":"Aurora","colorScheme":"dark","colors":{"tui.highlightForeground":"#abcdef"}}"##,
    )
    .unwrap();

    let choices = ThemeLoader::embedded()
        .unwrap()
        .choices(ThemeLoadOptions::new(
            &root,
            ThemeSurface::Terminal,
            ColorScheme::Dark,
        ));

    assert_eq!(choices.selected, "system");
    assert!(
        choices
            .themes
            .iter()
            .any(|theme| { theme.id == "system" && theme.kind == ThemeChoiceKind::System })
    );
    assert!(
        choices
            .themes
            .iter()
            .any(|theme| { theme.id == "zeta-dark" && theme.kind == ThemeChoiceKind::BuiltIn })
    );
    assert!(
        choices
            .themes
            .iter()
            .any(|theme| { theme.id == "aurora" && theme.kind == ThemeChoiceKind::User })
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn unavailable_theme_selection_does_not_rewrite_device_configuration() {
    let root = std::env::temp_dir().join(format!("zeta-theme-unavailable-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let configuration = r#"{"version":1,"values":{"workbench.colorTheme":"zeta-light"}}"#;
    fs::write(root.join("configuration.json"), configuration).unwrap();

    let error = ThemeLoader::embedded()
        .unwrap()
        .select(
            ThemeLoadOptions::new(&root, ThemeSurface::Terminal, ColorScheme::Dark),
            "missing-theme",
        )
        .unwrap_err();

    assert_eq!(error.to_string(), "theme 'missing-theme' is unavailable");
    assert_eq!(
        fs::read_to_string(root.join("configuration.json")).unwrap(),
        configuration
    );
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
