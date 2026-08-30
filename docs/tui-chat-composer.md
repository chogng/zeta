# Zeta TUI ChatComposer 边界

> 完整架构与交互决定见 [`tui-chat-architecture-discussion-v15.md`](tui-chat-architecture-discussion-v15.md)。当前源码区域见 [`zeta-code/tui/src/layout.md`](../zeta-code/tui/src/layout.md)。

## 快速理解

`ChatComposer` 不是整页容器。它只协调当前 Thread 的 `ChatInput`、Suggest 和 stacked Pane，不保存 Agent 请求或正文状态。

| 状态 | 所有者 |
| --- | --- |
| draft、Queue、正文滚动/选择/展开 | `features/thread/input.rs`，按 `ThreadId` 保存；正文交互使用 `TranscriptCellId` |
| 有序正文和命令调用 | `features/thread/transcript.rs`、`features/thread/exec_cell.rs` |
| Queue 身份、顺序、发送状态和 `/queue` 管理 | `features/queue.rs` |
| Session Manager、横向根页面、last viewed Thread | `features/sessions/` |
| Approval 请求、选择和提交 | `features/approval.rs` |
| Query 请求、自定义编辑器和提交 | `features/query.rs` |
| 普通页面几何 | `app/screen_layout.rs` |
| `/status` 只读覆盖层 | `components/quick_view.rs` + `features/status/` |

正常页面由 App 按 `Transcript → Goal → Plan → Queue → Query → ChatInput/Approval → StatusLine/KeyHints → SubagentPane` 分配。Query 位于普通输入框上方并保留草稿；Approval 替换普通输入区域；QuickView 和 Suggest 覆盖既有内容。

`ChatInput` 支持 Standard/Vim 两种本地编辑方式。补全弹层先处理导航、接受和 Esc；没有弹层时才由 Vim 的 Insert/Normal/Visual 状态或普通编辑器处理。该模式不改变 Pane、正文选择或应用级快捷键。
