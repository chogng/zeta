use ratatui::style::Color;
use zeta_terminal_detection::ColorLevel;
use zeta_theme::ColorScheme;
use zeta_theme::ThemeCatalog;

use super::RenderTheme;

#[test]
fn render_theme_maps_its_colors_for_each_terminal_capability() {
    let catalog = ThemeCatalog::embedded().unwrap();
    let snapshot = catalog
        .built_in_entry("zeta-code", ColorScheme::Dark)
        .unwrap();

    let true_color = RenderTheme::from_snapshot(&snapshot, ColorLevel::TrueColor).unwrap();
    let ansi256 = RenderTheme::from_snapshot(&snapshot, ColorLevel::Ansi256).unwrap();
    let ansi16 = RenderTheme::from_snapshot(&snapshot, ColorLevel::Ansi16).unwrap();
    let monochrome = RenderTheme::from_snapshot(&snapshot, ColorLevel::Monochrome).unwrap();

    assert!(matches!(true_color.accent(), Color::Rgb(..)));
    assert_eq!(true_color.highlight(), Color::Rgb(154, 145, 235));
    assert!(matches!(ansi256.accent(), Color::Indexed(..)));
    assert!(!matches!(
        ansi16.accent(),
        Color::Rgb(..) | Color::Indexed(..)
    ));
    assert_eq!(monochrome.accent(), Color::Reset);

    let light = RenderTheme::from_snapshot(
        &catalog
            .built_in_entry("zeta-code", ColorScheme::Light)
            .unwrap(),
        ColorLevel::TrueColor,
    )
    .unwrap();
    assert_eq!(light.background(), Color::Rgb(255, 255, 255));
    assert_eq!(light.foreground(), Color::Rgb(31, 35, 40));
}
