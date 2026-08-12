# `zeta-skills-extension`

跨 crate 的 Skill 语义和分阶段演进由 [`docs/skills.md`](../../../docs/skills.md) 维护；底层目录、
文件和 catalog 契约由 [`zeta-skills`](../../skills/README.md) 维护。

`zeta-skills-extension` 拥有 `zeta-skills` 之上的 Agent 运行时编排：组合 built-in、user 和当前
Workspace source，应用启用状态与兼容性策略，在 Turn 提交前冻结显式选择，在模型调用安全点重新
加载 exact content，并通过 `zeta-extension-api` 贡献有界元数据目录、exact Skill instructions
以及单个只读 `skills-read` 模型工具。

## 所有权

| Symbol | 当前职责 |
| --- | --- |
| `SkillRuntime` | catalog generation、source composition、exact activation 和 watcher lifecycle |
| `SkillActivationContributor` 实现 | 把显式 `UserInput::Skill` 选择转换为 durable activation |
| `TurnInputContributor` 实现 | 输出 metadata-only `catalog_prompt` 和 exact frozen activation fragment |
| `SkillToolContributor` | 注册 `SkillReadTool`，不依赖 App Server |
| `SkillReadTool` | 解析 exact `source + name`，通过 tagged target 读取完整说明或 digest-pinned package-relative text resource |
| `SkillRuntimeEventSink` | public catalog generation 改变后通知安装它的 host |

`zeta-skills` 仍是底层文件和 catalog authority。Core 只调用通用 extension lifecycle contract。
App Server 可以提供配置、事件 adapter 和 list DTO，但不得选择、激活、加载、缓存或渲染 Skill
instructions。

真实调用路径：

```text
install
  -> TurnInputContributor::contribute
       -> catalog_prompt (metadata only, at most 8 KiB)
  -> ReadOnlyToolContributor::contribute
       -> SkillReadTool::execute
            -> target = instructions
                 -> SkillRuntime::activate_model_selected
                      -> SkillCatalog::activate
            -> target = resource
                 -> SkillRuntime::read_model_resource
                      -> SkillCatalog::read_resource
```

手动选择和模型选择共用 catalog validation 与 exact file loading，但 durable shape 不同。手动选择
在 `TurnAccepted` 中冻结并作为 `PromptFragment` 贡献；模型选择发生在 Tool loop 中，成功的
`skills-read` result 成为下一次 invocation 的 durable model context。后端不做关键词分类。

当前模型侧 resource read 只接受 UTF-8 text、每次一个文件，上限 256 KiB。底层 resource contract
保留 binary bytes，但 MIME/artifact materialization 尚未接通；script execution 仍是普通 Tool
operation。catalog prompt 固定上限 8 KiB，Core 继续通过普通 BestEffort context budget 计算它。
