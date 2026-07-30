use zeta_ui::{
    Border, Component, CornerRadii, PaintRect, Point, Rect, ScrollMetrics, ScrollState, Size,
    TextBlock, TextLayoutEngine, TextLayoutWidth, TextStyle, UiScene,
};

use crate::document::{BlockContext, InlineRun, MarkdownBlockKind};
use crate::inline_layout::{InlineDecoration, InlineLayout, layout_inline, offset_rect};
use crate::table_layout::{
    CELL_HORIZONTAL_PADDING, CELL_VERTICAL_PADDING, TableLayout, layout_table,
};
use crate::{MarkdownDocument, MarkdownStyle};

const QUOTE_BAR_WIDTH: f32 = 3.0;
const INLINE_CODE_HORIZONTAL_PADDING: f32 = 3.0;
const INLINE_CODE_VERTICAL_INSET: f32 = 1.0;
const DECORATION_THICKNESS: f32 = 1.0;

/// Reusable Markdown paragraph shaper and block layout engine.
///
/// Hosts retain one engine and call [`layout`](Self::layout) whenever the document, bounds, style,
/// or viewport changes. The engine owns font shaping state only; product identity and scrolling
/// input remain with the host.
pub struct MarkdownLayoutEngine {
    text: TextLayoutEngine,
}

impl MarkdownLayoutEngine {
    pub fn new() -> Self {
        Self {
            text: TextLayoutEngine::new(),
        }
    }

    pub fn layout(
        &mut self,
        bounds: Rect,
        document: &MarkdownDocument,
        scroll: ScrollState,
        style: &MarkdownStyle,
    ) -> Markdown {
        if bounds.is_empty() || document.is_empty() {
            return Markdown {
                bounds,
                content_height: 0.0,
                vertical_offset: 0.0,
                rects: Vec::new(),
                text: Vec::new(),
                links: Vec::new(),
            };
        }
        let mut blocks = Vec::with_capacity(document.blocks.len());
        let mut top = 0.0;
        for block in &document.blocks {
            let projected =
                self.project_block(bounds.size.width, &block.kind, &block.context, style);
            let height = projected.height();
            blocks.push(PositionedBlock {
                top,
                context: block.context.clone(),
                projected,
            });
            top += height + style.block_gap();
        }
        let content_height = (top - style.block_gap()).max(0.0);
        let metrics = ScrollMetrics::new(bounds.size, Size::new(bounds.size.width, content_height));
        let vertical_offset = scroll
            .vertical_offset()
            .clamp(0.0, metrics.maximum_offset().y);
        let mut markdown = Markdown {
            bounds,
            content_height,
            vertical_offset,
            rects: Vec::new(),
            text: Vec::new(),
            links: Vec::new(),
        };
        for block in blocks {
            markdown.emit(block, style);
        }
        markdown
    }

    fn project_block(
        &mut self,
        total_width: f32,
        kind: &MarkdownBlockKind,
        context: &BlockContext,
        style: &MarkdownStyle,
    ) -> ProjectedBlock {
        let leading = context.quote_depth as f32 * style.quote_indent()
            + context.list_depth as f32 * style.list_indent();
        let width = (total_width - leading).max(1.0);
        match kind {
            MarkdownBlockKind::Paragraph(runs) => {
                self.project_text(runs, style.body().clone(), width, style)
            }
            MarkdownBlockKind::Heading { level, runs } => {
                self.project_text(runs, style.heading(*level), width, style)
            }
            MarkdownBlockKind::Table(table) => {
                ProjectedBlock::Table(layout_table(&mut self.text, table, width, style))
            }
            MarkdownBlockKind::CodeBlock { language, text } => {
                let padding = style.code_padding();
                let text_style = style.code_block();
                let text_width = (width - padding * 2.0).max(1.0);
                let label_height = language.as_ref().map_or(0.0, |_| text_style.line_height());
                let measured =
                    self.text
                        .measure_text(text, &text_style, TextLayoutWidth::Wrap(text_width));
                ProjectedBlock::Code {
                    language: language.clone(),
                    text: text.clone(),
                    style: text_style,
                    width,
                    text_height: measured.height,
                    label_height,
                    padding,
                }
            }
            MarkdownBlockKind::Rule => ProjectedBlock::Rule { width },
        }
    }

    fn project_text(
        &mut self,
        runs: &[InlineRun],
        base: TextStyle,
        width: f32,
        style: &MarkdownStyle,
    ) -> ProjectedBlock {
        ProjectedBlock::Text {
            inline: layout_inline(
                &mut self.text,
                runs,
                base,
                TextLayoutWidth::Wrap(width.max(1.0)),
                style,
            ),
            width,
        }
    }
}

impl Default for MarkdownLayoutEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Immutable, already-shaped Markdown component for one viewport frame.
pub struct Markdown {
    bounds: Rect,
    content_height: f32,
    vertical_offset: f32,
    rects: Vec<PaintRect>,
    text: Vec<TextBlock>,
    links: Vec<MarkdownLink>,
}

