# Agent Harness 设计

> 状态：Accepted（2026-08-03）；实现状态最后核对于 2026-08-23。
> 定位：回答"Core 的 agent loop 具体怎么搭起来"——一次模型调用长什么样、Turn 内循环与
> 失败弹性、steering、提示词组织、工具选择与注册时机、上下文裁剪压缩、prompt cache、评测。
>
> 分工：执行内核（单写者、durable commit、恢复、取消）由
> [`zeta-agent-runtime-architecture.md`](zeta-agent-runtime-architecture.md) 负责；上下文生命
> 周期抽象（ContextPlan/Manager/checkpoint）由 [`core-context.md`](core-context.md) 负责；
> 工具三层契约（定义/绑定/执行接口）由 [`tools.md`](tools.md) 负责；**逐工具规格（schema、
> 描述正文、错误文案）与系统提示词扩写正文**由
> [`agent-tools-spec.md`](agent-tools-spec.md) 负责。本文负责 harness 的**行为策略**。

## 快速理解

| 问题 | 结论 | 深入阅读 |
| --- | --- | --- |
| 模型调用失败怎么办？ | 按错误类别分流：退避重试 / 压缩后重试 / 立即失败 / 作为正常结果完成 | [§7](#7-turn-内循环与失败策略) |
| Agent 运行中用户发消息怎么办？ | durable 追加到当前 Turn；本地模型输出在安全点与 steer 原子仲裁，Codex 委托转发 exact `turn/steer` | [§8](#8-引导与并发输入) |
| 提示词怎么做？ | 四层：静态身份/策略 + per-profile 工具指导 + 会话冻结环境快照 + 工作区指令；动态走 append-only reminder | [§4](#4-提示词) |
| 工具选哪些？ | 共享七件套 + 按模型家族切编辑工具形态；逐工具规格见 tools-spec | [§5](#5-工具集) |
| 工具什么时候注册？ | Turn 接受时冻结；内置静态平铺，MCP 超阈值切检索式；不做运行时动态增删 | [§6](#6-工具注册时机) |
| 上下文怎么裁剪/压缩？ | 输入侧逐条限幅；历史语义单元保留；阈值用 `ModelInfo.effective_auto_compact_token_limit` | [§9](#9-上下文裁剪)、[§10](#10-压缩) |
| 缓存怎么搞？ | 前缀字节稳定 + append-only；`cache_control` 由 Anthropic adapter 注入 | [§11](#11-prompt-缓存) |
| 怎么知道 harness 变好了？ | 三层任务集 + token/成功率指标 + 前缀稳定性回归断言 | [§14](#14-评测) |

## 1. 现状差距

一个可用的 coding agent harness 需要的每个环节，对照 Zeta 当前实现：

| 环节 | 可用 harness 需要 | Zeta 现状 |
| --- | --- | --- |
| System prompt | 每次调用注入身份、策略、工具指导、环境 | 部分：`SYSTEM_PROMPT` 已经通过 `HarnessInstructions` 和 `ContextPlan` 注入；per-profile 工具指导仍未完成 |
| 环境上下文 | cwd、平台、日期、git 状态、项目指令 | ✅ Local Workspace host 在 model safe point 提供环境与 `.zeta/instructions` snapshot |
| 工具面 | 读/搜/改/执行闭环 | ❌ 只接了 `shell-command`；`file-system-tool`（仅 read/list/metadata）、`apply-patch`、`file-search` 是孤立 crate |
| 模型失败弹性 | 429/5xx 退避重试、溢出压缩重试、空响应处理 | ✅ 类型化错误、退避、单次溢出恢复、空响应重试和 Refusal 完成语义已接通 |
| Steering | 运行中排队注入用户消息 | ✅ typed command、receipt、delivery fact、App Server、Desktop、本地重规划和 Codex 转发均已接通 |
| 工具结果限幅 | 模型侧截断 + 保留头尾 | 部分：shell 有 256 KiB 执行上限，但无模型输入预算 |
| 上下文预算 | 窗口估算、溢出显式处理 | ✅ 已知/配置窗口走确定性预算；未知窗口明确退回 provider-managed |
| 压缩 | 阈值触发、durable checkpoint | ✅ `COMPACTION_PROMPT`、source digest、原子 commit、恢复校验与 commit 后重规划已接通 |
| Prompt cache | 前缀稳定 + 断点标注 | ❌ 只解析 `cache_read_input_tokens`，不写 `cache_control` |
| 多 Tool Call/响应 | 模型一次响应多个调用 | ❌ `parallel_tool_calls` 硬编码 `false` |
| 计划工具 | 长任务显式计划状态 | 部分：`ThreadItem::Plan` 存在，无工具产生它，assembler 跳过它 |
| 评测 | 任务集 + 指标回路 | ❌ 无 |

## 2. 一次模型调用的目标形态

```text
ModelRequest
├─ instructions                          ── 静态层（会话内不变，缓存友好）
│  ├─ 身份 + 指令优先级 + 安全策略          （zeta-prompts SYSTEM_PROMPT，静态）
│  ├─ 工具使用指导 + 输出风格               （per tool-profile，profile 内静态，
│  │                                        正文见 agent-tools-spec.md 附录 A）
│  ├─ Skill 清单                           （名称+一行描述，预算上限内，会话稳定）
│  └─ 环境快照                             （字段见 §4.2，Thread 首 Turn 冻结）
├─ input
│  ├─ [0] 工作区指令 message               （Global `.zeta/instructions`，§4.3，调用内冻结）
│  ├─ [1..] durable history                （append-only，语义单元完整，
│  │                                        可含 checkpoint summary 替代更早历史）
│  └─ [last] 当前 Turn 输入 / steering 消息（+ append-only reminder 块）
└─ tools                                  ── Turn 接受时冻结的 ToolProfile
```

三条硬规则，所有后续设计都从它们推导：

1. **前缀稳定**：`instructions`、`tools`、已有 history 在 Turn 之间逐字节不变，新内容只
   append（[§11](#11-prompt-缓存) 的前提）。
2. **语义单元完整**：Tool Call/Result 成对；一个 Turn 的内容原子保留或原子被 checkpoint
   吸收（[`core-context.md`](core-context.md) §6.3）。
3. **变化走安全点**：工具面、模型、策略、环境快照只在 Turn 边界或压缩边界变化。

## 3. Turn 内循环

一次 Turn 的执行循环（现有 `TurnExecutor::execute_steps` 的目标形态）：

```text
loop:
  1. 检查取消
  2. 读最新 durable snapshot（steering 消息在此自然进入）
  3. 预算检查：估算占用 ≥ 阈值 → 先压缩（§10），压缩失败且未超硬限 → 继续
  4. ContextManager.prepare → ContextPlan → ContextAssembler → ModelRequest
  5. 调用模型（含 §7 重试策略），流式 delta 经 sink 发布
  6. 响应分类：
     ├─ 有 Tool Call → durable 记录 → ToolScheduler 顺序执行（审批/沙箱/
     │                  升级语义见 core.md §11）→ 回到 1
     ├─ 只有文本   → durable 提交 agent message → Turn Completed
     ├─ Refusal    → 作为最终消息提交 → Turn Completed（不是失败）
     └─ 空响应     → §7 空响应处理
```

无固定迭代次数上限（runtime 文档决策）；失控防护见 [§7.3](#73-失控防护)。

## 4. 提示词

### 4.1 四层结构

| 层 | 内容 | 变化频率 | 存放位置 |
| --- | --- | --- | --- |
| 身份与策略 | 身份、指令优先级、注入防护、工作行为 | 随产品版本 | `zeta-prompts` `SYSTEM_PROMPT`（已存在） |
| 工具指导 + 输出风格 | 工具组合惯例、验证纪律、简洁度、路径引用格式 | 随 tool profile | `zeta-prompts` 新模板；正文已写好，见 [`agent-tools-spec.md` 附录 A](agent-tools-spec.md#附录-a系统提示词扩写正文) |
| 环境快照 | 见 §4.2 | Thread 首 Turn 冻结；压缩边界刷新 | host 提供 `AgentEnvironmentSnapshot`，Core 渲染 |
| 工作区指令 | Global `.zeta/instructions` | model invocation 内冻结；文件变化影响后续调用 | `input[0]` 独立 message |

外部产品如何组织项目指令只是参照系；Zeta 的原生 artifact、目录和加载策略由
[`agent-customizations.md`](agent-customizations.md) 定义。共同点是**静态与动态严格分离**——
这决定缓存命中率，比放 system 还是首条消息更重要。

### 4.2 环境快照：精确字段

```text
<environment>
working_directory: /abs/path
is_git_repo: true
platform: darwin | linux | windows
os_version: <uname/ver 摘要>
shell: zsh
today: 2026-08-03            ← 天级，不含时间
git_branch: main
git_main_branch: main
git_status: <porcelain 摘要，最多 40 行，超出标注截断>
git_recent_commits: <最近 5 条 oneline>
</environment>
This snapshot was taken at session start and does not update. Run commands
(e.g. `git status`) when you need current state.
```

冻结纪律：Thread 首 Turn 采集一次写死；压缩重建窗口是唯一例行刷新点；需要新鲜状态时模型
自己调工具。**不逐 Turn 刷新**——那会击穿全部缓存。

### 4.3 Workspace Instruction 发现与注入

- 发现：只读取 workspace root 的 `.zeta/instructions/*.md` 原生文件；`AGENTS.md` 和其他生态
  格式必须经 `zeta-agent-import`，native loader 不兼容扫描；
- 注入：当前只把 `load: global` 条目渲染为 `input[0]` user message，并标注其优先级低于
  system 与安全策略；
- 大小：每个文件最多 32 KiB、直接条目最多 128，非法条目产生隔离 diagnostic；
- `load: contextual` / `on-demand`：catalog 已保留类型化策略，但资源匹配和显式选择尚未实现；
- 目录不存在或没有合法 Global 条目：省略 `input[0]`，不放占位符；
- 文件变化：Workspace watcher 触发 catalog refresh；已经组装的 model request 不变，后续
  model invocation 从 `HarnessInstructionsProvider` 读取新 snapshot。

### 4.4 动态注入：append-only reminder

运行时事件（策略变化、文件被外部修改、计划停滞、按需 Instruction、检索到的 MCP
工具定义）以 `<system-reminder>` 文本块附着在**新增**内容上进入历史，永不回改已有消息。
reminder 声明自己是背景信息而非用户指令。这是 Skill 激活、hook 输出、审批结果回填的统一
入口。

## 5. 工具集

### 5.1 设计原则

1. **匹配训练分布**：Anthropic 系对 str-replace Edit/Glob/Grep 有深度训练，OpenAI 系对
   `apply_patch`（V4A）有专门训练。多 provider 的 Zeta **必须按模型家族分 profile**。
2. **结构化优于 shell 万能**：结构化工具才能给 `zeta-action-policy` 精确的审查材料（参数级
   capability、路径级沙箱判定），审批 UX 与 diff 展示也依赖结构。
3. **工具描述就是提示词**：进入每次调用的 tools 前缀，与 system prompt 同级打磨。
4. **少而精**：v1 ≤ 10 个。

### 5.2 工具面与配置档案

```text
ToolProfile = 共享核心 + 家族特定编辑工具

共享核心：shell / read_file / write_file / glob / grep / update_plan
家族特定：
  anthropic / google / 默认 → edit（str-replace + 唯一性）
  openai                    → apply_patch（V4A）
MCP（按 §6 阈值）：直接平铺 或 search_tools + call_mcp_tool
```

逐工具规格（schema、描述正文、校验、错误文案、限幅、capability 注记）：
[`agent-tools-spec.md`](agent-tools-spec.md)。

Profile 解析发生在 Turn 接受安全点：`ModelSelection` 解析目标模型 → 家族→profile 映射
（属 `model-provider-config` 声明层）→ 冻结进本 Turn。Core 不认识"模型家族"。跨 Turn 换
模型后，历史中另一 profile 的 Tool Call/Result 只是 transcript，无需处理；只有当前可调用
集合须与 profile 一致。

`parallel_tool_calls` 改为 profile 属性：允许模型一次响应发多个 Tool Call（执行侧仍按现有
调度器串行），Anthropic/OpenAI profile 默认开。

## 6. 工具注册时机

| 方案 | 结论 |
| --- | --- |
| 全量平铺（Codex 式） | ✅ 内置工具 + 小规模 MCP 采用 |
| 检索式（Claude ToolSearch 式） | ✅ MCP 超阈值采用 |
| 运行时动态增删 | ❌ 不采纳：打破 Turn 冻结、每次变化击穿缓存、审批/恢复无法绑定稳定工具面 |

规则：

1. 内置工具静态平铺，Turn 接受时随 profile 冻结；
2. MCP 聚合定义 ≤ 15 个工具且 ≤ 5k tokens 时平铺进冻结集；超阈值整体切检索式——注入
   `search_tools` + `call_mcp_tool` 两个元工具，被发现的定义以 reminder append 进历史
   （不改 tools 数组，保前缀）；
3. 配置/MCP server 变化走 [`tools.md`](tools.md) §7 的 registry snapshot，只在 Turn 边界
   推进。

## 7. Turn 内循环与失败策略

### 7.1 模型调用错误分类与处理

| 错误 | 判定依据 | 处理 |
| --- | --- | --- |
| 限流 | HTTP 429 | 退避重试；优先遵循 `Retry-After`（上限 60s） |
| 过载/服务端错误 | HTTP 5xx、529 | 退避重试 |
| 传输失败 | `ApiError::Transport`（超时/断连） | 退避重试 |
| 上下文溢出 | 供应商错误体解析 | 完整旧历史前缀持久化压缩后，以新快照重试一次；再次溢出以 `contextOverflow` 稳定失败 |
| 认证失败 | HTTP 401/403 | 不重试；Turn fail（stable error `providerAuth`），提示用户检查凭据 |
| 无效请求 | HTTP 400 / `InvalidRequest` | 不重试；Turn fail（stable error `invalidRequest`）；有界原始详情只进入受控日志 |
| 无效响应 | `InvalidResponse` | 重试 1 次（可能是瞬时截断）→ `invalidResponse` |
| 空响应 | 无文本、无 Tool Call、无 Refusal | 重试 1 次（同一请求）→ 仍空则 Turn fail（stable error `model_empty_response`） |
| Refusal | `ResponseItem::Refusal` | **不是错误**：作为最终消息提交，Turn Completed |
| 取消 | token 触发 | 传播；Turn → Interrupted |

退避参数：基数 1s、倍率 2、上限 30s、抖动 ±25%、**最多 4 次尝试**（1 次初始 + 3 次重
试）。退避等待期间以 ≤100ms 粒度轮询 cancellation token（或用带超时的 condvar），interrupt
必须立即生效。重试不跨越安全边界：只重试**未产生任何 durable 副作用**的模型调用本身；流式
场景下已发布的 transient delta 通过新 stream incarnation 作废（runtime 文档 §4.2）。

### 7.2 工具执行失败

工具失败默认是**正常结果**（`is_error: true` 的 Tool Result 回给模型继续处理），不使 Turn
失败——已是现状。政策拒绝的 circuit breaker（连续拒绝中断 Turn）已实现。

### 7.3 失控防护

- **重复失败调用**：同一工具 + 相同参数 digest 连续失败 ≥3 次 → 在下一条 Tool Result 附 reminder（"stop repeating this exact call; change approach or report the blocker"）；≥5 次 → Turn fail（stable error `tool_repetition`）。计数按 Turn 内连续窗口，成功即清零。
- **Turn 资源上限**：durable usage 记账（§9.3）之上可配置 per-Turn token/成本上限，超限
  在下一个安全点终止（stable error `turn_budget_exhausted`）。v1 默认不设限，仅记账。
- 无迭代次数硬上限（与 runtime 文档一致：上限应由可取消的资源策略表达，不用进程内计数器）。

### 7.4 当前接线与剩余恢复

- **已实现**：`ApiError` 分类 `RateLimited { retry_after_ms }`、`Overloaded`、`ContextOverflow`、`AuthFailed`、`InvalidRequest` 和 `InvalidResponse`；HTTP/SSE 适配器从状态码和 OpenAI、Anthropic、Google 错误体映射，`ModelProviderError` 与 `CoreError` 透传类别。
- **已实现**：重试循环位于 `ModelService` 之上的执行器；认证与无效请求不重试，无效响应只重试一次，瞬时错误保留类型化 `Retry-After`。
- **已实现**：`ContextOverflow` 触发一次 durable compaction；`ContextOverflowRecoveryCommitted` 把 checkpoint 与 Turn 级恢复标记原子提交，执行器随后从新 snapshot 重试一次；再次溢出保持 `contextOverflow`。

## 8. 引导与并发输入

Agent 运行中用户发来新消息：**durable 排队 + 下一个安全点注入当前 Turn**。Steer 不取消模型
或工具 I/O；模型响应回到 Core 时与 steer 原子仲裁，已经过期的响应整体丢弃并用最新 snapshot
重规划；正在执行的工具先按既有 once-only 规则收口，再由下一轮读取 steer。

```text
session/request::SteerTurn { command_id, expected_sequence, thread_id, turn_id, input }
→ 校验 Turn 处于 Running / WaitingForApproval / WaitingForUserInput
→ 原子提交 UserMessage/UserImage Item + TurnSteered + command receipt
→ execution backend 接受：local 读取 canonical snapshot；Codex 转发 exact remote turn/steer
→ durable TurnSteerDelivered
→ RPC 重试只回放 receipt/delivery result，不重复发送外部 steer
```

- **已实现**：`ThreadCommand::SteerTurn`、`TurnSteered`、`TurnSteerDelivered`、Core reducer、
  App Server `session/request`、Desktop Send/Stop 双动作、本地 executor 与 Codex adapter 已形成纵向切片；
- 模型调用进行中：消息先落盘；如果 steer 在该 invocation 的 source sequence 之后提交，reasoning、
  文本和 Tool Call 不会部分落盘，整个旧响应被丢弃后立即重规划；不自动 cancel provider 请求；
- 委托执行：`TurnSteered` 是外部副作用前 marker，upstream ack 后才写 delivery fact；进程或传输
  在两者之间失败时把结果视为 unknown，不自动重发同一 Codex steer；
- WaitingForApproval 期间：允许 steer，恢复执行后可见；
- Cancelling/终态：拒绝（稳定错误），客户端引导开新 Turn；
- 多条 steer 按提交顺序进入历史；append-only，缓存友好；
- Turn 处于 Idle（无运行 Turn）时 steer 无意义：返回错误，客户端用 `session/request` StartTurn。

## 9. 上下文裁剪

### 9.1 输入侧逐条限幅

进入模型的每条内容在**选入时**限幅（执行侧上限保护进程与存储，模型侧限幅保护窗口预算，
两者分开；数值是起点参考值，由 §14 评测调）：

| 内容 | 限幅 | 方式 |
| --- | --- | --- |
| shell 结果 | 30 KiB | 头尾各半，中间标注截断字节数 |
| read_file | 2000 行 / 行内 2000 字符 | 尾部提示用 offset 继续 |
| grep / glob | 100 条 | 标注总命中数 |
| MCP 工具结果 | 25 KiB | 同 shell |
| 图片 | 按 provider 上限降采样 | adapter 层 |

### 9.2 历史选择：不做静默滑窗

- 预算内全量保留；接近预算触发压缩（§10），更早历史被 checkpoint 原子吸收；
- 永不静默删除：当前 Turn 输入、安全约束、未完成 Tool Call/Result 组
  （[`core-context.md`](core-context.md) §7.3 的显式 overflow outcome）。

### 9.3 预算来源与 usage 校准

当前实现：

- 窗口与阈值来自 `ModelInfo`，或用户按模型 ID 配置的
  `ModelProviderConfig.model_context`；自动压缩阈值不超过窗口的 90%；
- `ContextWindow::Unknown` 不猜 128k，而是明确使用 `ContextBudget::ProviderManaged`；
- `zeta-context-engine` 已统一压力线、模型硬窗口、精准计量与带保守记账余量的估算结果；
- 生产 planner 仍由 `deterministic-bytes-v1` 以 bytes/4 加结构开销做确定性估算，并在诊断中记录
  revision；最终 request 接近压力线或 compaction 后会调用声明式 model binding 对应的 remote
  preflight：OpenAI exact，Anthropic、Google、Kimi、Z.AI estimated；本地 tokenizer adapter 尚未
  接入。

usage 按 Thread 持久化和基于 provider usage 的 EMA 校准尚未实现；在此之前不能把粗估写成精确
tokenizer 保证。

## 10. 压缩

### 10.1 触发

- **自动（已实现）**：估算历史超过 Core-managed input budget → 下一个 model safe point 先压缩、
  durable commit，再从新 snapshot 重规划；
- **手动（Proposed）**：`/compact` 与用户保留提示尚未实现；
- **溢出恢复（已实现）**：provider `ContextOverflow` 会压缩全部可安全吸收的 terminal 历史前缀；checkpoint durable commit 后重试一次，当前 Turn 与未完成工具组不被吸收。

### 10.2 流程与精确规则

机制（checkpoint schema、digest、失效回退）归 [`core-context.md`](core-context.md) §8。
当前规则：

- **tail 选择**：从最新 Turn 向前按完整 Turn 单元保留；当前 Turn 无条件保留，checkpoint 只
  覆盖 durable 前缀；
- **压缩调用**：独立模型调用使用被吸收前缀、上一个 checkpoint 和 `COMPACTION_PROMPT`；不带
  tools，summary target 在剩余预算内有界；
- **分批处理**：压缩请求本身也必须装入同一模型窗口。过长前缀分批提交 checkpoint；若单个
  新 Turn 连同 compaction envelope 都无法装入，则返回 `CompactionSourceTooLarge`，不循环重试；
- **失败路径**：空 summary、Tool Call、超目标 summary、source digest/Item/range 不一致或 store
  commit 失败都会失败即关闭；未 commit 的 summary 不进入 projection。

热文件重注入和用户定向压缩属于 Proposed，当前不会在压缩过程中重新读取 Workspace 文件。

### 10.3 窗口重建

```text
instructions（下一个 model safe point 重新冻结）
+ 工作区指令（mandatory fragment）
+ checkpoint summary
+ tail 原文
```

## 11. Prompt 缓存

### 11.1 三家机制

| Provider | 机制 | 要求 |
| --- | --- | --- |
| Anthropic | 显式 `cache_control` 断点（≤4），默认 5 分钟 TTL，读 ≈0.1× / 写 ≈1.25× | 前缀逐字节一致；断点前 ≥ 最小 token 数（约 1024） |
| OpenAI | 自动前缀缓存（≥1024 tokens） | 无需标注，前缀逐字节一致 |
| Google | 隐式（2.5+）+ 显式 cache API | 同上 |

### 11.2 组装硬约束

1. 序列化确定性：工具列表顺序固定（内置按 profile 声明序 → MCP 按 server+name 排序）；
2. append-only（§2 规则）；reminder 附着新内容；
3. 前缀不放每次变化的值（时间戳、实时 git status——§4.2 冻结纪律）；
4. 压缩、换 profile、换模型必然击穿缓存——接受为一次性成本。

### 11.3 落点

`cache_control` 不进 canonical `ModelRequest`，由 `zeta-api` 的 `anthropic_messages` 构造器
注入：断点 1 = tools 末尾；断点 2 = system 末尾；断点 3 = 倒数第二条 user 消息末尾（滚动，
命中上一轮全部历史）。观测：`cached_input_tokens` 已解析；**同一 Thread 连续 Turn 命中率
应接近 100%，低于即组装层引入了前缀抖动，按 bug 处理**（§14 的回归断言）。

## 12. Skills 与 slash commands

- **slash commands**（`zeta-rs/slash-commands` 已有 catalog）：在 `session/request` StartTurn 之前由
  App Server 展开，展开后的正文作为 durable UserMessage 进入 Turn 输入；消息内保留
  `<command-name>` 标注供模型识别来源。不在模型侧解析斜杠语法。
- **Skills**（`zeta-rs/skills` 已有 runtime；发现/信任归 [`skills.md`](skills.md)）：
  - v1 已实现：用户提交 exact `SkillRef`，App Server 冻结 digest/generation/reason，Core 在每个
    model safe point 重载 exact `SKILL.md`，以 `ActivatedSkill` instruction layer 注入；raw path
    输入被拒绝；
  - 自动候选检索（推迟）：只注入被 selector 激活的 Skill，不能把整个 catalog 正文或清单随
    catalog 数量线性塞进 prompt。

## 13. 供应商差异矩阵

canonical 层（`ModelRequest`）保持 provider 中立，差异全部压进 `zeta-api` 两个请求构造器。
authoring 规则以最严格交集为准：

| 维度 | Anthropic Messages | OpenAI Responses | canonical 规则 |
| --- | --- | --- | --- |
| instructions | `system` 字段 | `instructions` 字段 | 单值 `instructions`，adapter 映射 |
| 角色 | user/assistant | + developer | v1 canonical 只产生 System/User/Assistant；`Developer` 保留给 OpenAI 路径未来用 |
| 工具 schema | `input_schema`，无 strict | strict 模式：`additionalProperties:false` + 全字段 `required` | **按通用子集 authoring**（[`agent-tools-spec.md` §1](agent-tools-spec.md#1-schema-约定)）：顶层 object、全 required、可选性用 `["T","null"]`，两边同一份 schema 直接可用 |
| tool_choice | auto/any/tool | auto/required/function/none | 现有 `ToolChoice` 已覆盖 |
| 并行 Tool Call | 支持（`disable_parallel_tool_use`） | 支持（`parallel_tool_calls`） | profile 属性透传 |
| Tool result | user 消息内 `tool_result` block | `function_call_output` item | assembler 已配对，adapter 负责 wire 形态 |
| Tool call id | `tool_use.id` 回传 | `call_id` 回传 | `ToolCallId` 原样往返，adapter 不改写 |
| 图片 | base64 source block | data URI / URL | `ContentPart::ImageUrl` 承载，adapter 转换（已实现） |
| 缓存 | 显式断点（§11.3） | 自动 | adapter 差异，canonical 无感 |
| reasoning | thinking + budget_tokens | `reasoning.effort` | `ReasoningConfig.effort` 映射：Anthropic 按档位换算 budget；v1 不把 reasoning 内容回灌历史（assembler 已跳过） |
| 空响应/拒绝 | `stop_reason` + 空 content | `refusal` item | 统一映射 `ResponseItem::Refusal` / 空响应走 §7.1 |
| 错误分类 | `overloaded_error` 等错误体 | `context_length_exceeded` 等 code | 映射进 §7.4 的 `ApiError` 新分类 |

## 14. 评测

harness 的每层都直接影响成功率与 token 成本；没有评测回路，§9/§10 的所有参考值无法调优。

### 14.1 任务集

仓库内 `evals/harness/` 下的封闭夹具，每题 = 冻结的小型 git 仓库 + 任务提示 +
`verify.sh`（退出码 0 = 成功）：

| 层 | 数量 | 内容 | 考察 |
| --- | --- | --- | --- |
| T1 | 10 | 单文件 bug 修复 | 工具基本功、编辑正确率 |
| T2 | 5 | 跨文件小功能 | 搜索/多文件编辑/验证纪律 |
| T3 | 2 | 长会话（>30 次工具调用） | 计划工具、失控防护 |
| T4 | 2 | 强制压缩任务（低阈值配置） | 压缩后连贯性（约束保留、不重做已完成工作） |

### 14.2 指标

每次运行记录：成功与否、input/cached/output tokens、工具调用次数、墙钟时间。
按 profile × provider 出对比表。

### 14.3 运行方式

- **PR smoke（无真模型）**：组装快照测试——固定 Thread 夹具跑 assembler，断言
  （a）请求字节稳定（连续 Turn 前缀不变，§11 回归）；（b）限幅与结构不变量；用
  deterministic fake `ModelService` 跑 T1 子集的 loop 行为（重试、steering、循环终止）；
- **nightly（真模型）**：全任务集 × 主力 profile（anthropic / openai 各一），指标入库
  看趋势；
- M0–M6 每个里程碑的验收 = 对应任务层在接线前后的指标对比。

## 15. 落地顺序

M0–M6 只表示本文行为规格的覆盖状态，不再承担实际构建顺序。后续工作项、依赖、优先级和验收门由 [`agent-harness-implementation-plan.md`](agent-harness-implementation-plan.md) 的 S1–S7 唯一拥有。

| 里程碑 | 内容 | 关键改动点 | 前置接线 |
| --- | --- | --- | --- |
| M0（基本完成）提示词接线 | SYSTEM_PROMPT、环境快照、Global `.zeta/instructions`、稳定组装与工具指导已接线；家族 profile 指导随 M1 收口 | `ContextAssembler`、host 环境快照、`WorkspaceCustomizations` | 无 |
| M1（部分具备）工具最小闭环 | 当前工具面已能完成 coding；仍需统一文件工具 ownership、家族 ToolProfile、`update_plan`、逐项限幅和 T1/T2 | 本地工具组合、executor contributions、profile 声明层 | ToolProfile 冻结 contract |
| M2（部分具备）失败弹性 + steering | Provider 错误分类、退避、空响应、Refusal、overflow 恢复和 steering 已实现；仍需重复失败熔断 | executor 重试层、Thread command、App Server protocol | protocol/schema/Desktop 同批同步 |
| M3（部分具备）限幅/预算/压缩 | ContextPlan、配置窗口、preflight 与 durable compaction 已实现；仍需逐项限幅、usage/cost 账本、资源预算、手动压缩和 T4 | ContextPlan 选入路径、checkpoint、usage 持久化 | usage durable fact |
| M4（部分具备）缓存 | 请求组装已有字节稳定基线且 cached usage 已解析；仍需 Anthropic cache breakpoint、命中观测和 Provider 回归 | `anthropic_messages` adapter、组装 fixture | 无 |
| M5（部分具备）MCP 策略 | registry snapshot、deferred exposure 与 tool search 已实现；仍需 ≤15/≤5k 平铺阈值和超阈值整体检索式 contract | MCP registry 之上的冻结暴露策略 | ToolProfile contract |
| M6（部分具备）Skills/slash | slash、explicit SkillRef、frozen activation、`skills-read` 和 Desktop 显式选择已接通；仍需受信任自动 selector | App Server 展开、Skill metadata selector、ActivatedSkill layer | 评测与信任策略 |

当前已经具备“接入已配置模型即可 coding”的最小闭环；后续目标是将该闭环收口为可观测、可恢复、跨 Provider 一致的产品能力。实施时必须按 [`agent-harness-implementation-plan.md`](agent-harness-implementation-plan.md) 的工作项 ID 和完成纪律更新状态。

## 16. 参考

- [`agent-tools-spec.md`](agent-tools-spec.md) — 逐工具规格与提示词正文
- [`zeta-agent-runtime-architecture.md`](zeta-agent-runtime-architecture.md) — 执行内核与阶段计划
- [`core-context.md`](core-context.md) — ContextPlan / checkpoint 机制
- [`tools.md`](tools.md) — 工具三层契约与 registry snapshot
- [`skills.md`](skills.md) / [`slash-commands` crate](../zeta-rs/slash-commands/) — 扩展来源
- [`zeta-prompts` README](../zeta-rs/prompts/README.md) — 提示词资产 ownership
- Claude Code harness 行为（Edit 唯一性、Read 限幅、system-reminder、压缩后文件重注入）
- Codex harness 行为（apply_patch V4A、AGENTS.md、environment_context、update_plan）
