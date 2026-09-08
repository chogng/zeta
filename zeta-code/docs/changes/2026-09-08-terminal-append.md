# issue #4：终端内容顺序追加

已完成本次修复。Welcome 作为第一段终端内容保留，命令与消息追加在其后；屏幕写满后再滚动，交互区域不再占满剩余空白。

本文件记录当时的主屏幕追加实现和验收。该实现已由 [issue #13 全屏对话](2026-09-08-terminal-blank-lines.md)退场，下面的协议、路径和结果只代表当时版本。

## 目标与范围

来源：[issue #4](https://github.com/chogng/zeta/issues/4)，包括 2026-09-08 的追加效果说明。此记录合并小修复的目标、规格、实施与验收；现行要求由[终端规格](../spec/terminal.md)和[布局规格](../spec/layout.md)维护，职责见 [TUI 设计](../design/tui.md#全屏内容区与固定交互区)。

| 要求 | 操作与期望 |
| --- | --- |
| AC-1 | 首次输入 slash command 后，Welcome 与命令按顺序保留，不清空一整页来追加短内容 |
| AC-2 | 连续追加短消息和长正文，文本恰好一次、顺序正确，不产生整屏空白分隔 |
| AC-3 | 重绘、输出失败重试和尺寸变化不重复提交 Welcome 或正文；交互内容不泄漏到回滚区 |
| AC-4 | 输入、补全鼠标接管与终端滚轮保持原有操作语义 |

本次不改变模型请求、供应商配置、会话存储或终端自身的回滚保留上限。该历史实现的验证范围保留在下文；当前全屏实现见[开发指南](../../tui/README.md#全屏终端兼容性验证)。

## 实施

- [x] App 先提交 Welcome，再提交定稿正文；只有输出成功才记为已提交。
- [x] Frame 测量可变正文和辅助区域高度；TerminalSession 管理可移动的交互区域。
- [x] 从交互区域起点逐行追加正文，随后在正文后预留交互空间。退场原 `HistoryBackend::commit_top_row` 的顶端两行滚动协议。
- [x] 增加短内容连续追加、Welcome 输出失败重试和真实 PTY 场景；同步现行文档。

## 2026-09-08 验收

代码基线：`7f935080d0c39b1cdfebac25d93dceb7235bafae`。工作区已有供应商、输入组件和其他任务的改动，本次只修改上述 TUI 输出链路；测试使用当时完整工作区。候选源文件指纹（按下面顺序拼接路径、NUL、文件字节、NUL 后 SHA-256）：`63f9c41b48f5a4fa22b29a33e291ab739b43bc2ea67f11649ed1667150eac76b`。

```
zeta-code/tui/src/app/event_loop.rs
zeta-code/tui/src/app/event_loop_tests.rs
zeta-code/tui/src/app/frame.rs
zeta-code/tui/src/app/layout.rs
zeta-code/tui/src/app/state.rs
zeta-code/tui/src/terminal/session.rs
zeta-code/tui/src/terminal/session_tests.rs
zeta-code/tui/src/terminal/history_protocol_tests.rs
zeta-code/tui/src/thread/transcript/history.rs
```

| 验证 | 结果与覆盖 |
| --- | --- |
| `just check zeta-tui` | 通过；普通生产构建，无警告 |
| `just test zeta-tui --lib` | 初轮 714 通过、2 个 PTY 测试按约定忽略；后续增加重试断言与收尾调整后运行下述受影响测试 |
| `just test zeta-tui --lib terminal:: -- --nocapture` | 最终候选 32 通过；短追加不产生回滚行、长输出、模式恢复等，覆盖 AC-1 至 AC-3 |
| `just test zeta-tui --lib app::event_loop::tests -- --nocapture` | 最终候选 9 通过、2 个 PTY 测试忽略；Welcome 重试与去重、输入及鼠标边界，覆盖 AC-3、AC-4 |
| `just test zeta-tui --lib history_compatibility` | 2 通过，并导出当前生产追加协议 |
| Windows ConPTY 单独执行 `real_terminal_history_append` 与 `real_terminal_mouse_handoff`，带 `--ignored --nocapture --test-threads=1` | 两项均通过；使用已由上述命令编译的测试二进制，100×40 的真实 PTY |
| Playwright + Chromium / xterm.js 字符缓冲区核对 PTY 输出 | Welcome 1 次，40 条消息各 1 次且顺序正确，正文间连续空白最多 1 行；缩为 60×24 后仍完整，覆盖 AC-1 至 AC-3 |
| Playwright 执行 `terminal_history.js` | 6 组协议样本全部通过，含 12×1、中文与 emoji；缩放后完整，滚轮只滚回滚区；Status 五个重绘阶段均通过，覆盖 AC-2 至 AC-4 |

本地复核产物位于 `output/playwright/issue4/`：`capture.py`、`pty.ansi`、`mouse.ansi`、`verify.js`、`corpus/`。这些输出是本机验收材料，未作为快照基线提交。未修改字符快照；断言针对输出顺序、行数、模式和事件，不使用截图判断。

审查为本次自查。未宣称验证整个 VS Code 应用、所有终端、所有复用器组合或 Unix 挂起恢复。
