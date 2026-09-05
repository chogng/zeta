use super::Approval;
use super::ApprovalDecision;
use super::ApprovalOutcome;
use super::ApprovalSpec;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;

#[test]
fn approval_uses_fixed_choices_and_blocks_duplicate_submission() {
    let mut approval = approval();
    approval.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));

    assert_eq!(
        approval.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        ApprovalOutcome::Respond(ApprovalDecision::Decline)
    );
    assert_eq!(
        approval.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        ApprovalOutcome::Consumed
    );
}

#[test]
fn submission_failure_restores_the_approval_and_exposes_the_error() {
    let mut approval = approval();
    approval.activate(0);

    approval.submission_failed("request failed".into());

    assert!(!approval.view().submitting);
    assert_eq!(approval.view().error, Some("request failed"));
    assert_eq!(
        approval.activate(1),
        Some(ApprovalOutcome::Respond(ApprovalDecision::Decline))
    );
}

#[test]
fn pointer_activation_responds_to_its_target_without_moving_the_keyboard_choice() {
    let mut approval = approval();

    assert_eq!(
        approval.activate(1),
        Some(ApprovalOutcome::Respond(ApprovalDecision::Decline))
    );
    assert_eq!(approval.view().selected, ApprovalDecision::ApproveOnce);
}

fn approval() -> Approval {
    Approval::new(ApprovalSpec {
        title: "Approval required".into(),
        reason: "Run command?".into(),
        details: vec!["Process spawn  ·  cargo test".into()],
    })
}

#[test]
fn navigation_repeats_never_submit_or_wrap_the_approval_choice() {
    let mut approval = approval();
    for _ in 0..3 {
        assert_eq!(
            approval.handle_key(KeyEvent::new_with_kind(
                KeyCode::Char('j'),
                KeyModifiers::NONE,
                crossterm::event::KeyEventKind::Repeat
            )),
            ApprovalOutcome::Consumed
        );
    }
    assert_eq!(approval.view().selected, ApprovalDecision::Decline);
    assert_eq!(
        approval.handle_key(KeyEvent::new_with_kind(
            KeyCode::Enter,
            KeyModifiers::NONE,
            crossterm::event::KeyEventKind::Repeat
        )),
        ApprovalOutcome::Consumed
    );
    assert!(!approval.view().submitting);
    approval.handle_key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE));
    assert_eq!(
        approval.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
        ApprovalOutcome::Consumed
    );
    assert_eq!(approval.view().selected, ApprovalDecision::ApproveOnce);
}
