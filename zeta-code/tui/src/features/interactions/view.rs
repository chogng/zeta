use crate::components::pane::PaneViewModel;
use crate::components::selection::SelectionDismissal;
use crate::components::selection::SelectionItem;
use crate::components::selection::SelectionItemId;
use crate::components::selection::SelectionPreview;
use crate::components::selection::SelectionTab;
use crate::components::selection::SelectionViewModel;
use ratatui::text::Line;
use std::collections::BTreeMap;
use zeta_protocol::ActionApprovalCapability;
use zeta_protocol::ActionApprovalCapabilityKind;
use zeta_protocol::ActionApprovalDecision;
use zeta_protocol::ActionApprovalResponse;
use zeta_protocol::AgentRequest;
use zeta_protocol::AgentRequestEnvelope;
use zeta_protocol::AgentResponse;
use zeta_protocol::RequestId;
use zeta_protocol::RequestUserInput;
use zeta_protocol::RequestUserInputResponse;
use zeta_protocol::TurnId;
use zeta_protocol::UserInputAnswer;
use zeta_protocol::UserInputQuestion;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct InteractionResponse {
    pub(crate) turn_id: TurnId,
    pub(crate) request_id: RequestId,
    pub(crate) response: AgentResponse,
}

pub(crate) struct InteractionSelectionView {
    pub(crate) model: PaneViewModel<SelectionViewModel>,
    pub(crate) state: InteractionSelectionState,
}

#[derive(Debug)]
pub(crate) struct InteractionSelectionState {
    turn_id: TurnId,
    request_id: RequestId,
    stage: InteractionStage,
}

#[derive(Debug)]
enum InteractionStage {
    Approval(BTreeMap<SelectionItemId, AgentResponse>),
    UserInput(UserInputStage),
}

#[derive(Debug)]
struct UserInputStage {
    questions: Vec<UserInputQuestion>,
    current: usize,
    answers: BTreeMap<String, UserInputAnswer>,
    actions: BTreeMap<SelectionItemId, String>,
    free_form_action: Option<SelectionItemId>,
}

pub(crate) enum InteractionSelectionOutcome {
    Continue(PaneViewModel<SelectionViewModel>),
    Resolve(InteractionResponse),
}

pub(crate) fn interaction_selection_view(
    envelope: AgentRequestEnvelope,
) -> Result<InteractionSelectionView, String> {
    match envelope.interaction.request {
        AgentRequest::Approval { request } => Ok(approval_view(
            envelope.turn_id,
            envelope.interaction.request_id,
            request,
        )),
        AgentRequest::UserInput { request } => Ok(user_input_view(
            envelope.turn_id,
            envelope.interaction.request_id,
            request,
        )),
        AgentRequest::DynamicTool { .. } => {
            Err("dynamic Tool interaction is not supported by this TUI".into())
        }
    }
}

fn approval_view(
    turn_id: TurnId,
    request_id: RequestId,
    request: zeta_protocol::ActionApprovalRequest,
) -> InteractionSelectionView {
    let preview = approval_preview(&request.reason, &request.capabilities);
    let approve_id = SelectionItemId::new("interaction-approve-once");
    let decline_id = SelectionItemId::new("interaction-decline");
    let mut actions = BTreeMap::new();
    actions.insert(
        approve_id.clone(),
        approval_agent_response(ActionApprovalDecision::ApproveOnce),
    );
    actions.insert(
        decline_id.clone(),
        approval_agent_response(ActionApprovalDecision::Decline),
    );
    let items = vec![
        SelectionItem::new("Approve once")
            .with_id(approve_id)
            .with_description("authorize only this exact reviewed action")
            .with_preview(preview.clone()),
        SelectionItem::new("Decline")
            .with_id(decline_id)
            .with_description("deny this action and continue the Turn")
            .with_preview(preview),
    ];
    InteractionSelectionView {
        model: PaneViewModel::new(
            SelectionViewModel::new(
                "Approval required",
                vec![SelectionTab::new("Decision", items)],
            )
            .without_tab_bar()
            .with_dismissal(SelectionDismissal::Blocked),
            "↑/↓ select  ·  Enter respond  ·  Ctrl-C interrupt Turn",
        ),
        state: InteractionSelectionState {
            turn_id,
            request_id,
            stage: InteractionStage::Approval(actions),
        },
    }
}

fn approval_agent_response(decision: ActionApprovalDecision) -> AgentResponse {
    AgentResponse::Approval {
        response: ActionApprovalResponse { decision },
    }
}

fn user_input_view(
    turn_id: TurnId,
    request_id: RequestId,
    request: RequestUserInput,
) -> InteractionSelectionView {
    let mut stage = UserInputStage {
        questions: request.questions,
        current: 0,
        answers: BTreeMap::new(),
        actions: BTreeMap::new(),
        free_form_action: None,
    };
    let model = user_input_model(&mut stage);
    InteractionSelectionView {
        model,
        state: InteractionSelectionState {
            turn_id,
            request_id,
            stage: InteractionStage::UserInput(stage),
        },
    }
}

