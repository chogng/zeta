# `zeta-app-server`

> 本 README 解释 JSON-RPC dispatch、local composition、update broker、resource store 与 review
> model adapter。External method contract 见
> [`docs/zeta-app-server-api.md`](../../docs/zeta-app-server-api.md)，canonical product model 见
> [`docs/protocol.md`](../../docs/protocol.md)，workspace 搜索的跨层 ownership 见
> [`docs/search.md`](../../docs/search.md)，外部 MCP client runtime 的跨 crate 语义见
> [`docs/mcp.md`](../../docs/mcp.md)。

`zeta-app-server` 是产品客户端与 Zeta domain/runtime 的 application boundary。它解析
`zeta-app-server-protocol` request，调用 `SessionCoordinator`、Thread controller、
`TurnExecutor` 与 `ConfigStore`，再返回 typed result 或发布 canonical update。

它不实现 reducer，不定义第二套 Session/Thread/Turn model，也不拥有 rollout 文件格式。

## Runtime ownership

```text
JSONL / in-process caller
└─ AppServer + ConnectionState
   ├─ JSON-RPC validation / initialization gate
   ├─ protocol method dispatch
   ├─ SessionCoordinator / ThreadController
   ├─ TurnExecutor
   ├─ ConfigStore
   ├─ optional WorkspaceFileSystem
   ├─ optional WorkspaceSearchService
   ├─ optional McpRuntimeOwner → zeta-mcp
   ├─ SkillRuntime → zeta-skills + zeta-file-watcher
   ├─ connection-owned ResourceStore
   └─ UpdateBroker → session/update, thread/update, skills/changed
```

App Server connection 不是 product Session。关闭 connection 只失去 connection-local subscription、
request-ID set、notification queue 与 resource ownership；Session/Thread durable state 由 Core/store
继续拥有。

## Public contract

| Symbol | 职责 |
| --- | --- |
| `AppServer` | JSON-RPC application server 与 domain composition handle |
| `ConnectionState` | 每个 logical connection 的 initialized/request-ID/notification state |
| `AppServer::new` | 用 recovered `SessionCoordinator` + `ModelService` 构造 server |
| `AppServer::connection` | 分配 connection ID 并注册 notification queue |
| `AppServer::handle_json` | 处理一个 JSON-RPC request string |
| `AppServer::drain_notifications` | 取出该 connection 的 serialized notifications |
| `AppServer::{serve_stdio,serve_jsonl}` | 同步 JSON Lines service loop |
| `AppServer::create_resource` | 创建 5 分钟 TTL 的 connection-owned resource |
| `AppServer::with_config_store` | 开启 config/provider/MCP/Skill RPC |
| `AppServer::with_slash_command_catalog` | 安装 initialize 时下发的 immutable 动态命令 snapshot |
| `AppServer::with_file_system` | 注入受 workspace 约束的 filesystem authority |
| `AppServer::with_workspace_search` | 注入 workspace root 与冻结的 ripgrep executable |
| `AppServer::with_tool_service` | 安装同一 server 内所有 Turn 使用的 Core Tool/Policy ports |
| `open_local_app_server` | 打开 rollout/config、恢复 coordinator、组合 provider-backed model |
| `LocalAppServerOptions` | local state root + optional config/runtime Workspace + validated slash catalog + built-in Skill root selection |
| `BuiltInSkillRoot` | auto-detected release root、explicit test/host root 或 unavailable 的自解释选择 |
| `SlashCommandCatalog` | 校验动态命令名称、描述与唯一性，并冻结 server-advertised snapshot |
| `ReviewModelResolver` | 从 frozen config snapshot 选择 review-only model |
| `ProviderReviewModel` | `ModelInvoker → zeta_auto_review::ReviewModel` adapter |

