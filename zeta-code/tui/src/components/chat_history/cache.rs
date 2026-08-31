use super::Message;
use crate::render::RenderContext;
use crate::render::StreamingCodeHighlighter;
use crate::render::code_within_limits;
use crate::render::highlight_code;
use crate::render::line_to_borrowed;
use crate::render::push_owned_lines;
use crate::render::wrapped_height;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;
use ratatui::widgets::Wrap;
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::sync::Arc;

const MAX_CACHE_ENTRIES: usize = 256;
const MAX_CACHE_CELLS: usize = 250_000;
const MAX_CELL_CELLS: usize = 65_536;
const MAX_CODE_BLOCKS: usize = 64;
const MAX_CODE_SOURCE_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CellRenderMode {
    Normal,
    Selected,
    Expanded,
    ExpandedSelected,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CacheKey {
    cell_id: String,
    render_revision: u64,
    width: u16,
    theme_revision: u64,
    mode: CellRenderMode,
}

impl CacheKey {
    fn for_message(message: &Message, width: u16, context: RenderContext<'_>) -> Option<Self> {
        let cell_id = message.cell_id.clone()?;
        if message.render_revision == 0 {
            return None;
        }
        let mode = match (message.expanded, message.selected) {
            (false, false) => CellRenderMode::Normal,
            (false, true) => CellRenderMode::Selected,
            (true, false) => CellRenderMode::Expanded,
            (true, true) => CellRenderMode::ExpandedSelected,
        };
        Some(Self {
            cell_id,
            render_revision: message.render_revision,
            width,
            theme_revision: context.theme_revision(),
            mode,
        })
    }
}

#[derive(Debug)]
struct CacheEntry {
    key: CacheKey,
    cell: Arc<RenderedCell>,
    cost: usize,
}

#[derive(Debug)]
struct LayoutEntry {
    key: CacheKey,
    height: usize,
}

#[derive(Debug, Default)]
struct CacheEntries {
    entries: VecDeque<CacheEntry>,
    cells: usize,
}

#[derive(Debug, Default)]
pub(crate) struct ChatHistoryRenderCache {
    entries: RefCell<CacheEntries>,
    layouts: RefCell<HashMap<String, LayoutEntry>>,
    code_blocks: RefCell<CodeBlockEntries>,
}

#[derive(Debug, Default)]
struct CodeBlockEntries {
    entries: VecDeque<CodeBlockEntry>,
    source_bytes: usize,
}

#[derive(Debug)]
struct CodeBlockEntry {
    key: (String, usize),
    render: CodeBlockRender,
}

#[derive(Debug)]
struct CodeBlockRender {
    language: String,
    theme_revision: u64,
    complete_source: String,
    complete_lines: Vec<Line<'static>>,
    highlighter: Option<StreamingCodeHighlighter>,
}

impl ChatHistoryRenderCache {
    pub(crate) fn retain_messages(&self, messages: &[Message]) {
        let ids = messages
            .iter()
            .filter_map(|message| message.cell_id.as_ref())
            .collect::<HashSet<_>>();
        self.layouts
            .borrow_mut()
            .retain(|cell_id, _| ids.contains(cell_id));
        let mut entries = self.entries.borrow_mut();
        entries
            .entries
            .retain(|entry| ids.contains(&entry.key.cell_id));
        entries.cells = entries.entries.iter().map(|entry| entry.cost).sum();
        self.code_blocks.borrow_mut().retain(&ids);
    }

    pub(crate) fn measure<'a>(
        &self,
        message: &Message,
        width: u16,
        context: RenderContext<'_>,
        render: impl FnOnce() -> Vec<Line<'a>>,
    ) -> usize {
        let key = CacheKey::for_message(message, width, context);
        if let Some(key) = key.as_ref()
            && let Some(height) = self.cached_height(key)
        {
            return height;
        }
        let height = wrapped_height(&render(), width);
        if let Some(key) = key {
            self.insert_height(key, height);
        }
        height
    }

    pub(crate) fn prepare<'a>(
        &self,
        message: &Message,
        width: u16,
        context: RenderContext<'_>,
        render: impl FnOnce() -> Vec<Line<'a>>,
    ) -> PreparedCell {
        let key = CacheKey::for_message(message, width, context);
        if let Some(key) = key.as_ref()
            && let Some(cell) = self.cached(key)
        {
            return PreparedCell::Buffered(cell);
        }

        let borrowed = render();
        let mut lines = Vec::with_capacity(borrowed.len());
        push_owned_lines(&borrowed, &mut lines);
        let height = wrapped_height(&lines, width);
        if let Some(key) = key.as_ref() {
            self.insert_height(key.clone(), height);
        }
        let Some(cost) = usize::from(width).checked_mul(height) else {
            return PreparedCell::Lines {
                lines,
                background: message_background(message, context),
                separator_background: context.background(),
                height,
            };
        };
        let Some(buffer_height) = u16::try_from(height).ok() else {
            return PreparedCell::Lines {
                lines,
                background: message_background(message, context),
                separator_background: context.background(),
                height,
            };
        };
        if key.is_none() || cost > MAX_CELL_CELLS {
            return PreparedCell::Lines {
                lines,
                background: message_background(message, context),
                separator_background: context.background(),
                height,
            };
        }

        let area = Rect::new(0, 0, width, buffer_height);
        let mut buffer = Buffer::empty(area);
        buffer.set_style(
            area,
            Style::default()
                .fg(context.foreground())
                .bg(message_background(message, context)),
        );
        buffer.set_style(
            Rect::new(0, buffer_height.saturating_sub(1), width, 1),
            Style::default().bg(context.background()),
        );
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .render(area, &mut buffer);
        let cell = Arc::new(RenderedCell { buffer });
        self.insert(
            key.expect("cacheable messages have a key"),
            Arc::clone(&cell),
            cost,
        );
        PreparedCell::Buffered(cell)
    }

    fn cached(&self, key: &CacheKey) -> Option<Arc<RenderedCell>> {
        let mut cache = self.entries.borrow_mut();
        let index = cache.entries.iter().position(|entry| entry.key == *key)?;
        let entry = cache
            .entries
            .remove(index)
            .expect("the matching cache entry exists");
        let cell = Arc::clone(&entry.cell);
        cache.entries.push_back(entry);
        Some(cell)
    }

    fn cached_height(&self, key: &CacheKey) -> Option<usize> {
        self.layouts
            .borrow()
            .get(&key.cell_id)
            .filter(|entry| entry.key == *key)
            .map(|entry| entry.height)
    }

    fn insert_height(&self, key: CacheKey, height: usize) {
        self.layouts
            .borrow_mut()
            .insert(key.cell_id.clone(), LayoutEntry { key, height });
    }

    pub(crate) fn clear(&self) {
        *self.entries.borrow_mut() = CacheEntries::default();
        self.layouts.borrow_mut().clear();
        *self.code_blocks.borrow_mut() = CodeBlockEntries::default();
    }

    pub(crate) fn highlight_code_block(
        &self,
        message: &Message,
        block_index: usize,
        language: &str,
        source: &str,
        context: RenderContext<'_>,
    ) -> Vec<Line<'static>> {
        let Some(cell_id) = message.cell_id.as_ref() else {
            return highlight_code(source, language, context.into());
        };
        let key = (cell_id.clone(), block_index);
        let mut blocks = self.code_blocks.borrow_mut();
        if !code_within_limits(source) {
            blocks.remove(&key);
            return highlight_code(source, language, context.into());
        }
        let mut block = blocks
            .take(&key)
            .unwrap_or_else(|| CodeBlockRender::new(language, source, context));
        let lines = block.update(language, source, context);
        blocks.insert(key, block);
        lines
    }

    fn insert(&self, key: CacheKey, cell: Arc<RenderedCell>, cost: usize) {
        let mut cache = self.entries.borrow_mut();
        if let Some(index) = cache
            .entries
            .iter()
            .position(|entry| entry.key.cell_id == key.cell_id)
            && let Some(removed) = cache.entries.remove(index)
        {
            cache.cells = cache.cells.saturating_sub(removed.cost);
        }
        cache.cells = cache.cells.saturating_add(cost);
        cache.entries.push_back(CacheEntry { key, cell, cost });
        while cache.entries.len() > MAX_CACHE_ENTRIES || cache.cells > MAX_CACHE_CELLS {
            let Some(removed) = cache.entries.pop_front() else {
                break;
            };
            cache.cells = cache.cells.saturating_sub(removed.cost);
        }
    }

    #[cfg(test)]
    pub(crate) fn entry_count(&self) -> usize {
        self.entries.borrow().entries.len()
    }
}

