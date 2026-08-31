# Zeta 文档导航

> 本文件是跨产品 `docs/` 的工程文档索引。新增跨产品工程文档必须同时加入本索引。
> 面向使用者的文档和文档站由独立的 [`zeta-docs`](https://github.com/chogng/zeta-docs) 仓库维护。
> `app` 专属系统文档由 [`app/docs/README.md`](../app/docs/README.md) 索引，避免产品宿主
> 文档和共享系统文档混在同一目录。
> `zeta code` 专属系统文档由 [`zeta-code/docs/README.md`](../zeta-code/docs/README.md) 索引。
> 系统性的阅读方法（先系统后 crate、两层文档职责）见
> [`documentation-guidelines.md`](documentation-guidelines.md)。

## 快速理解

这份索引只负责跨产品、跨 crate 的系统文档；`app` 的产品宿主、渲染、终端和输入文档集中在
[`app/docs/README.md`](../app/docs/README.md)。按下面的边界选择入口：

| 目标 | 入口 |
| --- | --- |
| 理解共享 backend、协议、执行和存储 | 本目录对应的系统文档与 `zeta-rs/*/README.md` |
| 理解 `app` 产品行为 | [`app/docs/README.md`](../app/docs/README.md) |
| 理解 `zeta code` CLI/TUI | [`zeta-code/docs/README.md`](../zeta-code/docs/README.md) |
| 理解某个 crate 的实现契约 | 对应 crate 的 `README.md` |
| 学习如何使用 Zeta | [`zeta-docs`](https://github.com/chogng/zeta-docs) |

## 1. 我该看哪份？

按你要做的事查表，**按列出顺序读**：

| 你要做什么 | 看这些（按序） |
| --- | --- |
| 新人理解 Zeta 全貌 | [`architecture.md`](architecture.md) → 感兴趣系统的领域文档 |
| 改 Project / Session / Thread / Workspace / Environment 命名或多根关系 | [`domain-model.md`](domain-model.md) → [`environment-access.md`](environment-access.md) → [`multi-agent-development.md`](multi-agent-development.md) |
| **搭 / 改 agent harness**（提示词、工具、循环） | [`agent-harness-implementation-plan.md`](agent-harness-implementation-plan.md)（照着做）→ [`agent-tools-spec.md`](agent-tools-spec.md)（照着抄）→ [`agent-harness-design.md`](agent-harness-design.md)（查原因） |
| 改执行内核（Turn/Thread/恢复/取消） | [`zeta-agent-runtime-architecture.md`](zeta-agent-runtime-architecture.md) → [`core.md`](core.md) |
| 做上下文预算 / 压缩 | [`core-context.md`](core-context.md) + [`agent-harness-design.md`](agent-harness-design.md) §9–§11 |
| 做同 Session Agent tree / 子 Agent 运行时 | [`core-multi-agent.md`](core-multi-agent.md)（gate 条件见其状态头） |
| 做 Team、跨 Session Agent 或可靠多 Agent 代码开发 | [`multi-agent-development.md`](multi-agent-development.md) → [`domain-model.md`](domain-model.md) → [`core-multi-agent.md`](core-multi-agent.md) → [`chat-session-inspector.md`](chat-session-inspector.md) → [`permissions.md`](permissions.md) |
| 开发代码知识、Workspace Symbol 或跨语言导航 | [`code-intelligence.md`](code-intelligence.md) → [`syntax-analysis.md`](syntax-analysis.md) → [`lsp.md`](lsp.md) → [`codebase.md`](codebase.md) |
| 开发 Codebase / RAG | [`code-intelligence.md`](code-intelligence.md) → [`codebase.md`](codebase.md) → [`zeta-codebase` README](../zeta-rs/codebase/README.md) → [`zeta-cloud-codebase` README](../zeta-rs/cloud-codebase/README.md) |
| 设计 Instructions / Skills / Agents 或外部导入 | [`agent-customizations.md`](agent-customizations.md) → 对应 authority 文档 |
| 加 / 改一个工具 | [`agent-tools-spec.md`](agent-tools-spec.md) → [`tools.md`](tools.md)（契约层） |
| 改协议 / 加 App Server 方法 | [`protocol.md`](protocol.md) → [`zeta-app-server-api.md`](zeta-app-server-api.md) → [`app-server-client.md`](app-server-client.md) |
| 权限 / 审批 / 沙箱 | [`permissions.md`](permissions.md) → [`auto-review.md`](auto-review.md) → [`sandboxing.md`](sandboxing.md) |
| 环境选择与目录访问 | [`domain-model.md`](domain-model.md) → [`environment-access.md`](environment-access.md) → [`permissions.md`](permissions.md) |
| 接 / 改模型供应商 | [`model-provider.md`](model-provider.md) → [`model-provider-config.md`](model-provider-config.md) → [`models-manager.md`](models-manager.md) |
| 接 / 改独立 Marketplace | [`marketplace-integration.md`](marketplace-integration.md) → [`localization.md`](localization.md) 或对应 [`plugins.md`](plugins.md)、[`lsp.md`](lsp.md)、[`skills.md`](skills.md) 或 [`mcp.md`](mcp.md) → 对应 crate README |
| 改 Desktop UI | [`zeta-desktop-architecture.md`](zeta-desktop-architecture.md) → [`ui-styling-ownership.md`](ui-styling-ownership.md) |
| 设计或修改三端快捷键 | [`keybindings.md`](keybindings.md) → 对应端的实现 README |
| 开发 SSH Remote Workspace | [`remote-development.md`](remote-development.md) → [`zeta-desktop-architecture.md`](zeta-desktop-architecture.md) → [`app-server-client.md`](app-server-client.md) |
| 改 `app` 产品行为 | [`app/docs/README.md`](../app/docs/README.md) → 对应产品文档 → [`app/README.md`](../app/README.md) |
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
| [`agent-harness-implementation-plan.md`](agent-harness-implementation-plan.md) | 计划 | Agent Loop S1–S7 的构建顺序、状态和发布门 |
| [`agent-tools-spec.md`](agent-tools-spec.md) | 规格 | 逐工具 schema / 描述正文 / 错误文案 + 系统提示词扩写 |
| [`agent-harness-design.md`](agent-harness-design.md) | 设计 | harness 行为策略：提示词、循环、失败、裁剪、压缩、缓存 |
| [`zeta-agent-runtime-architecture.md`](zeta-agent-runtime-architecture.md) | 设计 | 执行内核总体设计、组件状态总账、阶段 A–E |
| [`core.md`](core.md) | 设计 | zeta-core 的 ownership、组件、端口、提交顺序 |
| [`core-context.md`](core-context.md) | 设计 | ContextPlan / Manager / checkpoint / compaction 机制 |
| [`core-multi-agent.md`](core-multi-agent.md) | 设计 | 同 Session Agent tree 的 delegation / Fresh spawn / delivery / 隔离（部分实现，缺口见状态头） |
| [`multi-agent-development.md`](multi-agent-development.md) | 设计 | Team、跨 Session 协作、Project 多根、工作契约、冲突、验证证据与集成门禁（计划设计） |
| [`agent-customizations.md`](agent-customizations.md) | 设计 | Instructions / Skills / Agents、`.zeta` 与外部导入边界 |
| [`tools.md`](tools.md) | 设计 | 工具三层契约、registry snapshot |
| [`exec.md`](exec.md) | 设计 | 进程执行 |
| [`marketplace-integration.md`](marketplace-integration.md) | 设计 | 远端签名 registry、Zeta 本地 Manager、opaque capability handoff 与旧消费链迁移 |
| [`localization.md`](localization.md) | 设计 | 内置 locale catalog、Marketplace localization 包与 UI fallback |
| [`plugins.md`](plugins.md) / [`connectors.md`](connectors.md) / [`skills.md`](skills.md) | 设计 | Plugin 扩展分发、Connector 外部账号连接与 Skill 指令运行时边界 |
| [`editor-extensions.md`](editor-extensions.md) | 设计 | 声明式扩展与 Zeta 原生可执行 Host v1 的双轨边界、信任、生命周期和产品接入状态 |
| [`mcp.md`](mcp.md) / [`mcp-server.md`](mcp-server.md) | 设计 | MCP 协议会话与能力调用、Connector ready binding，以及 Zeta 作为 MCP server |
| [`slash-commands.md`](slash-commands.md) | 设计 | Slash Command 与统一斜杠启动面板边界 |

### 协议与 API

[`domain-model.md`](domain-model.md)（领域身份与命名）、[`protocol.md`](protocol.md)（canonical 产品契约）、
[`zeta-app-server-api.md`](zeta-app-server-api.md)、[`app-server-client.md`](app-server-client.md)、
[`zeta-api.md`](zeta-api.md)、[`zeta-api-interface-requirements.md`](zeta-api-interface-requirements.md)、
[`zeta-api-interface-template.md`](zeta-api-interface-template.md)（模板）、
[`zeta-client.md`](zeta-client.md)、[`chatgpt-subscription.md`](chatgpt-subscription.md)（参考）

### 模型与配置

[`model-provider.md`](model-provider.md)、[`model-provider-config.md`](model-provider-config.md)、
[`models-manager.md`](models-manager.md)、[`config.md`](config.md)、[`login.md`](login.md)、
[`secrets.md`](secrets.md)

### 安全与权限

[`permissions.md`](permissions.md)、[`auto-review.md`](auto-review.md)、
[`sandboxing.md`](sandboxing.md)、[`environment-access.md`](environment-access.md)、
[`workspace-security.md`](workspace-security.md)（当前实现）、
[`windows-sandbox-acceptance-runbook.md`](windows-sandbox-acceptance-runbook.md)（参考/手册）

### 界面与体验

[`zeta-desktop-architecture.md`](zeta-desktop-architecture.md)、
[`ui-styling-ownership.md`](ui-styling-ownership.md)、
[`editor-architecture.md`](editor-architecture.md)、[`editor-core.md`](editor-core.md)、
[`workbench-pane-composite-design.md`](workbench-pane-composite-design.md)、
[`design-tokens.md`](design-tokens.md)、[`theme-authoring-template.md`](theme-authoring-template.md)（模板）、
[`menu-system.md`](menu-system.md)、[`icons.md`](icons.md)、[`search.md`](search.md)、
[`keybindings.md`](keybindings.md)、
[`code-intelligence.md`](code-intelligence.md)、
[`codebase.md`](codebase.md)、
[`syntax-analysis.md`](syntax-analysis.md)、[`lsp.md`](lsp.md)、
[`editor-extensions.md`](editor-extensions.md)、
[`chat-session-inspector.md`](chat-session-inspector.md)、[`pdf.md`](pdf.md)、[`typst.md`](typst.md)

### app 产品

[`app/docs/README.md`](../app/docs/README.md) 是产品文档入口，包含 Agent Console、Terminal
Surface、Native Text Input、Rendering、UI 迁移、Native 弃用、应用迁移和发布图。

### 平台与产品

[`architecture.md`](architecture.md)（总入口）、[`zeta-rs-architecture.md`](zeta-rs-architecture.md)、
[`zeta-code/docs/README.md`](../zeta-code/docs/README.md)、[`workbench-modes.md`](workbench-modes.md)、
[`product-lines.md`](product-lines.md)、[`remote-development.md`](remote-development.md)、[`git.md`](git.md)、
[`documentation-guidelines.md`](documentation-guidelines.md)

### 计划与迁移（会过期）

| 文档 | 状态 |
| --- | --- |
| [`app/docs/native-deprecation-plan.md`](../app/docs/native-deprecation-plan.md) | Native 弃用迁移 |
| [`app/docs/app-migration-plan.md`](../app/docs/app-migration-plan.md) | App 迁移 |
| [`app/docs/app-release-graph.md`](../app/docs/app-release-graph.md) | App 发布依赖 |

## 4. 维护规则

1. 新跨产品工程文档加入本索引并声明类型（设计/规格/计划/参考）；产品专属文档加入对应产品的 `docs/README.md`；面向使用者的内容加入 `zeta-docs/docs/toc.json`；
2. 计划类文档完成后：结论并入对应设计文档的状态标记，计划本体标 Done 或删除——
   不允许长期留着已完成的计划冒充现状；
3. 设计文档描述未实现组件必须挂状态标记（已实现/部分/仅设计/推迟），词表定义见
   [`zeta-agent-runtime-architecture.md`](zeta-agent-runtime-architecture.md)；
4. 同一主题冲突时的权威关系在两份文档开头互相声明（见
   [`documentation-guidelines.md`](documentation-guidelines.md)）。
