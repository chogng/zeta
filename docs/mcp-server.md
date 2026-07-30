# `zeta-mcp-server`：将 Zeta Agent 暴露为 MCP 能力

> 物理位置：`zeta-rs/mcp-server/`  
> Rust crate：`zeta_mcp_server`，binary：`zeta-mcp-server`  
> 当前状态：stdio 与 authenticated Streamable HTTP、bounded progress、form interaction 和
> durable invocation recovery 已实现；独立 App Server event driver、SSE redelivery、
> multi-tenant control plane 与 remote Agent bridge 仍为 Proposed  
> Crate implementation contract：[`zeta-rs/mcp-server/README.md`](../zeta-rs/mcp-server/README.md)  
> MCP client runtime：[`mcp.md`](mcp.md)  
> App Server wire contract：[`zeta-app-server-api.md`](zeta-app-server-api.md)  
> App Server client lifecycle：[`app-server-client.md`](app-server-client.md)  
> 内建多 Agent：[`core-multi-agent.md`](core-multi-agent.md)

本文是“外部 MCP Host 如何调用 Zeta Agent”的 canonical 系统文档。MCP client session、
外部 capability catalog 和 tool/resource/prompt adapter 由 [`mcp.md`](mcp.md) 拥有；本地
parent/child delegation、context inheritance、join、budget 和 recovery 由
[`core-multi-agent.md`](core-multi-agent.md) 拥有。

## 1. 决策

Zeta 应提供独立的 MCP server adapter，使 Codex、其他 Agent host 或另一个 Zeta 实例能够把
Zeta Agent 作为外部能力调用。它通过 App Server 的 canonical Session/Thread/Turn API 启动和
继续任务，不直接组装 `zeta-core`、model provider、Tool runtime、policy 或 stores。

```text
External MCP Host
        │ MCP over stdio / Streamable HTTP
        ▼
zeta-mcp-server
        │ typed App Server client
        ▼
App Server composition root
        │
        ▼
Session / Thread / TurnExecutor
```

同进程 Zeta Agent 创建 child Agent 时不经过 MCP server。该路径仍由
`MultiAgentCoordinator` 直接建立 durable delegation：

```text
local parent Agent ──► MultiAgentCoordinator ──► child Thread
```

另一个进程或机器上的 Zeta 可以通过 MCP 被调用；若调用方希望把它视为正式 child Agent，
调用方必须在本地先建立 delegation，再由 remote Agent adapter 将执行映射到 MCP。MCP
transport 本身不创造 parent/child 语义。

## 2. 产品场景与边界

| 场景 | 首选入口 | 语义 |
| --- | --- | --- |
| Zeta Desktop、CLI 或 TUI 控制本地 Zeta | App Server | 完整产品控制面 |
| 本地 Agent 创建 child Agent | `MultiAgentCoordinator` | 原生 delegation 和 Agent tree |
| Codex、Claude 或其他 MCP Host 调用 Zeta | `zeta-mcp-server` | Zeta 作为外部 Agent tool |
| Zeta A 调用独立的 Zeta B | `zeta-mcp` → `zeta-mcp-server` | 跨 runtime Agent 调用 |
| Zeta A 将 Zeta B 纳入自己的 Agent tree | remote Agent adapter + MCP | 本地 delegation，远端 execution |

`zeta-mcp-server` 不是：

- `zeta-mcp` 的服务端模块；两者方向和依赖独立；
- 内建多 Agent 的替代品；
- App Server 的第二套 Session/Thread authority；
- scheduler、worker lease 或跨机器 execution protocol；
- 允许调用方注入任意 system/developer instruction、raw config 或未授权 workspace 的后门；
- 将 MCP request ID 当作 durable Zeta identity 的兼容层。

## 3. 当前仓库状态

当前已实现：

- `zeta-mcp-server` crate、独立 binary 和 `zeta mcp-server` CLI 入口；
- MCP `2025-11-25` stdio 与 Streamable HTTP initialize、ping、tools/list、tools/call、
  `notifications/cancelled`、progress 和 form elicitation；
- `zeta` 创建 Session/root Thread/Turn，`zeta-reply` 继续同一 principal 获得授权的 Thread；
- typed App Server client 映射、Thread subscription、bounded polling/notification drain、
  有界 progress 和 final result；
- caller-generated invocation identity、进程内 single-flight、跨进程重启 exact replay、
  payload conflict 和 principal-scoped Thread binding；
- approval/user-input 到 MCP `elicitation/create` 的映射，以及 exact typed interaction resolve；
- bearer authentication、Origin 校验、MCP session/protocol header、DELETE termination、
  SSE Tool call response 和连接数限制；
