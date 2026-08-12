# `zeta-app-server`

> 本 README 解释 JSON-RPC dispatch、local composition、update broker、resource store 与 review
> model adapter。External method contract 见
> [`docs/zeta-app-server-api.md`](../../docs/zeta-app-server-api.md)，canonical product model 见
> [`docs/protocol.md`](../../docs/protocol.md)，workspace 搜索的跨层 ownership 见
> [`docs/search.md`](../../docs/search.md)，外部 MCP client runtime 的跨 crate 语义见
> [`docs/mcp.md`](../../docs/mcp.md)，Git/SCM 跨进程 ownership 见
> [`docs/git.md`](../../docs/git.md)，Workspace identity 与 trust boundary 见
> [`docs/workspace-security.md`](../../docs/workspace-security.md)。跨文件内容搜索的执行实现见
> [`zeta-rs/search/README.md`](../search/README.md)。

`zeta-app-server` 是产品客户端与 Zeta domain/runtime 的 application boundary。它解析
`zeta-app-server-protocol` request，调用 `SessionCoordinator`、Thread controller、
`TurnExecutor` 与 `ConfigStore`，再返回 typed result 或发布 canonical update。

它不实现 reducer，不定义第二套 Session/Thread/Turn model，也不拥有 SQLite schema。

## 运行时所有权

```text
JSONL / in-process caller
└─ AppServer + ConnectionState
   ├─ JSON-RPC validation / initialization gate
   ├─ protocol method dispatch
   ├─ SessionCoordinator / ThreadController
   ├─ TurnExecutor
   ├─ ConfigStore
   ├─ optional WorkspaceFileSystem + filesystem watcher
   ├─ optional GitRuntime → zeta-file-watcher + GitService → zeta-git
   ├─ optional SearchService → zeta-search
   ├─ optional CodeIndexRuntime → zeta-code-index + filesystem watcher
   ├─ optional CloudCodeIndexController → zeta-code-index-cloud + host provider registry
   ├─ request-scoped CodeRetrievalService → local/cloud RRF + verification + content budget
   ├─ optional TerminalService → zeta-utils-pty
   ├─ reloadable MCP Tool generation → zeta-mcp
   ├─ SkillRuntime → zeta-skills + zeta-file-watcher
   ├─ WorkspaceCustomizations → zeta-instructions + zeta-agents
   ├─ connection-owned ResourceStore
   └─ UpdateBroker → session/update, session/thread/update, config/changed, skills/changed, git/statusChanged, fs/changed
```

App Server connection 不是 product Session。关闭 connection 只失去 connection-local subscription、
request-ID set、notification queue 与 resource ownership。默认的 `SessionStateMode::Durable` 由
Core/store 继续拥有 Session/Thread durable state；需要进程内生命周期的 host 可以显式选择
`SessionStateMode::Ephemeral`，此时 coordinator 使用 in-memory stores，Session/Thread 与用户消息
不会从 profile SQLite 恢复或写入。ConfigStore 仍可按 profile 保存普通配置。

## 公共契约

| Symbol | 职责 |
| --- | --- |
| `AppServer` | JSON-RPC application server 与 domain composition handle |
| `ConnectionState` | 每个 logical connection 的 initialized/request-ID/notification state |
| `AppServer::new` | 用 recovered `SessionCoordinator` + `ModelService` 构造 server |
| `AppServer::connection` | 分配 connection ID 并注册 notification queue |
| `AppServer::connection_notifications` | 返回可阻塞等待、主动唤醒的 connection outbound source |
| `AppServer::close_connection` | 注销 subscription、关闭 notification source，并释放 Resource/Terminal/Syntax owner |
| `AppServer::handle_json` | 处理一个 JSON-RPC request string |
| `AppServer::drain_notifications` | JSONL 与同步适配 caller 取出该 connection 的 serialized notifications |
| `AppServer::{serve_stdio,serve_jsonl}` | 同步 JSON Lines service loop |
| `AppServer::create_resource` | 创建 5 分钟 TTL 的 connection-owned resource |
| `AppServer::with_config_store` | 开启 config/provider/MCP/Skill RPC |
| `AppServer::with_slash_command_catalog` | 安装 initialize 时下发的 immutable 动态命令 snapshot |
| `AppServer::with_file_system` | 注入受 workspace 约束的 filesystem authority |
| `AppServer::with_file_system_watcher` | 监听可信 workspace root 并发布相对路径 invalidation hint |
| `AppServer::with_git_root` | 冻结 workspace root，开启 Git status/mutation、watcher 与 revision notification |
| `AppServer::with_workspace_search` | 注入 workspace root 与冻结的 ripgrep executable，构造外部内容搜索服务 |
| `AppServer::with_code_index_storage_root` | 配置按 root identity 分隔的 persistent index cache |
| `AppServer::with_cloud_code_index_providers` | 注入冻结的 provider registry；空 registry 不广告 cloud capability |
| `AppServer::with_cloud_code_index_storage_root` | local composition 配置按 root identity 分隔的 durable grant/deletion state |
| `AppServer::with_tool_service` | 安装同一 server 内所有 Turn 使用的 Core Tool/Policy ports |
| `open_local_app_server` | 按 SessionStateMode 选择 durable/in-memory coordinator，打开 config 并组合 provider-backed model |
| `open_local_app_server_with_cloud_providers` | 在 Workspace 激活前注入 cloud code-index providers；默认入口使用空 registry |
| `LocalAppServerOptions` | user profile root + SessionStateMode + optional config/runtime Workspace + validated slash catalog + built-in Skill root selection + optional model operation client |
| `SessionStateMode` | 明确选择 profile SQLite durable history 或 process-local ephemeral Session/Thread state |
| `BuiltInSkillRoot` | auto-detected release root、explicit test/host root 或 unavailable 的自解释选择 |
| `zeta_slash_commands::SlashCommandCatalog` | 委托共享 crate 校验动态命令并冻结 server-advertised snapshot；App Server 只拥有 composition |
| `ReviewModelResolver` | 从 frozen config snapshot 选择 review-only model |
| `ProviderReviewModel` | `ModelInvoker → zeta_auto_review::ReviewModel` adapter |

