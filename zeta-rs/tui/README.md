# `zeta-tui`

> 本 README 解释当前 terminal loop、presentation state、snapshot polling 与 terminal cleanup。
> 更完整的交互客户端方向见 [`docs/tui.md`](../../docs/tui.md)，App Server client contract 见
> [`docs/app-server-client.md`](../../docs/app-server-client.md)。

`zeta-tui` 当前是一个小型同步 presentation shell：它通过已经初始化的 typed
`AppServerClient` 创建一个 Session/Thread，接受纯文本 prompt，启动或中断 Turn，轮询 canonical
Thread snapshot，并用 Ratatui 呈现消息与状态。

它不拥有 Agent runtime、Session/Thread reducer、App Server connection composition、model、
Tool、approval policy 或 persistence。

## 当前能力

- 启动时创建一个 product Session 和 root Thread；
- 单行文本 composer，支持 typing、Unicode-safe cursor/editing 与 bracketed paste；
- Enter 提交一个 text-only Turn；
- active Turn 期间每 25 ms event-loop iteration 读取 Thread snapshot；
- 以无外框 transcript 显示 latest completed Agent message、友好 failure、interruption 与
  waiting/cancelling state；
- 顶部显示低干扰的运行状态，底部使用圆角 composer 和只包含下一步操作的 footer；
- Ctrl-C、Ctrl-D（空输入）或 Esc：idle 时退出，active 时请求 interrupt；
- raw mode、alternate screen、bracketed paste 与 cursor cleanup；
- basic Unicode-aware wrapped-row estimation 和自动滚动到底部。

当前没有 Session browser、Thread navigation、Markdown、stream delta render、Tool transcript、
approval/user-input response UI、mouse/resize-specific state、remote connection selector 或 async event
pump。Slash discovery/completion 与 Vim mode/motion/operator 目前只有明确的组件所有权，尚未实现。
系统文档中的这些内容是演进方向，不是已实现功能。

从 repository root 启动当前 embedded TUI：

```bash
just zeta
```

等价的 Cargo 命令是：

```bash
cargo run --manifest-path zeta-rs/Cargo.toml -p zeta-cli
```

## Public contract

| Symbol | 职责 |
| --- | --- |
| `TuiOptions::new` | 提供启动时使用的 Session/Thread title |
| `run` | 拥有一次 terminal UI session，直到用户退出或 terminal/client failure |
| `TuiExit::UserRequested` | 正常退出原因 |
| `TuiError::Client` | typed App Server client failure |
| `TuiError::Terminal` | terminal setup/event/draw failure |

`run<T: JsonRpcTransport>` 接受一个已经初始化的 `AppServerClient<T>`。Transport/embedded/local/
remote 选择与 initialize/schema handshake 属于 CLI 和 app-server-client，不在 TUI 内重复实现。

## 文件与职责

```text
src/
├── lib.rs                    # RPC coordination + event loop + snapshot mapping
├── app.rs                    # global presentation status and keyboard-to-Action mapping
├── chatwidget/
│   └── mod.rs                # transcript state + top-pane coordination
├── toppane/
│   ├── mod.rs                # active interaction-surface routing
│   ├── chat_composer.rs      # submit semantics and slash extension boundary
│   └── textarea.rs           # text buffer, cursor and Vim extension boundary
├── render.rs                 # Ratatui layout/widgets
└── terminal.rs               # raw/alternate-screen/bracketed-paste lifecycle
```

实现 module 都是 private；crate 只导出启动 contract。

## 内部接口地图

