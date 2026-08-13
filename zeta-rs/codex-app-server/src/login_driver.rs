use crate::CodexAppServerOptions;
use crate::CodexAppServerRuntime;
use crate::process::ProcessError;
use crate::process::ProcessErrorKind;
use crate::process::UpstreamEvent;
use crate::runtime::EventHandling;
use crate::runtime::UpstreamConnectionId;
use crate::runtime::UpstreamEventHandler;
use serde_json::Value;
use serde_json::json;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::Weak;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;
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

const PROVIDER_ID: &str = "openai-chatgpt";

/// Adapts the upstream Codex App Server's managed ChatGPT login contract.
///
/// The driver never reads Codex credential storage or receives token fields. It
/// translates only account metadata, authorization instructions, and lifecycle
/// notifications into the redacted [`zeta_login`] control plane.
pub struct CodexAppServerLoginDriver {
    runtime: Arc<CodexAppServerRuntime>,
    self_weak: Weak<Self>,
    login_service: Mutex<Weak<LoginService>>,
    logins: Mutex<LoginMappings>,
    credential_revision: AtomicU64,
}

#[derive(Default)]
struct LoginMappings {
    starting: bool,
    local_to_upstream: BTreeMap<LoginId, String>,
    upstream_to_local: BTreeMap<String, LoginId>,
    early_completions: BTreeMap<String, UpstreamCompletion>,
    pending_successes: BTreeSet<LoginId>,
}

#[derive(Clone, Copy)]
struct UpstreamCompletion {
    success: bool,
}

impl CodexAppServerLoginDriver {
    pub fn new(options: CodexAppServerOptions) -> Arc<Self> {
        Self::with_runtime(CodexAppServerRuntime::new(options))
    }

    pub fn with_runtime(runtime: Arc<CodexAppServerRuntime>) -> Arc<Self> {
        let driver = Arc::new_cyclic(|self_weak| Self {
            runtime: Arc::clone(&runtime),
            self_weak: self_weak.clone(),
            login_service: Mutex::new(Weak::new()),
            logins: Mutex::new(LoginMappings::default()),
            credential_revision: AtomicU64::new(1),
        });
        let handler: Arc<dyn UpstreamEventHandler> = driver.clone();
        runtime.install_handler(&handler);
        driver
    }

    /// Connects asynchronous upstream notifications to one control-plane service.
    ///
    /// The weak binding avoids a process-driver/service ownership cycle. Product
    /// composition must install it before exposing login commands.
    pub fn install_login_service(&self, service: &Arc<LoginService>) -> Result<(), LoginError> {
        *self.login_service.lock().map_err(lock_error)? = Arc::downgrade(service);
        Ok(())
    }

    fn request(&self, method: &str, params: Value) -> Result<Value, LoginError> {
        self.runtime.request(method, params).map_err(process_error)
    }

    fn request_without_params(&self, method: &str) -> Result<Value, LoginError> {
        self.runtime
            .request_without_params(method)
            .map_err(process_error)
    }

    fn handle_login_completed(&self, params: Value) {
        let Some(upstream_id) = params.get("loginId").and_then(Value::as_str) else {
            return;
        };
        let completion = UpstreamCompletion {
            success: params
                .get("success")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        };
        let local_id = {
            let Ok(mut logins) = self.logins.lock() else {
                return;
            };
            let Some(local_id) = logins.upstream_to_local.remove(upstream_id) else {
                logins
                    .early_completions
                    .insert(upstream_id.to_owned(), completion);
                return;
            };
            logins.local_to_upstream.remove(&local_id);
            local_id
        };
        self.complete_local_login(local_id, completion);
    }

    fn handle_account_updated(&self) {
        self.credential_revision.fetch_add(1, Ordering::Relaxed);
        let pending = self
            .logins
            .lock()
            .map(|mut logins| std::mem::take(&mut logins.pending_successes))
            .unwrap_or_default();
        if pending.is_empty() {
            if let Some(service) = self.login_service() {
                let _ = service.refresh();
            }
            return;
        }
        for login_id in pending {
            self.complete_successful_login(login_id);
        }
    }