`AppServer::new` 默认用 `TurnExecutor::without_tools`。`with_tool_service` 才会替换为有 Tool 和
Policy port 的 executor。`open_local_app_server` 会从 user config snapshot 连接明确 `enabled`
的 unauthenticated MCP server，并把 catalog 与本地工具组合；Config commit 会在后台构建新
generation 并只切换未来的 prepare。已 prepare 的调用继续绑定原 Tool/Policy generation，直到
execute 完成。每次 MCP tool call 仍必须经过 durable one-time approval。它仅在调用方通过
`LocalAppServerOptions::with_workspace_root` 提供统一 Workspace 根时同时组合 filesystem、
`.zeta` 自定义 catalog、Workspace code index、Workspace search、Git SCM、connection-owned Terminal runtime、
只读 `rg` registry；Zeta CLI 的 stdio 与
in-process 路径都会使用同一个启动时解析结果：
`ZETA_WORKSPACE_ROOT` 优先，否则使用当前目录。不能因为 protocol 暴露 approval interaction 就
假设任意自定义 host 已经拥有 Tool registry。`rg` 安装候选来自
[`zeta-install-context`](../install-context/README.md)，App Server 只负责把候选交给
`RipgrepExecutable` 验证并组合成 Tool service。

默认 local composition 使用 lazy production model client：启动、配置读取和模型目录展示不会加载
system roots 或 proxy。Embedded test 可通过 `LocalAppServerOptions::with_model_operation_client`
注入离线 client；真实 transport 初始化失败在第一次模型 operation 返回，不得让 App Server 启动 panic。

local composition 会配置 `<profile>/code-index-cloud` 的 durable state 位置，但默认 provider registry
为空，因此不会安装 cloud controller、广告 `cloudCodeIndex` 或创建网络请求。具体 host 只有在注入
接受 Workspace-owned exact chunks 且满足幂等 grant deletion 的 provider 后，才能启用云能力。

Tab 关闭的停止语义属于本层适配，而不是 `zeta-protocol` 的新 command：前端通过
`session/request` 的 `Stop` operation 到达 Session mutation dispatcher，后者调用 Core 的
`SessionCoordinator::stop`，再从 durable gap 发布 Session 和 child Thread updates。单纯
`AppServer::close_connection` 只释放 connection-owned delivery/resource state，不停止产品
Session。

## 文件与职责

