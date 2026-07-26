# `zeta-mcp` 架构与演进方案

> 计划物理位置：`zeta-rs/mcp/`
> Rust crate：`zeta_mcp`
> 当前状态：Proposed，尚未创建 crate
> Core architecture：[`core.md`](core.md)
> Agent runtime：[`zeta-agent-runtime-architecture.md`](zeta-agent-runtime-architecture.md)
> Config authority 与 runtime snapshot 接入：[`config.md`](config.md)
> Plugin 分发边界：[`plugins.md`](plugins.md)
> Skill 指令边界：[`skills.md`](skills.md)

> 官方规范核对日期：2026-07-25。第一版以 MCP `2025-11-25` protocol revision 为实现目标。
> MCP 仍会演进；wire schema、授权流程和 experimental capability 必须以实现时的
> [官方规范](https://modelcontextprotocol.io/specification/2025-11-25/architecture/index) 与
> schema fixture 为准。

## 1. 结论

`zeta-mcp` 是 Zeta 的 MCP client runtime 和 provider-neutral capability adapter。它负责连接
本地或远程 MCP server，完成 version/capability negotiation，将 MCP tools、resources 和 prompts
投影为 Zeta 可消费的能力，并在连接、取消、超时和断线之间保持准确语义。

它不是：

- Plugin 的安装器、包管理器或信任根；
- Skill 的发现器、选择器或指令加载器；
- 绕过 Zeta tool approval、sandbox 和 durable Tool Call/Result 的执行捷径；
- MCP server 输出、tool annotation 或 prompt 内容的信任背书；
- Zeta Session、Thread、Turn 或 App Server connection 的替代品。

三者边界固定为：

```text
Plugin = 如何分发、安装、启用一组扩展贡献
Skill  = Agent 在什么任务下应遵循哪些渐进加载的指令
MCP    = 如何与一个外部 capability server 建立有状态协议会话
```

一个 Plugin 可以声明 MCP server，也可以携带 Skill；独立配置的 MCP server 和独立安装的 Skill
同样是一等来源。安装 Plugin 不等于允许其 MCP server 启动，启用 MCP server 也不等于自动批准
其每次 tool call。

## 2. 当前仓库审计

当前仓库只有 MCP 的目标边界，没有 MCP client 实现：

- [`core.md`](core.md) 与
  [`zeta-agent-runtime-architecture.md`](zeta-agent-runtime-architecture.md) 已将 MCP adapter
  放在 Core tool port 之外，并要求 HTTP/MCP cancellation 贯穿工具执行；
- `zeta-protocol` 已有 provider-independent `ToolDefinition`、`ToolCall`、`ToolResult`、
  `ToolName` 和 durable Thread tool item；
- `zeta-core` 已有 approval policy 基础，目标 `zeta-tool-executor` /
  `zeta-sandboxing` 已有本地执行边界；当前 process executor 的物理 crate 仍名为
  `zeta-exec`，后续按 [`exec.md`](exec.md) 迁移；
- App Server 尚无 MCP config、catalog、authorization 或 health method；
- 当前没有 JSON-RPC MCP codec、stdio/Streamable HTTP transport、MCP process supervisor、
  capability negotiation 或 tool/resource/prompt adapter。

因此本文描述目标架构。任何 phase 在 crate、测试和 App Server vertical slice 完成前都必须标记
为 Proposed，不能只因存在配置字段或 UI 入口就声称 MCP 可用。

## 3. 标准基线

MCP 是 host-client-server 架构：host 为每个 server 创建隔离 client，一个 client 与一个 server
维持一个 stateful session。初始化阶段先协商 protocol version 和 capabilities，之后才能进入正常
operation。[官方架构](https://modelcontextprotocol.io/specification/2025-11-25/architecture/index)
同时要求 host 控制权限、用户授权、模型集成和跨 server 隔离。

第一版支持矩阵：

| MCP surface | 第一版策略 | 原因 |
| --- | --- | --- |
| Base JSON-RPC lifecycle | 支持 | 所有其他 capability 的前提 |
| stdio | 首发支持 | 本地开发和 Plugin 集成的最小闭环 |
| Streamable HTTP | 第二个 vertical slice | 需要 OAuth、SSE resume 和连接安全完整落地 |
| Tools | 首发支持 | 可适配目标 Core Turn tool loop |
| Resources | 首发只做 list/read | 由应用或用户选择后进入 context，不自动注入 |
| Prompts | 首发只做 list/get | 作为用户显式选择的模板，不当作 Skill |
| Roots | stdio 首发支持 | 只暴露已授权 workspace root，不能替代 OS sandbox |
| Sampling | 默认不声明 | 会让 server 发起模型调用，需要独立预算、隐私和审批设计 |
| Elicitation | 默认不声明 | 依赖完整 Server → Client interaction delivery/recovery |
| Tasks | 暂不支持 | `2025-11-25` 中仍为 experimental，不能等同 Zeta Turn/Task |

MCP 标准 transport 是
[stdio 与 Streamable HTTP](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports)。
HTTP+SSE 是旧 revision 的兼容面，不进入 Zeta 第一版。

## 4. 职责与非职责

### 4.1 `zeta-mcp` 拥有

- MCP wire request/response/notification codec 和 protocol revision negotiation；
- 每个 configured server 的隔离 client session；
- stdio 与 Streamable HTTP transport adapter；
- initialize → operation → shutdown lifecycle；
- request ID correlation、per-request deadline、cancel propagation 和 late response discard；
- tools/resources/prompts 的分页 discovery、list-changed invalidation 和 immutable catalog；
- MCP schema、content 和 error 到 Zeta runtime value 的校验与转换；
- remote tool identity 到 model-facing `ToolName` 的无歧义绑定；
- server process/connection health、bounded queue 和不含秘密的 diagnostics；
- roots、sampling、elicitation 等 client feature 的 capability gate。

### 4.2 `zeta-mcp` 不拥有

- Plugin package 下载、签名、解压、版本解析、enable/disable authority；
- API token、OAuth refresh token、client secret 的持久化；
- workspace、filesystem 或 network 的最终授权策略；
- model selection、token budget、Core ContextAssembler 或 Agent loop；
- tool approval、side-effect classification 的最终判定；
- Thread event append、reducer、unknown-outcome recovery decision；
- MCP server 自己的业务正确性、rate limit 或下游事务；
- UI 中 MCP picker、OAuth browser window、confirmation dialog 的布局；
- 把 server instructions、prompt、resource 或 tool result 提升为 system instruction。

MCP auth runtime 拥有 credential/OAuth lifecycle，并把 opaque token persistence 委托给
`zeta-secrets`；`zeta-sandboxing` / host capability 实施资源边界，Agent runtime 决定何时调用
工具，Core 负责 durable commit，App Server 是 composition root。

## 5. 目标依赖与运行时结构

```text
                         zeta-protocol
                           ▲       ▲
            shared values  │       │ canonical tools/items
                           │       │
                      zeta-mcp   zeta-core ─────► Session/Thread stores
                           ▲       ▲
                           │       │ Core ToolService adapter
                           └───┬───┘
                         App Server
                       composition root
                       ▲       ▲       ▲
                       │       │       │
             zeta-plugins  credentials  process/HTTP host adapters
```

规则：

- `zeta-mcp` 可以依赖 `zeta-protocol` 和通用 async/JSON/HTTP 库；
- `zeta-mcp` 不依赖 `zeta-core`、stores、App Server、Desktop、CLI 或 Plugin manager；
- `zeta-core` 定义 consumer-owned `ToolService` port；App Server adapter 将 MCP tool
  handle/catalog 适配到该 port，Core 不读取 MCP wire DTO；
- Plugin manager 只产出经过验证的 server declaration，不持有 live MCP session；
- App Server 注入 credential materializer、process launcher、HTTP transport、workspace roots 和
  policy，不能让 `zeta-mcp` 自己扫描全局环境；
- MCP wire DTO 留在 `zeta-mcp` private module，不进入 `zeta-protocol`。

每个 live server 运行时：

```text
McpServerDefinition
    → policy/grant resolution
    → transport connect
    → initialize + capability negotiation
    → immutable capability/catalog snapshot
    → bounded request router
    → graceful shutdown / reconnect
```

## 6. Identity、定义与快照

### 6.1 Server identity

配置必须为每个 server 分配稳定 `McpServerId`。它标识 Zeta 配置中的逻辑 server，不使用 endpoint、
child PID、MCP session ID 或 Plugin 显示名称充当 identity。

```rust
pub struct McpServerId(String);

pub struct McpServerDefinition {
    pub id: McpServerId,
    pub display_name: String,
    pub transport: McpTransportDefinition,
    pub credential: McpCredentialBinding,
    pub policy: McpServerPolicy,
}

pub enum McpTransportDefinition {
    Stdio(StdioServerDefinition),
    StreamableHttp(StreamableHttpServerDefinition),
}

pub enum McpCredentialBinding {
    Unauthenticated,
    Reference(CredentialRef),
}
```

Plugin 贡献的 logical ID 解析为带 Plugin namespace 的 `McpServerId`；用户配置与 workspace 配置
使用各自 source namespace。不同来源不能通过同名静默覆盖。

### 6.2 Remote primitive identity

MCP tool name 只保证在单个 server 内唯一；resource URI 和 prompt name 同样不能脱离 server
解释。Zeta identity 必须保留二元组：

```text
McpToolRef     = (McpServerId, exact remote tool name)
McpResourceRef = (McpServerId, exact resource URI)
McpPromptRef   = (McpServerId, exact prompt name)
```

不得 lowercase、重写或丢弃 exact remote identity。model-facing alias 只是一次 catalog snapshot
内的路由名，不是远端 identity。

### 6.3 Tool alias 与绑定

当前 `zeta-protocol::ToolName` 允许 1–128 个 ASCII 字母、数字、`_`、`-`，而 MCP tool name
还可能包含 `.`。不能把不兼容字符简单替换成 `_` 后假定没有冲突。

目标使用显式绑定：

```rust
pub struct McpToolBinding {
    pub exposed_name: ToolName,
    pub remote: McpToolRef,
    pub schema_digest: ContentDigest,
    pub catalog_generation: u64,
}
```

alias 由可读 server/tool slug 加短 digest 生成，生成后做全 catalog collision check。Agent 发起
tool call 时必须从冻结的 binding snapshot 解析，不得根据字符串重新猜 server。list-changed 后
新的 binding 只在下一个 model safe point 生效；旧调用不能被静默路由到另一个工具。

### 6.4 Immutable snapshot

```rust
pub struct McpCatalogSnapshot {
    pub server: McpServerId,
    pub connection_generation: u64,
    pub catalog_generation: u64,
    pub negotiated: NegotiatedMcpCapabilities,
    pub tools: Vec<McpToolDescriptor>,
    pub resources: Vec<McpResourceDescriptor>,
    pub prompts: Vec<McpPromptDescriptor>,
    pub freshness: McpCatalogFreshness,
}
```

只有 consumer-visible catalog 变化才递增 `catalog_generation`。重连一定产生新的
`connection_generation`，即使 catalog 内容相同；这样 late response、旧 subscription 和旧
request ID 都不能污染新连接。

## 7. Connection lifecycle

状态不能压成 `connected: bool`：

```text
Disabled
→ Starting
→ Initializing
→ Ready
→ Degraded / Reconnecting
→ Stopping
→ Stopped
```

另有 terminal `Blocked`/`Misconfigured` 诊断状态。状态与 health、enablement 分开，避免
“已启用但认证失败”被错误显示为未安装。

[MCP lifecycle](https://modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle) 要求：

1. client 首先发送 `initialize`；
2. 双方协商 protocol revision 与 capabilities；
3. client 发送 `notifications/initialized` 后进入 operation；
4. 不支持 server 返回 revision 时断开，而不是尝试按最新 DTO 继续；
5. shutdown 停止接受新调用，取消或等待有界的 in-flight request，再关闭 transport。

本地 policy：

- initialize、list 和 call 使用分别配置的 deadline；
- request ID 只在一个 connection generation 内相关；
- cancellation 发送 MCP cancel notification，并停止本地等待；
- cancel 不证明远端副作用没有发生；
- server crash/EOF 使所有未完成请求进入 transport-lost 分类；
- 自动重连使用 backoff + jitter，并受全局启动并发限制；
- reconnect 成功后重新 initialize 和 discovery，不复用旧 server capability。

## 8. Transport

### 8.1 stdio

stdio supervisor 负责：

- 通过已批准的、解析后的 executable path 启动 child；
- 使用显式 argv，不通过 shell 拼接；
- 设置最小 env allowlist 和显式 working directory；
- 将 child 放入适用的 process/sandbox boundary；
- stdout 只解码 MCP JSON-RPC，stderr 作为有界 diagnostic stream；
- 限制单帧大小、stdout/stderr buffer、并发请求和 shutdown deadline；
- 终止时处理完整 child process tree。

Plugin manifest 中的 command/args/env 只是请求；真正启动前必须经过 install trust、runtime grant
和 workspace policy。`PATH` 查找结果必须在 activation snapshot 中冻结，不能每次调用重新解析。

### 8.2 Streamable HTTP

HTTP adapter 必须实现：

- 单一 MCP endpoint 的 POST/GET 与 JSON/SSE content negotiation；
- `MCP-Protocol-Version`、可选 `MCP-Session-Id` 和 session termination；
- SSE event ID / `Last-Event-ID` resume，但不把 event ID 当 Zeta durable sequence；
- Origin、redirect、TLS、proxy 和 endpoint allowlist policy；
- 401 challenge、OAuth metadata discovery 和 credential refresh；
- response/body/SSE frame 大小及 idle/read deadline；
- 同一 MCP message 不跨多个 SSE stream 重复交付。

断开连接不等于 MCP request cancellation；client 必须显式 cancel。HTTP session ID 是远端 transport
state，不进入 Zeta Session/Thread。

第一版不实现 custom transport。新增 transport 必须先定义 threat model、framing、cancellation、
authentication 和 shutdown contract，不能只提供 `send(Value)`。

## 9. Server primitives

### 9.1 Tools

MCP tool descriptor 转换为 Zeta tool definition 前必须：

- 完整处理分页，并施加 tool count/page/bytes 上限；
- 校验 name、input schema、可选 output schema 和 schema complexity；
- 保留 exact remote descriptor 与 digest；
- 将 annotation 标为 `UntrustedServerClaim`；
- 用本地 policy 计算 approval/risk，而不是采信 server 的 read-only/destructive hint；
- 对 model 不支持的 content type 给出明确降级或拒绝。

MCP `2025-11-25` 的 embedded schema 默认使用 JSON Schema 2020-12；tool output 若声明
`outputSchema`，Zeta 应校验 `structuredContent`。规范明确区分
[protocol error 与 tool execution error](https://modelcontextprotocol.io/specification/2025-11-25/server/tools)：

- unknown tool、malformed request、server protocol failure → MCP protocol error；
- API failure、业务校验失败等 `isError: true` → canonical `ToolResult { is_error: true }`。

### 9.2 Resources

Resource 是 application-controlled context，不是可自动执行的 tool。第一版：

- 支持 `resources/list`、template list 和 `resources/read`；
- 对每个 read 施加 scheme、MIME、byte/token 和 content-part 数量上限；
- resource URI 始终与 `McpServerId` 绑定；
- resource/list-changed 只使 catalog stale；
- subscription 只为用户已选或 Core ContextAssembler 正在引用的 resource 建立；
- update notification 触发重新读取，不直接信任 notification payload；
- binary 内容通过 Resource/content store 传递，不内嵌到普通日志。

Server 返回的 `file://` URI 不授予本地文件权限。只有 MCP `resources/read` 的返回内容可作为该
server resource；Zeta 不绕过 server 去读取同名本地路径。

### 9.3 Prompts

MCP Prompt 是 user-controlled、带参数的远端模板：

- 必须由用户显式选择或明确的产品 command 触发；
- `prompts/get` 返回内容按外部、不可信 instruction 处理；
- 不加入全局 developer/system instruction；
- 不持久注册成 Skill，也不因名称相同覆盖本地 Skill；
- prompt 中的 embedded resource 继续遵守 resource size、MIME 和 provenance policy。

如果用户要将稳定 MCP Prompt 保存为 Skill，应通过显式导入/复制流程生成新的本地 artifact，
记录来源与 digest；运行时不能把二者自动等同。

## 10. Client features

### 10.1 Roots

Roots 只告诉 server 当前 workspace 边界。它不是 sandbox，也不证明 server process 无法读取
root 之外的文件。

- root 来自 host 授权后的 workspace snapshot；
- 只发送 canonicalized `file://` URI；
- path traversal、symlink policy 和 root 可达性由 host 校验；
- workspace 切换后发送 list-changed，并使旧 root grant 失效；
- local MCP process 仍必须使用 OS/process sandbox；
- remote MCP server 只得到 root metadata，不自动得到文件内容。

### 10.2 Sampling

Sampling 允许 MCP server 请求 host 调用模型，涉及费用、凭据、conversation disclosure 和递归
tool loop。第一版不声明 sampling capability。

后续启用必须通过独立 `McpSamplingPolicy`：

- server 不能指定或读取 provider credential；
- Core ContextAssembler 只提供请求所需最小 context；
- 每 server/session 有 token、费用、并发和递归深度预算；
- user approval 与 model choice 由 Zeta host 决定；
- sampling 不能调用发起它的同一 tool 形成无界递归；
- request/result 进入可审计 trace，但不伪装成用户 Turn。

### 10.3 Elicitation

Elicitation 依赖 Zeta 的 typed Agent request/response delivery。完整 owner selection、deadline、
disconnect 和 recovery vertical slice 完成前不声明 capability。

将来 adapter 必须把 MCP request ID 与 Zeta `RequestId` 分开关联。Zeta interaction 可以 durable，
但 remote MCP connection/request 本身通常不能跨重启恢复；恢复后必须重新建立 server state 或
明确取消，不能向新 connection 发送旧 response。

### 10.4 Experimental tasks

MCP task ID 只标识 MCP 协议中的异步 request 状态，不是 Zeta 产品 Session、Thread、Turn 或
Codex 意义上的“任务”。在官方 capability 稳定且 Zeta 定义好 polling、expiry、authorization
binding、cancel 和 recovery 前不启用。

## 11. Tool loop、取消与 unknown outcome

```text
MCP catalog snapshot
    → Agent model receives exposed ToolDefinition
    → model emits Tool Call
    → resolve frozen McpToolBinding
    → validate arguments
    → evaluate Zeta approval + policy
    → durable commit Tool Call
    → MCP tools/call outside Thread state owner
    → validate result
    → durable commit Tool Result / UnknownOutcome
```

MCP adapter 不直接写 Thread。取消和 retry 遵循 Agent Runtime 的统一工具语义：

- 尚未发送：可安全取消；
- 已发送但 server 明确返回 execution error：记录失败结果；
- 连接丢失且工具可能有副作用：`UnknownOutcome`；
- read-only 也只有本地 policy 明确认定且 retry strategy 允许时才自动重试；
- server annotation 不能单独把调用升级为 `SafeRead`；
- reconnect 后不得自动重放未完成 write；
- late result 只能关联原 connection generation 和 ToolCallId，不能完成另一个调用。

## 12. 安全与授权

### 12.1 Trust boundary

以下全部默认不可信：

- server instructions；
- tool name、description、annotations 和 schema；
- prompt 和 resource 内容；
- tool output、resource link 与 embedded resource；
- Plugin 声明的 command、endpoint、env 和 permission request。

MCP 内容可能包含 prompt injection。Zeta 必须带 provenance 传入 Core ContextAssembler，用边界标记其
来源，且不能允许其覆盖 system/developer policy、approval 或 sandbox。

### 12.2 HTTP authorization

远程授权遵循
[MCP authorization](https://modelcontextprotocol.io/specification/2025-11-25/basic/authorization)
与 [security best practices](https://modelcontextprotocol.io/docs/tutorials/security/security_best_practices)：

- access/refresh token 的领域 lifecycle 属于 MCP auth，opaque bytes 只存于 `zeta-secrets`；
- config、Plugin manifest、snapshot、日志和 Thread event 只保存 `CredentialRef`；
- OAuth 使用 PKCE、state 和 exact redirect URI；
- token 必须绑定目标 resource/audience；
- 禁止把发给 MCP server 的 token 原样透传给下游服务；
- redirect 和 authorization server metadata 必须经过 allowlist/consent；
- localhost callback 使用随机 state、短期 listener 和一次性 completion；
- logout/revoke 后立即停止依赖该 credential 的 session。

### 12.3 Data egress

每次 tool call 前，approval UI 应展示目标 server、tool、materialized arguments 和潜在数据范围。
Secret field 按 schema/policy 脱敏展示，但实际发送仍需用户已授予的 credential/egress policy。

Resource、prompt、sampling 和 roots 也是数据出站面，不能只审计 `tools/call`。

## 13. Config、App Server 与持久化

Config authority、Plugin contribution、MCP manager reconcile 与 Core safe point 的完整组合流程
见 [`config.md`](config.md#6-pluginmcp-与-skill-接入流程)。

MCP ordinary config 属于 config/plugin authority，不属于 Thread event：

```text
server definition
enabled scope
transport declaration
credential reference
workspace/root grant
requested runtime permissions
local tool policy override
```

secret、PID、request ID、OAuth verifier、SSE cursor 和 live session ID 不持久化为 ordinary config。

目标 App Server surface 分为：

| 类别 | 方法示例 | 语义 |
| --- | --- | --- |
| Config | `mcp/server/list`、`mcp/server/update` | typed command 管理定义和 enablement |
| Runtime | `mcp/server/connect`、`mcp/server/disconnect` | process-local lifecycle intent |
| Catalog | `mcp/tool/list`、`mcp/resource/list`、`mcp/prompt/list` | 读取 snapshot |
| Auth | `mcp/auth/start`、`mcp/auth/complete`、`mcp/auth/revoke` | connection-owned OAuth flow |
| Diagnostics | `mcp/server/status`、`mcp/server/log/read` | redacted health 和 bounded logs |

命名只是目标语义，实施时必须与 `zeta-app-server-protocol` 同步生成 schema/TypeScript。配置修改
继续使用 `CommandId` 和 exact typed payload replay；connect/disconnect 是 runtime intent，不占用
Session/Thread sequence。

## 14. 错误与可观测性

错误至少分为：

```text
DefinitionError
PermissionDenied
CredentialUnavailable
TransportStartFailed
TransportLost
InitializeRejected
UnsupportedProtocolVersion
CapabilityUnavailable
CatalogInvalid
SchemaInvalid
RequestTimedOut
RequestCancelled
ProtocolError
ToolExecutionError
OutputRejected
UnknownOutcome
```

不能把它们全部压成 `McpError(String)`。用户可见错误保持稳定 code 和安全 message；诊断可额外
记录 server ID、connection generation、method、duration 和 redacted upstream code。

永不记录：

- Authorization/Cookie/header；
- OAuth code/verifier/token；
- 完整 tool arguments/result 默认值；
- resource/prompt 正文；
- 可能包含秘密的 endpoint query；
- child process 完整 env。

metrics 可记录连接状态、初始化耗时、catalog generation、call latency、timeout/cancel/error count、
queue saturation 和 output rejection。

## 15. 目标目录

第一版保持单 crate、private modules 和显式 public export：

```text
zeta-rs/mcp/src/
├── lib.rs
├── definition.rs
├── identity.rs
├── error.rs
├── session/
│   ├── mod.rs
│   ├── lifecycle.rs
│   ├── router.rs
│   └── catalog.rs
├── protocol/
│   ├── mod.rs
│   ├── message.rs
│   ├── capability.rs
│   └── content.rs
├── transport/
│   ├── mod.rs
│   ├── stdio.rs
│   └── streamable_http.rs
├── adapter/
│   ├── mod.rs
│   ├── tool.rs
│   ├── resource.rs
│   └── prompt.rs
├── client_feature/
│   ├── mod.rs
│   ├── roots.rs
│   ├── sampling.rs
│   └── elicitation.rs
└── *_tests.rs
```

不预建空模块。每个新增 trait 必须带 doc comment，说明实现方如何处理 cancellation、deadline、
security 和 identity。实现模块超过约 500 LoC 时按 lifecycle/primitive/transport 拆分；测试使用
显式 sibling `#[path = "..._tests.rs"]`。

## 16. 分阶段实施

### Phase M0：contract fixture

- 固定支持的 protocol revision；
- 建立 JSON-RPC/schema fixtures 和 fake in-memory transport；
- 实现 initialize/version/capability negotiation；
- 验证 frame、page、schema、content 和 queue limits。

完成条件：fake server 可覆盖正常、错误、late response、cancel 和 version mismatch。

### Phase M1：stdio + tools vertical slice

- process supervisor、stdio framing 和 graceful shutdown；
- tools/list pagination、catalog snapshot 和 list-changed；
- frozen tool binding、argument/output validation；
- 接入 Core Turn tool loop、approval、durable Tool Call/Result 和 UnknownOutcome。

完成条件：server crash、取消和 Thread recovery 不会静默重放有副作用调用。

### Phase M2：resources、prompts 与 roots

- resource/prompt list/read/get；
- context provenance、size/token budget 和 Resource store；
- workspace root projection 与 list-changed；
- Desktop/CLI/TUI 的只读 catalog 与显式选择入口。

完成条件：任何 MCP 内容都不会在未选择时自动进入模型 context。

### Phase M3：Streamable HTTP + OAuth

- POST/GET/SSE、session、resume 和 reconnect；
- protected resource metadata、PKCE、credential reference 和 revoke；
- endpoint/origin/redirect/egress policy。

完成条件：token 不进入 config/log/event，断线不被误判为取消或成功。

### Phase M4：可选 client features

- 先完成 App Server owner-directed interaction；
- 分别评审 sampling、elicitation 和 tasks；
- 每项独立 capability gate、budget、approval 和 recovery tests；
- 未完成的 capability 不在 initialize 中声明。

## 17. 验证门

除 workspace 常规 `fmt/clippy/test` 外，MCP 必须覆盖：

- revision negotiation 和 capability misuse；
- stdio stdout 污染、oversized frame、stderr flood、child crash；
- HTTP origin、redirect、session hijack、SSE duplicate/resume；
- pagination loop、list-changed race 和 catalog generation；
- tool alias collision、schema bomb、invalid structured output；
- approval denial、cancel-before-send、cancel-after-send、late response；
- disconnect 后 read retry 与 write unknown outcome；
- root traversal/symlink、resource URI confusion 和 oversized content；
- OAuth state/PKCE/audience/token redaction；
- reconnect 时旧 request、subscription 和 binding 不污染新 generation；
- shutdown deadline 与无 orphan child/process tree。

实现必须使用 deterministic fake clock/transport/server；单元测试不能依赖公网 MCP server 或真实
OAuth provider。
