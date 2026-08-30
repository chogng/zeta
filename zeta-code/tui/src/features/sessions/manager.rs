use super::branch_count_label;
use super::session_size_label;
use crate::components::detail_list::DetailList;
use crate::components::detail_list::DetailListRow;
use crate::components::pane::PaneSpec;
use crate::render::RenderContext;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use std::collections::BTreeSet;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;
use zeta_protocol::Session;
use zeta_protocol::SessionId;
use zeta_protocol::SessionManagerActivity;
use zeta_protocol::SessionManagerStatus;
use zeta_protocol::SessionStatus;
use zeta_utils_elapsed::format_compact_duration;

const ANIMATION_INTERVAL: Duration = Duration::from_millis(80);
const WORKING_FRAMES: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

#[derive(Debug)]
pub(crate) struct SessionManagerState {
    selected: Option<ManagerSelection>,
    focused: bool,
    pinned: BTreeSet<SessionId>,
    collapsed: BTreeSet<SessionGroup>,
    animation_frame: usize,
    last_animation_at: Option<Instant>,
    now_unix_ms: u64,
}

impl Default for SessionManagerState {
    fn default() -> Self {
        Self {
            selected: None,
            focused: false,
            pinned: BTreeSet::new(),
            collapsed: BTreeSet::new(),
            animation_frame: 0,
            last_animation_at: None,
            now_unix_ms: current_unix_millis(),
        }
    }
}

impl SessionManagerState {
    pub(crate) fn reconcile(&mut self, sessions: &[Session]) {
        self.pinned.retain(|session_id| {
            sessions
                .iter()
                .any(|session| &session.session_id == session_id)
        });
        let rows = manager_rows(sessions, &self.pinned, &self.collapsed);
        if self
            .selected
            .as_ref()
            .is_some_and(|selected| rows.iter().any(|row| row.selection() == *selected))
        {
            return;
        }
        self.selected = rows.first().map(ManagerRow::selection);
    }

    pub(crate) fn focus(&mut self) {
        self.focused = true;
    }

    pub(crate) fn blur(&mut self) {
        self.focused = false;
    }

    pub(crate) fn focused(&self) -> bool {
        self.focused
    }

    pub(crate) fn select_previous(&mut self, sessions: &[Session]) {
        self.select_offset(sessions, -1);
    }

    pub(crate) fn select_next(&mut self, sessions: &[Session]) -> bool {
        self.select_offset(sessions, 1)
    }

    pub(crate) fn selected_session(&self) -> Option<&SessionId> {
        match self.selected.as_ref() {
            Some(ManagerSelection::Session(session_id)) => Some(session_id),
            Some(ManagerSelection::Group(_)) | None => None,
        }
    }

    pub(crate) fn toggle_or_preview(
        &mut self,
        sessions: &[Session],
    ) -> Option<PaneSpec<DetailList>> {
        match self.selected.as_ref()? {
            ManagerSelection::Group(group) => {
                if !self.collapsed.remove(group) {
                    self.collapsed.insert(*group);
                }
                None
            }
            ManagerSelection::Session(session_id) => sessions
                .iter()
                .find(|session| &session.session_id == session_id)
                .map(|session| session_preview(session, self.now_unix_ms)),
        }
    }

    pub(crate) fn selected_archive_ids(&self, sessions: &[Session]) -> Vec<SessionId> {
        match self.selected.as_ref() {
            Some(ManagerSelection::Group(group)) => sessions
                .iter()
                .filter(|session| {
                    session.status == SessionStatus::Active && group.includes(session, &self.pinned)
                })
                .map(|session| session.session_id.clone())
                .collect(),
            Some(ManagerSelection::Session(session_id)) => sessions
                .iter()
                .filter(|session| {
                    &session.session_id == session_id && session.status == SessionStatus::Active
                })
                .map(|session| session.session_id.clone())
                .collect(),
            None => Vec::new(),
        }
    }

