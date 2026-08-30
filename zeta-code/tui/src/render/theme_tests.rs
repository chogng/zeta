use ratatui::style::Color;
use zeta_terminal_detection::ColorLevel;

use super::RenderTheme;
use super::ThemePalette;

#[test]
fn render_theme_maps_its_colors_for_each_terminal_capability() {
    let palette = ThemePalette::dark();
    let true_color = RenderTheme::from_palette(palette, ColorLevel::TrueColor);
    let ansi256 = RenderTheme::from_palette(palette, ColorLevel::Ansi256);
    let ansi16 = RenderTheme::from_palette(palette, ColorLevel::Ansi16);
    let monochrome = RenderTheme::from_palette(palette, ColorLevel::Monochrome);

    assert!(matches!(true_color.accent(), Color::Rgb(..)));
    assert_eq!(true_color.highlight(), Color::Rgb(154, 145, 235));
    assert_eq!(
        true_color.active_selection_foreground(),
        Color::Rgb(0, 0, 0)
    );
    assert_eq!(
        true_color.active_selection_background(),
        Color::Rgb(192, 192, 192)
    );
    assert_eq!(
        true_color.screen_selection_foreground(),
        Color::Rgb(13, 17, 23)
    );
    assert_eq!(
        true_color.screen_selection_background(),
        Color::Rgb(135, 206, 235)
    );
    assert_eq!(true_color.quick_view_background(), Color::Rgb(37, 37, 38));
    assert!(matches!(ansi256.accent(), Color::Indexed(..)));
    assert!(!matches!(
        ansi16.accent(),
        Color::Rgb(..) | Color::Indexed(..)
    ));
    assert_eq!(monochrome.accent(), Color::Reset);

    let light = RenderTheme::from_palette(ThemePalette::light(), ColorLevel::TrueColor);
    assert_eq!(light.background(), Color::Rgb(255, 255, 255));
    assert_eq!(light.foreground(), Color::Rgb(31, 35, 40));
    assert_eq!(light.quick_view_background(), Color::Rgb(248, 248, 248));
}
