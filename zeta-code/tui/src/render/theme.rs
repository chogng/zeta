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
    pub(crate) accent_surface_background: ThemeRgb,
    pub(crate) accent_surface_foreground: ThemeRgb,
    pub(crate) action_foreground: ThemeRgb,
    pub(crate) background: ThemeRgb,
    pub(crate) border: ThemeRgb,
    pub(crate) chat_input_chrome: ThemeRgb,
    pub(crate) danger: ThemeRgb,
    pub(crate) disabled_foreground: ThemeRgb,
    pub(crate) focus: ThemeRgb,
    pub(crate) foreground: ThemeRgb,
    pub(crate) function: ThemeRgb,
    pub(crate) hover_background: ThemeRgb,
    pub(crate) hover_foreground: ThemeRgb,
    pub(crate) inserted_background: ThemeRgb,
    pub(crate) inserted_marker: ThemeRgb,
    pub(crate) keyword: ThemeRgb,
    pub(crate) muted: ThemeRgb,
    pub(crate) overlay_background: ThemeRgb,
    pub(crate) pressed_background: ThemeRgb,
    pub(crate) pressed_foreground: ThemeRgb,
    pub(crate) removed_background: ThemeRgb,
    pub(crate) removed_marker: ThemeRgb,
    pub(crate) string: ThemeRgb,
    pub(crate) success: ThemeRgb,
    pub(crate) selection_background: ThemeRgb,
    pub(crate) selection_foreground: ThemeRgb,
    pub(crate) screen_selection_background: ThemeRgb,
    pub(crate) screen_selection_foreground: ThemeRgb,
    pub(crate) r#type: ThemeRgb,
    pub(crate) user_message_background: ThemeRgb,
    pub(crate) variable: ThemeRgb,
    pub(crate) warning: ThemeRgb,
}

impl ThemePalette {
    pub(crate) const fn dark() -> Self {
        Self {
            accent: ThemeRgb::new(0x58, 0xa6, 0xff),
            accent_surface_background: ThemeRgb::new(0x66, 0x58, 0xc7),
            accent_surface_foreground: ThemeRgb::new(0xff, 0xff, 0xff),
            action_foreground: ThemeRgb::new(0x58, 0xa6, 0xff),
            background: ThemeRgb::new(0x0d, 0x11, 0x17),
            border: ThemeRgb::new(0x2b, 0x2b, 0x2b),
            chat_input_chrome: ThemeRgb::new(0x8b, 0x94, 0x9e),
            danger: ThemeRgb::new(0xf8, 0x51, 0x49),
            disabled_foreground: ThemeRgb::new(0x8b, 0x94, 0x9e),
            focus: ThemeRgb::new(0x9a, 0x91, 0xeb),
            foreground: ThemeRgb::new(0xe6, 0xed, 0xf3),
            function: ThemeRgb::new(0xd2, 0xa8, 0xff),
            hover_background: ThemeRgb::new(0x25, 0x23, 0x3a),
            hover_foreground: ThemeRgb::new(0xf0, 0xed, 0xff),
            inserted_background: ThemeRgb::new(0x13, 0x2d, 0x1d),
            inserted_marker: ThemeRgb::new(0x3f, 0xb9, 0x50),
            keyword: ThemeRgb::new(0xff, 0x7b, 0x72),
            muted: ThemeRgb::new(0x8b, 0x94, 0x9e),
            overlay_background: ThemeRgb::new(0x25, 0x25, 0x26),
            pressed_background: ThemeRgb::new(0x3b, 0x35, 0x68),
            pressed_foreground: ThemeRgb::new(0xff, 0xff, 0xff),
            removed_background: ThemeRgb::new(0x35, 0x1b, 0x1b),
            removed_marker: ThemeRgb::new(0xf8, 0x51, 0x49),
            string: ThemeRgb::new(0xa5, 0xd6, 0xff),
            success: ThemeRgb::new(0x3f, 0xb9, 0x50),
            selection_background: ThemeRgb::new(0x2f, 0x2b, 0x52),
            selection_foreground: ThemeRgb::new(0xf0, 0xed, 0xff),
            screen_selection_background: ThemeRgb::new(0x87, 0xce, 0xeb),
            screen_selection_foreground: ThemeRgb::new(0x0d, 0x11, 0x17),
            r#type: ThemeRgb::new(0xd2, 0xa8, 0xff),
            user_message_background: ThemeRgb::new(0x16, 0x1b, 0x22),
            variable: ThemeRgb::new(0xff, 0xa6, 0x57),
            warning: ThemeRgb::new(0xff, 0xa6, 0x57),
        }
    }

