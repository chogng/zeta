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
| `ContextContributor` | Core before the first model invocation | Return bounded, low-trust evidence with current consent |
| `ReadOnlyToolContributor` | Host tool composition | Contribute in-process executors that require no ambient authority |
| `ExtensionRegistry` | Core | Invoke installed contributors in registration order |

App Server may construct the registry and adapt extension events to protocol notifications, but it
must not implement contributor selection, loading, or prompt composition. For read-only tools it
may validate definitions and adapt executors to the normal tool policy/registry pipeline; the domain
operation remains in the contributing extension. `ExtensionRegistry::contribute_read_only_tools`
rejects duplicate model-visible names before host composition. Read-only tools and context sources
are registered by extension identity; reinstalling that identity replaces its contribution in place.
`ContextEvidence` and `ContextSourceRequest` belong to this contract. Core also re-exports these values
for host context sources, applies the shared evidence budget, and never turns them into instructions.

The Memories extension contributes both first-invocation evidence and model-requested search/read
tools. Both routes resolve current host-bound Session/Thread authority and Memory consent on every
read. Tool execution contexts carry these identities independently of model arguments.

`ReadOnlyToolContributor` is intentionally not a generic capability escape hatch. Its executors may
read extension-owned source roots that were validated before registration; they must not use ambient
filesystem authority or perform filesystem mutation, process, network, credential, UI, or external
mutation operations. A future extension that needs those capabilities requires a separate
host-reviewed contract.

## 生命周期与展示项

- `LifecycleObserver` 接收已经提交的 Thread 创建、归档、恢复与 Turn 开始/终止事实，以及 Config generation 变化。
- 回调不重入 Core，不否决已提交状态；有持久副作用的消费者按 Thread 和 sequence 去重。读取和历史重放不再次触发回调。
- `IdleContributor` 在 Turn 终止后唤醒扩展自己的后台工作；持久工作仍须在启动时恢复。
- `ItemContributor` 返回指定 Thread 的有界文本展示项；注册表检查重复身份和边界，`zeta-extension-items` 定义共享数据结构。
- 队列已接入空闲唤醒和展示项，使用统计已接入 Turn/config 回调。
