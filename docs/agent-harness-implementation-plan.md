# Coding 最小闭环实施计划（M0+M1+基础弹性）

> 状态：Planned（2026-08-03）
> 目标：完成后**接入任意已配置的模型即可执行真实 coding 任务**——理解仓库、搜代码、
> 读代码、改代码、跑命令、要审批、被打断。
> 规格依据：[`agent-harness-design.md`](agent-harness-design.md)（行为策略）、
> [`agent-tools-spec.md`](agent-tools-spec.md)（schema/描述/错误文案，本文不重复）。
> 本文只负责：文件级改动点、顺序依赖、每项验收。

## 0. 范围

**做**（对应 harness 文档 M0 + M1 + M2 最小子集）：

| 工作项 | 内容 |
| --- | --- |
| WI1 | 提示词接线：instructions 注入 + 环境快照 + AGENTS.md |
| WI2 | read_file / write_file / edit 三工具 |
| WI3 | grep / glob 两工具（均基于已管理的 ripgrep） |
| WI4 | 本地工具组合改造：单工具 → 多工具套件分发 |
| WI5 | `parallel_tool_calls` 放开 + 工具描述接入 |
| WI6 | 模型调用基础弹性：429/5xx 退避、空响应重试 |

**明确不做**（后续里程碑，不阻塞"能 coding"）：`apply_patch` 与 ToolProfile 机制（v0 单
profile 用 edit，接 OpenAI 系时补）、`update_plan`、MCP 阈值策略、压缩、prompt cache、
`turn/steer`、嵌套 AGENTS.md。

## 1. WI1 提示词接线

### 1.1 `zeta-rs/prompts`