`AppServer::new` 默认用 `TurnExecutor::without_tools`。`with_tool_service` 才会替换为有 Tool 和
Policy port 的 executor。`open_local_app_server` 会从启动时 user config snapshot 连接明确
`enabled` 的 unauthenticated MCP server，并把冻结 catalog 与本地工具组合；每次 MCP tool call
仍必须经过 durable one-time approval。它仅在调用方通过
`LocalAppServerOptions::with_workspace_root` 提供统一 Workspace 根时同时组合 filesystem 与
workspace search、只读 `rg` registry；Zeta CLI 的 stdio 与 in-process 路径都会使用同一个启动时解析结果：
`ZETA_WORKSPACE_ROOT` 优先，否则使用当前目录。不能因为 protocol 暴露 approval interaction 就
假设任意自定义 host 已经拥有 Tool registry。`rg` 安装候选来自
[`zeta-install-context`](../install-context/README.md)，App Server 只负责把候选交给
`RipgrepExecutable` 验证并组合成 Tool service。

## 文件与职责

```text
src/
├── server.rs
│   └── server/
│       ├── operations.rs          # Session/Thread/Turn/Resource methods
│       ├── config_operations.rs   # Config/provider/MCP/Skill methods + DTO conversion
│       ├── skill_operations.rs    # Skill catalog/enablement DTO conversion and error mapping
│       ├── skills_runtime.rs       # source composition、catalog cache、watcher、projection
│       ├── fs_operations.rs       # root-relative filesystem DTO conversion/error mapping
│       ├── search_operations.rs   # search RPC decode、ownership 与稳定错误映射
│       └── update_broker.rs       # per-connection subscription/cursor/fanout
├── local.rs                       # persistent local composition + model safe point
├── local_tools.rs                 # frozen rg registry + Core Tool/Policy adapters
├── mcp_runtime.rs                 # continuously driven Tokio worker + synchronous Core bridge
├── mcp_tools.rs                   # Config materialization + MCP Tool/Policy adapters
├── tool_composition.rs            # frozen local/MCP definition and policy routing
├── review.rs                      # review-only provider adapter
├── resource_store.rs              # bounded in-memory connection-owned resources
└── workspace_search.rs            # bounded connection-owned ripgrep jobs
```

## 内部接口地图

