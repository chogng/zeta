# `zeta-tui`

> 本 README 解释当前 terminal loop、presentation state、snapshot polling 与 terminal cleanup。
> 更完整的交互客户端方向见 [`docs/tui.md`](../../docs/tui.md)，App Server client contract 见
> [`docs/app-server-client.md`](../../docs/app-server-client.md)。

`zeta-tui` 当前是一个小型同步 presentation shell：它通过已经初始化的 typed
`AppServerClient` 创建一个 Session/Thread，接受文本、本地图片路径与系统剪贴板图片输入，
启动或中断 Turn，轮询 canonical Thread snapshot，并用 Ratatui 呈现消息与状态。

它不拥有 Agent runtime、Session/Thread reducer、App Server connection composition、model、
Tool、approval policy 或 persistence。

## 当前能力

- 启动时创建一个 product Session 和 root Thread；
- 单行文本 composer，支持 typing、Unicode-safe cursor/editing 与 bracketed paste；超过 1000
  个 Unicode scalar value 的 paste 会显示为原子占位符，并只在提交时展开；
- 粘贴 PNG/JPEG/GIF/WEBP 本地文件路径时立即读取最多 16 MiB 的图片，显示可原子编辑的
  `[Image #N]` 占位符，并以结构化图片项提交；
- `Ctrl-V` 从系统剪贴板读取图片；文件列表与原始 RGBA 位图都会统一进入同一附件占位符和
  结构化提交路径；
- `@` 在当前 workspace 打开 mention popup；`zeta-file-search::PathSearchHandle` 使用与 Codex
  file search 相同 revision 的完整 `nucleo` engine，在后台增量扫描和匹配路径，支持 fuzzy
  排序、命中字符高亮、循环选择、Tab/Enter completion、Esc dismiss 和左键选择；选中路径作为
  原子文本插入；
- `/` 打开 command popup，支持 cursor-aware prefix filtering、循环选择、保留已有参数尾部的
  Tab completion、Esc dismiss 与左键单击可见命令；
- `/resume`、`/clear`、`/fork`、`/model` 与 `/new` 可解析 inline arguments，并在执行前展开
  large-paste placeholder；product command 明确拒绝 image arguments；
- command popup 只注册已有真实执行流的 built-ins：`/status`、`/skills`、`/mcp`、`/resume`、
  `/clear`、`/config`、`/fork`、`/help`、`/model`、`/new`、`/quit` 与 `/exit`；
- `/help` 使用保留 composer 的 interaction view stack 打开 Commands/Keys 双 Tab selection
  surface；支持直接输入搜索、左右键或 Tab/BackTab 循环切页、上下键循环选择，以及 Esc/Ctrl-C
  返回 composer；
- `/skills` 通过 typed `skills/list` 打开同一 interaction surface，提供
  All/Enabled/Disabled/Errors tabs、数量、搜索和 source-qualified metadata；`Space` 通过
  revision-checked `skill/enablement/set` 切换所选 Skill，`skills/changed` 会刷新仍在前台的页面；
  该页面不把 enablement 冒充为正文 activation；
- Session/Thread 命令调用 typed create/list/read/fork API 并切换当前 conversation；配置命令调用
  `config/read`，`/model` 使用 expected revision 更新 preferred model；
- 启动时读取 client 保存的 `initialize.slashCommands` snapshot，与 built-ins 做防冲突合并；
  server-advertised command 保留 `/name`、inline text/image/large-paste 参数并作为普通 Turn
  input 提交；
- Enter 按 composer 顺序提交由 text/image items 组成的 Turn；
- active Turn 期间每 25 ms event-loop iteration 读取 Thread snapshot；
- 以无外框 transcript 显示 latest completed Agent message、友好 failure、interruption 与
  waiting/cancelling state；
- 顶部显示低干扰的运行状态；composer 上方右对齐显示 preferred model 与 workspace，并按宽度
  依次使用短值或省略号；composer 只使用上下两条浅灰分隔线，footer 只包含下一步操作；
- Ctrl-C、Ctrl-D（空输入）或 Esc：idle 时退出，active 时请求 interrupt；
- raw mode、alternate screen、bracketed paste、mouse capture 与 cursor cleanup；
- basic Unicode-aware wrapped-row estimation 和自动滚动到底部。

