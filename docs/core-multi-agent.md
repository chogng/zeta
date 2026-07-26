# `zeta-core` 多 Agent 架构

> Core 总体边界：[`core.md`](core.md)
> Context 与 ContextManager：[`core-context.md`](core-context.md)
> Canonical Session/Thread/Turn contract：[`protocol.md`](protocol.md)

## 1. 结论

Zeta 的多 Agent 使用同一 Session 下的独立 Thread：

```text
Session
└─ parent Thread / Agent
   ├─ child Thread / Agent A
   └─ child Thread / Agent B
```

每个 Thread 独立拥有：

- durable sequence、Turn 和 Item；
- ThreadController 的逻辑提交序列；
- ContextManager、context window 与 compaction；
- model/tool/policy snapshot；
- cancellation domain；
- 恢复和 terminal outcome。

多 Agent 不是多个执行 task 共享一份 `SessionHistory`。父子和 sibling 不共享 mutable context、
projection、provider conversation ID 或 Tool state。

长期必须把三种关系分开：

1. **Thread lineage**：产品里的 root/fork/spawn 拓扑；
2. **Agent delegation**：谁委派了什么工作以及结果状态；
3. **Context inheritance**：child 首次 invocation 能看到 parent 的哪些内容。

只记录 `parentThreadId` 不能表达后两者。

## 2. 身份与聚合边界

### 2.1 SessionId

标识一个产品任务及其 Thread 拓扑。Session 只串行 membership、lineage、shared defaults 和
lifecycle，不串行 child Thread 执行。

### 2.2 ThreadId

标识一条独立 Agent execution branch。当前阶段一个活跃 Agent 对应一个 Thread，因此可以用
`ThreadId` 作为 Agent execution identity。

暂不增加与 Thread 一一对应的 `AgentId`。只有出现下列真实需求时才引入：

- 一个 Agent 身份跨多个 Thread 延续；
- Thread 被替换但 Agent identity 必须保持；
- 产品需要独立查询 Agent，而不能从 Thread/delegation 投影得到。

### 2.3 DelegationId

`DelegationId` 是必需的独立身份，标识一次 parent → child 工作委派。它不能由数组位置、child
Thread ID 或 Tool call completion 顺序隐式表达。

一个 delegation 至少关联：

- parent Thread/Turn；
- parent sequence anchor；
- child Thread；
- delegated task；
- context seed；
- lifecycle；
- result/delivery identity。

Agent spawn 可能由模型 Tool Call、用户操作或系统策略触发，因此 `DelegationId` 不直接等同
`ToolCallId`。

### 2.4 AgentMessageId

跨 Thread message/result 需要稳定 `AgentMessageId`，用于：

- sender retry；
- receiver deduplication；
- crash recovery；
- delivery receipt；
- provenance。

## 3. AgentCoordinator

`AgentCoordinator` 是 Core 内部的多 Agent 协调组件。

负责：

- spawn、send、join、cancel、close；
- delegation lifecycle；
- context seed 物化；
- parent/child cancellation tree；
- max depth、max children、并发和总资源预算；
- 协调 SessionCoordinator 创建 child Thread；
- 协调 ThreadController 提交 parent/child durable facts；
- 跨 Thread message/result 的 outbox/inbox delivery；
- crash reconciliation。

不负责：

- 持有 Thread transcript；
- 读取 live ContextManager；
- 直接执行 child Turn；
- model/Tool I/O；
- Session membership reducer；
- provider connection 或远端 Agent transport；
- UI projection。

AgentCoordinator 可以协调多个 aggregate，但不能建立跨所有 Thread 的大锁。长 I/O、等待 child
和等待 delivery receipt 都在 aggregate writer 之外。

## 4. Create、Fork 与 Spawn

三种操作语义不同：

| 操作 | 目的 | parent 关系 | context |
| --- | --- | --- | --- |
| Create | 新建独立 Thread | 无 | fresh |
| Fork | 创建产品历史分支 | immutable lineage anchor | 明确 fork selection |
| Spawn | 委派 child Agent | delegation + parent anchor | 明确 AgentContextSeed |

Fork 不自动等于 sub-agent。用户可以 fork side conversation；Agent 也可以在 fresh context 下
spawn child。

目标 `ThreadOrigin` 至少能区分：

```rust
enum ThreadOrigin {
    Root,
    Fork {
        parent_thread_id: ThreadId,
        parent_sequence: u64,
    },
    AgentSpawn {
        parent_thread_id: ThreadId,
        parent_sequence: u64,
        delegation_id: DelegationId,
    },
}
```

