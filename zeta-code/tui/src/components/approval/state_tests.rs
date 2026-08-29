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
    assert!(approval.select(1));
}

fn approval() -> Approval {
    Approval::new(ApprovalSpec {
        title: "Approval required".into(),
        reason: "Run command?".into(),
        details: vec!["Process spawn  ·  cargo test".into()],
    })
}
