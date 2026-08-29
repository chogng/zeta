# `zeta-prompts`

`zeta-prompts` 是共享提示词库，不是全项目提示词目录。它只收跨产品流程复用、需要统一审计的稳定提示词；模型和具体功能仍然拥有自己的提示词。

## 职责

- 提供 `PromptArtifact`、动态渲染绑定和冻结为 `TurnInstructions` 的统一方式。
- 拥有 context compaction 和代码 review 的共享提示词、target 渲染与测试。
- 不读取 Thread、Config、Workspace、Skill 或 provider runtime，也不决定最终 context 的组装顺序。

## 所有权规则

| 内容 | Owner |
| --- | --- |
| 模型基础 instructions | `zeta-models-manager` |
| context compaction、通用代码 review | `zeta-prompts` |
| Thread Goal、动态 context fragment | `zeta-core` 对应功能模块 |
| 自动审查协议提示词 | `zeta-auto-review` |
| Skill、扩展和工具描述 | 各能力 crate |
| 其他跨多个产品流程复用且语义稳定的模板 | 满足真实复用需求后放入本 crate |

新提示词默认放在功能 owner 的 `templates/` 下，由 owner 定义变量、escaping、revision 和测试。只有出现明确的共享产品流程时才放入这里；不能仅因为一段文本会被模型看到就集中存放。

依赖方向保持为：

```text
models-manager / feature owner -> zeta-prompts::PromptArtifact
App Server -> PromptArtifact::freeze -> durable TurnInstructions
Core context pipeline -> frozen TurnInstructions + dynamic fragments
```

普通 Turn 在 App Server 接受请求前冻结 `models-manager` 的基础 instructions；review Turn 冻结 `REVIEW_PROMPT`。Core 只读取 Turn 内已经持久化的快照，重启或后续模型配置变化不能改变它。

## 修改与测试

修改提示词正文时，在 owner crate 同步 revision、渲染测试和最终 context 组装测试。修改共享契约时运行：

```text
cargo test -p zeta-prompts
bazel test //zeta-rs/prompts:prompts-unit-tests
```
