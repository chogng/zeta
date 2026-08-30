use ratatui::style::Color;
use zeta_terminal_detection::ColorLevel;
use zeta_theme::Rgba;
use zeta_theme::ThemeError;
use zeta_theme::ThemeSnapshot;
use zeta_theme::tokens;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RenderTheme {
    accent: Color,
    background: Color,
    border: Color,
    chat_input_chrome: Color,
    danger: Color,
    foreground: Color,
    function: Color,
    highlight: Color,
    inserted_background: Color,
    inserted_marker: Color,
    keyword: Color,
    muted: Color,
    removed_background: Color,
    removed_marker: Color,
    string: Color,
    success: Color,
    r#type: Color,
    variable: Color,
    warning: Color,
}

impl RenderTheme {
    pub(crate) fn from_snapshot(
        snapshot: &ThemeSnapshot,
        capability: ColorLevel,
    ) -> Result<Self, ThemeError> {
        let capability = if snapshot.id().starts_with("zeta-code-ansi-") {
            ColorLevel::Ansi16
        } else {
            capability
        };
        let background = snapshot.required_color(tokens::TERMINAL_BACKGROUND)?;
        let projected = |token| {
            snapshot
                .required_color(token)
                .map(|color| terminal_color(color, background, capability))
        };
        Ok(Self {
            accent: projected(tokens::ACCENT_FOREGROUND)?,
            background: projected(tokens::TERMINAL_BACKGROUND)?,
            border: projected(tokens::BORDER)?,
            chat_input_chrome: projected(tokens::DESCRIPTION_FOREGROUND)?,
            danger: projected(tokens::ERROR_FOREGROUND)?,
            foreground: projected(tokens::EDITOR_FOREGROUND)?,
            function: projected(tokens::EDITOR_TOKEN_FUNCTION)?,
            highlight: projected(tokens::TUI_HIGHLIGHT_FOREGROUND)?,
            inserted_background: projected(tokens::DIFF_INSERTED_LINE_BACKGROUND)?,
            inserted_marker: projected(tokens::DIFF_INSERTED_LINE_MARKER)?,
            keyword: projected(tokens::EDITOR_TOKEN_KEYWORD)?,
            muted: projected(tokens::MUTED_FOREGROUND)?,
            removed_background: projected(tokens::DIFF_REMOVED_LINE_BACKGROUND)?,
            removed_marker: projected(tokens::DIFF_REMOVED_LINE_MARKER)?,
            string: projected(tokens::EDITOR_TOKEN_STRING)?,
            success: projected(tokens::SUCCESS_FOREGROUND)?,
            r#type: projected(tokens::EDITOR_TOKEN_TYPE)?,
            variable: projected(tokens::EDITOR_TOKEN_VARIABLE)?,
            warning: projected(tokens::WARNING_FOREGROUND)?,
        })
    }

    pub(crate) const fn fallback() -> Self {
        Self {
            accent: Color::Rgb(105, 170, 255),
            background: Color::Rgb(13, 17, 23),
            border: Color::DarkGray,
            chat_input_chrome: Color::Rgb(155, 155, 155),
            danger: Color::Rgb(245, 105, 105),
            foreground: Color::White,
            function: Color::Rgb(210, 168, 255),
            highlight: Color::Rgb(154, 145, 235),
            inserted_background: Color::Rgb(19, 48, 28),
            inserted_marker: Color::Rgb(63, 185, 80),
            keyword: Color::Rgb(255, 123, 114),
            muted: Color::DarkGray,
            removed_background: Color::Rgb(55, 25, 27),
            removed_marker: Color::Rgb(248, 81, 73),
            string: Color::Rgb(165, 214, 255),
            success: Color::Rgb(95, 210, 140),
            r#type: Color::Rgb(210, 168, 255),
            variable: Color::Rgb(255, 166, 87),
            warning: Color::Rgb(245, 190, 80),
        }
    }

    pub(crate) const fn accent(self) -> Color {
        self.accent
    }
    pub(crate) const fn background(self) -> Color {
        self.background
    }
    pub(crate) const fn border(self) -> Color {
        self.border
    }
    pub(crate) const fn chat_input_chrome(self) -> Color {
        self.chat_input_chrome
    }
    pub(crate) const fn danger(self) -> Color {
        self.danger
    }
    pub(crate) const fn foreground(self) -> Color {
        self.foreground
    }
    pub(crate) const fn function(self) -> Color {
        self.function
    }
    pub(crate) const fn highlight(self) -> Color {
        self.highlight
    }
    pub(crate) const fn inserted_background(self) -> Color {
        self.inserted_background
    }
    pub(crate) const fn inserted_marker(self) -> Color {
        self.inserted_marker
    }
    pub(crate) const fn keyword(self) -> Color {
        self.keyword
    }
    pub(crate) const fn muted(self) -> Color {
        self.muted
    }
    pub(crate) const fn removed_background(self) -> Color {
        self.removed_background
    }
    pub(crate) const fn removed_marker(self) -> Color {
        self.removed_marker
    }
    pub(crate) const fn string(self) -> Color {
        self.string
    }
    pub(crate) const fn success(self) -> Color {
        self.success
    }
    pub(crate) const fn r#type(self) -> Color {
        self.r#type
    }
    pub(crate) const fn variable(self) -> Color {
        self.variable
    }
    pub(crate) const fn warning(self) -> Color {
        self.warning
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct RenderContext<'a> {
    theme: &'a RenderTheme,
}