| Symbol | 可见性 | 当前职责 | 方向约束 |
| --- | --- | --- | --- |
| `AppServer::dispatch` | private | initialization gate 后对 `ClientMethod` exhaustive dispatch | method string lookup只来自 protocol registry |
| `decode<T>` | crate-private | params JSON → typed DTO，统一 InvalidParams | operation 不手读 arbitrary fields |
| `result<T>` | crate-private | typed result → JSON，统一 serialization failure | external result shape 由 protocol DTO 决定 |
| `core_error` | crate-private | Core error → stable App Server error | 不回传 internal error string |
| `RpcError` | crate-private | internal code + stable `AppServerErrorName` | 不成为第二套 public error DTO |
| `notify_session_updates` / `notify_thread_updates` | private methods | 从 durable store gap 读取并交给 broker | mutation 后从 authoritative history 发布 |
| `AppServerThreadUpdates` | private sink | 将 TurnExecutor live/committed update 接入 broker | 不修改 canonical update |
| `UpdateBroker` | private | subscription、durable cursor、weak queue fanout | 不持有 connection/session runtime authority |
| `UpdateBroker::publish_thread_update` | private | committed 按 durable cursor；transient 直接给 subscriber | 两类 cursor semantics 不得混合 |
| `SkillRuntime` | crate-private | 组合 built-in/user roots、缓存 metadata projection、叠加 config enablement | 不读取正文、不执行 Skill、不拥有 config durability |
| `SkillConfigSnapshotProvider` | crate-private trait | 给 runtime 最新 `SkillsConfig` 与待监听 config path | implementation 不把客户端 path 直接升级为 trusted root |
| `compose_sources` | private | enabled user absolute root + release root → validated `SkillSourceRoot` | Workspace/Plugin source 尚不在当前 composition |
| `watch_skill_sources` | private | watcher invalidation 后完整 reconcile 并更新 watched path registration | watch event 不是文件事实，不直接推进 generation |
| `notification<T>` | private | canonical update → JSON-RPC notification | method 来自 `ServerNotificationMethod` |
| `ResourceStore::resource` | private | cleanup + owner check | 所有 read/release 必须经过这里 |
| `ResourceStore::cleanup` | private | lazy TTL eviction | resource 不持久化 |
| `AppServer::file_system` | private | 读取注入的 `WorkspaceFileSystem` 或返回稳定 unavailable error | 不绕过 workspace authority |
| `file_type` in `fs_operations` | private | foundation file kind → protocol DTO | wire enum 只由 protocol crate 定义 |
| `WorkspaceSearchService` | crate-private | 持有 workspace、frozen rg 和 connection-owned job map | 不持有 Renderer 状态或模型 Tool authority |
| `run_search` | private | 以 typed argv 启动/取消 rg 并收束 terminal state | 不经过 shell，不回传任意 stderr |
| `parse_match` | private | rg JSON line → root-relative DTO，并把 byte range 转为 UTF-16 | 不执行 UI 分组或 editor opening |
| `ConfigBackedModelService::resolve_config` | private | user snapshot + optional Workspace snapshot merge | 每次 invocation safe point 重新解析 |
| `WorkspaceConfigTracker::read` | private | 内容变化才推进 synthetic workspace revision | 不监听/修改 workspace file |
| `compose_local_tools` | crate-private | 复用 App Server 已固定的 WorkspaceRoot、解析安装候选、冻结 rg、选择 native sandbox | discovery 失败时不降级成 unrestricted |
| `LocalShellToolService::materialize` | private | parse call、约束 workspace 参数、冻结 rg executable | policy review 前不启动进程 |
| `LocalReadOnlyPolicy::decide` | private | 只接受 exact revision/provenance/capability/sandbox | 不产生 unsandboxed grant |
| `McpRuntimeOwner` | crate-private | worker thread 持有 Tokio runtime 和 live `McpRuntime` | Core thread 不嵌套 `block_on` |
| `McpToolService::review_request` | private | exact binding/arguments/generation → MCP action digest | remote annotation 不授予只读信任 |
| `McpApprovalPolicy::decide` | private | 只接受已知 user MCP provenance 并返回 one-time approval | 不自动批准远端副作用 |
| `CompositeToolService` | private | model tool name → frozen local/MCP service | duplicate name 在 composition 时失败 |
| `CompositePolicyService` | private | trusted `ActionSource` → owning policy | 不依靠 trial-and-error policy fallback |
| `ModelSnapshotResolver` | private trait | frozen config → immutable invoker | implementation 不持有 mutable config view |
| `SlashCommandCatalog::new` | public constructor | 校验 lowercase ASCII/interior-hyphen name、非空描述与唯一性 | 不执行命令、不引用 TUI built-ins |
| `ProviderReviewModel::request` | private | system/input/schema → tool-disabled zero-temperature request | reviewer 不获得 Tool capability |
| config `*_dto` / `*_from_dto` helpers | private | external DTO 与 config domain 显式转换 | invalid identity 映射 InvalidParams |

## JSON-RPC 调用图

```text
AppServer::handle_json(connection, raw)
├─ serde_json::from_str<Value>
│  └─ ParseError
├─ deserialize JsonRpcRequest<Value>
├─ validate jsonrpc == 2.0 and positive numeric ID
├─ reject duplicate request ID within connection
├─ AppServer::dispatch
│  ├─ initialize allowed before gate
│  ├─ require connection.initialized
│  ├─ client_method(request.method)
│  └─ domain operation
│     ├─ decode<Params>
│     ├─ Core / Config / Resource call
│     ├─ notify committed gaps when applicable
│     └─ result<ResultDto>
└─ JsonRpcSuccess or JsonRpcFailure → JSON string
```

`serve_jsonl` 对每一行先写 response，再 drain/write causal notifications。当前 loop 是同步串行的；
protocol registry 中的 `SerializationScopeDefinition` 尚未接入并发 scheduler。

