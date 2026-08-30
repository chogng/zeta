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
use zeta_utils_elapsed::format_compact_duration;

const ANIMATION_INTERVAL: Duration = Duration::from_millis(80);
const WORKING_FRAMES: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

#[derive(Debug)]
pub(crate) struct SessionManagerState {
    selected: Option<SessionId>,
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
        if self
            .selected
            .as_ref()
            .is_some_and(|selected| sessions.iter().any(|item| &item.session_id == selected))
        {
            return;
        }
        self.selected = ordered_sessions(sessions, &self.pinned)
            .first()
            .map(|session| session.session_id.clone());
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

    pub(crate) fn selected(&self) -> Option<&SessionId> {
        self.selected.as_ref()
    }

    pub(crate) fn toggle_selected_pin(&mut self) -> bool {
        let Some(selected) = self.selected.clone() else {
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
            animation_frame: self.animation_frame,
            now_unix_ms: self.now_unix_ms,
        }
    }

    pub(crate) fn ordered<'a>(&self, sessions: &'a [Session]) -> Vec<&'a Session> {
        ordered_sessions(sessions, &self.pinned)
    }

    fn select_offset(&mut self, sessions: &[Session], delta: isize) -> bool {
        let ordered = ordered_sessions(sessions, &self.pinned);
        let Some(current) = self.selected.as_ref() else {
            self.selected = ordered.first().map(|session| session.session_id.clone());
            return self.selected.is_some();
        };
        let Some(index) = ordered
            .iter()
            .position(|session| &session.session_id == current)
        else {
            self.selected = ordered.first().map(|session| session.session_id.clone());
            return self.selected.is_some();
        };
        let next = index
            .saturating_add_signed(delta)
            .min(ordered.len().saturating_sub(1));
        let changed = next != index;
        self.selected = ordered.get(next).map(|session| session.session_id.clone());
        changed
    }
}

pub(crate) struct SessionManagerView<'a> {
    sessions: &'a [Session],
    selected: Option<&'a SessionId>,
    focused: bool,
    pinned: &'a BTreeSet<SessionId>,
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
    let rows = manager_rows(view.sessions, view.pinned);
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
        matches!(row, ManagerRow::Session(session) if view.selected == Some(&session.session_id))
    });
    let start = visible_start(&rows, selected_row, visible_rows);
    let lines = rows
        .into_iter()
        .skip(start)
        .take(visible_rows)
        .map(|row| match row {
            ManagerRow::Heading(label) => Line::styled(
                label,
                Style::default()
                    .fg(context.muted())
                    .add_modifier(Modifier::BOLD),
            ),
            ManagerRow::Session(session) => session_line(
                session,
                view.selected == Some(&session.session_id),
                view.focused,
                view.animation_frame,
                view.now_unix_ms,
                usize::from(area.width),
                context,
            ),
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(lines), area);
}

fn visible_start(
    rows: &[ManagerRow<'_>],
    selected_row: Option<usize>,
    visible_rows: usize,
) -> usize {
    let mut start = selected_row
        .map(|index| index.saturating_add(1).saturating_sub(visible_rows))
        .unwrap_or_default();
    let previous = start.saturating_sub(1);
    if start > 0
        && matches!(rows.get(start), Some(ManagerRow::Session(_)))
        && matches!(rows.get(previous), Some(ManagerRow::Heading(_)))
        && selected_row.is_some_and(|selected| selected < previous.saturating_add(visible_rows))
    {
        start = previous;
    }
    start
}

#[derive(Clone, Copy)]
enum ManagerRow<'a> {
    Heading(&'static str),
    Session(&'a Session),
}

#[derive(Clone, Copy)]
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

fn manager_rows<'a>(sessions: &'a [Session], pinned: &BTreeSet<SessionId>) -> Vec<ManagerRow<'a>> {
    let mut rows = Vec::new();
    for group in SessionGroup::ALL {
        let group_sessions = sessions
            .iter()
            .filter(|session| group.includes(session, pinned))
            .collect::<Vec<_>>();
        if group_sessions.is_empty() {
            continue;
        }
        rows.push(ManagerRow::Heading(group.label()));
        rows.extend(group_sessions.into_iter().map(ManagerRow::Session));
    }
    rows
}

fn ordered_sessions<'a>(sessions: &'a [Session], pinned: &BTreeSet<SessionId>) -> Vec<&'a Session> {
    SessionGroup::ALL
        .into_iter()
        .flat_map(|group| {
            sessions
                .iter()
                .filter(move |session| group.includes(session, pinned))
        })
        .collect()
}

fn session_line<'a>(
    session: &'a Session,
    selected: bool,
    focused: bool,
    animation_frame: usize,
    now_unix_ms: u64,
    width: usize,
    context: RenderContext<'_>,
) -> Line<'a> {
    let (icon, icon_color) = status_icon(session.manager.status, animation_frame, context);
    let elapsed = elapsed_label(session, now_unix_ms);
    let elapsed_width = elapsed.width();
    let icon_width = icon.width().unwrap_or(1);
    let after_icon = width.saturating_sub(icon_width + 1);
    let time_gap = usize::from(after_icon > elapsed_width);
    let body_width = after_icon.saturating_sub(elapsed_width + time_gap);
    let middle = activity_text(session);
    let (name_width, middle_gap, middle_width) = column_widths(body_width, !middle.is_empty());
    let name = pad_to_width(&truncate_to_width(&session.title, name_width), name_width);
    let middle = pad_to_width(&truncate_to_width(middle, middle_width), middle_width);
    let label_style = if selected && focused {
        Style::default()
            .fg(context.highlight())
            .add_modifier(Modifier::BOLD)
    } else if selected {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    Line::from(vec![
        Span::styled(icon.to_string(), Style::default().fg(icon_color)),
        Span::raw(" "),
        Span::styled(name, label_style),
        Span::raw(" ".repeat(middle_gap)),
        Span::styled(middle, label_style.fg(context.muted())),
        Span::raw(" ".repeat(time_gap)),
        Span::styled(elapsed, Style::default().fg(context.muted())),
    ])
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

fn status_icon(
    status: SessionManagerStatus,
    animation_frame: usize,
    context: RenderContext<'_>,
) -> (char, Color) {
    match status {
        SessionManagerStatus::Idle => ('○', context.muted()),
        SessionManagerStatus::NeedsInput => ('?', context.warning()),
        SessionManagerStatus::Working => (
            WORKING_FRAMES[animation_frame % WORKING_FRAMES.len()],
            context.accent(),
        ),
        SessionManagerStatus::ReadyForReview => ('◆', context.accent()),
        SessionManagerStatus::Completed => ('●', context.success()),
        SessionManagerStatus::Failed => ('●', context.danger()),
        SessionManagerStatus::Stopped => ('■', context.muted()),
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