- deadline/EOF/client cancellation 到精确 `turn/interrupt` 的传播、unknown-outcome grace，
  以及 client-cancelled request response suppression；
- protocol、真实 in-process App Server、binary stdio restart 和 HTTP socket/SSE tests；
- App Server 已提供 `session/create`、`session/thread/create`、`turn/start`、subscription 和
  canonical Session/Thread updates；
- `zeta-app-server-client` 已有 typed methods、schema hash 校验和 embedded
  `InProcessTransport`；
- `open_local_app_server` 是本地 composition root，`InProcessAppServer` 允许多个 MCP HTTP
  session 连接同一个 embedded host；
- Session、Thread、Turn 和 Item 已由 store/reducer 持久化。

当前缺口：

- 当前 `InProcessTransport` 仍以同步 `round_trip` 和 request 后 `drain_notifications` 工作，
  MCP adapter 通过 Thread subscription + bounded polling/drain 转发进度与 interaction，但还
  不是通用、独立、可唤醒的 event driver；
- 目标 `AppServerSession`、可克隆 request handle、独立 event stream 和显式 shutdown 尚未完成；
- `MultiAgentCoordinator` 和 remote Agent adapter 仍是 Proposed；
- dynamic Tool interaction、artifact reference 和命名 execution profile 尚未完成；
- HTTP 尚无 independent GET SSE、`Last-Event-ID` redelivery、OAuth、multi-tenant workspace
  binding、built-in TLS 或 remote App Server backend；
- receipt store 尚无多进程 file lock；一个 state root 当前只能运行一个 MCP server process。

本文以下章节同时固定 Current surface 与 Proposed 演进；当前 HTTP listener 是 remote MCP
adapter，不等于已具备公网 multi-tenant service 或 remote App Server execution plane。

## 4. 所有权

### 4.1 `zeta-mcp-server` 拥有

- MCP server initialize、capability advertisement、stdio 与 HTTP session lifecycle；
- MCP tool definition、input/output schema 和 server-side validation；
- MCP request 与 App Server request/Turn 的 correlation；
- MCP cancellation 到精确 App Server operation 的传播；
- Thread update 到有界 progress/final result 的转换；
- approval/user-input 与 MCP form elicitation 的双向映射；
- principal-scoped invocation receipt、replay 与 caller-to-Thread binding；
- HTTP bearer、Origin、session/protocol validation 和 connection limit；
- caller-visible `session_id`、`thread_id` 和 `turn_id` 的安全投影；
- connection-scoped pending call、backpressure、deadline 和 bounded diagnostics；
- MCP wire error 与 App Server/domain error 的稳定映射；
- Proposed：artifact reference、MCP task capability、SSE resume、OAuth/multi-tenant control
  plane、remote App Server backend 和 remote Agent bridge。

### 4.2 App Server 与核心保持拥有

| 能力 | Canonical owner |
| --- | --- |
| Session/Thread/Turn 创建和状态迁移 | App Server → Core |
| transcript、Item 和 durable sequence | Thread store/reducer |
| model、Tool、policy、approval 和 sandbox | Core ports + composition root |
| Config、credential 和 workspace resolution | 对应 authority/runtime |
| 本地 child delegation、join、budget 和 cancellation tree | `MultiAgentCoordinator` |
| model/tool execution | `TurnExecutor` / `ToolScheduler` |
| Desktop/TUI interaction owner selection | App Server |

`zeta-mcp-server` 不直接依赖 store，不写 Thread event，不持有第二份 transcript，也不根据 MCP
connection 状态推断 durable Turn 已成功或失败。

## 5. 依赖方向与部署形态

目标依赖：

```text
zeta-mcp-server
        │
        ▼
zeta-app-server-client ──► zeta-app-server-protocol
        │
        ├─ embedded backend ──► zeta-app-server
        └─ future remote backend ──► existing App Server
```

禁止：

```text
zeta-mcp-server ─X─► zeta-core internals
zeta-mcp-server ─X─► session/thread stores
zeta-core       ─X─► zeta-mcp-server
zeta-mcp       ─X─► zeta-mcp-server
```

stdio 与 Streamable HTTP 都通过 `zeta-app-server-client` 连接同一个 embedded App Server
composition。HTTP MCP session 拥有独立 App Server connection，但共享同一个 host 和 durable
receipt authority；它不会引入另一套 execution engine。Remote App Server backend 仍为
Proposed。

## 6. Agent 工具接口面

当前开发期只暴露最小 Agent-as-tool surface：

