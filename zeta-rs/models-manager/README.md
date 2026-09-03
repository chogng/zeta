# `zeta-models-manager`

> 本 README 是 crate 当前实现的权威说明。跨 crate 的目录语义、provider 调研和分阶段演进见
> [`docs/models-manager.md`](../../docs/models-manager.md)；provider declaration 见
> [`zeta-model-provider-config`](../model-provider-config/README.md)，调用 runtime 见
> [`zeta-model-provider`](../model-provider/README.md)。

`zeta-models-manager` 是 provider-independent 的模型目录控制面。它从
`STATIC_MODEL_CATALOG` 经 `ProviderConfigRegistry::builtin()` 投影为 `ProviderDefinition.models` 静态
seed，再把 provider discovery observation 合并到按 scope 隔离的
内存目录，并发布带 generation 的 immutable snapshot。它拥有 freshness、singleflight、字段级 merge、
筛选、typed resolution 和模型基础 instructions 资产；不拥有 provider HTTP DTO、credential、模型调用、prompt/context cache、
Config persistence 或 UI。

## 公共契约

| Symbol | 调用方用途 | 关键语义 |
| --- | --- | --- |
| `ModelsManager` | read、refresh、list、resolve | clone 共享同一个进程内 scope/cache authority |
| `CatalogScopeKey` | 标识 provider + endpoint/account/config revision | source scope 必须是不含秘密的一向指纹 |
| `ModelCatalogSource` | provider runtime 实现 discovery port | 返回完整或 partial observation，不提交半页结果 |
| `ModelCatalogSnapshot` | 上层读取 immutable catalog | 仅消费者可见内容变化时 generation 递增 |
| `CatalogReadPolicy` | `CachePreferred` / `RequireFresh` / `CacheOnly` | 禁止用 `fresh: bool` 模糊表达阻塞语义 |
| `CatalogQuery` | list filter | availability 与 unknown capability policy 显式命名 |
| `ModelRequirements` | invocation-safe resolution | unknown、unsupported、unavailable 和 retired 分开处理 |
| `DiscoveredCatalog` | source 的一次完整提交 | `CompleteAgentCatalog` 缺席可下架；`Partial` 缺席不改变 availability |
| `ModelMetadataPatch` | provider 明确返回的字段 | `Unknown` 不覆盖已有 known metadata |
| `ResolvedModel` | exact model + catalog generation + warnings | `AllowUnlisted` 产生 unverified synthetic metadata |
| `BASE_INSTRUCTIONS` | 当前支持模型的基础行为资产 | App Server 在普通 Turn 创建前冻结；本 crate 拥有文本和 revision |

`CatalogSourceScopeId` 不是 endpoint 或 credential reference。Host 必须先对 normalized endpoint、tenant、
credential revision 和 provider config revision 生成不可逆、无秘密的稳定指纹；任一输入变化都使用新
scope。晚到的旧请求只可能提交到旧 scope。

## 模块与内部所有权

```text
src/
├── manager.rs      # read/refresh/singleflight 与 list/resolve orchestration
├── cache.rs        # per-scope state、clock/freshness 与 snapshot generation rebuild
├── source.rs       # consumer-owned discovery port 与 observation patch
├── snapshot.rs     # immutable snapshot、generation、provenance、warning
├── merge.rs        # seed/live 字段级合并与 complete/partial availability
├── filter.rs       # capability/availability query 与 resolution checks
├── instructions.rs # 模型基础 instructions 资产与 revision
├── policy.rs       # freshness/read named policy
├── scope.rs        # opaque scope identity
├── error.rs        # typed manager failures
└── manager_tests.rs
```

| Private symbol | 当前职责 | 不能扩张到 |
| --- | --- | --- |
| `ManagedScope` | 一个 scope 的 snapshot state 与 async refresh gate | provider transport 或 Config mutation |
| `ScopeState` | records、validator、freshness evidence、last refresh result | durable Config/Thread authority |
| `ModelsManager::ensure_scope` | lazy seed snapshot construction | network discovery |
| `ModelsManager::commit_discovery` | scope/duplicate validation 后原子提交 observation | provider payload decoding |
| `rebuild_snapshot` | 比较 consumer-visible contents 后决定 generation | 每次 read 无条件 bump generation |
| `CatalogRecord` | 合并中的 `ModelInfo`、availability、lifecycle、provenance | 暴露 provider raw DTO |
| `apply_discovery` | complete/partial availability 与 live patch merge | 按 model ID 猜 capability |
| `matches_query` / `validate_requirements` | list 与 resolve 的 canonical gate | UI 搜索或排序偏好 persistence |

