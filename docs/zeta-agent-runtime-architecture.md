# Zeta Agent 执行架构与演进方案

> 状态：Proposed  
> 审查基线：`817e604af3a179d5ff70d14f4ed403a0f26cd47c` 加当前工作区改动  
> 最后审查：2026-07-25  
> 适用范围：Session、Thread 执行控制、Agent loop、工具执行、上下文恢复、Provider
> 切换和多 Agent 演进

本文收敛此前分散的 Session、ThreadActor、Canonical History、Provider Handoff、Tool
Execution 和 Memory 设计稿。Session-first 领域与存储基础已落地；本文只描述尚未完成的
异步 Agent 执行演进，不覆盖当前
[`zeta-app-server-api.md`](zeta-app-server-api.md) 契约，也不重复
[`zeta-protocol` 架构](protocol.md)。Core 的 ownership 与 crate 内部分层以
[`core.md`](core.md) 为准；Context/ContextManager 以
[`core-context.md`](core-context.md) 为准，多 Agent 以
[`core-multi-agent.md`](core-multi-agent.md) 为准。本文只保留跨 Core、App Server、provider
和 tool 的演进视角。

## 1. 结论

Canonical 产品层级由 [`protocol.md`](protocol.md) 定义。SessionStore、Session reducer 与
SessionCoordinator 已作为产品根 aggregate 落地。

采纳以下方向：

- 保持 Rust 拥有 Session、Thread、Turn、ThreadItem 和工具状态的权威事实；
- Session 持有任务生命周期、Thread membership/lineage 和共享默认设置；每个 Thread
  继续作为独立顺序、持久化、恢复和并发边界；
- Core 只消费 [`zeta-protocol` 定义的共享语义](protocol.md)，actor、策略、channel、
  IO 与 reducer 执行属于 Core；
- 将 provider-independent Agent loop 与 Thread durable commit 分层，第一阶段作为 Core 私有
  模块实现，只有出现第二个真实消费者后才提取 crate；
- 以每 Session 逻辑单写者串行 membership/settings 变更，但不让 SessionCoordinator 转发 token
  delta 或串行子 Thread 的模型/工具执行；
- 以每 Thread 逻辑单写者保证顺序，不要求每个持久化 Thread 常驻一个 actor task；
- 模型和工具在单写者之外执行，结果返回后先持久化，再更新投影和通知；
- 每次模型调用从 durable Thread history 构造 provider-neutral context；
- Provider 切换只影响下一个安全点创建的模型调用快照；
- App Server 的下一个协议版本采用“Turn durable accepted 后返回，执行异步继续”的语义。

不采纳以下抽象：

- 与 rollout 并列的 `CanonicalHistory` store；
- 常驻且持久化 provider websocket/cache 的 `ProviderLaneRegistry`；
- 把普通 summary 固化为 Provider 切换必需的 `ProviderHandoff` 协议；
- 未定义授权、删除、隔离和评测机制的跨 Thread 长期记忆。

## 2. 当前事实与缺口

当前工作区已经提供了后续演进需要的大部分地基，但这些改动仍应按工作区状态验证，不能用
“PR 已完成”代替实际 Git 历史和测试结果。

