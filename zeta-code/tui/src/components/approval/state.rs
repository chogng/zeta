use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;

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
    spec: ApprovalSpec,
    selected: ApprovalDecision,
    submitting: bool,
    error: Option<String>,
}

impl Approval {
    pub(crate) fn new(spec: ApprovalSpec) -> Self {
        Self {
            spec,
            selected: ApprovalDecision::ApproveOnce,
            submitting: false,
            error: None,
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

    pub(crate) fn select(&mut self, index: usize) -> bool {
        let Some(decision) = decision_at(index) else {
            return false;
        };
        if self.submitting {
            return false;
        }
        self.selected = decision;
        true
    }

    pub(crate) fn activate(&mut self, index: usize) -> Option<ApprovalOutcome> {
        self.select(index).then(|| {
            self.submitting = true;
            self.error = None;
            ApprovalOutcome::Respond(self.selected)
        })
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

fn decision_at(index: usize) -> Option<ApprovalDecision> {
    match index {
        0 => Some(ApprovalDecision::ApproveOnce),
        1 => Some(ApprovalDecision::Decline),
        _ => None,
    }
}

#[derive(Clone, Copy)]
pub(crate) struct ApprovalView<'a> {
    pub(crate) title: &'a str,
    pub(crate) reason: &'a str,
    pub(crate) details: &'a [String],
    pub(crate) selected: ApprovalDecision,
    pub(crate) submitting: bool,
    pub(crate) error: Option<&'a str>,
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