如果 manager 开始拼 discovery URL、读取 API key、发送 inference、解释 SSE，或者 provider runtime/UI
重新实现 `ListedOnly` / `AllowUnlisted` 判断，说明 ownership 已漂移。

## 执行路径

静态路径不访问网络：

```text
ModelsManager::static_snapshot / list_static / resolve_static
└─ ensure_scope(CatalogScopeKey::provider_seed)
   ├─ ProviderConfigRegistry::get
   ├─ seed_records(ProviderDefinition.models)
   └─ ModelCatalogSnapshot { generation: 1, freshness: StaticOnly }
```

动态刷新路径：

```text
ModelsManager::refresh(scope, source)
├─ capture refresh_serial
├─ await per-scope AsyncMutex
├─ serial changed → join prior result
├─ ModelCatalogSource::discover(previous validator)   # 不持有 catalog write lock
├─ validate exact scope + duplicate IDs
├─ apply_discovery
│  ├─ Partial: 仅 observed model → Available
│  └─ CompleteAgentCatalog: 缺席 record → Unavailable
└─ rebuild_snapshot
   └─ visible contents changed → generation + 1
```

不同 scope 使用不同 refresh gate，可以并行。相同 scope 的并发调用只执行一次 source request；等待者
复用同一成功 snapshot 或同一 typed error。future 被 drop 即为 cancellation，未返回完整 outcome 前不
修改 snapshot。

## 合并、缓存与失败

当前合并来源是 provider seed 与 provider live observation。Live 的明确字段覆盖 seed；
`ContextWindow::Unknown` 和 `CapabilitySupport::Unknown` 不擦除 known 值。Provenance 与
`ModelMetadataQuality` 随 snapshot entry 暴露。Static seed 的 availability 是 `Unverified`，不是
账号 entitlement 证明。

Freshness 使用 manager policy 与 source 明确给出的 cache hint 中更保守的时长：

| 状态 | `CachePreferred` | `RequireFresh` | `CacheOnly` |
| --- | --- | --- | --- |
| Fresh | 立即返回 | 立即返回 | 立即返回 |
| StaleUsable | 返回并后台 refresh | 等待/join refresh | 返回 stale |
| Expired | 等待/join refresh | 等待/join refresh | 返回 expired |
| 只有静态 seed | 离线时返回；有动态 source 时首次发现 | 有 source 时刷新，否则报错 | 返回 static |

Authentication/permission failure 把先前 `Available` 降为 `Unverified` 并保留 metadata；unsupported、
rate limit、transient、invalid payload 均保留 last-known records，并产生不包含 secret/raw body 的 warning。
Explicit `refresh` 仍返回 typed error，调用方可另行读取 last-known snapshot。

## 集成义务

- `zeta-model-provider-config` 提供 immutable `ProviderConfigRegistry` 和 seed，不依赖本 crate。
- `zeta-model-provider` 持有并公开同一个 `ModelsManager` clone；`Provider::resolve_model` 消费 manager
  的 static resolution，不再维护第二套 catalog gate。
- Local App Server 从 provider runtime 取得该 manager；`model/list` 只投影 manager entries，Session
  model validation 同样调用 manager。
- 动态 provider adapter 应在 `zeta-model-provider`/`zeta-api` 边界实现 `ModelCatalogSource`，本 crate
  不增加 provider switch。

## 测试、修改影响与当前限制

```text
cargo test -p zeta-models-manager
bazel test //zeta-rs/models-manager:models-manager-unit-tests
```

单元测试使用 fake source/fake clock，覆盖确定排序、listed/allow-unlisted、partial/complete 缺席、
Unknown merge、fresh/stale/expired、304 generation 稳定和 per-scope singleflight。修改 merge、freshness、
scope 或 resolution 时必须同步相应 table test、本文和系统文档；新增 protocol-visible 字段还要同步
App Server DTO/schema fixture。

当前实现只有进程内 memory cache，没有 persisted observation、全局/per-provider 并发上限、退避抖动或用户 trust/policy override。Ollama 已通过 provider runtime 接入 `/api/tags` 与 `/api/show`；其他 provider 动态目录仍未实现。App Server 的 `model/list` DTO 投影 identity、display name、access、context、capabilities 与 defaults；本 crate 的 availability、generation、freshness 和 warnings 都不进入产品模型列表，也不作为发送消息的门禁。App Server 还没有 `model/refresh` / `model/updated` wire method；这些是系统文档后续阶段，不应被描述为当前行为。