impl CodeBlockEntries {
    fn retain(&mut self, cell_ids: &HashSet<&String>) {
        self.entries.retain(|entry| cell_ids.contains(&entry.key.0));
        self.source_bytes = self
            .entries
            .iter()
            .map(|entry| entry.render.complete_source.len())
            .sum();
    }

    fn take(&mut self, key: &(String, usize)) -> Option<CodeBlockRender> {
        let index = self.entries.iter().position(|entry| &entry.key == key)?;
        let entry = self
            .entries
            .remove(index)
            .expect("the matching code block entry exists");
        self.source_bytes = self
            .source_bytes
            .saturating_sub(entry.render.complete_source.len());
        Some(entry.render)
    }

    fn remove(&mut self, key: &(String, usize)) {
        let _ = self.take(key);
    }

    fn insert(&mut self, key: (String, usize), render: CodeBlockRender) {
        self.source_bytes = self
            .source_bytes
            .saturating_add(render.complete_source.len());
        self.entries.push_back(CodeBlockEntry { key, render });
        while self.entries.len() > MAX_CODE_BLOCKS || self.source_bytes > MAX_CODE_SOURCE_BYTES {
            let Some(entry) = self.entries.pop_front() else {
                break;
            };
            self.source_bytes = self
                .source_bytes
                .saturating_sub(entry.render.complete_source.len());
        }
    }
}