Initialize 是每 connection 一次。重复 initialize 返回 `AlreadyInitialized`；初始化前的其他 method
返回 `NotInitialized`。Request ID 只接受正整数，且在 connection 生命周期内不能重复。
成功结果同时包含 composition 时冻结的 `slashCommands`。不同 connection 可获得同一 server
snapshot；单个 connection 生命周期中不会因 host 后续状态变化而更换 popup contract。

## Session、Thread 与 Turn orchestration

典型 create path：

```text
session/thread/create
├─ SessionCoordinator::create_thread
├─ subscribe caller to new Thread
├─ notify_session_updates(previous session sequence)
├─ notify_thread_updates(0)
└─ return current Session + ThreadId
```

`session/subscribe(afterSequence)` 与 `thread/subscribe(afterSequence)` 先读取 snapshot + durable gap，
再把 broker cursor 放到当前 aggregate sequence。订阅是 connection-local delivery state；真实 gap
来自 coordinator/store。

`turn/start`：

1. 校验 Thread 属于 supplied Session；
2. `start_turn` 使用 typed command ID + exact expected sequence；
3. replay 时读取既有 Turn，terminal failure/interruption 不伪装成 success；
4. 新 start 发布 durable update 后调用 `TurnExecutor::start`。

`turn/interaction/resolve` 使用 exact durable `RequestId`。当 response 是 Tool Call 对应的 approval，
且 Core 确实产生 `Resolved` disposition，App Server 再启动 executor 恢复 Tool path。这个判断依赖
pending interaction 的 item binding，不能简化为“所有 approval 都 restart”。

## Update broker

每个 `Subscriber` 保存 weak notification queue，以及 `SessionId → durable sequence`、
`ThreadId → durable sequence`：

```text
committed update
└─ only sequence > subscriber cursor
   ├─ enqueue
   └─ advance durable cursor

transient Thread update
└─ if subscribed: enqueue
   └─ do not advance durable cursor
```

Queue 的最后一个 strong owner 是 `ConnectionState`；connection drop 后 broker 在下一次 publish 时
通过 `Weak::upgrade` 失败清除 subscriber。目前没有 queue length/backpressure limit，slow consumer
可能积累内存，这是当前限制。

## Local composition 与 model safe point

`open_local_app_server` 的顺序：

```text
RolloutRepository::open(state_root)
└─ recover_coordinator

ConfigStore::open(config.authority.json)
└─ read_snapshot preflight
   └─ enabled user MCP declarations
      ├─ materialize absolute stdio executable / unauthenticated HTTP endpoint
      ├─ McpRuntimeOwner::start
      └─ immutable catalog + CompositeToolService

optional WorkspaceConfigStore
└─ WorkspaceConfigTracker::read preflight

ConfigBackedModelService
└─ AppServer::new(...).with_config_store(...)
   └─ SkillRuntime::new(...)
      ├─ release built-in root
      ├─ enabled user Skill source roots
      └─ SkillWatcher
```

每次 `ModelService::invoke` 重新读取 user config，与 optional workspace document 合并，再由
`ModelSnapshotResolver` 生成 immutable invoker。因此 config change 影响下一次 invocation，不会
改变已经运行的 invocation。`ProviderModelService` 在 invoker 调用前后检查 cancellation；底层同步
provider request 能否中途停止仍取决于 provider transport。

## Skill catalog runtime

Local composition 总会安装 `SkillRuntime`。`BuiltInSkillRoot::AutoDetect` 先查询
`InstallContext::bundled_resource_directory("skills")`，Cargo 开发构建再回退到仓库
`skills/assets`；测试与自定义 host 可用 explicit root，明确不提供时使用 `Unavailable`。
AutoDetect 两处都找不到时 runtime 发布 built-in `SourceUnavailable` diagnostic，使客户端的 Errors
tab 可解释“为什么没有内置 Skill”；显式 `Unavailable` 才表示 host 有意省略 built-in。
User source 只接受 config authority 中 enabled 的 absolute root reference，之后仍由
`SkillSourceRoot::user` 做目录、canonicalization 与 source-kind 校验。