当前没有 Session browser、Thread navigation、Markdown、stream delta render、Tool transcript、
approval/user-input response UI、resize-specific state、remote connection selector 或 async App
Server event pump。Mouse support 当前覆盖 slash 与 file mention popup 左键命中，不包含 hover、滚轮或其他
surface；缺少 typed backend contract 的 login、plugins、hooks、compact、service tier 等
命令不会进入 registry。Vim mode/motion/operator 目前只有明确的组件所有权，尚未实现。file
mention 只插入 workspace-relative 文本路径，不是 `app://`/`plugin://`
结构化 Mention，也不会读取文件内容。系统剪贴板图片依赖本机 clipboard backend；远程 SSH/
tmux 会话尚无 terminal-mediated image clipboard fallback。
status line 当前没有 Git、usage 或用户自定义 item/order；Git 后续应通过 `zeta-git` 的公开
异步接口进入更新路径，usage 必须等待 App Server typed snapshot 提供，不能从 transcript
推导。完整边界见 [`docs/tui.md` 的 status line 规划](../../docs/tui.md#111-status_line接口结果的展示投影)。
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
| `TuiOptions::new` | 提供 Session/Thread title，并默认以当前目录作为 file mention root |
| `TuiOptions::with_workspace_root` | 显式覆盖有界 file mention root |
| `run` | 校验 initialize snapshot、构造 runtime slash registry，并拥有 terminal UI session 直到退出或失败 |
| `TuiExit::UserRequested` | 正常退出原因 |
| `TuiError::Client` | typed App Server client failure |
| `TuiError::Terminal` | terminal setup/event/draw failure |

`run<T: JsonRpcTransport>` 接受一个已经初始化的 `AppServerClient<T>`。Transport/embedded/local/
remote 选择与 initialize/schema handshake 属于 CLI 和 app-server-client，不在 TUI 内重复实现；
TUI 只读取 client 保存的 immutable initialize result，并在创建 Session 前拒绝非法/冲突的
server slash snapshot。

## 文件与职责

```text
src/
├── lib.rs                    # RPC coordination + event loop + snapshot mapping
├── app.rs                    # global presentation status and keyboard-to-Action mapping
├── clipboard.rs              # native clipboard file/RGBA image read and PNG encoding
├── file_search.rs            # @file handle lifecycle, current-query filtering and snapshot polling
├── status_line.rs            # model/workspace display projection and width degradation
├── status_line_tests.rs      # status-line projection and Unicode width tests
├── chatwidget/
│   └── mod.rs                # transcript state + top-pane coordination
├── toppane/
│   ├── mod.rs                # composer + temporary interaction view stack routing
│   ├── attachments.rs        # image-path loading and atomic placeholder bindings
│   ├── chat_composer.rs      # submit orchestration, popup key routing and local dispatch
│   ├── mentions/             # @token parsing, async result application and popup state
│   ├── pending_pastes.rs     # large-paste placeholders and deferred payload expansion
│   ├── selection.rs          # generic tabs/search/filter/selection state
│   ├── slash_command_popup.rs # slash selection and dismissal state
│   ├── slash_input.rs        # cursor parsing, completion ranges and inline submission recognition
│   ├── slash_commands.rs     # built-in/dynamic command metadata and validated registry
│   └── textarea.rs           # text buffer, cursor and Vim extension boundary
├── render/
│   ├── mod.rs                # frame layout and render ordering
│   ├── header.rs             # product/status header
│   ├── history.rs            # transcript and empty state
│   ├── status_line.rs        # right-aligned context row
│   ├── composer.rs           # input surface and cursor
│   ├── mention_popup.rs      # workspace file mention overlay and hit testing
│   ├── selection_view.rs     # expanded interaction selection surface
│   ├── slash_command_popup.rs # slash command overlay
│   ├── footer.rs             # status-specific key hints
│   ├── layout.rs             # shared narrow layout helpers
│   └── theme.rs              # shared presentation colors
├── slash_command_dispatch.rs # executable built-in command flows + active conversation selection
└── terminal.rs               # raw/alternate-screen/paste/mouse lifecycle
```

实现 module 都是 private；crate 只导出启动 contract。

## 内部接口地图

| Symbol | 可见性 | 当前职责 | 方向约束 |
| --- | --- | --- | --- |
| `App` | crate-private | presentation `Status`、全局键与 `ChatWidgetOutcome` → `Action` | 不保存 canonical Thread authority 或编辑器细节 |
| `Action` | crate-private | `Quit`、`Interrupt`、`PasteImage`、`Submit(ComposerSubmission)` | keyboard mapping 后才触发 I/O |
| `Status` | crate-private | Ready/Working/waiting/Cancelling/Error display state | 只能由 canonical snapshot/result驱动 |
| `StatusLineModel` | crate-private | 直接把 config/workspace 接口结果变成长短展示值并执行宽度降级 | 不查询接口、不保存领域 authority、不渲染 |
| `App::apply_config_snapshot` | crate-private | 将 `ConfigReadResult` 映射进 status-line projection | 只在 update path 调用，不复制 config domain |
| `App::handle_key` | crate-private | 先委托局部输入，再处理未消费的全局键 | 不直接调用 client |
| `App::activate_slash_command` | crate-private | 将鼠标命中的 command index 委托给 composer 并复用 command dispatch | 不计算 terminal geometry |
| `App::quit_or_interrupt` | private | active state interrupt；idle/error quit | Cancelling 不重复发送 interrupt |
| `ChatWidget` | crate-private | transcript 与 sibling `InteractionPane` 协调 | 不拥有产品 lifecycle、RPC 或 feature catalog |
| `ChatWidget::handle_key` | crate-private | `InteractionPaneOutcome` → `ChatWidgetOutcome`，提交时记录 user message | 不执行提交动作 |
| `InteractionPane` | crate-private | 保留 composer、拥有 temporary view stack，并把 key/paste 路由到 active view 或 composer | 不保存 Plugin/Session 等产品 feature 状态 |
| `SelectionViewState` | crate-private | tabs、搜索 query、过滤索引、选择与循环导航 | 不执行 action、不依赖产品 ID 或 App Server |
| `ChatComposer` | private | blank/trim/submit、paste routing、slash completion application、参数结构化与 local dispatch | 不自行实现 slash grammar，不拥有 cursor、Vim state 或 RPC |
| `Attachments` | private | 图片 bytes/path、data URL 与原子占位符绑定、删除后重新编号 | 不直接读取系统 clipboard、不发 RPC、不渲染 |
| `clipboard::read_image` | crate-private | 从本机 clipboard 文件列表/RGBA image 读取并统一编码 PNG | 不改变 composer、不发 RPC、不持久化临时文件 |
| `FileSearchManager` | crate-private | 按当前 `@token` 创建/停止 `PathSearchHandle`，非阻塞 drain snapshot 并丢弃旧 query 结果 | 不解析输入、不保存 popup state、不读取文件内容 |
| `Mentions` / `MentionPopup` | private | `@token` query/range、异步结果应用、选择/关闭和原子路径补全 | 不扫描 workspace、不拥有 worker、不构造结构化 app/plugin Mention |
| `PendingPastes` | private | 超过 1000 字符的 text-paste payload、唯一占位符与提交时展开 | 不识别或保存图片，不解释 slash、不渲染、不直接提交 |
| `SlashCommandPopup` | private | 缓存 cursor-query/registry-derived matches、selection 与 dismissal state | 不解析输入、不执行命令、不渲染 Ratatui widget |
| `SlashInput` / `SlashCompletion` | private | 解析 cursor 下的 command token、返回替换 range、识别 bare/inline submission 和 command element range | 不改变 editor/popup、不执行命令 |
| `SlashCommandRegistry` / `SlashCommandItem` | private / crate-private | 合并 built-in 与已校验 dynamic metadata，为 discovery 和 submission 提供同一 snapshot | 不决定 product availability、不执行 App Server operation |
| `SlashCommandInvocation` | crate-private | command identity、trimmed display arguments 与有序 text/image argument items | 不执行 RPC |
| `ActiveConversation` | crate-private | 当前 Session/Thread identity、sequence 与 typed built-in command execution | 不解析 composer text、不拥有 App Server |
| `TextArea` | private | UTF-8 buffer、byte-safe cursor、原子元素 insert/delete/movement；Vim 的扩展 owner | 不保存 paste payload，不解释 Enter submission 或 slash command |
| `submit_prompt` | private | build typed `TurnStartParams` 并更新 sequence | 不手写 method string/JSON |
| `refresh_turn` | private | `thread/read` + update local sequence + drain notifications | snapshot 是当前 authoritative UI source |
| `interrupt_turn` | private | typed Turn interrupt + refresh | 使用当前 Thread sequence |
| `apply_active_turn_snapshot` | private | canonical Turn status/items → presentation state | 不从 log/text 猜 terminal state |
| `present_turn_error` | private | stable Turn error code → user-facing recovery message | 不显示 Rust Debug/provider secret |
| `request_key` | private | process ID + wall-clock nanos command ID | 一次逻辑 command 一个新 ID |
| `render::draw` | crate-private | frame 分区并按顺序协调 header/history/popup/status-line/composer/footer renderer | 不改变 App state |
| `render::{slash_command_index_at,mention_index_at}` | crate-private | 使用与各自 renderer 相同的 popup geometry 映射可见行点击 | 不执行命令、不改变选择状态 |
| `render::{header,history,status_line,composer,mention_popup,slash_command_popup,footer}` | private modules | 各自拥有一个 presentation surface | 不处理输入、不改变 App state |
| `estimated_wrapped_rows` | private | Unicode display-width based scroll estimate | width 0 不 panic |
| `TerminalSession::open` | crate-private | 进入 raw/alternate/paste/mouse mode 并创建 backend | partial failure 必须 rollback |
| `Drop for TerminalSession` | private impl | 恢复 terminal modes 与 cursor | 所有退出路径都依赖 RAII |

本地 `app::Status` 是 presentation state，不是 `zeta_protocol::TurnStatus` 的复制品。它可以包含
`Error(String)` 供显示，但不能被其他层当作 domain fact。

## 启动与事件循环

```text
run(client, options)
├─ client.create_session
├─ client.create_session_thread
├─ TerminalSession::open
├─ App::for_workspace
│  ├─ FileSearchManager::new
│  └─ StatusLineModel::for_workspace
├─ client.read_config → App::apply_config_snapshot
└─ loop
   ├─ App::poll_background_events → FileSearchManager::poll → Mentions
   ├─ if active/waiting/cancelling: refresh_turn
   ├─ TerminalSession::draw → render::draw
   ├─ event::poll(25 ms)
   └─ event::read
      ├─ key → App::handle_key
      │  ├─ local input → ChatWidget → InteractionPane
      │  │  ├─ active selection view → local view state
      │  │  └─ no active view → ChatComposer → TextArea
      │  ├─ Ctrl-V → PasteImage → clipboard::read_image → Attachments
      │  ├─ Quit → return
      │  ├─ Submit → submit_prompt
      │  └─ Interrupt → refresh + interrupt_turn
      ├─ left mouse down → render::{mention_index_at,slash_command_index_at}
      │  ├─ mention hit → App::activate_mention → atomic path completion
      │  └─ slash hit → App::activate_slash_command → existing command dispatch
      └─ Paste → App::handle_paste → ChatWidget → InteractionPane
         ├─ active selection view → search query
         └─ no active view → ChatComposer
         ├─ image path → Attachments + TextArea atomic placeholder
         └─ text → PendingPastes + TextArea
```

Session create 和 Thread create 使用独立 `CommandId`。Turn start/interrupt 使用当前
`thread_sequence` 作为 expected sequence；client error 会进入 visible error message/status，不退出
terminal session。

当前 create 后把 initial Thread sequence 设为 `1`，依赖 newly-created Thread 的 established
sequence contract。若 create result/schema 改为返回 sequence，应该移除这个 implicit assumption，
而不是在多个 UI path 复制常量。

`Event::Paste` 与普通 key editing 使用不同入口。`PendingPastes` 先把 CRLF/CR 规范化为 LF，
再以 Rust `char`（Unicode scalar value）数量判断大小：不超过 1000 时直接写入 `TextArea`；超过阈值时写入
`[Pasted Content N chars]` 原子元素并在内部保留原文。相同字符数的多个待提交 paste 使用
`#2`、`#3` 后缀避免绑定歧义。移动光标会整体跨过占位符，删除占位符会同时丢弃对应 payload；
`ChatComposer::submit` 在 trim、slash recognition 和 user-message recording 之前展开仍然存在的
占位符。

图片 paste 先尝试把完整字符串解释为本地文件路径；支持引号包裹和 shell 风格反斜杠转义。
`Attachments` 按文件签名识别 PNG/JPEG/GIF/WEBP，拒绝超过 16 MiB 的文件，并立即编码为
base64 data URL，避免提交时路径失效。占位符绑定到稳定 `TextElementId`，光标移动和删除保持
原子性，删除后剩余图片会重新编号。提交时 `ChatComposer` 按草稿顺序生成 text/image items；
展示记录保留 `[Image #N]`，App Server/Core 持久化规范化 URL 而不是本地路径。

`Ctrl-V` 是独立的 clipboard-image intent，不依赖 terminal `Event::Paste` 是否能携带位图。
adapter 优先读取 clipboard file list 中可解码的图片，否则读取 RGBA image data，并统一编码为
PNG bytes；`App` 再把 bytes 交给 `Attachments`，因此系统剪贴板和本地路径共享大小校验、
占位符绑定、删除和提交语义。active Turn 期间该快捷键被忽略。

该实现会让 data URL 进入 command receipt 与 durable Thread history，snapshot/store 体积随图片
增长；当前 16 MiB 上限是保护边界，不是长期附件存储方案。后续应由 resource/blob contract
替代大对象内联。

`ChatComposer` 只解析光标下 whitespace-delimited `@token`；`App` 把当前 query 同步给
`FileSearchManager`。manager 为 active token 保持一个 `PathSearchHandle`，后台 walker 使用
Git 作用域内的 ignore 语义、不跟随 symlink，并跳过 `.git`、`.zeta`、`node_modules` 与
`target`。完整 `nucleo` engine 在独立 matcher worker 中增量 reparse query；event loop 每轮通过
`App::poll_background_events` 非阻塞收取 snapshot。manager 同时校验 query revision 与文本，
popup 再校验 query，因此包括 A → B → A 在内的旧结果都不会覆盖新输入。结果按 `nucleo` 分数
降序、路径升序稳定打破平局，最多保留 50 项，字符索引交给 renderer 高亮。

补全只替换当前 `@token`，不会把 email 中的 `@` 当作 mention；选择结果作为 `TextArea` 原子元素
插入，但提交时仍属于普通 Text item。关闭 token 会 drop handle；裸 `@` 也会启动空 pattern
搜索并随着 walker 发现文件逐步更新候选。

`SlashInput::at_cursor` 只在光标位于第一行 `/name` token 内时提供 popup query；补全返回
`SlashCompletion { range, replacement }`，因此 `/mod provider/model` 可变成
`/model provider/model` 而不会
清空后缀、图片或 paste bindings。完成且后接 whitespace 的命令名会被标记为 `TextArea`
原子元素；移除 separator 后会解除标记，从而允许重新编辑。

提交路径先生成完整 `ComposerSubmission`，再由 `SlashInput::for_submission` 使用同一个
`SlashCommandRegistry` 识别命令。支持 inline arguments 的命令会生成
`SlashCommandInvocation`：display arguments 已 trim，structured arguments 保持原有
`ComposerInput::Text` / `ComposerInput::Image` 顺序。未知命令以及不支持参数却带参数的命令仍是
普通 prompt。Registry 可以合并已校验的 dynamic metadata，并拒绝非法名称、空描述和 built-in
冲突；App Server 在 initialize snapshot 中提供 host-composed dynamic command source。

Built-in command 进入 `ActiveConversation::execute`：Session/Thread lifecycle 使用 typed
Session/Thread API，查询命令读取 authoritative config，`/model` 通过 expected revision mutation
更新 preferred model。`/help` 和 `/skills` 复用 generic interaction selection surface；关闭
它们会恢复一直保留的 composer。`/skills` 映射 App Server 的 immutable catalog projection；
`Space` 产生 source-qualified `SkillId` enablement intent，成功写入 config 后重新读取页面。
catalog/file watcher 变化通过 `skills/changed` 触发同一刷新路径。TUI 不读取 Skill filesystem，
也没有正文 activation/context injection action。没有对应 typed contract 的产品命令不进入
registry，不显示占位提示，也不转成普通 prompt 冒充成功。

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
├─ Enter(/quit or /exit) → Quit
├─ Enter(其他 built-in command) → structured invocation → typed command dispatcher
├─ Enter(server dynamic command) → preserve /name + ordered arguments → Submit
├─ /query → cursor-aware popup；↑/↓ select；Tab range completion；Esc dismiss
├─ @query → workspace file popup；↑/↓ select；Tab/Enter complete；Esc dismiss
├─ popup 可见行左键单击 → 补全 mention 或执行 slash command
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
→ EnableMouseCapture
→ Terminal::new
→ clear
```

每个 partial initialization failure 都尽量回滚已获取的 mode。成功后 `Drop` 无条件尝试：

```text
disable_raw_mode
→ DisableMouseCapture
→ DisableBracketedPaste
→ LeaveAlternateScreen
→ show_cursor
```

Cleanup error 被忽略是 Drop 路径的刻意选择，避免 panic during unwind。新增 terminal capability
时必须同时更新 partial-failure rollback 与 Drop reverse cleanup。

## Rendering

当前 layout 在 composer 模式是固定五段：

1. 两行轻量 header，包括产品名和 canonical status 的 presentation label；
2. expandable、无外框的 transcript，空会话显示 centered welcome；
3. 一行右对齐 status line，显示现有接口提供的 model/workspace context；
4. 三行 composer：上下浅灰水平线，中间一行以浅灰 `❯` 开始；
5. 一行 recovery/help footer。

所有 interaction surface 都以 terminal 底部为锚点：composer/footer 固定在底部，slash/mention
popup 从 composer 上沿向上展开；temporary interaction view active 时替换 composer/footer 区域，
底边保持不动并按 view 的 desired height 只向上扩张。header 保持不变，transcript 至少保留四行。
temporary view active 时 status line 不占行。普通 composer 模式下，status line 只消费
`StatusLineModel`，不在 draw 中调用 config、Git 或 Thread 接口。
Selection surface 当前包含顶部分隔线、标题、可换行 Tabs、搜索框、可滚动窗口和 view-local
footer；关闭后恢复一直保留的 composer state。

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

Tests 当前覆盖后台路径 handle 的增量 query、Git ignore、稳定排序、高亮索引与旧结果过滤，
以及局部键到全局键的 routing、trimmed/blank submit、slash registry validation、
cursor filtering、range completion、bare/inline submission、dynamic metadata、原子 command token、
structured text/image/paste arguments、popup render/mouse hit testing 与 local quit dispatch、Unicode
cursor/editing、paste at cursor、large-paste placeholder expansion/binding/deletion、quit/interrupt
keyboard semantics、duplicate interrupt suppression、图片路径识别/占位符删除重编号/结构化提交、input lock、
response/error/interrupted transitions、interaction view 的 composer 保留、tabs wrap/左右循环切换、
搜索过滤/选择修复/Esc-Ctrl-C dismissal、selection render，以及 snapshot
terminal/wait/resume mapping，以及 transcript chrome、error 去重、role
label/Unicode/zero-width wrapping，以及 status-line 长短值降级、Unicode-safe truncation 和
composer 上方的右对齐渲染。

Render tests 使用 Ratatui `TestBackend` 固定 empty/error surface 和 row estimation，但还没有完整
snapshot/golden terminal test；`run` 也没有完整的 fake transport event-loop integration test。
下一阶段优先级应是 async request/notification pump、subscription projection + gap resync、
interaction resolve UI 和 richer transcript。演进时继续让 TUI 可丢弃、可重建，并始终通过
typed App Server client。
