# Zeta Code 文档

从[功能总览](capabilities.md)找功能，再进入对应规格。查某次为什么改、验证到哪一步，使用[变更记录](changes/README.md)。

## 文档各管什么

| 文档 | 回答的问题 | 更新方式 |
| --- | --- | --- |
| [capabilities.md](capabilities.md) | 目前支持什么，还有什么缺口？ | 随实现更新摘要，链接规格和证据 |
| `spec/` | 功能现在应该怎样工作？ | 每个主题保留一份现行要求；写进规格不等于已验收 |
| `design/` | 状态、模块、请求和生命周期怎样分工，为什么？ | 随架构更新，不重复命令用法和完整键表 |
| [changes/](changes/README.md) | 某次为什么改、改了哪些要求、实际验证了什么？ | 记录一次工作；完成后保留，后续独立工作另建记录 |
| [TUI 开发指南](../tui/README.md) | 实现入口、配置格式和测试命令在哪里？ | 跟随代码维护 |

## 按功能查规格

| 主题 | 现行规格 |
| --- | --- |
| CLI 命令、输出和退出码 | [cli.md](spec/cli.md) |
| 消息、附件、Queue、审批和提问 | [conversation.md](spec/conversation.md) |
| 会话列表、预览、详情、归档、恢复和回退 | [sessions.md](spec/sessions.md) |
| 命令面板、设置、快捷键、Skill、Connector、MCP | [commands.md](spec/commands.md) |
| 添加目录、重复与失败反馈、目录权限 | [directories.md](spec/directories.md) |
| OpenAI 官方 Key、自定义连接、ChatGPT 账户、模型目录 | [providers.md](spec/providers.md) |
| 状态行、用量和本机进程资源 | [status.md](spec/status.md) |
| 持续采集与报告导出 | [memory-diagnostics.md](spec/memory-diagnostics.md) |
| 正文类型、执行结果、合并、容量限制及输出示例 | [transcript.md](spec/transcript.md) |
| 正文绘制接口、测量和身份一致性 | [transcript-rendering.md](spec/transcript-rendering.md) |
| 鼠标、终端历史、断线恢复和退出 | [terminal.md](spec/terminal.md) |
| Welcome 宠物资源与动作要求 | [welcome-pet.md](spec/welcome-pet.md) |

## 跨功能规则与设计

| 主题 | 文档 |
| --- | --- |
| 输入归属、通用导航、焦点、返回和提示 | [交互规则](spec/interaction.md) |
| 页面区域、高度分配和覆盖顺序 | [布局](spec/layout.md) |
| 标记、颜色、主题状态和无颜色表达 | [样式](spec/styles.md) |
| CLI 与共享后端的职责、连接模式 | [CLI 架构](design/cli.md) |
| TUI 状态、事件、模块、绘制与终端生命周期 | [TUI 架构](design/tui.md) |
| 采样线程、需求版本、队列与有界历史 | [进程资源设计](design/process-resources.md) |

## API 去哪里查

| 接口范围 | 维护位置 |
| --- | --- |
| App Server 方法、参数、结果、通知和错误 | [共享产品 API](../../docs/zeta-app-server-api.md) |
| Client 初始化、连接、请求和关闭 | [App Server Client](../../docs/app-server-client.md) |
| TUI 对 CLI 的公开 Rust 接口 | [TUI 公共接口](../tui/README.md#公共接口)与 [lib.rs](../tui/src/lib.rs) |
| CLI 机器输出事件 | [exec 输出契约](../../docs/exec.md#7-输出契约) |

## 修改文档的顺序

1. 从功能表确定主题；跨功能规则只有变化时才更新。
2. 在[变更索引](changes/README.md)登记本次工作及对应的长期规格、设计。
3. 变更规格写本次差异，计划跟踪实施；验收绑定具体要求版本和候选代码。
4. 完成时合并有效要求到现行规格、更新功能状态，并保留历史记录与证据。

详细规则见[开发流程](../../docs/development-workflow.md)与[文档写作规范](../../.github/instructions/documentation.instructions.md)。