```text
skills/list
└─ SkillRuntime::list(Cached | Refresh)
   ├─ read current SkillsConfig
   ├─ rebuild catalog when source composition changes
   ├─ otherwise reuse cache or call SkillCatalog::refresh
   └─ overlay durable per-Skill enablement

skill/enablement/set
├─ require exact discovered SkillId
├─ ConfigStore::apply(SetSkillEnablement)
└─ reconcile → publish skills/changed when projection changes
```

Watcher 订阅当前 source roots 与 user config authority path。change、overflow 或 backend rescan
提示都只触发完整 reconcile；entry、diagnostic 或 enablement projection 没变时 generation 不变，
也不发 notification。Watcher 启动失败时 local App Server 仍可用，显式
`skills/list { reload: "refresh" }` 是恢复路径。

当前只组合 built-in 与 user source。Workspace/Plugin source、正文 activation、context assembly
和 invocation safe-point freezing 尚未实现；禁用只影响 catalog eligibility，不能改变已经运行的
Turn。

MCP runtime 当前只在 `open_local_app_server` safe point 构造。`enabled` 是建立连接/启动 server
的显式用户 intent；它不批准任何 tool call。stdio command 必须是存在的 absolute executable，
HTTP 当前只接受 unauthenticated endpoint，credential reference 会使 composition 明确失败。
Workspace MCP intent 仍保持 pending trust，不会接入。Config mutation 或 `tools/list_changed`
不会原地改 catalog：list-changed 后旧 runtime fail closed；当前需重启 App Server 才会构造新
generation。启动采用 `RequireAll`，任一 enabled server 无法 initialize 时 App Server 明确启动
失败，不会静默丢失部分 catalog。

缺少 preferred model/provider 时创建 `UnavailableModel`，使 invocation 显式失败，不回退到 echo
或任意默认 provider。

## Review model adapter

`ReviewModelResolver::resolve` 使用 frozen `ResolvedConfig`：

```text
resolve_approval_review_model(registry)
├─ locate selected provider config
├─ ModelProvider::runtime(ModelRuntimeRequest)
└─ ProviderReviewModel { exact ModelRef, immutable invoker }
```

`ProviderReviewModel::request` 把 trusted reviewer prompt 放入 instructions，把 action JSON 放入
user text，并附 response schema；同时清空 tools、设置 `ToolChoice::None`、禁止 parallel tool calls
并使用 temperature 0。Response 只拼接 text fragments，忽略 reasoning，拒绝 refusal、Tool Call 与
空 JSON。

这里负责选择/隔离 review runtime；classifier schema validation 与 authorization decision分别属于
`zeta-auto-review` 和 `zeta-policy`。

## Resource store

Resource 是 in-memory、connection-owned、TTL-bounded：

- 单个 resource 最大 `MAX_RESOURCE_BYTES` = 16 MiB；
- read chunk 最大 `MAX_READ_CHUNK_BYTES` = 262,144 bytes；
- `create_resource` 当前固定 TTL 为 300 秒；
- ID 是 process-local monotonic hex string；
- metadata 包含 MIME、byte length 和 SHA-256；
- read 返回 standard Base64、decoded length、offset 与 EOF；
- cleanup 在 create/read/metadata/release path lazy 执行。

Resource 不跨重启恢复，也不能被另一 connection 读取或 release。Connection drop 当前不会立即遍历
删除资源，只依赖 TTL lazy cleanup。

## Error mapping

