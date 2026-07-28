use super::ActiveConversation;
use super::help_text;
use crate::app::{App, MessageRole, Status};
use crate::toppane::{ComposerInput, SlashCommand, SlashCommandInvocation, SlashCommandItem};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use zeta_app_server_client::{
    AppServerClient, InProcessClientOptions, InProcessTransport, start_in_process_client,
};
use zeta_app_server_protocol::protocol::common::ClientInfo;
use zeta_app_server_protocol::protocol::config::{ProviderConfigDto, ProviderConfigureParams};
use zeta_protocol::CommandId;

#[test]
fn help_lists_only_executable_builtins() {
    let help = help_text();

    assert!(help.contains("/status"));
    assert!(help.contains("/resume"));
    assert!(help.contains("/model"));
    assert!(!help.contains("/login"));
    assert!(!help.contains("/plugins"));
    assert!(!help.contains("/review"));
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
        invocation(SlashCommand::Fork, "investigation"),
        &mut app,
    );

    assert_eq!(conversation.session_id(), &original_session);
    assert_ne!(conversation.thread_id(), &original_thread);
    assert_eq!(app.status(), &Status::Ready);

    conversation.execute(
        &mut client,
        invocation(SlashCommand::New, "fresh task"),
        &mut app,
    );
    assert_ne!(conversation.session_id(), &original_session);

    conversation.execute(
        &mut client,
        invocation(SlashCommand::Resume, original_session.as_str()),
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
fn status_config_mcp_skills_and_help_return_real_results() {
    let (mut client, state_root) = client();
    let mut conversation = ActiveConversation::start(&mut client, "commands".into()).unwrap();
    let mut app = App::new();

    for command in [
        SlashCommand::Status,
        SlashCommand::Config,
        SlashCommand::Mcp,
        SlashCommand::Skills,
        SlashCommand::Help,
    ] {
        conversation.execute(&mut client, invocation(command, ""), &mut app);
        assert_eq!(app.status(), &Status::Ready);
        assert_eq!(app.messages().last().unwrap().role, MessageRole::Notice);
    }

    assert!(
        app.messages()
            .iter()
            .any(|message| message.text.contains("Session:"))
    );
    assert!(
        app.messages()
            .iter()
            .any(|message| message.text.contains("Config revision:"))
    );
    assert!(
        app.messages()
            .iter()
            .any(|message| message.text == "No MCP servers configured.")
    );
    assert!(
        app.messages()
            .iter()
            .any(|message| message.text == "No skill sources configured.")
    );

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
            },
        })
        .unwrap();
    let mut app = App::new();

    conversation.execute(
        &mut client,
        invocation(SlashCommand::Model, "test/model-one"),
        &mut app,
    );

    let configured = client.read_config().unwrap();
    let selected = configured.preferred_model.unwrap();
    assert_eq!(selected.provider, "test");
    assert_eq!(selected.model, "model-one");

    conversation.execute(
        &mut client,
        invocation(SlashCommand::Model, "clear"),
        &mut app,
    );
    assert_eq!(client.read_config().unwrap().preferred_model, None);
    assert_eq!(app.status(), &Status::Ready);

    drop(client);
    let _ = fs::remove_dir_all(state_root);
}

#[test]
fn product_commands_reject_image_arguments_instead_of_silently_dropping_them() {
    let (mut client, state_root) = client();
    let mut conversation = ActiveConversation::start(&mut client, "images".into()).unwrap();
    let mut app = App::new();
    let invocation = SlashCommandInvocation {
        command: SlashCommandItem::Builtin(SlashCommand::Model),
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

fn invocation(command: SlashCommand, arguments: &str) -> SlashCommandInvocation {
    SlashCommandInvocation {
        command: SlashCommandItem::Builtin(command),
        display_arguments: arguments.into(),
        arguments: (!arguments.is_empty())
            .then(|| ComposerInput::Text(arguments.into()))
            .into_iter()
            .collect(),
    }
}

fn client() -> (AppServerClient<InProcessTransport>, PathBuf) {
    let state_root = std::env::temp_dir().join(format!(
        "zeta-tui-slash-dispatch-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let client = start_in_process_client(InProcessClientOptions::new(
        &state_root,
        ClientInfo {
            name: "zeta-tui-test".into(),
            version: "1".into(),
        },
    ))
    .unwrap();
    (client, state_root)
}
