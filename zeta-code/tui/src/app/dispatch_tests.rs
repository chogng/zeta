use super::ActiveConversation;
use crate::app::composer_slot::ComposerSlot;
use crate::app::{App, AppCommand, AppEvent, Status};
use crate::thread::composer::{
    ChatInputItem, SlashCommandInvocation, TuiSlashCommandAction, built_in_catalog_command,
};
use crate::thread::transcript::MessageRole;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::fs;
use std::ops::Deref;
use std::ops::DerefMut;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::MutexGuard;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};
use zeta_app_server_client::{
    AppServerClient, InProcessClientOptions, InProcessTransport, start_in_process_client,
};
use zeta_app_server_protocol::protocol::common::ClientInfo;
use zeta_app_server_protocol::protocol::config::{ProviderConfigDto, ProviderConfigureParams};
use zeta_app_server_protocol::protocol::environment::PermissionDto;
use zeta_app_server_protocol::protocol::environment::SessionDirListParams;
use zeta_app_server_protocol::protocol::session::SessionReadParams;
use zeta_app_server_protocol::protocol::session::SessionThreadReadParams;
use zeta_app_server_protocol::protocol::skills::SkillEnablementDto;
use zeta_client::ClientError;
use zeta_client::ClientRequest;
use zeta_client::ClientResponse;
use zeta_client::OperationClient;
use zeta_protocol::CommandId;
use zeta_protocol::SessionStatus;
use zeta_protocol::ThreadStatus;

#[test]
fn fork_persists_lineage_switches_threads_and_does_not_call_the_model() {
    let (mut client, state_root, model) = client_with_model_probe();
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

    let forked_thread_id = conversation.thread_id().clone();
    let persisted_session = client
        .read_session(SessionReadParams {
            session_id: original_session.clone(),
        })
        .unwrap()
        .session;
    let forked_membership = persisted_session
        .threads
        .iter()
        .find(|thread| thread.thread_id == forked_thread_id)
        .unwrap();
    assert_eq!(forked_membership.status, ThreadStatus::Active);
    assert_eq!(forked_membership.parent_thread_id, None);
    assert_eq!(
        forked_membership.forked_from_id.as_ref(),
        Some(&original_thread)
    );
    let persisted_thread = client
        .read_session_thread(SessionThreadReadParams {
            session_id: original_session.clone(),
            thread_id: forked_thread_id,
            history: None,
        })
        .unwrap()
        .thread;
    assert_eq!(conversation.thread_sequence(), persisted_thread.sequence);
    assert!(persisted_thread.turns.is_empty());
    assert_eq!(model.calls(), 0);

    conversation.execute(
        &mut client,
        invocation(TuiSlashCommandAction::New, "fresh task"),
        &mut app,
    );
    assert_ne!(conversation.session_id(), &original_session);
    assert_eq!(
        app.messages().last().unwrap().text,
        "Started a new session."
    );

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
    assert_eq!(model.calls(), 0);

    drop(client);
    let _ = fs::remove_dir_all(state_root);
}

#[test]
fn archive_persists_status_starts_a_new_session_and_does_not_call_the_model() {
    let (mut client, state_root, model) = client_with_model_probe();
    let mut conversation = ActiveConversation::start(&mut client, "archive me".into()).unwrap();
    let archived_session_id = conversation.session_id().clone();
    let mut app = App::new();

    conversation.execute(
        &mut client,
        invocation(TuiSlashCommandAction::Archive, ""),
        &mut app,
    );

    let archived = client
        .read_session(SessionReadParams {
            session_id: archived_session_id.clone(),
        })
        .unwrap()
        .session;
    assert_eq!(archived.status, SessionStatus::Archived);
    assert_ne!(conversation.session_id(), &archived_session_id);
    let next_session_id = conversation.session_id().clone();
    let next = client
        .read_session(SessionReadParams {
            session_id: next_session_id.clone(),
        })
        .unwrap()
        .session;
    assert_eq!(next.status, SessionStatus::Active);
    assert_eq!(
        app.messages().last().unwrap().text,
        "Archived the previous session and started a new session."
    );
    assert_eq!(model.calls(), 0);

    drop(client);
    let _ = fs::remove_dir_all(state_root);
}

