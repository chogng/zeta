# TUI 功能现状

主要用户流程的源码核对结果，基线为 2026-09-07 的 `f173facf5`。核对期间正文实现更新至包含 `67603c262` 的版本，相关路径已修正，行为尚未重新验证。本轮未运行产品测试；表中的测试链接是复查入口。

完整的用户流程要求与逐项核对见[功能规格](changes/tui-completeness/spec.md)和[验收记录](changes/tui-completeness/verification.md)。本表用于查看现状，不代替验收。

## 当前能力

| 功能 | 当前行为与限制 | 代码 / 测试 |
| --- | --- | --- |
| [消息输入](spec/interaction.md#页面辅助区域与浮层) | 空闲时提交新任务；运行中可排队等下一轮（Queue）或补充当前任务（Steer）。Steer 不能改变 Skill | [提交](../tui/src/thread/composer/submission.rs)、[测试](../tui/src/thread/composer/submission_tests.rs) |
| [审批与提问](spec/interaction.md#页面辅助区域与浮层) | 支持一次批准、拒绝、选项和自定义回答。Esc 不隐式提交结果 | [审批](../tui/src/thread/interaction/approval.rs)、[回答](../tui/src/thread/interaction/query.rs)、[测试](../tui/src/thread/request_tests.rs) |
| [会话管理](spec/interaction.md#session-管理列表的动作) | 各分组独立展开收起，支持归档与恢复。Archived 默认收起，其余默认展开 | [列表](../tui/src/sessions/manager.rs)、[场景测试](../tui/src/app/session_manager_tests.rs) |
| [预览与详情](design/tui.md#会话切换预览与详情) | 预览只读快照并可加载较早历史；Agent 详情持续刷新 | [详情](../tui/src/sessions/details.rs)、[详情测试](../tui/src/sessions/details_tests.rs)、[预览测试](../tui/src/app/session_manager_tests.rs) |
| [正文与执行结果](design/tui.md#正文与终端历史) | 回复逐步显示，相关执行结果合并展示；可展开和滚动，预览及详情受保留容量限制 | [模型](../tui/src/thread/transcript/model.rs)、[执行](../tui/src/thread/transcript/exec_cell.rs)、[测试](../tui/src/thread/transcript/view/render_tests.rs) |
| [终端历史](spec/interaction.md#鼠标规则) | 已定稿内容写入终端历史，仍在生成的内容继续刷新。不同终端及复用器组合尚未全面验证 | [终端会话](../tui/src/terminal/session.rs)、[协议测试](../tui/src/terminal/history_protocol_tests.rs) |
| [断线恢复](../tui/README.md#产品支持边界) | TUI 返回持久化身份，由 CLI 重建连接。不恢复旧连接中的待执行请求 | [断线处理](../tui/src/app/recovery.rs)、[恢复测试](../tui/src/sessions/active_tests.rs)、[PTY 场景](../cli/tests/tui_real_scenarios.rs) |
| [状态与资源](design/process-resources.md) | `/status` 提供 Thread / Processes 两页，状态行按需采样；`/memory` 提供后端持续诊断与导出，不自动确认泄漏 | [状态页](../tui/src/status/panel.rs)、[采样](../../zeta-rs/memory-diagnostics/src/process_resources.rs)、[测试](../../zeta-rs/memory-diagnostics/src/process_resources_tests.rs) |
| [目录管理](spec/interaction.md#每个命令面板) | `/add-dir` 面板可输入路径并按 Enter 添加；成功刷新并定位权限项，失败保留输入，重复添加保留原权限 | [面板](../tui/src/dirs/panel.rs)、[回归测试](../tui/src/dirs_tests.rs)、[验收](changes/dir-add/verification.md) |
| [设置与快捷键](spec/interaction.md#快捷键声明与保存) | 支持保存 TUI 设置、主题和应用快捷键；面板基础键仍固定，凭据使用专用接口 | [配置](../tui/src/config/request.rs)、[快捷键](../tui/src/keymap/settings.rs)、[主题测试](../tui/src/theme/resource_tests.rs)、[快捷键测试](../tui/src/keymap/settings_tests.rs) |
| [OpenAI 配置](changes/openai-panel/spec.md) | 供应商子页可保存 API Key、自定义兼容地址与独立 Key，并进入 ChatGPT 登录；没有远程模型发现与任意多个命名连接 | [面板](../tui/src/config/openai.rs)、[验收](changes/openai-panel/verification.md) |
| [ChatGPT 订阅](changes/chatgpt-provider/spec.md) | Providers → OpenAI 可查看账户、设备码登录、取消和退出；有 Codex 时只读复用，无 Codex 时维护登录和续期，缺失时创建兼容文件 | [账户页面与请求](../tui/src/config/subscription.rs)、[验收](changes/chatgpt-provider/verification.md) |
| [增强鼠标](spec/interaction.md#鼠标规则) | 仅可见的详情、补全覆盖浮层支持交互和字符选择；占布局高度的区域由终端管理鼠标 | [鼠标](../tui/src/terminal/mouse.rs)、[选择测试](../tui/src/terminal/screen_selection_tests.rs) |
| [Welcome 宠物](spec/welcome-pet.md) | 部分实现：静止绘制、动作资源和独立预览可用，Welcome 点击播放尚未接入 | [绘制](../tui/src/app/welcome/pet.rs)、[资源测试](../tui/src/app/welcome_view_tests.rs) |

## 明确缺口

- **正文 Markdown 链接**：尚不可点击，见[正文实现](../tui/src/thread/transcript/view.rs)和[支持边界](../tui/README.md#产品支持边界)。
- **独立 Agent 运行提示**：[样式中的方案](spec/styles.md#运行提示)尚未接入。
- **终端兼容性**：部分环境尚未实测，范围与复现方法见[兼容性记录](../tui/README.md#终端历史兼容性验证)。

完整 Markdown、桌面同等鼠标操作和自动内存诊断不属于当前支持范围。未列出的命令从[命令键表](spec/interaction.md#每个命令面板)继续核对。

支持范围或缺口变化时更新对应行；本次工作的完成结论记录在[验收文件](../../docs/development-workflow.md#验证)中。
