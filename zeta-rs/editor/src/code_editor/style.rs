//! CodeEditor-owned presentation tokens.

use zeta_ui::{Border, Color, Edges, FontFamily, FontWeight, PaintRect, Rect, TextStyle};

use super::{HEADER_HEIGHT, ROW_HEIGHT};

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
    text_style: TextStyle,
    header_style: TextStyle,
}

impl CodeEditorStyle {
    pub fn light() -> Self {
        Self {
            surface: Color::WHITE,
            header: Color::rgb(246, 246, 247),
            gutter: Color::rgb(247, 247, 248),
            divider: Color::rgb(222, 222, 224),
            text_muted: Color::rgb(126, 126, 132),
            selection: Color::rgba(68, 139, 202, 72),
            caret: Color::rgb(15, 110, 96),
            composition_underline: Color::rgb(15, 110, 96),
            text_style: TextStyle::new(13.0, Color::rgb(38, 38, 41))
                .with_family(FontFamily::Monospace)
                .with_line_height(ROW_HEIGHT),
            header_style: TextStyle::new(12.0, Color::rgb(38, 38, 41))
                .with_family(FontFamily::Monospace)
                .with_weight(FontWeight::Bold)
                .with_line_height(HEADER_HEIGHT),
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

    pub(super) const fn header_rect(&self, bounds: Rect) -> PaintRect {
        PaintRect::new(bounds, self.header)
            .with_border(Border::new(Edges::new(0.0, 0.0, 1.0, 0.0), self.divider))
    }
}