    fn complete_successful_login(&self, login_id: LoginId) {
        let Some(service) = self.login_service() else {
            return;
        };
        let outcome = match self.read_account() {
            Ok(Some(account)) => LoginCompletionOutcome::Succeeded { account },
            Ok(None) | Err(_) => LoginCompletionOutcome::Failed {
                failure: LoginFailure {
                    code: "account_unavailable".into(),
                    message: "Codex login completed but no ChatGPT account was available".into(),
                },
            },
        };
        let _ = service.complete(CompleteLogin { login_id, outcome });
    }

    fn await_account_update(&self, login_id: LoginId) {
        if let Ok(mut logins) = self.logins.lock() {
            logins.pending_successes.insert(login_id.clone());
        } else {
            return;
        }
        let weak = self.self_weak.clone();
        let _ = thread::Builder::new()
            .name("zeta-codex-login-account-fallback".into())
            .spawn(move || {
                thread::sleep(Duration::from_secs(2));
                let Some(driver) = weak.upgrade() else {
                    return;
                };
                let pending = driver
                    .logins
                    .lock()
                    .map(|mut logins| logins.pending_successes.remove(&login_id))
                    .unwrap_or(false);
                if pending {
                    driver.complete_successful_login(login_id);
                }
            });
    }

    fn complete_failed_login(&self, login_id: LoginId) {
        if let Some(service) = self.login_service() {
            let _ = service.complete(CompleteLogin {
                login_id,
                outcome: LoginCompletionOutcome::Failed {
                    failure: LoginFailure {
                        code: "login_failed".into(),
                        message: "Codex login did not complete".into(),
                    },
                },
            });
        }
    }

    fn complete_local_login(&self, login_id: LoginId, completion: UpstreamCompletion) {
        if completion.success {
            self.await_account_update(login_id);
        } else {
            self.complete_failed_login(login_id);
        }
    }

    fn login_service(&self) -> Option<Arc<LoginService>> {
        self.login_service
            .lock()
            .ok()
            .and_then(|service| service.upgrade())
    }

    fn register_login(
        &self,
        local_id: LoginId,
        upstream_id: String,
    ) -> Result<Option<UpstreamCompletion>, LoginError> {
        let mut logins = self.logins.lock().map_err(lock_error)?;
        logins.starting = false;
        logins
            .local_to_upstream
            .insert(local_id.clone(), upstream_id.clone());
        logins
            .upstream_to_local
            .insert(upstream_id.clone(), local_id);
        Ok(logins.early_completions.remove(&upstream_id))
    }

    fn reset_starting(&self) {
        if let Ok(mut logins) = self.logins.lock() {
            logins.starting = false;
        }
    }

    fn begin_start(&self) -> Result<(), LoginError> {
        let mut logins = self.logins.lock().map_err(lock_error)?;
        if logins.starting || !logins.local_to_upstream.is_empty() {
            return Err(LoginError::new(
                LoginErrorKind::Conflict,
                "another Codex login attempt is already active",
            ));
        }
        logins.starting = true;
        Ok(())
    }
}

impl UpstreamEventHandler for CodexAppServerLoginDriver {
    fn handle_event(
        &self,
        _connection_id: UpstreamConnectionId,
        event: &UpstreamEvent,
    ) -> EventHandling {
        let UpstreamEvent::Notification { method, params } = event else {
            return EventHandling::Ignored;
        };
        match method.as_str() {
            "account/login/completed" => {
                self.handle_login_completed(params.clone());
                EventHandling::Handled
            }
            "account/updated" => {
                self.handle_account_updated();
                EventHandling::Handled
            }
            _ => EventHandling::Ignored,
        }
    }
}

impl InteractiveLoginDriver for CodexAppServerLoginDriver {
    fn read_account(&self) -> Result<Option<AccountSnapshot>, LoginError> {
        let response = self.request("account/read", json!({ "refreshToken": false }))?;
        let Some(account) = response.get("account") else {
            return Ok(None);
        };
        if account.is_null() || account.get("type").and_then(Value::as_str) != Some("chatgpt") {
            return Ok(None);
        }
        let email = account
            .get("email")
            .and_then(Value::as_str)
            .map(str::to_owned);
        Ok(Some(AccountSnapshot {
            account: AccountRef {
                provider: PROVIDER_ID.into(),
                account_id: "current".into(),
            },
            email,
            display_name: None,
            organization: None,
            plan: account
                .get("planType")
                .and_then(Value::as_str)
                .map(str::to_owned),
            status: AccountStatus::Ready,
            credential_revision: self.credential_revision.load(Ordering::Relaxed),
        }))
    }