`ThreadOrigin` 只描述来源和不可变 anchor。具体 context inheritance 放在独立
`AgentContextSeed` 中，避免把拓扑与 prompt policy 混成一个 enum。

当前实现的 `Fork` 只保存 lineage anchor，并创建空 child Thread；它不是完整的 history fork 或
Agent spawn contract。文档和 API 在实现 inheritance 前必须保持这一事实。

## 5. AgentContextSeed

Spawn 时必须创建不可变、可验证的 seed：

```rust
struct AgentContextSeed {
    delegation_id: DelegationId,
    parent_thread_id: ThreadId,
    parent_sequence: u64,
    task: DelegatedTask,
    role: AgentRoleSnapshot,
    inheritance: AgentContextMode,
    selected_sources: Vec<ContextSourceRef>,
    policy_ceiling: DelegatedPolicyCeiling,
    capability_scope: DelegatedCapabilityScope,
    digest: ContextSeedDigest,
}
```

### 5.1 Inheritance mode

使用自描述 enum：

```rust
enum AgentContextMode {
    Fresh,
    Selected(SelectedContext),
    ForkedPrefix(ForkedContext),
}

enum ForkedContext {
    Full,
    LastTurns(NonZeroU32),
    CheckpointAndTail,
}
```

不使用 `inherit_context: bool` 或 `last_turns: Option<u32>`。

语义：

- `Fresh`：只注入 shared constraints、Agent role 和 delegated task；
- `Selected`：注入显式 Item/artifact references；
- `ForkedPrefix`：从固定 parent sequence 选择完整或裁剪历史。

所有模式都必须固定 source sequence 和 provenance。Spawn 后 parent 继续执行、compaction、
provider change 或 policy refresh 都不能偷偷改变 child seed。

### 5.2 Policy inheritance

Child policy 必须满足：

```text
effective child policy
  = session/system ceiling
  ∩ parent delegated ceiling
  ∩ child role restrictions
  ∩ current host safety policy
```

Child 可以被进一步收紧，不能静默获得 parent 没有的 capability 或放宽 approval。

### 5.3 Seed persistence

Seed 可以作为 child creation fact 或 stable artifact reference 持久化，但必须在 child 首次执行
前 durable。若 seed 引用 parent Item：

- source Thread/sequence 必须固定；
- original event log 必须仍可解析；
- digest 必须可验证；
- visibility/sensitivity policy 必须允许；
- materialization 失败时 child 不得在不完整上下文下静默启动。

## 6. Spawn saga

Spawn 跨 parent Thread、Session 和 child Thread，必须使用可恢复 saga：

```text
1. parent commits DelegationRequested
2. Session commits ChildThreadCreationPlanned
3. child Thread is created with AgentSpawn origin + ContextSeed
4. Session commits ChildThreadAttached
5. parent commits DelegationStarted(child ThreadId)
6. child Turn is accepted and scheduled
```

每一步都使用同一个 `DelegationId`。Crash recovery：

- 只有 step 1：继续计划 child，或按明确 policy 取消；
- 已计划但未创建：创建确定身份的 child；
- child 已创建但未 attach：完成 attach；
- attach 后 parent 未记录 started：补交 started；
- child 已运行：不得创建第二个 child；
- saga 失败：提交可解释的 terminal delegation outcome。

如果 storage 将来提供 multi-stream transaction，可以优化提交次数，但不能改变可观察语义。

## 7. Communication

### 7.1 显式 message

Parent、child 和 sibling 只能通过显式 Agent message 交流：

```rust
struct AgentMessage {
    message_id: AgentMessageId,
    delegation_id: Option<DelegationId>,
    sender_thread_id: ThreadId,
    receiver_thread_id: ThreadId,
    sender_sequence: u64,
    content: AgentMessageContent,
    provenance: AgentMessageProvenance,
}
```

不能通过：

- 读取另一个 ContextManager；
- 共享 mutable `Vec<Message>`；
- 复用 provider conversation ID；
- 直接修改 receiver projection；
- 把跨 Agent message 冒充普通 user input。

### 7.2 Durable delivery

跨 Thread delivery 使用 outbox/inbox 语义：

```text
sender commits AgentMessageSent
→ delivery worker/coordinator attempts receiver append
→ receiver deduplicates AgentMessageId
→ receiver commits AgentMessageReceived
→ sender records receipt when required
```

目标是 at-least-once transport + exactly-once durable application，而不是假设一次函数调用天然
exactly once。

Receiver terminal/archived、policy rejection 和 delivery timeout 都需要明确 outcome。消息不能
无声丢失。

### 7.3 Steering