```text
src/
├── server.rs
│   └── server/
│       ├── operations.rs          # Session/Thread/Turn/Resource methods
│       ├── config_operations.rs   # Config/provider/MCP/Skill methods + DTO conversion
│       ├── config_runtime.rs      # Config commit → config/changed fanout
│       ├── skill_operations.rs    # Skill catalog/enablement DTO conversion and error mapping
│       ├── skills_runtime.rs       # source composition、catalog cache、watcher、projection
│       ├── start_turn.rs           # durable command replay before mutable model/Skill resolution
│       ├── workspace_customizations.rs # Instruction/Agent catalogs + reloadable harness snapshot
│       ├── fs_operations.rs       # root-relative filesystem DTO conversion/error mapping
│       ├── fs_watcher.rs          # root watcher、内部 observer、相对路径投影与 fs/changed 发布
│       ├── git_operations.rs      # Git RPC decode 与稳定错误映射
│       ├── git_runtime.rs         # status projection/revision、watcher、去重与通知
│       ├── interaction_runtime.rs # pending interaction deadline enforcement
│       ├── notification_queue.rs  # bounded per-connection queue + wake/close semantics
│       ├── search_operations.rs   # search RPC decode、ownership 与稳定错误映射
│       ├── code_index_operations.rs # status/search/rebuild DTO 与 error mapping
│       ├── code_index_runtime.rs  # generation lifecycle、watcher reconcile 与 stale gate
│       ├── cloud_code_index_operations.rs # preview/grant/sync/revoke DTO 与稳定错误映射
│       ├── terminal_operations.rs # terminal RPC decode、ownership 与稳定错误映射
│       └── update_broker.rs       # per-connection subscription/cursor/fanout
├── local.rs                       # local composition, session backend selection + model safe point
├── local_tools.rs                 # frozen rg registry + Core Tool/Policy adapters
├── mcp_runtime.rs                 # continuously driven Tokio worker + synchronous Core bridge
├── mcp_tools.rs                   # Config materialization + MCP Tool/Policy adapters
├── tool_composition.rs            # local/MCP routing + generation-safe atomic replacement
├── review.rs                      # review-only provider adapter
├── resource_store.rs              # bounded in-memory connection-owned resources
├── git_service.rs                 # workspace root + GitClient + synchronous RPC runtime bridge
├── terminal_environment.rs        # secret-excluding host environment、platform normalization 与 terminal identity
├── terminal_profiles.rs           # trusted Shell discovery、ID 与 frozen environment
└── terminal_service.rs            # PTY runtime、output ring 与 connection-owned sessions
```

## 内部接口地图

