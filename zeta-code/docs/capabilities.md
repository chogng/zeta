# 功能总览

- 本表用于定位当前能力、限制、现行规格和验证入口；不会代替逐项验收。
- 通用源码核对来自 2026-09-07 的 `f173facf5`，正文随后更新至包含 `67603c262` 的版本；后续工作含未提交修改，分别看各项记录。
- 本次文档整理没有重新运行产品测试。原完整性核对中的 27 条要求及未决范围保留在[历史工作](changes/tui-completeness/README.md)。

## TUI 能力

| 功能 | 当前行为与限制 | 代码 / 测试 |
| --- | --- | --- |
| [Issue 开发与 PR](spec/issues.md) | 候选实现：统一面板与 Open / Closed 页签、多选合并为一个 Session、输入标签、固定开发起点、普通/草稿/自动合并 PR；正在验收 | [管理器](../tui/src/issues.rs)、[验收](changes/2026-09-07-issue-manager/verification.md) |
| [消息输入](spec/conversation.md) | 空闲时提交新任务；运行中可排队等下一轮（Queue）或补充当前任务（Steer）。Steer 不能改变 Skill | [提交](../tui/src/thread/composer/submission.rs)、[测试](../tui/src/thread/composer/submission_tests.rs) |
| [审批与提问](spec/conversation.md) | 支持一次批准、拒绝、选项和自定义回答。Esc 不隐式提交结果 | [审批](../tui/src/thread/interaction/approval.rs)、[回答](../tui/src/thread/interaction/query.rs)、[测试](../tui/src/thread/request_tests.rs) |
| [会话管理](spec/sessions.md) | 各分组独立展开收起，支持归档与恢复。Archived 默认收起，其余默认展开 | [列表](../tui/src/sessions/manager.rs)、[场景测试](../tui/src/app/session_manager_tests.rs) |
| [预览与详情](spec/sessions.md) | 预览只读快照并可加载较早历史；Agent 详情持续刷新 | [详情](../tui/src/sessions/details.rs)、[详情测试](../tui/src/sessions/details_tests.rs)、[预览测试](../tui/src/app/session_manager_tests.rs) |
| [正文与执行结果](spec/transcript.md) | 回复逐步显示，相关执行结果合并展示；可展开和滚动，预览及详情受保留容量限制 | [模型](../tui/src/thread/transcript/model.rs)、[执行](../tui/src/thread/transcript/exec_cell.rs)、[测试](../tui/src/thread/transcript/view/render_tests.rs) |
| [终端历史](spec/terminal.md#鼠标规则) | Welcome、命令和定稿消息顺序追加，屏幕写满后进入回滚区；仍在生成的内容继续刷新。不同终端及复用器组合尚未全面验证 | [终端会话](../tui/src/terminal/session.rs)、[协议测试](../tui/src/terminal/history_protocol_tests.rs) |
| [断线恢复](spec/terminal.md#连接恢复) | TUI 返回持久化身份，由 CLI 重建连接。不恢复旧连接中的待执行请求 | [断线处理](../tui/src/app/recovery.rs)、[恢复测试](../tui/src/sessions/active_tests.rs)、[PTY 场景](../cli/tests/tui_real_scenarios.rs) |
| [状态与资源](spec/status.md) | `/status` 提供 Thread / Processes 两页并只读展示；Config 控制持续内存诊断与状态栏简洁 / 生动风格，状态行可显示上下文占用与进度条并按需采样，不自动确认泄漏 | [状态页](../tui/src/status/panel.rs)、[采样](../../zeta-rs/memory-diagnostics/src/process_resources.rs)、[测试](../../zeta-rs/memory-diagnostics/src/process_resources_tests.rs) |
| [目录管理](spec/directories.md) | `/add-dir` 面板可输入路径并按 Enter 添加；成功刷新并定位权限项，失败保留输入，重复添加保留原权限 | [面板](../tui/src/dirs/panel.rs)、[回归测试](../tui/src/dirs_tests.rs)、[验收](changes/dir-add/verification.md) |
| [设置与快捷键](spec/commands.md#设置与快捷键) | 支持保存 TUI 设置、主题和应用快捷键；Config 可在 English、日本語、中文、Français 间切换并立即刷新根页面；面板基础键仍固定，凭据使用专用接口 | [配置](../tui/src/config/request.rs)、[本地化文案](../tui/src/nls.rs)、[快捷键](../tui/src/keymap/settings.rs)、[主题测试](../tui/src/theme/resource_tests.rs) |
| [OpenAI 配置](spec/providers.md) | 官方 Key、ChatGPT 订阅和多个命名连接以 tab 划分；先选中字段再 Enter 编辑，确认后停留当前字段；空目录和发现失败清除旧结果并提示原因。自定义连接可选 Responses / Chat Completions，独立保存 Key，并主动获取模型目录供 `/model` 选择；字段交互已自动化验证，Windows ConPTY 的编辑、取消、保存和模型发现已实测；真实账号未实测 | [面板](../tui/src/config/openai.rs)、[版本与验收](changes/2026-09-08-provider-input-tabs/verification.md) |
| [ChatGPT 订阅](spec/providers.md#chatgpt-账户) | Providers → OpenAI 可查看账户、设备码登录、取消和退出；有 Codex 时只读复用，无 Codex 时维护登录和续期，缺失时创建兼容文件 | [账户页面与请求](../tui/src/config/subscription.rs)、[初始与后续验收](changes/chatgpt-provider/README.md) |
| [增强鼠标](spec/terminal.md#鼠标规则) | 仅可见的详情、补全覆盖浮层支持交互和字符选择；占布局高度的区域由终端管理鼠标 | [鼠标](../tui/src/terminal/mouse.rs)、[选择测试](../tui/src/terminal/screen_selection_tests.rs) |
| [Welcome 宠物](spec/welcome-pet.md) | 部分实现：静止绘制、动作资源和独立预览可用，Welcome 点击播放尚未接入 | [绘制](../tui/src/app/welcome/pet.rs)、[资源测试](../tui/src/app/welcome_view_tests.rs) |

## CLI 与扩展入口

| 能力 | 规格 | 实现与共享约定 |
| --- | --- | --- |
| 启动、ask / exec、输出与退出码 | [CLI](spec/cli.md) | [CLI 架构](design/cli.md)、[exec](../../docs/exec.md)；本文不新增整条 CLI 的验收结论 |
| Skill、Connector、MCP 与其他命令面板 | [命令面板](spec/commands.md) | [开发指南](../tui/README.md#文件与职责)；完整性工作 AC-21、AC-22 尚待逐项验证 |
| 持续内存诊断 | [诊断规格](spec/memory-diagnostics.md) | [共享后端验收](../../zeta-rs/docs/changes/memory-diagnostics/verification.md) |

## 明确缺口

- **正文 Markdown 链接**：尚不可点击，见[正文实现](../tui/src/thread/transcript/view.rs)和[支持边界](../tui/README.md#产品支持边界)。
- **独立 Agent 运行提示**：[样式中的方案](spec/styles.md#运行提示)尚未接入。
- **终端兼容性**：部分环境尚未实测，范围与复现方法见[兼容性记录](../tui/README.md#终端历史兼容性验证)。

完整 Markdown、桌面同等鼠标操作及自动确认内存泄漏不属于当前已支持能力；持续诊断由 Config 明确开启，Status 只读展示。未列出的命令从[命令键表](spec/commands.md#每个命令面板)继续核对。

支持范围或缺口变化时更新对应行；本次工作的完成结论记录在[验收文件](../../docs/development-workflow.md#验证)中。
