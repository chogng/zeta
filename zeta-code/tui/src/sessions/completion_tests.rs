use super::Command;
use super::CommandRequest;
use super::prepare_command;
use crate::thread::composer::ChatInputItem;
use crate::thread::composer::ChatSubmission;
use zeta_protocol::ApprovalMode;
use zeta_protocol::SessionId;
use zeta_protocol::ThreadId;

#[test]
fn resume_request_preserves_the_selected_session_and_thread() {
    let thread_id = ThreadId::new("thread-2").unwrap();
    let request = prepare_command(
        ApprovalMode::AskPermissions,
        Command::Resume {
            session_id: "session-1".into(),
            preferred_thread_id: Some(thread_id.clone()),
        },
    );

    assert!(matches!(
        request,
        CommandRequest::Resume {
            session_id,
            preferred_thread_id: Some(preferred_thread_id),
        } if session_id == "session-1" && preferred_thread_id == thread_id
    ));
}

#[test]
fn archive_request_preserves_all_selected_sessions() {
    let first = SessionId::new("session-1").unwrap();
    let second = SessionId::new("session-2").unwrap();
    let request = prepare_command(
        ApprovalMode::AskPermissions,
        Command::Archive {
            session_ids: vec![first.clone(), second.clone()],
        },
    );

    assert!(matches!(
        request,
        CommandRequest::Archive { session_ids }
            if session_ids == vec![first, second]
    ));
}

#[test]
fn manager_request_preserves_submission_and_approval_mode() {
    let submission = ChatSubmission {
        display_text: "investigate the failure".into(),
        input: vec![ChatInputItem::Text("investigate the failure".into())],
    };
    let request = prepare_command(
        ApprovalMode::BypassPermissions,
        Command::CreateAndEnter {
            submission: submission.clone(),
        },
    );

    assert!(matches!(
        request,
        CommandRequest::CreateAndEnter {
            submission: prepared,
            approval_mode: ApprovalMode::BypassPermissions,
        } if prepared == submission
    ));
}

#[test]
fn switch_request_preserves_the_selected_thread() {
    let thread_id = ThreadId::new("thread-2").unwrap();
    let request = prepare_command(
        ApprovalMode::AskPermissions,
        Command::SwitchThread {
            thread_id: thread_id.clone(),
        },
    );

    assert!(matches!(
        request,
        CommandRequest::SwitchThread { thread_id: prepared } if prepared == thread_id
    ));
}
