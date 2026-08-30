use ratatui::style::Color;
use zeta_terminal_detection::ColorLevel;
use zeta_theme::Rgba;
use zeta_theme::ThemeError;
use zeta_theme::ThemeSnapshot;
use zeta_theme::tokens;

const fn hex(value: &str) -> Color {
    let bytes = value.as_bytes();
    assert!(
        bytes.len() == 7 && bytes[0] == b'#',
        "hex color must use the #RRGGBB format"
    );
    Color::Rgb(
        hex_pair(bytes[1], bytes[2]),
        hex_pair(bytes[3], bytes[4]),
        hex_pair(bytes[5], bytes[6]),
    )
}

const fn hex_pair(high: u8, low: u8) -> u8 {
    (hex_digit(high) << 4) | hex_digit(low)
}

const fn hex_digit(value: u8) -> u8 {
    match value {
        b'0'..=b'9' => value - b'0',
        b'a'..=b'f' => value - b'a' + 10,
        b'A'..=b'F' => value - b'A' + 10,
        _ => panic!("hex color contains an invalid digit"),
    }
}

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
    active_selection_background: Color,
    active_selection_foreground: Color,
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
            active_selection_background: projected(tokens::TUI_ACTIVE_SELECTION_BACKGROUND)?,
            active_selection_foreground: projected(tokens::TUI_ACTIVE_SELECTION_FOREGROUND)?,
            r#type: projected(tokens::EDITOR_TOKEN_TYPE)?,
            variable: projected(tokens::EDITOR_TOKEN_VARIABLE)?,
            warning: projected(tokens::WARNING_FOREGROUND)?,
        })
    }

    pub(crate) const fn fallback() -> Self {
        Self {
            accent: hex("#69aaff"),
            background: hex("#0d1117"),
            border: hex("#808080"),
            chat_input_chrome: hex("#9b9b9b"),
            danger: hex("#f56969"),
            foreground: hex("#ffffff"),
            function: hex("#d2a8ff"),
            highlight: hex("#9a91eb"),
            inserted_background: hex("#13301c"),
            inserted_marker: hex("#3fb950"),
            keyword: hex("#ff7b72"),
            muted: hex("#808080"),
            removed_background: hex("#37191b"),
            removed_marker: hex("#f85149"),
            string: hex("#a5d6ff"),
            success: hex("#5fd28c"),
            active_selection_background: hex("#c0c0c0"),
            active_selection_foreground: hex("#000000"),
            r#type: hex("#d2a8ff"),
            variable: hex("#ffa657"),
            warning: hex("#f5be50"),
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
    pub(crate) const fn active_selection_background(self) -> Color {
        self.active_selection_background
    }
    pub(crate) const fn active_selection_foreground(self) -> Color {
        self.active_selection_foreground
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
    theme_revision: u64,
}

impl<'a> RenderContext<'a> {
    pub(crate) const fn new(theme: &'a RenderTheme, theme_revision: u64) -> Self {
        Self {
            theme,
            theme_revision,
        }
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
    pub(crate) const fn function(self) -> Color {
        self.theme.function()
    }
    pub(crate) const fn highlight(self) -> Color {
        self.theme.highlight()
    }
    pub(crate) const fn muted(self) -> Color {
        self.theme.muted()
    }
    pub(crate) const fn keyword(self) -> Color {
        self.theme.keyword()
    }
    pub(crate) const fn string(self) -> Color {
        self.theme.string()
    }
    pub(crate) const fn success(self) -> Color {
        self.theme.success()
    }
    pub(crate) const fn active_selection_background(self) -> Color {
        self.theme.active_selection_background()
    }
    pub(crate) const fn active_selection_foreground(self) -> Color {
        self.theme.active_selection_foreground()
    }
    pub(crate) const fn r#type(self) -> Color {
        self.theme.r#type()
    }
    pub(crate) const fn variable(self) -> Color {
        self.theme.variable()
    }
    pub(crate) const fn warning(self) -> Color {
        self.theme.warning()
    }

    pub(crate) const fn theme_revision(self) -> u64 {
        self.theme_revision
    }
}

#[cfg(test)]
pub(crate) fn test_context() -> RenderContext<'static> {
    static THEME: RenderTheme = RenderTheme::fallback();
    RenderContext::new(&THEME, 0)
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
