use super::*;
use crate::components::selection::SelectionInputOutcome;
use crate::components::selection::SelectionViewState;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use zeta_protocol::ActionApprovalRequest;
use zeta_protocol::ItemId;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;
use zeta_protocol::TurnInteraction;
use zeta_protocol::UserInputOption;
use zeta_protocol::UserInputQuestion;

#[test]
fn approval_view_exposes_exact_responses_and_cannot_be_dismissed() {
    let mut view = interaction_selection_view(approval_envelope()).expect("approval view builds");
    let mut state = SelectionViewState::new(view.model.into_body());

    assert_eq!(state.title(), "Approval required");
    assert_eq!(
        state.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
        SelectionInputOutcome::Consumed
    );
    assert_eq!(
        state.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
        SelectionInputOutcome::Unhandled
    );
    let SelectionInputOutcome::Activate(item_id) =
        state.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
    else {
        panic!("Enter should activate the selected approval response");
    };
    assert!(matches!(
        view.state.activate_item(&item_id),
        Some(InteractionSelectionOutcome::Resolve(InteractionResponse {
            response: AgentResponse::Approval {
                response: ActionApprovalResponse {
                    decision: ActionApprovalDecision::ApproveOnce
                }
            },
            ..
        }))
    ));
}

#[test]
fn user_input_view_collects_options_and_free_form_before_resolving() {
    let mut view = interaction_selection_view(user_input_envelope()).expect("input view builds");
    let mut first = SelectionViewState::new(view.model.into_body());
    assert_eq!(first.title(), "Mode  (1/2)");
    let SelectionInputOutcome::Activate(first_item) =
        first.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
    else {
        panic!("first option should be actionable");
    };
    let Some(InteractionSelectionOutcome::Continue(second_model)) =
        view.state.activate_item(&first_item)
    else {
        panic!("first answer should advance the questionnaire");
    };
    let mut second = SelectionViewState::new(second_model.into_body());
    assert_eq!(second.title(), "Details  (2/2)");
    for character in "custom value".chars() {
        second.handle_key(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE));
    }
    let SelectionInputOutcome::ActivateFreeForm { item_id, value } =
        second.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::CONTROL))
    else {
        panic!("Ctrl-Enter should submit the free-form answer");
    };
    let Some(InteractionSelectionOutcome::Resolve(response)) =
        view.state.activate_free_form(&item_id, value)
    else {
        panic!("final answer should resolve the interaction");
    };
    let AgentResponse::UserInput { response } = response.response else {
        panic!("questionnaire should produce a user-input response");
    };
    assert_eq!(response.answers["mode"].value, "Fast");
    assert_eq!(response.answers["details"].value, "custom value");
}

fn approval_envelope() -> AgentRequestEnvelope {
    AgentRequestEnvelope {
        session_id: SessionId::new("session-1").expect("test ID is non-empty"),
        thread_id: ThreadId::new("thread-1").expect("test ID is non-empty"),
        turn_id: TurnId::new("turn-1").expect("test ID is non-empty"),
        interaction: TurnInteraction {
            request_id: RequestId::new("approval-1").expect("test ID is non-empty"),
            item_id: Some(ItemId::new("item-1").expect("test ID is non-empty")),
            request: AgentRequest::Approval {
                request: ActionApprovalRequest {
                    action_digest: "digest".into(),
                    policy_revision: "policy-1".into(),
                    capabilities: vec![ActionApprovalCapability {
                        kind: ActionApprovalCapabilityKind::Network,
                        scope: "api.example.test".into(),
                    }],
                    reason: "connect to the service".into(),
                    sandbox_denial: None,
                },
            },
            deadline: None,
        },
    }
}

fn user_input_envelope() -> AgentRequestEnvelope {
    AgentRequestEnvelope {
        session_id: SessionId::new("session-1").expect("test ID is non-empty"),
        thread_id: ThreadId::new("thread-1").expect("test ID is non-empty"),
        turn_id: TurnId::new("turn-1").expect("test ID is non-empty"),
        interaction: TurnInteraction {
            request_id: RequestId::new("input-1").expect("test ID is non-empty"),
            item_id: None,
            request: AgentRequest::UserInput {
                request: RequestUserInput {
                    questions: vec![
                        UserInputQuestion {
                            id: "mode".into(),
                            header: "Mode".into(),
                            question: "Which mode?".into(),
                            options: vec![UserInputOption {
                                label: "Fast".into(),
                                description: "Prefer speed".into(),
                            }],
                            allow_free_form: false,
                        },
                        UserInputQuestion {
                            id: "details".into(),
                            header: "Details".into(),
                            question: "What else should change?".into(),
                            options: Vec::new(),
                            allow_free_form: true,
                        },
                    ],
                },
            },
            deadline: None,
        },
    }
}