#[test]
fn status_mcp_connectors_and_skills_return_real_surfaces() {
    let (mut client, state_root) = client();
    let mut conversation = ActiveConversation::start(&mut client, "commands".into()).unwrap();
    let mut app = App::new();

    conversation.execute(
        &mut client,
        invocation(TuiSlashCommandAction::Status, ""),
        &mut app,
    );
    assert!(matches!(app.composer_slot(), Some(ComposerSlot::Status(_))));
    assert!(app.overlay().is_none());
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

    conversation.execute(
        &mut client,
        invocation(TuiSlashCommandAction::Mcp, ""),
        &mut app,
    );
    assert_eq!(app.list_selection().unwrap().title(), "MCP servers");
    assert!(app.list_selection().unwrap().search().is_some());
    app.update(AppEvent::ComposerSlotClosed);

    conversation.execute(
        &mut client,
        invocation(TuiSlashCommandAction::Connectors, ""),
        &mut app,
    );
    assert_eq!(app.list_selection().unwrap().title(), "Connectors");
    assert!(app.list_selection().unwrap().search().is_some());
    app.update(AppEvent::ComposerSlotClosed);

    conversation.execute(
        &mut client,
        invocation(TuiSlashCommandAction::Skills, ""),
        &mut app,
    );
    assert_eq!(app.status(), &Status::Ready);
    let selection = app.list_selection().unwrap();
    assert_eq!(selection.title(), "Skills");
    assert_eq!(selection.active_tab().label(), "All (1)");
    assert_eq!(selection.visible_items()[0].label(), "skill-creator");

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

    let all = app.list_selection().unwrap();
    assert_eq!(all.active_tab().label(), "All (1)");
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

    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    let enabled = app.list_selection().unwrap();
    assert_eq!(enabled.active_tab().label(), "Enabled (1)");
    assert_eq!(enabled.visible_items()[0].label(), "skill-creator");

    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
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

    let view = crate::skills::set_enablement(
        &mut client,
        &zeta_protocol::SessionId::new("test-session").unwrap(),
        skill_id,
        enablement,
    )
    .unwrap();
    app.update(AppEvent::SkillSettingsUpdated(view));
    app.handle_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE));
    let disabled = app.list_selection().unwrap();
    assert_eq!(disabled.active_tab().label(), "Disabled (1)");
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
    assert_eq!(
        app.status_line()
            .top_text_for_width(80, app.status_line_runtime()),
        "model-one"
    );
    assert_eq!(
        app.status_line()
            .policy_text_for_width(80, app.approval_mode()),
        "⏸ ask permissions on"
    );

    conversation.execute(
        &mut client,
        invocation(TuiSlashCommandAction::Model, "clear"),
        &mut app,
    );
    assert_eq!(client.read_config().unwrap().preferred_model, None);
    assert_eq!(
        app.status_line()
            .policy_text_for_width(80, app.approval_mode()),
        "⏸ ask permissions on"
    );
    assert_eq!(app.status(), &Status::Ready);

    drop(client);
    let _ = fs::remove_dir_all(state_root);
}

#[test]
fn theme_selection_updates_the_tui_toml_section() {
    let (mut client, state_root) = client();

    crate::theme::set_preference(&mut client, "zeta-code-light".into()).unwrap();

    assert_eq!(
        crate::theme::preference(&client.read_config().unwrap()),
        "zeta-code-light"
    );
    drop(client);
    let _ = fs::remove_dir_all(state_root);
}

#[test]
fn keybindings_and_status_line_are_persisted_in_the_tui_toml_section() {
    let (mut client, state_root) = client();
    let revision = client.read_config().unwrap().revision;

    crate::keymap::set_keymap(
        &mut client,
        crate::keymap::KeymapEdit {
            expected_revision: revision,
            command_id: "zetaCode.action.copyLastResponse".into(),
            kind: crate::keymap::KeymapEditKind::Set {
                key: "ctrl+y".into(),
                intent: crate::keymap::KeymapEditIntent::AddAlternate,
            },
        },
    )
    .unwrap();
    let revision = client.read_config().unwrap().revision;
    crate::status::set_status_line(
        &mut client,
        crate::status::StatusLineEdit {
            expected_revision: revision,
            item: crate::status::StatusLineItem::GitChanges,
            enabled: false,
        },
    )
    .unwrap();

    let config = client.read_config().unwrap();
    assert_eq!(
        config.tui.0.get("keybindings"),
        Some(&serde_json::json!([{
            "key": "ctrl+y",
            "command": "zetaCode.action.copyLastResponse"
        }]))
    );
    assert_eq!(
        config.tui.0.get("statusLine"),
        Some(&serde_json::json!(["permissions", "model", "git-branch"]))
    );

    drop(client);
    let _ = fs::remove_dir_all(state_root);
}