| Tool | 作用 | 返回时机 |
| --- | --- | --- |
| `zeta` | 创建独立 Session/root Thread 并执行第一个 Turn | Turn terminal 或 interaction policy 阻塞 |
| `zeta-reply` | 向指定 Thread 提交后续 Turn | 该 Turn terminal 或阻塞 |

当前 `zeta` 输入：

- `invocationId`：1–128 bytes 的受限 ASCII caller identity，用于 principal-scoped
  single-flight、跨重启 replay/resume；
- `prompt`：非空，最大 256 KiB；
- 可选 `timeoutMs`：由 server 的 10 分钟上限约束。

`zeta-reply` 额外接收 principal-authorized `threadId`。Workspace 由启动 server 的
`ZETA_WORKSPACE_ROOT` 固定，不是 Tool argument；model、provider、policy 和 credential 继续由
App Server/config authority 解析。

Adapter 派生的 App Server `commandId` 同时包含 principal hash、`invocationId` 和 operation，
因此两个 HTTP principal 使用相同 invocation identity 也不会互相重放 Session/Turn。

第一版不接受：

- 任意 raw config map；
- 调用方提供的 secret value；
- 任意 system/developer instruction replacement；
- `danger-full-access` 一类绕过 host policy 的 sandbox override；
- 把 parent transcript 整体作为隐式 context 读取的请求。

成功结果至少投影：

```text
invocation identity
session_id
thread_id
turn_id
terminal status
bounded final content
```

结果不包含完整 transcript、secret、raw model response、未过滤 Tool output 或内部 store location。
`zeta-reply` 必须校验 caller 是否仍有权访问目标 Thread，不能仅凭猜到的 `thread_id` 继续任务。
stdio 使用 local-user principal；HTTP principal 由 bearer token 的不可逆 hash 派生。
caller-to-Thread binding 持久化在 state root，因此新的 connection/process 仍可恢复授权，但
不能跨 principal 猜测 `thread_id`。

## 7. 端到端流程

### 7.1 启动（Start）

```text
MCP tools/call(zeta)
→ validate schema, caller scope, invocation identity and limits
→ start initialized in-process App Server client
→ session/create
→ session/thread/create
→ thread/subscribe
→ turn/start
→ bounded thread/read polling + notification drain
→ project bounded progress
→ forward approval/user input through elicitation/create when needed
→ resolve exact interaction identity
→ map terminal/blocked state to bounded MCP result
→ return final content + stable Zeta identities
```

这是 Current subscription + bounded polling/drain 路径。它可在 Turn 运行期间投影增量
progress 和 interaction，但 correctness 仍以 App Server durable snapshot/update 和最终 result
为准。目标独立 event driver 将替换轮询，而不改变 authority。

### 7.2 继续（Continue）

```text
MCP tools/call(zeta-reply)
→ authenticate caller-to-thread binding
→ validate thread is eligible for another Turn
→ turn/start
→ poll the exact Thread/Turn snapshot
→ return exact Turn result
```

继续已有 Thread 不创建新的 Session，也不把调用方当前对话自动复制进 Zeta context。调用方想
传递的新信息必须存在于本次显式 prompt 或授权 artifact reference 中。

### 7.3 取消与断开连接

MCP cancel notification 必须映射到该 MCP request 对应的精确 Turn/operation。Transport EOF 或
HTTP disconnect 不自动证明 Turn cancelled：

- server 发出 best-effort cancellation；
- 当前 adapter 调用精确 `turn/interrupt`，并给 canonical terminal state 两秒 grace；
- client 发送 `notifications/cancelled` 后，server 不再发送该 request 的 JSON-RPC response；
- 若连接已无法接收结果，持久状态仍由 App Server/Core 决定；
- 同一 principal 使用原 invocation identity 可在新 connection/process replay 或 resume；
- HTTP transport disconnect 不自动 interrupt；stdio EOF 会因为本地进程拥有该 connection 而
  cancel active work；
- 已开始 Tool 的副作用继续遵循 Zeta UnknownOutcome/reconciliation 语义。

## 8. 与内建子代理的关系

| 维度 | 内建 child Agent | MCP 调用 Zeta |
| --- | --- | --- |
| identity | `DelegationId` + child `ThreadId` | invocation identity + 独立 Session/Thread |
| topology | 同一 Session 的 Agent tree | 默认无 parent lineage |
| context | immutable `AgentContextSeed` | 显式 prompt/artifact input |
| policy | parent delegated ceiling 的交集 | server execution profile + caller grant |
| lifecycle | spawn/send/join/cancel/close | MCP call/reply/cancel |
| recovery | durable delegation/result delivery | invocation dedupe + App Server recovery |
| UI | native Agent tree | external/MCP-origin task |

