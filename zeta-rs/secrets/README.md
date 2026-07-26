# zeta-secrets

Provider-neutral secret persistence primitives for Zeta.

The canonical ownership and backend policy is documented in
[`docs/secrets.md`](../../docs/secrets.md). This crate deliberately does not implement provider
authentication, OAuth, token refresh, request headers, account selection, or App Server login
flows.

The initial implementation provides:

- validated, opaque `SecretKey` identities;
- redacted and zeroed-on-drop `SecretValue` values;
- the narrow `SecretStore::load/store/delete` port;
- an in-memory backend for ephemeral hosts and tests;
- an unavailable backend for hosts without a configured secure facility.

Production OS keyring and explicitly enabled encrypted/file backends should be added as sibling
modules only when their complete load/store/delete behavior and negative logging tests are
implemented.