impl<'a> RenderContext<'a> {
    pub(crate) const fn new(theme: &'a RenderTheme) -> Self {
        Self { theme }
    }

    pub(crate) const fn accent(self) -> Color {
        self.theme.accent()
    }
    pub(crate) const fn background(self) -> Color {
        self.theme.background()
    }
    pub(crate) const fn chat_input_chrome(self) -> Color {
        self.theme.chat_input_chrome()
    }
    pub(crate) const fn danger(self) -> Color {
        self.theme.danger()
    }
    pub(crate) const fn foreground(self) -> Color {
        self.theme.foreground()
    }
    pub(crate) const fn highlight(self) -> Color {
        self.theme.highlight()
    }
    pub(crate) const fn muted(self) -> Color {
        self.theme.muted()
    }
    pub(crate) const fn success(self) -> Color {
        self.theme.success()
    }
    pub(crate) const fn warning(self) -> Color {
        self.theme.warning()
    }
}

#[cfg(test)]
pub(crate) fn test_context() -> RenderContext<'static> {
    static THEME: RenderTheme = RenderTheme::fallback();
    RenderContext::new(&THEME)
}

fn terminal_color(foreground: Rgba, background: Rgba, capability: ColorLevel) -> Color {
    let rgb = composite(foreground, background);
    match capability {
        ColorLevel::TrueColor => Color::Rgb(rgb[0], rgb[1], rgb[2]),
        ColorLevel::Ansi256 => Color::Indexed(nearest_ansi256(rgb)),
        ColorLevel::Ansi16 => nearest_ansi16(rgb),
        ColorLevel::Monochrome => Color::Reset,
    }
}

fn composite(foreground: Rgba, background: Rgba) -> [u8; 3] {
    let [red, green, blue, alpha] = foreground.components();
    if alpha == 255 {
        return [red, green, blue];
    }
    let [background_red, background_green, background_blue, _] = background.components();
    let blend = |foreground: u8, background: u8| {
        let alpha = u32::from(alpha);
        ((u32::from(foreground) * alpha + u32::from(background) * (255 - alpha) + 127) / 255) as u8
    };
    [
        blend(red, background_red),
        blend(green, background_green),
        blend(blue, background_blue),
    ]
}

fn nearest_ansi256(rgb: [u8; 3]) -> u8 {
    let mut best = (0_u8, u32::MAX);
    for index in 16_u8..=255 {
        let candidate = ansi256_rgb(index);
        let distance = color_distance(rgb, candidate);
        if distance < best.1 {
            best = (index, distance);
        }
    }
    best.0
}

fn ansi256_rgb(index: u8) -> [u8; 3] {
    if index >= 232 {
        let gray = 8 + (index - 232) * 10;
        return [gray, gray, gray];
    }
    let cube = index - 16;
    let channel = |value: u8| if value == 0 { 0 } else { 55 + value * 40 };
    [
        channel(cube / 36),
        channel((cube % 36) / 6),
        channel(cube % 6),
    ]
}

fn nearest_ansi16(rgb: [u8; 3]) -> Color {
    const COLORS: [([u8; 3], Color); 16] = [
        ([0, 0, 0], Color::Black),
        ([128, 0, 0], Color::Red),
        ([0, 128, 0], Color::Green),
        ([128, 128, 0], Color::Yellow),
        ([0, 0, 128], Color::Blue),
        ([128, 0, 128], Color::Magenta),
        ([0, 128, 128], Color::Cyan),
        ([192, 192, 192], Color::Gray),
        ([128, 128, 128], Color::DarkGray),
        ([255, 0, 0], Color::LightRed),
        ([0, 255, 0], Color::LightGreen),
        ([255, 255, 0], Color::LightYellow),
        ([0, 0, 255], Color::LightBlue),
        ([255, 0, 255], Color::LightMagenta),
        ([0, 255, 255], Color::LightCyan),
        ([255, 255, 255], Color::White),
    ];
    COLORS
        .iter()
        .min_by_key(|(candidate, _)| color_distance(rgb, *candidate))
        .map(|(_, color)| *color)
        .unwrap_or(Color::Reset)
}

fn color_distance(left: [u8; 3], right: [u8; 3]) -> u32 {
    left.into_iter()
        .zip(right)
        .map(|(left, right)| {
            let difference = i32::from(left) - i32::from(right);
            difference.unsigned_abs().pow(2)
        })
        .sum()
}

#[cfg(test)]
#[path = "theme_tests.rs"]
mod tests;
