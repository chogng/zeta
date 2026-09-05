use super::branch_count_label;
use super::session_size_label;
use crate::render::InteractionState;
use crate::render::InteractionTarget;
use crate::render::RenderContext;
use crate::render::interaction_style;
use crate::render::selection_marker;
use crate::widgets::detail_list::DetailList;
use crate::widgets::detail_list::DetailListRow;
use ratatui::Frame;
use ratatui::layout::Position;
use ratatui::layout::Rect;
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

const ANIMATION_INTERVAL: Duration = Duration::from_millis(80);
const WORKING_FRAMES: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

#[derive(Debug)]
pub(crate) struct SessionManagerState {
    selected: Option<SessionManagerPointerTarget>,
    archived_expanded: bool,
    selected_archived: bool,
    focused: bool,
    pinned: BTreeSet<SessionId>,
    animation_frame: usize,
    last_animation_at: Option<Instant>,
    now_unix_ms: u64,
}

impl Default for SessionManagerState {
    fn default() -> Self {
        Self {
            selected: None,
            archived_expanded: false,
            selected_archived: false,
            focused: false,
            pinned: BTreeSet::new(),
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
        let rows = manager_rows(sessions, &self.pinned, self.archived_expanded);
        if self.selected.as_ref().is_some_and(|selected| {
            rows.iter()
                .any(|row| row.target().as_ref() == Some(selected))
        }) {
            self.update_selected_status(sessions);
            return;
        }
        self.selected = if self.selected_archived {
            Some(SessionManagerPointerTarget::Archived)
        } else {
            rows.into_iter().find_map(|row| row.target())
        };
        self.update_selected_status(sessions);
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

    pub(crate) fn navigate(
        &mut self,
        sessions: &[Session],
        navigation: crate::widgets::navigation::Navigation,
    ) {
        use crate::widgets::navigation::Navigation;
        let delta = match navigation {
            Navigation::Previous => -1,
            Navigation::Next => 1,
            Navigation::PagePrevious => -12,
            Navigation::PageNext => 12,
            Navigation::First => isize::MIN,
            Navigation::Last => isize::MAX,
        };
        self.select_offset(sessions, delta);
    }

    pub(crate) fn selected_session(&self) -> Option<&SessionId> {
        match self.selected.as_ref() {
            Some(SessionManagerPointerTarget::Session(id)) => Some(id),
            _ => None,
        }
    }

    pub(crate) fn details_selected(&self, sessions: &[Session]) -> Option<DetailList> {
        let selected = self.selected_session()?;
        sessions
            .iter()
            .find(|session| &session.session_id == selected)
            .map(|session| session_details(session, self.now_unix_ms))
    }

    pub(crate) fn toggle_archived(&mut self) {
        self.archived_expanded = !self.archived_expanded;
        self.selected = Some(SessionManagerPointerTarget::Archived);
        self.selected_archived = false;
    }

    pub(crate) fn set_archived_expanded(&mut self, expanded: bool) {
        self.archived_expanded = expanded;
    }

    pub(crate) fn archived_selected(&self) -> bool {
        self.selected == Some(SessionManagerPointerTarget::Archived)
    }

    pub(crate) fn selected_is_archived(&self) -> bool {
        self.selected_archived
    }

    fn update_selected_status(&mut self, sessions: &[Session]) {
        self.selected_archived = self.selected_session().is_some_and(|id| {
            sessions.iter().any(|session| {
                &session.session_id == id && session.status == SessionStatus::Archived
            })
        });
    }

    pub(crate) fn selected_archive_ids(&self, sessions: &[Session]) -> Vec<SessionId> {
        let Some(selected) = self.selected_session() else {
            return Vec::new();
        };
        sessions
            .iter()
            .filter(|session| {
                &session.session_id == selected && session.status == SessionStatus::Active
            })
            .map(|session| session.session_id.clone())
            .collect()
    }

    pub(crate) fn selection_hint(&self) -> &'static str {
        if self.archived_selected() {
            if self.archived_expanded {
                "Enter to collapse · Esc to return to input"
            } else {
                "Enter to expand · Esc to return to input"
            }
        } else if self.selected_archived {
            "Enter to restore · Space to preview · Ctrl+X to delete · i to details"
        } else if self.selected.is_some() {
            "Enter to open · Space to preview · Ctrl+X to archive · i to details"
        } else {
            "Esc to return to input"
        }
    }

    pub(crate) fn status_hint(&self) -> &'static str {
        if self.focused {
            self.selection_hint()
        } else {
            "Enter to return"
        }
    }

