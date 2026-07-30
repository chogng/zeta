# 会话与执行系统

> 物理位置：`zeta-rs/core/`
> Rust crate：`zeta_core`
> Canonical value 与 wire-neutral contract：[`protocol.md`](protocol.md)
> 上下文详细设计：[`core-context.md`](core-context.md)
> 多 Agent 详细设计：[`core-multi-agent.md`](core-multi-agent.md)
> 跨 Core、App Server、provider 与 Tool 的执行演进：
> [`zeta-agent-runtime-architecture.md`](zeta-agent-runtime-architecture.md)
> Tool shared contract、registry/binding 与 source adapter：[`tools.md`](tools.md)
> Config、Plugin、MCP 与 Skill snapshot：[`config.md`](config.md)
> Provider credential：[`model-provider.md`](model-provider.md#6-provider-credential-与-subscription-backend)
> Secret persistence：[`secrets.md`](secrets.md)
> Cancellation tree 实现：[`zeta-async-utils` README](../zeta-rs/async-utils/README.md)
> Session/Thread store ports：[`zeta-session-store`](../zeta-rs/session-store/README.md) /
> [`zeta-thread-store`](../zeta-rs/thread-store/README.md)
> Local recovery composition：[`zeta-rollout`](../zeta-rs/rollout/README.md)

## 快速理解

Core 是一次 Agent 工作的权威协调者：它推进 Turn、安排模型和工具调用，并保证状态按可恢复的
顺序提交，但不亲自实现模型、工具、网络或界面。

| 读者首先会问 | 直接答案 | 深入阅读 |
| --- | --- | --- |
| 一次工作保存在哪里？ | Session 聚合多个 Thread；每个 Thread 独立保存 Turn、Item 和逻辑序列 | [产品模型](#4-产品模型与执行模型) |
| 谁推进一个 Turn？ | `ThreadController` 保持单写者顺序，`TurnExecutor` 协调一次执行 | [核心组件](#5-核心组件) |
| 模型、工具和策略由谁实现？ | Core 只拥有消费方端口和调用顺序，具体实现由外部服务注入 | [依赖方向](#6-依赖方向与服务端口) |
| 什么时候算已经执行？ | 执行授权和开始事实必须先持久化，之后才能跨过副作用边界 | [提交与安全点](#7-durable-commit并发与安全点) |
| Core 是否拥有 UI 或传输？ | 不拥有；Desktop、CLI、TUI 和 App Server 只是不同入口 | [所有权边界](#3-所有权边界) |

## 1. 定位

`zeta-core` 是 Zeta 的 Agent 执行控制面，只负责五类事情：

1. Agent 生命周期；
2. Turn 执行；
3. 上下文管理；
4. Tool 调度；
5. 服务编排。

这里的“Agent 生命周期”包括 Session、Thread、Turn、delegation 的 durable 状态迁移，以及加载、
取消、等待、恢复和执行实例隔离。模型进程、MCP server、浏览器、终端和网络连接自身的生命周期
不属于 Core。

“服务编排”表示 Core 通过窄的 typed port 规定 model、tool、store、policy 与 capability 的调用
顺序、提交边界和 failure semantics。它不表示 service locator、依赖注入容器或具体服务实现。

Core 必须保留纯 reducer、durable commit ordering 和 recovery。它们不是额外的产品能力，而是
Agent 生命周期能够成为 authority 的前提。

长期不建立 `CoreRuntime`、`AgentRuntime`、`SessionRuntime` 或 `ThreadRuntime` 公共组件。
进程内执行状态确实存在，但由 `ThreadController`、`LoadedThreadState`、`TurnExecution` 等准确
名称表达。只有 Tokio、Wasm 或外部 Tool execution environment 这类真正的执行环境才使用
`runtime` 一词，而且其实现不属于 Core。

## 2. 当前状态

已经落地：

- Session/Thread reducer、projection 与 store port；
- `SessionCoordinator` 及可恢复 create/fork saga；
- `ThreadController`、command receipt、replay、conflict detection 与 recovery；
- per-Thread loaded projection、FIFO mutation gate、explicit incarnation、bounded execution mailbox
  与 idle eviction；
- durable Turn、Tool Call、Tool Result 与 interaction lifecycle；
- provider-independent `ModelService`；
- `TextDelta` / `ReasoningDelta` Core streaming contract；
- `TurnExecutor` 的顺序 model → tool → model 循环；
- 从 durable `ThreadSnapshot` 派生有序 text/image 请求的基础 `ContextAssembler`。

尚未完成：

- provider wire-level streaming 与 App Server 独立 outbound worker；
- `TurnPolicySnapshot` / `ModelInvocationSnapshot` 的完整闭环；
- `ContextManager`、instruction precedence、context budget 与 compaction；
- 并行 Tool、通用 deadline、声明式 retry 与 reconciliation；
- durable multi-Agent delegation、跨 Thread message/result 与 Agent tree resource budget；
- 所有 durable boundary 的 fault injection。

当前实现是演进地基，不应被描述成完整的异步、多 Agent 执行系统。

## 3. 所有权边界

### 3.1 核心拥有

- Session membership、lineage、shared defaults 与 lifecycle 的协调；
- 每个 Thread 的逻辑单写者、durable commit、加载和恢复；
- Turn acceptance、执行、等待、取消、完成、失败和恢复；
- Agent delegation 的 spawn、message、join、cancel 与结果合并；
- command idempotency、expected sequence 和 committed update ordering；
- Turn policy snapshot 与 model invocation snapshot；
- 从 durable facts 派生 provider-neutral context；
- context window、预算、checkpoint 选择和 compaction 协调；
- model → tool → model loop 及 sequential/parallel Tool 调度；
- approval、structured user input、dynamic tool 与 capability wait；
- cancellation tree、deadline、迟到结果和旧 incarnation 拒绝；
- crash 后模型、Tool、interaction 与 delegation 的恢复决策；
- 将外部错误映射为稳定 Core outcome。

### 3.2 核心不拥有

- canonical shared value 的定义；它属于 `zeta-protocol`；
- stored envelope、event framing、JSONL/SQLite 和 fsync；
- provider HTTP DTO、endpoint、SSE decoder、认证与 credential；
- model catalog 的发现、缓存、合并与筛选；
- Tool、MCP transport、child process、browser 或 terminal 的具体实现；
- sandbox/OS enforcement、approval UI 与 connection owner；
- Config 文件、环境变量、secret persistence 或 credential refresh；
- JSON-RPC method、subscription cursor 与 transport lifecycle；
- Desktop、CLI、TUI 的 projection；
- provider prompt cache、连接池或模型驻留；
- 未经单独设计和授权的跨 Thread 长期 memory。

### 3.3 容易混淆的边界

| 概念 | Core 拥有 | 外层拥有 |
| --- | --- | --- |
| Model | 调用时机、snapshot、结果消费、safe point | provider 解析、HTTP、认证、wire codec |
| Tool | 选择、approval、排序、取消、结果提交、失败决策 | schema adapter、执行、sandbox、MCP/进程 I/O |
| Context | 选择策略、预算、compaction 协调、纯组装 | provider cache、未经授权的 memory retrieval |
| Agent | delegation、Thread 绑定、消息、结果、取消、预算 | UI 呈现、远端进程或 capability 生命周期 |
| Service orchestration | 调用 typed ports、状态推进、补偿与降级 | 构造 adapter、配置加载、进程启动、网络监听 |

## 4. 产品模型与执行模型

### 4.1 Product Session

`Session` 是产品级根 aggregate，拥有：

- task lifecycle；
- Thread membership；
- root/fork/spawn lineage；
- shared defaults；
- 自己的 durable sequence。

Session 不持有任何 Thread transcript，不代理 token delta，也不等待 model/Tool I/O。同一 Session
下的 Thread 必须能够并行执行。

### 4.2 Thread

`Thread` 是独立的历史、排序、上下文和恢复边界，拥有：

- immutable `SessionId`；
- durable sequence；
- ordered Turn 与 Item；
- compaction checkpoint；
- pending interaction/delegation facts；
- 一个逻辑状态提交者。

每个已加载 Thread 可以持有一个可丢弃、可重建的 `ContextManager`。ContextManager 不改变
Thread 的 authority；它只能从 durable Thread facts 和不可变 snapshot 派生状态。

### 4.3 Agent

当前长期方案中，一个活跃 Agent 绑定一个 Thread。`ThreadId` 可以先作为 Agent 的执行身份，
不急于增加一对一的 `AgentId` aggregate。父子工作的相关性由独立 `DelegationId` 表达。

只有未来出现“一个 Agent 身份需要跨多个 Thread 延续”的真实需求时，才引入 `AgentId`。
无论是否引入，context 始终绑定 Thread，而不是绑定 Session 或整个 Agent tree。

### 4.4 Turn 与操作

必须区分：

```text
Product lifecycle
  Session / Thread / Turn / Delegation 的 durable status

Loaded execution lifecycle
  Thread load / idle / executing / closing + incarnation

Operation lifecycle
  model invocation / tool execution / interaction delivery / agent message delivery
```

三层不能复用一个 status、operation ID 或 cancellation token。

## 5. 核心组件

```text
MultiAgentCoordinator (multi-Agent only)
  ├─ topology operations ──► SessionCoordinator
  └─ cross-Thread work ────► ThreadController

SessionCoordinator ────────► ThreadController
membership / defaults        one logical writer
                                   │
                          LoadedThreadState
                          └─ ContextManager
                                   │
                                   ▼
                              TurnExecutor
                  context → model → tools → model
                     │          │          │
                     ▼          ▼          ▼
               ContextPlan  ToolScheduler  interactions
                     │          │
                     ▼          ▼
              ModelService   ToolService
```

### 5.1 SessionCoordinator

负责：

- 创建、读取和恢复 Session；
- create、attach、fork、spawn、archive Thread 的结构协调；
- root/parent/child lineage；
- Session lifecycle 与 shared defaults；
- 跨 Session/Thread stream 的可恢复 saga。

不负责：

- Thread transcript 或 context；
- Turn loop；
- token/reasoning streaming；
- 等待 model、Tool 或 child Agent 完成。

### 5.2 ThreadController

每个已加载 Thread 对应一个逻辑状态提交者。实现可以是 mailbox task 或 keyed executor，但必须
满足：

- 同一 Thread 的结构性 mutation FIFO；
- 不同 Thread 可并行；
- model、Tool 和 Agent I/O 不占 projection lock 或 writer lease；
- interrupt、interaction response、Tool completion、model completion 和 Agent result 都回到
  同一提交序列；
- 每次 load 生成新的 `ThreadIncarnationId`；
- completion 携带 operation ID 与 incarnation；
- 空闲状态可以回收，恢复只依赖 durable facts。

“一个 Thread 一个常驻 task”只是实现选择，不是公共契约。

### 5.3 LoadedThreadState

这是 Core-private 的进程内状态：

```text
LoadedThreadState
├─ durable projection snapshot
├─ ThreadIncarnationId
├─ ThreadActivity
│  ├─ Idle
│  ├─ Executing(TurnExecution)
│  └─ Waiting(Interaction | Delegation)
├─ ContextManager
├─ bounded mailbox
└─ cancellation/task ownership
```

它不能进入 protocol，也不能成为第二个 durable aggregate。进程重启后必须能够丢弃并重建。

当前实现由 private `loaded_thread::LoadedThreads` 保存轻量 Thread slot registry；
`loaded_thread::ThreadSlot` 的 ticket gate 串行结构性 mutation，slot 内的
`LoadedThreadState` 保存 projection 与 incarnation。`mailbox::ThreadExecutionMailboxes`
为每个 incarnation 创建有界 FIFO lane，`ThreadExecutionContext` 将 operation ID、
incarnation 与 cancellation 绑定。worker 空闲 30 秒后只回收匹配 incarnation 的 lane 和
projection；下一次访问从 `ThreadStore` 重建。运行中的 execution 会阻止显式 recovery，
因此 completion 不可能跨越 incarnation；测试中的强制 stale context 也会被拒绝。

### 5.4 TurnExecutor

负责一个已经 accepted 的 Turn：

```text
freeze TurnPolicySnapshot
→ prepare context
→ create ModelInvocationSnapshot
→ invoke model
→ consume stream/final response
→ no Tool Call ──────────────────────────────► complete
→ validate and schedule Tool Calls
→ approval if required
→ durable Tool Call intent
→ execute Tools
→ durable Tool Result / UnknownOutcome
→ prepare next invocation
```

TurnExecutor 只能提出 typed effect；ThreadController 负责实际 durable append。它不能直接修改
projection、写 store 或提前发布 committed update。

当前单 Agent loop 不设置固定的模型调用次数上限：只要模型仍产生 Tool Call 等 follow-up，
Turn 就继续执行。安全边界应由可取消的 token/cost/deadline policy 和 durable usage accounting
表达，不能使用 approval 或 recovery 后会重置的进程内计数器。

### 5.5 ContextManager 与 ContextAssembler

两者必须分开：

- `ContextManager`：每个 loaded Thread 一个，负责 context 生命周期、选择、预算、window、
  baseline、checkpoint 和 compaction 决策；
- `ContextAssembler`：无状态纯组件，把已经确定的 `ContextPlan` 组装成
  provider-neutral `ModelRequest`；
- `ContextPlan`：一次 invocation 的不可变派生结果；
- durable Thread history：唯一权威历史。

ContextManager 可以缓存按 `ThreadSequence`、policy revision 和 model capability revision
索引的派生视图，但不能维护第二份 canonical transcript。完整契约见
[`core-context.md`](core-context.md)。

### 5.6 MultiAgentCoordinator

这是只在多 Agent 模式下参与的跨 Thread 协调组件，负责：

- spawn、send、join、cancel、close；
- durable delegation 与 delivery identity；
- parent/child cancellation；
- 最大深度、子 Agent 数、并发、token/cost/deadline budget；
- 协调 SessionCoordinator 创建 child Thread；
- 协调跨 Thread message/result 的 durable delivery。

它不持有任何 Thread context，也不读取 live ContextManager。父子 Agent 的上下文传递必须通过
不可变 `AgentContextSeed`，结果必须通过显式 durable Item 合并。完整契约见
[`core-multi-agent.md`](core-multi-agent.md)。

单 Agent 执行已经由 ThreadController、TurnExecutor、ContextManager 和 ToolScheduler 完整表达，
不再建立泛化的 `agent/` facade。`multi_agent/` 只包含单 Thread 之外的 delegation、spawn、
message、join、tree budget 和 recovery。

## 6. 依赖方向与服务端口

目标依赖：

```text
                         zeta-protocol
                              ▲
                 ┌────────────┼────────────┐
                 │            │            │
        zeta-session-store  zeta-core  zeta-thread-store
                 ▲            ▲            ▲
                 │            │ ports      │
                 └──── composition root ───┘
                              │
              model / tool / policy / capability adapters
```

允许：

- `zeta-core → zeta-protocol`；
- `zeta-core → zeta-tools`；
- `zeta-core → zeta-session-store`；
- `zeta-core → zeta-thread-store`；
- `zeta-core → zeta-async-utils`；
- 外层 adapter 实现 Core 定义的 consumer-owned port。

禁止：

```text
zeta-core → zeta-app-server(-protocol)
zeta-core → zeta-storage / zeta-rollout
zeta-core → zeta-api / concrete model provider
zeta-core → shell-command / file-system / apply-patch / MCP implementation
zeta-core → config files / credentials / secrets
zeta-core → Desktop / CLI / TUI
```

最小端口集合：

| Port | Core 为什么消费 | 实现位置 |
| --- | --- | --- |
| `SessionStore` | Session load/append | storage adapter |
| `ThreadStore` | Thread load/append | storage adapter |
| `WriterLease<Id>` | aggregate 单写者 | storage/host |
| `ModelService` | provider-neutral model invocation | model-provider adapter |
| `ToolService` | 已物化 Tool call 的执行 | built-in/MCP host |
| `PolicyService` | action approval/sandbox 决策 | host policy layer |
| `CapabilityBroker` | Turn-scoped capability 解析 | App Server/host |
| `CompactionService` | 生成候选 summary | model/host adapter |
| `Clock` | deadline 与可测试时间 | host |
| `IdGenerator` | 稳定 operation identity | host |

Port 必须是 consumer-owned interface。新 trait 必须说明角色、实现不变量、取消和错误约束。
公共 request 不使用语义含糊的 `bool` 或 `Option` 参数。

目标 host policy adapter 由 [`auto-review.md`](auto-review.md) 中的 `zeta-policy` 实现：
deterministic rule/grant 是 authority，LLM classifier 只提供 recommendation。Core 只消费最终
decision，并负责 durable approval、retry 与 unknown-outcome lifecycle。

Composition root 负责读取各 authority、构造 provider/tool/store/policy adapter、materialize
credential、注入 Core 并启动 transport。Core 不提供隐藏全局状态的 service registry。

## 7. Durable commit、并发与安全点

### 7.1 提交顺序

所有权威 mutation 使用同一顺序：

```text
receive intent/completion
→ validate projection + operation identity + incarnation
→ build typed event batch
→ reduce into candidate projection
→ append atomic batch
→ install committed projection
→ publish committed update
→ schedule next external operation
```

必须满足：

- append 失败不安装 candidate projection；
- notification 不早于 durable append；
- live commit 与 recovery 使用同一 reducer；
- command receipt 与首个业务 event 原子提交；
- transient delta 可以丢失，final Item 必须 durable；
- model/Tool/Agent completion 不冒用用户 `CommandId`。

### 7.2 安全点

默认 safe point：

- model invocation 开始前；
- 一组 Tool Results durable commit 后；
- interaction resolved 后恢复执行前；
- Agent message/result durable delivery 后；
- Turn 尚未开始或已经 terminal；
- compaction checkpoint 已验证并 durable commit 后。

model、provider、tool availability、context policy 与非安全配置只在 safe point 进入新 snapshot。
安全策略允许执行中单调收紧，不能静默放宽。

### 7.3 取消

```text
Core shutdown source
└─ root Agent/Thread source
   ├─ Turn source
   │  ├─ model invocation source
   │  ├─ tool execution source
   │  └─ interaction source
   └─ child Agent source
      └─ child Turn/operation sources
```

父取消传播到所有后代；取消 child 不影响 parent 或 sibling。Cancellation 是协作式 best effort，
不能证明外部副作用未发生，也不能越过 durable terminal event 提前对外宣称完成。

当前单 Agent execution 的实际链路已经闭合：

```text
turn/interrupt
→ ThreadExecutionMailboxes::cancel(exact ThreadId + TurnId)
→ TurnExecutor-owned CancellationToken
├─ ModelService → ModelInvoker → OperationClient
└─ ToolScheduler → ToolService → exec / MCP adapter
```

同步 provider HTTP 在 token 取消后停止 Core/operation 层等待并禁止 retry；因为当前
`zeta-http-client` 仍是同步 unary transport，已经进入 socket 的 attempt 由 bounded transport
timeout 收束，其迟到 response 被丢弃。已越过 durable execution-start boundary 的 Tool 不会被
伪装为安全未执行：terminal Turn 保留 execution-start marker 且没有 Tool Result，恢复时按 unknown
outcome 处理，exact call 不自动重放。

## 8. 模型调用与流式处理

一个 Turn 固定：

```rust
pub struct TurnPolicySnapshot {
    // approval, sandbox intent, capability scope,
    // execution limits, agent role and policy revision
}
```

每次 provider 调用固定：

```rust
pub struct ModelInvocationSnapshot {
    // resolved model, ContextPlan, tools, output/reasoning settings,
    // provider/config/catalog revisions and invocation identity
}
```

`ModelService` 目标契约必须：

- async 且可取消；
- 输入 canonical request/snapshot，而不是 prompt `String`；
- 输出 typed stream event 与 typed terminal response；
- 区分 refusal、overflow、rate limit、authentication、transport 与 invalid response；
- 保留受控 usage/response reference，不泄漏 wire DTO；
- 不在 trait 内选择全局默认模型。

Transient streaming 必须携带 stream incarnation 与 cursor，通过有界 channel 发布。Channel
饱和时允许合并或丢弃非关键 delta；durable completion 不能依赖 transient stream，也不能被它
阻塞。

## 9. 上下文

每次 model invocation 都从以下不可变输入派生：

```text
durable Thread snapshot
+ TurnPolicySnapshot
+ resolved model limits/capabilities
+ instruction/environment snapshot
+ available Tool definitions
+ verified compaction checkpoints
+ optional AgentContextSeed / delivered Agent results
```

组织过程固定为：

```text
ContextManager
  → validate source revisions
  → select history/injected fragments/checkpoint
  → apply precedence and context budget
  → produce ContextPlan
ContextAssembler
  → validate structural invariants
  → assemble provider-neutral ModelRequest
```

Compaction 不删除 event log。任何影响未来 prompt 的 checkpoint、seed 或 result 都必须先成为
durable fact。ContextManager 的缓存和 baseline 可以丢失。详细的 ownership、预算、compaction、
恢复与目录设计见 [`core-context.md`](core-context.md)。

## 10. 多 Agent

多 Agent 使用同一 Session 下的独立 Thread：

```text
Session
└─ parent Thread
   ├─ child Thread A
   └─ child Thread B
```

但三种关系不能混用：

- fork lineage：产品分支关系；
- Agent delegation：谁委派了什么工作；
- context inheritance：child 首次 invocation 能看到什么。

Spawn 必须固定 parent Thread sequence，并选择显式 inheritance mode。Spawn 后父子 context
立即分叉，不共享可变历史。父子 communication/result 使用 stable message/delegation ID 和
durable delivery；不能直接读取对方 ContextManager，也不能自动把 child transcript 拼进 parent
prompt。

Session 只串行拓扑提交；MultiAgentCoordinator 负责 delegation；ThreadController 负责各自
Thread；ContextManager 负责各自 context。详细契约见
[`core-multi-agent.md`](core-multi-agent.md)。

## 11. 工具、交互与能力

### 11.1 工具边界

```text
model emits Tool Calls
→ durable Tool Call intents
→ ToolService materializes ActionReviewRequest
→ PolicyService evaluates sandbox / grant / approval / block
→ durable approval wait and exact continuation when required
→ durable ToolExecutionStarted with action/policy/authority identity
→ execute outside Thread writer
→ durable ordered Tool Results / UnknownOutcome
→ next model invocation
```

`ToolScheduler` 拥有校验、approval gate、并行计划、deadline、cancellation、operation correlation、
deterministic result ordering、retry 与 unknown-outcome 决策。

`ToolService` adapter 拥有具体参数转换、MCP/process/browser/terminal I/O、sandbox enforcement、
有界输出采集和 tool-specific reconciliation。Adapter 不得写 Thread store。

并行 Tool 只有在 policy、Tool definition 和 resource conflict 检查均允许时启用。完成顺序不决定
transcript 顺序；默认按 model call order 提交结果。

Retry policy 使用自描述 enum：

```text
Never
SafeRead
IdempotentWrite(operation key)
ReconcileBeforeRetry
```

未声明幂等性的写操作和 outcome 未知的副作用默认不自动重试。

### 11.2 交互

Approval、structured user input、dynamic tool 和 capability request 使用同一模式：

```text
Core commits InteractionRequested
→ App Server selects owner and delivers
→ client resolves exact RequestId
→ Core validates Thread/Turn/RequestId/sequence
→ Core commits InteractionResolved/Cancelled
→ execution resumes at a safe point
```

Core 拥有 durable request/wait/resolve/cancel。App Server 拥有 connection owner、投递、
disconnect 和 outbound queue。

Approval 使用独立 typed payload，不复用普通 user-input 文本：

```text
ActionApprovalRequest
  action_digest: SHA-256
  policy_revision
  complete capability set
  reason
  sandbox_denial?: structured safe-to-retry denial

ActionApprovalResponse
  ApproveOnce | Decline
```

Core 的 `PolicyService` 接收 immutable `ActionReviewRequest` 并返回 `ExecutionDecision`。
`AskUser` 必须先通过 `durable_approval_request` 绑定并转换，之后才能提交
`InteractionRequested`。Reducer 校验 digest、revision、非空 capability scope 和重复
capability，并把 Turn 转为 `WaitingForApproval`。响应通过原有的 RequestId、Thread/Turn identity
和 sequence gate 提交；`ApproveOnce` 只表示该 exact request 获得一次性批准，不创建持久或
跨 action grant。Tool scheduler 将它物化为绑定 RequestId、ToolCallId 和完整 approval payload
的 `OneTimeToolGrant`，并在恢复时重新验证 action digest、policy revision 与 capability。

当 sandboxed attempt 返回结构化 `SafeToRetry` denial 时，Core 先执行二次 policy review。
`AskUser` 会把完整 denial 写入新的 durable `ActionApprovalRequest`，不先提交 Tool Result。
`ApproveOnce` 只授权原始 action/policy/capability/ToolCall 组合的一次非 sandbox 重试；Core 在重试前
提交 `ToolExecutionEscalated`。若重启后已有 escalation marker 而没有 Tool Result，scheduler
提交 outcome-unknown failure 并禁止再次执行。`Decline`、`MayHaveSideEffects` denial 或 binding
漂移都不会触发重试。

副作用开始前必须提交 `ToolExecutionStarted`。如果恢复时存在 unresolved Tool Call 但没有
start marker，scheduler 可以从 policy safe point 继续；如果 start marker 已存在而 Tool Result
缺失，则提交 outcome-unknown Tool failure，禁止自动重放。Host 在完成 Tool/Policy service
装配后通过 `TurnExecutor::resume_recovered_tool_continuations` 恢复这些 Turn。

## 12. 恢复、错误与背压

### 12.1 恢复原则

恢复只依赖 durable facts：

- terminal Turn 保持 terminal；
- waiting interaction 恢复为 waiting；
- in-flight model request 从最后 safe point 按明确 policy 重建；
- 已提交 Tool intent 但没有 result 的副作用调用进入 `UnknownOutcome` 或 reconciliation；
- `Cancelling` 最终收敛到 `Interrupted`；
- reload 生成新 Thread incarnation，旧 completion 全部拒绝；
- Session create/fork/spawn saga 继续或补偿；
- Agent message/result delivery 通过 stable ID 去重并继续；
- ContextManager 从 Thread history、checkpoint 与 seed 重建。

### 12.2 错误分类

Core error 至少区分：

- validation / invalid transition；
- optimistic concurrency / command conflict；
- aggregate not found；
- store / lease failure；
- model config/capability/transient/permanent failure；
- Tool unavailable/denied/failed/unknown outcome；
- interaction timeout/cancel/owner unavailable；
- delegation unavailable/failed/delivery conflict；
- overloaded/closing/stale completion；
- internal invariant violation。

内部错误可以保留 source chain；wire mapping 只暴露稳定、安全的 code 与 message。

### 12.3 背压

以下集合必须有界：

- per-Thread mailbox；
- model delta channel；
- parallel Tool set；
- active child Agent set；
- cross-Agent delivery queue；
- committed/transient update publisher；
- App Server outbound queue。

Transient delta 可以合并或丢弃；durable event、terminal outcome、interaction request 和 Agent
result 不能无声丢失。

## 13. 目标目录与公开 API

```text
zeta-rs/core/src/
├─ lib.rs
├─ error.rs
├─ services.rs
├─ session/
│  ├─ coordinator.rs
│  ├─ reducer.rs
│  ├─ projection.rs
│  └─ saga.rs
├─ thread/
│  ├─ controller.rs
│  ├─ loaded.rs
│  ├─ reducer.rs
│  ├─ projection.rs
│  ├─ commit.rs
│  └─ recovery.rs
├─ turn/
│  ├─ executor.rs
│  ├─ lifecycle.rs
│  ├─ invocation.rs
│  └─ interaction.rs
├─ context/
│  ├─ model.rs
│  ├─ plan.rs
│  ├─ assembler.rs
│  ├─ budget.rs
│  └─ validation.rs
├─ context_manager/
│  ├─ manager.rs
│  ├─ selection.rs
│  ├─ window.rs
│  └─ compaction.rs
├─ multi_agent/
│  ├─ mod.rs
│  ├─ coordinator.rs
│  ├─ delegation.rs
│  ├─ spawn.rs
│  ├─ messaging.rs
│  ├─ join.rs
│  ├─ budget.rs
│  └─ recovery.rs
└─ tool/
   ├─ scheduler.rs
   ├─ policy.rs
   └─ outcome.rs
```

约束：

- 模块默认 private，只从 `lib.rs` named export 稳定 API；
- implementation module 目标低于 500 LoC；
- 超过约 800 LoC 的文件不继续增加新功能；
- 新测试使用 sibling `*_tests.rs` 与显式 `#[path = "..._tests.rs"]`；
- 不创建 `common.rs`、`utils.rs`、service registry 或巨型 facade；
- public trait 必须有职责与实现约束 doc comment；
- public API 不使用语义含糊的 bool/Option 参数；
- loaded phase、mailbox message、task handle、ContextManager cache 与 incarnation 不 re-export。

公开面只保留：

- `SessionCoordinator` / `ThreadController` / `MultiAgentCoordinator` handle；
- typed request/result；
- read-only projection/snapshot；
- consumer-owned service ports；
- 稳定 Core error；
- 最小 update/subscription source。

## 14. 测试与演进

### 14.1 验证矩阵

Pure tests：

- Session/Thread/delegation reducer 的合法与非法 transition；
- context precedence、Tool 配对、budget 与 checkpoint selection；
- Tool schedule 与 deterministic aggregation；
- snapshot 和 safe-point semantics。

Concurrency tests：

- 同 Thread FIFO、不同 Thread/Agent 并行；
- I/O 期间 projection lock 和 writer lease 已释放；
- interrupt/completion race；
- interrupt during active model transport / Tool execution；
- duplicate/late/stale-incarnation completion；
- child cancellation 不影响 parent/sibling；
- mailbox、Tool、Agent capacity saturation；
- idle eviction 后恢复。

Fault-injection tests：

- Turn accepted/started；
- Tool Call committed/execution started/result committed；
- interaction requested/delivered/resolved；
- Session Thread planned/created/attached；
- delegation requested/child created/result sent/result received；
- compaction generated/checkpoint committed。

### 14.2 落地顺序

1. 将现有 Session/Thread 大文件按 aggregate 拆入目标目录；
2. 已引入 `LoadedThreadState`、显式 incarnation、idle eviction、FIFO mutation gate 和有界
   execution mailbox；
3. 落地 `TurnPolicySnapshot` / `ModelInvocationSnapshot`；
4. 引入独立 `context/` 与 `context_manager/`，完成 budget/compaction；
5. 扩展现有 ToolScheduler：已完成 durable one-time approval、safe sandbox escalation、顺序
   Tool 与 UnknownOutcome 基线，后续增加并行计划、deadline、声明式 retry、reconciliation 与
   resource conflict；
6. 增加 `multi_agent/`、MultiAgentCoordinator、delegation protocol 与 context inheritance；
7. 完成 provider wire streaming、outbound writer 与 fault injection；
8. 通过 context continuity、多 Agent 隔离、取消和副作用恢复评测。

当前不创建独立 `zeta-agent` crate。只有出现至少两个真实执行宿主，并证明 Agent loop 不依赖
Thread projection、store、command receipt 或 App Server 时，才评审提取。

## 15. 固定决策

- Core 是 Agent 执行控制面，不是通用业务服务层；
- Session 是任务与 Thread 拓扑边界，不是共享 history/context；
- Thread 是 history、context、执行和恢复边界；
- 每个 loaded Thread 一个 ContextManager，不共享可变 context；
- ContextManager 是可重建派生协调状态，不是第二份 canonical history；
- ContextAssembler 是纯组装组件，不承担 context 生命周期；
- Agent delegation、fork lineage 与 context inheritance 是三种不同关系；
- child Agent 使用独立 Thread，父子只通过 durable seed/message/result 交流；
- 每个 Thread 的 context budget 与整个 Agent tree 的资源预算分开；
- 每个 Thread 一个逻辑写者，不要求常驻 actor；
- model、Tool、Agent I/O 永远在 durable commit 临界区之外；
- Tool 调度属于 Core，Tool 实现与 sandbox enforcement 不属于 Core；
- Core 通过 consumer-owned typed ports 编排服务；
- composition、Config、credential、transport 和 connection lifecycle 留在宿主层；
- Core 不把 `Runtime` 设计成组件、aggregate 或公共 API；
- transient delta 可丢失，final Item 和 terminal state 必须 durable；
- crash 后不静默重放 outcome 未知的副作用；
- model、provider、tool、policy 与 context 配置只在 safe point 进入新 snapshot。
