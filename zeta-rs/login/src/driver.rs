use crate::AccountRef;
use crate::AccountSnapshot;
use crate::AccountState;
use crate::BeginLogin;
use crate::BeginLoginRequest;
use crate::CancelLoginOutcome;
use crate::LoginCompletion;
use crate::LoginError;
use crate::LoginId;

/// Executes provider-owned interactive login operations without exposing credentials.
///
/// Implementations keep provider credentials, OAuth protocol state, callbacks,
/// and persistence private. They may return only redacted account metadata and
/// user-facing browser or device-code instructions.
pub trait InteractiveLoginDriver: Send + Sync {
    /// Returns the stable provider identity owned by this driver.
    fn provider_id(&self) -> &'static str;

    fn read_account(&self) -> Result<Option<AccountSnapshot>, LoginError>;

    fn begin(&self, request: BeginLoginRequest) -> Result<BeginLogin, LoginError>;

    fn cancel(&self, login_id: &LoginId) -> Result<CancelLoginOutcome, LoginError>;

    fn logout(&self, account: &AccountRef) -> Result<(), LoginError>;
}

/// Receives redacted login lifecycle events from [`crate::LoginService`].
///
/// Product hosts should convert these values to their own notification
/// protocol without adding credential or provider-transport details.
pub trait LoginEvents: Send + Sync {
    fn login_completed(&self, completion: LoginCompletion);

    fn account_updated(&self, state: AccountState);
}
