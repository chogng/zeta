# `zeta-prompts`

> 本 README 说明内置模型提示词的实现 ownership 和修改契约。上下文选择、instruction precedence、
> budget 与 compaction 生命周期的跨 crate canonical 设计见 [`docs/core-context.md`](../../docs/core-context.md)。

## 快速理解

`zeta-prompts` 只保存 Zeta 自带的模型可见提示词，并为每个资产提供稳定 ID 和 revision。它不决定
提示词何时注入；需要某类提示词的调用模块负责触发条件，并把资产交给 Core 的上下文流程。

| 提示词类别 | 当前资产 | 注入时机 owner | 当前状态 |
|---|---|---|---|
| System | `SYSTEM_PROMPT` (`system-v4`) | Agent invocation/context caller | ✅ 已通过 harness/context 注入；包含 canonical 编辑工具和计划工具 guidance |
| Compaction | `COMPACTION_PROMPT` | Core `ContextCompactionService` | ✅ 已接入 durable checkpoint 流程 |
| Goals | `GOALS_PROMPT` + `render_goals_prompt` | Goal lifecycle caller | ✅ 已接入 Core Thread Goal；按 active Goal 注入目标与累计 token 快照 |
| Review | `REVIEW_PROMPT` | Review caller | 已具备通用 review 资产 |

`zeta-auto-review` 的专用审查 prompt 不属于这里：它必须和 response schema、`review-protocol-3`
revision 绑定，继续由 [`zeta-auto-review`](../auto-review/README.md) 拥有。

## 所有权

本 crate 拥有：

- `templates/` 下四类 compile-time prompt body；
- `PromptCategory`、`PromptArtifact` 及资产的稳定 ID/revision；
- `GoalPromptContext`、`GoalBudget` 和带源 revision 的 `RenderedPrompt`；
- 资产的空内容、唯一身份和末尾换行测试；
- Cargo/Bazel 对嵌入资源的编译依赖。

本 crate 不拥有：

- `ThreadSnapshot`、ContextPlan、history 选择或 Tool Call/Result 配对；
- instruction precedence、token budget、compaction checkpoint 或 durable history；
- live Config、Skill/MCP filesystem discovery、credential、model/provider client；
- provider-specific message role、wire JSON 或 API 请求；
- UI 文案、审批弹窗和 shell prompt；
- 外部 Skill、workspace instruction 与 MCP Prompt 正文的信任背书。

依赖方向应保持为：

```text
prompt consumer / zeta-core Context pipeline
                    │
                    ▼
              zeta-prompts
                    │
                    ▼
        compile-time embedded templates
```

`zeta-prompts` 不得依赖 `zeta-core`、App Server 或 provider runtime。若未来动态模板需要上下文，
调用方应先构造只包含必要字段的 prompt-specific projection；不得让本 crate 读取 live runtime
状态或 `ThreadSnapshot`。

## Implementation

`src/artifact.rs` 的 `PromptArtifact` 是所有公共资产的共同载体：

- `category()` 标识四个内置类别；
- `id()` 是跨调用稳定的逻辑身份；
- `revision()` 在提示词语义变化时必须 bump；
- `body()` 返回 `include_str!` 嵌入的原始正文。

`RenderedPrompt` 将动态正文和源 `PromptArtifact` 绑定，调用方可以从同一个返回值读取 body 与
revision，不需要手工维护两条并行数据。

`src/system.rs`、`src/compact.rs` 和 `src/review.rs` 负责绑定对应模板、类别、ID 与 revision。
`src/goals.rs` 另外负责将 prompt-specific 的目标和预算投影渲染为 `RenderedPrompt`。`src/lib.rs`
保持模块私有并显式导出公共资产和渲染 API；不要把模板文件直接暴露成可变路径或让调用方自行
拼接字符串。

调用关系是：

```text
feature/context caller
  → zeta_prompts::<CATEGORY>_PROMPT or render_goals_prompt
  → PromptArtifact / RenderedPrompt body + revision
  → caller decides injection timing and semantic layer
  → ContextPlan / provider-neutral ModelRequest
```

模板资源通过 `include_str!` 嵌入，因此缺失会在编译期失败。当前动态 renderer 只存在于 goals
模块：它使用有名字的输入类型、明确的 XML text escaping 和 budget contract，并由 sibling test
module 覆盖。新增 renderer 也必须遵守这一模式；不要提供 `render(String, Option<...>)` 这类无法
表达调用意图的接口。

## 修改影响

修改提示词正文必须同时检查：

1. 对应资产 revision；
2. 调用方的注入条件、instruction layer、provenance 和 snapshot binding；
3. 相关 prompt contract/snapshot/evaluation tests；
4. [`docs/core-context.md`](../../docs/core-context.md) 中的当前状态或 ownership 描述。

新增类别必须同步更新 `PromptCategory`、对应 private module、`lib.rs` export、资源 glob、唯一性测试
和本 README。不要把外部 Skill、MCP Prompt 或专用 classifier protocol 为了复用文本迁移到本 crate。

## 测试

```text
cargo test -p zeta-prompts
bazel test //zeta-rs/prompts:prompts-unit-tests
```

`prompt_tests.rs` 验证所有内置资产具有非空正文、稳定且唯一的 ID/revision，并保持 authored trailing
newline；同时回归 `system-v4` 的 `apply_patch` 默认、`edit` 微编辑/降级、多文件非事务边界和
`update_plan` guidance。`goals_tests.rs` 验证空目标拒绝、文本 escaping、budget 渲染和 source revision binding。
调用时机和上下文 precedence 的测试属于调用方 crate，不应复制到本 crate。
