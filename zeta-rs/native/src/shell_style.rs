use zeta_editor::{
    CodeEditorPalette, CodeEditorStyle, CodeEditorSyntaxPalette, CodeEditorTokenRole,
    DiffEditorPalette, DiffEditorStyle, MultiDiffEditorPalette, MultiDiffEditorStyle,
};
use zeta_theme::{ThemeError, ThemeSnapshot, tokens};
use zeta_ui::{
    Color, CornerRadii, Edges, FontFamily, FontWeight, InputBoxStateColors, InputBoxStyle,
    ScrollViewStyle, ScrollbarStyle, SearchBoxStyle, TextStyle,
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
    pub(crate) error: Color,
    pub(crate) terminal_selection: Color,
    pub(crate) surface_hovered: Color,
    pub(crate) session_tab_highlight: Color,
    scrollbar: Color,
    scrollbar_hovered: Color,
    scrollbar_active: Color,
    diff_removed_line: Color,
    diff_inserted_line: Color,
    diff_removed_text: Color,
    diff_inserted_text: Color,
    diff_missing_line: Color,
    diff_unchanged_region: Color,
    diff_unchanged_region_foreground: Color,
    diff_removed_marker: Color,
    diff_inserted_marker: Color,
    terminal_colors: [Color; 16],
}

pub(crate) const SHELL_PALETTE: ShellPalette = ShellPalette {
    background: Color::rgb(252, 252, 253),
    surface: Color::WHITE,
    surface_raised: Color::rgb(246, 246, 247),
    border: Color::rgb(222, 222, 224),
    text: Color::rgb(38, 38, 41),
    text_muted: Color::rgb(126, 126, 132),
    accent: Color::rgb(15, 110, 96),
    error: Color::rgb(180, 38, 38),
    terminal_selection: Color::rgba(68, 139, 202, 72),
    surface_hovered: Color::rgb(248, 248, 249),
    session_tab_highlight: Color::rgb(235, 235, 237),
    scrollbar: Color::rgba(100, 100, 100, 51),
    scrollbar_hovered: Color::rgba(100, 100, 100, 89),
    scrollbar_active: Color::rgba(0, 0, 0, 51),
    diff_removed_line: Color::rgb(255, 235, 233),
    diff_inserted_line: Color::rgb(218, 251, 225),
    diff_removed_text: Color::rgb(255, 198, 194),
    diff_inserted_text: Color::rgb(166, 235, 183),
    diff_missing_line: Color::rgb(248, 248, 249),
    diff_unchanged_region: Color::rgb(241, 246, 252),
    diff_unchanged_region_foreground: Color::rgb(126, 126, 132),
    diff_removed_marker: Color::rgb(161, 38, 13),
    diff_inserted_marker: Color::rgb(16, 124, 16),
    terminal_colors: [
        Color::rgb(36, 41, 47),
        Color::rgb(207, 34, 46),
        Color::rgb(17, 99, 41),
        Color::rgb(154, 103, 0),
        Color::rgb(9, 105, 218),
        Color::rgb(130, 80, 223),
        Color::rgb(27, 124, 131),
        Color::rgb(38, 38, 41),
        Color::rgb(110, 119, 129),
        Color::rgb(164, 14, 38),
        Color::rgb(26, 127, 55),
        Color::rgb(191, 135, 0),
        Color::rgb(33, 139, 255),
        Color::rgb(164, 117, 249),
        Color::rgb(49, 146, 170),
        Color::rgb(140, 149, 159),
    ],
};

