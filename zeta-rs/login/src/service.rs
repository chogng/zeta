use crate::AccountState;
use crate::BeginLogin;
use crate::BeginLoginRequest;
use crate::CancelLoginOutcome;
use crate::CompleteLogin;
use crate::InteractiveLoginDriver;
use crate::LoginCompletion;
use crate::LoginCompletionOutcome;
use crate::LoginError;
use crate::LoginErrorKind;
use crate::LoginEvents;
use crate::LoginId;
use crate::LoginMethod;
use crate::LogoutOutcome;
use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

/// Coordinates stable login identities and revisioned redacted account state.
pub struct LoginService {
    driver: Arc<dyn InteractiveLoginDriver>,
    next_login_id: AtomicU64,
    state: Mutex<LoginServiceState>,
    events: Mutex<Option<Arc<dyn LoginEvents>>>,
}

struct LoginServiceState {
    account: AccountState,
    active_logins: BTreeSet<LoginId>,
    initialized: bool,
}

impl LoginService {
    pub fn new(driver: Arc<dyn InteractiveLoginDriver>) -> Result<Self, LoginError> {
        let account = driver.read_account()?;
        Ok(Self::from_account(driver, account, true))
    }

    /// Creates a control plane without performing provider I/O during composition.
    ///
    /// Product hosts should use this for lazy process-backed drivers and call
    /// [`Self::refresh`] when the first account projection is requested.
    pub fn deferred(driver: Arc<dyn InteractiveLoginDriver>) -> Self {
        Self::from_account(driver, None, false)
    }

    fn from_account(
        driver: Arc<dyn InteractiveLoginDriver>,
        account: Option<crate::AccountSnapshot>,
        initialized: bool,
    ) -> Self {
        Self {
            driver,
            next_login_id: AtomicU64::new(1),
            state: Mutex::new(LoginServiceState {
                account: AccountState {
                    revision: u64::from(account.is_some()),
                    account,
                },
                active_logins: BTreeSet::new(),
                initialized,
            }),
            events: Mutex::new(None),
        }
    }

    pub fn install_events(&self, events: Arc<dyn LoginEvents>) -> Result<(), LoginError> {
        *self.events.lock().map_err(lock_error)? = Some(events);
        Ok(())
    }

    pub fn read(&self) -> Result<AccountState, LoginError> {
        Ok(self.state.lock().map_err(lock_error)?.account.clone())
    }

    /// Reads the canonical projection, performing deferred provider I/O once.
    pub fn read_or_refresh(&self) -> Result<AccountState, LoginError> {
        if self.state.lock().map_err(lock_error)?.initialized {
            self.read()
        } else {
            self.refresh()
        }
    }

    pub fn begin(&self, method: LoginMethod) -> Result<BeginLogin, LoginError> {
        let sequence = self.next_login_id.fetch_add(1, Ordering::Relaxed);
        let login_id = LoginId::new(format!("login-{sequence:016x}"))?;
        self.state
            .lock()
            .map_err(lock_error)?
            .active_logins
            .insert(login_id.clone());
        let started = match self.driver.begin(BeginLoginRequest {
            login_id: login_id.clone(),
            method,
        }) {
            Ok(started) => started,
            Err(error) => {
                self.state
                    .lock()
                    .map_err(lock_error)?
                    .active_logins
                    .remove(&login_id);
                return Err(error);
            }
        };
        if started.login_id() != &login_id {
            let _ = self.driver.cancel(&login_id);
            self.state
                .lock()
                .map_err(lock_error)?
                .active_logins
                .remove(&login_id);
            return Err(LoginError::new(
                LoginErrorKind::Driver,
                "interactive login driver changed the assigned login ID",
            ));
        }
        Ok(started)
    }

    pub fn cancel(&self, login_id: &LoginId) -> Result<CancelLoginOutcome, LoginError> {
        let active = self
            .state
            .lock()
            .map_err(lock_error)?
            .active_logins
            .contains(login_id);
        if !active {
            return Ok(CancelLoginOutcome::NotFound);
        }
        let outcome = self.driver.cancel(login_id)?;
        if matches!(
            outcome,
            CancelLoginOutcome::Cancelled | CancelLoginOutcome::NotFound
        ) {
            self.state
                .lock()
                .map_err(lock_error)?
                .active_logins
                .remove(login_id);
        }
        Ok(outcome)
    }

    pub fn complete(&self, completion: CompleteLogin) -> Result<(), LoginError> {
        let event = {
            let mut state = self.state.lock().map_err(lock_error)?;
            if !state.active_logins.remove(&completion.login_id) {
                return Err(LoginError::new(
                    LoginErrorKind::NotFound,
                    "login attempt is not active",
                ));
            }
            if let LoginCompletionOutcome::Succeeded { account } = &completion.outcome {
                state.account.revision = state.account.revision.saturating_add(1);
                state.account.account = Some(account.clone());
            }
            state.initialized = true;
            LoginCompletion {
                login_id: completion.login_id,
                outcome: completion.outcome,
                account_state: state.account.clone(),
            }
        };
        if let Some(events) = self.events()? {
            events.login_completed(event.clone());
            if matches!(event.outcome, LoginCompletionOutcome::Succeeded { .. }) {
                events.account_updated(event.account_state);
            }
        }
        Ok(())
    }

    pub fn refresh(&self) -> Result<AccountState, LoginError> {
        let account = self.driver.read_account()?;
        let updated = {
            let mut state = self.state.lock().map_err(lock_error)?;
            state.initialized = true;
            if state.account.account == account {
                None
            } else {
                state.account.revision = state.account.revision.saturating_add(1);
                state.account.account = account;
                Some(state.account.clone())
            }
        };
        if let Some(updated) = updated {
            if let Some(events) = self.events()? {
                events.account_updated(updated.clone());
            }
            Ok(updated)
        } else {
            self.read()
        }
    }

    pub fn logout(&self) -> Result<LogoutOutcome, LoginError> {
        let account = self.read_or_refresh()?.account;
        let Some(account) = account else {
            return Ok(LogoutOutcome::AlreadyLoggedOut);
        };
        self.driver.logout(&account.account)?;
        let updated = {
            let mut state = self.state.lock().map_err(lock_error)?;
            if state.account.account.is_none() {
                return Ok(LogoutOutcome::LoggedOut);
            }
            state.account.revision = state.account.revision.saturating_add(1);
            state.account.account = None;
            state.account.clone()
        };
        if let Some(events) = self.events()? {
            events.account_updated(updated);
        }
        Ok(LogoutOutcome::LoggedOut)
    }

    fn events(&self) -> Result<Option<Arc<dyn LoginEvents>>, LoginError> {
        Ok(self.events.lock().map_err(lock_error)?.clone())
    }
}

fn lock_error<T>(_: std::sync::PoisonError<T>) -> LoginError {
    LoginError::new(LoginErrorKind::Unavailable, "login state lock poisoned")
}