| Symbol | 可见性 | 当前职责 | 方向约束 |
| --- | --- | --- | --- |
| `AppServer::dispatch` | private | initialization gate 后对 `ClientMethod` exhaustive dispatch | method string lookup只来自 protocol registry |
| `AppServer::session_request` | `pub(super)` | 解码 canonical `session/request`，按 typed operation 调用 Session/Thread/Turn helper 并发布 durable gaps | 不在 App Server reducer 或 `zeta-protocol` 中复制 mutation model |
| `decode<T>` | crate-private | params JSON → typed DTO，统一 InvalidParams | operation 不手读 arbitrary fields |
| `result<T>` | crate-private | typed result → JSON，统一 serialization failure | external result shape 由 protocol DTO 决定 |
| `core_error` | crate-private | Core error → stable App Server error | 不回传 internal error string |
| `RpcError` | crate-private | internal code + stable `AppServerErrorName` | 不成为第二套 public error DTO |
| `notify_session_updates` / `notify_thread_updates` | private methods | 从 durable store gap 读取并交给 broker | mutation 后从 authoritative history 发布 |
| `AppServerThreadUpdates` | private sink | 将 TurnExecutor live/committed update 接入 broker | 不修改 canonical update |
| `UpdateBroker` | private | subscription、durable cursor、weak queue fanout | 不持有 connection/session runtime authority |
| `UpdateBroker::publish_thread_update` | private | committed 按 durable cursor；transient 直接给 subscriber | 两类 cursor semantics 不得混合 |
| `InteractionDeadlineWatcher` | crate-private | 在 workspace mutation gate 下重检 durable pending request，持久化 `DeadlineElapsed` cancellation 并失败 Turn | 不选择 UI、不解释 approval policy、不修改 reducer |
| `SkillRuntime` | crate-private | 组合 roots、缓存 metadata projection、叠加 enablement，并把 exact Skill body 适配为 Core 的通用 `SkillInstruction` | 不执行 Skill 脚本、不拥有 config durability 或 instruction precedence |
| `start_turn::replayed_result` | private | 用 durable command receipt 校验重复 `StartTurn` 输入并返回原 Turn 结果 | 重放不重新读取 model config 或 Skill 文件；不同输入返回 `CommandConflict` |
| `SkillConfigSnapshotProvider` | crate-private trait | 给 runtime 最新 `SkillsConfig` 与 commit signal | implementation 不把客户端 path 直接升级为 trusted root |
| `compose_sources` | private | release root + enabled user absolute roots + `.zeta/skills` → validated `SkillSourceRoot` | Plugin source 尚不在当前 composition |
| `watch_skill_sources` | private | watcher invalidation 后完整 reconcile 并更新 watched path registration | watch event 不是文件事实，不直接推进 generation |
| `WorkspaceCustomizations` | crate-private | 发现/刷新 `.zeta/instructions` 与 `.zeta/agents`，发布 future-invocation harness snapshot | 不执行 Agent、不激活 Skill、不解析外部生态格式 |
| `WorkspaceFileChangeSink` | crate-private trait | 在 `fs/changed` 发布前把 projected invalidation 交给内部 runtime owner | callback 不把 watcher event 当作文件事实 |
| `notification<T>` | private | canonical update → JSON-RPC notification | method 来自 `ServerNotificationMethod` |
| `ResourceStore::resource` | private | cleanup + owner check | 所有 read/release 必须经过这里 |
| `ResourceStore::cleanup` | private | lazy TTL eviction | resource 不持久化 |
| `AppServer::file_system` | private | 读取注入的 `WorkspaceFileSystem` 或返回稳定 unavailable error | 不绕过 workspace authority |
| `fs_watcher::project_event` | private | watcher path → root-relative invalidation 或 rescan hint | 不把 event 当作文件内容事实 |
| `GitService` | crate-private | 持有 `MutateRepository` 的 `TrustedWorkspace`、映射 repository path、持有 Tokio runtime，并调用 `zeta-git` query/mutation API | 不从 client path 或 repository config 自行授予 trust |
| `GitRuntime` | crate-private | 串行 operation、为每次 runtime incarnation 创建 `StreamInstanceId`、投影 workspace status、推进实例内 revision 并发布去重 notification | watcher event 不直接成为 Git truth |
| `project_status` in `git_runtime` | private | `zeta-git` snapshot → renderer-safe protocol DTO | 不回传绝对 metadata path 或 internal stderr |
| `file_type` in `fs_operations` | private | foundation file kind → protocol DTO | wire enum 只由 protocol crate 定义 |
| `search_operations::{search_query, search_page}` | private | `WorkspaceSearch*` DTO 与 `zeta-search` 领域类型之间的显式转换 | 不复制查询校验、rg argv、job state 或 parsing |
| `SearchService` | external crate | 持有 active workspace、frozen rg 和 owner-bound job map | App Server 不把 connection/DTO/UI 语义写入该 crate |
| `CodeIndexRuntime` | crate-private | 串行 rebuild/refresh，投影 lifecycle，并在返回前 materialize | 不拥有 scan/chunk/schema，也不创建网络请求 |
| `CodeIndexRefreshWorker` | private | 单 wake + merged paths/rescan priority 的后台刷新 | 不阻塞 filesystem notification thread，不建立无界 event queue |
| `code_index_operations::project_status` | private | runtime state + last usable snapshot → stable protocol status | 不暴露 SQLite/internal error text |
| `CloudCodeIndexController` | external crate | root-bound grant、publication/deletion lifecycle 与 provider port | App Server 不复制 consent state 或允许 provider 直接读 Workspace |
| `cloud_code_index_operations::project_status` | private | cloud lifecycle + grant → stable protocol DTO | 不回传 credential、绝对路径或 provider error text |
| `TerminalService` | crate-private | 持有 `ExecuteProcess` 的 `TrustedWorkspace`、Tokio runtime、PTY session map 与 1 MiB output ring | 不从 client path 自行授予 process authority |
| `TerminalEnvironment` | crate-private | 二次过滤 host environment、规范化 Windows key、固定 `TERM`/`COLORTERM`/`TERM_PROGRAM` | 不继承凭据或接受 client mutation |
| `TerminalProfileCatalog` | crate-private | 冻结可信 Shell Profile、program 与 `TerminalEnvironment` | external DTO 不暴露 executable/args/environment |
| `TerminalService::create` | crate-private | 将 default/profile ID 解析到 catalog 并启动 workspace-rooted PTY | client 不能提交 executable/environment |
| `spawn_output_drainers` | private | raw output/exit 并发收束；尾部输出 EOF 后才标记 exited | 不在 exit code 到达时提前丢弃尾部 bytes |
| `read_state` | private | after-sequence cursor → bounded Base64 chunks + gap/exited state | ring eviction 必须显式返回 `output_gap` |
| `ConfigBackedModelService::resolve_config` | private | user snapshot + optional Workspace snapshot merge | 每次 invocation safe point 重新解析 |
| `WorkspaceConfigTracker::read` | private | 内容变化才推进 synthetic workspace revision | 不监听/修改 workspace file |
| `compose_local_tools` | crate-private | 要求 root-bound `ExecuteProcess` capability、解析安装候选、冻结 rg、选择 native sandbox | containment、trust 或 discovery 失败时不降级成 unrestricted |
| `LocalShellToolService::materialize` | private | parse call、约束 workspace 参数、冻结 rg executable | policy review 前不启动进程 |
| `LocalReadOnlyPolicy::decide` | private | 只接受 exact revision/provenance/capability/sandbox | 不产生 unsandboxed grant |
| `McpRuntimeOwner` | crate-private | worker thread 持有 Tokio runtime 和 live `McpRuntime` | Core thread 不嵌套 `block_on` |
| `McpToolService::review_request` | private | exact binding/arguments/generation → MCP action digest | remote annotation 不授予只读信任 |
| `McpApprovalPolicy::decide` | private | 只接受已知 user MCP provenance 并返回 one-time approval | 不自动批准远端副作用 |
| `CompositeToolService` | private | model tool name → frozen local/MCP service | duplicate name 在 composition 时失败 |
| `CompositePolicyService` | private | trusted `ActionSource` → owning policy | 不依靠 trial-and-error policy fallback |
| `ReloadableToolPorts` | crate-private | 原子替换未来 Tool generation，并为 prepared call 固定 service/policy | reconcile failure 保留上一份可用 runtime |
| `ModelSnapshotResolver` | private trait | frozen config → immutable invoker | implementation 不持有 mutable config view |
| `zeta_slash_commands::SlashCommandCatalog::new` | shared public constructor | 校验 lowercase ASCII/interior-hyphen name、非空描述与唯一性 | App Server 不复制 grammar、不执行命令、不引用 client-local commands |
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

