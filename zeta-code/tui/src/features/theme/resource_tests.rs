use std::fs;

use ratatui::style::Color;
use zeta_terminal_detection::ColorLevel;
use zeta_terminal_detection::TerminalRgb;

use super::ThemeAppearance;
use super::ThemeResource;

#[test]
fn catalog_and_resolution_use_the_supplied_config_preference() {
    let root = std::env::temp_dir().join(format!("zeta-tui-theme-command-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let resource =
        ThemeResource::for_test(root.clone(), ColorLevel::TrueColor, ThemeAppearance::Dark);

    let catalog = resource.catalog("system").unwrap();
    assert_eq!(catalog.choices.len(), 8);
    assert_eq!(catalog.choices[0].label, "Auto");
    assert_eq!(catalog.choices[0].palette_label, "GitHub Dark");
    assert!(catalog.choices[0].selected);
    assert_eq!(catalog.choices[7].label, "Custom color theme");
    assert_eq!(catalog.choices[3].palette_label, "GitHub Dark Colorblind");
    assert_eq!(
        catalog.choices[5].palette_label,
        "GitHub Dark · ANSI 16 colors"
    );

    let selection = resource.resolve("zeta-code-dark").unwrap();
    assert_eq!(selection.theme.background(), Color::Rgb(13, 17, 23));
    assert!(resource.catalog("zeta-code-dark").unwrap().choices[1].selected);
    assert!(resource.resolve("zeta-dark").is_err());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn ansi_modes_force_ansi_16_colors() {
    let root = std::env::temp_dir().join(format!("zeta-tui-ansi-theme-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let resource =
        ThemeResource::for_test(root.clone(), ColorLevel::TrueColor, ThemeAppearance::Dark);

    let ansi = resource.catalog("system").unwrap().choices[5].palette;
    assert!(!matches!(ansi.keyword, Color::Rgb(..) | Color::Indexed(..)));
    assert!(!matches!(
        ansi.inserted_marker,
        Color::Rgb(..) | Color::Indexed(..)
    ));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn auto_uses_the_terminal_reported_background_scheme() {
    assert_eq!(
        super::detect_system_appearance(Some(TerminalRgb::new(245, 245, 245))),
        ThemeAppearance::Light
    );
    assert_eq!(
        super::detect_system_appearance(Some(TerminalRgb::new(13, 17, 23))),
        ThemeAppearance::Dark
    );
}

#[test]
fn custom_themes_are_read_only_from_the_zeta_code_theme_directory() {
    let profile_root =
        std::env::temp_dir().join(format!("zeta-tui-custom-theme-{}", std::process::id()));
    let product_root = profile_root.join("zeta-code");
    let _ = fs::remove_dir_all(&profile_root);
    fs::create_dir_all(product_root.join("themes")).unwrap();
    fs::write(
        product_root.join("themes/graphite.json"),
        r##"{
  "schemaVersion": 1,
  "id": "graphite",
  "label": "Graphite",
  "appearance": "dark",
  "colors": {
    "background": "#101010",
    "quickViewBackground": "#303030",
    "screenSelectionBackground": "#80cfff",
    "screenSelectionForeground": "#101820"
  }
}"##,
    )
    .unwrap();
    fs::write(
        profile_root.join("configuration.json"),
        r#"{"version":1,"values":{"workbench.colorTheme":"desktop-theme"}}"#,
    )
    .unwrap();
    let resource =
        ThemeResource::for_test(product_root, ColorLevel::TrueColor, ThemeAppearance::Dark);

    let catalog = resource.catalog("system").unwrap();
    assert!(catalog.choices[0].selected);
    assert_eq!(catalog.custom_choices.len(), 1);
    assert_eq!(catalog.custom_choices[0].label, "Graphite");

    let selected = resource.resolve("graphite").unwrap();
    assert_eq!(selected.theme.background(), Color::Rgb(16, 16, 16));
    assert_eq!(
        selected.theme.quick_view_background(),
        Color::Rgb(48, 48, 48)
    );
    assert_eq!(
        selected.theme.screen_selection_background(),
        Color::Rgb(128, 207, 255)
    );
    assert_eq!(
        selected.theme.screen_selection_foreground(),
        Color::Rgb(16, 24, 32)
    );

    fs::remove_dir_all(profile_root).unwrap();
}

#[test]
fn graphical_theme_tokens_are_rejected_by_the_tui_theme_reader() {
    let product_root = std::env::temp_dir().join(format!(
        "zeta-tui-graphical-theme-token-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&product_root);
    fs::create_dir_all(product_root.join("themes")).unwrap();
    fs::write(
        product_root.join("themes/desktop.json"),
        r##"{
  "schemaVersion": 1,
  "id": "desktop",
  "label": "Desktop",
  "appearance": "dark",
  "colors": { "editor.background": "#101010" }
}"##,
    )
    .unwrap();
    let resource = ThemeResource::for_test(
        product_root.clone(),
        ColorLevel::TrueColor,
        ThemeAppearance::Dark,
    );

    let loaded = resource.load("system").unwrap();

    assert_eq!(loaded.theme.background(), Color::Rgb(13, 17, 23));
    assert!(
        loaded
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("unknown TUI theme color 'editor.background'"))
    );
    fs::remove_dir_all(product_root).unwrap();
}
