use crate::render::RenderContext;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use zeta_protocol::Session;
use zeta_protocol::SessionId;
use zeta_protocol::SessionStatus;

#[derive(Debug, Default)]
pub(crate) struct SessionManagerState {
    selected: Option<SessionId>,
    focused: bool,
}

impl SessionManagerState {
    pub(crate) fn reconcile(&mut self, sessions: &[Session]) {
        if self
            .selected
            .as_ref()
            .is_some_and(|selected| sessions.iter().any(|item| &item.session_id == selected))
        {
            return;
        }
        self.selected = sessions.first().map(|session| session.session_id.clone());
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

    fn select_offset(&mut self, sessions: &[Session], delta: isize) -> bool {
        let Some(current) = self.selected.as_ref() else {
            self.reconcile(sessions);
            return self.selected.is_some();
        };
        let Some(index) = sessions
            .iter()
            .position(|session| &session.session_id == current)
        else {
            self.reconcile(sessions);
            return self.selected.is_some();
        };
        let next = index
            .saturating_add_signed(delta)
            .min(sessions.len().saturating_sub(1));
        let changed = next != index;
        self.selected = sessions.get(next).map(|session| session.session_id.clone());
        changed
    }
}

pub(crate) struct SessionManagerView<'a> {
    pub(crate) sessions: &'a [Session],
    pub(crate) selected: Option<&'a SessionId>,
    pub(crate) focused: bool,
}

pub(crate) fn draw_manager(
    frame: &mut Frame<'_>,
    area: Rect,
    view: SessionManagerView<'_>,
    context: RenderContext<'_>,
) {
    let mut lines = vec![Line::styled(
        "Sessions",
        Style::default().add_modifier(Modifier::BOLD),
    )];
    if view.sessions.is_empty() {
        lines.push(Line::styled(
            "No sessions",
            Style::default().fg(context.muted()),
        ));
    }
    for session in view.sessions {
        let selected = view.selected == Some(&session.session_id);
        let marker = if view.focused && selected { ">" } else { " " };
        let status = match session.status {
            SessionStatus::Active => "active",
            SessionStatus::Archived => "completed",
        };
        let style = if selected {
            Style::default().fg(context.accent())
        } else {
            Style::default()
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{marker} {}", session.title), style),
            Span::styled(format!(" · {status}"), Style::default().fg(context.muted())),
        ]));
    }
    frame.render_widget(Paragraph::new(lines), area);
}
