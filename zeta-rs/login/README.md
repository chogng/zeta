# `zeta-login`

`zeta-login` owns Zeta's redacted interactive-account control plane. It assigns stable login IDs,
tracks active attempts, projects revisioned account state, and emits completion/account events.
Provider adapters retain OAuth protocol, callback, credential persistence, refresh, and logout I/O.

## Ownership and execution

`LoginService` is the state owner. `InteractiveLoginDriver` is its consumer-owned provider port;
`LoginEvents` is the product-host output port. The normal path is:

```text
LoginService::begin
→ InteractiveLoginDriver::begin(BeginLoginRequest { exact LoginId, method })
→ BeginLogin::Browser | BeginLogin::DeviceCode
→ provider adapter observes completion
→ LoginService::complete
→ LoginEvents::login_completed
→ LoginEvents::account_updated on success
```

`LoginService::cancel` only accepts an active exact `LoginId` and delegates idempotent cancellation.
`LoginService::logout` passes only `AccountRef` to the driver, then clears the redacted projection and
increments its revision. `refresh` reads a redacted driver snapshot and publishes only when it changed.

## Failure and security contract

`LoginErrorKind` supplies stable internal categories; product protocols map those categories without
forwarding provider payloads. Neither public types nor events contain access tokens, refresh tokens,
API keys, cookies, authorization codes, PKCE state, secret-store references, or credential paths.

The production ChatGPT/Codex implementation lives in `zeta-codex-app-server` and delegates managed
login to an upstream local Codex App Server. This crate remains provider-neutral and does not depend
on that adapter. Tests here use a fake driver to keep identity, cancellation, revision, and event
semantics independent of any provider process.
