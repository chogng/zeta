use glyphon::{Attrs, Buffer, Metrics, Shaping, Wrap};

use crate::font::mapping::{glyphon_family, glyphon_style, glyphon_weight};
use crate::font::new_font_system;
use crate::{Rect, Size, TextSpan, TextStyle};

/// Horizontal constraint used while measuring shaped text.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TextLayoutWidth {
    Unbounded,
    Wrap(f32),
}

/// Shaped logical geometry for a rich-text paragraph.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextLayout {
    size: Size,
    span_fragments: Vec<Vec<Rect>>,
}

impl TextLayout {
    pub const fn size(&self) -> Size {
        self.size
    }

    /// Returns visual-line fragments for the source span at `span_index`.
    ///
    /// Bounds are relative to the paragraph origin and follow the same shaping result used to
    /// measure the paragraph. A span can produce multiple fragments after wrapping or BiDi layout.
    pub fn span_fragments(&self, span_index: usize) -> &[Rect] {
        self.span_fragments
            .get(span_index)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }
}

/// Reusable shaping state for measuring plain or rich text with the UI renderer's font policy.
///
/// Hosts should retain one engine for repeated layout. The engine returns logical-pixel geometry
/// and does not own scene state, GPU resources, component identity, or input routing.
pub struct TextLayoutEngine {
    font_system: glyphon::FontSystem,
}

impl TextLayoutEngine {
    pub fn new() -> Self {
        Self {
            font_system: new_font_system(),
        }
    }

    pub fn measure_text(&mut self, text: &str, style: &TextStyle, width: TextLayoutWidth) -> Size {
        self.measure(text, &[], style, width)
    }

    pub fn measure_spans(
        &mut self,
        spans: &[TextSpan],
        style: &TextStyle,
        width: TextLayoutWidth,
    ) -> Size {
        self.layout_spans(spans, style, width).size()
    }

    /// Shapes rich text and returns paragraph extent plus per-span visual fragments.
    pub fn layout_spans(
        &mut self,
        spans: &[TextSpan],
        style: &TextStyle,
        width: TextLayoutWidth,
    ) -> TextLayout {
        let text = spans.iter().map(TextSpan::text).collect::<String>();
        self.layout(&text, spans, style, width)
    }

    fn measure(
        &mut self,
        text: &str,
        spans: &[TextSpan],
        style: &TextStyle,
        width: TextLayoutWidth,
    ) -> Size {
        self.layout(text, spans, style, width).size()
    }

    fn layout(
        &mut self,
        text: &str,
        spans: &[TextSpan],
        style: &TextStyle,
        width: TextLayoutWidth,
    ) -> TextLayout {
        if text.is_empty() {
            return TextLayout {
                size: Size::new(0.0, 0.0),
                span_fragments: vec![Vec::new(); spans.len()],
            };
        }
        let metrics = Metrics::new(style.font_size(), style.line_height());
        let mut buffer = Buffer::new(&mut self.font_system, metrics);
        match width {
            TextLayoutWidth::Unbounded => {
                buffer.set_wrap(Wrap::None);
                buffer.set_size(None, None);
            }
            TextLayoutWidth::Wrap(width) => {
                buffer.set_wrap(Wrap::WordOrGlyph);
                buffer.set_size(Some(width.max(0.0)), None);
            }
        }
        let attrs = attrs_for_style(style);
        if spans.is_empty() {
            buffer.set_text(text, &attrs, Shaping::Advanced, None);
        } else {
            buffer.set_rich_text(
                spans.iter().enumerate().map(|(index, span)| {
                    (
                        span.text(),
                        attrs_for_style(span.style()).metadata(index + 1),
                    )
                }),
                &attrs,
                Shaping::Advanced,
                None,
            );
        }
        buffer.shape_until_scroll(&mut self.font_system, false);
        let mut measured = Size::new(0.0, 0.0);
        let mut span_fragments = vec![Vec::new(); spans.len()];
        for run in buffer.layout_runs() {
            measured.width = measured.width.max(run.line_w);
            measured.height = measured.height.max(run.line_top + run.line_height);
            let mut glyphs = run
                .glyphs
                .iter()
                .filter_map(|glyph| {
                    glyph
                        .metadata
                        .checked_sub(1)
                        .filter(|index| *index < spans.len())
                        .map(|index| (index, glyph.x, glyph.x + glyph.w))
                })
                .collect::<Vec<_>>();
            glyphs.sort_by(|left, right| left.1.total_cmp(&right.1));
            let mut fragment: Option<(usize, f32, f32)> = None;
            for (index, left, right) in glyphs {
                match fragment {
                    Some((current, start, end))
                        if current == index && left <= end + f32::EPSILON =>
                    {
                        fragment = Some((current, start, end.max(right)));
                    }
                    Some(previous) => {
                        push_fragment(&mut span_fragments, previous, run.line_top, run.line_height);
                        fragment = Some((index, left, right));
                    }
                    None => fragment = Some((index, left, right)),
                }
            }
            if let Some(fragment) = fragment {
                push_fragment(&mut span_fragments, fragment, run.line_top, run.line_height);
            }
        }
        TextLayout {
            size: measured,
            span_fragments,
        }
    }
}

impl Default for TextLayoutEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn attrs_for_style(style: &TextStyle) -> Attrs<'_> {
    Attrs::new()
        .family(glyphon_family(style.family()))
        .weight(glyphon_weight(style.weight()))
        .style(glyphon_style(style.style()))
        .metrics(Metrics::new(style.font_size(), style.line_height()))
}

fn push_fragment(
    fragments: &mut [Vec<Rect>],
    (index, left, right): (usize, f32, f32),
    top: f32,
    height: f32,
) {
    fragments[index].push(Rect::from_xywh(left, top, (right - left).max(0.0), height));
}

#[cfg(test)]
#[path = "text_layout_tests.rs"]
mod tests;
