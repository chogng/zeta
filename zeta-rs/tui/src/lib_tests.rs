use super::app::App;
use super::app::AppEvent;
use super::app::Status;
use crate::app::apply_active_turn_snapshot;
use crate::app::slash_command_registry;
use crate::components::transcript::MessageRole;
use crate::features::thread::present_turn_error;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use zeta_app_server_protocol::protocol::slash_commands::{
    SlashCommandArgumentModeDto, SlashCommandDefinition,
};
use zeta_protocol::ItemId;
use zeta_protocol::SessionId;
use zeta_protocol::StableTurnError;
use zeta_protocol::Thread;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadItem;
use zeta_protocol::ThreadStatus;
use zeta_protocol::Turn;
use zeta_protocol::TurnId;
use zeta_protocol::TurnStatus;

#[test]
fn completed_active_turn_only_updates_lifecycle_after_snapshot_mapping() {
    let turn_id = turn_id();
    let mut active_turn = Some(turn_id.clone());
    let mut app = working_app();
    let turn = Turn {
        turn_id: turn_id.clone(),
        status: TurnStatus::Completed,
        items: vec![
            ThreadItem::UserMessage {
                item_id: ItemId::new("item_1").unwrap(),
                turn_id: turn_id.clone(),
                text: "prompt".into(),
            },
            ThreadItem::AgentMessage {
                item_id: ItemId::new("item_2").unwrap(),
                turn_id,
                text: "complete response".into(),
            },
        ],
        pending_interaction: None,
        error: None,
    };
    app.update(AppEvent::ThreadSnapshotReceived(Thread {
        session_id: SessionId::new("session_1").unwrap(),
        thread_id: ThreadId::new("thread_1").unwrap(),
        title: "Thread".into(),
        status: ThreadStatus::Active,
        sequence: 3,
        turns: vec![turn.clone()],
    }));

    apply_active_turn_snapshot(&mut app, &mut active_turn, &[turn]);

    assert_eq!(active_turn, None);
    assert_eq!(app.status(), &Status::Ready);
    assert_eq!(app.messages().len(), 2);
    assert_eq!(app.messages().last().unwrap().role, MessageRole::Agent);
    assert_eq!(app.messages().last().unwrap().text, "complete response");
}

#[test]
fn waiting_active_turn_remains_interruptible() {
    let turn_id = turn_id();
    let mut active_turn = Some(turn_id.clone());
    let mut app = working_app();
    let turn = Turn {
        turn_id,
        status: TurnStatus::WaitingForUserInput,
        items: Vec::new(),
        pending_interaction: None,
        error: None,
    };

    apply_active_turn_snapshot(&mut app, &mut active_turn, &[turn]);

    assert!(active_turn.is_some());
    assert_eq!(app.status(), &Status::WaitingForUserInput);
    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
        Some(super::app::AppCommand::Interrupt)
    );
}

#[test]
fn resumed_active_turn_returns_from_waiting_to_working() {
    let turn_id = turn_id();
    let mut active_turn = Some(turn_id.clone());
    let mut app = working_app();
    let waiting_turn = Turn {
        turn_id: turn_id.clone(),
        status: TurnStatus::WaitingForUserInput,
        items: Vec::new(),
        pending_interaction: None,
        error: None,
    };
    apply_active_turn_snapshot(&mut app, &mut active_turn, &[waiting_turn]);

    let resumed_turn = Turn {
        turn_id,
        status: TurnStatus::Running,
        items: Vec::new(),
        pending_interaction: None,
        error: None,
    };
    apply_active_turn_snapshot(&mut app, &mut active_turn, &[resumed_turn]);

    assert_eq!(app.status(), &Status::Working);
}

#[test]
fn failed_turn_uses_a_friendly_error_instead_of_debug_output() {
    let turn_id = turn_id();
    let mut active_turn = Some(turn_id.clone());
    let mut app = working_app();
    let turn = Turn {
        turn_id,
        status: TurnStatus::Failed,
        items: Vec::new(),
        pending_interaction: None,
        error: Some(StableTurnError::model_invocation_failed()),
    };

    apply_active_turn_snapshot(&mut app, &mut active_turn, &[turn]);

    assert_eq!(app.status(), &Status::Error);
    let message = &app.messages().last().unwrap().text;
    assert!(message.contains("configured model"));
    assert!(!message.contains("StableTurnError"));
    assert!(!message.contains("ModelInvocationFailed"));
}

#[test]
fn persistence_failure_explains_that_the_response_was_not_saved() {
    assert_eq!(
        present_turn_error(&StableTurnError::completion_persistence_failed()),
        "Zeta generated a response but couldn't save it. Please try again."
    );
}

#[test]
fn server_slash_commands_become_the_tui_runtime_registry() {
    let registry = slash_command_registry(&[SlashCommandDefinition {
        name: "diagnose".into(),
        description: "inspect the current workspace".into(),
        argument_mode: SlashCommandArgumentModeDto::Optional,
    }])
    .unwrap();

    assert!(registry.command_named("diagnose").is_some());
}

#[test]
fn server_slash_commands_cannot_shadow_local_builtins() {
    let error = slash_command_registry(&[SlashCommandDefinition {
        name: "quit".into(),
        description: "replace local quit".into(),
        argument_mode: SlashCommandArgumentModeDto::None,
    }])
    .unwrap_err();

    assert!(matches!(
        error,
        zeta_app_server_client::ClientError::Protocol(message)
            if message.contains("duplicate slash command name 'quit'")
    ));
}

fn working_app() -> App {
    let mut app = App::new();
    app.insert_text("prompt");
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app
}

fn turn_id() -> TurnId {
    TurnId::new("turn_1").unwrap()
}
