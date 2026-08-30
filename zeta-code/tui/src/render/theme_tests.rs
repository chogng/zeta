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
    assert_eq!(true_color.focus(), Color::Rgb(154, 145, 235));
    assert_eq!(true_color.action_foreground(), Color::Rgb(88, 166, 255));
    assert_eq!(true_color.selection_foreground(), Color::Rgb(240, 237, 255));
    assert_eq!(true_color.selection_background(), Color::Rgb(47, 43, 82));
    assert_eq!(true_color.hover_background(), Color::Rgb(37, 35, 58));
    assert_eq!(true_color.pressed_background(), Color::Rgb(59, 53, 104));
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

#[test]
fn built_in_palettes_keep_the_documented_interaction_colors() {
    let cases = [
        (
            ThemePalette::dark(),
            Color::Rgb(88, 166, 255),
            Color::Rgb(154, 145, 235),
            Color::Rgb(47, 43, 82),
            Color::Rgb(37, 35, 58),
            Color::Rgb(59, 53, 104),
        ),
        (
            ThemePalette::light(),
            Color::Rgb(9, 105, 218),
            Color::Rgb(102, 88, 199),
            Color::Rgb(233, 229, 255),
            Color::Rgb(242, 240, 255),
            Color::Rgb(216, 209, 255),
        ),
        (
            ThemePalette::colorblind_dark(),
            Color::Rgb(88, 166, 255),
            Color::Rgb(88, 166, 255),
            Color::Rgb(18, 41, 75),
            Color::Rgb(23, 42, 70),
            Color::Rgb(31, 79, 133),
        ),
        (
            ThemePalette::colorblind_light(),
            Color::Rgb(9, 105, 218),
            Color::Rgb(9, 105, 218),
            Color::Rgb(221, 244, 255),
            Color::Rgb(238, 248, 255),
            Color::Rgb(182, 227, 255),
        ),
    ];

    for (palette, action, focus, selection, hover, pressed) in cases {
        let theme = RenderTheme::from_palette(palette, ColorLevel::TrueColor);
        assert_eq!(theme.action_foreground(), action);
        assert_eq!(theme.focus(), focus);
        assert_eq!(theme.selection_background(), selection);
        assert_eq!(theme.hover_background(), hover);
        assert_eq!(theme.pressed_background(), pressed);
    }
}
