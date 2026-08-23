# Agent 执行架构整体重审：设计文档 + 实施计划

产出物：修订后的总体设计文档 + 分阶段实施计划。**本次只写文档，不改代码。**

## 一、重审结论（设计文档将采取的立场）

基于对五篇架构文档与 `zeta-rs/core` 实际实现的交叉审读：

### 1. 经验证保留的决策（不再重开）

- protocol → core → store 分层；App Server 唯一门禁；Core 不依赖 provider/storage/transport
- Session（membership/lineage/defaults）与 Thread（history/执行/恢复边界）的聚合拆分
- 每 Thread 逻辑单写者 + typed event log + 纯 reducer + command receipt 幂等回放
- 副作用前 durable（`ToolExecutionStarted`）、unknown outcome 不自动重放、incarnation 拒绝迟到 completion
- 否决 `CanonicalHistory` / `ProviderLaneRegistry` / `ProviderHandoff` / 无契约长期记忆

### 2. 需要修订的决策（重审的核心产出）

| # | 原设计 | 修订立场 | 理由 |
|---|---|---|---|
| R1 | `TurnPolicySnapshot` 为进程内不可变结构 | **policy 冻结改为 durable fact**：Turn 接受时持久化 policy revision/标识，恢复时确定性重建快照 | 进程内快照不能跨 crash-resume 存活；恢复后的 Turn 会从"当前配置"重建 policy，违反"不得静默放宽"自己的验收标准。model 已经这样做了（`TurnAccepted.model`），policy 应对齐 |
| R2 | Phase 3 端口演进为 async streaming（隐含 tokio 化） | **不承诺 tokio 迁移**：保留同步端口 + per-Thread OS 线程邮箱；真实流式经 wire-level SSE decoder → `ModelStreamSink` 达成；App Server 补独立 outbound writer 线程 | 桌面级并发（几十个 Thread 上限）；同步代码对 durability 不变量更易验证；cancellation tree 已闭环；sink 契约已为流式预留。async 化收益不成比例，留作显式 deferred 决策 |
| R3 | ContextManager 完整形态（cache/baseline/estimate）一步到位 | **裁剪落地**：纯函数 `ContextPlanner` + `ContextPlan` 先行，ContextManager 只做薄协调（无 cache）；compaction checkpoint 的 durable schema 提前进 protocol | 纯函数可独立验证 precedence/budget/配对/确定性；cache 失效是最难验对的部分，推迟到有真实性能证据 |
| R4 | 多 Agent 十步落地顺序 | **契约冻结与运行时分离**：先只冻结身份语义（`DelegationId`/`AgentMessageId`/`ThreadOrigin::AgentSpawn`/seed schema）进 protocol；coordinator 运行时 gate 在上下文系统完成之后 | context isolation 与 seed 依赖 ContextPlan；先冻结契约避免后续 protocol 破坏性变更 |
| R5 | 文档以现在时描述未实现组件 | **每个组件挂显式状态标记**（已实现/部分/仅设计/推迟），总体文档设"状态总账"表 | `ContextManager`/`MultiAgentCoordinator`/两级快照在代码中零引用，但 core.md §5/§13 读起来像现状；这是当前最大的架构文档风险 |

### 3. 需要修正的文档-事实矛盾（写作时逐项落实）

- core.md §13 "超过 800 LoC 不再增功能" vs `thread_controller.rs`(1189)/`session_coordinator.rs`(998)/`thread_reducer.rs`(947) 已越线 → 在实施计划中列为阶段 A 的拆分对象，文档如实标注
- core.md §6 端口表列出 `CapabilityBroker`/`CompactionService`/`Clock`/`IdGenerator`，代码中不存在 → 标注"仅设计"，并补上实际存在的 `PolicyService`/`ThreadUpdateSink`/`ToolOutputSink`
- core-context.md 已注明 assembler 直接吃 `ThreadSnapshot` 是过渡 API → 保留，接入阶段 B
- 目标目录树（session/ thread/ turn/ context_manager/ multi_agent/ tool/）vs 实际扁平布局 → 标注为迁移目标 + 归入阶段 A

### 4. 优先级推导（用户委托重审结论决定）

三条轴：**正确性风险**（context 溢出无显式 outcome、恢复时 policy 漂移）> **产品可用性**（长会话必然超窗）> **用户体验**（真 token 流式）。多 Agent 依赖前两者。

结论主线：**地基修整 → 上下文系统 → 真实流式 → 多 Agent 契约 → 多 Agent 运行时**。
（B/C 无强依赖，可并行推进；文档中注明。）

## 二、实施计划骨架（写入总体文档的阶段表）

- **阶段 A｜地基修整**：三个越线大文件按目标目录拆分（纯迁移不改语义）；policy 冻结 durable 化（R1）。完成条件：模块 <500 LoC、全部现有测试通过、"恢复后 policy 不漂移"新测试。
- **阶段 B｜上下文系统**：`ContextInput`/`ContextPlan` + 纯 planner/budget → assembler 改为 `ContextPlan → ModelRequest` → 薄 ContextManager（无 cache）→ checkpoint protocol/store schema → compaction flow。完成条件：overflow 五类显式 outcome、同输入字节级等价 plan、checkpoint 前后 crash 测试。
- **阶段 C｜真实流式**：provider SSE decoder → `ModelStreamSink`；App Server 独立 outbound writer；Desktop gap/resync。完成条件：增量 delta 到端、断连重放、流中取消 race 测试。
- **阶段 D｜多 Agent 契约冻结**：protocol 增加身份/seed/delegation facts + schema/TS 同步，无运行时。
- **阶段 E｜MultiAgentCoordinator**：spawn saga、outbox/inbox delivery、join、tree budget、cancellation tree。
- 贯穿：每个新 durable boundary 补 fault injection。

## 三、要写/改的文件（全部为文档）

| 文件 | 动作 |
|---|---|
| `docs/zeta-agent-runtime-architecture.md` | **重写**为总体设计 v2：状态总账、修订后分层、R1–R5 决策与理由、阶段 A–E 实施计划（替换原阶段 0–8）、验证门 |
| `docs/core.md` | 手术式修订：§5 组件加状态标记；§6 端口表对齐实际导出；§13 矛盾修正 |
| `docs/core-context.md` | 加状态头；迁移顺序对齐阶段 B；planner/manager 标"仅设计" |
| `docs/core-multi-agent.md` | 加状态头；落地顺序改为"契约冻结（D）→ 运行时（E）"两段并写明 gate 条件 |
| `docs/architecture.md` | 小改：Agent 运行时行的指引与状态说法对齐 |

写作遵循 `docs/documentation-guidelines.md`：中文、两层文档职责、canonical ownership 双向链接、不以现在时描述未实现能力。

## 四、验证

- 文档中每一条"已实现"声明都以本次审读的代码符号为据（已核对：mailbox/incarnation/approval/escalation/circuit breaker/saga 均存在；ContextManager 等零引用）
- 五篇文档交叉链接与状态标记一致性自查
- 文档生成如受影响则运行 `corepack pnpm --dir docs-site run generate:docs` 验证（仅当生成器消费这些文档）
