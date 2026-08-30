use crate::features::thread::ThreadRequestKind;
use crate::features::thread::ThreadRequestResponse;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use zeta_protocol::ActionApprovalCapability;
use zeta_protocol::ActionApprovalCapabilityKind;
use zeta_protocol::ActionApprovalDecision;
use zeta_protocol::ActionApprovalRequest;
use zeta_protocol::ActionApprovalResponse;
use zeta_protocol::AgentResponse;
use zeta_protocol::RequestId;
use zeta_protocol::TurnId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ApprovalDecision {
    ApproveOnce,
    Decline,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ApprovalSpec {
    pub(crate) title: String,
    pub(crate) reason: String,
    pub(crate) details: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ApprovalOutcome {
    Consumed,
    Respond(ApprovalDecision),
    Unhandled,
}

#[derive(Debug)]
pub(crate) struct Approval {
    turn_id: TurnId,
    request_id: RequestId,
    spec: ApprovalSpec,
    selected: ApprovalDecision,
    submitting: bool,
    error: Option<String>,
}

impl Approval {
    pub(crate) fn open(
        turn_id: TurnId,
        request_id: RequestId,
        request: ActionApprovalRequest,
    ) -> Self {
        Self {
            turn_id,
            request_id,
            spec: ApprovalSpec {
                title: "Approval required".into(),
                reason: request.reason,
                details: request.capabilities.iter().map(capability_detail).collect(),
            },
            selected: ApprovalDecision::ApproveOnce,
            submitting: false,
            error: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn new(spec: ApprovalSpec) -> Self {
        Self {
            turn_id: TurnId::new("test-turn").expect("the test Turn ID is valid"),
            request_id: RequestId::new("test-request").expect("the test request ID is valid"),
            spec,
            selected: ApprovalDecision::ApproveOnce,
            submitting: false,
            error: None,
        }
    }

    pub(crate) fn matches_request(&self, turn_id: &TurnId, request_id: &RequestId) -> bool {
        &self.turn_id == turn_id && &self.request_id == request_id
    }

    pub(crate) fn request_id(&self) -> &RequestId {
        &self.request_id
    }

    pub(crate) fn response(&self, decision: ApprovalDecision) -> ThreadRequestResponse {
        let decision = match decision {
            ApprovalDecision::ApproveOnce => ActionApprovalDecision::ApproveOnce,
            ApprovalDecision::Decline => ActionApprovalDecision::Decline,
        };
        ThreadRequestResponse {
            kind: ThreadRequestKind::Approval,
            turn_id: self.turn_id.clone(),
            request_id: self.request_id.clone(),
            response: AgentResponse::Approval {
                response: ActionApprovalResponse { decision },
            },
        }
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> ApprovalOutcome {
        if key.kind != KeyEventKind::Press || self.submitting {
            return ApprovalOutcome::Consumed;
        }
        match key.code {
            KeyCode::Up | KeyCode::Down => {
                self.selected = match self.selected {
                    ApprovalDecision::ApproveOnce => ApprovalDecision::Decline,
                    ApprovalDecision::Decline => ApprovalDecision::ApproveOnce,
                };
                ApprovalOutcome::Consumed
            }
            KeyCode::Enter if key.modifiers.is_empty() => {
                self.submitting = true;
                self.error = None;
                ApprovalOutcome::Respond(self.selected)
            }
            KeyCode::Esc => ApprovalOutcome::Consumed,
            _ => ApprovalOutcome::Unhandled,
        }
    }

    pub(crate) fn activate(&mut self, index: usize) -> Option<ApprovalOutcome> {
        let decision = decision_at(index)?;
        if self.submitting {
            return None;
        }
        self.submitting = true;
        self.error = None;
        Some(ApprovalOutcome::Respond(decision))
    }

    pub(crate) fn submission_failed(&mut self, error: String) {
        self.submitting = false;
        self.error = Some(error);
    }

    pub(crate) fn view(&self) -> ApprovalView<'_> {
        ApprovalView {
            title: &self.spec.title,
            reason: &self.spec.reason,
            details: &self.spec.details,
            selected: self.selected,
            submitting: self.submitting,
            error: self.error.as_deref(),
        }
    }
}

fn capability_detail(capability: &ActionApprovalCapability) -> String {
    format!(
        "{}  ·  {}",
        capability_kind(capability.kind),
        capability.scope
    )
}

fn capability_kind(kind: ActionApprovalCapabilityKind) -> &'static str {
    match kind {
        ActionApprovalCapabilityKind::FileRead => "File read",
        ActionApprovalCapabilityKind::FileWrite => "File write",
        ActionApprovalCapabilityKind::ProcessSpawn => "Process spawn",
        ActionApprovalCapabilityKind::Network => "Network",
        ActionApprovalCapabilityKind::CredentialUse => "Credential use",
        ActionApprovalCapabilityKind::ExternalMutation => "External mutation",
        ActionApprovalCapabilityKind::SystemConfiguration => "System configuration",
        ActionApprovalCapabilityKind::UserInterface => "User interface",
    }
}

fn decision_at(index: usize) -> Option<ApprovalDecision> {
    match index {
        0 => Some(ApprovalDecision::ApproveOnce),
        1 => Some(ApprovalDecision::Decline),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ApprovalView<'a> {
    pub(crate) title: &'a str,
    pub(crate) reason: &'a str,
    pub(crate) details: &'a [String],
    pub(crate) selected: ApprovalDecision,
    pub(crate) submitting: bool,
    pub(crate) error: Option<&'a str>,
}

use crate::render::RenderContext;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;

const MAX_DETAIL_ROWS: usize = 5;

pub(crate) fn desired_height(view: ApprovalView<'_>) -> u16 {
    let content_rows = 4usize
        .saturating_add(view.details.len().min(MAX_DETAIL_ROWS))
        .saturating_add(usize::from(view.error.is_some()));
    u16::try_from(content_rows.saturating_add(2)).unwrap_or(u16::MAX)
}

pub(crate) fn draw(
    frame: &mut Frame<'_>,
    area: Rect,
    view: ApprovalView<'_>,
    hovered: Option<usize>,
    context: RenderContext<'_>,
) {
    let presented = hovered.and_then(decision_at).unwrap_or(view.selected);
    let mut lines = vec![Line::styled(
        view.reason,
        Style::default().add_modifier(Modifier::BOLD),
    )];
    lines.extend(
        view.details
            .iter()
            .take(MAX_DETAIL_ROWS)
            .map(|detail| Line::styled(detail, Style::default().fg(context.muted()))),
    );
    lines.push(choice_line(
        "Approve once",
        presented == ApprovalDecision::ApproveOnce,
        context,
    ));
    lines.push(choice_line(
        "Decline",
        presented == ApprovalDecision::Decline,
        context,
    ));
    if view.submitting {
        lines.push(Line::styled(
            "Submitting…",
            Style::default().fg(context.muted()),
        ));
    } else if let Some(error) = view.error {
        lines.push(Line::styled(
            error,
            Style::default().fg(ratatui::style::Color::Red),
        ));
    }
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .title(view.title)
                .borders(Borders::ALL)
                .style(Style::default().bg(context.background())),
        ),
        area,
    );
}

pub(crate) fn choice_index_at(
    area: Rect,
    view: ApprovalView<'_>,
    column: u16,
    row: u16,
) -> Option<usize> {
    if column <= area.x || column >= area.right().saturating_sub(1) {
        return None;
    }
    let first_choice_row = area
        .y
        .saturating_add(1)
        .saturating_add(1)
        .saturating_add(view.details.len().min(MAX_DETAIL_ROWS) as u16);
    let index = usize::from(row.saturating_sub(first_choice_row));
    (row >= first_choice_row && index < 2).then_some(index)
}

fn choice_line<'a>(label: &'a str, selected: bool, context: RenderContext<'_>) -> Line<'a> {
    let marker = if selected { "› " } else { "  " };
    let style = if selected {
        Style::default()
            .fg(context.highlight())
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    Line::from(vec![
        Span::styled(marker, style),
        Span::styled(label, style),
    ])
}

#[cfg(test)]
#[path = "approval_tests.rs"]
mod tests;