    pub(crate) fn toggle_selected_pin(&mut self) -> bool {
        if self.selected_archived {
            return false;
        }
        let Some(selected) = self.selected_session().cloned() else {
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
            archived_expanded: self.archived_expanded,
            focused: self.focused,
            pinned: &self.pinned,
            animation_frame: self.animation_frame,
            now_unix_ms: self.now_unix_ms,
        }
    }

    fn select_offset(&mut self, sessions: &[Session], delta: isize) -> bool {
        let selectable = manager_rows(sessions, &self.pinned, self.archived_expanded)
            .into_iter()
            .filter_map(|row| row.target())
            .collect::<Vec<_>>();
        let index = self
            .selected
            .as_ref()
            .and_then(|selected| selectable.iter().position(|row| row == selected));
        let next = index
            .map(|index| {
                index
                    .saturating_add_signed(delta)
                    .min(selectable.len().saturating_sub(1))
            })
            .unwrap_or(0);
        let changed = index != Some(next);
        self.selected = selectable.get(next).cloned();
        self.update_selected_status(sessions);
        changed
    }
}

fn session_details(session: &Session, now_unix_ms: u64) -> DetailList {
    let mut rows = vec![
        DetailListRow::new("Session", session.title.clone()),
        DetailListRow::new("ID", session.session_id.to_string()),
        DetailListRow::new(
            "Status",
            if session.status == SessionStatus::Archived {
                "archived"
            } else {
                manager_status_label(session.manager.status)
            },
        ),
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
    DetailList::new("Session details", rows)
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
    selected: Option<&'a SessionManagerPointerTarget>,
    archived_expanded: bool,
    focused: bool,
    pinned: &'a BTreeSet<SessionId>,
    animation_frame: usize,
    now_unix_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SessionManagerPointerTarget {
    Session(SessionId),
    Archived,
}

pub(crate) fn pointer_target_at(
    area: Rect,
    view: SessionManagerView<'_>,
    column: u16,
    row: u16,
) -> Option<SessionManagerPointerTarget> {
    if !area.contains(Position::new(column, row)) {
        return None;
    }
    let rows = manager_rows(view.sessions, view.pinned, view.archived_expanded);
    let selected_row = rows.iter().position(|row| {
        row.target()
            .as_ref()
            .is_some_and(|target| Some(target) == view.selected)
    });
    let viewport = manager_viewport(rows.len(), selected_row, usize::from(area.height));
    let line = usize::from(row.saturating_sub(area.y));
    let top_notice = usize::from(viewport.start > 0);
    let index = viewport.start.saturating_add(line.checked_sub(top_notice)?);
    (index < viewport.end)
        .then(|| rows[index].target())
        .flatten()
}

pub(crate) fn draw_manager(
    frame: &mut Frame<'_>,
    area: Rect,
    view: SessionManagerView<'_>,
    hovered: Option<&SessionManagerPointerTarget>,
    pressed: Option<&SessionManagerPointerTarget>,
    context: RenderContext<'_>,
) {
    if area.is_empty() {
        return;
    }
    let rows = manager_rows(view.sessions, view.pinned, view.archived_expanded);
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
        row.target()
            .as_ref()
            .is_some_and(|target| Some(target) == view.selected)
    });
    let viewport = manager_viewport(rows.len(), selected_row, visible_rows);
    let mut lines = Vec::with_capacity(visible_rows);
    if viewport.start > 0 {
        lines.push(more_line(
            '↑',
            viewport.start,
            usize::from(area.width),
            context,
        ));
    }
    lines.extend(
        rows[viewport.start..viewport.end]
            .iter()
            .copied()
            .map(|row| match row {
                ManagerRow::Heading { group, count } => {
                    let mut line = group_line(group, count, usize::from(area.width), context);
                    if group == SessionGroup::Archived {
                        let state = manager_state(
                            &SessionManagerPointerTarget::Archived,
                            view.selected,
                            view.focused,
                            hovered,
                            pressed,
                        );
                        line = line.style(
                            Style::default()
                                .fg(context.muted())
                                .add_modifier(Modifier::BOLD)
                                .patch(interaction_style(context, state)),
                        );
                    }
                    line
                }
                ManagerRow::Session(session) => session_line(
                    session,
                    manager_state(
                        &SessionManagerPointerTarget::Session(session.session_id.clone()),
                        view.selected,
                        view.focused,
                        hovered,
                        pressed,
                    ),
                    view.animation_frame,
                    view.now_unix_ms,
                    usize::from(area.width),
                    context,
                ),
            }),
    );
    if viewport.end < rows.len() {
        lines.push(more_line(
            '↓',
            rows.len() - viewport.end,
            usize::from(area.width),
            context,
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

#[derive(Clone, Copy)]
enum ManagerRow<'a> {
    Heading { group: SessionGroup, count: usize },
    Session(&'a Session),
}

impl ManagerRow<'_> {
    fn target(&self) -> Option<SessionManagerPointerTarget> {
        match self {
            Self::Heading {
                group: SessionGroup::Archived,
                ..
            } => Some(SessionManagerPointerTarget::Archived),
            Self::Heading { .. } => None,
            Self::Session(session) => Some(SessionManagerPointerTarget::Session(
                session.session_id.clone(),
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum SessionGroup {
    Archived,
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
    const ALL: [Self; 9] = [
        Self::Pinned,
        Self::NeedsInput,
        Self::Working,
        Self::ReadyForReview,
        Self::Failed,
        Self::Stopped,
        Self::Completed,
        Self::Idle,
        Self::Archived,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::Archived => "Archived",
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
        if session.status == SessionStatus::Archived {
            return self == Self::Archived;
        }
        let is_pinned = pinned.contains(&session.session_id);
        match self {
            Self::Archived => false,
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
    archived_expanded: bool,
) -> Vec<ManagerRow<'a>> {
    let mut rows = Vec::new();
    for group in SessionGroup::ALL {
        let group_sessions = sessions
            .iter()
            .filter(|session| group.includes(session, pinned))
            .collect::<Vec<_>>();
        if group_sessions.is_empty() && group != SessionGroup::Archived {
            continue;
        }
        rows.push(ManagerRow::Heading {
            group,
            count: group_sessions.len(),
        });
        if group != SessionGroup::Archived || archived_expanded {
            rows.extend(group_sessions.into_iter().map(ManagerRow::Session));
        }
    }
    rows
}

fn session_line<'a>(
    session: &'a Session,
    state: InteractionState,
    animation_frame: usize,
    now_unix_ms: u64,
    width: usize,
    context: RenderContext<'_>,
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
    let row_style = if state.selected || state.hovered || state.pressed {
        interaction_style(context, state)
    } else {
        Style::default().fg(context.muted())
    };

    Line::from(vec![
        Span::styled(selection_marker(state.selected), row_style),
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
    width: usize,
    context: RenderContext<'_>,
) -> Line<'static> {
    let text = format!("{} ({count})", group.label());
    let style = Style::default()
        .fg(context.muted())
        .add_modifier(Modifier::BOLD);
    Line::styled(pad_to_width(&truncate_to_width(&text, width), width), style)
}

fn more_line(
    direction: char,
    count: usize,
    width: usize,
    context: RenderContext<'_>,
) -> Line<'static> {
    let position = if direction == '↑' { "above" } else { "below" };
    let text = format!("{direction} {count} more {position}");
    Line::styled(
        pad_to_width(&truncate_to_width(&text, width), width),
        Style::default()
            .fg(context.muted())
            .add_modifier(Modifier::ITALIC),
    )
}

fn manager_state(
    target: &SessionManagerPointerTarget,
    selected: Option<&SessionManagerPointerTarget>,
    focused: bool,
    hovered: Option<&SessionManagerPointerTarget>,
    pressed: Option<&SessionManagerPointerTarget>,
) -> InteractionState {
    InteractionState {
        target: InteractionTarget::Rest,
        selected: focused && selected == Some(target),
        hovered: hovered == Some(target),
        pressed: pressed == Some(target),
    }
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
        return String::new();
    }
    let elapsed = whole_hour_label(Duration::from_millis(
        now_unix_ms.saturating_sub(session.manager.status_changed_at_unix_ms),
    ));
    if elapsed.is_empty() {
        return elapsed;
    }
    if session.manager.status == SessionManagerStatus::Completed {
        format!("{elapsed} ago")
    } else {
        elapsed
    }
}

fn whole_hour_label(duration: Duration) -> String {
    let hours = duration.as_secs() / 3_600;
    if hours == 0 {
        return String::new();
    }
    let days = hours / 24;
    let remaining_hours = hours % 24;
    match (days, remaining_hours) {
        (0, hours) => format!("{hours}h"),
        (days, 0) => format!("{days}d"),
        (days, hours) => format!("{days}d {hours:02}h"),
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
