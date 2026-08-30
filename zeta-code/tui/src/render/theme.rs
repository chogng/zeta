use ratatui::style::Color;
use zeta_terminal_detection::ColorLevel;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ThemeRgb {
    red: u8,
    green: u8,
    blue: u8,
}

impl ThemeRgb {
    pub(crate) const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }

    pub(crate) fn parse(value: &str) -> Result<Self, String> {
        let hex = value
            .strip_prefix('#')
            .filter(|hex| hex.len() == 6 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()))
            .ok_or_else(|| format!("invalid TUI theme color '{value}'; expected #RRGGBB"))?;
        let component = |start| {
            u8::from_str_radix(&hex[start..start + 2], 16)
                .map_err(|_| format!("invalid TUI theme color '{value}'; expected #RRGGBB"))
        };
        Ok(Self::new(component(0)?, component(2)?, component(4)?))
    }

    const fn components(self) -> [u8; 3] {
        [self.red, self.green, self.blue]
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ThemePalette {
    pub(crate) accent: ThemeRgb,
    pub(crate) background: ThemeRgb,
    pub(crate) border: ThemeRgb,
    pub(crate) chat_input_chrome: ThemeRgb,
    pub(crate) danger: ThemeRgb,
    pub(crate) foreground: ThemeRgb,
    pub(crate) function: ThemeRgb,
    pub(crate) highlight: ThemeRgb,
    pub(crate) inserted_background: ThemeRgb,
    pub(crate) inserted_marker: ThemeRgb,
    pub(crate) keyword: ThemeRgb,
    pub(crate) muted: ThemeRgb,
    pub(crate) quick_view_background: ThemeRgb,
    pub(crate) removed_background: ThemeRgb,
    pub(crate) removed_marker: ThemeRgb,
    pub(crate) string: ThemeRgb,
    pub(crate) success: ThemeRgb,
    pub(crate) active_selection_background: ThemeRgb,
    pub(crate) active_selection_foreground: ThemeRgb,
    pub(crate) screen_selection_background: ThemeRgb,
    pub(crate) screen_selection_foreground: ThemeRgb,
    pub(crate) r#type: ThemeRgb,
    pub(crate) variable: ThemeRgb,
    pub(crate) warning: ThemeRgb,
}

impl ThemePalette {
    pub(crate) const fn dark() -> Self {
        Self {
            accent: ThemeRgb::new(0x58, 0xa6, 0xff),
            background: ThemeRgb::new(0x0d, 0x11, 0x17),
            border: ThemeRgb::new(0x2b, 0x2b, 0x2b),
            chat_input_chrome: ThemeRgb::new(0x8b, 0x94, 0x9e),
            danger: ThemeRgb::new(0xf8, 0x51, 0x49),
            foreground: ThemeRgb::new(0xe6, 0xed, 0xf3),
            function: ThemeRgb::new(0xd2, 0xa8, 0xff),
            highlight: ThemeRgb::new(0x9a, 0x91, 0xeb),
            inserted_background: ThemeRgb::new(0x13, 0x2d, 0x1d),
            inserted_marker: ThemeRgb::new(0x3f, 0xb9, 0x50),
            keyword: ThemeRgb::new(0xff, 0x7b, 0x72),
            muted: ThemeRgb::new(0x8b, 0x94, 0x9e),
            quick_view_background: ThemeRgb::new(0x25, 0x25, 0x26),
            removed_background: ThemeRgb::new(0x35, 0x1b, 0x1b),
            removed_marker: ThemeRgb::new(0xf8, 0x51, 0x49),
            string: ThemeRgb::new(0xa5, 0xd6, 0xff),
            success: ThemeRgb::new(0x3f, 0xb9, 0x50),
            active_selection_background: ThemeRgb::new(0xc0, 0xc0, 0xc0),
            active_selection_foreground: ThemeRgb::new(0x00, 0x00, 0x00),
            screen_selection_background: ThemeRgb::new(0x87, 0xce, 0xeb),
            screen_selection_foreground: ThemeRgb::new(0x0d, 0x11, 0x17),
            r#type: ThemeRgb::new(0xd2, 0xa8, 0xff),
            variable: ThemeRgb::new(0xff, 0xa6, 0x57),
            warning: ThemeRgb::new(0xff, 0xa6, 0x57),
        }
    }

    pub(crate) const fn light() -> Self {
        Self {
            accent: ThemeRgb::new(0x09, 0x69, 0xda),
            background: ThemeRgb::new(0xff, 0xff, 0xff),
            border: ThemeRgb::new(0xe5, 0xe5, 0xe5),
            chat_input_chrome: ThemeRgb::new(0x57, 0x60, 0x6a),
            danger: ThemeRgb::new(0xcf, 0x22, 0x2e),
            foreground: ThemeRgb::new(0x1f, 0x23, 0x28),
            function: ThemeRgb::new(0x82, 0x50, 0xdf),
            highlight: ThemeRgb::new(0x66, 0x58, 0xc7),
            inserted_background: ThemeRgb::new(0xda, 0xfb, 0xe1),
            inserted_marker: ThemeRgb::new(0x1a, 0x7f, 0x37),
            keyword: ThemeRgb::new(0xcf, 0x22, 0x2e),
            muted: ThemeRgb::new(0x57, 0x60, 0x6a),
            quick_view_background: ThemeRgb::new(0xf8, 0xf8, 0xf8),
            removed_background: ThemeRgb::new(0xff, 0xeb, 0xe9),
            removed_marker: ThemeRgb::new(0xcf, 0x22, 0x2e),
            string: ThemeRgb::new(0x0a, 0x30, 0x69),
            success: ThemeRgb::new(0x1a, 0x7f, 0x37),
            active_selection_background: ThemeRgb::new(0xc0, 0xc0, 0xc0),
            active_selection_foreground: ThemeRgb::new(0x00, 0x00, 0x00),
            screen_selection_background: ThemeRgb::new(0x87, 0xce, 0xeb),
            screen_selection_foreground: ThemeRgb::new(0x0d, 0x11, 0x17),
            r#type: ThemeRgb::new(0x82, 0x50, 0xdf),
            variable: ThemeRgb::new(0x95, 0x38, 0x00),
            warning: ThemeRgb::new(0x95, 0x38, 0x00),
        }
    }

    pub(crate) const fn colorblind_dark() -> Self {
        Self {
            danger: ThemeRgb::new(0xd4, 0x76, 0x16),
            highlight: ThemeRgb::new(0x58, 0xa6, 0xff),
            inserted_background: ThemeRgb::new(0x12, 0x29, 0x4b),
            inserted_marker: ThemeRgb::new(0x58, 0xa6, 0xff),
            keyword: ThemeRgb::new(0xec, 0x8e, 0x2c),
            removed_background: ThemeRgb::new(0x40, 0x28, 0x10),
            removed_marker: ThemeRgb::new(0xd4, 0x76, 0x16),
            success: ThemeRgb::new(0x58, 0xa6, 0xff),
            variable: ThemeRgb::new(0xfd, 0xac, 0x54),
            warning: ThemeRgb::new(0xfd, 0xac, 0x54),
            ..Self::dark()
        }
    }

    pub(crate) const fn colorblind_light() -> Self {
        Self {
            danger: ThemeRgb::new(0xb3, 0x59, 0x00),
            highlight: ThemeRgb::new(0x09, 0x69, 0xda),
            inserted_background: ThemeRgb::new(0xdd, 0xf4, 0xff),
            inserted_marker: ThemeRgb::new(0x09, 0x69, 0xda),
            keyword: ThemeRgb::new(0xb3, 0x59, 0x00),
            removed_background: ThemeRgb::new(0xff, 0xf1, 0xe5),
            removed_marker: ThemeRgb::new(0xb3, 0x59, 0x00),
            success: ThemeRgb::new(0x09, 0x69, 0xda),
            variable: ThemeRgb::new(0x8a, 0x46, 0x00),
            warning: ThemeRgb::new(0x8a, 0x46, 0x00),
            ..Self::light()
        }
    }
}

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
    quick_view_background: Color,
    string: Color,
    success: Color,
    active_selection_background: Color,
    active_selection_foreground: Color,
    screen_selection_background: Color,
    screen_selection_foreground: Color,
    r#type: Color,
    variable: Color,
    warning: Color,
}