impl ShellPalette {
    pub(crate) fn from_theme(theme: &ThemeSnapshot) -> Result<Self, ThemeError> {
        let mut terminal_colors = [Color::TRANSPARENT; 16];
        for (index, token) in tokens::TERMINAL_ANSI.iter().enumerate() {
            terminal_colors[index] = theme_color(theme, token)?;
        }
        Ok(Self {
            background: theme_color(theme, tokens::WORKBENCH_BACKGROUND)?,
            surface: theme_color(theme, tokens::EDITOR_BACKGROUND)?,
            surface_raised: theme_color(theme, tokens::SIDE_BAR_BACKGROUND)?,
            border: theme_color(theme, tokens::BORDER)?,
            text: theme_color(theme, tokens::FOREGROUND)?,
            text_muted: theme_color(theme, tokens::MUTED_FOREGROUND)?,
            accent: theme_color(theme, tokens::ACCENT_FOREGROUND)?,
            error: theme_color(theme, tokens::ERROR_FOREGROUND)?,
            terminal_selection: theme_color(theme, tokens::SELECTION_BACKGROUND)?,
            surface_hovered: theme_color(theme, tokens::LIST_HOVER_BACKGROUND)?,
            session_tab_highlight: theme_color(theme, tokens::LIST_ACTIVE_SELECTION_BACKGROUND)?,
            scrollbar: theme_color(theme, tokens::SCROLLBAR_SLIDER_BACKGROUND)?,
            scrollbar_hovered: theme_color(theme, tokens::SCROLLBAR_SLIDER_HOVER_BACKGROUND)?,
            scrollbar_active: theme_color(theme, tokens::SCROLLBAR_SLIDER_ACTIVE_BACKGROUND)?,
            diff_removed_line: theme_color(theme, tokens::DIFF_REMOVED_LINE_BACKGROUND)?,
            diff_inserted_line: theme_color(theme, tokens::DIFF_INSERTED_LINE_BACKGROUND)?,
            diff_removed_text: theme_color(theme, tokens::DIFF_REMOVED_TEXT_BACKGROUND)?,
            diff_inserted_text: theme_color(theme, tokens::DIFF_INSERTED_TEXT_BACKGROUND)?,
            diff_missing_line: theme_color(theme, tokens::DIFF_MISSING_LINE_BACKGROUND)?,
            diff_unchanged_region: theme_color(theme, tokens::DIFF_UNCHANGED_REGION_BACKGROUND)?,
            diff_unchanged_region_foreground: theme_color(
                theme,
                tokens::DIFF_UNCHANGED_REGION_FOREGROUND,
            )?,
            diff_removed_marker: theme_color(theme, tokens::DIFF_REMOVED_LINE_MARKER)?,
            diff_inserted_marker: theme_color(theme, tokens::DIFF_INSERTED_LINE_MARKER)?,
            terminal_colors,
        })
    }

    pub(crate) fn terminal_indexed_color(self, index: u8) -> Color {
        self.terminal_colors
            .get(usize::from(index))
            .copied()
            .unwrap_or(self.text)
    }

    pub(crate) fn session_search_style(self) -> SearchBoxStyle {
        let input_box = InputBoxStyle::new(
            InputBoxStateColors::new(Color::TRANSPARENT, Color::TRANSPARENT, Color::TRANSPARENT),
            InputBoxStateColors::new(Color::TRANSPARENT, Color::TRANSPARENT, Color::TRANSPARENT),
            TextStyle::new(12.0, self.text).with_line_height(16.0),
            TextStyle::new(12.0, self.text_muted).with_line_height(16.0),
        )
        .with_border_width(0.0)
        .with_corner_radii(CornerRadii::uniform(4.0))
        .with_padding(Edges::new(4.0, 8.0, 4.0, 8.0))
        .with_selection_color(self.terminal_selection)
        .with_caret_color(self.accent)
        .with_preedit_underline_color(self.accent);
        SearchBoxStyle::new(input_box, self.text_muted).with_icon_size(18.0)
    }

    pub(crate) fn terminal_scroll_view_style(self) -> ScrollViewStyle {
        self.overlay_scroll_view_style()
    }

    pub(crate) fn file_list_scroll_view_style(self) -> ScrollViewStyle {
        self.overlay_scroll_view_style()
    }

    pub(crate) fn picker_scroll_view_style(self) -> ScrollViewStyle {
        self.overlay_scroll_view_style()
    }

    fn overlay_scroll_view_style(self) -> ScrollViewStyle {
        ScrollViewStyle::new(
            ScrollbarStyle::new(Color::TRANSPARENT, self.scrollbar)
                .with_hovered_colors(Color::TRANSPARENT, self.scrollbar_hovered)
                .with_active_colors(Color::TRANSPARENT, self.scrollbar_active),
        )
    }

    pub(crate) fn multi_diff_editor_style(self) -> MultiDiffEditorStyle {
        let code_editor = CodeEditorStyle::new(CodeEditorPalette {
            surface: self.surface,
            header: self.surface_raised,
            gutter: self.surface_raised,
            divider: self.border,
            text: self.text,
            text_muted: self.text_muted,
            selection: self.terminal_selection,
            caret: self.accent,
            composition_underline: self.accent,
            syntax: CodeEditorSyntaxPalette::uniform(self.text),
        });
        let diff_editor = DiffEditorStyle::new(DiffEditorPalette {
            code_editor,
            divider: self.border,
            removed_marker: self.diff_removed_marker,
            added_marker: self.diff_inserted_marker,
            removed_line: self.diff_removed_line,
            added_line: self.diff_inserted_line,
            removed_inline: self.diff_removed_text,
            added_inline: self.diff_inserted_text,
            missing_line: self.diff_missing_line,
            fold_line: self.diff_unchanged_region,
            fold_marker: self.diff_unchanged_region_foreground,
        });
        MultiDiffEditorStyle::new(MultiDiffEditorPalette {
            surface: self.surface,
            file_header: self.surface_raised,
            divider: self.border,
            file_name: TextStyle::new(12.0, self.text)
                .with_family(FontFamily::Monospace)
                .with_weight(FontWeight::Bold)
                .with_line_height(18.0),
            diff_editor,
            scroll_view: self.overlay_scroll_view_style(),
        })
        .cards()
    }
}

