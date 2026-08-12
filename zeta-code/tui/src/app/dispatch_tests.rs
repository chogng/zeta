use super::ActiveConversation;
use crate::app::help_selection_view;
use crate::app::{App, AppCommand, AppEvent, Status};
use crate::components::composer::{
    ComposerInput, SlashCommandInvocation, TuiSlashCommandAction, built_in_catalog_command,
};
use crate::components::transcript::MessageRole;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};
use zeta_app_server_client::{
    AppServerClient, InProcessClientOptions, InProcessTransport, start_in_process_client,
};
use zeta_app_server_protocol::protocol::common::ClientInfo;
use zeta_app_server_protocol::protocol::config::{ProviderConfigDto, ProviderConfigureParams};
use zeta_app_server_protocol::protocol::skills::SkillEnablementDto;
use zeta_client::ClientError;
use zeta_client::ClientRequest;
use zeta_client::ClientResponse;
use zeta_client::OperationClient;
use zeta_protocol::CommandId;

#[test]
fn help_lists_only_builtins_with_execution_paths() {
    let help =
        crate::components::selection::SelectionViewState::new(help_selection_view().into_body());
    let help = help
        .visible_items()
        .into_iter()
        .map(|item| item.label())
        .collect::<Vec<_>>();

    assert!(help.contains(&"/status"));
    assert!(help.contains(&"/resume"));
    assert!(help.contains(&"/rewind"));
    assert!(help.contains(&"/model"));
    assert!(help.contains(&"/theme"));
    assert!(!help.contains(&"/login"));
    assert!(!help.contains(&"/plugins"));
    assert!(!help.contains(&"/review"));
}

#[test]
fn new_fork_and_resume_change_the_active_typed_conversation() {
    let (mut client, state_root) = client();
    let mut conversation = ActiveConversation::start(&mut client, "original".into()).unwrap();
    let original_session = conversation.session_id().clone();
    let original_thread = conversation.thread_id().clone();
    let mut app = App::new();

    conversation.execute(
        &mut client,
        invocation(TuiSlashCommandAction::Fork, "investigation"),
        &mut app,
    );

    assert_eq!(conversation.session_id(), &original_session);
    assert_ne!(conversation.thread_id(), &original_thread);
    assert_eq!(app.status(), &Status::Ready);

    conversation.execute(
        &mut client,
        invocation(TuiSlashCommandAction::New, "fresh task"),
        &mut app,
    );
    assert_ne!(conversation.session_id(), &original_session);

    conversation.execute(
        &mut client,
        invocation(TuiSlashCommandAction::Resume, original_session.as_str()),
        &mut app,
    );
    assert_eq!(conversation.session_id(), &original_session);
    assert_eq!(app.messages().last().unwrap().role, MessageRole::Notice);
    assert!(
        app.messages()
            .last()
            .unwrap()
            .text
            .starts_with("Resumed session")
    );

    drop(client);
    let _ = fs::remove_dir_all(state_root);
}

#[test]
fn status_config_mcp_skills_and_help_return_real_surfaces() {
    let (mut client, state_root) = client();
    let mut conversation = ActiveConversation::start(&mut client, "commands".into()).unwrap();
    let mut app = App::new();

    conversation.execute(
        &mut client,
        invocation(TuiSlashCommandAction::Status, ""),
        &mut app,
    );
    assert_eq!(app.selection_view().unwrap().title(), "Status");
    app.update(AppEvent::SelectionViewClosed);

    conversation.execute(
        &mut client,
        invocation(TuiSlashCommandAction::Config, ""),
        &mut app,
    );
    assert_eq!(app.selection_view().unwrap().title(), "Config");
    assert!(app.selection_view().unwrap().search().is_some());
    app.update(AppEvent::SelectionViewClosed);

    conversation.execute(
        &mut client,
        invocation(TuiSlashCommandAction::Mcp, ""),
        &mut app,
    );
    assert_eq!(app.selection_view().unwrap().title(), "MCP servers");
    assert!(app.selection_view().unwrap().search().is_some());
    app.update(AppEvent::SelectionViewClosed);
    conversation.execute(
        &mut client,
        invocation(TuiSlashCommandAction::Skills, ""),
        &mut app,
    );
    assert_eq!(app.status(), &Status::Ready);
    let selection = app.selection_view().unwrap();
    assert_eq!(selection.title(), "Skills");
    assert_eq!(
        selection.tabs()[selection.active_tab_index()].label(),
        "All (1)"
    );
    assert_eq!(selection.visible_items()[0].label(), "skill-creator");

    conversation.execute(
        &mut client,
        invocation(TuiSlashCommandAction::Help, ""),
        &mut app,
    );
    assert_eq!(app.status(), &Status::Ready);
    let selection = app.selection_view().unwrap();
    assert_eq!(
        selection.tabs()[selection.active_tab_index()].label(),
        "Commands"
    );

    drop(client);
    let _ = fs::remove_dir_all(state_root);
}