| Symbol | 可见性 | 当前职责 | 方向约束 |
| --- | --- | --- | --- |
| `App` | crate-private | presentation `Status`、全局键与 `ChatWidgetOutcome` → `Action` | 不保存 canonical Thread authority 或编辑器细节 |
| `Action` | crate-private | `Quit`、`Interrupt`、`Submit(String)` | keyboard mapping 后才触发 I/O |
| `Status` | crate-private | Ready/Working/waiting/Cancelling/Error display state | 只能由 canonical snapshot/result驱动 |
| `App::handle_key` | crate-private | 先委托局部输入，再处理未消费的全局键 | 不直接调用 client |
| `App::quit_or_interrupt` | private | active state interrupt；idle/error quit | Cancelling 不重复发送 interrupt |
| `ChatWidget` | crate-private | transcript 与 sibling `TopPane` 协调 | 不拥有产品 lifecycle、RPC 或 Vim keymap |
| `ChatWidget::handle_key` | crate-private | `TopPaneOutcome` → `ChatWidgetOutcome`，提交时记录 user message | 不执行提交动作 |
| `TopPane` | crate-private | 把当前交互面的键盘和 paste 委托给 composer | 不解释 slash 或编辑文本 |
| `ChatComposer` | private | blank/trim/submit 语义；slash discovery/completion 的扩展 owner | 不拥有 cursor、Vim state 或 RPC |
| `TextArea` | private | UTF-8 buffer、byte-safe cursor、insert/delete/movement；Vim 的扩展 owner | 不解释 Enter submission 或 slash command |
| `submit_prompt` | private | build typed `TurnStartParams` 并更新 sequence | 不手写 method string/JSON |
| `refresh_turn` | private | `thread/read` + update local sequence + drain notifications | snapshot 是当前 authoritative UI source |
| `interrupt_turn` | private | typed Turn interrupt + refresh | 使用当前 Thread sequence |
| `apply_active_turn_snapshot` | private | canonical Turn status/items → presentation state | 不从 log/text 猜 terminal state |
| `present_turn_error` | private | stable Turn error code → user-facing recovery message | 不显示 Rust Debug/provider secret |
| `request_key` | private | process ID + wall-clock nanos command ID | 一次逻辑 command 一个新 ID |
| `render::draw` | crate-private | history/input/status layout 与 cursor | 不改变 App state |
| `estimated_wrapped_rows` | private | Unicode display-width based scroll estimate | width 0 不 panic |
| `TerminalSession::open` | crate-private | 进入 raw/alternate/paste并创建 backend | partial failure 必须 rollback |
| `Drop for TerminalSession` | private impl | 恢复 terminal modes 与 cursor | 所有退出路径都依赖 RAII |

本地 `app::Status` 是 presentation state，不是 `zeta_protocol::TurnStatus` 的复制品。它可以包含
`Error(String)` 供显示，但不能被其他层当作 domain fact。

## 启动与事件循环

```text
run(client, options)
├─ client.create_session
├─ client.create_session_thread
├─ TerminalSession::open
├─ App::new
└─ loop
   ├─ if active/waiting/cancelling: refresh_turn
   ├─ TerminalSession::draw → render::draw
   ├─ event::poll(25 ms)
   └─ event::read
      ├─ key → App::handle_key
      │  ├─ local input → ChatWidget → TopPane → ChatComposer → TextArea
      │  ├─ Quit → return
      │  ├─ Submit → submit_prompt
      │  └─ Interrupt → refresh + interrupt_turn
      └─ Paste → App → ChatWidget → TopPane → ChatComposer → TextArea
```

Session create 和 Thread create 使用独立 `CommandId`。Turn start/interrupt 使用当前
`thread_sequence` 作为 expected sequence；client error 会进入 visible error message/status，不退出
terminal session。

当前 create 后把 initial Thread sequence 设为 `1`，依赖 newly-created Thread 的 established
sequence contract。若 create result/schema 改为返回 sequence，应该移除这个 implicit assumption，
而不是在多个 UI path 复制常量。

## Snapshot → UI mapping

`apply_active_turn_snapshot` 只观察当前 `active_turn`：

| Canonical `TurnStatus` | UI effect |
| --- | --- |
| `Created` / `Running` | `Status::Working` |
| `WaitingForApproval` | waiting status；仍可 interrupt |
| `WaitingForUserInput` | waiting status；当前不能 resolve |
| `WaitingForCapability` | waiting status；当前不能 resolve |
| `Cancelling` | `Status::Cancelling`，抑制重复 interrupt |
| `Completed` | 取该 Turn 最后一个 `AgentMessage`，返回 Ready |
| `Failed` | 显示 stable Turn error，清除 active turn |
| `Interrupted` | 添加 notice，返回 Ready |

Completed Turn 没有 Agent message 会被显示为 error。已知 stable Turn error 由
`present_turn_error` 映射成面向用户的恢复提示，错误详情只在 transcript 出现一次；footer 只说明
可以 retry 或退出。Reasoning、Plan、ToolCall、ToolResult 与多个 Agent message 当前不呈现在
transcript；UI 只取最后一个 Agent message。