| 能力 | 当前事实 | 结论 |
|---|---|---|
| Canonical protocol | 当前完成度由 [`protocol.md`](protocol.md) 维护 | Core 只消费，不在本文重复设计 |
| Session | canonical model、SessionStore、pure reducer、SessionCoordinator 与 create/fork saga 已实现 | 保持为产品根 aggregate |
| 权威历史 | per-Thread typed event log、atomic batch、pure reducer、durable Items | 继续使用，不新建 Canonical History |
| 状态投影 | `ThreadSnapshot` 可由 rollout 重建 | 作为读取模型，不是第二份权威状态 |
| 模型调用 | `ModelService::stream` 产生 canonical text/reasoning delta，并接受 cancellation token | provider adapter 仍需实现 wire-level SSE decoder |
| Turn 执行 | `turn/start` durable accepted 后投递 Core `TurnExecutor` mailbox | App Server stdio 仍缺独立 outbound writer，实时通知由下一次 transport poll 取走 |
| 并发 | 每个已执行 Thread 有有界 execution mailbox；durable projection 使用短全局临界区 | 全部结构性 command 尚未进入同一 mailbox，尚无 idle eviction/incarnation |
| 工具执行 | 已有顺序模型—工具循环，Tool Call/Tool Result 均先后 durable | 尚无 approval、并行调度和 unknown-outcome 恢复语义 |
| 取消 | `zeta-async-utils` 已有 cancellation tree | 可以复用到 provider、tool 和 child process |
| Provider 配置 | 每次模型调用读取最新配置快照 | 已具备安全切换基础，不需要先做持久 Provider Lane |
| App Server | Rust server 仍是同步 read-dispatch-write | 需要 processor、outbound writer 和 keyed queue |
| Desktop | transport、peer、session、supervisor 已拆分 | 后续补 projection/resync，不把权威状态移入 Renderer |

直接相关的当前实现：

- [`core/src/thread_controller.rs`](../zeta-rs/core/src/thread_controller.rs)
- [`core/src/thread_reducer.rs`](../zeta-rs/core/src/thread_reducer.rs)
- [`thread-store/src/store.rs`](../zeta-rs/thread-store/src/store.rs)
- [`async-utils/src/lib.rs`](../zeta-rs/async-utils/src/lib.rs)
- [`app-server/src/server.rs`](../zeta-rs/app-server/src/server.rs)

## 3. 可行性评估

| 设计项 | 判断 | 风险 | 处理方式 |
|---|---|---|---|
| Agent loop 与产品 Harness 分层 | 采纳 | 中 | 先在 Core 内分模块，用 fake provider 做单 Turn vertical slice；满足提取条件后再拆 crate |
| 每 Thread 逻辑单写者 | 采纳 | 中高 | keyed mailbox/executor；长 I/O 不占有状态锁 |
| 异步 `turn/start` | 采纳，开发期直接修改当前契约 | 高 | 以 contract test 固定 acceptance、通知和取消顺序 |
| durable context + compaction | 采纳 | 中 | 原始 event log 保留；summary 只作带 provenance 的派生 checkpoint |
| Provider 切换 | 采纳简化版 | 中 | 下一个 model invocation 重新构造 context 和 invocation snapshot |
| Provider Lane/Handoff schema | 暂缓 | 中高 | 先用统一 ContextManager/ContextPlan；只有评测证明不足时再引入 |
| Tool loop | 采纳 | 高 | 副作用前持久化、明确 approval、unknown outcome 和 per-tool retry |
| Session aggregate | 采纳，分阶段 | 高 | Session 与 Thread 独立 sequence；跨 stream 创建使用可恢复 saga 或明确的原子事务 |
| 长期 Memory | 单独 RFC | 高 | 必须先定义 consent、scope、删除、保留期、隐私和质量评测 |

因此，Session 是目标模型的一部分，但不能与 Thread actor、History、Lane、Handoff 和 Memory
一次性上线。先固定 canonical contract 和 per-Thread durability，再实现 SessionStore、
SessionCoordinator 与 fork saga。

## 4. 术语与所有权

### 4.1 产品领域

产品实体、command/event/update、ID、sequence 和 cursor 的语义以
[`protocol.md`](protocol.md) 为准。本执行方案只追加三个约束：

- Session 与 Thread 分别保持逻辑单写者；
- 模型和工具 I/O 不占用 aggregate 状态提交临界区；
- snapshot、SQLite 和 Renderer state 都是可重建投影，不是第二份 authority。

### 4.2 Session 的三种语义

代码和文档必须显式区分：

- `Session`：产品级根 aggregate，拥有 Thread membership/lineage；
- `AppServerConnection`：一条 RPC connection 的初始化、pending request、subscription 和
  resource owner；现有 Desktop `AppServerSession` 属于此类，后续命名不得反向定义产品层；
- `BrowserSession` / `TerminalSession`：具体 capability 的生命周期。

