# `zeta-tui` 源码结构

> 界面区域见 [`layout.md`](layout.md)，crate 契约见 [`../README.md`](../README.md)，设计依据见 [`docs/tui-chat-architecture-discussion-v15.md`](../../../docs/tui-chat-architecture-discussion-v15.md)。

## 目录职责

| 路径 | 负责 | 不负责 |
| --- | --- | --- |
| `app/` | 单写者协调、全局布局、根页面与焦点路由、请求完成分发 | Queue、Session、Subagent、Goal、Plan 或交互状态机 |
| `features/` | 产品功能状态与展示：Sessions、Thread、Queue、Approval、Query、StatusLine，以及 TUI 主题文件和选择 | 通用编辑器和终端外壳 |
| `components/` | ChatInput、ChatHistory、Pane、QuickView、KeyHints、列表等可复用 UI 机制 | 产品事实、请求和外部副作用 |
| `render/` | 纯布局、文本、高亮、TUI 完整调色板和终端色彩转换 | 主题文件、产品状态和外部副作用 |
| `client/` | 类型化事件和异步请求完成投递 | 产品状态归约 |
| `host/`、`terminal/`、`ui/` | 系统适配与终端生命周期 | Session/Thread 流程和主题所有权 |

依赖只向下：`app → features → components → ui`。`components` 不依赖 `features`。

## 关键所有权

- `features/sessions/` 维护 `Manager ←→ Sessions`、每个 Session 最后查看的 Thread，以及 Manager 列表状态。
- `features/thread/input.rs` 按 `ThreadId` 保存草稿、Queue 和正文展示状态；选择、展开和滚动锚点使用 `TranscriptCellId`。`transcript.rs` 维护有序 `TranscriptCell`，`exec_cell.rs` 从首个 `ToolCallId` 确定单元身份，并按精确 `ToolCallId` 路由、限制命令输出；`subagent_pane.rs` 只使用 Session 的真实 Thread 树。
- `features/queue.rs` 拥有稳定 `QueueId`、顺序、发送状态、Inline 与 `/queue` Pane；ChatInput 只返回完整待排队内容。
- `features/approval.rs` 和 `features/query.rs` 分别拥有自己的请求绑定、选择、提交和错误状态；Query 的自定义文本不进入 ChatInput。
- `features/status_line/` 把运行状态、plan/queue 数量、后台 Agent/Subagent 数量和配置项组成最多两行；`features/status/` 单独提供 `/status` QuickView。
- `features/config/` 解释并通过 App Server 读写 `config.toml` 的完整 `[tui]` 表；`features/theme/` 只读取 `zeta-code/themes/*.json`，`render/theme.rs` 定义 TUI 调色板。它们都不依赖图形界面主题链。
- `components/chat_input/completion/` 与 `vim.rs` 分别拥有 ChatInput 的 Slash/Mention/Skill 补全和 Insert/Normal/Visual 编辑状态；补全先处理按键，Vim 不接管应用级导航。

## `ChatComposer` 边界

`ChatComposer` 只协调 Thread-owned `ChatInput` 与 stacked Pane 的输入优先级和提交目标。补全状态归 `ChatInput`；它不拥有 Approval、Query、Queue、Goal、Plan、Session、Thread 草稿或正文选择状态。

完整页面顺序固定为：

```text
Transcript → Goal → Plan → Queue → Query → ChatInput/Approval → StatusLine/KeyHints → SubagentPane
```