不能仅因为调用方也是 Zeta，就把 MCP-created Thread 标记为 `AgentSpawn`。只有 remote Agent adapter
已经在调用方 Core 内创建 `DelegationId`、context seed、budget reservation 和 result delivery
contract 时，它才能把远端执行结果作为 child result 接回：

```text
parent Thread
→ MultiAgentCoordinator creates Delegation
→ RemoteAgentAdapter invokes remote Zeta over MCP
→ remote Zeta runs independent Thread
→ adapter validates bounded result
→ parent records DelegationResultReceived
```

远端 Zeta 的 `thread_id` 是 execution correlation，不替代调用方的 `DelegationId`。

## 9. 交互、权限与信任

Agent invocation 可能触发 shell、filesystem、network、credential 或 external mutation。MCP
server 不能因为调用者请求了某种能力就自动批准。

当前规则：

- stdio caller 与 OS user 相同不等于拥有所有 workspace/capability；
- workspace、execution profile 和 model 都由 host authority 解析；
- 调用方文本始终是 user-controlled content，不能升级为 system policy；
- 若 client 不支持所需 interaction/elicitation，Turn 必须以可解释 blocked outcome 暂停或失败；
- 疑似请求 password、token、credential、payment card 或 private key 的 user-input 不通过 form
  elicitation 转发；
- approval response 必须绑定精确 interaction/request identity；
- server 不代表用户批准 side effect；
- MCP progress、log 和 error 不包含 token、credential、完整 environment 或敏感 prompt。

当前 HTTP 已有 bearer authentication、精确 Origin allowlist 和 process connection limit，但
尚无 tenant/workspace provisioning、OAuth、redirect/egress control plane、built-in TLS 或
per-principal distributed rate limit。公网部署必须由 TLS/auth reverse proxy 补齐这些边界。

## 10. Progress、结果与兼容性

MCP client 对 progress 和 custom notification 的支持不一致。最终 `tools/call` result 是唯一必须
可消费的完成载体；notification 只提供增量体验，不能成为 correctness authority。

事件投影规则：

- 只发送 caller 有权看到的事件；
- 只投影 bounded lifecycle message，连续重复 message 去重，每次 call 最多 256 条；
- progress 使用 caller 的 exact `progressToken`；Zeta session/thread/turn identity 保留在最终
  structured result；
- approval、blocked、cancelled、failed 和 unknown outcome 不压成普通成功文本；
- 客户端忽略所有 progress 时仍能从最终 result 得到有界、准确的 terminal outcome；
- App Server event schema 演进先在 typed adapter 内处理，不把内部 DTO 原样变成 MCP public API。

## 11. 失败与恢复

| Failure | Required semantics |
| --- | --- |
| schema/caller validation failure | 不创建 Session，返回 stable invalid-request error |
| App Server 尚未 ready | 不启动 Turn，返回 infrastructure error |
| duplicate invocation | 同一 payload exact replay/resume 或 in-progress；不同 payload conflict |
| transport loss before start commit | 可安全重试同一 invocation identity |
| transport loss after start commit | 新 connection/process 以 durable receipt + deterministic command identity resume/replay |
| interaction unsupported | durable blocked/failed outcome，不自动批准 |
| Turn cancelled | client cancel 不返回原 response；server deadline 返回 canonical interrupted/unknown outcome |
| output exceeds limit | 当前返回截断标记且不修改 durable Thread；artifact reference 为 Proposed |
| embedded server shutdown | bounded cancellation、driver join 和清晰 process exit |

MCP connection、request ID 和 process PID 都不是 durable authority。当前 adapter 将
principal-scoped receipt 原子持久化到 `<ZETA_STATE_ROOT>/mcp-server/receipts-v1.json`；它只
负责外部 invocation correlation 与 Thread authorization，canonical Session/Thread/Turn 状态
仍以 App Server/Core 为准。当前无多进程 file lock，同一 state root 只能有一个 server writer。

## 12. 可观测性

允许记录：

- protocol revision、transport 和 negotiated capability；
- redacted caller/server identity；
- invocation/session/thread/turn correlation 的安全 hash 或 opaque ID；
- startup、queue、Turn 和 result latency；
- cancel、blocked、error、unknown outcome 和 truncation count；
- active connection/call count 和 backpressure。

禁止记录：