impl CodeBlockRender {
    fn new(language: &str, source: &str, context: RenderContext<'_>) -> Self {
        let (complete, _) = complete_source(source);
        let (highlighter, complete_lines) = StreamingCodeHighlighter::start(
            complete,
            language,
            context.into(),
            context.theme_revision(),
        )
        .expect("a complete code prefix is accepted by the streaming highlighter");
        Self {
            language: language.to_owned(),
            theme_revision: context.theme_revision(),
            complete_source: complete.to_owned(),
            complete_lines,
            highlighter: Some(highlighter),
        }
    }

    fn update(
        &mut self,
        language: &str,
        source: &str,
        context: RenderContext<'_>,
    ) -> Vec<Line<'static>> {
        let (complete, partial) = complete_source(source);
        let reusable = self.language == language
            && self.theme_revision == context.theme_revision()
            && complete.starts_with(&self.complete_source);
        if reusable && complete.len() > self.complete_source.len() {
            let appended = &complete[self.complete_source.len()..];
            let highlighter = self
                .highlighter
                .take()
                .expect("code block render state owns its highlighter");
            if let Some((highlighter, lines)) =
                highlighter.append(appended, context.into(), context.theme_revision())
            {
                self.highlighter = Some(highlighter);
                self.complete_source.push_str(appended);
                self.complete_lines.extend(lines);
            } else {
                let replacement = StreamingCodeHighlighter::start(
                    complete,
                    language,
                    context.into(),
                    context.theme_revision(),
                )
                .expect("a complete code prefix is accepted by the streaming highlighter");
                self.replace(language, complete, context, replacement);
            }
        } else if !reusable || complete.len() < self.complete_source.len() {
            let replacement = StreamingCodeHighlighter::start(
                complete,
                language,
                context.into(),
                context.theme_revision(),
            )
            .expect("a complete code prefix is accepted by the streaming highlighter");
            self.replace(language, complete, context, replacement);
        }