#[test]
fn skills_view_toggles_catalog_entries_by_enablement() {
    let (mut client, state_root) = client();
    let mut conversation = ActiveConversation::start(&mut client, "skills".into()).unwrap();
    let mut app = App::new();

    conversation.execute(
        &mut client,
        invocation(TuiSlashCommandAction::Skills, ""),
        &mut app,
    );

    let all = app.selection_view().unwrap();
    assert_eq!(all.tabs()[all.active_tab_index()].label(), "All (1)");
    assert_eq!(
        all.visible_items()
            .iter()
            .map(|item| item.label())
            .collect::<Vec<_>>(),
        vec!["skill-creator"]
    );
    assert!(
        all.visible_items()[0]
            .description()
            .unwrap()
            .contains("enabled  ·  built-in  ·  builtin:skill-source:zeta-release")
    );

    app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
    let enabled = app.selection_view().unwrap();
    assert_eq!(
        enabled.tabs()[enabled.active_tab_index()].label(),
        "Enabled (1)"
    );
    assert_eq!(enabled.visible_items()[0].label(), "skill-creator");

    app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
    let action = app
        .handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        .unwrap();
    let AppCommand::SetSkillEnablement {
        skill_id,
        enablement,
    } = action
    else {
        panic!("Enter should request a skill enablement change");
    };
    assert_eq!(skill_id.name.as_str(), "skill-creator");
    assert_eq!(enablement, SkillEnablementDto::Disabled);

    let view = crate::features::skills::set_enablement(&mut client, skill_id, enablement).unwrap();
    app.update(AppEvent::SkillsViewReplaced(view));
    app.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
    let disabled = app.selection_view().unwrap();
    assert_eq!(
        disabled.tabs()[disabled.active_tab_index()].label(),
        "Disabled (1)"
    );
    assert_eq!(disabled.visible_items()[0].label(), "skill-creator");

    drop(client);
    let _ = fs::remove_dir_all(state_root);
}

#[test]
fn model_command_updates_and_clears_preferred_model_with_config_revision() {
    let (mut client, state_root) = client();
    let mut conversation = ActiveConversation::start(&mut client, "model".into()).unwrap();
    let revision = client.read_config().unwrap().revision;
    client
        .configure_provider(ProviderConfigureParams {
            command_id: CommandId::new("configure-test-provider").unwrap(),
            expected_revision: revision,
            config: ProviderConfigDto {
                provider: "test".into(),
                base_url: None,
                max_output_tokens: None,
                model_context: Default::default(),
            },
        })
        .unwrap();
    let mut app = App::new();

    conversation.execute(
        &mut client,
        invocation(TuiSlashCommandAction::Model, "test/model-one"),
        &mut app,
    );

    let configured = client.read_config().unwrap();
    let selected = configured.preferred_model.unwrap();
    assert_eq!(selected.provider, "test");
    assert_eq!(selected.model, "model-one");
    assert_eq!(app.status_line().text_for_width(80), "test/model-one · .");

    conversation.execute(
        &mut client,
        invocation(TuiSlashCommandAction::Model, "clear"),
        &mut app,
    );
    assert_eq!(client.read_config().unwrap().preferred_model, None);
    assert_eq!(app.status_line().text_for_width(80), ".");
    assert_eq!(app.status(), &Status::Ready);

    drop(client);
    let _ = fs::remove_dir_all(state_root);
}

