use crate::credential::RefreshRequest;
use crate::credential::RefreshResponse;
use crate::credential::TokenCredential;
use crate::device_flow;
use serde_json::to_vec;
use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::Weak;
use std::thread;
use zeta_async_utils::CancellationSource;
use zeta_client::ClientRequest;
use zeta_client::OperationClient;
use zeta_client::ResolvedApiTarget;
use zeta_client::RetryPolicy;
use zeta_client::ZetaClient;
use zeta_http_client::HttpHeader;
use zeta_http_client::UreqHttpClient;
use zeta_login::AccountRef;
use zeta_login::AccountSnapshot;
use zeta_login::AccountStatus;
use zeta_login::BeginLogin;
use zeta_login::BeginLoginRequest;
use zeta_login::CancelLoginOutcome;
use zeta_login::CompleteLogin;
use zeta_login::InteractiveLoginDriver;
use zeta_login::LoginCompletionOutcome;
use zeta_login::LoginError;
use zeta_login::LoginErrorKind;
use zeta_login::LoginFailure;
use zeta_login::LoginId;
use zeta_login::LoginMethod;
use zeta_login::LoginService;
use zeta_secrets::SecretKey;
use zeta_secrets::SecretStore;
use zeta_secrets::SecretValue;

pub const OPENAI_CHATGPT_PROVIDER_ID: &str = "openai-chatgpt";
pub const CHATGPT_RESPONSES_BASE_URL: &str = "https://chatgpt.com/backend-api/codex";

pub(crate) const AUTH_BASE_URL: &str = "https://auth.openai.com";
pub(crate) const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const CREDENTIAL_KEY: &str = "provider/openai-chatgpt/current/oauth";

/// Sanitized ChatGPT OAuth or credential failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChatGptError {
    message: String,
}

impl ChatGptError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ChatGptError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ChatGptError {}

/// Owns native ChatGPT device OAuth, local token lifecycle, and request-time auth headers.
pub struct ChatGptOAuth {
    client: Arc<dyn OperationClient>,
    secrets: Arc<dyn SecretStore>,
    self_weak: Weak<Self>,
    login_service: Mutex<Weak<LoginService>>,
    active: Mutex<BTreeMap<LoginId, CancellationSource>>,
    refresh: Mutex<()>,
}

impl ChatGptOAuth {
    pub fn production(secrets: Arc<dyn SecretStore>) -> Result<Arc<Self>, ChatGptError> {
        let transport = UreqHttpClient::new()
            .map_err(|_| ChatGptError::new("ChatGPT HTTPS transport is unavailable"))?;
        Ok(Self::with_client(
            secrets,
            Arc::new(ZetaClient::new(Arc::new(transport))),
        ))
    }

    pub fn with_client(
        secrets: Arc<dyn SecretStore>,
        client: Arc<dyn OperationClient>,
    ) -> Arc<Self> {
        Arc::new_cyclic(|self_weak| Self {
            client,
            secrets,
            self_weak: self_weak.clone(),
            login_service: Mutex::new(Weak::new()),
            active: Mutex::new(BTreeMap::new()),
            refresh: Mutex::new(()),
        })
    }

    /// Connects asynchronous device-flow completion to the shared login control plane.
    pub fn install_login_service(&self, service: &Arc<LoginService>) -> Result<(), LoginError> {
        *self.login_service.lock().map_err(login_lock_error)? = Arc::downgrade(service);
        Ok(())
    }

    /// Resolves fresh ChatGPT subscription credentials for one Responses invocation.
    pub fn api_target(&self) -> Result<ResolvedApiTarget, ChatGptError> {
        let _refresh = self
            .refresh
            .lock()
            .map_err(|_| ChatGptError::new("ChatGPT credential refresh state is unavailable"))?;
        let mut credential = self
            .load_credential()?
            .ok_or_else(|| ChatGptError::new("ChatGPT is not signed in"))?;
        if credential.needs_refresh() {
            self.refresh_token(&mut credential)?;
            self.store_credential(&credential)?;
            self.publish_account_update(&credential);
        }
        Ok(ResolvedApiTarget::new(
            CHATGPT_RESPONSES_BASE_URL,
            api_headers(&credential),
        ))
    }

