# `zeta-tui` 源码结构

> 界面区域见 [`layout.md`](layout.md)，crate 契约见 [`../README.md`](../README.md)，界面类型和生命周期见 [`zeta-code/docs/tui.md`](../../docs/tui.md#131-界面结构输入位置与覆盖层)。

## 目录职责

| 路径 | 负责 | 不负责 |
| --- | --- | --- |
| `app/` | 单写者协调、`TerminalScreen` 切换、`ComposerMode`、一个 Overlay、全局布局和请求完成分发 | feature 内部流程、ChatInput completion 和正文绘制细节 |
| `features/` | 产品功能状态与展示：Sessions、Thread、Queue、Approval、Query、StatusLine，以及 TUI 主题文件和选择 | 通用编辑器和终端外壳 |
| `components/` | ChatInput、ChatHistory、Overlay、KeyHints、列表等可复用 UI 机制 | 产品事实、请求和外部副作用 |
| `render/` | 纯布局、文本、高亮、TUI 完整调色板和终端色彩转换 | 主题文件、产品状态和外部副作用 |
| `client/` | 类型化事件和异步请求完成投递 | 产品状态归约 |
| `host/`、`terminal/`、`ui/` | 系统适配与终端生命周期 | Session/Thread 流程和主题所有权 |

依赖只向下：`app → features → components → ui`。`components` 不依赖 `features`。

## 关键所有权

- `features/sessions/` 维护 `TerminalScreen::Manager/Session`、每个 Session 最后查看的 Thread，以及 Manager 列表状态；切换界面时由 App 关闭 `ComposerMode` 和 Overlay。
- `features/thread/input.rs` 按 `ThreadId` 保存草稿、Queue 和正文展示状态；选择、展开和滚动锚点使用 `TranscriptCellId`。`transcript.rs` 维护有序 `TranscriptCell`，`exec_cell.rs` 从首个 `ToolCallId` 确定单元身份，并按精确 `ToolCallId` 路由、限制命令输出；`subagent_picker.rs` 只使用 Session 的真实 Thread 树。
- `features/queue.rs` 拥有稳定 `QueueId`、顺序、发送状态、Inline 与 `/queue` picker；ChatInput 只返回完整待排队内容。
- `features/approval.rs` 和 `features/query.rs` 分别拥有自己的请求绑定、选择、提交和错误状态；Query 的自定义文本不进入 ChatInput。
- `features/status_line/` 把运行状态、plan/queue 数量、后台 Agent/Subagent 数量和配置项组成最多两行；`features/status/` 单独提供 `/status` Overlay。
- `features/config/` 解释并通过 App Server 读写 `config.toml` 的完整 `[tui]` 表；`features/theme/` 只读取 `zeta-code/themes/*.json`，`render/theme.rs` 定义 TUI 调色板。它们都不依赖图形界面主题链。
- `components/chat_input/completion/` 与 `vim.rs` 分别拥有 ChatInput 的 Slash/Mention/Skill 补全和 Insert/Normal/Visual 编辑状态；补全先处理按键，Vim 不接管应用级导航。
- `features/config/editor.rs`、`keymap/editor.rs` 和 `theme/picker.rs` 各自保存自己的多步页面；`components/list_selection` 只提供列表交互、action 绑定、绘制和命中。

## `ChatComposer` 边界

`ChatComposer` 只协调 Thread-owned `ChatInput` 的 Start、Queue、Steer 提交目标。补全状态归 `ChatInput`；`ComposerMode` 和 Overlay 归 App，Approval、Query、Queue、Goal、Plan 等状态归对应 feature。

结构类型只有两种：`TerminalScreen` 决定整屏内容，Overlay 覆盖当前帧且不改变高度。其他内容是界面直接排列的普通组件；组件报告多少行只是布局数据。`ComposerMode` 仅记录 Session 输入位置当前显示什么。ChatInput completion 由 ChatInput 保存，按 Overlay 的顺序最后绘制，但不进入 App 的 Overlay 状态。

Session 界面的占高顺序固定为：

```text
Transcript → Goal → Plan → Queue → Query → ChatInput/Approval/ComposerMode → StatusLine/KeyHints → SubagentPicker
```