`refresh_turn` 每次成功 read 都用 snapshot sequence 覆盖 local expected sequence。它随后调用
`drain_notifications`，但当前 presentation 不消费 notification payload 来增量更新 projection；
Thread snapshot polling 才是 authority。这保证实现简单，也意味着高延迟和额外 read traffic。

## Keyboard state machine

```text
Ready / Error
├─ Enter(non-empty) → Submit → Working
├─ Esc / Ctrl-C / empty Ctrl-D → Quit
└─ typing/paste/cursor movement/editing accepted

Working / Waiting*
├─ Esc / Ctrl-C / empty Ctrl-D → Interrupt → Cancelling
└─ typing/paste/second submit ignored

Cancelling
└─ further quit/interrupt keys ignored until snapshot terminal state
```

`record_interrupt_failure` 把状态恢复到 Working，使用户可以再次请求 interrupt；ordinary client
failure 进入 Error 并允许输入新 prompt。

## Terminal lifecycle

`TerminalSession::open` 按以下顺序获取资源：

```text
enable_raw_mode
→ EnterAlternateScreen
→ EnableBracketedPaste
→ Terminal::new
→ clear
```

每个 partial initialization failure 都尽量回滚已获取的 mode。成功后 `Drop` 无条件尝试：

```text
disable_raw_mode
→ DisableBracketedPaste
→ LeaveAlternateScreen
→ show_cursor
```

Cleanup error 被忽略是 Drop 路径的刻意选择，避免 panic during unwind。新增 terminal capability
时必须同时更新 partial-failure rollback 与 Drop reverse cleanup。

## Rendering

当前 layout 是固定四段：

1. 两行轻量 header，包括产品名和 canonical status 的 presentation label；
2. expandable、无外框的 transcript，空会话显示 centered welcome；
3. 三行圆角 composer；
4. 一行 recovery/help footer。

Transcript marker 使用 role-specific color，正文是 plain text。`estimated_wrapped_rows` 使用
`unicode_width::UnicodeWidthStr`，把 label width 计入首行，然后计算 bottom scroll。它是估算，
不处理完整 grapheme/reflow/Markdown layout。

## 方向偏差检查

- TUI 直接依赖 Core/store/model：绕过 App Server product boundary；
- TUI 手写 method string 或 JSON：typed client/protocol source 被绕过；
- `App` 保存本地 reducer/command receipt：presentation 变成第二 authority；
- 从 stderr/log/notification text 推断 Turn terminal state：canonical snapshot 被绕过；
- `TerminalSession` 新增 mode 但 Drop 不恢复：退出后破坏用户 terminal；
- `render::draw` 修改 state 或发 RPC：view 与 coordination 耦合；
- retry 逻辑生成新 CommandId 却重用同一 intent：idempotency semantic 被破坏；
- 把 `TerminalSession`、RPC connection 和 product `Session` 命名/生命周期混为一谈；
- docs 中把 planned Markdown/approval/streaming 写成当前能力。

## 同步修改关系

| 修改 | 必须同步检查 |
| --- | --- |
| 新 `Status` | `handle_key`/`quit_or_interrupt`、run polling guard、snapshot mapping、render status、tests |
| 新 `Action` | keyboard mapping、run action match、I/O failure behavior |
| 新 canonical Turn state/item | `apply_active_turn_snapshot`、render behavior、protocol compatibility |
| 新 terminal mode | `open` rollback、`Drop` cleanup、manual terminal recovery |
| Composer behavior | `accepts_input`、paste/key handling、cursor width、app tests |
| Incremental notifications | durable sequence/cursor projection、gap/resync、client event pump、snapshot fallback |

## 测试、限制与演进

```text
cargo test -p zeta-tui
bazel test //zeta-rs/tui:tui-unit-tests
```

Tests 当前覆盖局部键到全局键的 routing、trimmed/blank submit、Unicode cursor/editing、paste at
cursor、quit/interrupt keyboard semantics、duplicate interrupt suppression、input lock、
response/error/interrupted transitions、snapshot terminal/wait/resume mapping，以及 transcript chrome、
error 去重、role label/Unicode/zero-width wrapping。

Render tests 使用 Ratatui `TestBackend` 固定 empty/error surface 和 row estimation，但还没有完整
snapshot/golden terminal test；`run` 也没有完整的 fake transport event-loop integration test。
下一阶段优先级应是 async request/notification pump、subscription projection + gap resync、
interaction resolve UI 和 richer transcript。演进时继续让 TUI 可丢弃、可重建，并始终通过
typed App Server client。