    pub(crate) fn selection_hint(&self) -> &'static str {
        match self.selected.as_ref() {
            Some(ManagerSelection::Group(group)) if self.collapsed.contains(group) => {
                "↑↓ select · space to expand · ctrl+x to archive all"
            }
            Some(ManagerSelection::Group(_)) => {
                "↑↓ select · space to collapse · ctrl+x to archive all"
            }
            Some(ManagerSelection::Session(_)) => {
                "↑↓ select · space to preview · ctrl+x to archive"
            }
            None => "esc to input",
        }
    }

    pub(crate) fn toggle_selected_pin(&mut self) -> bool {
        let Some(ManagerSelection::Session(selected)) = self.selected.clone() else {
            return false;
        };
        if !self.pinned.remove(&selected) {
            self.pinned.insert(selected);
        }
        true
    }

    pub(crate) fn refresh_time(&mut self, now: Instant, sessions: &[Session]) -> bool {
        let next_unix_ms = current_unix_millis();
        let elapsed_label_changed = self.now_unix_ms / 1_000 != next_unix_ms / 1_000;
        self.now_unix_ms = next_unix_ms;
        if !sessions
            .iter()
            .any(|session| session.manager.status == SessionManagerStatus::Working)
        {
            self.last_animation_at = None;
            return elapsed_label_changed;
        }
        let Some(last_animation_at) = self.last_animation_at else {
            self.last_animation_at = Some(now);
            return elapsed_label_changed;
        };
        let elapsed = now.saturating_duration_since(last_animation_at);
        let steps = elapsed.as_millis() / ANIMATION_INTERVAL.as_millis();
        if steps == 0 {
            return elapsed_label_changed;
        }
        self.animation_frame = (self.animation_frame
            + usize::try_from(steps).unwrap_or(usize::MAX))
            % WORKING_FRAMES.len();
        self.last_animation_at = Some(
            last_animation_at
                + ANIMATION_INTERVAL.saturating_mul(u32::try_from(steps).unwrap_or(u32::MAX)),
        );
        true
    }

    pub(crate) fn view<'a>(&'a self, sessions: &'a [Session]) -> SessionManagerView<'a> {
        SessionManagerView {
            sessions,
            selected: self.selected.as_ref(),
            focused: self.focused,
            pinned: &self.pinned,
            collapsed: &self.collapsed,
            animation_frame: self.animation_frame,
            now_unix_ms: self.now_unix_ms,
        }
    }

    fn select_offset(&mut self, sessions: &[Session], delta: isize) -> bool {
        let rows = manager_rows(sessions, &self.pinned, &self.collapsed);
        let Some(current) = self.selected.as_ref() else {
            self.selected = rows.first().map(ManagerRow::selection);
            return self.selected.is_some();
        };
        let Some(index) = rows.iter().position(|row| row.selection() == *current) else {
            self.selected = rows.first().map(ManagerRow::selection);
            return self.selected.is_some();
        };
        let next = index
            .saturating_add_signed(delta)
            .min(rows.len().saturating_sub(1));
        let changed = next != index;
        self.selected = rows.get(next).map(ManagerRow::selection);
        changed
    }
}

fn session_preview(session: &Session, now_unix_ms: u64) -> PaneSpec<DetailList> {
    let mut rows = vec![
        DetailListRow::new("Session", session.title.clone()),
        DetailListRow::new("ID", session.session_id.to_string()),
        DetailListRow::new("Status", manager_status_label(session.manager.status)),
        DetailListRow::new("Time", elapsed_label(session, now_unix_ms)),
        DetailListRow::new("Branches", branch_count_label(session)),
        DetailListRow::new("Size", session_size_label(session)),
    ];
    let activity = activity_text(session);
    if !activity.is_empty() {
        rows.push(DetailListRow::new("Activity", activity));
    }
    for thread in &session.threads {
        rows.push(DetailListRow::new(
            if thread.parent_thread_id.is_some() {
                "Branch"
            } else {
                "Root"
            },
            format!("{} · {}", thread.title, thread_status_label(thread.status)),
        ));
    }
    PaneSpec::new(DetailList::new("Session preview", rows), "Esc back")
}

