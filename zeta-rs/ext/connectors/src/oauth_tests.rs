use std::sync::Arc;
use std::sync::Mutex;

use url::Url;
use zeta_connectors::ConnectorAccountId;
use zeta_connectors::ConnectorConnectionGeneration;
use zeta_connectors::ConnectorConnectionState;
use zeta_connectors::ConnectorDefinition;
use zeta_connectors::ConnectorId;
use zeta_connectors::ConnectorRuntimeBinding;
use zeta_secrets::MemorySecretStore;
use zeta_secrets::SecretKey;
use zeta_secrets::SecretStore;
use zeta_secrets::SecretValue;

use super::*;
use crate::ConnectorAuthority;

struct TestProvider {
    verifier: Mutex<Option<String>>,
}

impl ConnectorOAuthProvider for TestProvider {
    fn authorization_url(
        &self,
        _: &ConnectorDefinition,
        challenge: ConnectorOAuthChallenge<'_>,
    ) -> Result<String, ConnectorOAuthError> {
        let mut url = Url::parse("https://accounts.example.test/authorize").unwrap();
        url.query_pairs_mut()
            .append_pair("state", challenge.state)
            .append_pair("code_challenge", challenge.code_challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("redirect_uri", challenge.redirect_uri);
        Ok(url.to_string())
    }

    fn exchange(
        &self,
        _: &ConnectorDefinition,
        request: ConnectorOAuthExchangeRequest<'_>,
    ) -> Result<ConnectorOAuthCredential, ConnectorOAuthError> {
        assert_eq!(request.authorization_code.expose(), b"authorization-code");
        assert_eq!(request.redirect_uri, "http://127.0.0.1:49152/callback");
        *self.verifier.lock().unwrap() = Some(request.pkce_verifier.to_string());
        Ok(ConnectorOAuthCredential {
            account_id: ConnectorAccountId::new("octocat").unwrap(),
            account_display_name: "Octocat".into(),
            secret: SecretValue::new(b"access-and-refresh-token-bundle".to_vec()),
        })
    }
}

fn fixture() -> (
    ConnectorOAuthService,
    ConnectorId,
    Arc<MemorySecretStore>,
    Arc<TestProvider>,
) {
    let connector_id = ConnectorId::new("acme/github:connector:account").unwrap();
    let definition = ConnectorDefinition::new(
        connector_id.clone(),
        "GitHub",
        "Connect GitHub.",
        ConnectorRuntimeBinding::mcp_server("plugin:acme/github:mcp:github").unwrap(),
    )
    .unwrap();
    let authority = ConnectorAuthority::in_memory([definition]).unwrap();
    let secrets = Arc::new(MemorySecretStore::default());
    let credentials = Arc::new(ConnectorCredentialService::new(authority, secrets.clone()));
    let provider = Arc::new(TestProvider {
        verifier: Mutex::new(None),
    });
    let provider_port: Arc<dyn ConnectorOAuthProvider> = provider.clone();
    let oauth = ConnectorOAuthService::new(credentials, [(connector_id.clone(), provider_port)]);
    (oauth, connector_id, secrets, provider)
}

#[test]
fn oauth_pkce_callback_publishes_only_provider_validated_account_and_secret_reference() {
    let (oauth, connector_id, secrets, provider) = fixture();
    let expected_generation = oauth.credentials.authority().snapshot().generation();
    let authorization = oauth
        .start(ConnectorOAuthStartRequest {
            command_id: ConnectorCommandId::new("oauth-github").unwrap(),
            expected_generation,
            connector_id: connector_id.clone(),
            connection_generation: ConnectorConnectionGeneration::new(1),
            redirect_uri: "http://127.0.0.1:49152/callback".into(),
        })
        .unwrap();
    assert!(matches!(
        oauth
            .credentials
            .authority()
            .snapshot()
            .entry(&connector_id)
            .unwrap()
            .connection()
            .state(),
        ConnectorConnectionState::Connecting
    ));
    let state = Url::parse(&authorization.authorization_url)
        .unwrap()
        .query_pairs()
        .find(|(key, _)| key == "state")
        .unwrap()
        .1
        .into_owned();

    oauth
        .complete(ConnectorOAuthCompleteRequest {
            flow_id: authorization.flow_id,
            state: SecretValue::new(state.into_bytes()),
            authorization_code: SecretValue::new(b"authorization-code".to_vec()),
        })
        .unwrap();

    assert!(provider.verifier.lock().unwrap().is_some());
    let snapshot = oauth.credentials.authority().snapshot();
    let account = match snapshot.entry(&connector_id).unwrap().connection().state() {
        ConnectorConnectionState::Connected(account) => account,
        state => panic!("expected connected account, got {state:?}"),
    };
    assert_eq!(account.account_id().as_str(), "octocat");
    let key = SecretKey::new(account.credential_reference().as_str().to_string()).unwrap();
    assert_eq!(
        secrets.load(&key).unwrap().unwrap().expose(),
        b"access-and-refresh-token-bundle"
    );
}

#[test]
fn oauth_state_mismatch_consumes_attempt_and_revokes_readiness() {
    let (oauth, connector_id, _, _) = fixture();
    let authorization = oauth
        .start(ConnectorOAuthStartRequest {
            command_id: ConnectorCommandId::new("oauth-state-mismatch").unwrap(),
            expected_generation: oauth.credentials.authority().snapshot().generation(),
            connector_id: connector_id.clone(),
            connection_generation: ConnectorConnectionGeneration::new(1),
            redirect_uri: "http://localhost:49152/callback".into(),
        })
        .unwrap();

    let error = oauth
        .complete(ConnectorOAuthCompleteRequest {
            flow_id: authorization.flow_id,
            state: SecretValue::new(b"wrong-state".to_vec()),
            authorization_code: SecretValue::new(b"authorization-code".to_vec()),
        })
        .unwrap_err();

    assert_eq!(error.kind(), ConnectorOAuthErrorKind::StateMismatch);
    assert!(matches!(
        oauth
            .credentials
            .authority()
            .snapshot()
            .entry(&connector_id)
            .unwrap()
            .connection()
            .state(),
        ConnectorConnectionState::Unavailable { .. }
    ));
}

#[test]
fn oauth_cancel_consumes_the_exact_attempt_and_revokes_connecting_state() {
    let (oauth, connector_id, _, _) = fixture();
    let authorization = oauth
        .start(ConnectorOAuthStartRequest {
            command_id: ConnectorCommandId::new("oauth-cancel").unwrap(),
            expected_generation: oauth.credentials.authority().snapshot().generation(),
            connector_id: connector_id.clone(),
            connection_generation: ConnectorConnectionGeneration::new(1),
            redirect_uri: "http://127.0.0.1:49152/callback".into(),
        })
        .unwrap();

    oauth.cancel(&authorization.flow_id).unwrap();

    assert!(matches!(
        oauth
            .credentials
            .authority()
            .snapshot()
            .entry(&connector_id)
            .unwrap()
            .connection()
            .state(),
        ConnectorConnectionState::Unavailable { .. }
    ));
    assert_eq!(
        oauth.cancel(&authorization.flow_id).unwrap_err().kind(),
        ConnectorOAuthErrorKind::InvalidRequest
    );
}

#[test]
fn oauth_callback_rejects_a_changed_connector_definition_before_exchange() {
    let (oauth, connector_id, _, provider) = fixture();
    let authorization = oauth
        .start(ConnectorOAuthStartRequest {
            command_id: ConnectorCommandId::new("oauth-definition-change").unwrap(),
            expected_generation: oauth.credentials.authority().snapshot().generation(),
            connector_id: connector_id.clone(),
            connection_generation: ConnectorConnectionGeneration::new(1),
            redirect_uri: "http://127.0.0.1:49152/callback".into(),
        })
        .unwrap();
    oauth
        .credentials
        .authority()
        .reconcile_definitions([ConnectorDefinition::new(
            connector_id,
            "GitHub changed",
            "Connect GitHub.",
            ConnectorRuntimeBinding::mcp_server("plugin:acme/github:mcp:github-v2").unwrap(),
        )
        .unwrap()])
        .unwrap();
    let state = Url::parse(&authorization.authorization_url)
        .unwrap()
        .query_pairs()
        .find(|(key, _)| key == "state")
        .unwrap()
        .1
        .into_owned();

    let error = oauth
        .complete(ConnectorOAuthCompleteRequest {
            flow_id: authorization.flow_id,
            state: SecretValue::new(state.into_bytes()),
            authorization_code: SecretValue::new(b"authorization-code".to_vec()),
        })
        .unwrap_err();

    assert_eq!(error.kind(), ConnectorOAuthErrorKind::InvalidRequest);
    assert!(provider.verifier.lock().unwrap().is_none());
}

#[test]
fn authorization_url_rejects_duplicate_security_parameters_and_fragments() {
    let error = validate_authorization_url(
        "https://accounts.example.test/authorize?state=wrong&state=right&code_challenge=challenge&code_challenge_method=S256&redirect_uri=https%3A%2F%2Fapp.example.test%2Fcallback",
        "right",
        "challenge",
        "https://app.example.test/callback",
    )
    .unwrap_err();
    assert_eq!(error.kind(), ConnectorOAuthErrorKind::ProviderFailure);

    let error = validate_authorization_url(
        "https://accounts.example.test/authorize?state=right&code_challenge=challenge&code_challenge_method=S256&redirect_uri=https%3A%2F%2Fapp.example.test%2Fcallback#token",
        "right",
        "challenge",
        "https://app.example.test/callback",
    )
    .unwrap_err();
    assert_eq!(error.kind(), ConnectorOAuthErrorKind::ProviderFailure);
}