- 完整 prompt、reasoning、Tool arguments/result；
- secret、authorization header、完整 environment；
- raw MCP frame；
- 未验证 workspace path 或 artifact body。

## 13. 分阶段落地

### 阶段 MS0：契约与 App Server 前置条件

状态：部分具备。

- 完成 owned async `AppServerSession`、request handle、event stream 和 explicit shutdown；
- 固定 MCP revision、tool schema、invocation identity 和 error/result fixture；
- 使用 fake App Server session 覆盖 start、event、terminal、cancel 和 duplicate；
- 明确 external invocation provenance，不与 `AgentSpawn` 混用。

完成条件：adapter 不依赖同步 `drain_notifications`，重复 start 不会创建第二个任务。

当前已完成 tool schema、invocation identity、error/result、duplicate 和 cancel fixture；owned
async `AppServerSession` 与独立 event stream 尚未完成。MCP adapter 当前用 subscription +
bounded polling/drain 提供实时能力，仍应迁移到该通用 session。

### 阶段 MS1：stdio `zeta` 纵向切片

状态：Current 已完成。

- stdio initialize、tools/list 和 tools/call；
- embedded App Server composition；
- `zeta` → Session/root Thread/Turn；
- bounded progress 和 final result；
- safe execution profile、deadline、cancel 和 graceful shutdown。

完成条件：MCP Inspector/fake client 可以启动一个 Zeta Turn，并在 client 不消费 progress 时仍得到
准确结果。

当前已完成 stdio、embedded composition、Session/root Thread/Turn、deadline、cancel、graceful
shutdown、bounded progress 和 final result。

### 阶段 MS2：continue 与交互

状态：Current 部分完成。

- `zeta-reply` 和 caller-to-thread authorization；
- interaction/approval 的 owner-directed delivery；
- reconnect/query/replay；
- artifact reference 和 output truncation。

完成条件：继续请求不会串到错误 Thread，unsupported approval 不会被静默放行。

当前已完成 `zeta-reply`、principal-scoped durable Thread authorization、approval/user-input
delivery、跨重启 replay/resume 和 output truncation；dynamic Tool interaction 与 artifact
reference 尚未完成。

### 阶段 MS3：远程部署

状态：Current 部分完成。

- 已完成 Streamable HTTP POST/DELETE、JSON/SSE 响应、Bearer 认证、Origin 白名单、安全会话
  ID、协议标头校验和进程连接数量限制；
- 已完成 HTTP 会话重建后，使用持久化调用身份恢复 Agent 调用；
- 尚未实现独立 GET SSE、`Last-Event-ID` 重投递、OAuth、租户/工作区绑定、按主体分布式限流、
  内置 TLS 以及重定向/数据外发控制面；
- 远程 App Server 后端仍处于计划阶段。

完成条件：向公网暴露前完成租户隔离、凭据、断开连接和滥用防护测试。

### 阶段 MS4：远程 Agent 桥接

- `MultiAgentCoordinator` 的 remote Agent port；
- local `DelegationId` 到 remote invocation 的绑定；
- context seed projection、budget/deadline 和 cancellation；
- bounded result/artifact validation 与 durable delivery。

完成条件：远端执行失败、迟到或重复结果不会破坏本地 Agent tree。

## 14. 验证

除 workspace 常规 `fmt/clippy/test` 外，必须覆盖：

- protocol revision、initialize 和 tools schema fixtures；
- start/continue 的 exact identity mapping；
- duplicate、late response、cancel、EOF 和 App Server restart；
- event gap、subscription cursor、out-of-order update 和 terminal result；
- unsupported progress/elicitation client；
- workspace/profile/model authorization；
- prompt injection、secret/log redaction 和 output limit；
- approval identity mismatch；
- external invocation 与 `AgentSpawn` provenance 不混淆；
- fake clock、fake App Server 和 deterministic transport；单元测试不依赖公网或真实模型。

## 15. 长期不变量

- App Server/Core 始终是 Session/Thread/Turn authority；
- `zeta-mcp-server` 始终是 adapter，不建立第二套 Agent runtime；
- 同进程子代理始终走 `MultiAgentCoordinator`，不经 MCP 自调用；
- MCP request ID 不等于 Session、Thread、Turn、Delegation 或 invocation identity；
- 外部调用默认创建独立 root task，不伪装成 child Agent；
- retry 不得静默重复启动可能产生副作用的 Agent Turn；
- 调用方不能通过 MCP 参数扩大 host policy、workspace、credential 或 sandbox 权限；
- transport 断开不等于 durable operation 已取消；
- progress 是体验层，最终结果和 App Server durable state 才是 correctness authority。