pub(crate) fn code_editor_style(theme: &ThemeSnapshot) -> Result<CodeEditorStyle, ThemeError> {
    let syntax = CodeEditorSyntaxPalette::uniform(theme_color(theme, tokens::EDITOR_FOREGROUND)?)
        .with_color(
            CodeEditorTokenRole::Attribute,
            theme_color(theme, tokens::EDITOR_TOKEN_ATTRIBUTE)?,
        )
        .with_color(
            CodeEditorTokenRole::Comment,
            theme_color(theme, tokens::EDITOR_TOKEN_COMMENT)?,
        )
        .with_color(
            CodeEditorTokenRole::Constant,
            theme_color(theme, tokens::EDITOR_TOKEN_CONSTANT)?,
        )
        .with_color(
            CodeEditorTokenRole::Constructor,
            theme_color(theme, tokens::EDITOR_TOKEN_CONSTRUCTOR)?,
        )
        .with_color(
            CodeEditorTokenRole::Embedded,
            theme_color(theme, tokens::EDITOR_TOKEN_EMBEDDED)?,
        )
        .with_color(
            CodeEditorTokenRole::Function,
            theme_color(theme, tokens::EDITOR_TOKEN_FUNCTION)?,
        )
        .with_color(
            CodeEditorTokenRole::Keyword,
            theme_color(theme, tokens::EDITOR_TOKEN_KEYWORD)?,
        )
        .with_color(
            CodeEditorTokenRole::Label,
            theme_color(theme, tokens::EDITOR_TOKEN_LABEL)?,
        )
        .with_color(
            CodeEditorTokenRole::Module,
            theme_color(theme, tokens::EDITOR_TOKEN_MODULE)?,
        )
        .with_color(
            CodeEditorTokenRole::Number,
            theme_color(theme, tokens::EDITOR_TOKEN_NUMBER)?,
        )
        .with_color(
            CodeEditorTokenRole::Operator,
            theme_color(theme, tokens::EDITOR_TOKEN_OPERATOR)?,
        )
        .with_color(
            CodeEditorTokenRole::Property,
            theme_color(theme, tokens::EDITOR_TOKEN_PROPERTY)?,
        )
        .with_color(
            CodeEditorTokenRole::Punctuation,
            theme_color(theme, tokens::EDITOR_TOKEN_PUNCTUATION)?,
        )
        .with_color(
            CodeEditorTokenRole::Regexp,
            theme_color(theme, tokens::EDITOR_TOKEN_REGEXP)?,
        )
        .with_color(
            CodeEditorTokenRole::String,
            theme_color(theme, tokens::EDITOR_TOKEN_STRING)?,
        )
        .with_color(
            CodeEditorTokenRole::Type,
            theme_color(theme, tokens::EDITOR_TOKEN_TYPE)?,
        )
        .with_color(
            CodeEditorTokenRole::Variable,
            theme_color(theme, tokens::EDITOR_TOKEN_VARIABLE)?,
        );
    Ok(CodeEditorStyle::new(CodeEditorPalette {
        surface: theme_color(theme, tokens::EDITOR_BACKGROUND)?,
        header: theme_color(theme, tokens::SIDE_BAR_BACKGROUND)?,
        gutter: theme_color(theme, tokens::SIDE_BAR_BACKGROUND)?,
        divider: theme_color(theme, tokens::BORDER)?,
        text: theme_color(theme, tokens::EDITOR_FOREGROUND)?,
        text_muted: theme_color(theme, tokens::MUTED_FOREGROUND)?,
        selection: theme_color(theme, tokens::SELECTION_BACKGROUND)?,
        caret: theme_color(theme, tokens::ACCENT_FOREGROUND)?,
        composition_underline: theme_color(theme, tokens::ACCENT_FOREGROUND)?,
        syntax,
    }))
}

fn theme_color(theme: &ThemeSnapshot, token: &str) -> Result<Color, ThemeError> {
    let [red, green, blue, alpha] = theme.required_color(token)?.components();
    Ok(Color::rgba(red, green, blue, alpha))
}
