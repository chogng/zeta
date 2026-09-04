use super::top_line;
use crate::render::RenderContext;
use crate::render::RenderTheme;
use crate::render::ThemePalette;
use crate::render::ThemeRgb;
use crate::status::model::StatusLineSegment;
use ratatui::style::Color;
use zeta_terminal_detection::ColorLevel;

#[test]
fn git_diff_statistics_use_the_theme_marker_colors() {
    let mut palette = ThemePalette::dark();
    palette.inserted_marker = ThemeRgb::new(1, 2, 3);
    palette.removed_marker = ThemeRgb::new(4, 5, 6);
    let theme = RenderTheme::from_palette(palette, ColorLevel::TrueColor);
    let context = RenderContext::new(&theme, 1);
    let line = top_line(
        vec![
            StatusLineSegment::inserted("+14"),
            StatusLineSegment::chrome(" "),
            StatusLineSegment::removed("-3"),
        ],
        context,
    );

    assert_eq!(line.spans[0].content, "+14");
    assert_eq!(line.spans[0].style.fg, Some(context.inserted_marker()));
    assert_eq!(line.spans[1].content, " ");
    assert_eq!(line.spans[1].style.fg, Some(context.chat_input_chrome()));
    assert_eq!(line.spans[2].content, "-3");
    assert_eq!(line.spans[2].style.fg, Some(context.removed_marker()));
    assert_eq!(context.inserted_marker(), Color::Rgb(1, 2, 3));
    assert_eq!(context.removed_marker(), Color::Rgb(4, 5, 6));
}