| Source | External error |
| --- | --- |
| malformed JSON | `ParseError` / `-32700` |
| invalid envelope/ID/params | `InvalidRequest` / `InvalidParams` |
| unknown method | `MethodNotFound` |
| Core command ID conflict | `CommandConflict` |
| other Core failure | `CoreOperationFailed` |
| missing config store | `ConfigUnavailable` |
| config sequence mismatch | `ConfigRevisionConflict` |
| missing Skill runtime | `SkillsUnavailable` |
| invalid/missing exact Skill target | `SkillNotFound` |
| Skill config/catalog failure | `SkillOperationFailed` |
| resource ownership/bounds | corresponding stable resource error |
| missing filesystem authority | `FileSystemUnavailable` |
| filesystem path/I/O failure | `FileSystemOperationFailed` |
| missing search backend | `SearchUnavailable` |
| unknown/cross-connection search job | `SearchNotFound` / `SearchNotOwner` |
| search job capacity exhausted | `SearchBusy` |
| rg spawn/parse/exit failure | terminal `WorkspaceSearchReadResult.error` with stable redacted text |
| poisoned lock/serialization invariant | `ServerOverloaded` or `InternalError` |

External errors不携带 `CoreError`、`ConfigError` 或 backend error text。新增 error mapping 时先更新
protocol enum，再在本 crate 显式转换。

## 方向偏差检查

- `server.rs` 定义新的 RPC params/result：contract ownership 从 protocol crate 漂移；
- App Server 直接构造/store event 或执行 reducer：Core ownership漂移；
- App Server 解析 rollout files：repository/storage ownership漂移；
- ConnectionState 保存 product Session/Thread authoritative snapshot：delivery 与 domain混合；
- Broker 用 transient event 推进 durable cursor：reconnect gap 可能丢失；
- Model invocation 长期持有 mutable config snapshot：safe-point guarantee 被破坏；
- Review model 保留 Tool definitions 或接受 model-selected provider：review isolation 失效；
- Resource owner check只在某些 operation：跨 connection data leak；
- Error response回显 arbitrary internal text：external contract与安全边界漂移；
- 新 method只改 `dispatch` 未改 protocol registry/artifacts：client/server contract分叉。

## 测试、限制与演进

```text
cargo test -p zeta-app-server
bazel test //zeta-rs/app-server:app-server-unit-tests
```

## Typst integration

`AppServer::typst_compile` owns the application-level bridge from
`document/typst/compile` to `zeta_typst::TypstCompiler`. It maps expected
source failures to `TypstCompileResult::Failed`, creates a 300-second
connection-owned `application/pdf` resource on success, and maps compiler
diagnostics into protocol DTOs through `typst_diagnostic_dto`.

The compiler boundary itself belongs to `zeta-typst`; this crate must not add
host path resolution, package download, or font discovery around it. A change
from connection-owned ephemeral resources to durable document storage is a
separate ownership decision. The cross-crate trust model is documented in
[`docs/typst.md`](../../docs/typst.md).

Tests 覆盖 initialization/request IDs、Session-first flow、command replay/conflict、fork lineage、
Turn replay/model-once、multi-connection update、reconnect durable gap、connection-owned resources、
config command、interaction/approval resolve、response-before-notification、model config safe point、
Workspace override、review-only request、只读 `rg` definition/materialization/policy/execution，
MCP worker bridge、exact provenance/approval policy、local/MCP 路由与 collision rejection，以及
Skill built-in/user composition、enablement overlay、watcher refresh 与 `skills/changed`。

local tool 的参数白名单、discovery、取消与输出限制由
[`zeta-shell-command`](../shell-command/README.md) 和 [`zeta-exec`](../exec/README.md) 维护；
本 README 只拥有 App Server 组合与 Core port binding。

当前 server 是 synchronous JSONL/in-process boundary；没有 async multi-connection scheduler、
serialization-scope enforcement、notification backpressure、durable resource、immediate disconnect
cleanup 或 complete network server lifecycle。MCP 当前没有 credential materialization、stdio
process sandbox、runtime hot reload、list-changed rebuild、progress/elicitation delivery 或 image
result 的原生 Core content path；MCP image 暂时编码进 bounded JSON text result。演进这些能力时
应保留 protocol registry唯一性、Core/store authority、snapshot + durable gap 和
per-invocation config safe point。