    pub(crate) const fn light() -> Self {
        Self {
            accent: ThemeRgb::new(0x09, 0x69, 0xda),
            accent_surface_background: ThemeRgb::new(0x66, 0x58, 0xc7),
            accent_surface_foreground: ThemeRgb::new(0xff, 0xff, 0xff),
            action_foreground: ThemeRgb::new(0x09, 0x69, 0xda),
            background: ThemeRgb::new(0xff, 0xff, 0xff),
            border: ThemeRgb::new(0xe5, 0xe5, 0xe5),
            chat_input_chrome: ThemeRgb::new(0x57, 0x60, 0x6a),
            danger: ThemeRgb::new(0xcf, 0x22, 0x2e),
            disabled_foreground: ThemeRgb::new(0x57, 0x60, 0x6a),
            focus: ThemeRgb::new(0x66, 0x58, 0xc7),
            foreground: ThemeRgb::new(0x1f, 0x23, 0x28),
            function: ThemeRgb::new(0x82, 0x50, 0xdf),
            hover_background: ThemeRgb::new(0xf2, 0xf0, 0xff),
            hover_foreground: ThemeRgb::new(0x34, 0x2b, 0x72),
            inserted_background: ThemeRgb::new(0xda, 0xfb, 0xe1),
            inserted_marker: ThemeRgb::new(0x1a, 0x7f, 0x37),
            keyword: ThemeRgb::new(0xcf, 0x22, 0x2e),
            muted: ThemeRgb::new(0x57, 0x60, 0x6a),
            overlay_background: ThemeRgb::new(0xf8, 0xf8, 0xf8),
            pressed_background: ThemeRgb::new(0xd8, 0xd1, 0xff),
            pressed_foreground: ThemeRgb::new(0x27, 0x1f, 0x63),
            removed_background: ThemeRgb::new(0xff, 0xeb, 0xe9),
            removed_marker: ThemeRgb::new(0xcf, 0x22, 0x2e),
            string: ThemeRgb::new(0x0a, 0x30, 0x69),
            success: ThemeRgb::new(0x1a, 0x7f, 0x37),
            selection_background: ThemeRgb::new(0xe9, 0xe5, 0xff),
            selection_foreground: ThemeRgb::new(0x34, 0x2b, 0x72),
            screen_selection_background: ThemeRgb::new(0x87, 0xce, 0xeb),
            screen_selection_foreground: ThemeRgb::new(0x0d, 0x11, 0x17),
            r#type: ThemeRgb::new(0x82, 0x50, 0xdf),
            user_message_background: ThemeRgb::new(0xf6, 0xf8, 0xfa),
            variable: ThemeRgb::new(0x95, 0x38, 0x00),
            warning: ThemeRgb::new(0x95, 0x38, 0x00),
        }
    }

    pub(crate) const fn colorblind_dark() -> Self {
        Self {
            accent_surface_background: ThemeRgb::new(0x09, 0x69, 0xda),
            action_foreground: ThemeRgb::new(0x58, 0xa6, 0xff),
            danger: ThemeRgb::new(0xd4, 0x76, 0x16),
            focus: ThemeRgb::new(0x58, 0xa6, 0xff),
            hover_background: ThemeRgb::new(0x17, 0x2a, 0x46),
            hover_foreground: ThemeRgb::new(0xdd, 0xf4, 0xff),
            inserted_background: ThemeRgb::new(0x12, 0x29, 0x4b),
            inserted_marker: ThemeRgb::new(0x58, 0xa6, 0xff),
            keyword: ThemeRgb::new(0xec, 0x8e, 0x2c),
            pressed_background: ThemeRgb::new(0x1f, 0x4f, 0x85),
            removed_background: ThemeRgb::new(0x40, 0x28, 0x10),
            removed_marker: ThemeRgb::new(0xd4, 0x76, 0x16),
            success: ThemeRgb::new(0x58, 0xa6, 0xff),
            selection_background: ThemeRgb::new(0x12, 0x29, 0x4b),
            selection_foreground: ThemeRgb::new(0xdd, 0xf4, 0xff),
            screen_selection_background: ThemeRgb::new(0x80, 0xcc, 0xff),
            variable: ThemeRgb::new(0xfd, 0xac, 0x54),
            warning: ThemeRgb::new(0xfd, 0xac, 0x54),
            ..Self::dark()
        }
    }