fn manager_status_label(status: SessionManagerStatus) -> &'static str {
    match status {
        SessionManagerStatus::Idle => "idle",
        SessionManagerStatus::NeedsInput => "needs input",
        SessionManagerStatus::Working => "working",
        SessionManagerStatus::ReadyForReview => "ready for review",
        SessionManagerStatus::Completed => "completed",
        SessionManagerStatus::Failed => "failed",
        SessionManagerStatus::Stopped => "stopped",
    }
}

fn thread_status_label(status: zeta_protocol::ThreadStatus) -> &'static str {
    match status {
        zeta_protocol::ThreadStatus::Active => "active",
        zeta_protocol::ThreadStatus::Archived => "archived",
    }
}

pub(crate) struct SessionManagerView<'a> {
    sessions: &'a [Session],
    selected: Option<&'a ManagerSelection>,
    focused: bool,
    pinned: &'a BTreeSet<SessionId>,
    collapsed: &'a BTreeSet<SessionGroup>,
    animation_frame: usize,
    now_unix_ms: u64,
}

pub(crate) fn draw_manager(
    frame: &mut Frame<'_>,
    area: Rect,
    view: SessionManagerView<'_>,
    context: RenderContext<'_>,
) {
    if area.is_empty() {
        return;
    }
    let rows = manager_rows(view.sessions, view.pinned, view.collapsed);
    if rows.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::styled(
                "No sessions yet",
                Style::default().fg(context.muted()),
            )),
            area,
        );
        return;
    }
    let visible_rows = usize::from(area.height);
    let selected_row = rows.iter().position(|row| {
        view.selected
            .is_some_and(|selected| row.selection() == *selected)
    });
    let viewport = manager_viewport(rows.len(), selected_row, visible_rows);
    let mut lines = Vec::with_capacity(visible_rows);
    if viewport.start > 0 {
        lines.push(more_line('↑', viewport.start, usize::from(area.width)));
    }
    lines.extend(
        rows[viewport.start..viewport.end]
            .iter()
            .copied()
            .map(|row| match row {
                ManagerRow::Heading { group, count } => group_line(
                    group,
                    count,
                    view.collapsed.contains(&group),
                    view.selected == Some(&ManagerSelection::Group(group)),
                    view.focused,
                    usize::from(area.width),
                ),
                ManagerRow::Session(session) => session_line(
                    session,
                    view.selected == Some(&ManagerSelection::Session(session.session_id.clone())),
                    view.focused,
                    view.animation_frame,
                    view.now_unix_ms,
                    usize::from(area.width),
                ),
            }),
    );
    if viewport.end < rows.len() {
        lines.push(more_line(
            '↓',
            rows.len() - viewport.end,
            usize::from(area.width),
        ));
    }
    frame.render_widget(Paragraph::new(lines), area);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ManagerViewport {
    start: usize,
    end: usize,
}