        let mut lines = self.complete_lines.clone();
        if !partial.is_empty() {
            lines.push(Line::from(Span::styled(
                partial.to_owned(),
                Style::default().fg(context.foreground()),
            )));
        }
        if lines.is_empty() {
            lines.push(Line::default());
        }
        lines
    }

    fn replace(
        &mut self,
        language: &str,
        complete: &str,
        context: RenderContext<'_>,
        replacement: (StreamingCodeHighlighter, Vec<Line<'static>>),
    ) {
        self.language = language.to_owned();
        self.theme_revision = context.theme_revision();
        self.complete_source = complete.to_owned();
        self.highlighter = Some(replacement.0);
        self.complete_lines = replacement.1;
    }
}

fn complete_source(source: &str) -> (&str, &str) {
    let complete_len = source.rfind('\n').map_or(0, |index| index + 1);
    source.split_at(complete_len)
}

pub(crate) enum PreparedCell {
    Buffered(Arc<RenderedCell>),
    Lines {
        lines: Vec<Line<'static>>,
        background: Color,
        separator_background: Color,
        height: usize,
    },
}

impl PreparedCell {
    pub(crate) fn render(&self, target: &mut Buffer, area: Rect, source_row: usize) {
        match self {
            Self::Buffered(cell) => cell.render(target, area, source_row),
            Self::Lines {
                lines,
                background,
                separator_background,
                height,
            } => {
                target.set_style(area, Style::default().bg(*background));
                if let Some(separator_row) = height.checked_sub(1).and_then(|separator_row| {
                    separator_row
                        .checked_sub(source_row)
                        .filter(|row| *row < usize::from(area.height))
                }) {
                    target.set_style(
                        Rect::new(
                            area.x,
                            area.y.saturating_add(separator_row as u16),
                            area.width,
                            1,
                        ),
                        Style::default().bg(*separator_background),
                    );
                }
                let (lines, source_row) = visible_lines(lines, area.width, source_row);
                let lines = lines.iter().map(line_to_borrowed).collect::<Vec<_>>();
                Paragraph::new(lines)
                    .wrap(Wrap { trim: false })
                    .scroll((source_row, 0))
                    .render(area, target);
            }
        }
    }
}

fn message_background(message: &Message, context: RenderContext<'_>) -> Color {
    if message.role == super::MessageRole::User {
        context.user_message_background()
    } else {
        context.background()
    }
}

#[derive(Debug)]
pub(crate) struct RenderedCell {
    buffer: Buffer,
}

impl RenderedCell {
    fn render(&self, target: &mut Buffer, area: Rect, source_row: usize) {
        let width = area.width.min(self.buffer.area.width);
        for row in 0..area.height {
            let source_y = source_row.saturating_add(usize::from(row));
            if source_y >= usize::from(self.buffer.area.height) {
                break;
            }
            let source_y = source_y as u16;
            for column in 0..width {
                let Some(source) = self.buffer.cell((column, source_y)) else {
                    continue;
                };
                if let Some(destination) =
                    target.cell_mut((area.x.saturating_add(column), area.y.saturating_add(row)))
                {
                    *destination = source.clone();
                }
            }
        }
    }
}

fn visible_lines<'a>(
    lines: &'a [Line<'static>],
    width: u16,
    mut source_row: usize,
) -> (&'a [Line<'static>], u16) {
    let mut first = 0;
    while first < lines.len() {
        let height = wrapped_height(std::slice::from_ref(&lines[first]), width);
        if source_row < height {
            break;
        }
        source_row -= height;
        first += 1;
    }
    (
        &lines[first..],
        source_row.min(usize::from(u16::MAX)) as u16,
    )
}

#[cfg(test)]
#[path = "cache_tests.rs"]
mod tests;