发送给正在执行 child 的新 instruction 是 steering，不等于修改它已经冻结的 invocation：

- 先 durable delivery；
- 当前 invocation 是否 interrupt 由明确 policy 决定；
- 默认在下一个 model safe point 生效；
- safety tightening 可以立即传播；
- 普通 steering 不回写历史快照。

## 8. Result 与 Join

Child 完成时生成有界结果：

```rust
struct DelegationResult {
    delegation_id: DelegationId,
    child_thread_id: ThreadId,
    status: DelegationResultStatus,
    summary: String,
    artifacts: Vec<ArtifactRef>,
    source_range: ThreadSequenceRange,
    digest: DelegationResultDigest,
}
```

结果 flow：

```text
child commits terminal result
→ result delivery uses stable AgentMessageId
→ parent commits DelegationResultReceived
→ waiting join condition is re-evaluated
→ parent resumes at safe point
→ parent ContextManager may select the bounded result
```

Parent 不自动继承 child transcript。需要更多细节时，parent 可以通过显式 message 请求，或在
授权范围内引用 child artifact。

Join policy 使用 enum：

```text
All
Any
Quorum(NonZeroU32)
Explicit(Vec<DelegationId>)
```

Join 等待必须 durable；它不能靠一个进程内 `join_all` future 表达。

## 9. Cancellation 与 terminal semantics

Cancellation tree：

```text
parent Agent source
├─ child A source
│  └─ child A operations
└─ child B source
   └─ child B operations
```

规则：

- parent cancellation 传播到所有 live descendants；
- child cancellation 不影响 parent 或 sibling；
- Session shutdown 可以取消所有 roots；
- cancel signal 是 best effort；
- child 已执行的 Tool 副作用仍按 UnknownOutcome/reconciliation 处理；
- parent 取消后迟到的 child result 可以 durable 记录为 late，但不能恢复已 terminal parent Turn；
- terminal delegation 状态必须 durable，不能只看 task handle 是否结束。

可区分的 terminal outcome 至少包括：

- completed；
- failed；
- cancelled；
- policy denied；
- capacity rejected；
- context seed invalid；
- delivery failed；
- unknown outcome。

## 10. Resource budget 与调度

必须区分：

```text
Per-invocation context budget
  ContextManager 负责

Per-Turn execution limits
  TurnExecutor 负责

Per-Agent-tree resource budget
  AgentCoordinator 负责
```

Agent tree budget 至少包含：

- max depth；
- max live children；
- max total descendants；
- model concurrency；
- Tool concurrency；
- cumulative token/cost budget；
- deadline；
- optional per-role quota。

Reservation 必须在 spawn 前完成。并发 slot 是进程内资源，可以在 crash 后重建；已经消耗的
usage 和 durable delegation 状态不能只存在内存。

Capacity saturation 返回稳定 retryable/terminal outcome，不能无限排队。不同 Session 之间还需要
宿主级 fairness，但其全局 scheduler 实现不必进入 Core aggregate。

## 11. Context isolation

每个 child ContextManager 只读取：

- child Thread history；
- child creation 时的 AgentContextSeed；
- durable delivered message/result；
- child 自己的 TurnPolicySnapshot；
- composition root 提供的 child environment/capability snapshot。

不能读取：

- parent/sibling live ContextManager；
- parent 在 anchor sequence 之后的 history；
- Session 中其他 Thread 的 transcript；
- 未授权的 cross-thread memory；
- parent provider cache。

Parent result injection 规则：

- result 是独立 canonical Item；
- 必须携带 delegation/child/provenance；
- 由 parent ContextManager 参与预算和选择；
- 不能跳过 instruction precedence；
- child 的不可信文本不能升级为 system fact。

## 12. Approval 与 capability

Child 的 approval request 仍属于 child Thread/Turn。Interaction 必须携带 child identity 和
delegation provenance。

默认规则：

- parent Agent 不是安全 authority；
- parent 不能替用户批准超出 delegated ceiling 的 action；
- UI 可以聚合显示同一 Agent tree 的 requests；
- resolve 必须命中精确 RequestId；
- parent cancel 可取消 child pending approval；
- disconnect 与 owner selection 仍由 App Server 负责。

Capability handle 不能跨 Agent 隐式共享。若 child 需要 browser/terminal/resource：

- spawn seed 声明 capability scope；
- host 在 child Turn safe point 解析 handle；
- handle lifecycle 仍由宿主拥有；
- child completion/cancel 触发明确 release policy。

## 13. Recovery

恢复顺序：

