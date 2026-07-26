# Model provider configuration

The canonical architecture and evolution plan is
[`docs/model-provider-config.md`](../../docs/model-provider-config.md).

This crate owns Zeta's declarative, serializable model-provider configuration. It contains no
transport, API client, credentials, or other process-local runtime state.

Provider definitions declare adapter identities, endpoint defaults, model catalogs, normalization
rules, and non-secret defaults. `ProviderConfigRegistry` validates and merges those definitions,
then normalizes a `ModelProviderConfig` before the sibling `zeta-model-provider` crate creates a
runnable provider.
