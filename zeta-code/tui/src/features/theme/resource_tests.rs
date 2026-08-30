use std::fs;

use ratatui::style::Color;
use zeta_terminal_detection::ColorLevel;
use zeta_terminal_detection::TerminalRgb;
use zeta_theme::ColorScheme;

use super::ThemeResource;

#[test]
fn choices_and_selection_use_the_tui_device_preference() {
    let root = std::env::temp_dir().join(format!("zeta-tui-theme-command-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let resource = ThemeResource::for_test(root.clone(), ColorLevel::TrueColor, ColorScheme::Dark);

    let catalog = resource.catalog().unwrap();
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

    resource.select("zeta-code-dark").unwrap();
    let configuration = fs::read_to_string(root.join("configuration.json")).unwrap();
    assert!(configuration.contains(r#""tui.colorTheme": "zeta-code-dark""#));
    assert!(resource.select("zeta-dark").is_err());

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn ansi_modes_force_ansi_16_colors() {
    let root = std::env::temp_dir().join(format!("zeta-tui-ansi-theme-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let resource = ThemeResource::for_test(root.clone(), ColorLevel::TrueColor, ColorScheme::Dark);

    let ansi = resource.catalog().unwrap().choices[5].palette;
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
