use zeta_ui::Color;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum ShellTheme {
    #[default]
    Midnight,
    Daylight,
}

impl ShellTheme {
    pub(crate) const fn next(self) -> Self {
        match self {
            Self::Midnight => Self::Daylight,
            Self::Daylight => Self::Midnight,
        }
    }

    pub(crate) const fn toggle_label(self) -> &'static str {
        match self {
            Self::Midnight => "Light mode",
            Self::Daylight => "Dark mode",
        }
    }

    pub(crate) const fn palette(self) -> ShellPalette {
        match self {
            Self::Midnight => ShellPalette::MIDNIGHT,
            Self::Daylight => ShellPalette::DAYLIGHT,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct ShellPalette {
    pub(crate) background: Color,
    pub(crate) surface: Color,
    pub(crate) surface_raised: Color,
    pub(crate) surface_hovered: Color,
    pub(crate) surface_pressed: Color,
    pub(crate) surface_selected: Color,
    pub(crate) border: Color,
    pub(crate) border_focused: Color,
    pub(crate) text: Color,
    pub(crate) text_muted: Color,
    pub(crate) accent: Color,
}

impl ShellPalette {
    const MIDNIGHT: Self = Self {
        background: Color::rgb(9, 11, 14),
        surface: Color::rgb(16, 20, 26),
        surface_raised: Color::rgb(22, 27, 35),
        surface_hovered: Color::rgb(29, 36, 46),
        surface_pressed: Color::rgb(39, 50, 64),
        surface_selected: Color::rgb(35, 45, 58),
        border: Color::rgb(45, 53, 65),
        border_focused: Color::rgb(74, 125, 171),
        text: Color::rgb(235, 239, 244),
        text_muted: Color::rgb(145, 157, 173),
        accent: Color::rgb(104, 170, 222),
    };

    const DAYLIGHT: Self = Self {
        background: Color::rgb(228, 233, 239),
        surface: Color::rgb(244, 247, 250),
        surface_raised: Color::rgb(255, 255, 255),
        surface_hovered: Color::rgb(232, 240, 248),
        surface_pressed: Color::rgb(216, 230, 244),
        surface_selected: Color::rgb(219, 233, 247),
        border: Color::rgb(190, 200, 212),
        border_focused: Color::rgb(52, 112, 166),
        text: Color::rgb(28, 34, 42),
        text_muted: Color::rgb(88, 100, 115),
        accent: Color::rgb(34, 111, 174),
    };
}