1. replay Session topology；
2. replay parent/child Thread；
3. reconcile incomplete spawn saga；
4. rebuild delegation projection；
5. redeliver unacknowledged messages/results；
6. rebuild cancellation and capacity state；
7. rebuild each child ContextManager；
8. schedule eligible non-terminal Turns。

必须拒绝：

- 重复 spawn 相同 delegation 的第二个 child；
- 重复应用相同 AgentMessageId；
- 旧 incarnation 的 completion；
- source digest 不一致的 context seed；
- terminal parent/child 上的非法恢复；
- policy scope 已失效但仍尝试继续的 operation。

## 14. Protocol 影响

目标 canonical contract 需要逐步增加：

- `DelegationId`、`AgentMessageId`；
- `ThreadOrigin::AgentSpawn` 或等价来源模型；
- `AgentContextSeed` / stable seed reference；
- delegation requested/started/terminal facts；
- Agent message sent/received/delivery outcome；
- `DelegationResult` Item；
- durable join/wait state；
- readable Agent tree/delegation projection；
- stable error/outcome code。

不要把 Core-private mailbox、task handle、ContextManager 或 cancellation node 暴露到 protocol。

Protocol 变更必须同步更新：

- Rust canonical types；
- JSON Schema；
- generated TypeScript；
- App Server method/update contract；
- store envelope/schema version；
- Desktop/CLI/TUI projection；
- contract fixtures 和 recovery tests。

## 15. 目标目录

```text
core/src/
└─ agent/
   ├─ mod.rs
   ├─ coordinator.rs
   ├─ delegation.rs
   ├─ delivery.rs
   ├─ budget.rs
   ├─ recovery.rs
   └─ *_tests.rs
```

相关 ownership：

```text
AgentCoordinator
├─ uses SessionCoordinator for topology saga
├─ uses ThreadController for durable parent/child mutations
├─ creates immutable AgentContextSeed
├─ owns delegation/delivery/resource policy
└─ never owns ContextManager
```

模块默认 private。若 App Server 需要操作多 Agent，只公开 typed command/result 和 read-only
projection，不公开 coordinator 内部状态机。

## 16. 落地顺序

1. 冻结 `DelegationId`、Agent spawn 与 message/result 语义；
2. 区分 `ThreadOrigin::Fork` 与 Agent spawn；
3. 定义 ContextSeed 及 `Fresh/Selected/ForkedPrefix`；
4. 增加 parent delegation durable facts；
5. 用可恢复 saga 创建 child Thread；
6. 增加 child execution、result 和 durable join；
7. 增加 outbox/inbox delivery 与 steering；
8. 增加 cancellation tree 与 Agent tree budget；
9. 增加 UI projection 和跨 crate contract；
10. 完成 crash、duplicate、late result 与 isolation 测试。

第一阶段不需要：

- 独立 `zeta-agent` crate；
- 与 Thread 一一对应的 Agent aggregate；
- 任意 Agent graph workflow engine；
- 跨机器 Agent transport；
- 自动共享长期 memory；
- parent/child provider cache 共享。

## 17. 验证

必须覆盖：

- spawn 的每个 durable boundary crash；
- 相同 DelegationId 幂等恢复；
- fork 与 spawn 语义不混淆；
- Fresh/Selected/ForkedPrefix 的可见内容；
- parent anchor 后新增内容对子 Agent 不可见；
- parent/child/sibling ContextManager 隔离；
- message at-least-once transport + exactly-once apply；
- duplicate/late/out-of-order result；
- join All/Any/Quorum；
- parent cancel、child cancel 和 sibling independence；
- max depth/children/concurrency/budget；
- child Tool UnknownOutcome；
- invalid seed digest；
- parent terminal 后的迟到 result；
- restart 后 delegation、delivery、join 与 context 等价恢复。

## 18. 固定决策

- 一个活跃 Agent 当前绑定一个独立 Thread；
- ThreadId 暂时足够表达 Agent execution identity；
- DelegationId 与 AgentMessageId 是独立稳定身份；
- fork lineage、Agent delegation 与 context inheritance 分离；
- child Agent 不共享 parent/sibling mutable context；
- context seed 在 spawn 时固定 parent sequence；
- AgentCoordinator 协调拓扑和 delivery，但不拥有 context；
- parent/child 只通过 durable message/result 通信；
- parent 不自动继承 child transcript；
- per-Thread context budget 与 Agent tree resource budget 分离；
- cancellation 向下传播，不向 parent/sibling 反向传播；
- 多 Agent 恢复只依赖 durable facts；
- 在出现真实多 Thread Agent identity 需求前不增加 Agent aggregate。
