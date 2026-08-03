# 上下文系统

> Core 总体边界：[`core.md`](core.md)
> Canonical Thread/Turn/Item contract：[`protocol.md`](protocol.md)
> 多 Agent context inheritance：[`core-multi-agent.md`](core-multi-agent.md)
> 内置模型提示词资产：[`zeta-prompts`](../zeta-rs/prompts/README.md)

## 快速理解

上下文系统决定某一次模型调用能看到哪些信息；它从可持久化历史生成模型输入，但不能成为第二份
对话历史。

| 读者首先会问 | 直接答案 | 深入阅读 |
| --- | --- | --- |
| Thread 历史就是模型上下文吗？ | 不是；历史是权威事实，上下文是按预算选择出的派生窗口 | [权威与派生关系](#3-权威与派生关系) |
| 多个 Thread 会共享上下文吗？ | 不会；每个 Thread 有独立的 `ContextManager`、窗口和压缩检查点 | [为什么不放进 Session](#2-为什么不放进-session) |
| 一次模型调用使用什么？ | 使用不可变的 `ContextPlan`，再由 `ContextAssembler` 组装请求 | [数据模型](#4-数据模型) |
| 超出模型预算怎么办？ | 先按规则选择和裁剪；需要压缩时产生可恢复检查点 | [上下文预算](#7-上下文预算)、[压缩](#8-压缩) |
| 模型或配置变化会污染旧窗口吗？ | 不会静默复用；相关 revision 变化会使派生状态失效并重建 | [供应商变更](#9-上下文窗口与供应商变更) |

## 1. 结论

Zeta 长期必须区分四个概念：

```text
Thread history
  durable authority

ContextManager
  per-loaded-Thread 的可重建上下文协调状态

ContextPlan
  一次 model invocation 的不可变选择结果

ContextAssembler
  ContextPlan → provider-neutral ModelRequest 的纯组装
```

上下文不是 Session 级对象。每个 Thread 有独立 history、sequence、context window、compaction
checkpoint 和 ContextManager。父子 Agent、fork Thread 或 side conversation 都不能共享可变
ContextManager。

ContextManager 也不能成为第二份 canonical history。它允许保存 cache、baseline、token estimate
和当前 window revision，但所有影响未来 prompt 的事实必须能够从 durable Thread history、
checkpoint、context seed 与 policy snapshot 重建。

## 2. 为什么不放进 Session

Zeta 的 `Session` 是多个 Thread 的产品容器，只拥有 membership、lineage、shared defaults 和
lifecycle。若 Session 持有 context，会产生无法接受的问题：

- 多个 Thread 竞争同一 history/window；
- child Agent 看到 parent 或 sibling 的未授权内容；
- 一个 Thread 的 compaction 改变另一个 Thread；
- provider/model 切换无法按 Thread safe point 生效；
- Session lock 被 model/context I/O 放大；
- Thread 无法独立恢复、并行和回收。

其他代码库中名为 `Session` 的对象可能实际代表一条 conversation/thread 的加载实例。Zeta 必须
按领域语义映射，不能按目录或类型名照搬。

## 3. 权威与派生关系

```text
Session event stream
  └─ membership / lineage / defaults

Thread event stream
  ├─ Turns / Items / Tool Calls / Tool Results
  ├─ interactions / Agent deliveries
  ├─ compaction checkpoints
  └─ context seed references
          │
          ▼ reduce
    durable ThreadSnapshot
          │
          ▼ derive
      ContextManager
          │
          ▼ prepare
       ContextPlan
          │
          ▼ assemble
       ModelRequest
```

规则：

- event stream 是 authority；
- `ThreadSnapshot` 是可重建 projection；
- `ContextManager` 是进程内可重建派生状态；
- `ContextPlan` 只属于一次 invocation；
- provider cache 是可丢弃优化；
- compaction summary 是带 provenance 的 durable 派生 artifact，不替代原始事件。

## 4. 数据模型

以下类型表达目标语义，名称可在实现时细化。

### 4.1 ContextInput

```rust
struct ContextInput {
    thread: ThreadSnapshot,
    turn_policy: TurnPolicySnapshot,
    model: ResolvedModelLimits,
    instructions: InstructionSnapshot,
    environment: AgentEnvironmentSnapshot,
    tools: Vec<ToolDefinition>,
    checkpoints: Vec<VerifiedContextCheckpoint>,
    seed: Option<AgentContextSeed>,
}
```

`ContextInput` 是一次 prepare 操作看到的完整不可变输入。它不得包含 live config manager、
credential store、provider client 或可变 Session history。

### 4.2 ContextPlan

```rust
struct ContextPlan {
    source_thread_sequence: u64,
    context_policy_revision: ContextPolicyRevision,
    model_capability_revision: ModelCapabilityRevision,
    instructions: ResolvedInstructions,
    selected_items: Vec<SelectedContextItem>,
    omitted_ranges: Vec<OmittedContextRange>,
    tools: Vec<ToolDefinition>,
    budget: ContextBudgetReport,
    checkpoint: Option<ContextCheckpointRef>,
}
```

ContextPlan 必须可诊断：

- 从哪个 Thread sequence 派生；
- 使用哪个 policy/model revision；
- 选中了什么；
- 省略了什么以及原因；
- token 预算如何分配；
- 使用了哪个 checkpoint。

它不要求成为公共 wire model。测试和诊断可以使用 Core-private readable view。

### 4.3 ContextManager 状态

```rust
struct ContextManager {
    observed_thread_sequence: u64,
    window_revision: ContextWindowRevision,
    reference_baseline: Option<ContextBaseline>,
    token_estimate: Option<TokenEstimate>,
    cached_plan: Option<CachedContextPlan>,
}
```

这些字段都可以丢失。若某个字段丢失会改变恢复后的语义，它就不应只存在 ContextManager 中，
而应先成为 durable fact。

## 5. 组件职责

### 5.1 ContextManager

负责：

- 检查 Thread/policy/model revision；
- 选择 context window 与有效 checkpoint；
- 组织 instruction、seed、history、Tool Result 和 Agent result；
- 调用纯 budget/selection planner；
- 决定是否需要 compaction；
- 管理 reference baseline 与 cache invalidation；
- 生成 `ContextPlan` 或明确的 `ContextPreparation` outcome。

不负责：

- 写 Thread store；
- 直接修改 Thread projection；
- 持有 provider client；
- 读取 live Config/MCP/Skill manager；
- 将未经验证的 memory 自动注入；
- 直接执行 summary model；
- 跨 Thread 读取另一个 live ContextManager。

### 5.2 ContextPlanner

选择和预算算法应保持纯函数：

```text
ContextInput + ContextPolicy
  → Ready(ContextPlan)
  | NeedsCompaction(CompactionPlan)
  | ContextError
```

把 planner 保持为纯函数可以独立验证 precedence、budget、Tool pairing、checkpoint selection 和
determinism。`ContextManager` 只协调生命周期与缓存。

### 5.3 ContextAssembler

只负责：

- 将 resolved instructions 写入 canonical request；
- 将 selected Item 转成 provider-neutral `InputItem`；
- 保持 Tool Call/Tool Result 配对与顺序；
- 应用已经确定的 tools、tool choice 和 output limit；
- 拒绝损坏 JSON、悬空引用和不支持的 canonical shape。

它不重新选择历史、不决定 compaction，也不读取 ThreadController。

当前基础 `ContextAssembler` 直接接受 `ThreadSnapshot` 是过渡 API。长期应改为接受
`ContextPlan`，Thread snapshot 的读取和选择由 ContextManager/Planner 完成。

### 5.4 内置提示词资产

[`zeta-prompts`](../zeta-rs/prompts/README.md) 只拥有四类 Zeta 内置、模型可见的提示词资产：system、
compaction、goals 和通用 review。它提供稳定的 asset ID、revision 与 compile-time body，但不决定
何时注入，也不读取 Thread、Config、Skill、MCP 或 provider runtime。

需要某类提示词的功能模块负责触发条件和生命周期；当资产进入 Agent 的 canonical context 后，仍由
Core context pipeline 负责 instruction layer、precedence、budget、provenance 和最终 request 组装。
`zeta-auto-review` 的 prompt/schema/revision 专用契约继续由该 crate 自己拥有。

当前 assembler 会把同一 Turn 中相邻的 `UserMessage` / `UserImage` 按 durable 顺序合并成一个
provider-neutral user `Message`，分别映射为 `ContentPart::Text` 与
`ContentPart::ImageUrl`。图片内容在 Turn 接受阶段已经由 Core 校验；provider adapter 只负责
转换为各自的 URL/base64 image block。

## 6. 上下文组织

### 6.1 逻辑层次

一次 invocation 的模型可见内容按语义分层：

```text
1. Product/system constraints
2. Developer and workspace instructions
3. Session shared defaults snapshot
4. Agent role and delegated task
5. Environment/capability snapshot
6. Verified context seed or checkpoint
7. Thread conversation history
8. Tool/Agent results required by current continuation
9. Current Turn input
```

该顺序表示 precedence，不要求每个 provider 使用相同 wire role 或数组布局。Provider adapter
只能做 wire 映射，不能改变 Core 已解析的 precedence。

### 6.2 指令优先级

每段 instruction 必须带：

- source kind；
- source revision；
- scope；
- precedence；
- provenance；
- sensitivity/visibility metadata。

低优先级内容不能覆盖高优先级约束。Agent delegation 可以收紧 policy 或增加任务说明，但不能
放宽 system/session policy ceiling。

### 6.3 结构不变量

任何 ContextPlan 必须满足：

- 当前用户输入不可被预算算法删除；
- Tool Result 必须引用可见 Tool Call；
- 需要继续执行的 Tool Call/Result group 原子保留；
- Agent result 必须引用可见 delegation/message identity；
- checkpoint 与 tail 不重复覆盖同一 source range；
- omission 不破坏角色和调用边界；
- provider 不支持的内容只能按显式降级策略转换或报错；
- reasoning/plan 是否回灌必须由 canonical contract 明确允许。

## 7. 上下文预算

### 7.1 预算输入

预算至少包含：

- model context window；
- reserved output tokens；
- system/developer instruction cost；
- Tool definitions cost；
- current Turn minimum；
- Tool/Agent continuation minimum；
- safety margin；
- compaction threshold；
- tokenizer/estimate revision。

不得使用一个含义不明的 `max_tokens: Option<u32>` 同时表达 context window、output limit 和缺省。
这些值使用独立 newtype/enum。

### 7.2 预算顺序

推荐顺序：

```text
model context window
− reserved output budget
− mandatory instructions/tools/current input
− safety margin
= history budget
```

历史选择按完整语义单元处理，而不是按字符串尾部截断：

- Turn group；
- Tool Call/Result group；
- Agent delegation/result group；
- checkpoint + uncovered tail；
- attachment/artifact reference group。

### 7.3 Overflow 结果

预算不足必须产生明确 outcome：

- `NeedsCompaction`；
- `CurrentInputTooLarge`；
- `MandatoryInstructionsTooLarge`；
- `ToolDefinitionsTooLarge`；
- `UnsupportedContextShape`。

不能静默删除当前输入、权限约束或未完成 Tool/Agent continuation。

## 8. 压缩

### 8.1 持久化检查点

Checkpoint 至少记录：

- stable checkpoint ID；
- source Thread；
- covered sequence range；
- referenced Item/Event IDs；
- source digest；
- summary 内容或 artifact reference；
- schema、prompt 与 context policy revision；
- generator/model metadata；
- creation time；
- verification status。

原始 event log 永不因 compaction 删除。

### 8.2 压缩流程

```text
ContextManager detects pressure
→ returns NeedsCompaction(CompactionPlan)
→ Turn/Core orchestration invokes CompactionService
→ validate summary + provenance + digest
→ ThreadController durable commits checkpoint
→ install new ThreadSnapshot
→ ContextManager invalidates old window/cache
→ prepare again from committed snapshot
```

Summary model I/O 不持有 Thread writer。只有 checkpoint durable commit 后，后续 invocation 才能
依赖它。

### 8.3 恢复与失效

以下情况拒绝 checkpoint 并退回原始 history：

- source range 不存在或重叠错误；
- digest 不匹配；
- schema/policy 不兼容；
- referenced Item 缺失；
- checkpoint 来自另一个 Thread；
- visibility 或 sensitivity policy 已变化且不能证明兼容。

Provider/model 切换不自动使 checkpoint 失效，但必须重新按新 model limits 和 capabilities 规划。

## 9. 上下文窗口与供应商变更

Context window 是模型可见历史的派生窗口，不是新的 history aggregate。

每次 model/provider/context policy 变化：

```text
wait for safe point
→ freeze new revisions
→ invalidate incompatible cached plan/baseline
→ rebuild from durable ThreadSnapshot
→ create new ModelInvocationSnapshot
```

Provider response ID、prompt cache key 和 connection state 可以作为 adapter 优化，但不能成为
恢复正确性的前提。

## 10. 多 Agent 规则

每个 child Agent 的 ContextManager 只读取：

- child Thread 的 durable history；
- spawn 时固定的 `AgentContextSeed`；
- durable delivered parent/peer messages；
- child 自己的 policy/environment snapshot。

它不能读取 parent/sibling 的 live history 或 ContextManager。

Spawn context mode 必须显式：

```text
Fresh
Selected(item/artifact references)
ForkedPrefix(full | last turns | checkpoint plus tail)
```

所有模式都固定 parent Thread sequence 和 provenance。Spawn 后 parent compaction 或继续执行不改变
child 的 seed。Child result 回到 parent 后只是一个有界、可追踪的 durable result Item，由 parent
ContextManager 决定是否选入下一次 ContextPlan。

详细协议见 [`core-multi-agent.md`](core-multi-agent.md)。

## 11. 目录与可见性

```text
core/src/
├─ context/
│  ├─ mod.rs
│  ├─ model.rs
│  ├─ plan.rs
│  ├─ assembler.rs
│  ├─ budget.rs
│  ├─ validation.rs
│  └─ *_tests.rs
└─ context_manager/
   ├─ mod.rs
   ├─ manager.rs
   ├─ selection.rs
   ├─ window.rs
   ├─ compact.rs
   └─ *_tests.rs
```

`context` 保存不可变 value 与纯算法；`context_manager` 保存 per-loaded-Thread 协调逻辑。两个模块
默认 private。只有确有外部消费者时才从 `lib.rs` 导出窄 value/port，不能公开 cache、baseline、
window mutable state 或 ContextManager 自身。

长期 ownership：

```text
ThreadController
└─ LoadedThreadState
   └─ ContextManager

TurnExecutor
└─ asks ContextManager to prepare
   └─ receives immutable ContextPlan
```

## 12. 当前迁移

当前：

- `context/assembler.rs` 从完整 `ThreadSnapshot` 直接构造 `ModelRequest`；
- 已覆盖同一 Turn 的有序 user text/image、assistant message 与 Tool Call/Result 基础映射；
- 已拒绝损坏的 Tool arguments 和悬空 Tool Result；
- 尚无 ContextManager、instructions、预算、checkpoint 和 compaction。

迁移顺序：

1. 增加 `ContextInput`、`ContextPlan` 和纯 validation/budget；
2. 将现有 assembler 改为 `ContextPlan → ModelRequest`；
3. 增加 per-Thread `ContextManager`，但先不做 cache；
4. 在 `LoadedThreadState` 中持有 ContextManager；
5. 增加 instruction/environment/policy snapshot，并由调用方决定何时贡献内置 prompt asset；
6. 增加 checkpoint schema 与 compaction flow；
7. 最后增加 cache/reference baseline，并以 sequence/revision 严格失效。

不要先复制一份 mutable history 再逐步“变成”ContextManager。

## 13. 验证

必须覆盖：

- 相同 ContextInput 产生字节级等价的 ContextPlan；
- instruction precedence；
- budget boundary 与 mandatory overflow；
- Tool Call/Result 原子保留；
- Agent delegation/result 原子保留；
- checkpoint + tail 无遗漏、无重复；
- corrupt checkpoint 回退；
- provider/model/policy revision invalidation；
- ContextManager 丢失后从 durable facts 重建等价结果；
- parent/child/sibling context 隔离；
- compaction crash before/after durable commit；
- current input 永不被静默删除。

## 14. 固定决策

- Context 属于一次 model invocation；
- ContextManager 属于一个 loaded Thread；
- Session 和 Agent tree 不拥有共享 ContextManager；
- Thread event stream 是唯一 history authority；
- ContextManager 可丢失、可重建；
- ContextAssembler 保持纯组装；
- 选择、预算和 compaction 决策不进入 provider adapter；
- checkpoint durable，但不删除原始 history；
- 多 Agent 只通过 immutable seed 和 durable message/result 传递 context；
- 跨 Thread memory 在单独 RFC 完成前不进入 ContextManager。
