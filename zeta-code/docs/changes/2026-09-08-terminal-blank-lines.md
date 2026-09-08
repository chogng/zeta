# issue #13：全屏对话与固定输入区

Zeta Code 改为在备用屏幕中绘制完整页面，ChatInput 和状态栏固定，历史由 Transcript 内容区统一滚动。该结构消除主屏幕追加协议产生的整页空行。来源：[issue #13](https://github.com/chogng/zeta/issues/13)。现行行为见[终端规格](../spec/terminal.md)和[布局规格](../spec/layout.md)，职责见 [TUI 设计](../design/tui.md#全屏内容区与固定交互区)。

## 目标与验收

| 要求 | 操作与结果 |
| --- | --- |
| AC-1 | 启动、输入、发送、流式更新和面板开关只重绘备用屏幕，不向主屏幕或终端回滚区追加空行 |
| AC-2 | Transcript 占据固定交互区以外的空间；ChatInput、TopTip 和 StatusLine 的位置不因消息数量变化 |
| AC-3 | Welcome、用户消息、Agent 回复、工具结果和本地命令保持顺序；后端快照不能删除或重排本地命令 |
| AC-4 | 内容溢出后无论 Enhanced TUI 是否开启，都可用滚轮、PageUp、PageDown、Ctrl+Home 和 Ctrl+End 浏览；到达已加载开头后继续读取历史分页 |
| AC-5 | 命令面板和浮层不写入 Transcript；退出、失败和挂起恢复进入前的终端画面与模式 |
| AC-6 | 输入框滚轮不召回或提交输入历史；真实键盘的 ↑ / ↓ 继续在多行移动与输入历史之间切换 |

## 根因

旧实现同时维护终端回滚区和可重绘交互区。启动时为主屏幕输出一个终端高度的换行，之后又通过清屏、换行和动态 viewport 模拟消息追加。首次发送或区域高度变化会把这些空行滚入 VS Code 的回滚区，形成整页空白；快照替换还依赖已经不可修改的终端历史保存本地命令。

本次对照本机 `../codex/codex-rs/tui` 的全屏与 inline 两条实现，并实测 Claude Code 2.1.263。用户选择全屏固定交互区，因此不继续复制 Codex 为终端原生回滚准备的自定义 inline 协议。

## 实施

- `TerminalSession` 使用备用屏幕，删除主屏幕正文追加、动态 viewport 和专用后端。
- `frame` 始终从完整 Transcript 绘制 Welcome、历史和流式内容；固定交互区由页面布局统一分配。
- TUI 始终捕获带位置的滚轮：内容区滚轮更新 Transcript 锚点，浮层独占自身滚动；Enhanced TUI 只控制点击、悬停、拖选和自动复制。
- 进入备用屏幕时保存并关闭 alternate scroll，退出时恢复；Zeta 的 xterm 宿主补充该模式的状态与滚轮路由，避免 xterm.js 无视 `1007l` 后继续合成方向键。
- `TranscriptModel` 在快照刷新时保留按前一后端条目定位的本地命令，并用新确认的用户消息替换本地提前显示的消息；切换 Thread 时清理旧 Thread 内容。
- 退场 `terminal/backend.rs`、`terminal/history_protocol_tests.rs`、`transcript/history.rs` 以及旧终端回放脚本。

## 验证

代码基线：`d8424edba492`。候选实现与测试文件按 `git diff --name-only -- zeta-code/tui zeta-code/cli/tests` 排序，拼接路径、NUL、文件内容或 `<deleted>`、NUL 后的 SHA-256 为 `21532ebbfaba8b47855accae24184eb01a079eef6c4fc67d99454700425f8fd3`。

| 验证 | 结果与覆盖 |
| --- | --- |
| `just check zeta-tui` | 通过，无警告；覆盖生产构建 |
| `just test zeta-tui --lib` | 700 通过、1 个真实 PTY 测试按约定忽略；覆盖状态、布局、Transcript、滚动、模式恢复和字符绘制 |
| `just test zeta-tui --lib app::event_loop::tests -- --nocapture` | 最终鼠标候选 8 通过、1 个真实 PTY 测试忽略；覆盖整屏滚动、选择、浮层消费和关闭增强后的残留事件 |
| `just test zeta-tui --lib snapshot_keeps_local_commands_in_order -- --nocapture` | 最终候选通过；覆盖快照确认用户消息前后本地命令的稳定顺序 |
| `just test zeta-cli --test tui_real_scenarios actual_tui_input_keeps_hint_bar_without_blank_line_growth -- --nocapture` | 100×32 与 60×16 通过；输入、回复、Status 开关前后 ChatInput 行号固定；审查并接受 `issue13/fullscreen_conversation.snap` |
| `just test zeta-cli --test tui_real_scenarios actual_tui_multiple_commands_preserve_internal_history_and_fixed_input -- --nocapture` | 两种尺寸通过；12 次 `/status`、12 轮消息和回复，真实鼠标滚轮进入历史，Ctrl+Home 回到 Welcome 与本地命令，Ctrl+End 回到 `MESSAGE-11` / `REPLY-11` |
| Windows ConPTY 单独运行 `real_terminal_mouse_handoff` | 通过；输出包含一次备用屏幕进入与退出、一次整屏鼠标捕获与释放 |
| 本机 VS Code 1.136.1 随附的 xterm.js 6.1.0 beta 与 ConPTY DLL，以及系统 ConPTY | 两条路径均为 `baseY = 0`；启动、逐字输入、Status 开关和发送后的 ChatInput 上边线始终在第 40 行；备用屏幕中没有 shell 内容或终端回滚页 |
| 变更文档相对链接检查 | 13 个变更文档无断链；全量 65 个文档仅保留既有 `openai-panel/source.sha256` 缺失项 |
| 快照检查 | 逐项审查 6 份更新和 1 份新增字符快照；无 `.snap.new` |
| `git diff --check -- . ':(exclude)**/*.snap'` | 通过；字符快照按现有格式有意保留终端行尾空格 |

### 2026-09-08 滚轮职责补充验收

在同一基线与工作区上补充 AC-4 与 AC-6。候选实现和测试文件按路径排序后的 SHA-256 为 `aa9e6cbb90db156a7090ea4d67b2ecab940c3f41b54f79419679e0bcec4169bb`：

```
zeta-code/tui/src/app/event_loop.rs
zeta-code/tui/src/app/event_loop_tests.rs
zeta-code/tui/src/app/frame.rs
zeta-code/tui/src/app/state.rs
zeta-code/tui/src/app/state_tests.rs
zeta-code/tui/src/config/editor_tests.rs
zeta-code/tui/src/nls.rs
zeta-code/tui/src/terminal/mouse.rs
zeta-code/tui/src/terminal/session.rs
zeta-code/tui/src/terminal/session_tests.rs
zeta-ts/src/zeta/workbench/contrib/terminal/browser/instance/alternateScroll.ts
zeta-ts/src/zeta/workbench/contrib/terminal/browser/instance/terminalInstanceWidget.ts
zeta-ts/src/zeta/workbench/contrib/terminal/test/browser/alternate-scroll.test.ts
```

| 验证 | 结果与覆盖 |
| --- | --- |
| `just check zeta-tui` | 通过；覆盖 TUI 生产构建 |
| `just test zeta-tui --lib` | 701 通过、1 个真实 PTY 测试按约定忽略；Enhanced TUI 关闭后内容区滚轮仍更新 Transcript，输入历史仍由 ↑ / ↓ 访问，终端模式获取与恢复通过 |
| `just test zeta-tui --lib real_terminal_mouse_handoff -- --ignored --nocapture --test-threads=1` | 通过；真实 ConPTY 输出顺序为进入备用屏幕、保存并关闭 alternate scroll、保留滚轮捕获，退出前恢复 alternate scroll 并释放鼠标 |
| 定向 TypeScript 编译 `terminalInstanceWidget.ts` | 通过；xterm 解析器、滚轮处理器和生产依赖类型正确 |
| 定向编译并运行 `alternate-scroll.test.ts` | 2 通过；覆盖默认、关闭、鼠标上报、重新开启和保存恢复 |
| Playwright + 本机 VS Code xterm.js | 关闭模式后滚轮不产生输入；鼠标上报开启时产生 SGR 滚轮事件；恢复模式后重新产生方向键，覆盖终端宿主真实事件路径 |

`zeta-ts` 全量单测预编译仍被本次修改之外的 Electron 类型缺失、Session/Chat 测试数据和协议类型不一致阻塞；Renderer 工程还保留既有 Chat、Debug 与 Thread 类型错误。本次定向编译和浏览器行为验证没有出现终端相关错误。

当前验证未覆盖 Windows Terminal、WezTerm、macOS、Linux 终端和 tmux/Zellij 组合。全屏方案不承诺终端原生回滚；对话历史由 Zeta 的 Transcript、分页和滚动入口负责。
