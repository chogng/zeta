use super::*;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

#[derive(Default)]
struct FakeDriver {
    account: Mutex<Option<AccountSnapshot>>,
    active: Mutex<Vec<LoginId>>,
    reads: AtomicUsize,
}

impl InteractiveLoginDriver for FakeDriver {
    fn read_account(&self) -> Result<Option<AccountSnapshot>, LoginError> {
        self.reads.fetch_add(1, Ordering::Relaxed);
        Ok(self.account.lock().unwrap().clone())
    }

    fn begin(&self, request: BeginLoginRequest) -> Result<BeginLogin, LoginError> {
        self.active.lock().unwrap().push(request.login_id.clone());
        Ok(match request.method {
            LoginMethod::OpenAiChatGptBrowser => BeginLogin::Browser {
                login_id: request.login_id,
                authorization_url: "https://auth.example.test/start".into(),
            },
            LoginMethod::OpenAiChatGptDeviceCode => BeginLogin::DeviceCode {
                login_id: request.login_id,
                verification_url: "https://auth.example.test/device".into(),
                user_code: "ABCD-EFGH".into(),
            },
        })
    }

    fn cancel(&self, login_id: &LoginId) -> Result<CancelLoginOutcome, LoginError> {
        let mut active = self.active.lock().unwrap();
        let Some(index) = active.iter().position(|candidate| candidate == login_id) else {
            return Ok(CancelLoginOutcome::NotFound);
        };
        active.remove(index);
        Ok(CancelLoginOutcome::Cancelled)
    }

    fn logout(&self, _: &AccountRef) -> Result<(), LoginError> {
        *self.account.lock().unwrap() = None;
        Ok(())
    }
}

#[derive(Default)]
struct RecordedEvents {
    completions: Mutex<Vec<LoginCompletion>>,
    accounts: Mutex<Vec<AccountState>>,
}

impl LoginEvents for RecordedEvents {
    fn login_completed(&self, completion: LoginCompletion) {
        self.completions.lock().unwrap().push(completion);
    }

    fn account_updated(&self, state: AccountState) {
        self.accounts.lock().unwrap().push(state);
    }
}

fn account() -> AccountSnapshot {
    AccountSnapshot {
        account: AccountRef {
            provider: "openai-chatgpt".into(),
            account_id: "acct_redacted".into(),
        },
        email: Some("person@example.test".into()),
        display_name: Some("Person".into()),
        organization: None,
        plan: Some("plus".into()),
        status: AccountStatus::Ready,
        credential_revision: 4,
    }
}

#[test]
fn browser_login_completion_updates_redacted_state_and_emits_events() {
    let driver = Arc::new(FakeDriver::default());
    let service = LoginService::new(driver).unwrap();
    let events = Arc::new(RecordedEvents::default());
    service.install_events(events.clone()).unwrap();

    let started = service.begin(LoginMethod::OpenAiChatGptBrowser).unwrap();
    let login_id = started.login_id().clone();
    service
        .complete(CompleteLogin {
            login_id,
            outcome: LoginCompletionOutcome::Succeeded { account: account() },
        })
        .unwrap();

    let state = service.read().unwrap();
    assert_eq!(state.revision, 1);
    assert_eq!(state.account, Some(account()));
    assert_eq!(events.completions.lock().unwrap().len(), 1);
    assert_eq!(events.accounts.lock().unwrap().as_slice(), &[state]);
}

#[test]
fn cancellation_is_scoped_to_one_active_login() {
    let service = LoginService::new(Arc::new(FakeDriver::default())).unwrap();
    let started = service.begin(LoginMethod::OpenAiChatGptDeviceCode).unwrap();

    assert_eq!(
        service.cancel(started.login_id()).unwrap(),
        CancelLoginOutcome::Cancelled
    );
    assert_eq!(
        service.cancel(started.login_id()).unwrap(),
        CancelLoginOutcome::NotFound
    );
}

#[test]
fn deferred_service_reads_the_provider_once_before_using_its_projection() {
    let driver = Arc::new(FakeDriver::default());
    *driver.account.lock().unwrap() = Some(account());
    let service = LoginService::deferred(driver.clone());

    assert_eq!(driver.reads.load(Ordering::Relaxed), 0);
    assert_eq!(service.read_or_refresh().unwrap().account, Some(account()));
    assert_eq!(service.read_or_refresh().unwrap().account, Some(account()));
    assert_eq!(driver.reads.load(Ordering::Relaxed), 1);
}
