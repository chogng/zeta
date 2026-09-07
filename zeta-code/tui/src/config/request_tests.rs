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
fn connection_fetch_failure_keeps_saved_configuration_and_can_retry() {
    use crate::config::openai::Operation;
    use crate::config::openai::Request;
    let mut current = empty_config_snapshot();
    let config = zeta_app_server_protocol::protocol::config::ProviderConfigDto {
        provider: "custom-one".into(),
        base_url: Some("https://example.test/v1".into()),
        custom: Some(
            zeta_app_server_protocol::protocol::config::CustomProviderConfigDto {
                name: "Example".into(),
                protocol:
                    zeta_app_server_protocol::protocol::config::CustomProviderProtocolDto::Responses,
            },
        ),
        max_output_tokens: None,
        model_context: Default::default(),
    };
    current
        .providers
        .insert(config.provider.clone(), config.clone());
    current.revision = 4;
    let requests = Arc::new(Mutex::new(Vec::new()));
    let mut client = AppServerClient::new(RecordingTransport {
        requests: requests.clone(),
        responses: VecDeque::from([
            response(1, serde_json::to_value(&current).unwrap()),
            serde_json::json!({"jsonrpc":"2.0","id":2,"error":{"code":-32000,"message":"Model endpoint unavailable"}}).to_string(),
            response(3, serde_json::to_value(&current).unwrap()),
            response(4, serde_json::json!({"providers":[]})),
        ]),
    });
    let result = super::execute(
        &mut client,
        super::Command::Connection(Request {
            id: crate::client::new_command_id("test"),
            revision: 4,
            config,
            key: None,
            operation: Operation::FetchModels,
        }),
    )
    .unwrap();
    let super::Event::Connection(reply) = result else {
        panic!("expected connection result")
    };
    let (_, models) = reply.result.unwrap();
    assert!(models.unwrap().is_err());
    let requests = requests.lock().unwrap();
    assert_eq!(
        requests
            .iter()
            .map(|request| request["method"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec![
            "config/read",
            "provider/models/list",
            "config/read",
            "provider/list"
        ]
    );
    assert_eq!(requests[1]["params"]["provider"], "custom-one");
}

#[test]
fn custom_provider_saves_settings_and_key_separately_then_refreshes() {
    let config = zeta_app_server_protocol::protocol::config::ProviderConfigDto {
        provider: "custom-one".into(),
        custom: Some(
            zeta_app_server_protocol::protocol::config::CustomProviderConfigDto {
                name: "Example".into(),
                protocol:
                    zeta_app_server_protocol::protocol::config::CustomProviderProtocolDto::Responses,
            },
        ),
        base_url: Some("https://example.test/v1".into()),
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
        requests: requests.clone(),
        responses: VecDeque::from([
            response(1, serde_json::to_value(empty_config_snapshot()).unwrap()),
            response(
                2,
                serde_json::json!({"revision":8,"generation":2,"disposition":"updated"}),
            ),
            response(
                3,
                serde_json::json!({"provider":"custom-one","apiKeyConfigured":true}),
            ),
            response(4, serde_json::to_value(refreshed).unwrap()),
            response(5, serde_json::json!({"providers":[]})),
        ]),
    });
    let result = super::execute(
        &mut client,
        super::Command::Connection(crate::config::openai::Request {
            id: crate::client::new_command_id("test"),
            revision: 7,
            config: config.clone(),
            key: Some(crate::config::ProviderApiKeyEdit::new(
                "custom-one".into(),
                "test-key".into(),
            )),
            operation: crate::config::openai::Operation::Save,
        }),
    )
    .unwrap();
    assert!(matches!(result, super::Event::Connection(reply) if reply.result.is_ok()));
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 5);
    assert_eq!(requests[1]["method"], "provider/configure");
    assert_eq!(requests[1]["params"]["expectedRevision"], 7);
    assert_eq!(
        requests[1]["params"]["config"],
        serde_json::to_value(config).unwrap()
    );
    assert!(!requests[1].to_string().contains("test-key"));
    assert_eq!(requests[2]["method"], "provider/apiKey/set");
    assert_eq!(requests[2]["params"]["provider"], "custom-one");
}

#[test]
fn rejected_connection_update_does_not_send_key_or_report_saved() {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let mut client = AppServerClient::new(RecordingTransport {
        requests: requests.clone(), responses: VecDeque::from([
            response(1, serde_json::to_value(empty_config_snapshot()).unwrap()),
            serde_json::json!({"jsonrpc":"2.0","id":2,"error":{"code":-32602,"message":"InvalidParams"}}).to_string(),
        ]),
    });
    let result = super::execute(
        &mut client,
        super::Command::Connection(crate::config::openai::Request {
            id: crate::client::new_command_id("test"),
            revision: 7,
            config: zeta_app_server_protocol::protocol::config::ProviderConfigDto {
                provider: "openai-compatible".into(),
                custom: None,
                base_url: Some("invalid".into()),
                max_output_tokens: None,
                model_context: Default::default(),
            },
            key: Some(crate::config::ProviderApiKeyEdit::new(
                "openai-compatible".into(),
                "test-key".into(),
            )),
            operation: crate::config::openai::Operation::Save,
        }),
    )
    .unwrap();
    assert!(matches!(result, super::Event::Connection(reply) if reply.result.is_err()));
    assert_eq!(requests.lock().unwrap().len(), 2);
}