`serve_jsonl` 对每一行先写 response，再 drain/write causal notifications。当前 JSONL loop 是
同步串行的；owned embedded client 通过 `ConnectionNotifications` 在独立 event pump 中等待同一
queue 的 condition-variable wake。protocol registry 中的 `SerializationScopeDefinition`
尚未接入并发 scheduler。

Terminal 因此使用 bounded `terminal/read` pull：`TerminalService` 在独立 runtime 中持续 drain
PTY raw bytes，保留最多 1 MiB，并按 sequence 返回最多 128 个 chunk。`terminal/write` 的单批
UTF-8 输入上限为 64 KiB，rows/cols 上限均为 512；未知 ID、跨 connection 使用和 runtime
capacity 分别映射稳定 Terminal error。`serve_jsonl` 结束时调用 `close_owner`，不会把 PTY
留给失效 connection。

`terminal/profile/list` 从 composition 时冻结的 `TerminalProfileCatalog` 返回安全显示信息；
`terminal/create.profile` 只能选择 default 或已列出的稳定 ID。Windows catalog 可发现 Command
Prompt、PowerShell、Git Bash，以及 PATH 中已安装的 Zsh；Unix catalog 可发现默认 Shell 与已
安装的常见 Shell；路径、args 和 environment 始终留在 Rust authority。Host 只传入明确的
session/environment allowlist，`TerminalEnvironment` 再排除未拥有变量并固定终端 identity；
底层 PTY spawn 先 `env_clear`，不会意外继承 App Server 的其他环境。客户端提交 unknown
terminal params（包括 `terminal/create.environment`）会在协议 decode 时返回 `InvalidParams`。

Initialize 是每 connection 一次。重复 initialize 返回 `AlreadyInitialized`；初始化前的其他 method
返回 `NotInitialized`。Request ID 只接受正整数，且在 connection 生命周期内不能重复。
成功结果同时包含 composition 时冻结的 `slashCommands`。不同 connection 可获得同一 server
snapshot；单个 connection 生命周期中不会因 host 后续状态变化而更换 popup contract。

## Session、Thread 与 Turn 编排

典型 create path：

```text
session/request { type: createThread }
├─ SessionCoordinator::create_thread
├─ subscribe caller to new Thread
├─ notify_session_updates(previous session sequence)
├─ notify_thread_updates(0)
└─ return current Session + ThreadId
```

`session/request { type: rewindThread }` 是非破坏性回退：`SessionCoordinator::rewind_thread` 记录包含 source Thread、
source sequence 与 excluded Turn 的 Rewind lineage，再让 `ThreadController::create_rewound_thread`
向新 Thread 写入单个 `HistoryImported` durable event。该事件只携带 checkpoint 之前的 terminal
Turns；source Thread 及其后续历史不被改写。调用方随后订阅新 Thread，旧 Thread 仍可审计和恢复。

`session/subscribe(afterSequence)` 是产品宿主使用的 aggregate port：它返回 Session snapshot、Session
durable gap 和每个 child Thread 的 projection，并在同一 connection 上建立
`session/thread/update` child update delivery。
`session/request` 是产品 mutation 的 canonical aggregate port：它携带一个 `CommandId`、Session
sequence 和 typed `SessionRequest`，统一路由 Session lifecycle、child Thread lifecycle、Turn
start/interrupt 与 interaction resolve，并以 tagged `SessionRequestResult` 返回对应结果。
旧的独立 Session/Thread/Turn mutation methods 已从 registry 和 dispatcher 删除；所有 mutation
都通过 `session/request`。Thread snapshot 和 gap 使用带 Session scope 的
`session/thread/read` / `session/thread/subscribe`。这些订阅都是 connection-local delivery state；
真实 gap 来自 coordinator/store。