    pub(crate) const fn colorblind_light() -> Self {
        Self {
            accent_surface_background: ThemeRgb::new(0x09, 0x69, 0xda),
            action_foreground: ThemeRgb::new(0x09, 0x69, 0xda),
            danger: ThemeRgb::new(0xb3, 0x59, 0x00),
            focus: ThemeRgb::new(0x09, 0x69, 0xda),
            hover_background: ThemeRgb::new(0xee, 0xf8, 0xff),
            hover_foreground: ThemeRgb::new(0x03, 0x4b, 0x7a),
            inserted_background: ThemeRgb::new(0xdd, 0xf4, 0xff),
            inserted_marker: ThemeRgb::new(0x09, 0x69, 0xda),
            keyword: ThemeRgb::new(0xb3, 0x59, 0x00),
            pressed_background: ThemeRgb::new(0xb6, 0xe3, 0xff),
            pressed_foreground: ThemeRgb::new(0x03, 0x3d, 0x66),
            removed_background: ThemeRgb::new(0xff, 0xf1, 0xe5),
            removed_marker: ThemeRgb::new(0xb3, 0x59, 0x00),
            success: ThemeRgb::new(0x09, 0x69, 0xda),
            selection_background: ThemeRgb::new(0xdd, 0xf4, 0xff),
            selection_foreground: ThemeRgb::new(0x03, 0x4b, 0x7a),
            screen_selection_background: ThemeRgb::new(0x80, 0xcc, 0xff),
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
    accent_surface_background: Color,
    accent_surface_foreground: Color,
    action_foreground: Color,
    background: Color,
    border: Color,
    chat_input_chrome: Color,
    danger: Color,
    disabled_foreground: Color,
    focus: Color,
    foreground: Color,
    function: Color,
    hover_background: Color,
    hover_foreground: Color,
    inserted_background: Color,
    inserted_marker: Color,
    keyword: Color,
    muted: Color,
    pressed_background: Color,
    pressed_foreground: Color,
    removed_background: Color,
    removed_marker: Color,
    overlay_background: Color,
    string: Color,
    success: Color,
    selection_background: Color,
    selection_foreground: Color,
    screen_selection_background: Color,
    screen_selection_foreground: Color,
    r#type: Color,
    user_message_background: Color,
    variable: Color,
    warning: Color,
}

impl RenderTheme {
    pub(crate) fn from_palette(palette: ThemePalette, capability: ColorLevel) -> Self {
        let projected = |color| terminal_color(color, capability);
        Self {
            accent: projected(palette.accent),
            accent_surface_background: projected(palette.accent_surface_background),
            accent_surface_foreground: projected(palette.accent_surface_foreground),
            action_foreground: projected(palette.action_foreground),
            background: projected(palette.background),
            border: projected(palette.border),
            chat_input_chrome: projected(palette.chat_input_chrome),
            danger: projected(palette.danger),
            disabled_foreground: projected(palette.disabled_foreground),
            focus: projected(palette.focus),
            foreground: projected(palette.foreground),
            function: projected(palette.function),
            hover_background: projected(palette.hover_background),
            hover_foreground: projected(palette.hover_foreground),
            inserted_background: projected(palette.inserted_background),
            inserted_marker: projected(palette.inserted_marker),
            keyword: projected(palette.keyword),
            muted: projected(palette.muted),
            pressed_background: projected(palette.pressed_background),
            pressed_foreground: projected(palette.pressed_foreground),
            removed_background: projected(palette.removed_background),
            removed_marker: projected(palette.removed_marker),
            overlay_background: projected(palette.overlay_background),
            string: projected(palette.string),
            success: projected(palette.success),
            selection_background: projected(palette.selection_background),
            selection_foreground: projected(palette.selection_foreground),
            screen_selection_background: projected(palette.screen_selection_background),
            screen_selection_foreground: projected(palette.screen_selection_foreground),
            r#type: projected(palette.r#type),
            user_message_background: projected(palette.user_message_background),
            variable: projected(palette.variable),
            warning: projected(palette.warning),
        }
    }

    pub(crate) const fn fallback() -> Self {
        Self {
            accent: hex("#69aaff"),
            accent_surface_background: hex("#6658c7"),
            accent_surface_foreground: hex("#ffffff"),
            action_foreground: hex("#69aaff"),
            background: hex("#0d1117"),
            border: hex("#808080"),
            chat_input_chrome: hex("#9b9b9b"),
            danger: hex("#f56969"),
            disabled_foreground: hex("#808080"),
            focus: hex("#9a91eb"),
            foreground: hex("#ffffff"),
            function: hex("#d2a8ff"),
            hover_background: hex("#25233a"),
            hover_foreground: hex("#f0edff"),
            inserted_background: hex("#13301c"),
            inserted_marker: hex("#3fb950"),
            keyword: hex("#ff7b72"),
            muted: hex("#808080"),
            pressed_background: hex("#3b3568"),
            pressed_foreground: hex("#ffffff"),
            removed_background: hex("#37191b"),
            removed_marker: hex("#f85149"),
            overlay_background: hex("#252526"),
            string: hex("#a5d6ff"),
            success: hex("#5fd28c"),
            selection_background: hex("#2f2b52"),
            selection_foreground: hex("#f0edff"),
            screen_selection_background: hex("#87ceeb"),
            screen_selection_foreground: hex("#0d1117"),
            r#type: hex("#d2a8ff"),
            user_message_background: hex("#161b22"),
            variable: hex("#ffa657"),
            warning: hex("#f5be50"),
        }
    }

    pub(crate) const fn accent(self) -> Color {
        self.accent
    }
    pub(crate) const fn accent_surface_background(self) -> Color {
        self.accent_surface_background
    }
    pub(crate) const fn accent_surface_foreground(self) -> Color {
        self.accent_surface_foreground
    }
    pub(crate) const fn action_foreground(self) -> Color {
        self.action_foreground
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
    pub(crate) const fn disabled_foreground(self) -> Color {
        self.disabled_foreground
    }
    pub(crate) const fn focus(self) -> Color {
        self.focus
    }
    pub(crate) const fn foreground(self) -> Color {
        self.foreground
    }
    pub(crate) const fn function(self) -> Color {
        self.function
    }
    pub(crate) const fn hover_background(self) -> Color {
        self.hover_background
    }
    pub(crate) const fn hover_foreground(self) -> Color {
        self.hover_foreground
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
    pub(crate) const fn pressed_background(self) -> Color {
        self.pressed_background
    }
    pub(crate) const fn pressed_foreground(self) -> Color {
        self.pressed_foreground
    }
    pub(crate) const fn removed_background(self) -> Color {
        self.removed_background
    }
    pub(crate) const fn removed_marker(self) -> Color {
        self.removed_marker
    }
    pub(crate) const fn overlay_background(self) -> Color {
        self.overlay_background
    }
    pub(crate) const fn string(self) -> Color {
        self.string
    }
    pub(crate) const fn success(self) -> Color {
        self.success
    }
    pub(crate) const fn selection_background(self) -> Color {
        self.selection_background
    }
    pub(crate) const fn selection_foreground(self) -> Color {
        self.selection_foreground
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
    pub(crate) const fn user_message_background(self) -> Color {
        self.user_message_background
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
    pub(crate) const fn accent_surface_background(self) -> Color {
        self.theme.accent_surface_background()
    }
    pub(crate) const fn accent_surface_foreground(self) -> Color {
        self.theme.accent_surface_foreground()
    }
    pub(crate) const fn action_foreground(self) -> Color {
        self.theme.action_foreground()
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
    pub(crate) const fn disabled_foreground(self) -> Color {
        self.theme.disabled_foreground()
    }
    pub(crate) const fn focus(self) -> Color {
        self.theme.focus()
    }
    pub(crate) const fn foreground(self) -> Color {
        self.theme.foreground()
    }
    pub(crate) const fn function(self) -> Color {
        self.theme.function()
    }
    pub(crate) const fn hover_background(self) -> Color {
        self.theme.hover_background()
    }
    pub(crate) const fn hover_foreground(self) -> Color {
        self.theme.hover_foreground()
    }
    pub(crate) const fn muted(self) -> Color {
        self.theme.muted()
    }
    pub(crate) const fn pressed_background(self) -> Color {
        self.theme.pressed_background()
    }
    pub(crate) const fn pressed_foreground(self) -> Color {
        self.theme.pressed_foreground()
    }
    pub(crate) const fn overlay_background(self) -> Color {
        self.theme.overlay_background()
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
    pub(crate) const fn selection_background(self) -> Color {
        self.theme.selection_background()
    }
    pub(crate) const fn selection_foreground(self) -> Color {
        self.theme.selection_foreground()
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
    pub(crate) const fn user_message_background(self) -> Color {
        self.theme.user_message_background()
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
