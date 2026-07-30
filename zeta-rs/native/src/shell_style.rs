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
    pub(crate) session_tab_highlight: Color,
    pub(crate) border_focused: Color,
}

pub(crate) const SHELL_PALETTE: ShellPalette = ShellPalette {
    background: Color::rgb(252, 252, 253),
    surface: Color::WHITE,
    surface_raised: Color::rgb(246, 246, 247),
    border: Color::rgb(222, 222, 224),
    text: Color::rgb(38, 38, 41),
    text_muted: Color::rgb(126, 126, 132),
    accent: Color::rgb(15, 110, 96),
    terminal_selection: Color::rgba(68, 139, 202, 72),
    surface_hovered: Color::rgb(248, 248, 249),
    session_tab_highlight: Color::rgb(235, 235, 237),
    border_focused: Color::TRANSPARENT,
};

impl ShellPalette {
    pub(crate) fn terminal_indexed_color(self, index: u8) -> Color {
        match index {
            0 => Color::rgb(36, 41, 47),
            1 => Color::rgb(207, 34, 46),
            2 => Color::rgb(17, 99, 41),
            3 => Color::rgb(154, 103, 0),
            4 => Color::rgb(9, 105, 218),
            5 => Color::rgb(130, 80, 223),
            6 => Color::rgb(27, 124, 131),
            7 => self.text,
            8 => Color::rgb(110, 119, 129),
            9 => Color::rgb(164, 14, 38),
            10 => Color::rgb(26, 127, 55),
            11 => Color::rgb(191, 135, 0),
            12 => Color::rgb(33, 139, 255),
            13 => Color::rgb(164, 117, 249),
            14 => Color::rgb(49, 146, 170),
            15 => Color::rgb(140, 149, 159),
            _ => self.text,
        }
    }

    pub(crate) fn composer_style(self) -> InputBoxStyle {
        InputBoxStyle::new(
            InputBoxStateColors::new(self.surface, self.surface_hovered, self.surface),
            InputBoxStateColors::new(
                self.border_focused,
                self.border_focused,
                self.border_focused,
            ),
            TextStyle::new(15.0, self.text)
                .with_family(FontFamily::Monospace)
                .with_line_height(20.0),
            TextStyle::new(15.0, self.text_muted)
                .with_family(FontFamily::Monospace)
                .with_line_height(20.0),
        )
        .with_border_width(0.0)
        .with_corner_radii(CornerRadii::uniform(0.0))
        .with_padding(Edges::new(12.0, 8.0, 12.0, 8.0))
        .with_selection_color(self.terminal_selection)
        .with_caret_color(self.accent)
        .with_preedit_underline_color(self.accent)
    }
}
