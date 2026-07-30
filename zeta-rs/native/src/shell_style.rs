use zeta_ui::{
    Color, CornerRadii, Edges, FontFamily, InputBoxStateColors, InputBoxStyle, TextStyle,
};

#[derive(Clone, Copy)]
pub(crate) struct ShellPalette {
    pub(crate) background: Color,
    pub(crate) surface: Color,
    pub(crate) surface_raised: Color,
    pub(crate) border: Color,
    pub(crate) text: Color,
    pub(crate) text_muted: Color,
    pub(crate) accent: Color,
    pub(crate) terminal_selection: Color,
    pub(crate) surface_hovered: Color,
    pub(crate) border_focused: Color,
}

pub(crate) const SHELL_PALETTE: ShellPalette = ShellPalette {
    background: Color::rgb(9, 11, 14),
    surface: Color::rgb(16, 20, 26),
    surface_raised: Color::rgb(22, 27, 35),
    border: Color::rgb(45, 53, 65),
    text: Color::rgb(235, 239, 244),
    text_muted: Color::rgb(145, 157, 173),
    accent: Color::rgb(104, 170, 222),
    terminal_selection: Color::rgba(104, 170, 222, 96),
    surface_hovered: Color::rgb(29, 36, 46),
    border_focused: Color::rgb(74, 125, 171),
};

impl ShellPalette {
    pub(crate) fn composer_style(self) -> InputBoxStyle {
        InputBoxStyle::new(
            InputBoxStateColors::new(
                self.surface_raised,
                self.surface_hovered,
                self.surface_raised,
            ),
            InputBoxStateColors::new(self.border, self.border, self.border_focused),
            TextStyle::new(15.0, self.text)
                .with_family(FontFamily::Monospace)
                .with_line_height(20.0),
            TextStyle::new(15.0, self.text_muted)
                .with_family(FontFamily::Monospace)
                .with_line_height(20.0),
        )
        .with_corner_radii(CornerRadii::uniform(10.0))
        .with_padding(Edges::new(18.0, 16.0, 18.0, 16.0))
        .with_selection_color(self.terminal_selection)
        .with_caret_color(self.accent)
        .with_preedit_underline_color(self.accent)
    }
}
