//! Plugin contribution and product discovery integration for Connector snapshots.

mod auth;
mod authority;
mod command;
mod oauth;
mod projection;

pub use auth::ConnectorApiTokenConnectRequest;
pub use auth::ConnectorCredentialCleanup;
pub use auth::ConnectorCredentialService;
pub use auth::ConnectorCredentialServiceError;
pub use auth::ConnectorCredentialServiceErrorKind;
pub use auth::ConnectorDisconnectResult;
pub use authority::ConnectorAuthority;
pub use authority::ConnectorAuthoritySubscription;
pub use command::ConnectorAuthorityCommand;
pub use command::ConnectorAuthorityError;
pub use command::ConnectorAuthorityErrorKind;
pub use command::ConnectorCommandDisposition;
pub use command::ConnectorCommandId;
pub use command::ConnectorCommandRequest;
pub use command::ConnectorCommandResult;
pub use oauth::ConnectorOAuthAuthorization;
pub use oauth::ConnectorOAuthChallenge;
pub use oauth::ConnectorOAuthCompleteRequest;
pub use oauth::ConnectorOAuthCredential;
pub use oauth::ConnectorOAuthError;
pub use oauth::ConnectorOAuthErrorKind;
pub use oauth::ConnectorOAuthExchangeRequest;
pub use oauth::ConnectorOAuthFlowId;
pub use oauth::ConnectorOAuthProvider;
pub use oauth::ConnectorOAuthService;
pub use oauth::ConnectorOAuthStartRequest;
pub use projection::ConnectorCatalog;
pub use projection::ConnectorCatalogError;

#[cfg(test)]
#[path = "projection_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "authority_tests.rs"]
mod authority_tests;

#[cfg(test)]
#[path = "auth_tests.rs"]
mod auth_tests;