fn manager_viewport(
    row_count: usize,
    selected_row: Option<usize>,
    visible_rows: usize,
) -> ManagerViewport {
    if row_count <= visible_rows {
        return ManagerViewport {
            start: 0,
            end: row_count,
        };
    }
    if visible_rows <= 1 {
        let start = selected_row
            .unwrap_or_default()
            .min(row_count.saturating_sub(1));
        return ManagerViewport {
            start,
            end: (start + 1).min(row_count),
        };
    }
    let selected = selected_row
        .unwrap_or_default()
        .min(row_count.saturating_sub(1));
    for start in 0..=selected {
        let top_notice = usize::from(start > 0);
        let mut capacity = visible_rows.saturating_sub(top_notice).max(1);
        if row_count.saturating_sub(start) > capacity && capacity > 1 {
            capacity -= 1;
        }
        let end = start.saturating_add(capacity).min(row_count);
        if selected < end {
            return ManagerViewport { start, end };
        }
    }
    ManagerViewport {
        start: selected,
        end: (selected + 1).min(row_count),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ManagerSelection {
    Group(SessionGroup),
    Session(SessionId),
}

#[derive(Clone, Copy)]
enum ManagerRow<'a> {
    Heading { group: SessionGroup, count: usize },
    Session(&'a Session),
}

impl ManagerRow<'_> {
    fn selection(&self) -> ManagerSelection {
        match self {
            Self::Heading { group, .. } => ManagerSelection::Group(*group),
            Self::Session(session) => ManagerSelection::Session(session.session_id.clone()),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum SessionGroup {
    Pinned,
    NeedsInput,
    Working,
    ReadyForReview,
    Failed,
    Stopped,
    Completed,
    Idle,
}

impl SessionGroup {
    const ALL: [Self; 8] = [
        Self::Pinned,
        Self::NeedsInput,
        Self::Working,
        Self::ReadyForReview,
        Self::Failed,
        Self::Stopped,
        Self::Completed,
        Self::Idle,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::Pinned => "Pinned",
            Self::NeedsInput => "Needs input",
            Self::Working => "Working",
            Self::ReadyForReview => "Ready for review",
            Self::Failed => "Failed",
            Self::Stopped => "Stopped",
            Self::Completed => "Completed",
            Self::Idle => "Idle",
        }
    }

    fn includes(self, session: &Session, pinned: &BTreeSet<SessionId>) -> bool {
        let is_pinned = pinned.contains(&session.session_id);
        match self {
            Self::Pinned => is_pinned,
            Self::NeedsInput => {
                !is_pinned && session.manager.status == SessionManagerStatus::NeedsInput
            }
            Self::Working => !is_pinned && session.manager.status == SessionManagerStatus::Working,
            Self::ReadyForReview => {
                !is_pinned && session.manager.status == SessionManagerStatus::ReadyForReview
            }
            Self::Failed => !is_pinned && session.manager.status == SessionManagerStatus::Failed,
            Self::Stopped => !is_pinned && session.manager.status == SessionManagerStatus::Stopped,
            Self::Completed => {
                !is_pinned && session.manager.status == SessionManagerStatus::Completed
            }
            Self::Idle => !is_pinned && session.manager.status == SessionManagerStatus::Idle,
        }
    }
}

fn manager_rows<'a>(
    sessions: &'a [Session],
    pinned: &BTreeSet<SessionId>,
    collapsed: &BTreeSet<SessionGroup>,
) -> Vec<ManagerRow<'a>> {
    let mut rows = Vec::new();
    for group in SessionGroup::ALL {
        let group_sessions = sessions
            .iter()
            .filter(|session| group.includes(session, pinned))
            .collect::<Vec<_>>();
        if group_sessions.is_empty() {
            continue;
        }
        rows.push(ManagerRow::Heading {
            group,
            count: group_sessions.len(),
        });
        if !collapsed.contains(&group) {
            rows.extend(group_sessions.into_iter().map(ManagerRow::Session));
        }
    }
    rows
}

fn session_line<'a>(
    session: &'a Session,
    selected: bool,
    focused: bool,
    animation_frame: usize,
    now_unix_ms: u64,
    width: usize,
) -> Line<'a> {
    let icon = status_icon(session.manager.status, animation_frame);
    let elapsed = elapsed_label(session, now_unix_ms);
    let elapsed_width = elapsed.width();
    let icon_width = icon.width().unwrap_or(1);
    let indent = 2;
    let after_icon = width.saturating_sub(indent + icon_width + 1);
    let time_gap = usize::from(after_icon > elapsed_width);
    let body_width = after_icon.saturating_sub(elapsed_width + time_gap);
    let middle = activity_text(session);
    let (name_width, middle_gap, middle_width) = column_widths(body_width, !middle.is_empty());
    let name = pad_to_width(&truncate_to_width(&session.title, name_width), name_width);
    let middle = pad_to_width(&truncate_to_width(middle, middle_width), middle_width);
    let row_style = if selected && focused {
        selected_style()
    } else {
        Style::default().fg(Color::Gray)
    };

    Line::from(vec![
        Span::styled("  ", row_style),
        Span::styled(icon.to_string(), row_style),
        Span::raw(" "),
        Span::styled(name, row_style),
        Span::raw(" ".repeat(middle_gap)),
        Span::styled(middle, row_style),
        Span::raw(" ".repeat(time_gap)),
        Span::styled(elapsed, row_style),
    ])
    .style(row_style)
}