`session/request::StartTurn`：

1. 校验 Thread 属于 supplied Session；
2. 校验 Session 仍为 active；
3. 读取 Session 当前模型，并把它作为 `TurnAccepted` 的 durable snapshot；
4. `start_turn` 使用 typed command ID + exact expected sequence；
5. replay 时读取既有 Turn，terminal failure/interruption 不伪装成 success；
6. 新 start 发布 durable update 后调用 `TurnExecutor::start`。

`model/list` 由 `ModelCatalog` 投影当前已配置 provider 的可选模型；`session/request::SetModel`
先通过同一 catalog 校验，再提交 Session command。全局 `preferredModel` 只作为新 Session
和历史无模型 Session 的默认值，不承担当前 Session 的模型切换。

`session/request::ResolveInteraction` 使用 exact durable
`RequestId`，且只接受 `UpdateBroker` 选出的、声明该 kind 并订阅 scope 的 connection；full request
只通过 `agent/request` 投递，普通 Thread snapshot 保持 redacted。owner 断连或退订会确定性重选，
非 owner 返回 `AgentInteractionNotOwner`，过期响应返回 `AgentInteractionExpired`。当 response 是 Tool Call 对应的 approval，
且 Core 确实产生 `Resolved` disposition，App Server 再启动 executor 恢复 Tool path。这个判断依赖
pending interaction 的 item binding，不能简化为“所有 approval 都 restart”。
同一路径同时恢复执行前 approval 与带结构化 `sandboxDenial` 的 sandbox escalation approval；
App Server 不解释或扩大授权，Core 会在恢复后重新校验 action、policy、capability 与 ToolCall
binding，并保证升级重试最多启动一次。

`InteractionDeadlineWatcher` 每 50 ms 检查 broker 已知的 pending request，并在同一个
`workspace_authority_gate` 内重新读取 Core snapshot；只有 exact request 仍 pending 且 wall-clock
deadline 已到时，才持久化 `InteractionCancelled(DeadlineElapsed)` 和稳定、可重试的
`InteractionDeadlineElapsed` Turn failure。deadline policy 属于 App Server delivery/runtime，TUI
只消费最终 snapshot；Core reducer 不运行 timer。

## 更新代理

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

`notification_queue::NotificationQueue::{push,extend}` 在空 queue 获得新值时唤醒 listener；每个 connection 最多保留
4096 条通知。达到上限时先清除可重建的 transient Thread notification，使下一 cursor gap 触发
snapshot resync；如果 4096 条都是 must-deliver control notification，则关闭 queue，让 client 走
connection failure/recovery，而不是丢 control fact 或继续增长内存。
`AppServer::close_connection` 显式 unregister subscriber 并唤醒 blocked listener。Connection
若未显式 close，最后一个 strong owner drop 后 broker 仍会在下一次 publish 时通过
`Weak::upgrade` 失败清除 subscriber。共享 embedded client 和 TUI event pump 也使用 1024 项有界
channel，把 slow-consumer 背压传回该 queue。

## Local composition 与模型安全点

`open_local_app_server` 的顺序：

```text
SessionStateMode::Durable
└─ LocalStateRepository::open(profile_root)
   ├─ SqliteSessionStore(profile_root/state.sqlite3)
   ├─ SqliteThreadStore(profile_root/state.sqlite3)
   └─ recover_coordinator

SessionStateMode::Ephemeral
└─ SessionCoordinator::with_store(InMemorySessionStore, ThreadController::with_store(InMemoryThreadStore))

ConfigStore::open_with_paths(profile_root/state.sqlite3, profile_root/config.toml)
└─ read_snapshot preflight

Workspace authority
├─ host-configured initial root → HostConfiguration capability
└─ client workspace/switch → latest WorkspaceTrustConfig lookup
   ├─ missing / Restricted → filesystem + watcher + local code index + customizations
   └─ Trusted → ExplicitUserDecision capability + optional cloud controller when providers exist

ConfigChange trust revocation
├─ revoke shared capability lease
├─ persist cloud Revoking + request idempotent provider deletion + remove cloud controller
├─ remove local Tool / Git / search / terminal ports
├─ terminate PTY and search processes
├─ interrupt active Turns
└─ retain restricted filesystem + watcher + local code index + customizations

optional WorkspaceConfigStore
└─ WorkspaceConfigTracker::read preflight

ConfigBackedModelService
└─ AppServer::new(...).with_config_store(...)
   └─ SkillRuntime::new(...)
      ├─ release built-in root
      ├─ enabled user Skill source roots
      ├─ active Workspace `.zeta/skills`
      └─ SkillWatcher(source events + ConfigChange + Workspace binding)

Workspace activation
└─ WorkspaceCustomizations::discover
   ├─ InstructionCatalog(.zeta/instructions)
   │  └─ Global content → HarnessInstructionsProvider
   ├─ AgentDefinitionCatalog(.zeta/agents)
   └─ FileSystemWatcher invalidation → bounded catalog refresh

local tools + enabled user MCP declarations
├─ materialize absolute stdio executable / unauthenticated HTTP endpoint
├─ combine duplicate-free model Tool names
├─ install ReloadableToolPorts
└─ ToolConfigWatcher(ConfigChange)

Plugin request + Hook declaration
└─ config/read + typed mutation only
   ├─ Plugin install/activation manager 尚未实现
   └─ Hook execution/policy runtime 尚未实现

Language-server preference
└─ config/read + languageServer/configure|remove
   └─ App Server 持久化 mode/path；Native catalog/runtime 负责解析与执行
```

