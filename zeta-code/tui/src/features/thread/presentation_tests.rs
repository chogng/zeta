use super::present_turn_error;
use super::recover_active_turn;
use zeta_protocol::StableTurnError;
use zeta_protocol::ThreadItem;
use zeta_protocol::Turn;
use zeta_protocol::TurnId;
use zeta_protocol::TurnStatus;

#[test]
fn recovery_selects_the_oldest_nonterminal_turn_for_a_serial_queue() {
    let turns = vec![
        turn("completed", TurnStatus::Completed),
        turn("waiting", TurnStatus::WaitingForApproval),
        turn("running", TurnStatus::Running),
    ];

    assert_eq!(recover_active_turn(&turns).unwrap().as_str(), "waiting");
}

#[test]
fn recovery_does_not_reopen_terminal_turns() {
    let turns = vec![
        turn("completed", TurnStatus::Completed),
        turn("failed", TurnStatus::Failed),
        turn("interrupted", TurnStatus::Interrupted),
    ];

    assert_eq!(recover_active_turn(&turns), None);
}

#[test]
fn every_stable_turn_error_has_a_user_facing_message() {
    let errors = [
        StableTurnError::model_invocation_failed(),
        StableTurnError::context_overflow(),
        StableTurnError::provider_auth(),
        StableTurnError::invalid_request(),
        StableTurnError::invalid_response(),
        StableTurnError::completion_persistence_failed(),
        StableTurnError::interaction_deadline_elapsed(),
        StableTurnError::tool_repetition(),
        StableTurnError::usage_limited(),
    ];

    for error in errors {
        let message = present_turn_error(&error);
        assert!(!message.trim().is_empty());
        assert!(!message.contains(&format!("{:?}", error.code)));
    }
}

fn turn(id: &str, status: TurnStatus) -> Turn {
    Turn {
        turn_id: TurnId::new(id).unwrap(),
        status,
        model: None,
        tool_profile: None,
        usage: zeta_protocol::ModelUsageSummary::default(),
        items: Vec::<ThreadItem>::new(),
        plan: None,
        pending_interaction: None,
        error: (status == TurnStatus::Failed).then(StableTurnError::model_invocation_failed),
    }
}