impl InteractionSelectionState {
    pub(crate) fn activate_item(
        &mut self,
        item_id: &SelectionItemId,
    ) -> Option<InteractionSelectionOutcome> {
        match &mut self.stage {
            InteractionStage::Approval(actions) => {
                let response = actions.get(item_id)?.clone();
                Some(InteractionSelectionOutcome::Resolve(
                    self.response(response),
                ))
            }
            InteractionStage::UserInput(stage) => {
                let value = stage.actions.get(item_id)?.clone();
                Some(advance_user_input(
                    &self.turn_id,
                    &self.request_id,
                    stage,
                    value,
                ))
            }
        }
    }

    pub(crate) fn activate_free_form(
        &mut self,
        item_id: &SelectionItemId,
        value: String,
    ) -> Option<InteractionSelectionOutcome> {
        let InteractionStage::UserInput(stage) = &mut self.stage else {
            return None;
        };
        if stage.free_form_action.as_ref() != Some(item_id) {
            return None;
        }
        Some(advance_user_input(
            &self.turn_id,
            &self.request_id,
            stage,
            value,
        ))
    }

    fn response(&self, response: AgentResponse) -> InteractionResponse {
        InteractionResponse {
            turn_id: self.turn_id.clone(),
            request_id: self.request_id.clone(),
            response,
        }
    }
}

fn advance_user_input(
    turn_id: &TurnId,
    request_id: &RequestId,
    stage: &mut UserInputStage,
    value: String,
) -> InteractionSelectionOutcome {
    if let Some(question) = stage.questions.get(stage.current) {
        stage
            .answers
            .insert(question.id.clone(), UserInputAnswer { value });
        stage.current += 1;
    }
    if stage.current >= stage.questions.len() {
        return InteractionSelectionOutcome::Resolve(InteractionResponse {
            turn_id: turn_id.clone(),
            request_id: request_id.clone(),
            response: AgentResponse::UserInput {
                response: RequestUserInputResponse {
                    answers: std::mem::take(&mut stage.answers),
                },
            },
        });
    }
    InteractionSelectionOutcome::Continue(user_input_model(stage))
}

fn user_input_model(stage: &mut UserInputStage) -> PaneViewModel<SelectionViewModel> {
    stage.actions.clear();
    stage.free_form_action = None;
    let Some(question) = stage.questions.get(stage.current) else {
        let continue_id = SelectionItemId::new("interaction-user-input-complete");
        stage.actions.insert(continue_id.clone(), String::new());
        return PaneViewModel::new(
            SelectionViewModel::new(
                "Input requested",
                vec![SelectionTab::new(
                    "Response",
                    vec![SelectionItem::new("Continue").with_id(continue_id)],
                )],
            )
            .without_tab_bar()
            .with_dismissal(SelectionDismissal::Blocked),
            "Enter respond  ·  Ctrl-C interrupt Turn",
        );
    };
    let preview = SelectionPreview::new("Question", vec![Line::from(question.question.clone())])
        .with_margins(1, 0);
    let mut items = question
        .options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            let item_id =
                SelectionItemId::new(format!("interaction-user-input-{}-{index}", stage.current));
            stage.actions.insert(item_id.clone(), option.label.clone());
            SelectionItem::new(&option.label)
                .with_id(item_id)
                .with_description(&option.description)
                .with_preview(preview.clone())
        })
        .collect::<Vec<_>>();
    if items.is_empty() && !question.allow_free_form {
        let item_id =
            SelectionItemId::new(format!("interaction-user-input-{}-empty", stage.current));
        stage.actions.insert(item_id.clone(), String::new());
        items.push(
            SelectionItem::new("Continue with an empty answer")
                .with_id(item_id)
                .with_preview(preview.clone()),
        );
    }
    let mut model = SelectionViewModel::new(
        format!(
            "{}  ({}/{})",
            question.header,
            stage.current + 1,
            stage.questions.len()
        ),
        vec![SelectionTab::new("Answer", items)],
    )
    .without_tab_bar()
    .with_dismissal(SelectionDismissal::Blocked);
    let key_hints = if question.allow_free_form {
        let item_id = SelectionItemId::new(format!(
            "interaction-user-input-{}-free-form",
            stage.current
        ));
        stage.free_form_action = Some(item_id.clone());
        model = model.with_free_form("Type another answer", item_id);
        "Enter choose  ·  Ctrl-Enter use typed answer  ·  Ctrl-C interrupt Turn"
    } else {
        "↑/↓ select  ·  Enter respond  ·  Ctrl-C interrupt Turn"
    };
    PaneViewModel::new(model, key_hints)
}

fn approval_preview(reason: &str, capabilities: &[ActionApprovalCapability]) -> SelectionPreview {
    let mut lines = vec![Line::from(reason.to_owned())];
    lines.extend(capabilities.iter().map(|capability| {
        Line::from(format!(
            "{}  ·  {}",
            capability_kind(capability.kind),
            capability.scope
        ))
    }));
    SelectionPreview::new("Requested capability", lines).with_margins(1, 0)
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

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
