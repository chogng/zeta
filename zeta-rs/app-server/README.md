# `zeta-app-server`

> 本 README 解释 JSON-RPC dispatch、local composition、update broker、resource store 与 review
> model adapter。External method contract 见
> [`docs/zeta-app-server-api.md`](../../docs/zeta-app-server-api.md)，canonical product model 见
> [`docs/protocol.md`](../../docs/protocol.md)，workspace 搜索的跨层 ownership 见
> [`docs/search.md`](../../docs/search.md)，外部 MCP client runtime 的跨 crate 语义见
> [`docs/mcp.md`](../../docs/mcp.md)，Git/SCM 跨进程 ownership 见
> [`docs/git.md`](../../docs/git.md)，Workspace identity 与 trust boundary 见
> [`docs/workspace-security.md`](../../docs/workspace-security.md)。跨文件内容搜索的执行实现见
> [`zeta-rs/workspace-search/README.md`](../workspace-search/README.md)。Desktop 浏览器能力的跨进程所有权和当前限制见
> [`docs/zeta-desktop-architecture.md`](../../docs/zeta-desktop-architecture.md#7-浏览器能力)。Marketplace
> Manager client composition、capability handoff、迁移边界和启动失败隔离见
> [`docs/marketplace-integration.md`](../../docs/marketplace-integration.md)。

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
   ├─ MultiAgentCoordinator → child Thread spawn/context seed + durable delivery/join/cancellation
   ├─ TurnBackendHandle → local TurnExecutor (API-key and subscription model targets)
   ├─ ConfigStore
   ├─ optional WorkspaceFileSystem + filesystem watcher
   ├─ optional GitRuntime → zeta-file-watcher + GitService → zeta-git
   ├─ optional WorkspaceSearchService → zeta-workspace-search
   ├─ AgentGrepService → frozen rg | private UDS client → zeta-fast-regex-search worker
   ├─ optional CodeIndexRuntime → zeta-code-index + filesystem watcher
   ├─ optional SymbolIndexRuntime → zeta-symbol-index + CodeIndex source/overlay authority
   ├─ optional CodeIndexSemanticService → local SQLite vectors + host model adapters
   ├─ optional CloudCodeIndexController → zeta-code-index-cloud + host provider registry
   ├─ request-scoped CodeRetrievalService → symbol/lexical/semantic/remote RRF + verification + budget
   ├─ optional TerminalService → zeta-utils-pty
   ├─ optional LspServerProviders → zeta-lsp-manager → zeta-lsp
   ├─ optional MarketplaceManager → local installation state + remote client
   │                              → Skill/MCP/Connector/Theme/Language/Editor Extension consumers
   ├─ reloadable MCP Tool generation + status/OAuth/form interaction → zeta-mcp-extension → zeta-mcp
   ├─ ConnectorCredentialService → list/connect/disconnect + ready MCP reconcile
   ├─ client-hosted dynamic tools → durable Agent interaction owner
   ├─ BrowserHost → connection-owned reverse JSON-RPC browser capability
   │              → built-in browser Tool port + one-time approval policy
   ├─ read-only/capability ToolExecutor contributions → zeta-extension-api
   ├─ optional Web Search executor → zeta-web-search-extension
   ├─ trusted Workspace coding tools → read_file / write_file / edit / grep / glob / apply_patch / shell-command / update_plan
   ├─ trusted Workspace Agent tools → spawn_agent / send_agent_message / wait_agent
   ├─ Thread Goal tools → get_goal / create_goal / update_goal
   ├─ Skill RPC/config/event adapters → zeta-skills-extension
   │                                  → zeta-skills + zeta-file-watcher
   ├─ WorkspaceCustomizations → zeta-instructions + zeta-agents
   ├─ connection-owned ResourceStore
   └─ UpdateBroker → session/update, session/thread/update, config/changed, connector/changed, skills/changed, git/statusChanged, fs/changed
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
| `AppServer::connection_notifications` | 返回可阻塞等待、主动唤醒的 connection outbound message source |
| `AppServer::close_connection` | 注销 subscription、关闭 notification source，并释放 Resource/Terminal/Syntax owner 与 Marketplace leases |
| `AppServer::handle_json` | 处理一个 JSON-RPC request string |
| `AppServer::drain_notifications` | JSONL 与同步适配 caller 取出该 connection 的 serialized notifications |
| `AppServer::{serve_stdio,serve_jsonl}` | 同步 JSON Lines service loop |
| `AppServer::create_resource` | 创建 5 分钟 TTL 的 connection-owned resource |
| `AppServer::with_config_store` | 开启 config/provider/MCP/Skill RPC |
| `AppServer::with_connector_service` | 开启 Connector RPC、capability 与 generation notification watcher |
| `AppServer::with_slash_command_catalog` | 安装 initialize 时下发的 immutable 动态命令 snapshot |
| `AppServer::with_file_system` | 注入受 workspace 约束的 filesystem authority |
| `AppServer::with_file_system_watcher` | 监听可信 workspace root 并发布相对路径 invalidation hint |
| `AppServer::with_git_root` | 冻结 workspace root，开启 Git status/mutation、watcher 与 revision notification |
| `AppServer::with_workspace_search` | 注入 workspace root 与冻结的 ripgrep executable，构造外部内容搜索服务 |
| `AppServer::with_language_server_providers` | 注入 `LspServerProviders` provider collection；activation confirmation 启用 packaged route，显式 Config `Disabled` 仍可关闭 |
| `AppServer::with_marketplace_manager_client` | 持有 Zeta 本地 Manager 的 typed business client，广告 `marketplace` capability 并投影通用 package/capability RPC |
| `AppServer::with_local_marketplace_manager` | 除业务 RPC 外，绑定只在进程内可见的 verified capability source；不会向 transport 暴露 host path |
| `AppServer::with_marketplace_language_runtime` | 将通用 Marketplace 已验证的 Language/Executable capability 投影为本地 `LspServerProviders` collection；不拥有发现、下载或安装 |
| `AppServer::with_marketplace_editor_extension_admission` | 注入 Marketplace executable 的产品 enable/grant generation、通知与 drain authority；安装本身不会执行代码 |
| `AppServer::with_extension_host_runtime` | 将 legacy Plugin 与已授权 Marketplace executable 规范化为同一 deployment fleet；Host 不解析 package manifest |
| `LocalProfileRuntime::clear_workspace_indexes` | 显式清理一个未使用工作区的四类可重建本地索引 |
| `LocalProfileRuntime::clear_all_workspace_indexes` | 显式清理当前 profile 下所有未使用的工作区索引 |
| `LocalCodeIndexProviders::with_semantic_models` | 在 Workspace activation 前注入本地 semantic 使用的 immutable embedding/rerank adapters |
| `AppServer::with_cloud_code_index_providers` | 注入冻结的 provider registry；空 registry 不广告 cloud capability |
| `AppServer::with_cloud_code_index_storage_root` | local composition 配置按 root identity 分隔的 durable grant/deletion state |
| `AppServer::with_tool_service` | 安装同一 server 内所有 Turn 使用的 Core Tool/Policy ports；正常 local composition 另在 Turn 接受时冻结 `coding-v1` snapshot |
| `AppServer::with_mcp_oauth_service` | 安装独立 Config MCP 的 process-local OAuth coordinator，并广告 `mcpOAuth` capability |
| `AppServer::with_dynamic_tools` | 校验 client-hosted dynamic specs，并接入共享 registry、审批和 durable interaction 执行链 |
| `AppServer::resume_recovered_agent_coordinations` | 恢复 Agent spawn/delivery/join/cancellation saga，并调度恢复期间新建的 child Turn |
| `AppServer::resume_recovered_goal_continuations` | 重启后恢复 idle active Goal 的隐藏 continuation Turn |
| `open_local_app_server` | 按 SessionStateMode 选择 durable/in-memory coordinator，打开 config 并组合 provider-backed model |
| `open_local_app_server_with_cloud_providers` | 在 Workspace 激活前注入 cloud code-index providers；默认入口使用空 registry |
| `open_local_app_server_with_code_index_providers` | 在 Workspace 激活前同时注入本地 semantic models 与可选 cloud providers |
| `LocalProfileRuntime` | 单进程、单 profile 的 Session/Thread projection、Config 与跨 Workspace Session notification authority |
| `LocalAppServerOptions` | user profile root + SessionStateMode + optional Workspace/Connector/language-provider runtime + validated slash catalog + built-in Skill root selection + optional model operation client/MCP OAuth providers |
| `LocalAppServerOptions::with_profile_runtime` | 在 profile daemon 内复用 durable authority，同时为连接组合独立 Workspace runtime |
| `LocalAppServerOptions::with_language_server_providers` | 在 local App Server 启动前注入额外 `LspServerProviders`；receipt providers 由 composition 自动合并 |
| `LocalAppServerOptions::with_marketplace_registry` | 组合进程内 Zeta Manager 与 product-pinned HTTPS/TUF registry client |
| `LocalAppServerOptions::with_marketplace_manager_client` | 注入由外部 supervisor 或测试拥有的 Manager client |
| `LocalAppServerOptions::with_mcp_oauth_providers` | 把 exact MCP server ID → provider adapter 注入使用共享 SecretStore 的 OAuth service |
| `LocalConnectorRuntime` | Connector credential service + shared SecretStore + legacy Plugin/Marketplace MCP providers |
| `LocalAppServerOptions::with_plugin_activation` | 从 exact activation 构造 package-rooted Connector/MCP runtime |
| `LocalConnectorRuntime::from_plugin_activation` | activation → Connector catalog + SQLite authority + Plugin MCP provider |
| `SessionStateMode` | 明确选择 profile SQLite durable history 或 process-local ephemeral Session/Thread state |
| `BuiltInSkillRoot` | auto-detected release root、explicit test/host root 或 unavailable 的自解释选择 |
| `zeta_slash_commands::SlashCommandCatalog` | 委托共享 crate 校验动态命令并冻结 server-advertised snapshot；App Server 只拥有 composition |
| `ReviewModelResolver` | 从 frozen config snapshot 选择 review-only model |
| `ProviderReviewModel` | `ModelInvoker → zeta_auto_review::ReviewModel` adapter |

`AppServer::new` 默认用 `TurnExecutor::without_tools`。`with_tool_service` 才会替换为有 Tool 和
Policy port 的 executor。`open_local_app_server` 会从 user config snapshot 连接明确 `enabled`
的 unauthenticated 或 SecretStore-backed MCP server，并把 catalog 与本地工具组合。Host 可用
`LocalAppServerOptions::with_plugin_activation` 注入固定 activation，或用 `with_plugin_authority` 注入 live
authority；两者都会自动构造 Connector catalog、SQLite authority 与 package-rooted Plugin MCP provider。
没有注入时，local composition 打开 `<profile>/plugins` 的 durable `PluginActivationAuthority`。`LocalProfileRuntime` 同时打开 `<profile>/secrets` 的唯一 `FileSecretStore`，并向该 profile 内所有 Workspace App Server 共享同一 `Arc<dyn SecretStore>`。显式 embedded host 仍可注入 `KeyringSecretStore` 或其他 backend；共享 profile runtime 时，如果 Connector runtime 携带了另一个 store，composition 会拒绝启动。Authority generation 会重建 Connector definitions 与 Plugin MCP provider，并复用 MCP safe-point replacement。Config、Connector
或 MCP list-changed hint 会在后台构建新 generation；每次 model invocation 同时冻结可见 definitions 和响应后的 binder，因此 watcher 在模型
响应前发布新 registry 也不会把旧响应劫持到同名新工具。已绑定调用继续持有原 Tool/Policy generation，
直到 execute 排空；Plugin-backed call 在 dispatch 前获取 exact activation lease，Connector-bound call 再
额外复核 live connection generation/digest，
disconnect 会等待已经 dispatch 的调用结束。每次 MCP tool
call 仍必须经过 durable one-time approval。Host 通过
`LocalAppServerOptions::with_mcp_oauth_providers` 注入具体 provider 后，local composition 使用 Connector
runtime 同一个 SecretStore 构造 `McpOAuthService`；App Server 只暴露 PKCE/callback/refresh/revoke RPC，
不拥有 provider discovery、scope 或 token wire。Host 安装的 read-only
extension executor（当前包括统一的 `skills-read`）和 client-hosted dynamic tools 也进入同一个 registry，
不得绕过 binding、policy 或 durable result commit。它仅在调用方通过
`LocalAppServerOptions::with_workspace_root` 提供统一 Workspace 根时同时组合 filesystem、
`.zeta` 自定义 catalog、Workspace code index、Workspace search、Git SCM、connection-owned/leased Terminal runtime、
只读 `rg` registry；Zeta CLI 的 stdio 与
in-process 路径都会使用同一个启动时解析结果：
`ZETA_WORKSPACE_ROOT` 优先，否则使用当前目录。不能因为 protocol 暴露 approval interaction 就
假设任意自定义 host 已经拥有 Tool registry。`rg` 安装候选来自
[`zeta-install-context`](../install-context/README.md)，App Server 只负责把候选交给
`RipgrepExecutable` 验证并组合成 Tool service。

Web Search 默认不注册。Embedding Tool Search 只负责在当前 registry 中找工具，与外部网页检索无关。
Host 只有通过 `LocalAppServerOptions::with_web_search_backend` 注入
`zeta-web-search-extension::WebSearchBackend` 后，`web_search` 才进入 eager tool set；其 endpoint host
和 optional credential reference 会冻结到 action digest，并要求 exact one-time approval。共享扩展仅
提供 provider-neutral contract 与 JSON HTTP adapter，不硬编码 Codex 私有 Search endpoint。

默认 local composition 使用 lazy production model client：启动、配置读取和模型目录展示不会加载
system roots 或 proxy。Embedded test 可通过 `LocalAppServerOptions::with_model_operation_client`
注入离线 client；真实 transport 初始化失败在第一次模型 operation 返回，不得让 App Server 启动 panic。

local composition 会配置 `<profile>/code-index-cloud` 的 durable state 位置，但默认 provider registry
为空，因此不会安装 cloud controller、广告 `cloudCodeIndex` 或创建网络请求。具体 host 只有在注入
接受 Workspace-owned exact chunks 且满足幂等 grant deletion 的 provider 后，才能启用云能力。

local composition 把可重建索引统一放在
`<profile>/cache/workspaces/<workspace-digest>/indexes/{agent-grep,lexical,symbols,semantic}`，并通过
`<profile>/cache/locks` 下的独立锁文件协调多个进程。它同时安装共享的
`SemanticModelProvider` resolver，但 semantic CodeIndex 默认仍关闭，所以不会后台发送 chunks 或创建
vectors。用户选择模型并对 exact Workspace 授权源码外发后，Trusted Workspace 的 lexical generation
更新才会在 refresh worker 中同步本地 semantic SQLite。模型只返回 embedding/rerank 结果，召回与
排序仍由本地 domain crate 完成；host 也可用 `LocalCodeIndexProviders::with_semantic_models` 注入测试
或专用 immutable adapters。

Fast Regex 提供 `workspace/agentGrep/fastRegex/status|rebuild|disableAndDelete`。普通配置切换到
`ripgrep` 只关闭并释放常驻 worker、保留磁盘索引；`disableAndDelete` 先提交配置切换，再删除对应项目的
Agent grep 索引，并明确返回删除成功、原本不存在或仍在使用。

Trusted Workspace 同时获得 built-in read-only `search_code` 工具。它只接受
`workspace-code-index-read-only` exact grant，调用 canonical `CodeRetrievalService`，并返回 bounded、
current-source-verified excerpts 与 degradation；未配置 semantic 时自然退回 lexical。semantic grant
精确绑定 Workspace、model selection 与 provider config，provider URL 或模型变化会卸载旧 runtime 并要求重新授权。`provider/list` 通过 `zeta-model-provider::ProviderCredentialService` 从 built-in provider registry 读取供应商名称、API Key 策略与是否已配置；`provider/apiKey/set` 只负责协议转换并委托同一服务写入 profile `SecretStore`，App Server 不复制密钥校验、SecretKey 或 Header 规则，也不把密钥写入普通配置、响应或日志。

Tool Search 拥有独立的 `toolSearch.embeddingModel`，不复用 CodeIndex 的模型选择。只有 User Config
明确设置 `toolSearch.mode = "hybridEmbedding"` 才会调用；默认 `lexical` 不产生 embedding 请求。
`toolSearch/configure` 先解析 exact provider/model 并执行固定文本 readiness probe，失败返回
`ToolSearchUnavailable` 且不提交配置。外部配置或启动恢复遇到不可用模型时，App Server 通过
`config/read.toolSearch.embeddingStatus` 报告原因，hybrid 自然语言搜索明确失败；只有用户显式切回
`lexical` 才启用纯 BM25。Regex 始终是本地显式策略。门禁通过后，Tool Search 只缓存当前 registry
generation 的工具向量；实际调用或响应校验失败会使该次搜索明确失败，不静默回落到 BM25。

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
│       ├── connector_operations.rs # list/API-token connect/disconnect DTO adapter
│       ├── connector_runtime.rs   # authority generation → connector/changed fanout
│       ├── skill_operations.rs    # Skill catalog/enablement/resource DTO conversion and error mapping
│       ├── start_turn.rs           # durable command replay before mutable model/Skill resolution
│       ├── workspace_customizations.rs # Instruction/Agent catalogs + reloadable harness snapshot
│       ├── fs_operations.rs       # root-relative filesystem DTO conversion/error mapping
│       ├── fs_watcher.rs          # root watcher、内部 observer、相对路径投影与 fs/changed 发布
│       ├── git_operations.rs      # Git RPC decode 与稳定错误映射
│       ├── git_runtime.rs         # status projection/revision、watcher、去重与通知
│       ├── interaction_runtime.rs # pending interaction deadline enforcement
│       ├── multi_agent_tools.rs   # Core coordinator Tool adapter + child Turn scheduling
│       ├── notification_queue.rs  # bounded per-connection queue + wake/close semantics
│       ├── search_operations.rs   # search RPC decode、ownership 与稳定错误映射
│       ├── code_index_operations.rs # status/search/rebuild DTO 与 error mapping
│       ├── code_index_runtime.rs  # generation lifecycle、watcher reconcile 与 stale gate
│       ├── cloud_code_index_operations.rs # preview/grant/sync/revoke DTO 与稳定错误映射
│       ├── terminal_operations.rs # terminal RPC decode、ownership 与稳定错误映射
│       ├── language_runtime.rs   # workspace state + provider definitions → shared LSP manager
│       ├── marketplace_operations.rs # Manager business RPC、稳定错误与 connection-owned lease
│       ├── marketplace_projection.rs # Manager contract → App Server DTO 的无路径机械投影
│       ├── marketplace_language_runtime.rs # 已验证 Language/Executable capability → LspServerProviders
│       ├── marketplace_extension_sources.rs # 已验证 Language capability → Extension catalog source
│       ├── marketplace_skill_sources.rs # 已验证 Skill capability → Skill catalog source
│       ├── request_serialization.rs # cross-connection scope FIFO/shared-read scheduler
│       └── update_broker.rs       # per-connection subscription/cursor/fanout
├── local.rs                       # local composition, session backend selection + model safe point
├── local_tools.rs                 # frozen rg registry + Core Tool/Policy adapters
├── browser_host.rs                # reverse JSON-RPC broker、target owner、timeout/cancellation + screenshot resource binding
├── browser_tool.rs                # semantic browser Tool definitions、input validation 与 approval policy
├── dynamic_tools.rs               # dynamic spec validation + durable interaction Tool adapter
├── extension_tools.rs             # read-only/capability contributors → reviewed ToolExecutor port
├── tool_executor_adapter.rs       # frozen payload/binding → ToolExecutor invocation
├── tool_composition.rs            # local/MCP/dynamic/extension routing + generation-safe replacement
├── review.rs                      # review-only provider adapter
├── resource_store.rs              # per-resource/per-connection bounded in-memory resources
├── git_service.rs                 # workspace root + GitClient + synchronous RPC runtime bridge
├── terminal_environment.rs        # secret-excluding host environment、platform normalization 与 terminal identity
├── terminal_profiles.rs           # trusted Shell discovery、ID 与 frozen environment
└── terminal_service.rs            # PTY runtime、output ring、connection owner 与 reconnect lease
```

## 内部接口地图

| Symbol | 可见性 | 当前职责 | 方向约束 |
| --- | --- | --- | --- |
| `AppServer::dispatch` | private | initialization gate 后对 `ClientMethod` exhaustive dispatch | method string lookup只来自 protocol registry |
| `RequestScheduler` | private | 对 registry 解析出的 global、Session 和 connection-resource key 实施 FIFO/shared-read 调度与 queued disconnect cancellation | 不解析 params、不拥有 transport accept、不做 CPU/token 配额 |
| `ConnectionDispatchActivity` | private | 在多个 request worker 与 notification waiter 之间维持 response-before-causal-notification barrier | 不决定 method scope，不跨 connection 共享背压 |
| `AppServer::session_request` | `pub(super)` | 解码 canonical `session/request`，按 typed operation 调用 Session/Thread/Turn helper 并发布 durable gaps | 不在 App Server reducer 或 `zeta-protocol` 中复制 mutation model |
| `decode<T>` | crate-private | params JSON → typed DTO，统一 InvalidParams | operation 不手读 arbitrary fields |
| `result<T>` | crate-private | typed result → JSON，统一 serialization failure | external result shape 由 protocol DTO 决定 |
| `core_error` | crate-private | Core error → stable App Server error | 不回传 internal error string |
| `RpcError` | crate-private | internal code + stable `AppServerErrorName` | 不成为第二套 public error DTO |
| `notify_session_updates` / `notify_thread_updates` | private methods | 从 durable store gap 读取并交给 broker | mutation 后从 authoritative history 发布 |
| `AppServerThreadUpdates` | private sink | 将 TurnExecutor live/committed update 接入 broker | 不修改 canonical update |
| `UpdateBroker` | private | subscription、durable cursor、weak queue fanout | 不持有 connection/session runtime authority |
| `UpdateBroker::publish_thread_update` | private | committed 按 durable cursor；transient 直接给 subscriber | 两类 cursor semantics 不得混合 |
| `BrowserHost` | crate-private | capable connection、pending reverse request 和 exact target owner 的唯一 broker | 不执行 Electron/CDP，不选择活动标签，不暴露任意 host method |
| `BrowserHost::request` | private | 产生独立字符串 ID、发布 Server → Client request、等待 30 秒 deadline 并观察取消 | 不复用 client numeric request-ID set；取消发送 `$/cancelRequest` |
| `BrowserHost::handle_response` | crate-private | 校验 owner、terminal response 和 retired request outcome | 非 owner 不能移除 pending；只忽略已放弃请求的晚到响应 |
| `BrowserHost::register_screenshot` | private | 校验 PNG MIME、长度、Base64 和 signature，再创建 connection-owned Resource | 不信任 host 声明的媒体类型，不把 bytes 放进 Tool result |
| `BrowserToolService` | crate-private | 十个内置语义浏览器工具的 schema、materialization 和 Core capability 调用 | URL/node/target 校验不委托给模型或 Desktop |
| `BrowserToolPolicy` | crate-private | 只接受 built-in `BrowserInteraction` + 单个 `UserInterface` capability，并要求一次性批准 | 不复用 local shell、MCP 或 extension policy |
| `InteractionDeadlineWatcher` | crate-private | 在 workspace mutation gate 下重检 durable pending request，持久化 `DeadlineElapsed` cancellation 并失败 Turn | 不选择 UI、不解释 approval policy、不修改 reducer |
| `zeta-skills-extension::SkillRuntime` | external runtime | 组合 roots、缓存 metadata projection、叠加 enablement，并贡献 durable activation/context fragment | App Server 只安装 runtime、提供 config/event adapter 与协议 projection |
| `start_turn::replayed_result` | private | 用 durable command receipt 校验重复 `StartTurn` 输入和审批模式并返回原 Turn 结果 | 重放不重新读取 model config 或 Skill 文件；不同输入返回 `CommandConflict` |
| `SkillConfigSnapshotProvider` adapter | crate-private implementation | 给 external runtime 最新 `SkillsConfig` 与 commit signal | implementation 不把客户端 path 直接升级为 trusted root |
| `SkillRuntimeEventSink` adapter | `UpdateBroker` implementation | 把 runtime generation change 投影为 `skills/changed` | 不参与 catalog reconcile |
| `WorkspaceCustomizations` | crate-private | 发现/刷新 `.zeta/instructions` 与 `.zeta/agents`，发布 future-invocation harness snapshot | 不执行 Agent、不激活 Skill、不解析外部生态格式 |
| `WorkspaceFileChangeSink` | crate-private trait | 在 `fs/changed` 发布前把 projected invalidation 交给内部 runtime owner | callback 不把 watcher event 当作文件事实 |
| `notification<T>` | private | canonical update → JSON-RPC notification | method 来自 `ServerNotificationMethod` |
| `ResourceStore::resource` | private | cleanup + owner check | 所有 read/release 必须经过这里 |
| `ResourceStore::cleanup` | private | lazy TTL eviction | resource 不持久化 |
| `AppServer::file_system` | private | 读取注入的 `WorkspaceFileSystem` 或返回稳定 unavailable error | 不绕过 workspace authority |
| `fs_watcher::project_event` | private | watcher path → root-relative invalidation 或 rescan hint | 不把 event 当作文件内容事实 |
| `GitService` | crate-private | 持有 `InspectRepository` 或 `MutateRepository` 的 `TrustedWorkspace`、映射 repository path、持有 Tokio runtime，并调用 `zeta-git` query/mutation API；在 service 入口再次校验读写 capability | 不从 client path 或 repository config 自行授予 trust |
| `GitRuntime` | crate-private | 串行 operation、为每次 runtime incarnation 创建 `StreamInstanceId`、投影 workspace status、推进实例内 revision 并发布去重 notification | watcher event 不直接成为 Git truth |
| `project_status` in `git_runtime` | private | `zeta-git` snapshot → renderer-safe protocol DTO | 不回传绝对 metadata path 或 internal stderr |
| `file_type` in `fs_operations` | private | foundation file kind → protocol DTO | wire enum 只由 protocol crate 定义 |
| `search_operations::{search_query, search_page}` | private | `WorkspaceSearch*` DTO 与 `zeta-workspace-search` 领域类型之间的显式转换 | 不复制查询校验、rg argv、job state 或 parsing |
| `WorkspaceSearchService` | external crate | 持有 active workspace、frozen rg 和 owner-bound job map | App Server 不把 connection/DTO/UI 语义写入该 crate |
| `CodeIndexRuntime` | crate-private | 串行 rebuild/refresh，投影 lifecycle，并在返回前 materialize | 不拥有 scan/chunk/schema，也不创建网络请求 |
| `CodeIndexRefreshWorker` | private | 单 wake + merged paths/rescan priority 的后台刷新 | 不阻塞 filesystem notification thread，不建立无界 event queue |
| `SymbolIndexRuntime` | crate-private | 在 canonical source generation 后 reconcile，保持 last-ready/stale 状态，并暴露 current index | 不扫描 Workspace、不请求 LSP、不拥有 UI fusion |
| `WorkspaceDocumentOverlay` composition | crate-private | 先同步 CodeIndex canonical dirty snapshot，再投影 SymbolIndex；close/replacement 同步清理 | 不把 overlay 持久化或在每次编辑触发 embedding |
| `CodeIndexSemanticService` | external crate | 同步 exact lexical generation、复用/持久化 vectors、本地 recall/rerank | App Server 不解释模型分数或拥有 vector schema |
| `code_index_operations::project_status` | private | runtime state + last usable snapshot → stable protocol status | 不暴露 SQLite/internal error text |
| `CloudCodeIndexController` | external crate | root-bound grant、publication/deletion lifecycle 与 provider port | App Server 不复制 consent state 或允许 provider 直接读 Workspace |
| `cloud_code_index_operations::project_status` | private | cloud lifecycle + grant → stable protocol DTO | 不回传 credential、绝对路径或 provider error text |
| `TerminalService` | crate-private | 持有 `ExecuteProcess` 的 `TrustedWorkspace`、Tokio runtime、PTY session map 与 1 MiB output ring | 不从 client path 自行授予 process authority |
| `AppServerLanguageRuntime::configured_provider_definitions` | private | 只对 Config 显式启用的已注册 provider 生成 definition，并保留 authoritative host override | 不下载包、不自己启动 child、不复制 restart policy |
| `TerminalEnvironment` | crate-private | 二次过滤 host environment、规范化 Windows key、固定 `TERM`/`COLORTERM`/`TERM_PROGRAM` | 不继承凭据或接受 client mutation |
| `TerminalProfileCatalog` | crate-private | 冻结可信 Shell Profile、program 与 `TerminalEnvironment` | external DTO 不暴露 executable/args/environment |
| `TerminalService::create` | crate-private | 将 default/profile ID 解析到 catalog 并启动 workspace-rooted PTY | client 不能提交 executable/environment |
| `TerminalService::attach` | crate-private | 校验 detached 状态、短租约和一次性 token，再转移 owner 并旋转凭据 | attached/过期/wrong-token/replay 都不能获取 PTY authority |
| `spawn_output_drainers` | private | raw output/exit 并发收束；尾部输出 EOF 后才标记 exited | 不在 exit code 到达时提前丢弃尾部 bytes |
| `read_state` | private | after-sequence cursor → bounded Base64 chunks + gap/exited state | ring eviction 必须显式返回 `output_gap` |
| `ConfigBackedModelService::resolve_config` | private | user snapshot + optional Workspace snapshot merge | 每次 invocation safe point 重新解析 |
| `WorkspaceConfigTracker::read` | private | 内容变化才推进 synthetic workspace revision | 不监听/修改 workspace file |
| `compose_local_tools_with_config` | crate-private | 要求 root-bound `ExecuteProcess` capability、组合 Host/User/Workspace exec-policy snapshot、冻结统一 revision、解析安装候选与沙箱 | containment、trust、policy composition 或 discovery 失败时不降级成 unrestricted |
| `LocalToolConfig::from_resolved` | crate-private | 从 safe-point `ResolvedConfig` 提取 exec-policy 和 Agent grep 执行方式 | 不读取文件或自己决定 layer trust |
| `AgentGrepService` | crate-private | 仅为 Agent `grep` 选择冻结 `rg` 或快速正则索引，并接收工作区 watcher 刷新 | 不参与编辑器 Search、`glob`、`file-search` 或 code retrieval |
| `LocalShellToolService::materialize` | private | parse call、约束 workspace 参数、冻结 rg executable | policy review 前不启动进程 |
| `LocalShellPolicy::decide` | private | 将 frozen `ExecPolicySnapshot` 交给 `ActionPolicyEngine`，返回 exact typed decision | 不复制 rule precedence 或绕过 grant binding |
| `zeta_hooks::DeclarativeHookRuntime` | external crate | immutable Hook snapshot → exact safe-point match → host policy → sandboxed process executor | App Server 只负责 Config reconcile、trust gate 与 Core port binding；实现契约见 [`zeta-hooks`](../hooks/README.md) |
| `zeta_mcp_extension::McpRuntimeOwner` | ext/mcp private | worker thread 持有 Tokio runtime 和 live `McpRuntime` | Core thread 不嵌套 `block_on` |
| `zeta_mcp_extension::McpToolService::review_request` | ext/mcp private | exact binding/arguments/generation → MCP action digest | remote annotation 不授予只读信任 |
| `zeta_mcp_extension::McpApprovalPolicy::decide` | ext/mcp private | 只接受已知 user MCP provenance 并返回 one-time approval | 不自动批准远端副作用 |
| `mcp_operations::{mcp_oauth_start,mcp_oauth_complete,mcp_oauth_refresh,mcp_oauth_revoke}` | private | protocol DTO → exact Config target → `McpOAuthService` → runtime reconcile | 不解析 provider token response 或返回 secret bytes |
| `McpRuntimeIntents::reconcile` | crate-private | 唤醒既有 Tool reconcile worker，使 OAuth/lifecycle mutation 立即生效 | 不修改 durable Config enablement |
| `CompositeToolService` | private | frozen binding/runtime key → local、MCP、dynamic、extension runtime | duplicate name 在 composition 时失败，不按 live name 猜 executor |
| `CompositeActionPolicyService` | private | trusted `ActionSource` → owning policy | 不依靠 trial-and-error policy fallback |
| `ReloadableToolPorts` | crate-private | 原子替换未来 Tool generation，并为 prepared call 固定 service/policy | reconcile failure 保留上一份可用 runtime |
| `DynamicToolService` | private | exact spec digest + arguments → durable `AgentRequest::DynamicTool`，并校验 owner response | 同名新定义不能认领旧 interaction |
| `ExtensionToolReviewer` | private | host-installed executor + declared authority → frozen payload 与 exact Plugin provenance | capability contributor 不直接获得网络或 credential authority |
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
├─ resolve registry serialization scope from params
├─ acquire global / Session / connection-resource FIFO permit
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

`serve_jsonl` 使用 64 项有界 inbound request queue、4 个固定 connection worker 和 256 项有界
outbound queue。不同 scope 可并发执行；相同 scope 的 exclusive request 按进入 scheduler 的顺序
执行，相邻 shared read 可一起执行。`ConnectionDispatchActivity` 让 notification waiter 等全部
正在 dispatch 的 request response 进入 outbound queue 后再 drain causal notifications。owned
embedded client 继续通过 `ConnectionNotifications` 在独立 event pump 中等待同一 queue 的
condition-variable wake。

同一 outbound queue 也承载 `BrowserHost` 的 Server → Client request。Desktop connection 只有在
`initialize.capabilities.browser.version == 1` 且声明至少一种语义操作时才会注册为 browser host。
内置浏览器 Tool port 还受 Workspace trust gate 约束：只有可信工作区与至少一个同时声明
`observe + input` 的 live browser host 存在时才进入当前 Tool generation；最后一个完整 owner
断开或信任撤销都会原子移除该 port。
`BrowserHost` 使用 `browser-host:<connection>:<counter>` 字符串 ID，与 App Server 接受的 client
正整数 ID 隔离；`serve_jsonl` 在普通 request dispatch 之前识别对应 response，并交给 broker
完成 pending request。一次典型工具调用为：

```text
BrowserToolService::execute
└─ BrowserHost::{create_target,observe,perform,close_target}
   ├─ resolve capable connection / exact target owner
   ├─ NotificationQueue::push(JsonRpcRequest)
   ├─ Desktop browser handler
   ├─ serve_jsonl → BrowserHost::handle_response
   └─ typed Core result / connection-owned screenshot Resource
```

创建成功后 `target_id → connection_id` 固定；观察与动作必须回传同一 target ID。connection
关闭会失败所有 pending request 并忘记其 target authority；Desktop Supervisor 同时关闭该连接
创建的原生目标。取消和 30 秒超时都会先本地退休 pending request，再发送 `$/cancelRequest`；
因此晚到的已放弃响应可忽略，已完成请求的重复响应仍是协议错误。

Terminal 因此使用 bounded `terminal/read` pull：`TerminalService` 在独立 runtime 中持续 drain
PTY raw bytes，保留最多 1 MiB，并按 sequence 返回最多 128 个 chunk。`terminal/write` 的单批
UTF-8 输入上限为 64 KiB，rows/cols 上限均为 512；未知 ID、跨 connection 使用和 runtime
capacity 分别映射稳定 Terminal error。`serve_jsonl` 结束时调用 `close_owner`：普通 Terminal
立即终止；`reconnectable` Terminal 解除旧 connection owner 并保留 30 秒，期间只接受 256-bit
bearer token。`terminal/attach` 成功后把 owner 转给新 connection、恢复尺寸并旋转 token；错误、
过期和重放统一返回 `TerminalAttachRejected`。后台 sweeper 会终止过期 PTY。

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
start/compact/interrupt 与 interaction resolve，并以 tagged `SessionRequestResult` 返回对应结果。
旧的独立 Session/Thread/Turn mutation methods 已从 registry 和 dispatcher 删除；所有 mutation
都通过 `session/request`。Thread snapshot 和 gap 使用带 Session scope 的
`session/thread/read` / `session/thread/subscribe`。这些订阅都是 connection-local delivery state；
真实 gap 来自 coordinator/store。

`session/request::StartTurn`：

1. 校验 Thread 属于 supplied Session；
2. 校验 Session 仍为 active；
3. 读取 Session 当前模型与 `next_approval_mode`，冻结 `zeta-models-manager` 的基础 instructions，从 exact registry definitions 构造 `coding-v1` `ToolProfileSnapshot`，再把模型、instructions、profile 和批准模式一起作为 `TurnAccepted` 的 durable snapshot；Thread Goal 通过独立的 `thread/goal/*` mutation 写入同一条 Thread event log；
4. `start_turn` 使用 typed command ID + exact expected sequence；
5. replay 在读取 mutable model、批准模式和 Skill authority 前按 command receipt 校验 input，直接返回原 Turn；后续 Session 设置变化不会改写已接受的 Turn，terminal failure/interruption 不伪装成 success；
6. 新 start 发布 durable update 后调用 `TurnExecutor::start`；执行器在每次模型调用前复核冻结的工具名、顺序和 definition digest，漂移时 fail closed。

`session/request::StartReview` 接受 `UncommittedChanges`、`BaseBranch`、`Commit` 或 `Custom` target。App Server 使用 `zeta-prompts` 渲染精确的用户审查范围，并把共享 review rubric 冻结为该 Turn 的 `TurnInstructions`；它不复用动作授权的 `zeta-auto-review` 提示词。Core 持久化 `TurnKind::Review`，因此 active Goal 不会注入 review，也不会在 review 完成后启动 Goal continuation。review rubric 要求只读检查、只报告变更引入且可验证的问题，并输出固定 JSON findings/verdict。

Trusted Workspace 的 canonical local coding surface 由 direct `LocalToolSuite` 唯一提供
`read_file`、`write_file`、`edit`、`grep` 和 `glob`，由独立 executor 提供 `shell-command` 与
`apply_patch`；legacy operation-enum `file-system` 不暴露给 Agent。`update_plan` 使用 durable caller
identity 提交 `ThreadEvent::PlanUpdated`，相同内容幂等，Desktop 只从 `Turn.plan` 重建计划。

`session/request::CompactContext` 校验 Thread 归属和 active Session，读取当前所选模型，再创建独立的
`ThreadCommand::CompactContext` Turn。可选 retention prompt 会 trim、限制为 8 KiB 并冻结在 command
receipt；相同 command replay 返回原 Turn 且不再次启动 backend。Core 拒绝在 Thread 存在任何非终态
Turn 时开始压缩。所有模型只压缩完整 terminal 前缀，通过 durable usage/checkpoint 后从新 snapshot 继续分批。Desktop 的 server `/compact` 直接走此 mutation，不发送普通聊天文本。

`session/request::SteerTurn` 只接受当前 Running、WaitingForApproval 或 WaitingForUserInput Turn。
Core 先把输入 Item、`TurnSteered` 与 exact command receipt 原子提交；App Server 随后调用当前
`TurnExecutionBackend::steer`，成功后再写 `TurnSteerDelivered`。本地 executor 的 backend ack 不做
第二份排队，因为下一模型安全点直接读取 canonical snapshot。相同 command replay 只有在 delivery fact 已存在时返回原成功结果；
进程若在 receipt 与 delivery fact 之间退出，恢复路径根据 canonical Thread 状态重新唤醒本地 executor，而不重复提交 steer。Desktop 的运行中 Send 走这条 mutation，Stop 仍独立映射 `InterruptTurn`。

`model/list` 由 `ModelCatalog` adapter 投影 shared `zeta-models-manager` 中从
`zeta-model-provider-config::STATIC_MODEL_CATALOG` 派生的 provider seed。列表统一携带 identity、display name、access、context window、automatic
compaction threshold、capabilities、reasoning efforts 和默认 personality，不读取 provider 配置、
ChatGPT 账户或 upstream model catalog 做健康检查。`session/request::SetModel` 只校验 exact identity 属于静态
产品目录再提交 Session command；配置、认证、entitlement、rate limit 和调用错误由实际 Turn backend
处理并持久化为 Turn failure。App Server 不维护第二份模型清单。全局
`preferredModel` 只作为新 Session 和历史无模型 Session 的默认值，不承担当前 Session 的模型切换。
`provider/list` 独立投影 provider registry，因此也覆盖没有静态 model row 的 Ollama、Hugging Face 与
OpenAI-compatible；它不从 `model/list` 反推供应商目录。

`session/request::ResolveInteraction` 使用 exact durable
`RequestId`，且只接受 `UpdateBroker` 选出的、声明该 kind 并订阅 scope 的 connection；full request
只通过 `agent/request` 投递，普通 Thread snapshot 保持 redacted。Dynamic owner 还必须在 initialize
capability 中声明 exact hosted tool name；仅声明 interaction kind 不能领取其他 dynamic tool。
approval/user-input owner 断连或
退订可以确定性重选；已投递的 dynamic tool 不会转交给另一个连接，而是持久化
`InteractionCancelled(OwnerDisconnected)`，恢复原 Tool path并收口为 unknown-outcome failure，
非 owner 返回 `AgentInteractionNotOwner`，过期响应返回 `AgentInteractionExpired`。当 response 是
Tool Call 对应的 approval、user input 或 dynamic tool result，且 Core 确实产生 `Resolved`
disposition，App Server 才考虑恢复 Tool path。Core result 的 `live_execution_woken` 表明同进程
`ToolInteractionService` waiter 已取得响应；此时 App Server 不再调用 backend `resume`。只有恢复后
没有 live waiter 的 durable interaction 才启动 executor recovery。这个判断依赖 exact
Thread/Turn/request 和 pending item binding，不能简化为“所有 interaction 都 restart”。
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

stdio/JSONL host 使用独立 notification waiter、固定 request worker pool 和单一 writer thread。
request response 与主动 notification 统一进入 256 项有界 outbound channel；connection-local
dispatch activity barrier 保证 request 内产生的 causal notification 排在该 response 之后，而没有
active request 时 notification 不必等待下一行输入即可写出。每个 connection 拥有独立 inbound、
outbound 和 worker 状态，因此一个慢 writer 的背压不会占用其他 connection 的队列。

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

Workspace Trust management (workspaceTrustHost)
├─ workspace/trust/read → durable decision + effective Restricted/Trusted state for one root
├─ workspace/trust/list → trusted folders only + display-only root metadata
├─ workspace/trust/set → canonicalize root, persist the decision, and reconcile the active root when it matches
└─ workspace/trust/forget → remove one decision by opaque WorkspaceTrustId

Session additional-directory authority
├─ workspace/additionalDirectories/list → current Session roots + complete permissions + revision
├─ workspace/additionalDirectories/add → canonicalize + explicit session lease + caller-declared initial permissions
├─ workspace/additionalDirectories/permissions/set → revision-checked complete permission replacement + old lease revocation
└─ workspace/additionalDirectories/remove → revoke exact root lease + refresh capability-specific Session roots

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
   │  └─ Global content → HarnessContextProvider
   ├─ WorkspaceEnvironment → zeta-agent-environment snapshot
   ├─ SessionWorkspaceAccess → per-Session WorkspaceAccessAuthority
   ├─ AgentDefinitionCatalog(.zeta/agents)
   └─ FileSystemWatcher invalidation → bounded catalog refresh

local tools + enabled user MCP declarations
├─ materialize absolute stdio executable / HTTP endpoint / SecretStore credential
├─ collect host read-only extension executors and accepted dynamic specs
├─ combine duplicate-free model Tool names
├─ install ReloadableToolPorts
└─ ToolConfigWatcher(ConfigChange)

External package lifecycle + legacy Plugin compatibility + Hook declaration
├─ MarketplaceManager(<profile>/marketplace-manager)
│  └─ one installation → Skill/MCP/Connector/Theme/Language/Executable capability consumers
├─ legacy PluginActivationAuthority(<profile>/plugins)
│  └─ live generation → Connector/MCP reconcile + invocation drain
├─ legacy plugin list/grant/enable/disable/revoke/uninstall RPC
├─ generic Marketplace search/install/update/uninstall/capability RPC
├─ optional Zeta Editor Extension sidecar → independent admission + Manager lease → Host deployment
└─ App Server trusted Workspace gate → zeta-hooks runtime → Host Policy → sandboxed process executor

Language-server preference
└─ config/read + languageServer/configure|remove
   ├─ App Server 持久化 mode/path，并负责 resolver resolution 与 LSP manager
   └─ language/hover|completions|locations|hierarchy|workspaceSymbols
      + language/prepareRename|rename|codeActions|resolveCodeAction
      + language/synchronize|close and revision-bound language/diagnostics notifications
```

每次 `ModelService::invoke` 重新读取 user config，与 optional workspace document 合并，再由
`ModelSnapshotResolver` 生成 immutable invoker。因此 config change 影响下一次 invocation，不会
改变已经运行的 invocation。`ProviderModelService` 把 Core token 传入
`ModelInvoker::invoke_with_cancellation`；同一 config snapshot 还选择图片输入策略，attachment adapter
只对 preflight/invocation 的临时 data URL 按 detail、dimension 和 patch ceiling 降采样，不改写
durable 原图。取消被保留为 `CoreError::Cancelled`，不会降级为普通模型失败。`model_provider_error::map_model_provider_error` 把上下文溢出、认证失败、无效请求和无效响应映射为不含原始供应商错误体的类型化 `CoreError`，原始详情只写入调试日志。Core 对首次上下文溢出持久化压缩 terminal 旧历史并以新快照重试一次，对无效响应只重试一次，401/403 不重试，最终失败通过持久化 `StableTurnError` 投影到对话。生产环境的供应商操作会立即停止本地等待、禁止重试，并丢弃同步 HTTP 尝试的迟到响应。

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

skill/resource/open { SkillId, SKILL.md digest, package-relative path }
├─ SkillRuntime::read_resource → exact digest/root/file-identity validation
├─ conservative MIME projection; active HTML/SVG remains application/octet-stream
└─ ResourceStore::create(connection owner, bytes, TTL)

session/request StartTurn { SkillRef }
├─ start_turn::replayed_result
│  └─ existing command → validate exact input and return durable result without mutable reads
└─ Core extension lifecycle
   ├─ SkillsExtension activation contributor → exact frozen activation
   ├─ ThreadEvent::TurnAccepted persists FrozenSkillActivation
   ├─ ThreadEvent::TurnAccepted persists ToolProfileSnapshot
   └─ TurnExecutor safe point → TurnInputContributor → PromptFragment → ContextPlan
```

Watcher 订阅当前 source roots；Config authority 通过 commit channel（包括 TOML 外部编辑与
SQLite cross-connection change）触发配置 reconcile。change、overflow 或 backend rescan 提示都只触发
完整 reconcile；entry、diagnostic 或 enablement projection 没变时 generation 不变，也不发
notification。Watcher 启动失败时 local App Server 仍可用，显式
`skills/list { reload: "refresh" }` 是恢复路径。

`zeta-skills-extension` 当前组合 built-in、user 与 active Workspace `.zeta/skills` source。显式正文
activation、durable provenance、通用 prompt fragment injection 和 invocation safe-point reload
已实现；其 `ReadOnlyToolContributor` 现在把统一的 `skills-read` 作为普通 `ToolExecutor` 投影进
共享 registry，由模型按 metadata 中的 exact source/name 按需加载正文，再用 pinned Skill digest
读取单个有界 package-relative UTF-8 文件。Binary resource 通过 `skill/resource/open` 进入现有
connection-owned Resource store，并使用保守的 MIME/signature 校验；active HTML/SVG 不会被声明为
可直接 preview。禁用只影响 future Turn eligibility，不能改变已经冻结的 Turn。Plugin source 和
大目录候选检索的更高层决策、Renderer preview 与 script execution adapter 尚未实现。TUI 与 Desktop 已从
metadata-only catalog 生成直接 `/name` Skill command，
而 `/skills` 只承担管理；正文变化或 source 消失会使恢复/后续 safe point 失败即
关闭，不会用新 bytes 替换 frozen digest。

MCP runtime 在 `open_local_app_server` 构造初始 generation；后台 `ToolConfigWatcher` 同时订阅 Config
和 Connector authority，完整构造下一 generation 后原子切换 future model safe points，旧 generation
由 model snapshot 和已绑定调用持有到排空。`enabled` 是建立连接/启动 server 的显式用户 intent；它不
批准任何 tool call。stdio command 必须是存在的 absolute executable，独立 Config HTTP 当前只接受
unauthenticated endpoint 或由共享 SecretStore materialize 的 credential reference；注入的 Connector
runtime 可在 ready account 下 materialize credential-bearing transport。exact MCP OAuth provider 由
host 注入，`mcp/oauth/complete|refresh` 会设置 connect intent 并立即 reconcile，revoke 在 provider
调用前先设置 disconnect intent，并等待 active Tool generation 移除该 server。Workspace MCP intent
仍保持 pending trust，不会接入。
`tools/list_changed` 通过 `McpCatalogUpdates` 触发同一 host reconcile；重建失败保留旧 generation
并记录诊断，不发布半成品 catalog。
启动与重建采用 `RequireAll`，任一 enabled server 无法 initialize 时保留旧 generation 并记录诊断，
不会静默发布不完整 catalog。

当 MCP server 在工具调用期间发起 form elicitation，RMCP host 通过 task-local exact call binding
进入 Core `ToolInteractionService`。Core 先持久化 `AgentRequest::UserInput`，owner response 再唤醒同一
工具执行；URL elicitation、数组/多选与完整 JSON Schema format 当前失败关闭或 decline。进程重启后
remote request 不可恢复，已 started 的调用保持 unknown outcome，不能向新 MCP connection 重放。

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

`ApprovalModeActionPolicyService` 先调用 authoritative base policy，并且只处理其 `AskUser` 结果：

| Turn 模式 | `AskUser` 后续行为 | 不变量 |
| --- | --- | --- |
| `AskPermissions` | 保留 durable approval interaction | 用户仍只能 approve once / decline |
| `AutoReview` | 用冻结的 review runtime 创建 `LlmActionClassifier`，由 `ActionPolicyEngine` 应用风险矩阵 | reviewer 缺失或失败时回退 `AskUser` |
| `BypassPermissions` | 签发绑定 action digest、完整 capabilities 与 policy revision 的 permission-bypass authority | 只跳过交互；不能改变 `Block`、revision error 或其他 base decision |

这里负责选择/隔离 review runtime 和按 Turn 模式组合 policy；classifier schema validation 与
Auto Review authorization decision 分别属于 `zeta-auto-review` 和 `zeta-action-policy`。review runtime
当前在 local App Server 启动时从 frozen config 解析；配置变化需要重启 App Server 才能换用新模型。

## 资源存储

Resource 是 in-memory、connection-owned、TTL-bounded：

- 单个 resource 最大 `MAX_RESOURCE_BYTES` = 16 MiB；
- 每个 connection 同时最多 128 个 resource、合计最多 64 MiB；
- read chunk 最大 `MAX_READ_CHUNK_BYTES` = 262,144 bytes；
- `create_resource` 当前固定 TTL 为 300 秒；
- ID 是 process-local monotonic hex string；
- metadata 包含 MIME、byte length 和 SHA-256；
- read 返回 standard Base64、decoded length、offset 与 EOF；
- cleanup 在 create/read/metadata/release path lazy 执行。

Resource 不跨重启恢复，也不能被另一 connection 读取或 release。Connection drop 当前不会立即遍历
删除资源，只依赖 TTL lazy cleanup。

浏览器截图是这一存储的当前 producer 之一。Host response 中的 PNG 最多 16 MiB，必须同时通过
decoded length、标准 Base64、`image/png` 和 PNG signature 校验；Tool result 只公开 resource ID、
MIME、size 与 SHA-256，不复制图片 bytes。

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
| stale additional-directory permission page | `WorkspaceAccessRevisionConflict` |
| missing exact MCP server/runtime | `McpServerNotFound` / `McpRuntimeUnavailable` |
| missing OAuth service/provider | `McpOAuthUnavailable` |
| replayed, changed-target or state-mismatched OAuth callback | `McpOAuthInvalidCallback` |
| expired OAuth flow | `McpOAuthExpired` |
| provider exchange/refresh/revoke or credential store failure | `McpOAuthOperationFailed` |
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
| invalid reconnect lease or token replay | `TerminalAttachRejected` |
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
- Browser target 由“当前活动标签”解析，或非 owner response 能移除 pending：宿主边界漂移；
- App Server 把任意 CDP method 暴露给 Core/Tool：语义 browser capability 退化为远程调试后门；
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
手动压缩的独立 Turn/replay/backend 单次启动、
多连接更新、重连后的持久化缺口、有界通知 overflow、连接拥有的资源、配置命令、owner-directed
交互/批准解决、断连重选与 deadline、
先响应后通知、模型配置安全点、
Workspace override、review-only request、只读 `rg` definition/materialization/policy/execution，
MCP worker bridge、exact provenance/approval policy、local/MCP 路由与 collision rejection、
connect/disconnect/status、OAuth PKCE/one-shot callback/refresh/revoke/reconcile、form elicitation 的
durable live wake 与 concurrent call isolation、
Config commit/cross-connection notification、future Tool generation replacement 与 prepared-call
generation retention，以及
可信 Terminal Profile、真实 PTY create/attach/write/read/exit、Terminal owner/lease/token replay/error/ring limits，
Skill 内置/用户/工作区来源组合、启用状态叠加、监听器刷新、精确激活、摘要变化失败即关闭、
删除源文件后的 command receipt 重放与 `skills/changed`；工作区定制覆盖全局指令注入、Agent 目录刷新和
`AGENTS.md` 非原生来源隔离。
Git 覆盖 workspace projection、runtime stream identity、revision 去重、`git/statusChanged`、
text diff、local branch list/switch、path mutation 与 commit。Filesystem 覆盖有界原子写入、权限保留、root containment、
相对路径 `fs/changed` 与 watcher overflow rescan。
Syntax 覆盖 revision mismatch、Unicode UTF-16 projection、按需 selection ancestor scopes 与 invalid
surrogate boundary；CodeIndex/SymbolIndex 覆盖 persistent reopen、watcher reconcile、dirty overlay、
save handoff、stale materialization 和 symbol-aware retrieval。
Browser host 覆盖反向 request/response、非 owner 拒绝、target identity、断连 pending failure、
截图 Resource ownership，以及 Desktop restart-safe handler registration 和原生目标回收。
Request scheduler 覆盖同 key exclusive FIFO、相邻 shared read、不同 Session 并发、跨 connection
resource isolation、queued disconnect cancellation，以及慢 connection writer 不阻塞另一 connection。

local tool 的参数白名单、discovery、取消与输出限制由
[`zeta-shell-command`](../shell-command/README.md) 和
[`zeta-tool-executor`](../tool-executor/README.md) 维护；
本 README 只拥有 App Server 组合与 Core port binding。

当前 JSONL 服务已具备有界 per-connection worker/queue、跨 connection 的 scope serialization、
可唤醒通知来源，以及显式订阅、资源和终端清理；remote broker 仍负责 Unix socket accept/thread
生命周期，当前没有 WebSocket transport、持久化临时资源或 weighted-fair CPU/token quota
scheduler。MCP desired config 的热更新、SecretStore credential materialization、provider-injected
OAuth lifecycle、form elicitation 和未来 Tool catalog replacement 已实现；当前仍没有 stdio 进程
沙箱、自动 OAuth discovery/内建 provider、完整 runtime health/log API、progress 产品投影、URL/复杂
form elicitation 或 MCP resources/prompts。Marketplace 安装由 Manager 唯一拥有；legacy Plugin
grant/enable/disable/revoke/uninstall 仍有 typed compatibility RPC，不能作为远端安装旁路；Renderer 管理
UI 不属于本 crate。MCP 与 dynamic image result 已进入 canonical
`ContentPart`，Core 会把结构化内容写入 durable `ToolResult`，并在最终模型请求处按 model
capability 统一处理 image detail；旧 transcript 的纯 text shape 仍可读取。Reloadable Tool service
会在调用落盘前冻结 generation、definition digest、source chain 和 process incarnation；重启后的
历史调用当前选择失败关闭，而不是按同名 live tool 重放。演进这些能力时应保留 protocol registry
唯一性、Core/store authority、snapshot + durable gap 和 per-invocation config safe point。

## Static extension 资源

跨层行为和信任边界由 [`docs/editor-extensions.md`](../../docs/editor-extensions.md) 维护，可复用包目录
由 [`zeta-extensions`](../extensions/README.md) 拥有。本 crate 按“内置目录 → Marketplace/legacy
Plugin dynamic exact packages → 用户目录”的顺序组合来源，把目录值适配为协议 DTO，并把打开的 bytes 放入 connection-owned
`ResourceStore`。Renderer 调用方不能提交绝对主机路径。

`extensions/list` 返回一个不可变目录代次。`extensions/resource/open` 绑定代次、扩展 ID 和包内相对
路径；旧代请求明确失败，不会重新打开可变 package 文件。App Server 把目录诊断和错误映射到协议
类别并拥有临时资源生命周期，但不解析 language、grammar、snippet、theme 或 debugger 贡献。未来
App consumer 应直接依赖 `zeta-extensions`，不能导入 App Server 或 Desktop transport code。
