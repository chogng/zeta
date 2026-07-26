# `zeta-core` 架构与演进方案

> 物理位置：`zeta-rs/core/`  
> Rust crate：`zeta_core`  
> 当前状态：Session/Thread durable state、ContextAssembler、顺序 Tool loop、per-Thread
> execution mailbox 与 Core streaming port 已落地；provider wire streaming、outbound transport
> worker 与完整恢复语义尚未完成  
> Canonical contract：[`protocol.md`](protocol.md)  
> Agent 执行详细演进：[`zeta-agent-runtime-architecture.md`](zeta-agent-runtime-architecture.md)
> Config 与 Plugin/MCP/Skill snapshot 组合：[`config.md`](config.md)
> Provider credential：[`model-provider.md`](model-provider.md#6-provider-credential-与-subscription-backend)  
> ChatGPT/Codex subscription：[`codex-app-server.md`](codex-app-server.md)
> Secret persistence：[`secrets.md`](secrets.md)

## 1. 结论

把 Core 收敛为以下五项职责是合理的：

1. Agent 生命周期；
2. Turn 执行；
3. 上下文组装；
4. Tool 调度；
5. 服务编排。

但其中两个词必须采用窄定义，否则 Core 会重新变成无边界的“大脑层”：

- **Agent 生命周期**包括 Session/Thread/Turn 的权威状态迁移、durable commit、取消、等待、
  恢复和执行实例隔离；它不包括模型进程、浏览器、终端或 MCP server 自身的生命周期。
- **服务编排**是通过窄的 typed port 协调 model、tool、store、policy 和 capability，并规定调用
  顺序与 failure semantics；它不是 dependency injection container，也不拥有具体 provider、
  credential、sandbox、transport、JSON-RPC 或配置实现。

因此，`zeta-core` 的准确定位是：

> Zeta 的 Agent 执行控制面。它把一个已接受的产品意图可靠地推进为一组有序、可恢复的
> Agent/Turn 状态变化，但不亲自实现被编排的外部能力。

Core 还必须保留纯 Session/Thread reducer。Reducer 不是第六项独立职责，而是生命周期与恢复
正确性的纯状态转换内核。没有 reducer、commit ordering 和 recovery，所谓 Agent 生命周期只会
剩下进程内 task 管理，无法成为产品 authority。

当前不创建单独的 `zeta-agent` crate。Provider-independent Agent loop 先作为 `zeta-core` 的
私有 `turn` 模块落地；只有出现第二个真实消费者，并且能够证明该循环不依赖 durable commit、
Thread policy 或产品等待状态时，才提取为独立 crate。

长期方案保留“进程内执行状态”这一语义，但不建立名为 `Runtime` 的 Core 组件或公共 facade。
具体职责必须由 `SessionCoordinator`、`ThreadController`、`TurnExecutor` 等名称表达。

## 2. 当前仓库审计

当前 `zeta-core` 已经拥有：

- Session reducer、Session projection、SessionCoordinator 和 create/fork saga；
- Thread reducer、Thread projection、ThreadController 和 recovery；
- typed command receipt、replay 与 conflict detection；
- Session/Thread store port 的消费；
- Turn、Tool Call、Tool Result 和 interaction 的 durable lifecycle；
- writer lease port；
- canonical `ModelService`、顺序 ToolService loop、approval policy 和 browser capability port。

主要缺口与结构风险如下：

| 现状 | 风险 | 目标 |
| --- | --- | --- |
| `ThreadController` 同时负责 API、ID、projection、commit、recovery 和 tool record | 模块过大，职责难以独立验证 | 拆成 `thread/controller`、`thread/commit`、`thread/recovery` |
| durable projection 仍以全局短临界区提交 | 不应让长 I/O 占有该锁 | 已由 per-Thread bounded execution mailbox 承载 model/tool I/O；后续拆成真正 per-Thread projection state |
| `ModelService::stream` | 已支持 canonical incremental text/reasoning 与 cancellation；旧 adapter 退化为最终响应 delta | provider adapter 实现 wire-level SSE/stream decoder |
| `TurnExecutor` 已实现顺序 model → tool → model loop | 尚无 approval、并行调度和 unknown-outcome 恢复 | 增加 Tool scheduler 与完整副作用语义 |
| 所有 Core model invocation 已由 `ContextAssembler` 从 durable snapshot 派生 | 尚无 instructions、预算和 compaction | 增加 policy snapshot、limit 与 checkpoint |
| Browser contract 位于 Core 且包含具体能力形态 | Core public API 容易随宿主能力膨胀 | 通用 capability/tool port；具体 adapter 留在外层 |
| `session_coordinator.rs` 和 `thread_controller.rs` 已较大 | 新功能继续堆叠会固化巨型模块 | 新功能只进入新模块，相关测试随模块迁移 |

文档描述的是目标边界，不把当前同步 API 误称为完整 Agent 执行系统。

## 3. Core 拥有与不拥有

### 3.1 Core 拥有

- Session 结构生命周期：创建、Thread membership/lineage、归档、完成和恢复；
- 每个 Thread 的逻辑单写者、加载/空闲回收和执行实例隔离；
- Turn acceptance、执行、等待、取消、完成、失败和恢复；
- command idempotency、expected sequence、durable commit ordering 和 committed update 发布；
- Turn 固定 policy snapshot 与每次模型调用的 invocation snapshot；
- 从 durable history 派生 provider-neutral context；
- model → tool → model 循环及 sequential/parallel 调度；
- approval、user input、dynamic tool 和 capability wait 的领域协调；
- cancellation tree、deadline、迟到结果和旧 incarnation 拒绝；
- crash 后未完成模型/工具工作的恢复决策；
- 将外部服务错误映射成稳定的 Core execution outcome；
- 不含 secret 和敏感 payload 的执行诊断。

### 3.2 Core 不拥有

- canonical shared value 的定义；它属于 `zeta-protocol`；
- stored envelope、原子 append contract 和 event framing；分别属于 store port 与 storage；
- provider HTTP DTO、endpoint、stream decoder、retry header 和 credential；
- 模型目录发现、缓存、合并与筛选；它属于 `zeta-models-manager`；
- tool 的具体实现、MCP transport、child process、browser backend 或 terminal backend；
- sandbox enforcement、OS permission、approval UI 和 resource ownership；
- Config 文件 authority、secret persistence 或 credential refresh；
- JSON-RPC method、connection、subscription cursor 和客户端 owner selection；
- Desktop/CLI/TUI projection 与界面状态；
- provider prompt cache、模型驻留或网络 connection pool；
- 跨 Thread 长期 memory 的存储和检索策略。

### 3.3 三个容易混淆的边界

| 概念 | Core 的部分 | 外层的部分 |
| --- | --- | --- |
| Tool | 选择、排序、approval gate、取消、结果提交、失败决策 | schema adapter、执行、sandbox、MCP/进程 I/O |
| Model | invocation snapshot、调用时机、结果消费、safe point | provider 选择实现、HTTP、认证、wire codec |
| Service orchestration | 调用依赖 port、状态推进、补偿与降级 | 构造具体实现、配置加载、进程启动、网络监听 |

## 4. 依赖方向

目标依赖关系：

```text
                         zeta-protocol
                              ▲
                              │ canonical values
                 ┌────────────┼────────────┐
                 │            │            │
        zeta-session-store  zeta-core  zeta-thread-store
                 ▲            ▲            ▲
                 │            │ ports      │
                 │       ┌────┴─────┐      │
                 │       │          │      │
                 │ model service  tool service
                 │       │          │
                 └──── composition root ────┘
                           App Server
```

允许：

- `zeta-core → zeta-protocol`；
- `zeta-core → zeta-session-store`；
- `zeta-core → zeta-thread-store`；
- `zeta-core → zeta-async-utils`，用于 cancellation 等与领域无关的并发原语；
- 外层 adapter 实现 Core 定义的 consumer-owned ports。

禁止：

```text
zeta-core → zeta-app-server(-protocol)
zeta-core → zeta-storage / zeta-rollout
zeta-core → zeta-api / concrete model provider
zeta-core → built-in-tools / MCP implementation
zeta-core → config file / credentials implementation
zeta-core → Desktop / CLI / TUI
```

Core 可以消费由 composition root 从
[`AgentEnvironmentSnapshot`](config.md#11-runtime-snapshot-与-safe-point) 投影出的不可变窄值，
但不得读取配置文件、环境变量、Plugin/MCP/Skill live manager 或 credential store。具体 adapter
的构造仍由 App Server 或其他宿主 composition root 完成。

## 5. 核心执行组件

```text
SessionCoordinator
  │ membership / lineage / shared defaults
  ▼
ThreadController
  │ one logical writer / durable commit / recovery
  ▼
TurnExecutor
  │ context → model → tools → model → terminal outcome
  ├────────► ContextAssembler
  ├────────► ModelService port
  ├────────► ToolScheduler ─────► ToolService port
  └────────► InteractionCoordinator
```

### 5.1 SessionCoordinator

`SessionCoordinator` 只协调产品 Session 结构：

- 创建和恢复 Session；
- 创建、attach、fork、archive Thread；
- 维护 root/parent/child lineage；
- 提交 Session lifecycle 与 shared defaults；
- 为 Thread 创建解析默认设置的输入；
- 协调涉及 Session/Thread 两个 stream 的可恢复 saga。

它不持有 Thread transcript，不代理 token delta，也不等待模型或 Tool I/O。一个 Session 下的
多个 Thread 必须能够并行执行。

### 5.2 ThreadController

每个已加载 Thread 对应一个逻辑状态提交者。实现可以是 mailbox task，也可以是 keyed executor，
但必须满足：

- 同一 Thread 的结构性 mutation FIFO；
- 不同 Thread 可并行；
- model/tool I/O 不持有 projection lock 或 writer lease；
- interrupt、interaction response、tool completion 和 model completion 都回到同一提交序列；
- 空闲 controller 可回收，恢复时只依赖 durable history；
- 每次 load 生成新的 `ThreadIncarnationId`；
- completion 必须携带 operation ID 与 incarnation，迟到或重复结果不得提交。

`ThreadController` 是权威执行边界；“一个 Thread 一个常驻 task”只是可选实现，不是契约。
当前代码已经采用 `ThreadController` 名称；后续应按职责拆分，而不是继续扩张该文件。

当前实现以每个已执行 Thread 一个有界 FIFO mailbox 和后台 worker 落地：模型/Tool I/O 不占
projection lock，`turn/interrupt` 会取消该 Thread 的 active execution。mailbox/worker 仍是
Core-private；idle eviction、显式 incarnation 与所有结构性 command 进入同一 mailbox 是下一阶段。

### 5.3 TurnExecutor

`TurnExecutor` 负责一个 Turn 内的 provider-independent 循环：

```text
accept Turn
→ freeze TurnPolicySnapshot
→ assemble ModelInvocationSnapshot
→ invoke model
→ consume stream / final response
→ zero tool calls ───────────────► complete
→ one or more tool calls
   → validate and plan
   → request approval if needed
   → durable commit Tool Call
   → execute through ToolService
   → durable commit Tool Result
   → assemble next invocation
```

Executor 只能提出要提交的 typed effect，实际 durable append 由 ThreadController 串行完成。它
不能直接修改 projection、写 store 或发布 committed update。

第一版不需要设计通用 workflow engine。TurnExecutor 只实现 Agent Turn 已知状态机，避免引入
任意 DAG、动态 service registry 或字符串事件总线。

## 6. Agent 生命周期

必须区分三层生命周期：

```text
Product lifecycle
  Session / Thread / Turn 的 durable status

Loaded execution lifecycle
  Thread controller load / active / idle / closing + incarnation

Operation lifecycle
  一次 model invocation、tool execution 或 interaction delivery
```

三者不能共用一个 `status` 或 cancellation token。

### 6.1 `runtime` 一词的使用边界

Core 长期需要处理只在当前进程存活期间存在的执行状态，但这些状态是
`ThreadController` 的内部实现，不是新的领域对象：

```text
ThreadController
├─ durable Thread projection
├─ LoadedThreadState
├─ ThreadIncarnationId
├─ ThreadActivity
│  ├─ Idle
│  └─ Executing(TurnExecution)
└─ ThreadTaskSet / cancellation ownership
```

规则固定为：

- 不提供 `CoreRuntime`、`AgentRuntime`、`SessionRuntime` 或 `ThreadRuntime` 作为长期公共类型；
- 不创建与 Session/Thread 并列的 durable `Runtime` aggregate；
- loaded phase、incarnation、task handle、mailbox 和 cancellation token 保持 Core-private；
- restart 后丢失的执行状态只能由 durable Session/Thread facts 重建；
- 原 `SessionRuntime` 已直接收敛为 `SessionCoordinator`，没有保留兼容 alias；
- 只有确实表示一种执行环境时才使用 `runtime`，例如 Tokio runtime、Wasm runtime 或外部
  Tool runtime；这些环境的实现不属于 Core。

因此，“runtime”在 Core 架构中是对进程内执行状态的描述词，不是组件名、层名或职责名。

### 6.2 Loaded Thread phase

建议使用明确 enum：

```text
Unloaded
  → Loading
  → Idle
  → RunningTurn
      ├─ WaitingForInteraction
      ├─ Cancelling
      └─ Idle
  → Closing
  → Unloaded
```

Loaded Thread phase 是进程内协调状态，不进入 `zeta-protocol`，也不能替代 durable
`TurnStatus`。

### 6.3 Cancellation

取消层级：

```text
Core shutdown token
└─ Thread controller token
   └─ Turn token
      ├─ model invocation token
      ├─ tool execution token
      └─ interaction wait token
```

规则固定为：

- `turn/interrupt` 在 durable acceptance 后触发 Turn cancellation；
- cancellation 是协作式、best effort，不等于外部副作用一定没有发生；
- cancel signal 不得越过 durable terminal event 抢先对客户端宣称完成；
- 新 invocation 不得复用已经取消的 operation token；
- shutdown 有 deadline；超时后仍要留下可恢复的 durable 状态。

### 6.4 Safe point

下列位置是默认 safe point：

- 一个 model invocation 开始前；
- 一组 Tool Results 全部 durable commit 后；
- interaction resolved 后恢复执行前；
- Turn 尚未开始或已经 terminal；
- compaction 完成并验证 provenance 后。

模型、provider、tool availability 和非安全策略配置只在 safe point 创建新快照。安全策略可以在
执行中单调收紧，不能静默放宽。

## 7. Durable commit 与发布顺序

所有权威 mutation 固定使用：

```text
receive intent/completion
→ validate against current projection and operation identity
→ build typed event batch
→ reduce into candidate projection
→ append atomic batch
→ install committed projection
→ publish committed update
→ schedule next external operation
```

必须保持：

- append 失败时不安装 candidate projection；
- notification 永远不能早于 durable append；
- live commit 与 recovery 调用同一 reducer；
- command receipt 与首个业务 event 原子提交；
- replay 先匹配 command receipt，再返回原 response sequence；
- model/tool completion 自身不是 command receipt，不得冒用用户 `CommandId`；
- transient delta 可以丢失，final Item 必须 durable 后才对外可见。

Core 依赖 storage-neutral store trait，不知道 JSONL、SQLite、checksum、文件路径或 fsync 实现。

## 8. Turn 执行模型

### 8.1 两类不可变快照

```rust
pub struct TurnPolicySnapshot {
    // approval, sandbox intent, working directory, capability scope,
    // execution limits and policy revision
}

pub struct ModelInvocationSnapshot {
    // resolved model, context, tools, reasoning/output settings,
    // provider/config/catalog revisions and invocation identity
}
```

`TurnPolicySnapshot` 生命周期覆盖整个 Turn。`ModelInvocationSnapshot` 只覆盖一次 provider
request。不要用 `bool` 或语义不明的 `Option` 表示 policy；使用 enum、newtype 或具名 variant。

### 8.2 Model port

Core 定义自己消费的 provider-independent port。目标形态需要：

- async；
- 可通过 cancellation context 取消；
- 输入为 canonical request/snapshot，不是 prompt `String`；
- 输出为 typed stream event 或 typed final response；
- 区分 refusal、context overflow、rate limit、authentication、transport 和 invalid response；
- 保留 provider usage/response reference 作为受控 metadata，不泄漏 wire DTO；
- 不在 trait 内选择全局默认模型。

模型解析由 models manager/provider layer 完成，Core 只在 invocation safe point 接收
`ResolvedModel`。刷新 catalog 不改变 in-flight invocation，也不能在后台静默替换模型。

### 8.3 Streaming

当前 `ModelService::stream` 已向 Core 暴露 canonical `TextDelta` 与 `ReasoningDelta`。TurnExecutor
为每次 invocation 分配 stream instance 与 item ID，发布 transient `ItemStarted`/`ItemDelta`，并在
最终 durable Item 中复用同一 ID。没有 wire stream 能力的 adapter 使用默认 bridge，在完整响应返回
后发出一个或多个 delta；通用 SSE framing 应实现于 `zeta-client`，Provider event decoder
实现于 `zeta-api`，两者由 model-provider runtime 组合。

Transient token/reasoning delta：

- 可以在 model operation task 中产生；
- 必须携带 stream incarnation/cursor；
- 通过有界 channel 回到 Core update publisher；
- channel 饱和时允许 coalesce 或丢弃非关键 delta；
- 不能阻塞 durable completion；
- 不能作为恢复输入。

## 9. 上下文组装

`ContextAssembler` 是 Core 内部的纯派生组件：

```text
ContextInput
  = durable Thread snapshot
  + TurnPolicySnapshot
  + resolved model limits/capabilities
  + system/developer instruction snapshot
  + available tool definitions
  + optional verified compaction checkpoints

ContextInput → ContextPlan | ContextError
```

当前基础实现已统一重建 user/agent message、Tool Call 与 Tool Result，并拒绝损坏的参数或悬空
Tool Result。下面列出的是完整目标契约；instruction precedence、模型预算与 compaction checkpoint
仍未实现。

它负责：

- 选择当前 Turn 输入和相关历史 Item；
- 保持 Tool Call/Tool Result 配对与引用完整；
- 应用 instruction precedence；
- 应用模型 context/output limit；
- 选择经过验证的 compaction checkpoint；
- 输出 provider-neutral messages、tools 和预算诊断；
- 记录被省略范围及原因，便于测试和诊断。

它不负责：

- 读取 provider cache 或 provider response history；
- 修改 durable Thread history；
- 直接调用 summarization model；
- 检索未经授权的跨 Thread memory；
- 根据 model ID 字符串猜 capability；
- 把模型声称的文件或环境变化当作系统事实。

Compaction 由 Core 在安全点编排，但 summary 生成可通过独立 port 完成。Checkpoint 必须带 source
sequence range、Item/Event references、digest、schema/policy version 和生成时间；原始 event log
永不因 compaction 删除。

## 10. Tool 调度

### 10.1 职责拆分

`ToolScheduler` 拥有：

- Tool definition/name/call ID 校验；
- availability 与 capability scope 检查；
- sequential/parallel plan；
- approval 和 policy gate；
- execution deadline、cancellation 与 operation correlation；
- result ordering；
- Tool failure 对 Agent loop 的影响；
- unknown outcome 与 retry 决策。

`ToolService` adapter 拥有：

- 参数到具体实现的转换；
- MCP、process、browser、terminal 或 remote I/O；
- sandbox/OS enforcement；
- stdout/stderr/resource 的有界采集；
- tool-specific reconciliation；
- 返回 provider-independent typed result。

Tool adapter 永远不能直接写 Thread store 或 projection。

当前 `TurnExecutor` 已实现最小顺序路径：先 durable commit 全部 Tool Call，再逐个调用
`ToolService`、提交 Tool Result，并从最新 snapshot 组装下一次模型请求。独立 scheduler、
approval、并行执行、deadline、retry 与 `UnknownOutcome` 仍是后续工作。

### 10.2 Durable boundary

```text
model emits call
→ validate
→ evaluate approval
→ durable Tool Call / waiting fact
→ execute outside Thread writer
→ durable Tool Result or UnknownOutcome
→ publish
→ next model invocation
```

对于有副作用的 Tool，执行前必须已经有稳定 `ToolCallId` 和 durable intent。Crash 后看到 intent
但没有 result 时，Core 不得默认重放。

### 10.3 Parallel tools

只有同时满足下列条件才允许并行：

- 模型与 Turn policy 允许；
- 每个 Tool definition 声明可以并行；
- call 之间没有已知 resource conflict；
- approval 已分别解决；
- result aggregation 有确定顺序和有界并发。

并行完成顺序不决定 durable transcript 顺序。默认按模型产生的 call order 提交一个完整 result
batch；若未来允许逐个提交，必须在 protocol 中明确可观察顺序。

### 10.4 Retry 与 unknown outcome

Retry policy 必须是自描述 enum，例如：

```text
Never
SafeRead
IdempotentWrite(operation key)
ReconcileBeforeRetry
```

Core 负责执行已声明 policy，Tool adapter 负责提供 idempotency/reconciliation 能力。参数错误、
policy denial、unknown outcome 和未声明幂等性的写操作默认不自动重试。

## 11. 服务编排

Core 中的“服务”只是有领域意义的依赖端口，不建立泛化 `Service` trait 或字符串 registry。
建议最小端口集合：

| Port | Core 为什么消费 | 实现位于 |
| --- | --- | --- |
| `SessionStore` | Session durable load/append | storage adapter |
| `ThreadStore` | Thread durable load/append | storage adapter |
| `WriterLease<Id>` | aggregate 单写者 | storage/host |
| `ModelService` | 执行 canonical model invocation | composition adapter over model-provider |
| `ToolService` | 执行已物化 Tool call | composition adapter over built-in/MCP host |
| `PolicyService` | 对完整 action 做 approval/sandbox 决策 | host policy layer |
| `CapabilityBroker` | 解析一次 Turn 可使用的宿主能力 | App Server/host |
| `Clock` | deadline 与可测试时间 | host adapter |
| `IdGenerator` | 稳定 operation/item identity | host adapter |

端口遵循 consumer-owned interface：Core 只定义自己真正需要的最小操作。新 trait 必须有 doc
comment，说明角色、实现不变量、取消和错误约束。

Composition root 负责：

- 读取各领域 authority，并组合一致的 Config、Plugin、MCP、Skill 与 policy snapshot；
- 通过 model-provider 的 direct credential binding 或已注入 subscription backend 构造
  `ModelService`；
- 构造 provider、tool、store 和 policy adapter；
- 将 adapter 注入 Core；
- 启动 App Server transport；
- 选择进程级 shutdown deadline。

这些行为不得进入 `zeta-core::CoreBuilder` 背后成为隐藏全局状态。Core 可以提供显式
`CoreServices` 构造参数，但不能 service-locator 式地运行时按字符串取依赖。

ChatGPT login、OAuth callback、token refresh、revoke 和 secret store 都不进入 Core。Core 只会从
`ModelService` 观察到稳定的 authentication/reauthentication execution outcome；它不能读取
`~/.codex/auth.json`、`zeta-secrets` 或任何 access token。

完整的 Config commit → manager reconcile → `AgentEnvironmentSnapshot` → Core safe point 流程
由 [`config.md`](config.md#6-pluginmcp-与-skill-接入流程) 统一规定。Core 不观察各 manager 的
中间状态，也不因 runtime health 变化反向修改 desired config。

## 12. Interaction 与 capability

Approval、structured user input、dynamic tool 和 capability 请求共享同一种协调模式：

```text
Core commits InteractionRequested
→ App Server selects connection owner and delivers
→ client resolves exact RequestId
→ Core validates Turn + RequestId + expected sequence
→ Core commits InteractionResolved/Cancelled
→ TurnExecutor continues at a safe point
```

Core 拥有 durable request/wait/resolve/cancel；App Server 拥有 connection owner、投递、disconnect
和 outbound queue。普通 Thread read 只返回 pending state，不返回待投递的完整 request payload。

Browser、terminal 等能力不应逐个扩张 Core 顶层 public API。优先把它们建模为 typed tool 或由
窄 `CapabilityBroker` 解析的 Turn-scoped handle；具体 target ownership 和 backend lifecycle
留在宿主层。

## 13. 错误、恢复与降级

### 13.1 错误分层

Core 错误至少区分：

- validation/invalid transition；
- optimistic concurrency/command conflict；
- aggregate not found；
- store/lease failure；
- model capability/config failure；
- model transient/permanent failure；
- Tool unavailable/policy denied/execution failure/unknown outcome；
- interaction timeout/cancel/owner unavailable；
- execution overloaded/closing/stale completion；
- internal invariant violation。

不要把这些重新压成 `Model(String)`、`Journal(String)` 或通用 `Failed(String)`。内部 error 可以保留
source chain，向 protocol/App Server 的映射只暴露稳定、安全的 code 与 message。

### 13.2 恢复规则

恢复只依赖 durable facts：

- terminal Turn 保持 terminal；
- waiting interaction 恢复为 waiting，由 App Server 重新建立可投递状态；
- in-flight model request 默认不存在，按明确 policy 从最后 safe point 重新创建；
- 已 durable Tool Call 但没有结果的副作用调用进入 `UnknownOutcome` 或 reconciliation；
- `Cancelling` Turn 最终收敛到 `Interrupted`，不能永久停留；
- Thread incarnation 更新，旧 task completion 全部拒绝；
- Session create/fork saga 从 planned state 继续或补偿。

模型调用可以重试的前提是没有外部 Tool 副作用被遗漏，并且不会重复提交已存在 Item。

### 13.3 Backpressure

以下队列必须有界：

- per-Thread mailbox；
- model delta channel；
- Tool parallel execution set；
- committed/transient update publisher；
- App Server outbound queue。

Core 对 mailbox saturation 返回稳定 retryable error。Transient delta 可以合并或丢弃；durable
event、terminal outcome 和 interaction request 不能无声丢弃。

## 14. 目标目录与公开 API

目标结构：

```text
zeta-rs/core/src/
├─ lib.rs
├─ error.rs
├─ services.rs
├─ session/
│  ├─ mod.rs
│  ├─ coordinator.rs
│  ├─ reducer.rs
│  ├─ projection.rs
│  ├─ saga.rs
│  └─ *_tests.rs
├─ thread/
│  ├─ mod.rs
│  ├─ controller.rs
│  ├─ loaded.rs
│  ├─ reducer.rs
│  ├─ projection.rs
│  ├─ commit.rs
│  ├─ recovery.rs
│  └─ *_tests.rs
├─ turn/
│  ├─ mod.rs
│  ├─ executor.rs
│  ├─ lifecycle.rs
│  ├─ invocation.rs
│  ├─ interaction.rs
│  └─ *_tests.rs
├─ context/
│  ├─ mod.rs
│  ├─ assembler.rs
│  ├─ budget.rs
│  ├─ compaction.rs
│  └─ *_tests.rs
└─ tool/
   ├─ mod.rs
   ├─ scheduler.rs
   ├─ policy.rs
   ├─ outcome.rs
   └─ *_tests.rs
```

约束：

- 模块默认 private，只从 `lib.rs` named export 稳定 API；
- implementation module 目标低于 500 LoC；
- 超过约 800 LoC 的现有文件不再增加新功能；
- 新测试模块使用 sibling `*_tests.rs` 和显式 `#[path = "..._tests.rs"]`；
- 不创建 `common.rs`、`utils.rs`、`services/registry.rs` 或巨型 `controller.rs`；
- public trait 必须有职责和实现约束 doc comment；
- public request 不使用含义不明的 `bool` 或 `Option` 参数；
- public Core API 不使用 `*Runtime` 命名；
- Core-private loaded phase、mailbox message、task handle 和 incarnation 不 re-export。

建议公开面只包含：

- `SessionCoordinator` / `ThreadController` handle；
- typed command request/result；
- read-only projection/snapshot；
- consumer-owned service ports；
- 稳定、可映射的 Core error；
- subscription/update source 的最小接口。

Reducer 可以为 store recovery/test 显式导出，但 projection 内部 command receipt 不进入公共
wire model。

## 15. 测试与验证

### 15.1 Pure tests

- Session/Thread reducer 对每个合法与非法 transition；
- context assembly 的 instruction precedence、Tool 配对和预算；
- Tool schedule 的顺序、并行条件与 deterministic aggregation；
- error classification；
- snapshot safe-point semantics。

### 15.2 并发与生命周期测试

- 同 Thread FIFO、不同 Thread 并发；
- model/tool I/O 期间 writer lease 已释放；
- interrupt 与 model completion race；
- duplicate/late/stale-incarnation completion；
- acceptance append 失败、completion append 失败和 publish 失败；
- waiting interaction resolve/timeout/cancel/disconnect；
- mailbox saturation 与 transient delta coalescing；
- shutdown deadline；
- controller 空闲回收后恢复。

### 15.3 Fault-injection tests

在每个 durable boundary 前后注入 crash：

- Turn accepted / started；
- Tool Call committed / execution started / result committed；
- interaction requested / delivered / resolved；
- Session Thread planned / child created / attached；
- compaction generated / checkpoint committed。

验收重点不是“没有报错”，而是恢复后不会重复副作用、丢失 final Item、留下永久 Running，或
接受旧执行实例的迟到结果。

最小验证：

```bash
cargo fmt --manifest-path zeta-rs/Cargo.toml --all -- --check
cargo clippy --manifest-path zeta-rs/Cargo.toml -p zeta-core --all-targets -- -D warnings
cargo test --manifest-path zeta-rs/Cargo.toml -p zeta-core
```

涉及 protocol/store contract 时还必须运行对应 crate tests；涉及 App Server update 顺序时必须
运行 App Server 和 client tests。

## 16. 分阶段落地

### Phase C0：固定边界（完成）

- 以本文作为 `zeta-core` ownership 的权威来源；
- 新代码已停止增长旧 `thread_manager.rs` 和 `session_runtime.rs`；
- 同步文本 `AgentModel` 已替换为 canonical `ModelService`；
- 用测试固定当前 reducer、receipt 和 recovery 行为。

### Phase C1：按 aggregate 拆模块

- 将 Session reducer/coordinator/saga 拆入 `session/`；
- 将 Thread reducer/projection/commit/recovery 拆入 `thread/`；
- 迁移相关 sibling tests；
- 保持公开 API 由 `lib.rs` named export，不增加兼容 module alias。

### Phase C2：异步 ThreadController（基础完成）

- 已引入 Core-private per-Thread bounded execution mailbox；
- `turn/start` durable accepted 后立即返回；
- model/tool I/O 不占 projection lock；
- interrupt 取消 active execution，不同 Thread 可由独立 mailbox 并行执行；
- 待为全部结构性 command 引入同一 FIFO mailbox、idle eviction 与 explicit incarnation。

### Phase C3：TurnExecutor 与 model port（基础完成）

- 已在 Core 私有 `turn/` 中实现 deterministic model/tool vertical slice；
- 已使用 canonical `ModelService::stream`、transient stream cursor/item delta 和 cancellation safe point；
- 已由 App Server composition adapter 接入 model-provider，provider 不反向依赖 Core；
- 待由 model-provider 实现 wire streaming，并补齐 invocation policy snapshot；
- 不创建独立 `zeta-agent` crate。

### Phase C4：Context 与 Tool loop（基础完成）

- 已让所有 Core model invocation 统一经过 ContextAssembler；
- 已实现 durable Tool Call 后顺序执行并 durable commit Tool Result；
- 待接通 approval/capability interaction；
- 待完成 context budget/compaction、parallel policy、unknown outcome 和 retry contract。

### Phase C5：恢复与生产验证

- fault injection 覆盖所有 durable boundary；
- idle eviction、shutdown、backpressure 和 stale completion 完整；
- compaction checkpoint 与 provider change 在 safe point 生效；
- 以并发、取消、工具副作用和 context continuity 评测作为发布门。

### Phase C6：可选 crate 提取

只有满足以下条件才评审提取 `zeta-agent`：

- 至少两个真实执行宿主需要同一个 loop；
- loop 的输入输出已经稳定为 provider-independent typed contract；
- loop 不依赖 store、Thread projection、command receipt 或 App Server；
- 提取能减少依赖或重复，而不是只增加 facade 和 DTO 转换。

## 17. 固定决策

- Core 是 Agent 执行控制面，不是通用业务服务层；
- 五项职责成立，但 durable state/recovery 属于 Agent lifecycle 的不可删除部分；
- Session 只串行结构操作，Thread 是独立执行与恢复边界；
- 每个 Thread 一个逻辑写者，不要求常驻 actor；
- model/tool I/O 永远在状态提交临界区之外；
- context 是 durable history 的纯派生物，不建立第二个 canonical history；
- Tool 调度属于 Core，Tool 实现与 sandbox enforcement 不属于 Core；
- Core 通过 consumer-owned typed ports 编排服务，不依赖具体 adapter；
- composition、Config、credential、transport 和 connection lifecycle 留在宿主层；
- Core 保留进程内执行状态，但不把 `Runtime` 设计成组件、aggregate 或公共 API；
- 当前 Agent loop 先留在 Core 私有模块，不提前创建 `zeta-agent` crate；
- transient delta 可丢失，final Item 和 terminal state 必须先 durable；
- crash 后不静默重放 outcome 未知的副作用；
- provider/model/tool 配置只在明确 safe point 进入新快照；
- reducer、loaded execution phase、operation task 和 wire update 是不同层的概念，不混用。