Session 不嵌入所有 Thread transcript，也不保存共享可变 `SessionHistory`。它只持有
Thread reference、父子关系、任务生命周期和共享默认设置。Thread 创建同时涉及 Session 与
Thread 两个 stream，存储层必须提供 multi-stream 原子事务，或使用
`ThreadCreationRequested → ThreadCreated → ThreadAttached` 的可恢复 saga；不能只增加一个
`session_id` 字段而不定义 crash consistency。

### 4.3 ThreadActor 的准确含义

Actor 是实现策略，不是新的 durable aggregate。目标组件统一称为 `ThreadController`：

```text
一个已加载 Thread
    → 一个逻辑命令序列
    → 一个状态提交者
    → 零到多个外部异步任务
```

实现可以使用 mailbox task，也可以使用 keyed executor 加短临界区。必须满足：

- 同一 Thread 的结构性提交 FIFO；
- 不同 Thread 可并发；
- provider/tool I/O 不持有 Thread 状态锁；
- interrupt、approval response 和 completion 都回到同一命令序列；
- 空闲 controller 可回收，恢复时由 rollout 重建。

## 5. 目标架构

```text
CLI / Desktop / future daemon
             │
             ▼
Versioned App Server
  connection gate / dispatcher / subscriptions / resync
             │
             ▼
SessionCoordinator (zeta-core)
  membership / lineage / defaults / Session durable commit
             │ ThreadHandle registry
             ▼
ThreadController (zeta-core, one logical writer per loaded Thread)
  policy snapshot / durable commit / recovery / product decisions
             │
             ▼
TurnExecutor (zeta-core private module)
  context → model → tool calls → tool results → next model request
       │                              │
       ▼                              ▼
   ModelPort                     AgentTool
       │                              │
model-provider              shell-command / file-system / file-search / apply-patch / MCP adapters

SessionCoordinator ── append ──► SessionStore
ThreadController  ─── append ──► ThreadStore ──► rollout
      │ committed events and transient deltas
      └─────────────────────────► subscription hub
```

依赖方向：

```text
zeta-protocol
   ▲        ▲
   │        └──────── zeta-thread-store ◄──────── zeta-storage
   │
zeta-core ──────────► SessionStore + ThreadStore
   ▲
   │ Core-owned ports
App Server adapters ─────► model-provider / shell-command / file-system / file-search / apply-patch / MCP
   │
   └─────────────────────► config / credentials / rollout
```

禁止的依赖：

- `zeta-core` 不依赖 JSON-RPC DTO 或 provider HTTP wire 类型；
- `zeta-core` 不依赖 concrete model provider、Tool adapter、storage 或 rollout；
- composition adapter 可以同时依赖 Core port 与具体 service，不能把该依赖反向放进 Core；
- `app-server-protocol` 只复用经过审核的 canonical public view；Core-private aggregate、
  command receipt 和 pending request state 永不进入 wire；
- Tool adapter 不直接修改 Thread projection。

### 5.1 Provider 配置与运行时边界

Provider 配置拆成单向依赖的两层：

```text
zeta-protocol
      ▲
      │
zeta-model-provider-config
  declarations / schema / validation / defaults / normalization / registry merge
      ▲
      │
zeta-model-provider ─────► zeta-api / zeta-client / zeta-http-client
  provider execution / resolved endpoint / provider-specific adapter / execution errors
```

边界判定规则：

- 能被持久化、合并或生成 schema 的 provider 信息属于 `model-provider-config`；
- 依赖当前进程、网络、client、transport 或 secret 的行为属于 `model-provider`；
- endpoint 默认值和归一化规则是声明，归一化后的 endpoint 是一次 invocation snapshot；
- adapter ID 是声明，具体 `zeta-api` endpoint/profile binding 和固定请求头是运行时实现；
- 静态配置错误在实例化前返回 `ProviderConfigError`，连接和协议错误返回
  `ModelProviderError`；
- 当前阶段不声明认证字段；后续只允许配置层保存 credential reference/policy，secret
  读取、刷新和请求头生成必须留在运行时层。

