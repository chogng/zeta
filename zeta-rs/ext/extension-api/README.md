# `zeta-extension-api`

Canonical context ownership and ordering are documented in
[`docs/core-context.md`](../../../docs/core-context.md). Skill-specific lifecycle semantics are
documented in [`docs/skills.md`](../../../docs/skills.md).

`zeta-extension-api` owns the backend-neutral lifecycle contracts used to contribute agent
behavior without placing domain orchestration in Core or App Server. It contains no Skill catalog,
filesystem, JSON-RPC, product UI, or model-provider implementation.

The supported lifecycle is deliberately small:

| Contract | Called by | Purpose |
| --- | --- | --- |
| `SkillActivationContributor` | Core before a new Turn is committed | Resolve explicit selections into durable activations |
| `TurnInputContributor` | Core at each model-invocation safe point | Produce immutable, provenance-bearing prompt fragments |
| `ReadOnlyToolContributor` | Host tool composition | Contribute in-process executors that require no ambient authority |
| `ExtensionRegistry` | Core | Invoke installed contributors in registration order |

App Server may construct the registry and adapt extension events to protocol notifications, but it
must not implement contributor selection, loading, or prompt composition. For read-only tools it
may validate definitions and adapt executors to the normal tool policy/registry pipeline; the domain
operation remains in the contributing extension. `ExtensionRegistry::contribute_read_only_tools`
rejects duplicate model-visible names before host composition.

`ReadOnlyToolContributor` is intentionally not a generic capability escape hatch. Its executors may
read extension-owned source roots that were validated before registration; they must not use ambient
filesystem authority or perform filesystem mutation, process, network, credential, UI, or external
mutation operations. A future extension that needs those capabilities requires a separate
host-reviewed contract.
