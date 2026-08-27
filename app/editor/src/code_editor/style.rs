//! CodeEditor-owned presentation tokens.

use zui::ui::{Border, Color, Edges, FontFamily, FontWeight, PaintRect, Rect, TextStyle};

use super::{
    CodeEditorDiagnosticPalette, CodeEditorDiagnosticSeverity, CodeEditorSyntaxPalette,
    CodeEditorTokenRole, HEADER_HEIGHT, ROW_HEIGHT,
};

/// Resolved color inputs used to construct one CodeEditor presentation style.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodeEditorPalette {
    pub surface: Color,
    pub header: Color,
    pub gutter: Color,
    pub divider: Color,
    pub text: Color,
    pub text_muted: Color,
    pub selection: Color,
    pub caret: Color,
    pub composition_underline: Color,
    pub diagnostics: CodeEditorDiagnosticPalette,
    pub syntax: CodeEditorSyntaxPalette,
}

/// Semantic surface and typography owned by the shared CodeEditor viewport.
#[derive(Clone, Debug, PartialEq)]
pub struct CodeEditorStyle {
    surface: Color,
    header: Color,
    gutter: Color,
    divider: Color,
    text_muted: Color,
    selection: Color,
    caret: Color,
    composition_underline: Color,
    diagnostics: CodeEditorDiagnosticPalette,
    text_style: TextStyle,
    header_style: TextStyle,
    syntax: CodeEditorSyntaxPalette,
}

impl CodeEditorStyle {
    pub fn light() -> Self {
        let text = Color::rgb(38, 38, 41);
        let syntax = CodeEditorSyntaxPalette::uniform(text)
            .with_color(CodeEditorTokenRole::Comment, Color::rgb(126, 126, 132))
            .with_color(CodeEditorTokenRole::Function, Color::rgb(15, 110, 96))
            .with_color(CodeEditorTokenRole::Keyword, Color::rgb(130, 80, 223))
            .with_color(CodeEditorTokenRole::Constant, Color::rgb(130, 80, 223))
            .with_color(CodeEditorTokenRole::String, Color::rgb(154, 103, 0))
            .with_color(CodeEditorTokenRole::Number, Color::rgb(154, 103, 0))
            .with_color(CodeEditorTokenRole::Operator, Color::rgb(207, 34, 46))
            .with_color(CodeEditorTokenRole::Punctuation, Color::rgb(207, 34, 46))
            .with_color(CodeEditorTokenRole::Property, Color::rgb(9, 105, 218))
            .with_color(CodeEditorTokenRole::Variable, Color::rgb(9, 105, 218));
        Self::new(CodeEditorPalette {
            surface: Color::WHITE,
            header: Color::rgb(246, 246, 247),
            gutter: Color::rgb(247, 247, 248),
            divider: Color::rgb(222, 222, 224),
            text,
            text_muted: Color::rgb(126, 126, 132),
            selection: Color::rgba(68, 139, 202, 72),
            caret: Color::rgb(15, 110, 96),
            composition_underline: Color::rgb(15, 110, 96),
            diagnostics: CodeEditorDiagnosticPalette {
                error: Color::rgb(180, 38, 38),
                warning: Color::rgb(154, 103, 0),
                information: Color::rgb(9, 105, 218),
                hint: Color::rgb(126, 126, 132),
            },
            syntax,
        })
    }

    pub fn new(palette: CodeEditorPalette) -> Self {
        Self {
            surface: palette.surface,
            header: palette.header,
            gutter: palette.gutter,
            divider: palette.divider,
            text_muted: palette.text_muted,
            selection: palette.selection,
            caret: palette.caret,
            composition_underline: palette.composition_underline,
            diagnostics: palette.diagnostics,
            text_style: TextStyle::new(13.0, palette.text)
                .with_family(FontFamily::Monospace)
                .with_line_height(ROW_HEIGHT),
            header_style: TextStyle::new(12.0, palette.text)
                .with_family(FontFamily::Monospace)
                .with_weight(FontWeight::Bold)
                .with_line_height(HEADER_HEIGHT),
            syntax: palette.syntax,
        }
    }

    pub(crate) const fn surface(&self) -> Color {
        self.surface
    }

    pub(super) const fn gutter(&self) -> Color {
        self.gutter
    }

    pub(super) const fn selection(&self) -> Color {
        self.selection
    }

    pub(super) const fn caret(&self) -> Color {
        self.caret
    }

    pub(super) const fn composition_underline(&self) -> Color {
        self.composition_underline
    }

    pub(super) const fn diagnostic_color(&self, severity: CodeEditorDiagnosticSeverity) -> Color {
        self.diagnostics.color(severity)
    }

    pub(super) const fn text_style(&self) -> &TextStyle {
        &self.text_style
    }

    pub(super) const fn header_text_style(&self) -> &TextStyle {
        &self.header_style
    }

    pub(super) fn muted_text_style(&self) -> TextStyle {
        self.text_with_color(self.text_muted)
    }

    pub(super) fn text_with_color(&self, color: Color) -> TextStyle {
        TextStyle::new(self.text_style.font_size(), color)
            .with_family(self.text_style.family().clone())
            .with_line_height(self.text_style.line_height())
            .with_weight(self.text_style.weight())
            .with_style(self.text_style.style())
    }

    pub(super) const fn syntax_color(&self, role: CodeEditorTokenRole) -> Color {
        self.syntax.color(role)
    }

    pub(super) const fn header_rect(&self, bounds: Rect) -> PaintRect {
        PaintRect::new(bounds, self.header)
            .with_border(Border::new(Edges::new(0.0, 0.0, 1.0, 0.0), self.divider))
    }
}