/// A laid-out link whose destination remains untrusted until the host activates it.
#[derive(Clone, Debug, PartialEq)]
pub struct MarkdownLink {
    destination: String,
    bounds: Vec<Rect>,
}

impl MarkdownLink {
    pub fn destination(&self) -> &str {
        &self.destination
    }

    /// Returns visible, viewport-clipped hit fragments for this link.
    pub fn bounds(&self) -> &[Rect] {
        &self.bounds
    }
}

impl Markdown {
    pub const fn bounds(&self) -> Rect {
        self.bounds
    }

    pub const fn content_height(&self) -> f32 {
        self.content_height
    }

    pub const fn vertical_offset(&self) -> f32 {
        self.vertical_offset
    }

    pub fn links(&self) -> &[MarkdownLink] {
        &self.links
    }

    pub fn link_at(&self, point: Point) -> Option<&MarkdownLink> {
        self.bounds.contains(point).then_some(())?;
        self.links
            .iter()
            .find(|link| link.bounds.iter().any(|bounds| bounds.contains(point)))
    }

    fn emit(&mut self, block: PositionedBlock, style: &MarkdownStyle) {
        let leading = block.context.quote_depth as f32 * style.quote_indent()
            + block.context.list_depth as f32 * style.list_indent();
        let x = self.bounds.origin.x + leading;
        let y = self.bounds.origin.y + block.top - self.vertical_offset;
        let height = block.projected.height();
        let block_bounds = Rect::from_xywh(x, y, block.projected.width(), height);
        if block_bounds.intersection(self.bounds).is_empty() {
            return;
        }
        self.emit_quotes(y, height, block.context.quote_depth, style);
        self.emit_marker(y, height, &block.context, style);
        match block.projected {
            ProjectedBlock::Text { inline, width } => {
                self.emit_inline(inline, Point::new(x, y), width, style);
            }
            ProjectedBlock::Code {
                language,
                text,
                style: text_style,
                width,
                text_height,
                label_height,
                padding,
            } => {
                self.rects.push(
                    PaintRect::new(
                        Rect::from_xywh(x, y, width, height),
                        style.code_background(),
                    )
                    .with_border(Border::uniform(1.0, style.border()))
                    .with_corner_radii(CornerRadii::uniform(4.0)),
                );
                let mut text_y = y + padding;
                if let Some(language) = language {
                    self.text.push(TextBlock::new(
                        language,
                        Point::new(x + padding, text_y),
                        Size::new((width - padding * 2.0).max(1.0), label_height.max(1.0)),
                        TextStyle::new(text_style.font_size(), style.muted())
                            .with_family(text_style.family().clone())
                            .with_line_height(text_style.line_height()),
                    ));
                    text_y += label_height;
                }
                if !text.is_empty() {
                    self.text.push(TextBlock::new(
                        text,
                        Point::new(x + padding, text_y),
                        Size::new((width - padding * 2.0).max(1.0), text_height.max(1.0)),
                        text_style,
                    ));
                }
            }
            ProjectedBlock::Rule { width } => {
                self.rects.push(PaintRect::new(
                    Rect::from_xywh(x, y, width, 1.0),
                    style.border(),
                ));
            }
            ProjectedBlock::Table(table) => self.emit_table(table, Point::new(x, y), style),
        }
    }

    fn emit_inline(
        &mut self,
        inline: InlineLayout,
        origin: Point,
        width: f32,
        style: &MarkdownStyle,
    ) {
        for decoration in inline.decorations {
            match decoration {
                InlineDecoration::Code { fragments } => {
                    for fragment in fragments {
                        let bounds = offset_rect(fragment, origin);
                        self.rects.push(
                            PaintRect::new(
                                Rect::from_xywh(
                                    bounds.origin.x - INLINE_CODE_HORIZONTAL_PADDING,
                                    bounds.origin.y + INLINE_CODE_VERTICAL_INSET,
                                    bounds.size.width + INLINE_CODE_HORIZONTAL_PADDING * 2.0,
                                    (bounds.size.height - INLINE_CODE_VERTICAL_INSET * 2.0)
                                        .max(1.0),
                                ),
                                style.inline_code_background(),
                            )
                            .with_corner_radii(CornerRadii::uniform(3.0)),
                        );
                    }
                }
                InlineDecoration::Link {
                    destination,
                    fragments,
                } => {
                    let bounds = fragments
                        .into_iter()
                        .map(|fragment| offset_rect(fragment, origin))
                        .map(|fragment| fragment.intersection(self.bounds))
                        .filter(|fragment| !fragment.is_empty())
                        .collect::<Vec<_>>();
                    for fragment in &bounds {
                        self.rects.push(PaintRect::new(
                            Rect::from_xywh(
                                fragment.origin.x,
                                fragment.bottom() - DECORATION_THICKNESS,
                                fragment.size.width,
                                DECORATION_THICKNESS,
                            ),
                            style.link(),
                        ));
                    }
                    if !bounds.is_empty() {
                        self.links.push(MarkdownLink {
                            destination,
                            bounds,
                        });
                    }
                }
                InlineDecoration::Strikethrough { color, fragments } => {
                    for fragment in fragments {
                        let fragment = offset_rect(fragment, origin);
                        self.rects.push(PaintRect::new(
                            Rect::from_xywh(
                                fragment.origin.x,
                                fragment.origin.y + fragment.size.height * 0.52,
                                fragment.size.width,
                                DECORATION_THICKNESS,
                            ),
                            color,
                        ));
                    }
                }
            }
        }
        if !inline.spans.is_empty() {
            self.text.push(TextBlock::from_spans(
                inline.spans,
                origin,
                Size::new(width, inline.size.height.max(1.0)),
                inline.style,
            ));
        }
    }

