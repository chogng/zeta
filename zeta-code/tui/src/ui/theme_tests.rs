use std::fs;

use ratatui::style::Color;
use zeta_terminal_detection::ColorLevel;
use zeta_terminal_detection::TerminalRgb;
use zeta_theme::ColorScheme;
use zeta_theme::ThemeCatalog;

use super::TuiTheme;

#[test]
fn tui_projects_only_its_theme_subset_for_each_terminal_capability() {
    let catalog = ThemeCatalog::embedded().unwrap();
    let snapshot = catalog
        .built_in_entry("zeta-code", ColorScheme::Dark)
        .unwrap();

    let true_color = TuiTheme::from_snapshot(&snapshot, ColorLevel::TrueColor).unwrap();
    let ansi256 = TuiTheme::from_snapshot(&snapshot, ColorLevel::Ansi256).unwrap();
    let ansi16 = TuiTheme::from_snapshot(&snapshot, ColorLevel::Ansi16).unwrap();
    let monochrome = TuiTheme::from_snapshot(&snapshot, ColorLevel::Monochrome).unwrap();

    assert!(matches!(true_color.accent, Color::Rgb(..)));
    assert_eq!(true_color.highlight, Color::Rgb(154, 145, 235));
    assert!(matches!(ansi256.accent, Color::Indexed(..)));
    assert!(!matches!(
        ansi16.accent,
        Color::Rgb(..) | Color::Indexed(..)
    ));
    assert_eq!(monochrome.accent, Color::Reset);

    let light = TuiTheme::from_snapshot(
        &catalog
            .built_in_entry("zeta-code", ColorScheme::Light)
            .unwrap(),
        ColorLevel::TrueColor,
    )
    .unwrap();
    assert_eq!(light.background, Color::Rgb(255, 255, 255));
    assert_eq!(light.foreground, Color::Rgb(31, 35, 40));
}

#[test]
fn theme_choices_and_selection_use_the_tui_device_preference() {
    let root = std::env::temp_dir().join(format!("zeta-tui-theme-command-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);

    let catalog = super::theme_catalog_at(&root, ColorLevel::TrueColor, ColorScheme::Dark).unwrap();
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
    assert!(
        catalog
            .choices
            .iter()
            .all(|choice| !choice.label.contains("app"))
    );

    super::select_theme_at(
        &root,
        "zeta-code-dark",
        ColorLevel::TrueColor,
        ColorScheme::Dark,
    )
    .unwrap();
    let configuration = fs::read_to_string(root.join("configuration.json")).unwrap();
    assert!(configuration.contains(r#""tui.colorTheme": "zeta-code-dark""#));
    assert!(
        super::select_theme_at(&root, "zeta-dark", ColorLevel::TrueColor, ColorScheme::Dark,)
            .is_err()
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn ansi_modes_force_the_ansi_16_projection() {
    let root = std::env::temp_dir().join(format!("zeta-tui-ansi-theme-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);

    let catalog = super::theme_catalog_at(&root, ColorLevel::TrueColor, ColorScheme::Dark).unwrap();
    let ansi = &catalog.choices[5].palette;
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
        super::detect_system_scheme(Some(TerminalRgb::new(245, 245, 245))),
        ColorScheme::Light
    );
    assert_eq!(
        super::detect_system_scheme(Some(TerminalRgb::new(13, 17, 23))),
        ColorScheme::Dark
    );
}
