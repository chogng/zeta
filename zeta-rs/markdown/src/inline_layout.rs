use zeta_ui::{Color, Point, Rect, Size, TextLayoutEngine, TextLayoutWidth, TextSpan, TextStyle};

use crate::MarkdownStyle;
use crate::document::InlineRun;

pub(crate) struct InlineLayout {
    pub(crate) spans: Vec<TextSpan>,
    pub(crate) style: TextStyle,
    pub(crate) size: Size,
    pub(crate) decorations: Vec<InlineDecoration>,
}

pub(crate) enum InlineDecoration {
    Code {
        fragments: Vec<Rect>,
    },
    Link {
        destination: String,
        fragments: Vec<Rect>,
    },
    Strikethrough {
        color: Color,
        fragments: Vec<Rect>,
    },
}

pub(crate) fn layout_inline(
    text: &mut TextLayoutEngine,
    runs: &[InlineRun],
    base: TextStyle,
    width: TextLayoutWidth,
    style: &MarkdownStyle,
) -> InlineLayout {
    let spans = runs
        .iter()
        .map(|run| TextSpan::new(&run.text, style.inline(&base, &run.format)))
        .collect::<Vec<_>>();
    let shaped = text.layout_spans(&spans, &base, width);
    let decorations = runs
        .iter()
        .enumerate()
        .flat_map(|(index, run)| {
            let fragments = shaped.span_fragments(index).to_vec();
            let mut decorations = Vec::with_capacity(3);
            if run.format.code && !fragments.is_empty() {
                decorations.push(InlineDecoration::Code {
                    fragments: fragments.clone(),
                });
            }
            if let Some(destination) = run.format.link.as_ref()
                && !fragments.is_empty()
            {
                decorations.push(InlineDecoration::Link {
                    destination: destination.clone(),
                    fragments: fragments.clone(),
                });
            }
            if run.format.strikethrough && !fragments.is_empty() {
                decorations.push(InlineDecoration::Strikethrough {
                    color: spans[index].style().color(),
                    fragments,
                });
            }
            decorations
        })
        .collect();
    InlineLayout {
        spans,
        style: base,
        size: shaped.size(),
        decorations,
    }
}

pub(crate) fn offset_rect(rect: Rect, origin: Point) -> Rect {
    Rect::from_xywh(
        rect.origin.x + origin.x,
        rect.origin.y + origin.y,
        rect.size.width,
        rect.size.height,
    )
}