每次 `ModelService::invoke` 重新读取 user config，与 optional workspace document 合并，再由
`ModelSnapshotResolver` 生成 immutable invoker。因此 config change 影响下一次 invocation，不会
改变已经运行的 invocation。`ProviderModelService` 把 Core token 传入
`ModelInvoker::invoke_with_cancellation`；取消被保留为 `CoreError::Cancelled`，不会降级为普通
model failure。production provider operation 会立即停止本地等待、禁止 retry，并丢弃同步 HTTP
attempt 的迟到 response。

## Skill 目录运行时

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

session/request StartTurn { SkillRef }
├─ start_turn::replayed_result
│  └─ existing command → validate exact input and return durable result without mutable reads
└─ new command
   ├─ SkillRuntime::activate_explicit
   │  ├─ require enabled + compatible catalog entry
   │  └─ SkillCatalog::activate → exact body + frozen digest/generation/reason
   ├─ ThreadEvent::TurnAccepted persists FrozenSkillActivation
   └─ TurnExecutor safe point → SkillInstructionsProvider::resolve frozen digest → ContextPlan
```

Watcher 订阅当前 source roots；Config authority 通过 commit channel（包括 TOML 外部编辑与
SQLite cross-connection change）触发配置 reconcile。change、overflow 或 backend rescan 提示都只触发
完整 reconcile；entry、diagnostic 或 enablement projection 没变时 generation 不变，也不发
notification。Watcher 启动失败时 local App Server 仍可用，显式
`skills/list { reload: "refresh" }` 是恢复路径。

当前组合 built-in、user 与 active Workspace `.zeta/skills` source。显式正文 activation、durable
provenance、通用 `SkillInstruction` context injection 和 invocation safe-point reload 已实现；禁用只影响 future Turn
eligibility，不能改变已经冻结的 Turn。Plugin source、reference/resource resolver、automatic
selection 尚未实现。TUI 与 Desktop 已从 metadata-only catalog 生成直接 `/name` Skill command，
而 `/skills` 只承担管理；正文变化或 source 消失会使恢复/后续 safe point 失败即
关闭，不会用新 bytes 替换 frozen digest。

MCP runtime 当前只在 `open_local_app_server` safe point 构造。`enabled` 是建立连接/启动 server
的显式用户 intent；它不批准任何 tool call。stdio command 必须是存在的 absolute executable，
HTTP 当前只接受 unauthenticated endpoint，credential reference 会使 composition 明确失败。
Workspace MCP intent 仍保持 pending trust，不会接入。Config mutation 或 `tools/list_changed`
不会原地改 catalog：list-changed 后旧 runtime fail closed；当前需重启 App Server 才会构造新
generation。启动采用 `RequireAll`，任一 enabled server 无法 initialize 时 App Server 明确启动
失败，不会静默丢失部分 catalog。

缺少 preferred model/provider 时创建 `UnavailableModel`，使 invocation 显式失败，不回退到 echo
或任意默认 provider。

## 审查模型适配器

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

## 资源存储

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

## 错误映射

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
| missing / initial code index | `CodeIndexUnavailable` / `CodeIndexNotReady` |
| index I/O、SQLite 或 stale materialization | `CodeIndexOperationFailed` |
| missing cloud controller | `CloudCodeIndexUnavailable` |
| invalid/conflicting cloud grant | `CloudCodeIndexInvalidGrant` / `CloudCodeIndexConsentConflict` |
| cloud byte ceiling exceeded | `CloudCodeIndexEgressLimitExceeded` |
| missing capability/deletion guarantee | `CloudCodeIndexProviderUnavailable` |
| cloud persistence/publication/deletion failure | `CloudCodeIndexOperationFailed` |
| retrieval local failure / source mismatch | `CodeRetrievalOperationFailed` |
| missing search backend | `SearchUnavailable` |
| unknown/cross-connection search job | `SearchNotFound` / `SearchNotOwner` |
| search job capacity exhausted | `SearchBusy` |
| rg spawn/parse/exit failure | terminal `WorkspaceSearchReadResult.error` with stable redacted text |
| missing terminal backend | `TerminalUnavailable` |
| unknown/cross-connection Terminal | `TerminalNotFound` / `TerminalNotOwner` |
| terminal capacity/runtime operation failure | `TerminalBusy` / `TerminalOperationFailed` |
| poisoned lock/serialization invariant | `ServerOverloaded` or `InternalError` |

External errors不携带 `CoreError`、`ConfigError` 或 backend error text。新增 error mapping 时先更新
protocol enum，再在本 crate 显式转换。

## 方向偏差检查

- `server.rs` 定义新的 RPC params/result：contract ownership 从 protocol crate 漂移；
- App Server 直接构造/store event 或执行 reducer：Core ownership漂移；
- App Server 直接解释 SQLite Session/Thread tables：repository/storage ownership 漂移；
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

## Typst 集成

`AppServer::typst_compile` 拥有从 `document/typst/compile` 到
`zeta_typst::TypstCompiler` 的应用级桥接。它把预期内的源码失败映射为
`TypstCompileResult::Failed`；成功时创建由当前连接拥有、有效期为 300 秒的
`application/pdf` 资源；并通过 `typst_diagnostic_dto` 把编译器诊断映射为协议数据结构。

编译器边界属于 `zeta-typst`；本 crate 不得在其周围增加宿主路径解析、包下载或字体发现。
把当前连接拥有的临时资源改为持久化文档存储，是另一项所有权决策。跨 crate 信任模型见
[`docs/typst.md`](../../docs/typst.md)。

测试覆盖初始化/请求 ID、Session 优先流程、命令重放/冲突、分叉谱系、Turn 重放/模型只调用一次、
多连接更新、重连后的持久化缺口、有界通知 overflow、连接拥有的资源、配置命令、owner-directed
交互/批准解决、断连重选与 deadline、
先响应后通知、模型配置安全点、
Workspace override、review-only request、只读 `rg` definition/materialization/policy/execution，
MCP worker bridge、exact provenance/approval policy、local/MCP 路由与 collision rejection、
Config commit/cross-connection notification、future Tool generation replacement 与 prepared-call
generation retention，以及
可信 Terminal Profile、真实 PTY create/write/read/exit、Terminal owner/error/ring limits，
Skill 内置/用户/工作区来源组合、启用状态叠加、监听器刷新、精确激活、摘要变化失败即关闭、
删除源文件后的 command receipt 重放与 `skills/changed`；工作区定制覆盖全局指令注入、Agent 目录刷新和
`AGENTS.md` 非原生来源隔离。
Git 覆盖 workspace projection、runtime stream identity、revision 去重、`git/statusChanged`、
text diff、local branch list/switch、path mutation 与 commit。Filesystem 覆盖有界原子写入、权限保留、root containment、
相对路径 `fs/changed` 与 watcher overflow rescan。
Syntax 覆盖 connection owner、revision mismatch、Unicode UTF-16 batch 与非重叠 token encoding。

local tool 的参数白名单、discovery、取消与输出限制由
[`zeta-shell-command`](../shell-command/README.md) 和 [`zeta-exec`](../exec/README.md) 维护；
本 README 只拥有 App Server 组合与 Core port binding。

当前 JSONL 服务仍使用同步循环；自有嵌入式连接已具备可唤醒通知来源，以及显式订阅、资源和
终端清理；尚无异步多连接调度器、序列化范围强制、持久化资源或完整网络服务
生命周期。MCP desired config 的热更新和未来 Tool catalog replacement 已实现；当前仍没有凭据
具体化、stdio 进程沙箱、对外 runtime health/diagnostic API、progress/elicitation delivery 或
image result 的原生 Core content path；MCP image 暂时编码进 bounded JSON text result。演进这些
能力时应保留 protocol registry唯一性、Core/store authority、snapshot + durable gap 和
per-invocation config safe point。

## Static extension resources

The reusable filesystem catalog is owned by [`zeta-extensions`](../extensions/README.md).
This crate only resolves host roots, adapts catalog values to protocol DTOs, and stores opened
bytes in the connection-owned `ResourceStore`. `zeterm` should depend on `zeta-extensions`
directly; it should not depend on App Server or desktop transport code.
