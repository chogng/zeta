# TUI 功能完整性核对

- 状态：原工作共 27 条要求，未逐项执行产品验收；Q-1 至 Q-4 仍待确定。
- 本次整理只把通用要求按主题建立长期入口，不把这项工作宣布完成。
- 原 AC 编号继续用于本轮验收；长期规格按功能维护，不以该工作作为所有后续功能的总规格。

## 本次记录

| 目标 | 本次核对要求 | 计划 | 验收 |
| --- | --- | --- | --- |
| [intent.md](intent.md) | [spec.md](spec.md) | [plan.md](plan.md) | [verification.md](verification.md) |

## AC 与长期文档的对应

| AC | 内容 | 现行规格 |
| --- | --- | --- |
| AC-1、AC-2 | 启动与恢复会话 | [终端](../../spec/terminal.md)、[会话](../../spec/sessions.md) |
| AC-3 至 AC-10 | 输入、附件、Queue、状态、中断、审批、提问 | [对话交互](../../spec/conversation.md) |
| AC-11 至 AC-15 | 会话列表、预览、切换、详情、归档、分支与回退 | [会话管理](../../spec/sessions.md) |
| AC-16、AC-18 | 长输出、展开、复制和导出 | [正文输出](../../spec/transcript.md) |
| AC-17 | 终端历史 | [终端行为](../../spec/terminal.md) |
| AC-19、AC-20 | 设置与面板操作 | [命令面板](../../spec/commands.md)、[交互规则](../../spec/interaction.md) |
| AC-21、AC-22 | Skill、补全、Connector 与 MCP | [对话交互](../../spec/conversation.md)、[命令面板](../../spec/commands.md) |
| AC-23 | 目录与权限 | [目录权限](../../spec/directories.md) |
| AC-24、AC-25 | 断线、退出与挂起 | [终端行为](../../spec/terminal.md) |
| AC-26 | 键盘、鼠标、窄窗口与无颜色 | [交互](../../spec/interaction.md)、[终端](../../spec/terminal.md)、[布局](../../spec/layout.md)、[样式](../../spec/styles.md) |
| AC-27 | 状态与资源 | [状态规格](../../spec/status.md) |

## 尚未确定的范围

| 编号 | 问题 | 对应文档 |
| --- | --- | --- |
| Q-1 | 可点击链接、表格和完整 Markdown 的交付要求 | [正文](../../spec/transcript.md) |
| Q-2 | 必须验证的系统、终端和复用器组合 | [终端](../../spec/terminal.md) |
| Q-3 | 断线或重启后未发送草稿、图片和本地队列是否必须恢复 | [终端恢复](../../spec/terminal.md#连接恢复) |
| Q-4 | Welcome 点击播放是否属于本轮交付 | [宠物规格](../../spec/welcome-pet.md) |

## 设计与后续工作

- 状态、请求和界面生命周期：[TUI 架构](../../design/tui.md)。
- CLI 接入与连接：[CLI 架构](../../design/cli.md)。
- 进程资源后台工作：[采样设计](../../design/process-resources.md)。
- 单项功能的后续工作分别登记在[变更索引](../README.md)，不在本工作中覆盖其他工作的验收证据。