    fn emit_table(&mut self, table: TableLayout, origin: Point, style: &MarkdownStyle) {
        let mut row_y = origin.y;
        for row in table.rows {
            if row.header {
                self.rects.push(PaintRect::new(
                    Rect::from_xywh(origin.x, row_y, table.width, row.height),
                    style.table_header_background(),
                ));
            }
            let mut cell_x = origin.x;
            for (index, cell) in row.cells.into_iter().enumerate() {
                self.emit_inline(
                    cell,
                    Point::new(
                        cell_x + CELL_HORIZONTAL_PADDING,
                        row_y + CELL_VERTICAL_PADDING,
                    ),
                    (table.column_widths[index] - CELL_HORIZONTAL_PADDING * 2.0).max(1.0),
                    style,
                );
                cell_x += table.column_widths[index];
            }
            row_y += row.height;
            self.rects.push(PaintRect::new(
                Rect::from_xywh(origin.x, row_y - DECORATION_THICKNESS, table.width, 1.0),
                style.border(),
            ));
        }
        self.rects.push(PaintRect::new(
            Rect::from_xywh(origin.x, origin.y, table.width, DECORATION_THICKNESS),
            style.border(),
        ));
        let mut boundary_x = origin.x;
        self.rects.push(PaintRect::new(
            Rect::from_xywh(boundary_x, origin.y, DECORATION_THICKNESS, table.height),
            style.border(),
        ));
        for column_width in table.column_widths {
            boundary_x += column_width;
            self.rects.push(PaintRect::new(
                Rect::from_xywh(
                    boundary_x - DECORATION_THICKNESS,
                    origin.y,
                    DECORATION_THICKNESS,
                    table.height,
                ),
                style.border(),
            ));
        }
    }

    fn emit_quotes(&mut self, y: f32, height: f32, depth: usize, style: &MarkdownStyle) {
        for index in 0..depth {
            self.rects.push(PaintRect::new(
                Rect::from_xywh(
                    self.bounds.origin.x + index as f32 * style.quote_indent(),
                    y,
                    QUOTE_BAR_WIDTH,
                    height,
                ),
                style.border(),
            ));
        }
    }

    fn emit_marker(&mut self, y: f32, height: f32, context: &BlockContext, style: &MarkdownStyle) {
        let Some(marker) = context.marker.as_ref() else {
            return;
        };
        let x = self.bounds.origin.x
            + context.quote_depth as f32 * style.quote_indent()
            + context.list_depth.saturating_sub(1) as f32 * style.list_indent();
        self.text.push(TextBlock::new(
            marker,
            Point::new(x, y),
            Size::new(style.list_indent(), height.max(1.0)),
            TextStyle::new(style.body().font_size(), style.muted())
                .with_family(style.body().family().clone())
                .with_line_height(style.body().line_height()),
        ));
    }
}

impl Component for Markdown {
    fn paint(&self, scene: &mut UiScene) {
        if self.bounds.is_empty() {
            return;
        }
        scene.with_clip(self.bounds, |scene| {
            for rect in &self.rects {
                scene.draw_rect(*rect);
            }
            for text in &self.text {
                scene.draw_text(text.clone());
            }
        });
    }
}

struct PositionedBlock {
    top: f32,
    context: BlockContext,
    projected: ProjectedBlock,
}

enum ProjectedBlock {
    Text {
        inline: InlineLayout,
        width: f32,
    },
    Code {
        language: Option<String>,
        text: String,
        style: TextStyle,
        width: f32,
        text_height: f32,
        label_height: f32,
        padding: f32,
    },
    Rule {
        width: f32,
    },
    Table(TableLayout),
}

impl ProjectedBlock {
    fn height(&self) -> f32 {
        match self {
            Self::Text { inline, .. } => inline.size.height,
            Self::Code {
                text_height,
                label_height,
                padding,
                ..
            } => text_height + label_height + padding * 2.0,
            Self::Rule { .. } => 1.0,
            Self::Table(table) => table.height,
        }
    }

    const fn width(&self) -> f32 {
        match self {
            Self::Text { width, .. } | Self::Code { width, .. } | Self::Rule { width } => *width,
            Self::Table(table) => table.width,
        }
    }
}