- `templates/system/base.md`：追加 [`agent-tools-spec.md` 附录 A](agent-tools-spec.md#附录-a系统提示词扩写正文)
  的 A.1（工具指导共享段）+ A.2 edit 变体 + A.3（输出风格）。v0 单 profile，不拆
  per-profile 模板；
- `src/system.rs`：bump `SYSTEM_PROMPT` revision；
- 按 crate README 修改清单同步 `prompt_tests.rs`（正文非空/trailing newline 已覆盖，无新增）。

### 1.2 `zeta-rs/core`

- 新增 `src/context/instructions.rs`（private module）：

  ```text
  HarnessInstructions {
      system_body: String,          // SYSTEM_PROMPT body（含工具指导与输出风格）
      environment: String,          // 渲染后的 <environment> 块
      workspace_instructions: Option<String>,  // AGENTS.md 正文（≤32 KiB，超出截断标注）
  }
  ```

  渲染规则按 [`agent-harness-design.md` §4.2](agent-harness-design.md#42-环境快照精确字段)：
  environment 块字段全集 + 冻结声明文案。
- `src/context/assembler.rs`：`assemble(snapshot, tools)` →
  `assemble(snapshot, tools, instructions: &HarnessInstructions)`：
  - `ModelRequest.instructions = Some(system_body + "\n\n" + environment)`；
  - `workspace_instructions` 存在时作为 `input[0]` user message，正文包裹
    "工作区指令，优先级低于系统与安全策略"标注（文案照 harness §4.3）；
  - `parallel_tool_calls: true`（WI5，顺手在此改）。
- `src/turn/executor.rs`：`TurnExecutor` 增加
  `with_instructions(Arc<HarnessInstructions>)` builder；未设置时用空 environment 的默认
  值（向后兼容现有测试）；`execute_steps` 调 assemble 时传入。
- `lib.rs`：导出 `HarnessInstructions`（host 构造它）。

### 1.3 `zeta-rs/app-server`

- `src/local.rs`（workspace runtime 组装处）：新增环境采集函数——cwd、platform、
  OS 版本（`uname -r` 级）、shell、日期（天级）、`git branch --show-current` /
  `git status --porcelain`（≤40 行）/ `git log --oneline -5`，git 命令失败按非 git 仓库
  处理；读 workspace root 的 `AGENTS.md`；构造 `HarnessInstructions` 注入 `TurnExecutor`。
  采集只在 workspace runtime 创建时执行一次（冻结纪律）。

### 1.4 验收

- assembler 单测：instructions 注入、`input[0]` 存在/缺省两分支、AGENTS.md 截断标注；
- **字节稳定测试**：同一 snapshot 连续 assemble 两次，序列化结果逐字节相等（M4 缓存回归
  的基线，现在就建）；
- 手动冒烟：CLI 连本地 App Server + 真实 provider，问"这个仓库是干什么的"，模型应引用
  AGENTS.md 与 git 状态回答。

## 2. WI2 read_file / write_file / edit

### 2.1 `zeta-rs/file-system-tool`

现状是单个 operation-enum 工具（read/list/metadata，64 KiB 字节上限）。改造为三个模型可见
工具（schema/描述/错误文案全部照 [`agent-tools-spec.md`](agent-tools-spec.md) §3–§5）：

- 新增 `src/read.rs`：行式读取（替代 64 KiB 字节截断）——2000 行默认、offset/limit、
  单行 2000 字符截断、`cat -n` 行号、空文件与二进制/图片分支（v0 图片可先返回
  "binary file" 错误，图片 content 留到 provider 图片链路验证后）；
- 新增 `src/write.rs`：全量写 + 父目录创建 + 已读前置校验；
- 新增 `src/edit.rs`：唯一命中校验（0 命中 / 多命中 / old==new / 读后外部修改四分支错误
  文案）+ `replace_all` + 成功输出 ±4 行带行号片段；
- 旧 operation-enum 工具保留给现有消费者，标记 deprecated，agent 路径不再暴露。

### 2.2 已读文件集（edit/write 前置）

- `zeta-rs/core/src/services.rs`：`ToolService::execute` /`execute_streaming` 增加
  `facts: &ToolExecutionFacts` 参数：

  ```text
  ToolExecutionFacts { read_paths: BTreeSet<PathBuf> }
  ```

- `src/turn/tool_scheduler.rs`：执行前从 `ThreadSnapshot` 推导——历史中成功的 read_file
  Tool Result 提取路径（解析 Tool Call arguments 的 path 字段）；
- 前置违规在 `execute` 内返回 `is_error` Tool Result（不在 prepare 阶段报错——prepare
  失败会 fail Turn，而前置违规是模型可自纠的正常失败）；
- 波及面：`NoTools`、MCP 适配器、所有测试 fake 的签名同步（机械改动）。

### 2.3 验收

- 三工具各自的单测覆盖 tools-spec 错误文案表的每一行；
- 集成测：fake model 发起 read → edit → 验证文件变更 durable 落盘；未读先改被拒且 Turn
  不失败。

## 3. WI3 grep / glob

两者都基于已被 App Server 发现和管理的 `RipgrepExecutable`
（`zeta-rs/shell-command/src/ripgrep.rs`），**不扩展 `file-search` crate**（它是 TUI 模糊
搜索，语义不同）：

- 新增 `zeta-rs/app-server/src/local_tools/grep.rs`：`rg -n --no-heading` + glob/大小写参
  数映射，100 条与 500 字符/行限幅，正则错误透传 + 转义提示文案；
- 新增 `local_tools/glob.rs`：`rg --files --glob <pattern>`，结果按 mtime 降序，100 条
  限幅；
- 两者 capability 为只读 + workspace 路径域，沙箱兼容（`SandboxCompatibility` 按现有
  shell 只读判定复用）。

验收：单测（限幅、排序、gitignore 默认遵守、非法 pattern 文案）。

## 4. WI4 本地工具组合改造

`zeta-rs/app-server/src/local_tools.rs` 现状是 `LocalShellToolService` 单工具（definitions
返回一个、prepare/materialize 按单一 name 校验）。改造：

- 新增 `local_tools/suite.rs`：`LocalToolSuite` 持有 shell / read_file / write_file /
  edit / grep / glob 六个成员，`definitions()` 聚合（顺序 = tools-spec 章节序，固定——
  这是缓存前缀稳定性的一部分）；`prepare` / `execute` 按 `call.name` 分发；
- 每个工具的 `ActionReviewRequest` 构造：read/grep/glob → 只读 capability + 路径；
  write/edit → 写 capability + 精确路径（喂给 `zeta-policy` 做审批/沙箱判定，复用现
  有 shell 的 provenance/digest 模式，`source_id` 按工具名区分）；
- `LOCAL_POLICY_REVISION` bump（`local-shell-v2` → `local-tools-v3`）；
- `compose_local_tools` 返回签名不变，内部换 suite。

验收：现有 `local_tools_tests.rs` / `tool_composition_tests.rs` 迁移 + 新工具的
prepare→policy→execute 路径各一条；审批弹窗场景（写文件首次要求批准）手动验证。

## 5. WI5 描述与并行调用

- 工具描述/schema 常量：各工具实现文件内以 `const` 持有 tools-spec 的正文（描述即提示词，
  改动走 tools-spec 附录 B 的修改清单）；
- `parallel_tool_calls: true` 已在 WI1 assembler 改动中带入；调度器保持串行执行（模型可
  一次发多个调用，执行顺序 = 调用顺序，现有 `next_pending_call` 循环天然支持——确认
  多 Tool Call 消息的 durable 记录顺序测试即可）。

## 6. WI6 模型调用基础弹性

范围是 [`agent-harness-design.md` §7.1](agent-harness-design.md#71-模型调用错误分类与处理)
的最小子集（429/5xx/传输错误退避 + 空响应重试；溢出→压缩链路属 M3 不做）：

- `zeta-rs/zeta-api/src/error.rs`：`ApiError` 增加
  `RateLimited { retry_after_ms: Option<u64> }` 与 `Overloaded`；两个请求构造器从
  HTTP 429/5xx/529 + `Retry-After` 头映射（`HttpStatus` 保底不删）；
- `zeta-rs/model-provider` / `zeta-rs/core`：错误类别透传（`CoreError` 增加
  `ModelTransient(String)` 类别，adapter 层映射）；
- `zeta-rs/core/src/turn/executor.rs`：模型调用点外包重试环——基数 1s、倍率 2、上限
  30s、抖动 ±25%、最多 4 次尝试；退避等待以 ≤100ms 步长轮询 cancellation；仅
  `ModelTransient` 触发；空响应（无文本/无 Tool Call/无 Refusal）同请求重试 1 次后按现
  有失败路径走；
- Refusal 分支：现状空响应与 refusal 都走 `response_failure_message` 失败——改为
  `ResponseItem::Refusal` 单独分支，作为最终 agent message 完成 Turn（harness §7.1）。

验收：fake `ModelService` 注入 429×2→成功、5xx×4（耗尽失败）、空→成功、Refusal 完成、
退避中 interrupt 立即取消，五条并发/时序测试。

## 7. 顺序与依赖

```text
WI1 提示词（独立，先行——一天级，立刻可感知）
   │
WI4 组合改造 ──► WI2 文件三工具 ──┐
   │            WI3 grep/glob ────┤──► WI5 描述收口 ──► 端到端验收
WI6 弹性（独立，可并行）──────────┘
```

- WI4 先于 WI2/WI3（套件骨架就位，工具逐个挂入）；
- WI2 的 `ToolService` 签名变更尽早合（波及面大、纯机械）；
- WI6 与工具线完全无关，可并行。

## 8. 端到端验收

全部合入后，用真实 provider 手动跑三个场景（同时作为
[`agent-harness-design.md` §14](agent-harness-design.md#14-评测) 任务集 T1/T2 的首批夹具）：

1. **修 bug**：夹具仓库一个单文件 bug，提示"tests 里有一个失败，修掉它"——期望：
   grep 定位 → read → edit → shell 跑测试 → 报告，全程无人工纠偏；
2. **跨文件小功能**：加一个函数并在两处调用——期望 glob/grep 探索 + 多文件 edit；
3. **权限路径**：让它写一个新文件——期望首次写触发审批，批准后完成，拒绝后 agent 报告
   而非重试。

回归门（每个 WI 合入都跑）：

```bash
cargo fmt --manifest-path zeta-rs/Cargo.toml --all -- --check
cargo clippy --manifest-path zeta-rs/Cargo.toml --workspace --all-targets -- -D warnings
cargo test --manifest-path zeta-rs/Cargo.toml --workspace
```

（本计划无 protocol schema 变更——`turn/steer`、checkpoint 等都在范围外，因此不涉及
`pnpm verify:protocol` 之外的客户端同步。）

## 9. 风险与开放点

| 风险 | 处理 |
| --- | --- |
| `ToolService` 签名变更波及 MCP 适配器与全部测试 fake | 纯机械；单独一个提交先行合入 |
| 行式 read 与现有 64 KiB 字节上限的关系 | 新 read.rs 独立实现行式；旧 operation 工具不动 |
| git 环境采集在超大仓库慢（status 全量） | `--porcelain` 截断 40 行 + 3s 超时，超时省略该字段 |
| OpenAI 系模型在 edit 工具上的正确率 | 已知取舍（v0 单 profile）；接 OpenAI 主力模型前补 apply_patch + profile |
| Refusal 语义改变现有失败测试 | executor 测试同步改（原空响应失败测试拆成 空响应/Refusal 两条） |