    fn refresh_token(&self, credential: &mut TokenCredential) -> Result<(), ChatGptError> {
        if credential.refresh_token.trim().is_empty() {
            return Err(ChatGptError::new("ChatGPT sign-in has expired"));
        }
        let body = to_vec(&RefreshRequest {
            client_id: CLIENT_ID,
            grant_type: "refresh_token",
            refresh_token: &credential.refresh_token,
        })
        .map_err(|_| ChatGptError::new("ChatGPT token refresh could not be encoded"))?;
        let request = ClientRequest::post(
            format!("{AUTH_BASE_URL}/oauth/token"),
            vec![
                HttpHeader::new("Content-Type", "application/json"),
                HttpHeader::new("Accept", "application/json"),
                HttpHeader::new("User-Agent", user_agent()),
            ],
            body,
            RetryPolicy::never(),
        )
        .map_err(|_| ChatGptError::new("ChatGPT token refresh could not be constructed"))?;
        let cancellation = CancellationSource::new();
        let response = self
            .client
            .execute_with_cancellation(&request, &cancellation.token())
            .map_err(|_| ChatGptError::new("ChatGPT OAuth service is unavailable"))?;
        if !response.is_success() {
            return Err(ChatGptError::new(format!(
                "ChatGPT token refresh failed with HTTP {}",
                response.status()
            )));
        }
        let response: RefreshResponse = serde_json::from_slice(response.body())
            .map_err(|_| ChatGptError::new("OpenAI returned an invalid token refresh response"))?;
        credential.apply_refresh(response)
    }

    pub(crate) fn credential_key() -> SecretKey {
        SecretKey::new(CREDENTIAL_KEY).expect("static ChatGPT credential key is valid")
    }

    pub(crate) fn load_credential(&self) -> Result<Option<TokenCredential>, ChatGptError> {
        self.secrets
            .load(&Self::credential_key())
            .map_err(|_| ChatGptError::new("ChatGPT credential store is unavailable"))?
            .map(|value| {
                serde_json::from_slice(value.expose())
                    .map_err(|_| ChatGptError::new("stored ChatGPT credential is invalid"))
            })
            .transpose()
    }

    pub(crate) fn store_credential(
        &self,
        credential: &TokenCredential,
    ) -> Result<(), ChatGptError> {
        let encoded = serde_json::to_vec(credential)
            .map_err(|_| ChatGptError::new("ChatGPT credential could not be encoded"))?;
        self.secrets
            .store(&Self::credential_key(), &SecretValue::new(encoded))
            .map_err(|_| ChatGptError::new("ChatGPT credential store is unavailable"))
    }

    fn account_snapshot(&self, credential: &TokenCredential) -> AccountSnapshot {
        AccountSnapshot {
            account: AccountRef {
                provider: OPENAI_CHATGPT_PROVIDER_ID.into(),
                account_id: credential
                    .account_id
                    .clone()
                    .unwrap_or_else(|| "current".into()),
            },
            email: credential.email.clone(),
            display_name: Some("ChatGPT".into()),
            organization: credential.account_id.clone(),
            plan: credential.plan.clone(),
            status: if credential.is_usable() {
                AccountStatus::Ready
            } else {
                AccountStatus::ReauthenticationRequired
            },
            credential_revision: credential.credential_revision,
        }
    }

    fn finish_login(&self, login_id: LoginId, outcome: LoginCompletionOutcome) {
        let active = self
            .active
            .lock()
            .map(|mut active| active.remove(&login_id).is_some())
            .unwrap_or(false);
        if !active {
            return;
        }
        let service = self
            .login_service
            .lock()
            .ok()
            .and_then(|service| service.upgrade());
        if let Some(service) = service {
            let _ = service.complete(CompleteLogin { login_id, outcome });
        }
    }

    fn publish_account_update(&self, credential: &TokenCredential) {
        let service = self
            .login_service
            .lock()
            .ok()
            .and_then(|service| service.upgrade());
        if let Some(service) = service {
            let _ = service.update_account(self.account_snapshot(credential));
        }
    }
}

