# Zeta 文档导航

> 本文件是 `docs/` 的唯一索引。新增文档必须同时加入本索引和
> [`docs-site/scripts/generate-docs.mjs`](../docs-site/scripts/generate-docs.mjs) 的分组。
> 系统性的阅读方法（先系统后 crate、两层文档职责）见
> [`documentation-guidelines.md`](documentation-guidelines.md)。

## 1. 我该看哪份？

按你要做的事查表，**按列出顺序读**：

| 你要做什么 | 看这些（按序） |
| --- | --- |
| 新人理解 Zeta 全貌 | [`architecture.md`](architecture.md) → 感兴趣系统的领域文档 |
| **搭 / 改 agent harness**（提示词、工具、循环） | [`agent-harness-implementation-plan.md`](agent-harness-implementation-plan.md)（照着做）→ [`agent-tools-spec.md`](agent-tools-spec.md)（照着抄）→ [`agent-harness-design.md`](agent-harness-design.md)（查原因） |
| 改执行内核（Turn/Thread/恢复/取消） | [`zeta-agent-runtime-architecture.md`](zeta-agent-runtime-architecture.md) → [`core.md`](core.md) |
| 做上下文预算 / 压缩 | [`core-context.md`](core-context.md) + [`agent-harness-design.md`](agent-harness-design.md) §9–§11 |
| 做多 Agent | [`core-multi-agent.md`](core-multi-agent.md)（gate 条件见其状态头） |
| 加 / 改一个工具 | [`agent-tools-spec.md`](agent-tools-spec.md) → [`tools.md`](tools.md)（契约层） |
| 改协议 / 加 App Server 方法 | [`protocol.md`](protocol.md) → [`zeta-app-server-api.md`](zeta-app-server-api.md) → [`app-server-client.md`](app-server-client.md) |
| 权限 / 审批 / 沙箱 | [`permissions.md`](permissions.md) → [`auto-review.md`](auto-review.md) → [`sandboxing.md`](sandboxing.md) |
| 接 / 改模型供应商 | [`model-provider.md`](model-provider.md) → [`model-provider-config.md`](model-provider-config.md) → [`models-manager.md`](models-manager.md) |
| 改 Desktop UI | [`zeta-desktop-architecture.md`](zeta-desktop-architecture.md) → [`ui-styling-ownership.md`](ui-styling-ownership.md) |
| 写 / 改文档本身 | [`documentation-guidelines.md`](documentation-guidelines.md) |

## 2. 文档类型

每份文档属于四类之一，读法不同：

| 类型 | 含义 | 过期方式 |
| --- | --- | --- |
| 设计 | 回答"为什么这样设计、边界在哪"；组件挂状态标记（已实现/部分/仅设计/推迟） | 长期维护，重审时修订 |
| 规格 | 逐字照抄的实现规格（schema、文案、字段） | 随实现同步，改规格必须跑对应评测 |
| 计划 | 阶段性工作计划 | **完成即过期**——做完后并入设计文档的状态标记，本体标记 Done 或删除 |
| 参考 | 运行手册、模板、外部快照 | 按需更新 |

## 3. 分类清单

### Agent 与运行时（当前主战场）

| 文档 | 类型 | 一句话 |
| --- | --- | --- |
| [`agent-harness-implementation-plan.md`](agent-harness-implementation-plan.md) | 计划 | coding 最小闭环（WI1–WI6）的文件级步骤 |
| [`agent-tools-spec.md`](agent-tools-spec.md) | 规格 | 逐工具 schema / 描述正文 / 错误文案 + 系统提示词扩写 |
| [`agent-harness-design.md`](agent-harness-design.md) | 设计 | harness 行为策略：提示词、循环、失败、裁剪、压缩、缓存 |
| [`zeta-agent-runtime-architecture.md`](zeta-agent-runtime-architecture.md) | 设计 | 执行内核总体设计、组件状态总账、阶段 A–E |
| [`core.md`](core.md) | 设计 | zeta-core 的 ownership、组件、端口、提交顺序 |
| [`core-context.md`](core-context.md) | 设计 | ContextPlan / Manager / checkpoint / compaction 机制 |
| [`core-multi-agent.md`](core-multi-agent.md) | 设计 | delegation / spawn / join / 隔离（仅设计，gate 见状态头） |
| [`tools.md`](tools.md) | 设计 | 工具三层契约、registry snapshot |
| [`exec.md`](exec.md) | 设计 | 进程执行 |
| [`skills.md`](skills.md) / [`plugins.md`](plugins.md) / [`slash-commands.md`](slash-commands.md) | 设计 | 扩展来源：Skill / Plugin / 斜杠命令 |
| [`mcp.md`](mcp.md) / [`mcp-server.md`](mcp-server.md) | 设计 | MCP 客户端 / Zeta 作为 MCP server |

