use super::set_language_server_mode;
use crate::config::LanguageServerEdit;
use crate::test_support::empty_config_snapshot;
use crate::widgets::list_selection::ListSelectionState;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;
use zeta_app_server_client::AppServerClient;
use zeta_app_server_client::ClientError;
use zeta_app_server_client::JsonRpcTransport;
use zeta_app_server_protocol::protocol::config::ConfigCommandDispositionDto;
use zeta_app_server_protocol::protocol::config::ConfigCommandResult;
use zeta_app_server_protocol::protocol::config::LanguageServerConfigDto;
use zeta_app_server_protocol::protocol::config::LanguageServerModeDto;
use zeta_app_server_protocol::protocol::provider::ProviderListResult;

#[derive(Clone)]
struct RecordingTransport {
    responses: VecDeque<String>,
    requests: Arc<Mutex<Vec<serde_json::Value>>>,
}

impl JsonRpcTransport for RecordingTransport {
    fn round_trip(&mut self, request: &str) -> Result<String, ClientError> {
        self.requests
            .lock()
            .expect("request log is not poisoned")
            .push(serde_json::from_str(request).expect("request is valid JSON"));
        self.responses
            .pop_front()
            .ok_or_else(|| ClientError::Transport("no response".into()))
    }
}

#[test]
fn language_server_switch_uses_the_backend_config_authority_and_refreshes_the_tab() {
    let executable = "C:\\tools\\rust-analyzer.exe";
    let mut refreshed = empty_config_snapshot();
    refreshed.revision = 8;
    refreshed.language_servers.insert(
        "rust-analyzer".into(),
        LanguageServerConfigDto {
            mode: LanguageServerModeDto::Enabled,
            executable: Some(executable.into()),
        },
    );
    let requests = Arc::new(Mutex::new(Vec::new()));
    let mut client = AppServerClient::new(RecordingTransport {
        responses: VecDeque::from([
            response(
                1,
                serde_json::to_value(ConfigCommandResult {
                    revision: 8,
                    generation: 2,
                    disposition: ConfigCommandDispositionDto::Updated,
                })
                .unwrap(),
            ),
            response(2, serde_json::to_value(refreshed).unwrap()),
            response(
                3,
                serde_json::to_value(ProviderListResult {
                    providers: Vec::new(),
                })
                .unwrap(),
            ),
        ]),
        requests: Arc::clone(&requests),
    });

    let result = set_language_server_mode(
        &mut client,
        LanguageServerEdit {
            expected_revision: 7,
            server_id: "rust-analyzer".into(),
            config: LanguageServerConfigDto {
                mode: LanguageServerModeDto::Enabled,
                executable: Some(executable.into()),
            },
        },
    )
    .unwrap();

    let requests = requests.lock().expect("request log is not poisoned");
    assert_eq!(requests.len(), 3);
    assert_eq!(requests[0]["method"], "languageServer/configure");
    assert_eq!(requests[0]["params"]["expectedRevision"], 7);
    assert_eq!(requests[0]["params"]["serverId"], "rust-analyzer");
    assert_eq!(requests[0]["params"]["config"]["mode"], "enabled");
    assert_eq!(requests[0]["params"]["config"]["executable"], executable);
    assert_eq!(requests[1]["method"], "config/read");
    assert_eq!(requests[2]["method"], "provider/list");
    drop(requests);

    let mut state = ListSelectionState::new(result.choices.model);
    state.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    state.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    let _ = state.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    let _ = state.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    assert_eq!(state.visible_items().len(), 1);
    assert_eq!(
        state.visible_items()[0].description(),
        Some("C:\\tools\\rust-analyzer.exe [ ✔ ]")
    );
}

fn response(id: u64, result: serde_json::Value) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    })
    .to_string()
}

#[test]
fn custom_provider_saves_non_secret_settings_with_revision_and_refreshes() {
    use zeta_app_server_protocol::protocol::config::ProviderConfigDto;
    use zeta_app_server_protocol::protocol::config::ProviderConfigureParams;
    let config = ProviderConfigDto {
        provider: "openai-compatible".into(),
        base_url: Some("https://gateway.example/v1".into()),
        max_output_tokens: Some(2048),
        model_context: Default::default(),
    };
    let mut refreshed = empty_config_snapshot();
    refreshed.revision = 8;
    refreshed
        .providers
        .insert(config.provider.clone(), config.clone());
    let requests = Arc::new(Mutex::new(Vec::new()));
    let mut client = AppServerClient::new(RecordingTransport {
        responses: VecDeque::from([
            response(
                1,
                serde_json::to_value(ConfigCommandResult {
                    revision: 8,
                    generation: 2,
                    disposition: ConfigCommandDispositionDto::Updated,
                })
                .unwrap(),
            ),
            response(2, serde_json::to_value(refreshed).unwrap()),
            response(
                3,
                serde_json::to_value(ProviderListResult {
                    providers: Vec::new(),
                })
                .unwrap(),
            ),
        ]),
        requests: Arc::clone(&requests),
    });
    let event = super::execute(
        &mut client,
        super::Command::ConfigureProvider(ProviderConfigureParams {
            command_id: crate::client::new_command_id("test"),
            expected_revision: 7,
            config: config.clone(),
        }),
    )
    .unwrap();
    assert!(matches!(event, super::Event::ProviderConfigured(_)));
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 3);
    assert_eq!(requests[0]["method"], "provider/configure");
    assert_eq!(requests[0]["params"]["expectedRevision"], 7);
    assert_eq!(
        requests[0]["params"]["config"],
        serde_json::to_value(config).unwrap()
    );
    assert_eq!(requests[1]["method"], "config/read");
    assert_eq!(requests[2]["method"], "provider/list");
}

#[test]
fn rejected_custom_provider_update_does_not_report_saved_or_send_a_key() {
    use zeta_app_server_protocol::protocol::config::ProviderConfigDto;
    use zeta_app_server_protocol::protocol::config::ProviderConfigureParams;
    let requests = Arc::new(Mutex::new(Vec::new()));
    let mut client = AppServerClient::new(RecordingTransport {
        responses: VecDeque::from([serde_json::json!({"jsonrpc":"2.0", "id":1, "error":{"code":-32602,"message":"InvalidParams"}}).to_string()]),
        requests: Arc::clone(&requests),
    });
    assert!(
        super::execute(
            &mut client,
            super::Command::ConfigureProvider(ProviderConfigureParams {
                command_id: crate::client::new_command_id("test"),
                expected_revision: 7,
                config: ProviderConfigDto {
                    provider: "openai-compatible".into(),
                    base_url: Some("invalid".into()),
                    max_output_tokens: None,
                    model_context: Default::default()
                },
            })
        )
        .is_err()
    );
    assert_eq!(requests.lock().unwrap().len(), 1);
}