fn group_line(
    group: SessionGroup,
    count: usize,
    collapsed: bool,
    selected: bool,
    focused: bool,
    width: usize,
) -> Line<'static> {
    let arrow = if collapsed { '▸' } else { '▾' };
    let text = format!("{arrow} {} ({count})", group.label());
    let style = if selected && focused {
        selected_style().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(Color::Gray)
            .add_modifier(Modifier::BOLD)
    };
    Line::styled(pad_to_width(&truncate_to_width(&text, width), width), style)
}

fn more_line(direction: char, count: usize, width: usize) -> Line<'static> {
    let position = if direction == '↑' { "above" } else { "below" };
    let text = format!("{direction} {count} more {position}");
    Line::styled(
        pad_to_width(&truncate_to_width(&text, width), width),
        Style::default()
            .fg(Color::Gray)
            .add_modifier(Modifier::ITALIC),
    )
}

fn selected_style() -> Style {
    Style::default().fg(Color::Black).bg(Color::Gray)
}

fn column_widths(body_width: usize, has_middle: bool) -> (usize, usize, usize) {
    if !has_middle || body_width < 12 {
        return (body_width, 0, 0);
    }
    let name_width = (body_width / 3).clamp(8, 28).min(body_width);
    let middle_gap = usize::from(body_width > name_width);
    let middle_width = body_width.saturating_sub(name_width + middle_gap);
    (name_width, middle_gap, middle_width)
}

fn activity_text(session: &Session) -> &str {
    match &session.manager.activity {
        Some(SessionManagerActivity::Operation { text })
        | Some(SessionManagerActivity::Question { text })
        | Some(SessionManagerActivity::Failure { text }) => text,
        None => session.manager.summary.as_deref().unwrap_or_default(),
    }
}

fn elapsed_label(session: &Session, now_unix_ms: u64) -> String {
    if session.manager.status_changed_at_unix_ms == 0 {
        return "—".into();
    }
    let elapsed = format_compact_duration(Duration::from_millis(
        now_unix_ms.saturating_sub(session.manager.status_changed_at_unix_ms),
    ));
    if session.manager.status == SessionManagerStatus::Completed {
        format!("{elapsed} ago")
    } else {
        elapsed
    }
}

fn status_icon(status: SessionManagerStatus, animation_frame: usize) -> char {
    match status {
        SessionManagerStatus::Idle => '○',
        SessionManagerStatus::NeedsInput => '?',
        SessionManagerStatus::Working => WORKING_FRAMES[animation_frame % WORKING_FRAMES.len()],
        SessionManagerStatus::ReadyForReview => '◆',
        SessionManagerStatus::Completed | SessionManagerStatus::Failed => '●',
        SessionManagerStatus::Stopped => '■',
    }
}

fn truncate_to_width(text: &str, width: usize) -> String {
    text.chars()
        .scan(0, |used, character| {
            let character_width = character.width().unwrap_or(0);
            (*used + character_width <= width).then(|| {
                *used += character_width;
                character
            })
        })
        .collect()
}

fn pad_to_width(text: &str, width: usize) -> String {
    format!("{text}{}", " ".repeat(width.saturating_sub(text.width())))
}

fn current_unix_millis() -> u64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be after the Unix epoch")
        .as_millis();
    u64::try_from(millis).expect("Unix millisecond timestamp must fit u64")
}

#[cfg(test)]
#[path = "manager_tests.rs"]
mod tests;