impl RenderTheme {
    pub(crate) fn from_palette(palette: ThemePalette, capability: ColorLevel) -> Self {
        let projected = |color| terminal_color(color, capability);
        Self {
            accent: projected(palette.accent),
            background: projected(palette.background),
            border: projected(palette.border),
            chat_input_chrome: projected(palette.chat_input_chrome),
            danger: projected(palette.danger),
            foreground: projected(palette.foreground),
            function: projected(palette.function),
            highlight: projected(palette.highlight),
            inserted_background: projected(palette.inserted_background),
            inserted_marker: projected(palette.inserted_marker),
            keyword: projected(palette.keyword),
            muted: projected(palette.muted),
            removed_background: projected(palette.removed_background),
            removed_marker: projected(palette.removed_marker),
            quick_view_background: projected(palette.quick_view_background),
            string: projected(palette.string),
            success: projected(palette.success),
            active_selection_background: projected(palette.active_selection_background),
            active_selection_foreground: projected(palette.active_selection_foreground),
            screen_selection_background: projected(palette.screen_selection_background),
            screen_selection_foreground: projected(palette.screen_selection_foreground),
            r#type: projected(palette.r#type),
            variable: projected(palette.variable),
            warning: projected(palette.warning),
        }
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
            quick_view_background: hex("#252526"),
            string: hex("#a5d6ff"),
            success: hex("#5fd28c"),
            active_selection_background: hex("#c0c0c0"),
            active_selection_foreground: hex("#000000"),
            screen_selection_background: hex("#87ceeb"),
            screen_selection_foreground: hex("#0d1117"),
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
    pub(crate) const fn quick_view_background(self) -> Color {
        self.quick_view_background
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
    pub(crate) const fn screen_selection_background(self) -> Color {
        self.screen_selection_background
    }
    pub(crate) const fn screen_selection_foreground(self) -> Color {
        self.screen_selection_foreground
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
    pub(crate) const fn quick_view_background(self) -> Color {
        self.theme.quick_view_background()
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
    pub(crate) const fn screen_selection_background(self) -> Color {
        self.theme.screen_selection_background()
    }
    pub(crate) const fn screen_selection_foreground(self) -> Color {
        self.theme.screen_selection_foreground()
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

fn terminal_color(color: ThemeRgb, capability: ColorLevel) -> Color {
    let rgb = color.components();
    match capability {
        ColorLevel::TrueColor => Color::Rgb(rgb[0], rgb[1], rgb[2]),
        ColorLevel::Ansi256 => Color::Indexed(nearest_ansi256(rgb)),
        ColorLevel::Ansi16 => nearest_ansi16(rgb),
        ColorLevel::Monochrome => Color::Reset,
    }
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
