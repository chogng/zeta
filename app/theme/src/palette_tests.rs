use super::UiTheme;
use zeta_theme::ColorScheme;
use zeta_theme::ThemeCatalog;
use zui::ui::Color;

#[test]
fn snapshot_projection_resolves_layout_component_tokens_and_standard_sizes() {
    let snapshot = ThemeCatalog::embedded()
        .unwrap()
        .built_in(ColorScheme::Light)
        .unwrap();
    let theme = UiTheme::from_snapshot(&snapshot).unwrap();

    assert_eq!(theme.font_size_body, 13.0);
    assert_eq!(theme.font_size_label, 12.0);
    assert_eq!(theme.scrollbar_size, 10.0);
    assert_eq!(theme.hover_foreground, Color::rgb(245, 245, 247));
    assert_eq!(theme.hover_background, Color::rgb(45, 46, 51));
    assert_eq!(theme.hover_border, Color::rgba(255, 255, 255, 24));
    assert_eq!(theme.hover_shadow, Color::rgba(0, 0, 0, 48));
    assert_eq!(theme.menu_hover_background, Color::rgb(226, 226, 228));
    assert_eq!(theme.action_bar_background, Color::rgb(245, 245, 246));
    assert_eq!(theme.tab_hover_background, Color::rgb(226, 226, 228));
    assert_eq!(theme.editor_foreground, Color::rgb(51, 51, 51));
    assert_eq!(theme.editor_syntax.comment, Color::rgb(0, 128, 0));
}