    fn begin(&self, request: BeginLoginRequest) -> Result<BeginLogin, LoginError> {
        self.begin_start()?;
        let params = match request.method {
            LoginMethod::OpenAiChatGptBrowser => json!({ "type": "chatgpt" }),
            LoginMethod::OpenAiChatGptDeviceCode => json!({ "type": "chatgptDeviceCode" }),
        };
        let response = match self.request("account/login/start", params) {
            Ok(response) => response,
            Err(error) => {
                self.reset_starting();
                return Err(error);
            }
        };
        let parsed = (|| {
            let upstream_id = response
                .get("loginId")
                .and_then(Value::as_str)
                .ok_or_else(incompatible_response)?
                .to_owned();
            let started = match request.method {
                LoginMethod::OpenAiChatGptBrowser => BeginLogin::Browser {
                    login_id: request.login_id.clone(),
                    authorization_url: response
                        .get("authUrl")
                        .and_then(Value::as_str)
                        .ok_or_else(incompatible_response)?
                        .to_owned(),
                },
                LoginMethod::OpenAiChatGptDeviceCode => BeginLogin::DeviceCode {
                    login_id: request.login_id.clone(),
                    verification_url: response
                        .get("verificationUrl")
                        .and_then(Value::as_str)
                        .ok_or_else(incompatible_response)?
                        .to_owned(),
                    user_code: response
                        .get("userCode")
                        .and_then(Value::as_str)
                        .ok_or_else(incompatible_response)?
                        .to_owned(),
                },
            };
            Ok((upstream_id, started))
        })();
        let (upstream_id, started) = match parsed {
            Ok(parsed) => parsed,
            Err(error) => {
                self.reset_starting();
                return Err(error);
            }
        };
        let early = self.register_login(request.login_id.clone(), upstream_id)?;
        if let Some(early) = early {
            self.complete_local_login(request.login_id, early);
        }
        Ok(started)
    }

    fn cancel(&self, login_id: &LoginId) -> Result<CancelLoginOutcome, LoginError> {
        let upstream_id = self
            .logins
            .lock()
            .map_err(lock_error)?
            .local_to_upstream
            .get(login_id)
            .cloned();
        let Some(upstream_id) = upstream_id else {
            return Ok(CancelLoginOutcome::NotFound);
        };
        let response = self.request("account/login/cancel", json!({ "loginId": upstream_id }))?;
        let outcome = match response.get("status").and_then(Value::as_str) {
            Some("canceled" | "cancelled") => CancelLoginOutcome::Cancelled,
            Some("notFound" | "notfound") => CancelLoginOutcome::NotFound,
            _ => return Err(incompatible_response()),
        };
        let mut logins = self.logins.lock().map_err(lock_error)?;
        logins.local_to_upstream.remove(login_id);
        logins.upstream_to_local.remove(&upstream_id);
        Ok(outcome)
    }

    fn logout(&self, account: &AccountRef) -> Result<(), LoginError> {
        if account.provider != PROVIDER_ID {
            return Err(LoginError::new(
                LoginErrorKind::InvalidInput,
                "account is not owned by the Codex login driver",
            ));
        }
        self.request_without_params("account/logout")?;
        Ok(())
    }
}

fn process_error(error: ProcessError) -> LoginError {
    let kind = match error.kind {
        ProcessErrorKind::Unavailable => LoginErrorKind::Unavailable,
        ProcessErrorKind::Unsupported | ProcessErrorKind::Rejected => LoginErrorKind::Driver,
    };
    LoginError::new(kind, error.message)
}

fn incompatible_response() -> LoginError {
    LoginError::new(
        LoginErrorKind::Driver,
        "installed Codex App Server returned an incompatible account response",
    )
}

fn lock_error<T>(_: std::sync::PoisonError<T>) -> LoginError {
    LoginError::new(
        LoginErrorKind::Unavailable,
        "Codex login adapter state was unavailable",
    )
}
