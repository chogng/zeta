use super::credential::TokenCredential;
use super::*;
use base64::Engine;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use zeta_client::ClientError;
use zeta_client::ClientRequest;
use zeta_client::ClientResponse;
use zeta_client::OperationClient;
use zeta_http_client::HttpResponse;
use zeta_login::AccountRef;
use zeta_login::AccountStatus;
use zeta_login::BeginLogin;
use zeta_login::InteractiveLoginDriver;
use zeta_login::LoginMethod;
use zeta_login::LoginService;
use zeta_secrets::MemorySecretStore;
use zeta_secrets::SecretStore;

struct ScriptedClient {
    responses: Mutex<VecDeque<ClientResponse>>,
    requests: Mutex<Vec<ClientRequest>>,
}

impl ScriptedClient {
    fn new(responses: impl IntoIterator<Item = ClientResponse>) -> Self {
        Self {
            responses: Mutex::new(responses.into_iter().collect()),
            requests: Mutex::new(Vec::new()),
        }
    }
}

impl OperationClient for ScriptedClient {
    fn execute(&self, request: &ClientRequest) -> Result<ClientResponse, ClientError> {
        self.requests.lock().unwrap().push(request.clone());
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| ClientError::Transport("script exhausted".into()))
    }
}

#[test]
fn device_flow_persists_tokens_and_projects_subscription_headers() {
    let id_token = jwt(serde_json::json!({
        "email": "person@example.com",
        "https://api.openai.com/auth": {
            "chatgpt_plan_type": "plus",
            "chatgpt_user_id": "user-1",
            "chatgpt_account_id": "account-1"
        }
    }));
    let access_token = jwt(serde_json::json!({ "exp": 4_000_000_000_u64 }));
    let client = Arc::new(ScriptedClient::new([
        response(
            200,
            r#"{"device_auth_id":"device-1","user_code":"ABCD-EFGH","interval":"1"}"#,
        ),
        response(
            200,
            r#"{"authorization_code":"auth-code","code_challenge":"unused","code_verifier":"verifier"}"#,
        ),
        response(
            200,
            &format!(
                r#"{{"id_token":"{id_token}","access_token":"{access_token}","refresh_token":"refresh-secret"}}"#
            ),
        ),
    ]));
    let secrets = Arc::new(MemorySecretStore::default());
    let runtime = ChatGptOAuth::with_client(secrets.clone(), client.clone());
    let driver: Arc<dyn InteractiveLoginDriver> = runtime.clone();
    let service = Arc::new(LoginService::new(driver).unwrap());
    runtime.install_login_service(&service).unwrap();

    let started = service.begin(LoginMethod::OpenAiChatGptDeviceCode).unwrap();
    assert!(matches!(
        started,
        BeginLogin::DeviceCode {
            ref verification_url,
            ref user_code,
            ..
        } if verification_url == "https://auth.openai.com/codex/device"
            && user_code == "ABCD-EFGH"
    ));
    for _ in 0..100 {
        if !service.read().unwrap().accounts.is_empty() {
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }
    let account = service.read().unwrap().accounts[0].clone();
    assert_eq!(account.account.provider, OPENAI_CHATGPT_PROVIDER_ID);
    assert_eq!(account.email.as_deref(), Some("person@example.com"));
    assert_eq!(account.plan.as_deref(), Some("plus"));
    assert_eq!(account.status, AccountStatus::Ready);

    let target = runtime.api_target().unwrap();
    assert_eq!(target.base_url, CHATGPT_RESPONSES_BASE_URL);
    assert!(target.headers.iter().any(|header| {
        header.name() == "Authorization" && header.value() == format!("Bearer {access_token}")
    }));
    assert!(
        target.headers.iter().any(|header| {
            header.name() == "ChatGPT-Account-ID" && header.value() == "account-1"
        })
    );
    assert!(
        target
            .headers
            .iter()
            .any(|header| header.name() == "Originator" && header.value() == "zeta")
    );
    assert!(
        secrets
            .load(&ChatGptOAuth::credential_key())
            .unwrap()
            .is_some()
    );

    let requests = client.requests.lock().unwrap();
    assert_eq!(
        requests[0].url(),
        "https://auth.openai.com/api/accounts/deviceauth/usercode"
    );
    assert_eq!(
        requests[1].url(),
        "https://auth.openai.com/api/accounts/deviceauth/token"
    );
    assert_eq!(requests[2].url(), "https://auth.openai.com/oauth/token");
    let exchange = std::str::from_utf8(requests[2].body()).unwrap();
    assert!(exchange.contains("code=auth-code"));
    assert!(
        exchange.contains("redirect_uri=https%3A%2F%2Fauth.openai.com%2Fdeviceauth%2Fcallback")
    );
}

#[test]
fn api_target_refreshes_expiring_credentials() {
    let old_id = jwt(serde_json::json!({
        "https://api.openai.com/auth": { "chatgpt_account_id": "account-1" }
    }));
    let new_id = old_id.clone();
    let old_access = jwt(serde_json::json!({ "exp": 1 }));
    let new_access = jwt(serde_json::json!({ "exp": 4_000_000_000_u64 }));
    let client = Arc::new(ScriptedClient::new([response(
        200,
        &format!(
            r#"{{"id_token":"{new_id}","access_token":"{new_access}","refresh_token":"new-refresh"}}"#
        ),
    )]));
    let secrets = Arc::new(MemorySecretStore::default());
    let runtime = ChatGptOAuth::with_client(secrets, client.clone());
    runtime
        .store_credential(&TokenCredential {
            id_token: old_id,
            access_token: old_access,
            refresh_token: "old-refresh".into(),
            expires_at: Some(1),
            email: None,
            plan: None,
            user_id: None,
            account_id: Some("account-1".into()),
            is_fedramp: false,
            credential_revision: 7,
        })
        .unwrap();

    let target = runtime.api_target().unwrap();
    assert!(target.headers.iter().any(|header| {
        header.name() == "Authorization" && header.value() == format!("Bearer {new_access}")
    }));
    assert_eq!(
        runtime.read_account().unwrap().unwrap().credential_revision,
        8
    );
    let requests = client.requests.lock().unwrap();
    let body: serde_json::Value = serde_json::from_slice(requests[0].body()).unwrap();
    assert_eq!(body["grant_type"], "refresh_token");
    assert_eq!(body["refresh_token"], "old-refresh");
}

#[test]
fn logout_removes_only_the_local_chatgpt_envelope() {
    let client = Arc::new(ScriptedClient::new([]));
    let secrets = Arc::new(MemorySecretStore::default());
    let runtime = ChatGptOAuth::with_client(secrets.clone(), client);
    let id_token = jwt(serde_json::json!({}));
    let access_token = jwt(serde_json::json!({ "exp": 4_000_000_000_u64 }));
    runtime
        .store_credential(&TokenCredential {
            id_token,
            access_token,
            refresh_token: "refresh".into(),
            expires_at: Some(4_000_000_000),
            email: None,
            plan: None,
            user_id: None,
            account_id: None,
            is_fedramp: false,
            credential_revision: 1,
        })
        .unwrap();

    runtime
        .logout(&AccountRef {
            provider: OPENAI_CHATGPT_PROVIDER_ID.into(),
            account_id: "current".into(),
        })
        .unwrap();

    assert!(
        secrets
            .load(&ChatGptOAuth::credential_key())
            .unwrap()
            .is_none()
    );
}

fn jwt(payload: serde_json::Value) -> String {
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_vec(&payload).unwrap());
    format!("e30.{payload}.signature")
}

fn response(status: u16, body: &str) -> ClientResponse {
    HttpResponse::new(status, Vec::new(), body.as_bytes().to_vec())
}
