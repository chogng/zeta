# Native Agent Console：产品结构与执行边界

> 状态：Current。本文定义 Native Agent-first Console 的 canonical 产品结构。terminal
> grid、BlockList 与 PTY compatibility 实现契约仍由
> [`native-terminal-ui.md`](native-terminal-ui.md) 和
> [`zeta-terminal` README](../../zeta-rs/terminal/README.md) 维护；Session、Thread、Turn 与
> ThreadItem 的权威语义由 [`protocol.md`](../../docs/protocol.md) 维护。

## 快速理解

Native 产品的主界面是一条 Agent Thread timeline。用户消息、Agent 消息、Agent 发起的 Tool
Call 和用户直接发起的 Shell command 都进入同一个 Thread；它们共享 Composer 与 presentation
primitive，但保持不同的提交语义。

```text
AgentWorkspace
├─ ThreadTimeline
│  ├─ UserMessage
│  ├─ AgentMessage
│  ├─ CommandCard
│  ├─ ApprovalRequest
│  └─ FileChange
├─ AgentComposer
│  ├─ ComposerInteractionPane → active View stack
│  │  └─ current: SlashCommandView | ModelPickerView
│  ├─ ComposerInfoBar → contextual hints (`/ for commands`)
│  ├─ ComposerEditor → CodeEditor(Compact)
│  ├─ ComposerContextToolbar
│  └─ Agent | Shell mode
└─ optional TerminalPane
```

Terminal 不再拥有产品 Session、Thread 或 transcript。普通 shell execution 作为 typed Tool
Call 投影到 CommandCard；只有 `vim`、`top`、`ssh` 等需要持续直接输入和 terminal protocol
compatibility 的程序进入独立 TerminalPane。

## 当前状态与目标

| 能力 | 当前实现 | 目标 |
| --- | --- | --- |
| Native 产品根 | `AgentWorkspace` + `ThreadTimeline` | ✅ |
| Composer | `AgentComposer` 显式 Agent/Shell mode | ✅ |
| Composer 可展开交互 Pane | 展示当前 View；Slash 状态委托 `zeta-slash-commands`，Native 拥有 `/model` 子 View 和 WGPU row projection | ✅ |
| Agent authority | App Server `session/subscribe` aggregate snapshot/update + gap resubscribe | ✅ |
| Agent Tool Call | durable ToolCall/ToolResult → CommandCard | ✅ |
| 用户直接 Shell | `session/request` → `StartShellTurn` → model-free durable Shell Turn | ✅ |
| 流式 Agent text | transient Item delta + durable final item | ✅ |
| 流式 Tool output | typed stdout/stderr delta + durable final ToolResult | 部分具备；local adapter 当前在捕获完成时发布分流 delta |
| 交互式 terminal | 独立 `WorkspaceSurface::Terminal`，`Cmd/Ctrl+J` 切换 | ✅；当前为全主区域 Surface |

`BlockList` 只保留为 terminal compatibility model，不再绘制 Agent 主界面，也不拥有 Agent
Thread transcript。

`zeterm` 的 embedded App Server 在 `agent_session` 中显式选择 `SessionStateMode::Ephemeral`：
当前进程内仍使用同一 Session/Thread/Core contract，但 Session、Thread 和用户消息不会从共享
profile SQLite 恢复或写入。`zeta-cli`/TUI 的默认 durable composition 不因此改变。

## 所有权

| 状态或能力 | Owner | Native 义务 |
| --- | --- | --- |
| Session、Thread、Turn 与 durable ThreadItem | Core / App Server | 订阅 snapshot/update，不复制 reducer |
| transient Agent/Tool delta | App Server update stream | 检测 stream cursor gap；gap 后重新订阅 |
| Timeline scroll、selection、collapse | Native presentation | 可丢弃、可从 snapshot 重建 |
| Composer text、mode、IME 与 caret | Native `ComposerEditor` + `zeta-editor::CodeEditorDocument` | mode 明确，不猜测输入内容 |
| Composer 面板与 interaction list geometry | [`zeta-composer`](../composer/README.md) | 只消费 logical bounds、preferred heights 与 item count，返回 panel、固定行、list viewport 和 selection scroll geometry |
| Composer 信息栏 | Native `composer_panel` | 消费 `zeta-composer` 返回的 info-bar bounds，展示当前 mode 的提示信息 |
| Composer 交互 Pane 宿主 | Native `composer_panel` + `ComposerInteractionPaneState` + `zeta-ui::{ScrollView,ListView}` | 只放置 active View、保留 viewport offset，并委托 `zeta-composer` 与通用 UI 基座完成 geometry、裁剪、滚动与可见范围；不解释 Slash、Model 或 Plan |
| Composer active View model | Native `ComposerInteractionModel` | 当前实现选择 Slash Command / Model Picker View；不拥有 Pane 几何与滚动 |
| Slash Command interaction model | `zeta-slash-commands::SlashCommandsState` | 消费初始化命令快照，拥有 query、selection、dismiss 与 completion；滚动属于各 renderer |
| Composer Changes 文件数与增删行 | `zeta-git::GitTextDiffSnapshot` | 只过滤工作区范围并展示；点击后刷新并展开 Changes Pane |
| Agent submission | `session/request` → `StartTurn` | 提交 UserInput 并消费 canonical result |
| 用户直接 Shell submission | Shell Turn contract | 经过同一 policy、approval、sandbox 与 durable commit |
| Agent-managed Tool execution | Core Tool scheduler | 只投影 ToolCall/ToolResult |
| Terminal grid、alternate screen 与 direct input | `zeta-terminal` + TerminalPane host | 不反推 Agent/Turn 状态 |
| Files、Changes 与 Diff presentation | Native sidebar + typed workspace services | 不从 terminal output 推断 |
| 主题 catalog、用户 JSON 与颜色变换 | `zeta-theme` + shared manifest | Native 只把 snapshot 映射为组件 palette，不维护第二份默认色表 |