## 6. 目标目录结构

Core 内部目标目录不在本文维护，以
[`core.md`](core.md#13-目标目录与公开-api) 为唯一来源。跨 crate 的长期分层为：

```text
zeta/
├─ docs/
│  ├─ core.md
│  ├─ core-context.md
│  ├─ core-multi-agent.md
│  ├─ zeta-agent-runtime-architecture.md
├─ zeta-rs/
│  ├─ protocol/              canonical values and contracts
│  ├─ core/                  execution control plane
│  ├─ session-store/
│  ├─ thread-store/
│  ├─ storage/
│  ├─ model-provider/
│  ├─ shell-command/
│  ├─ file-system/
│  ├─ file-search/
│  ├─ apply-patch/
│  ├─ app-server-protocol/
│  ├─ app-server-transport/
│  └─ app-server/            composition root and transport coordination
└─ desktop/                  client projection and interaction UI
```

本执行文档不再复制各 crate 内部文件树；具体目录由各 crate 架构文档维护。

## 7. TurnExecutor、SessionCoordinator 与 ThreadController 边界

### 7.1 TurnExecutor

Core 私有 `turn` 模块负责一个 Turn 内与模型、工具相关的 provider-independent 循环：

- 构造和消费 provider-neutral message context；
- 调用 `ModelPort`；
- 解析模型 lifecycle 和 Tool Call；
- 执行 sequential/parallel tool policy；
- 在安全点消费 steer/follow-up；
- 传播 cancellation；
- 产生 typed lifecycle event。

它不决定哪些事件已经 durable，也不直接发送 JSON-RPC notification。

端口应是异步、可取消且可作为 trait object 使用。公开 trait 必须写清实现约束，避免
`foo(false)`、`bar(None)` 一类含义不明的调用形态。

### 7.2 SessionCoordinator

`zeta-core` 的 `SessionCoordinator` 只负责 Session 结构语义。当前实现类型仍名为
`SessionCoordinator`：

- 创建、attach、fork、archive Thread；
- 维护 root/parent/child lineage；
- durable commit Session settings 和生命周期；
- 解析 Session defaults，但不修改已开始 Turn 的 policy snapshot；
- 持有可回收的 Thread handle registry。

它不代理 Thread token delta，不持有 Thread transcript，也不等待模型或工具 I/O。不同 Thread
仍可并行执行。

### 7.3 ThreadController

`zeta-core` 的 `ThreadController` 负责产品语义：

- durable accept Turn；
- 创建整个 Turn 固定的 `TurnPolicySnapshot`；
- 在每次模型调用前创建 `ModelInvocationSnapshot`；
- 把 Agent lifecycle 映射为 durable Thread events；
- 管理 approval、user input 和 capability 等待；
- 决定 Tool failure 是继续循环还是终止 Turn；
- 在 commit 后发布领域事件；
- 恢复未完成 Turn；
- 在空闲安全点执行 compaction、fork preparation 和 provider change。

### 7.4 不可变快照

一个产品 Turn 可能包含多次模型调用，因此至少需要两个快照：

```text
TurnPolicySnapshot
    approval / sandbox / cwd / capability owner
    生命周期：整个 Turn

ModelInvocationSnapshot
    model / system prompt / messages / tools / reasoning settings
    生命周期：一次 provider request
```

Session/Thread 配置更新只影响下一个安全点创建的快照。安全策略不能在 Turn 中途静默
放宽；需要变更时使用显式 interrupt/restart 或仅允许单调收紧。

## 8. Session、Thread 状态与并发

Sequence 与 transient cursor 的领域定义见
[`protocol.md`](protocol.md#5-sequencecursor-与-id)。Core 必须据此为 Session 与各 Thread
建立独立调度/提交 scope；禁止让一个 Session 下的 Thread 共享执行队列，否则多 Agent 会
退化成全局串行。

建议 phase：

```text
Idle
├─ start turn ───────────────► Turn
├─ compact ─────────────────► Compacting
├─ prepare fork ────────────► PreparingFork
└─ unload ──────────────────► Closing

Turn
├─ wait approval/input/capability
├─ cancel ──────────────────► Cancelling ─► Idle
├─ complete ──────────────────────────────► Idle
└─ fail ──────────────────────────────────► Idle
```

关键点：

- phase 只表示进程内协调状态；durable Turn status 仍由 typed event 定义；
- `turn/start` 的 acceptance batch 提交后即可响应；
- 模型和工具 task 通过 Core-owned cancellation token 和 task ID 回传结果；
- 迟到、重复或来自旧 Thread incarnation 的 completion 必须拒绝；
- mailbox/backlog 有界，满时返回稳定 retryable error；
- keyed queue 只串行短的验证和提交阶段，不等待完整模型调用。

## 9. History、Context 与 Provider 切换

Context 的 authority、ContextManager、ContextPlan、budget、compaction 和多 Agent isolation
统一由 [`core-context.md`](core-context.md) 定义。本文只规定 provider 切换的跨层流程。

Provider change 是“未来调用配置变更”，不是新的事实存储层：

```text
请求切换 Provider
    → 排入 ThreadController
    → 到达 model safe point
    → 从 durable history 重新构造 context
    → 解析目标 provider/model
    → 创建新的 ModelInvocationSnapshot
```

运行中的 provider request 默认继续使用旧快照，或由用户显式 interrupt。Provider-specific
response ID、cache key 或连接可以由 adapter 暂存；即使持久化为诊断元数据，也不能成为
恢复正确性的前提。

只有以下评测持续失败时，才引入正式 `ProviderHandoff`：

- 约束保留；
- 已完成工作识别；
- 决策一致性；
- Tool Result 引用准确性；
- 切换后的继续执行成功率。

即使引入，Handoff 也只是带 provenance 的派生 context artifact，不是权威历史。

## 10. Tool 执行

### 10.1 生命周期

```text
Model emits Tool Call
    → validate schema and availability
    → evaluate approval/sandbox policy
    → durable commit Tool Call / approval state
    → execute outside Thread state owner
    → durable commit Tool Result or terminal execution state
    → publish committed event
    → build next ModelInvocationSnapshot
```

Tool adapter 不得直接修改 Thread state。

### 10.2 取消与 unknown outcome

取消是 best effort：

- 本地 child process 应尝试终止整个受控进程树；
- HTTP/MCP 调用应传播 cancellation；
- 远端副作用可能在本地取消前已经完成；
- crash 时不能假设 Running Tool 仍在，也不能自动重放。

恢复后，未完成且可能有副作用的调用进入明确的 `UnknownOutcome` 或等价终态，并要求用户或
tool-specific reconciliation。不能把它伪装成 `Cancelled` 或 `Failed`。

### 10.3 Retry

Retry 不是统一的执行层开关。每个工具显式声明策略：

```text
Never
SafeRead { attempts, backoff }
IdempotentWrite { operation_key, attempts, backoff }
ReconcileBeforeRetry { reconciliation }
```

参数错误、permission denial、syntax error 和 unknown outcome 默认不自动 retry。Tool failure
可以作为 Tool Result 返回模型继续处理；只有执行控制、持久化或不可恢复策略错误才必然使
Turn 失败。

## 11. Memory 与多 Agent

### 11.1 当前只保留三层

```text
Working context
    单次模型调用的派生输入，不是永久存储

Thread history
    当前任务的 durable events/items，是执行正确性的依据

Cross-thread memory
    尚未设计，不进入当前实现
```

长期记忆实施前必须单独接受以下契约：

- 用户明确授权和可见性；
- user/workspace/project scope；
- provenance 和置信度；
- 冲突、过期和撤回；
- 查询、导出和彻底删除；
- 敏感信息过滤、加密和保留期；
- 注入相关性与错误记忆评测。

### 11.2 多 Agent

多 Agent 的 identity、delegation、spawn saga、context seed、message/result delivery、
cancellation、resource budget 与恢复统一由
[`core-multi-agent.md`](core-multi-agent.md) 定义。本文只要求 App Server 能订阅多个独立 Thread，
并把 child lifecycle、interaction 和 result 投影给客户端；App Server 不合并父子 history。

## 12. App Server 与 Desktop 影响

当前 `turn/start` 已采用“durable accepted 后返回、Core execution mailbox 异步继续”的语义。
provider wire streaming 与 App Server 独立 outbound writer 仍会直接演进当前开发契约，并在同一
变更中迁移 Rust、Desktop、CLI、TUI、schema 与 fixtures。

目标 App Server task topology：

```text
bounded transport reader
    → connection gate
    → request processor
    → keyed Session/Thread scheduling
    → bounded outbound router
    → per-connection writer
```

需要区分：

- `$/cancelRequest`：取消一个 RPC handler 的等待；
- `turn/interrupt`：把 durable Turn 推向 Interrupted；
- connection close：清理 connection-owned request/resource/subscription；
- server shutdown：有 deadline 的全局 graceful stop。

Desktop Renderer 仍不持有 raw peer。后续由单一 projection service 消费 notification，
检测 durable sequence/stream cursor gap，并通过 `session/subscribe` 或
`thread/subscribe` 的 snapshot + gap 重建。

## 13. 分阶段实施

### Phase 0：固定当前地基（完成）

- 对当前工作区执行 Rust、协议生成和 Desktop Main tests；
- 迁移到 Session-first current contract；
- 用 typed command receipt 替代 operation identity/sidecar ledger；
- 统一 Session/Thread 的物理 event-stream engine；
- 新功能不继续增长超过约 800 LoC 的 `server.rs` 和 `thread_controller.rs`。

完成条件：当前工作区的实现状态有测试证据，文档不再用虚构 PR 编号表示完成度。

### Phase 1：固定 canonical Session/Thread contract（完成）

Session-first 基础迁移已经完成；准确范围和仍未完成的 protocol 契约见
[`protocol.md` 的当前完成度](protocol.md#9-当前完成度)。本阶段不再维护另一份类型清单。

完成条件：Core、storage 和协议测试不再依赖混合 durable/transient/request 的巨型 `Event`。

### Phase 2：Core TurnExecutor 最小 vertical slice（基础完成）

- 已在 `zeta-core` 内新建私有 `turn` 模块，没有提前创建 facade crate；
- 已定义 canonical `ModelService`、`ToolService` 与 cancellation contract；
- 已用 deterministic fake service 覆盖文本完成、顺序 Tool loop、取消与模型失败；
- 已通过 App Server composition adapter 接入 model-provider，Core 与 provider 均不反向依赖；
- 待在 Phase 3 将同步端口和调用路径演进为 async streaming。

基础完成条件已满足：Agent loop 不依赖 storage/App Server，提交顺序和取消有单元测试。完整
完成仍依赖异步 streaming 与 execution incarnation。

### Phase 3：ThreadController 与异步协议（基础完成）

- 已引入 per-Thread bounded execution mailbox；
- `turn/start` acceptance commit 后返回；
- 已实现 keyed model/tool scheduling、有界 backlog 和多 Thread 并发；
- interrupt 会取消 active model execution；
- 待拆分 App Server reader、processor、outbound writer，并加入 explicit incarnation 与 idle eviction。

基础完成条件已满足：一个 Thread 的长模型调用不阻塞另一个 Thread，且同 Thread execution FIFO。
完整完成仍依赖所有 command 的同一 mailbox 和独立 outbound transport worker。

### Phase 4：SessionCoordinator、fork 与一致性（基础完成）

- 已增加 SessionStore、Session reducer 与只串行结构操作的 SessionCoordinator；
- Thread 创建/fork 已使用可恢复 saga；
- 每个 Thread 已保持独立 lease 与 sequence；
- per-Thread async controller 与 provider context 隔离仍由 Phase 3 完成。

完成条件：故障注入不能产生不可回收的 orphan Thread 或永久 pending membership。

### Phase 5：Tool loop、approval 与 capability（顺序基础完成）

- 已实现 durable Tool Call 后顺序执行并提交 Tool Result；
- approval/user-input/capability 使用有 owner、deadline、cancel 的 Server → Client request；
- Tool Result、UnknownOutcome 和 retry policy 完整建模；
- cancellation 贯穿 tool、exec、sandbox 和 MCP adapter。

完成条件：在每个 durable boundary 注入故障后，不会静默重复副作用或留下永久 Running。

### Phase 6：Context、compaction 与 Provider 切换（Context 基础完成）

- 已实现统一 ContextAssembler，重建 durable message 与 Tool Call/Result；
- 引入 per-Thread ContextManager 与不可变 ContextPlan；
- 加入 instruction precedence、budget 与带 provenance 的 compaction checkpoint；
- Provider change 在 safe point 生效；
- 建立跨 provider continuity evaluation。

完成条件：切换 Provider 后能保持约束、决策和 Tool Result 引用；失败时可追溯到原始 Item。

### Phase 7：Projection 与多 Agent

- Renderer projection/resync；
- `multi_agent/`、MultiAgentCoordinator、durable delegation 与 spawn saga；
- immutable ContextSeed 与 child Thread context isolation；
- 跨 Thread message/result 的 durable delivery 与 join；
- Agent tree cancellation 和 resource budget；
- 根据真实负载决定 controller idle eviction。

### Phase 8：Memory RFC

只有隐私、生命周期和质量评测契约被接受后才开始实现跨 Thread memory。

## 14. 验证门

Rust：

```bash
cargo fmt --manifest-path zeta-rs/Cargo.toml --all -- --check
cargo clippy --manifest-path zeta-rs/Cargo.toml --workspace --all-targets -- -D warnings
cargo test --manifest-path zeta-rs/Cargo.toml --workspace
```

Desktop 与协议：

```bash
pnpm verify:protocol
pnpm --dir desktop run build:host
pnpm --dir desktop run typecheck:renderer
pnpm --dir desktop run test:main
```

新增异步执行控制还必须覆盖：

- 同 Thread FIFO 与不同 Thread 并发；
- Session 结构提交 FIFO，且不阻塞子 Thread 模型/工具执行；
- Session/Thread 双 stream 创建或 fork 的 crash reconciliation；
- acceptance response 与 notification 顺序；
- provider/tool cancellation race；
- completion 迟到、重复和旧 incarnation；
- mailbox、stream 和 outbound queue saturation；
- partial durable batch、crash recovery 和 idempotency replay；
- Tool unknown outcome 不自动重放；
- Provider 切换前后 context 一致性；
- Renderer gap/resync。

## 15. 验收标准

- rollout 是唯一权威历史，任何投影都能重建；
- Session 是 Thread membership/lineage 的权威边界，Thread 是独立执行与恢复边界；
- 长模型或工具调用期间仍可 interrupt 并服务其他 Thread；
- 同 Thread 只有一个逻辑状态提交者；
- 所有用户可见 final Item 在通知前 durable；
- provider/tool/execution 失败都有明确 terminal path；
- 断连、取消、超时和 shutdown 不留下永久 pending；
- Provider 切换不依赖旧 provider cache；
- Tool 副作用在 unknown outcome 下不被静默重放；
- 新模块遵守私有模块、显式导出、文件大小和 sibling tests 约束；
- connection/capability Session 不得与产品 Session 混用；
- 长期 Memory 不在没有独立 consent、scope、删除和评测契约时进入核心模型。

## 16. 参考

- [Zeta 架构索引](architecture.md)
- [zeta-rs 产品内核与统一对外层](zeta-rs-architecture.md)
- [Zeta App Server API](zeta-app-server-api.md)
- [OpenAI Codex App Server snapshot](https://github.com/openai/codex/blob/322d5b96cfa5c8fd52bd83ecfdb79cd9b330205f/codex-rs/app-server/README.md)
- [Pi session format snapshot](https://github.com/earendil-works/pi/blob/5bc1c2c0a6f07e00e8c240304182f213ab8d311f/packages/coding-agent/docs/session-format.md)