impl InteractiveLoginDriver for ChatGptOAuth {
    fn provider_id(&self) -> &'static str {
        OPENAI_CHATGPT_PROVIDER_ID
    }

    fn read_account(&self) -> Result<Option<AccountSnapshot>, LoginError> {
        self.load_credential()
            .map(|credential| credential.map(|value| self.account_snapshot(&value)))
            .map_err(login_driver_error)
    }

    fn begin(&self, request: BeginLoginRequest) -> Result<BeginLogin, LoginError> {
        if !matches!(
            request.method,
            LoginMethod::OpenAiChatGptBrowser | LoginMethod::OpenAiChatGptDeviceCode
        ) {
            return Err(LoginError::new(
                LoginErrorKind::InvalidInput,
                "login method is not owned by ChatGPT",
            ));
        }
        if !self.active.lock().map_err(login_lock_error)?.is_empty() {
            return Err(LoginError::new(
                LoginErrorKind::Conflict,
                "a ChatGPT login is already active",
            ));
        }
        let device =
            device_flow::request_device_code(self.client.as_ref()).map_err(login_driver_error)?;
        let cancellation = CancellationSource::new();
        self.active
            .lock()
            .map_err(login_lock_error)?
            .insert(request.login_id.clone(), cancellation.clone());
        let weak = self.self_weak.clone();
        let login_id = request.login_id.clone();
        let worker_device = device.clone();
        thread::spawn(move || {
            let Some(runtime) = weak.upgrade() else {
                return;
            };
            let result = device_flow::complete_device_login(
                runtime.client.as_ref(),
                &worker_device,
                &cancellation.token(),
            )
            .and_then(|tokens| TokenCredential::from_tokens(tokens, 1));
            if cancellation.token().is_cancelled() {
                return;
            }
            match result {
                Ok(credential) => {
                    let completion = match runtime.store_credential(&credential) {
                        Ok(()) => LoginCompletionOutcome::Succeeded {
                            account: runtime.account_snapshot(&credential),
                        },
                        Err(error) => LoginCompletionOutcome::Failed {
                            failure: LoginFailure {
                                code: "credential_store_failed".into(),
                                message: error.to_string(),
                            },
                        },
                    };
                    runtime.finish_login(login_id, completion);
                }
                Err(error) => runtime.finish_login(
                    login_id,
                    LoginCompletionOutcome::Failed {
                        failure: LoginFailure {
                            code: "oauth_failed".into(),
                            message: error.to_string(),
                        },
                    },
                ),
            }
        });
        Ok(BeginLogin::DeviceCode {
            login_id: request.login_id,
            verification_url: device.verification_url,
            user_code: device.user_code,
        })
    }

    fn cancel(&self, login_id: &LoginId) -> Result<CancelLoginOutcome, LoginError> {
        let source = self
            .active
            .lock()
            .map_err(login_lock_error)?
            .remove(login_id);
        let Some(source) = source else {
            return Ok(CancelLoginOutcome::NotFound);
        };
        source.cancel();
        Ok(CancelLoginOutcome::Cancelled)
    }

    fn logout(&self, account: &AccountRef) -> Result<(), LoginError> {
        if account.provider != OPENAI_CHATGPT_PROVIDER_ID {
            return Err(LoginError::new(
                LoginErrorKind::InvalidInput,
                "account is not owned by the ChatGPT login driver",
            ));
        }
        self.secrets
            .delete(&Self::credential_key())
            .map(|_| ())
            .map_err(|_| {
                LoginError::new(
                    LoginErrorKind::Unavailable,
                    "ChatGPT credential store is unavailable",
                )
            })
    }
}

fn api_headers(credential: &TokenCredential) -> Vec<HttpHeader> {
    let mut headers = vec![
        HttpHeader::new(
            "Authorization",
            format!("Bearer {}", credential.access_token),
        ),
        HttpHeader::new("Originator", "zeta"),
        HttpHeader::new("User-Agent", user_agent()),
    ];
    if let Some(account_id) = &credential.account_id {
        headers.push(HttpHeader::new("ChatGPT-Account-ID", account_id));
    }
    if credential.is_fedramp {
        headers.push(HttpHeader::new("X-OpenAI-Fedramp", "true"));
    }
    headers
}

pub(crate) fn user_agent() -> String {
    format!("Zeta/{}", env!("CARGO_PKG_VERSION"))
}

fn login_driver_error(error: ChatGptError) -> LoginError {
    LoginError::new(LoginErrorKind::Driver, error.to_string())
}

fn login_lock_error<T>(_: std::sync::PoisonError<T>) -> LoginError {
    LoginError::new(
        LoginErrorKind::Unavailable,
        "ChatGPT login state is unavailable",
    )
}
