use zeta_ui_components::ButtonBackgrounds;
use zeta_ui_components::ButtonStyle;
use zeta_ui_theme::UiTheme;
use zui::ui::{CornerRadii, Edges, FontFamily, FontWeight, TextStyle};

use crate::{
    CodeEditorDiagnosticPalette, CodeEditorPalette, CodeEditorStyle, CodeEditorSyntaxPalette,
    CodeEditorTokenRole, DiffEditorPalette, DiffEditorStyle, MultiDiffEditorPalette,
    MultiDiffEditorStyle,
};

impl CodeEditorStyle {
    pub fn from_theme(theme: UiTheme) -> Self {
        let colors = theme.editor_syntax;
        let syntax = CodeEditorSyntaxPalette::uniform(theme.editor_foreground)
            .with_color(CodeEditorTokenRole::Attribute, colors.attribute)
            .with_color(CodeEditorTokenRole::Comment, colors.comment)
            .with_color(CodeEditorTokenRole::Constant, colors.constant)
            .with_color(CodeEditorTokenRole::Constructor, colors.constructor)
            .with_color(CodeEditorTokenRole::Embedded, colors.embedded)
            .with_color(CodeEditorTokenRole::Function, colors.function)
            .with_color(CodeEditorTokenRole::Keyword, colors.keyword)
            .with_color(CodeEditorTokenRole::Label, colors.label)
            .with_color(CodeEditorTokenRole::Module, colors.module)
            .with_color(CodeEditorTokenRole::Number, colors.number)
            .with_color(CodeEditorTokenRole::Operator, colors.operator)
            .with_color(CodeEditorTokenRole::Property, colors.property)
            .with_color(CodeEditorTokenRole::Punctuation, colors.punctuation)
            .with_color(CodeEditorTokenRole::Regexp, colors.regexp)
            .with_color(CodeEditorTokenRole::String, colors.string)
            .with_color(CodeEditorTokenRole::Type, colors.type_name)
            .with_color(CodeEditorTokenRole::Variable, colors.variable);
        Self::new(CodeEditorPalette {
            surface: theme.content_background,
            header: theme.side_bar_background,
            gutter: theme.side_bar_background,
            divider: theme.border,
            text: theme.editor_foreground,
            text_muted: theme.muted_foreground,
            selection: theme.text_selection_background,
            caret: theme.accent,
            composition_underline: theme.accent,
            diagnostics: CodeEditorDiagnosticPalette {
                error: theme.error,
                warning: theme.warning,
                information: theme.accent,
                hint: theme.muted_foreground,
            },
            syntax,
        })
    }
}

impl MultiDiffEditorStyle {
    pub fn from_theme(theme: UiTheme) -> Self {
        let diff_editor = DiffEditorStyle::new(DiffEditorPalette {
            code_editor: CodeEditorStyle::from_theme(theme),
            divider: theme.border,
            removed_marker: theme.diff_removed_marker,
            added_marker: theme.diff_inserted_marker,
            removed_line: theme.diff_removed_line,
            added_line: theme.diff_inserted_line,
            removed_inline: theme.diff_removed_text,
            added_inline: theme.diff_inserted_text,
            missing_line: theme.diff_missing_line,
            fold_line: theme.diff_unchanged_region,
            fold_marker: theme.diff_unchanged_region_foreground,
        });
        Self::new(MultiDiffEditorPalette {
            surface: theme.content_background,
            file_header: theme.side_bar_background,
            divider: theme.border,
            file_name: TextStyle::new(theme.font_size_label(), theme.foreground)
                .with_family(FontFamily::Monospace)
                .with_weight(FontWeight::Bold)
                .with_line_height(18.0),
            diff_editor,
            header_button: ButtonStyle::new(
                ButtonBackgrounds::new(theme.side_bar_background)
                    .with_hovered(theme.list_hover_background)
                    .with_focused(theme.list_active_background)
                    .with_pressed(theme.list_active_background),
                TextStyle::new(theme.font_size_label(), theme.foreground),
            )
            .with_padding(Edges::uniform(5.0))
            .with_icon_size(14.0)
            .with_corner_radii(CornerRadii::uniform(4.0)),
            header_icon: theme.muted_foreground,
            scroll_view: theme.file_list_scroll_view_style(),
        })
        .cards()
    }
}
