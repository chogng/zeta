use super::runtime::GuiConfig;
use super::runtime::gui_editor_text_style;
use zui::ui::{Color, FontFamily};

#[test]
fn gui_config_builds_the_editor_text_style_without_theme_typography_defaults() {
    let gui = GuiConfig {
        theme: "zeta-dark".into(),
        editor_font_family: "JetBrains Mono".into(),
        editor_font_size: 15,
        editor_line_height: 24,
    };

    let style = gui_editor_text_style(&gui, Color::WHITE);

    assert_eq!(style.family(), &FontFamily::Named("JetBrains Mono".into()));
    assert_eq!(style.font_size(), 15.0);
    assert_eq!(style.line_height(), 24.0);
    assert_eq!(style.color(), Color::WHITE);
}

#[test]
fn gui_config_maps_semantic_font_families() {
    for (configured, expected) in [
        ("monospace", FontFamily::Monospace),
        ("sans-serif", FontFamily::SansSerif),
        ("serif", FontFamily::Serif),
    ] {
        let style = gui_editor_text_style(
            &GuiConfig {
                theme: "system".into(),
                editor_font_family: configured.into(),
                editor_font_size: 13,
                editor_line_height: 20,
            },
            Color::WHITE,
        );
        assert_eq!(style.family(), &expected);
    }
}