## Composer 语义

同一输入框拥有显式模式：

```rust
enum ComposerMode {
    Agent,
    Shell,
}

enum ComposerSubmission {
    AgentMessage(String),
    ShellCommand(String),
}
```

Agent mode 通过 `session/request` 的 `StartTurn` operation 提交。Shell mode 通过同一 Session
request 的 `StartShellTurn` operation 提交；
不得增加 `session/request` 中 `StartTurn` 一类模糊参数，也不得根据文本是否像命令来猜 mode。

Composer 使用 `CodeEditorPresentation::Compact`：保留多行文档、selection、undo/redo 和 IME，
隐藏文件编辑器的 header、行号与 marker gutter。内容从紧凑基线自动增长，最多显示八行，超过后
由 retained viewport 跟随 caret。`Enter` 提交当前 mode，`Shift+Enter` 插入换行；Shell mode
在首行/末行边界使用 Up/Down 浏览已提交命令，并在返回末端时恢复尚未提交的 draft。该行为属于
`AgentComposer`，不属于通用 `CodeEditor`。

`zeta-composer::ComposerPanelLayout` 定义 Composer Panel 的固定顺序“信息栏 → 输入框 → 底部 toolbar”；
信息栏用于显示 `/ for commands`
等上下文提示。其上方的 Composer Interaction Pane 是可展开、可收起的 View 宿主：Panel 只为它
分配位置，`ComposerInteractionPaneState` 只保留 offset，`zeta-ui::ScrollView` / `ListView` 只根据
viewport 与 content geometry 完成裁剪、滚动、scrollbar 和可见范围投影。三者都不解释挂载的是
Slash、Model、Plan 还是未来的其他 View。当前 Agent mode 输入 `/` 时，Slash Command View 使其
在信息栏上方出现；没有 active View 时 Pane 收起。整个 Panel 增高并压缩 ThreadTimeline，不覆盖
已有消息，也不移动固定的三行。共享 `SlashCommandsState` 按当前无空格前缀过滤并保留键盘选择与
dismiss；Native renderer 单独保留滚动。根 View 关闭时交互区收起，子 View 关闭时返回上一层。
`/model` 将模型选择 View 压入同一个 View 栈。方向键移动选择，Enter 进入或确认，Tab 补全 Slash
Command，Escape 在子 View 中返回上一层、在根 View 中关闭。列表最多显示八行，超出后由方向键或
滚轮滚动；关闭时保留 Composer draft。模型目录来自 App Server，选中项以精确 `ModelRef` 写入
当前 Session。

Shell Turn 不调用模型。`StartShellTurn` 原子记录 Turn acceptance、精确 `shell-command`
ToolCall 与 Turn start，随后复用 Tool scheduler 的 policy、workspace sandbox、one-time
approval、unknown-outcome recovery 和 durable ToolResult；结束时直接记录 TurnCompleted。
Agent 后续可以从 Thread context 看见这些事实。

## 投影与恢复

Native 首次选择 Session 时调用 `session/subscribe`。App Server 在这个 Session port 内装配
Session snapshot、durable gap、child Thread projection 和 connection-local live update；Native
只选择 active Thread 并应用 projection，不直接访问 Thread subscription endpoint，也不实现第二份
Thread reducer：

- durable update 到达后，以 canonical snapshot/gap 推进 projection；
- transient `ItemStarted` / `ItemDelta` 只改善低延迟展示；
- `streamInstanceId` 改变或 sequence 出现空洞时，立即丢弃 transient buffer 并重新订阅；
- UI 不从 Markdown、terminal output 或当前可见文字推断 Turn terminal state；
- ToolCall 与 ToolResult 只按稳定 `toolCallId` 聚合成 CommandCard。

## CommandCard 与 TerminalPane

CommandCard 适用于有界、非交互式执行：

```text
Shell · Agent
$ cargo test
Compiling...
test result: ok
exit 0 · 4.2s
```

用户与 Agent 发起的命令使用相同 presentation，通过所属 Turn 的 kind 表达来源。stdout/stderr
delta 是 transient presentation；最终 ToolResult 是恢复 authority。

Terminal Surface 适用于持续交互程序。`Cmd/Ctrl+J` 在 Agent 与 Terminal Surface 间切换；
Terminal 激活后 keyboard/IME/paste 可直接编码到 PTY。后台 PTY 的 alternate-screen 状态不能
抢占 Agent Surface。Terminal transcript 不自动伪装为 ThreadItem；需要进入 Agent context 的
结果必须通过 typed Tool/attachment contract 提交。

## 当前限制与下一阶段

| 项目 | 当前边界 | 下一阶段 |
| --- | --- | --- |
| Tool output latency | stdout/stderr 类型与 UI 增量已贯通；local adapter 在进程完成后发布捕获结果 | `zeta-tool-executor` pipe reader 实时发布有界 chunk |
| Terminal layout | 独立全主区域 Surface | 可调整 TerminalPane / 多 Pane tree |
| Agent message | ThreadTimeline 基本文本布局 | Markdown block、selection、折叠与虚拟化 |
| Interaction UI | durable approval 已可恢复 | Timeline 内 approval card 与响应控件 |
| Composer interaction | Slash Command 与模型选择 View 栈已接入 | 继续增加只消费 typed catalog 的命令专属 View |
| Session UI | 通过 App Server Session/Thread snapshot 创建、投影并切换多个 Session Tab | 独立的 Session action mutation、跨窗口恢复与每个 tab 的完整本地 presentation state |