### 协议与 API

[`protocol.md`](protocol.md)（canonical 产品契约）、
[`zeta-app-server-api.md`](zeta-app-server-api.md)、[`app-server-client.md`](app-server-client.md)、
[`zeta-api.md`](zeta-api.md)、[`zeta-api-interface-requirements.md`](zeta-api-interface-requirements.md)、
[`zeta-api-interface-template.md`](zeta-api-interface-template.md)（模板）、
[`zeta-client.md`](zeta-client.md)、[`codex-app-server.md`](codex-app-server.md)（参考）

### 模型与配置

[`model-provider.md`](model-provider.md)、[`model-provider-config.md`](model-provider-config.md)、
[`models-manager.md`](models-manager.md)、[`config.md`](config.md)、[`login.md`](login.md)、
[`secrets.md`](secrets.md)

### 安全与权限

[`permissions.md`](permissions.md)、[`auto-review.md`](auto-review.md)、
[`sandboxing.md`](sandboxing.md)、[`workspace-security.md`](workspace-security.md)、
[`windows-sandbox-acceptance-runbook.md`](windows-sandbox-acceptance-runbook.md)（参考/手册）

### 界面与体验

[`zeta-desktop-architecture.md`](zeta-desktop-architecture.md)、
[`ui-styling-ownership.md`](ui-styling-ownership.md)、
[`rendering-architecture.md`](rendering-architecture.md)、
[`editor-architecture.md`](editor-architecture.md)、[`editor-core.md`](editor-core.md)、
[`workbench-pane-composite-design.md`](workbench-pane-composite-design.md)、
[`design-tokens.md`](design-tokens.md)、[`theme-authoring-template.md`](theme-authoring-template.md)（模板）、
[`menu-system.md`](menu-system.md)、[`icons.md`](icons.md)、[`search.md`](search.md)、
[`syntax-analysis.md`](syntax-analysis.md)、[`lsp.md`](lsp.md)、
[`native-text-input.md`](native-text-input.md)、[`native-terminal-ui.md`](native-terminal-ui.md)、
[`native-agent-console.md`](native-agent-console.md)、[`tui.md`](tui.md)、
[`chat-session-inspector.md`](chat-session-inspector.md)、[`pdf.md`](pdf.md)、[`typst.md`](typst.md)

### 平台与产品

[`architecture.md`](architecture.md)（总入口）、[`zeta-rs-architecture.md`](zeta-rs-architecture.md)、
[`zeta-cli-architecture.md`](zeta-cli-architecture.md)、[`product-editions.md`](product-editions.md)、
[`product-lines.md`](product-lines.md)、[`git.md`](git.md)、
[`documentation-guidelines.md`](documentation-guidelines.md)

### 计划与迁移（会过期）

| 文档 | 状态 |
| --- | --- |
| [`zeta-code-architecture-codex-style-v2.md`](zeta-code-architecture-codex-style-v2.md) | 长期演进方向 |
| [`native-deprecation-plan.md`](native-deprecation-plan.md) | Native 弃用迁移 |
| [`ui-component-migration-plan.md`](ui-component-migration-plan.md) | UI 组件迁移 |
| [`zeterm-app-migration-plan.md`](zeterm-app-migration-plan.md) | Zeterm 迁移 |
| [`zeterm-release-graph.md`](zeterm-release-graph.md) | Zeterm 发布依赖 |

## 4. 维护规则

1. 新文档：入本索引 + docs-site 分组，声明类型（设计/规格/计划/参考）；
2. 计划类文档完成后：结论并入对应设计文档的状态标记，计划本体标 Done 或删除——
   不允许长期留着已完成的计划冒充现状；
3. 设计文档描述未实现组件必须挂状态标记（已实现/部分/仅设计/推迟），词表定义见
   [`zeta-agent-runtime-architecture.md`](zeta-agent-runtime-architecture.md)；
4. 同一主题冲突时的权威关系在两份文档开头互相声明（见
   [`documentation-guidelines.md`](documentation-guidelines.md)）。