#[test]
fn resume_and_model_without_arguments_open_actionable_pickers() {
    let (mut client, state_root) = client();
    let mut conversation = ActiveConversation::start(&mut client, "current".into()).unwrap();
    let current_session = conversation.session_id().to_string();
    let mut app = App::new();

    conversation.execute(
        &mut client,
        invocation(TuiSlashCommandAction::Resume, ""),
        &mut app,
    );
    assert_eq!(app.list_selection().unwrap().title(), "Resume session");
    assert!(app.list_selection().unwrap().search().is_some());
    assert!(app.session_manager_view().is_none());
    assert_eq!(
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        Some(AppCommand::ResumeSession {
            session_id: current_session,
            preferred_thread_id: None,
        })
    );
    app.update(AppEvent::ComposerSlotClosed);

    conversation.execute(
        &mut client,
        invocation(TuiSlashCommandAction::Model, ""),
        &mut app,
    );
    assert_eq!(app.list_selection().unwrap().title(), "Model");
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
fn rewind_without_arguments_opens_the_checkpoint_picker() {
    let (mut client, state_root) = client();
    let mut conversation = ActiveConversation::start(&mut client, "rewind".into()).unwrap();
    let mut app = App::new();

    conversation.execute(
        &mut client,
        invocation(TuiSlashCommandAction::Rewind, ""),
        &mut app,
    );

    assert_eq!(app.list_selection().unwrap().title(), "Rewind");
    assert!(app.list_selection().unwrap().search().is_some());
    assert!(app.list_selection().unwrap().visible_items().is_empty());

    drop(client);
    let _ = fs::remove_dir_all(state_root);
}

#[test]
fn add_dir_adds_lists_and_removes_the_exact_session_directory() {
    let _test_guard = dispatch_test_guard();
    let state_root = std::env::temp_dir().join(format!(
        "zeta-tui-add-dir-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let dir = state_root.join("dir");
    let additional = state_root.join("additional");
    fs::create_dir_all(&dir).unwrap();
    fs::create_dir_all(&additional).unwrap();
    let mut client = start_in_process_client(
        InProcessClientOptions::new(
            &state_root,
            ClientInfo {
                name: "zeta-tui-add-dir-test".into(),
                version: "1".into(),
            },
        )
        .with_capabilities(crate::client_capabilities())
        .with_dir_root(&dir)
        .with_model_operation_client(Arc::new(OfflineOperationClient::default())),
    )
    .unwrap();
    let mut conversation = ActiveConversation::start(&mut client, "add dir".into()).unwrap();
    let mut app = App::new();

    let output = super::execute_product_command(
        conversation,
        &mut client,
        invocation(
            TuiSlashCommandAction::AddDir,
            &additional.display().to_string(),
        ),
    )
    .unwrap();
    conversation = output.conversation;
    for event in output.events {
        app.update(event);
    }

    let listed = client
        .list_session_dirs(SessionDirListParams {
            session_id: conversation.session_id().clone(),
        })
        .unwrap();
    assert_eq!(listed.dirs.len(), 1);
    assert_eq!(
        listed.dirs[0].path.canonicalize().unwrap(),
        additional.canonicalize().unwrap()
    );
    assert_eq!(listed.dirs[0].permissions, Vec::<PermissionDto>::new());
    conversation.execute(
        &mut client,
        invocation(TuiSlashCommandAction::AddDir, ""),
        &mut app,
    );
    assert_eq!(app.list_selection().unwrap().title(), "Directories");
    let Some(AppCommand::RemoveDir { path }) =
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
    else {
        panic!("the selected directory emits an exact remove command")
    };
    assert_eq!(
        path.canonicalize().unwrap(),
        additional.canonicalize().unwrap()
    );

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
        arguments: vec![ChatInputItem::Image {
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
            .then(|| ChatInputItem::Text(arguments.into()))
            .into_iter()
            .collect(),
    }
}

fn client() -> (DispatchTestClient, PathBuf) {
    let (client, state_root, _) = client_with_model_probe();
    (client, state_root)
}

fn client_with_model_probe() -> (DispatchTestClient, PathBuf, Arc<OfflineOperationClient>) {
    let guard = dispatch_test_guard();
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
    let model = Arc::new(OfflineOperationClient::default());
    let client = start_in_process_client(
        InProcessClientOptions::new(
            &state_root,
            ClientInfo {
                name: "zeta-tui-test".into(),
                version: "1".into(),
            },
        )
        .with_model_operation_client(model.clone()),
    )
    .unwrap();
    (
        DispatchTestClient {
            client,
            _guard: guard,
        },
        state_root,
        model,
    )
}

fn dispatch_test_guard() -> MutexGuard<'static, ()> {
    crate::test_support::in_process_test_guard()
}

struct DispatchTestClient {
    client: AppServerClient<InProcessTransport>,
    _guard: MutexGuard<'static, ()>,
}

impl Deref for DispatchTestClient {
    type Target = AppServerClient<InProcessTransport>;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl DerefMut for DispatchTestClient {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.client
    }
}

#[derive(Default)]
struct OfflineOperationClient {
    calls: AtomicU64,
}

impl OfflineOperationClient {
    fn calls(&self) -> u64 {
        self.calls.load(Ordering::SeqCst)
    }
}

impl OperationClient for OfflineOperationClient {
    fn execute(&self, _: &ClientRequest) -> Result<ClientResponse, ClientError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err(ClientError::Transport(
            "model transport is disabled in TUI command tests".into(),
        ))
    }
}
