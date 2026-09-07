use super::*;
use base64::Engine;
use std::sync::atomic::AtomicUsize;

#[derive(Clone, Copy, Eq, PartialEq)]
enum ResponseCase {
    Recover,
    Denied,
    StreamError,
    PartialStream,
}

struct RecoveryClient {
    calls: Mutex<Vec<String>>,
    attempts: AtomicUsize,
    refreshed_access: String,
    response: ResponseCase,
}

impl OperationClient for RecoveryClient {
    fn execute(&self, request: &ClientRequest) -> Result<ClientResponse, ClientError> {
        self.calls.lock().unwrap().push(request.url().into());
        if request.url() == "https://auth.openai.com/oauth/token" {
            let body: Value = serde_json::from_slice(request.body()).unwrap();
            assert_eq!(body["grant_type"], "refresh_token");
            return Ok(ClientResponse::new(
                200,
                Vec::new(),
                serde_json::to_vec(
                    &json!({"access_token":self.refreshed_access,"refresh_token":"renewed"}),
                )
                .unwrap(),
            ));
        }
        assert_luna_low(request);
        if self.attempts.fetch_add(1, Ordering::SeqCst) == 0
            || self.response == ResponseCase::Denied
        {
            Ok(ClientResponse::new(
                401,
                Vec::new(),
                br#"{"error":{"code":"invalid_api_key"}}"#.to_vec(),
            ))
        } else {
            Ok(ClientResponse::new(
                200,
                Vec::new(),
                serde_json::to_vec(&responses_response("recovered")).unwrap(),
            ))
        }
    }

    fn execute_streaming(
        &self,
        request: &ClientRequest,
        sink: &mut dyn OperationStreamSink,
    ) -> Result<ClientResponse, ClientError> {
        if matches!(
            self.response,
            ResponseCase::StreamError | ResponseCase::PartialStream
        ) {
            self.calls.lock().unwrap().push(request.url().into());
            assert_luna_low(request);
            if self.response == ResponseCase::PartialStream {
                sink.emit(b"event: response.output_text.delta\ndata: {\"type\":\"response.output_text.delta\",\"delta\":\"already delivered\"}\n\n")?;
            }
            sink.emit(b"event: response.failed\ndata: {\"type\":\"response.failed\",\"response\":{\"error\":{\"code\":\"invalid_api_key\",\"message\":\"Authentication failed\"}}}\n\n")?;
            return Ok(ClientResponse::new(200, Vec::new(), Vec::new()));
        }
        let response = self.execute(request)?;
        if !response.is_success() {
            return Ok(response);
        }
        StreamingTransport.execute_streaming(request, sink)
    }
}

fn assert_luna_low(request: &ClientRequest) {
    let body: Value = serde_json::from_slice(request.body()).unwrap();
    assert_eq!(body["model"], "gpt-5.6-luna");
    assert_eq!(body["reasoning"]["effort"], "low");
}

fn request() -> ModelRequest {
    let mut request = ModelRequest::text("hello");
    request.reasoning = Some(zeta_protocol::ReasoningConfig {
        effort: zeta_protocol::ReasoningEffort::Low,
        summary: false,
    });
    request
}

fn fixture(
    response: ResponseCase,
) -> (
    tempfile::TempDir,
    Arc<RecoveryClient>,
    Arc<dyn ModelInvoker>,
) {
    let jwt = |value: Value| {
        format!(
            "e30.{}.signature",
            base64::engine::general_purpose::URL_SAFE_NO_PAD
                .encode(serde_json::to_vec(&value).unwrap())
        )
    };
    let home = tempfile::tempdir().unwrap();
    std::fs::write(home.path().join("auth.json"), serde_json::to_vec(&json!({
        "auth_mode":"chatgpt","OPENAI_API_KEY":null,"last_refresh":"2026-09-07T00:00:00Z",
        "tokens":{"id_token":jwt(json!({"https://api.openai.com/auth":{"chatgpt_account_id":"account-1"}})),"access_token":jwt(json!({"exp":4_000_000_000_u64,"jti":"old"})),"refresh_token":"old-refresh","account_id":"account-1"}
    })).unwrap()).unwrap();
    let client = Arc::new(RecoveryClient {
        calls: Mutex::new(Vec::new()),
        attempts: AtomicUsize::new(0),
        refreshed_access: jwt(json!({"exp":4_000_000_000_u64,"jti":"new"})),
        response,
    });
    let secrets = Arc::new(MemorySecretStore::default());
    let auth = ChatGptOAuth::with_client(
        home.path().into(),
        secrets.clone(),
        client.clone(),
        zeta_chatgpt::ChatGptAuthManagement::Zeta,
    );
    let runtime = ModelProviderRuntime::with_client_and_secrets(
        ProviderConfigRegistry::builtin(),
        client.clone(),
        secrets,
    )
    .with_chatgpt_oauth(auth);
    let model = runtime
        .build_model(
            &provider_config("openai"),
            &model_ref("openai", "gpt-5.6-luna"),
        )
        .unwrap();
    (home, client, model)
}

#[test]
fn managed_chatgpt_recovers_one_rejected_unary_or_stream_request() {
    for streaming in [false, true] {
        let (_home, client, model) = fixture(ResponseCase::Recover);
        let response = if streaming {
            model.stream_with_cancellation(
                &request(),
                &CancellationSource::new().token(),
                &mut RecordedModelEvents::default(),
            )
        } else {
            model.invoke(&request())
        }
        .unwrap();
        assert_eq!(
            response.text(),
            if streaming { "live" } else { "recovered" }
        );
        assert_eq!(
            *client.calls.lock().unwrap(),
            vec![
                "https://chatgpt.com/backend-api/codex/responses",
                "https://auth.openai.com/oauth/token",
                "https://chatgpt.com/backend-api/codex/responses",
            ]
        );
    }
}

#[test]
fn chatgpt_stream_with_delivered_output_is_never_replayed() {
    let (home, client, model) = fixture(ResponseCase::PartialStream);
    let before = std::fs::read(home.path().join("auth.json")).unwrap();
    let mut events = RecordedModelEvents::default();
    let result =
        model.stream_with_cancellation(&request(), &CancellationSource::new().token(), &mut events);
    assert!(matches!(result, Err(ModelProviderError::AuthFailed(_))));
    assert_eq!(
        events.0,
        vec![ModelStreamEvent::TextDelta("already delivered".into())]
    );
    assert_eq!(client.calls.lock().unwrap().len(), 1);
    assert_eq!(
        std::fs::read(home.path().join("auth.json")).unwrap(),
        before
    );
}

#[test]
fn chatgpt_accepted_stream_error_without_output_is_not_replayed() {
    let (_home, client, model) = fixture(ResponseCase::StreamError);
    let mut events = RecordedModelEvents::default();
    let result =
        model.stream_with_cancellation(&request(), &CancellationSource::new().token(), &mut events);
    assert!(matches!(result, Err(ModelProviderError::AuthFailed(_))));
    assert!(events.0.is_empty());
    assert_eq!(client.calls.lock().unwrap().len(), 1);
}

#[test]
fn chatgpt_stops_after_one_recovery_attempt() {
    let (_home, client, model) = fixture(ResponseCase::Denied);
    assert!(matches!(
        model.invoke(&request()),
        Err(ModelProviderError::AuthFailed(_))
    ));
    assert_eq!(client.calls.lock().unwrap().len(), 3);
    assert!(model.invoke(&request()).is_err());
    assert_eq!(
        client
            .calls
            .lock()
            .unwrap()
            .iter()
            .filter(|url| url.ends_with("/oauth/token"))
            .count(),
        1
    );
}
