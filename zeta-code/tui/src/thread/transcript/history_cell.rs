mod cache;
mod content;
mod local_command;
mod text;

pub(crate) use cache::ChatHistoryRenderCache;
pub(super) use cache::PreparedCell;
pub(super) use content::ContentCell;
pub(super) use local_command::LocalCommandCell;
pub(crate) use local_command::LocalCommandCompletion;
pub(super) use text::prefixed_body;
pub(super) use text::push_detail_lines;

use super::model::TranscriptCell;
use crate::render::RenderContext;
use crate::render::action_style;
use ratatui::text::Line;
use ratatui::text::Span;
use std::borrow::Cow;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MessageRole {
    User,
    Agent,
    Reasoning,
    Plan,
    Command,
    Notice,
    Error,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CommandStatus {
    Submitted,
    Running,
    Succeeded,
    Failed,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum CellMode {
    #[default]
    Collapsed,
    Expanded,
    History,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct CellLayout {
    pub(super) height: usize,
    pub(super) details_row: Option<usize>,
}

pub(super) struct CellLines {
    pub(super) lines: Vec<Line<'static>>,
    pub(super) user_input_lines: usize,
    pub(super) details_line: Option<usize>,
}

impl CellLines {
    pub(super) fn layout(&self, width: u16) -> CellLayout {
        CellLayout {
            height: crate::render::wrapped_height(&self.lines, width),
            details_row: self
                .details_line
                .map(|line| crate::render::wrapped_height(&self.lines[..line], width)),
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum SyntaxHighlighting {
    Enabled,
    Disabled,
}

#[derive(Clone, Copy)]
pub(super) enum DetailFormat {
    Plain,
    Ansi,
}

/// A concrete transcript item owns its text, display lines and expansion behavior.
/// The transcript composes these outputs without interpreting tool or message fields.
pub(super) trait HistoryCell: std::fmt::Debug {
    fn role(&self) -> MessageRole;
    fn summary(&self, mode: CellMode) -> Cow<'_, str>;
    fn detail(&self, mode: CellMode) -> Option<Cow<'_, str>>;
    fn can_expand(&self) -> bool {
        false
    }
    fn has_details(&self) -> bool {
        false
    }
    fn full_details(&self) -> Option<String> {
        None
    }
    fn lines(
        &self,
        view: &CellView<'_>,
        context: RenderContext<'_>,
        cache: Option<&ChatHistoryRenderCache>,
        highlighting: SyntaxHighlighting,
    ) -> CellLines;
}

#[derive(Clone, Debug)]
pub(crate) struct CellView<'a> {
    pub(super) cell: Cow<'a, TranscriptCell>,
    pub(crate) cell_id: Option<String>,
    pub(crate) render_revision: u64,
    pub(crate) can_expand: bool,
    pub(crate) expanded: bool,
    pub(crate) has_details: bool,
    pub(crate) selected: bool,
    pub(super) mode: CellMode,
}

impl CellView<'_> {
    pub(super) fn owner(&self) -> &dyn HistoryCell {
        self.cell.history_cell()
    }
    pub(crate) fn text(&self) -> Cow<'_, str> {
        self.owner().summary(self.mode)
    }
    pub(crate) fn detail(&self) -> Option<Cow<'_, str>> {
        self.owner().detail(self.mode)
    }
    pub(crate) fn role(&self) -> MessageRole {
        self.owner().role()
    }
    pub(super) fn lines(
        &self,
        context: RenderContext<'_>,
        cache: Option<&ChatHistoryRenderCache>,
        highlighting: SyntaxHighlighting,
    ) -> CellLines {
        self.owner().lines(self, context, cache, highlighting)
    }
}

pub(super) fn finish_lines(
    lines: &mut Vec<Line<'static>>,
    view: &CellView<'_>,
    context: RenderContext<'_>,
) -> Option<usize> {
    let details_line = (view.expanded && view.has_details).then_some(lines.len());
    if view.expanded && view.has_details {
        lines.push(Line::from(Span::styled(
            "   view full",
            action_style(context),
        )));
    }
    lines.push(Line::default());
    details_line
}