#[test]
fn resume_and_model_without_arguments_open_actionable_panes() {
    let (mut client, state_root) = client();
    let mut conversation = ActiveConversation::start(&mut client, "current".into()).unwrap();
    let current_session = conversation.session_id().to_string();
    let mut app = App::new();

    conversation.execute(
        &mut client,
        invocation(TuiSlashCommandAction::Resume, ""),
        &mut app,
    );
    assert_eq!(app.selection_view().unwrap().title(), "Resume session");
    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        Some(AppCommand::ResumeSession {
            session_id: current_session,
        })
    );
    app.update(AppEvent::SelectionViewClosed);

    conversation.execute(
        &mut client,
        invocation(TuiSlashCommandAction::Model, ""),
        &mut app,
    );
    assert_eq!(app.selection_view().unwrap().title(), "Model");
    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        Some(AppCommand::SetPreferredModel {
            preference: "clear".into(),
        })
    );

    drop(client);
    let _ = fs::remove_dir_all(state_root);
}

#[test]
fn rewind_without_arguments_opens_the_checkpoint_pane() {
    let (mut client, state_root) = client();
    let mut conversation = ActiveConversation::start(&mut client, "rewind".into()).unwrap();
    let mut app = App::new();

    conversation.execute(
        &mut client,
        invocation(TuiSlashCommandAction::Rewind, ""),
        &mut app,
    );

    assert_eq!(app.selection_view().unwrap().title(), "Rewind");
    assert!(app.selection_view().unwrap().search().is_some());
    assert!(app.selection_view().unwrap().visible_items().is_empty());

    drop(client);
    let _ = fs::remove_dir_all(state_root);
}

#[test]
fn product_commands_reject_image_arguments_instead_of_silently_dropping_them() {
    let (mut client, state_root) = client();
    let mut conversation = ActiveConversation::start(&mut client, "images".into()).unwrap();
    let mut app = App::new();
    let invocation = SlashCommandInvocation {
        command: built_in_catalog_command(TuiSlashCommandAction::Model),
        origin: zeta_slash_commands::SlashCommandOrigin::Local,
        display_arguments: "[Image #1]".into(),
        arguments: vec![ComposerInput::Image {
            url: "data:image/png;base64,cG5n".into(),
        }],
    };

    conversation.execute(&mut client, invocation, &mut app);

    assert_eq!(app.status(), &Status::Error);
    assert_eq!(app.messages().last().unwrap().role, MessageRole::Error);
    assert!(
        app.messages()
            .last()
            .unwrap()
            .text
            .contains("do not accept image arguments")
    );

    drop(client);
    let _ = fs::remove_dir_all(state_root);
}

fn invocation(command: TuiSlashCommandAction, arguments: &str) -> SlashCommandInvocation {
    SlashCommandInvocation {
        command: built_in_catalog_command(command),
        origin: zeta_slash_commands::SlashCommandOrigin::Local,
        display_arguments: arguments.into(),
        arguments: (!arguments.is_empty())
            .then(|| ComposerInput::Text(arguments.into()))
            .into_iter()
            .collect(),
    }
}

fn client() -> (AppServerClient<InProcessTransport>, PathBuf) {
    static NEXT_STATE_ROOT: AtomicU64 = AtomicU64::new(1);
    let state_root = std::env::temp_dir().join(format!(
        "zeta-tui-slash-dispatch-{}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        NEXT_STATE_ROOT.fetch_add(1, Ordering::Relaxed)
    ));
    let client = start_in_process_client(
        InProcessClientOptions::new(
            &state_root,
            ClientInfo {
                name: "zeta-tui-test".into(),
                version: "1".into(),
            },
        )
        .with_model_operation_client(Arc::new(OfflineOperationClient)),
    )
    .unwrap();
    (client, state_root)
}

struct OfflineOperationClient;

impl OperationClient for OfflineOperationClient {
    fn execute(&self, _: &ClientRequest) -> Result<ClientResponse, ClientError> {
        Err(ClientError::Transport(
            "model transport is disabled in TUI command tests".into(),
        ))
    }
}
