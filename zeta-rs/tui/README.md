# `zeta-tui`

> 文档所有权：本 README 是当前 crate 实现、真实调用路径、限制与修改影响的 canonical 文档。
> TUI 的跨 crate ownership、长期稳态架构基准和阶段退出条件见
> [`docs/tui.md`](../../docs/tui.md)；App Server client contract 见
> [`docs/app-server-client.md`](../../docs/app-server-client.md)。本文不把 Proposed 架构写成
> 当前能力。

`zeta-tui` 当前是 `AppServerSession` 上的 presentation shell：它从 owned session 取得
cloneable typed request handle 与独立 `AppServerEvents`，创建一个 Session/Thread，接受文本、
本地图片路径与系统剪贴板图片输入，订阅 active Thread、启动或中断 Turn，并用 canonical
Thread snapshot 驱动 Ratatui 呈现。

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
- Session/Thread 命令调用 typed create/list/read/fork API 并切换当前 Thread context；配置命令调用
  `config/read`，`/model` 使用 expected revision 更新 preferred model；
- 启动时读取 client 保存的 `initialize.slashCommands` snapshot，通过
  [`zeta-slash-commands`](../slash-commands/README.md) 与 built-ins 做防冲突合并；
  server-advertised command 保留 `/name`、inline text/image/large-paste 参数并作为普通 Turn
  input 提交；
- Enter 按 composer 顺序提交由 text/image items 组成的 Turn；
- 启动及 `/new`、`/fork`、`/resume` 后维护 active Thread subscription；typed update 按
  Session/Thread scope 和 durable sequence 过滤，并通过 `thread/read` 触发权威 snapshot
  resync，不在 TUI 内复制 Thread reducer；
- `AppServerEvents` 与 terminal input 由独立 event source 主动唤醒单写者 loop；active Turn
  不再使用 25 ms `thread/read` polling fallback；
- 以无外框 transcript 显示 latest completed Agent message、友好 failure、interruption 与
  waiting/cancelling state；
- 顶部显示低干扰的运行状态；composer 上方右对齐显示 preferred model 与 workspace，并按宽度
  依次使用短值或省略号；composer 只使用上下两条浅灰分隔线，footer 只包含下一步操作；
- Ctrl-C、Ctrl-D（空输入）或 Esc：idle 时退出，active 时请求 interrupt；
- Unix `SIGINT`/`SIGTERM` 进入同一个 event loop 退出路径，确保 watcher 重启和 host termination
  仍执行 session shutdown 与 terminal RAII cleanup；
- raw mode、alternate screen、bracketed paste、mouse capture 与 cursor cleanup；
- 启动时通过 `zeta-theme` 读取共享用户主题；只投影 accent/chrome/error/success/warning/muted/highlight
  子集，并按 TrueColor、ANSI-256、ANSI-16 或 Monochrome 能力确定性降级；
- basic Unicode-aware wrapped-row estimation 和自动滚动到底部。

当前没有 Session browser、Thread navigation、Markdown、stream delta render、Tool transcript、
approval/user-input response UI、resize-specific state 或 remote connection selector。Mouse
support 当前覆盖 slash 与 file mention popup 左键命中，不包含 hover、滚轮或其他 surface；
缺少 typed backend contract 的 login、plugins、hooks、compact、service tier 等
命令不会进入 registry。Vim mode/motion/operator 目前只有明确的组件所有权，尚未实现。file
mention 只插入 workspace-relative 文本路径，不是 `app://`/`plugin://`
结构化 Mention，也不会读取文件内容。系统剪贴板图片依赖本机 clipboard backend；远程 SSH/
tmux 会话尚无 terminal-mediated image clipboard fallback。
status line 当前没有 Git、usage 或用户自定义 item/order；Git 后续应通过 `zeta-git` 的公开
异步接口进入更新路径，usage 必须等待 App Server typed snapshot 提供，不能从 transcript
推导。完整边界见 [`docs/tui.md` 的 status line 规划](../../docs/tui.md#111-status_line接口结果的展示模型)。
App Server notification 已独立唤醒 loop，但 typed request method 当前仍在单写者线程同步等待
配对 response；异常缓慢的 control-plane request 仍可能短暂阻塞输入，后续需要 typed
completion event，不能用恢复 notification polling 规避。
系统文档中的这些内容是演进方向，不是已实现功能。

从 repository root 启动当前 embedded TUI：

```bash
just tui
```

等价的 Cargo 命令是：

```bash
cargo run --manifest-path zeta-rs/Cargo.toml -p zeta-cli
```

## 公共契约

| Symbol | 职责 |
| --- | --- |
| `TuiOptions::new` | 提供 Session/Thread title，并默认以当前目录作为 file mention root |
| `TuiOptions::with_workspace_root` | 显式覆盖有界 file mention root |
| `run` | 接管 ready `AppServerSession`，校验 initialize snapshot、驱动 terminal/client events，并在退出时显式 shutdown |
| `TuiExit::UserRequested` | 用户通过按键或 command 请求正常退出 |
| `TuiExit::TerminationRequested` | Unix host termination signal 请求正常退出 |
| `TuiError::Client` | typed App Server client failure |
| `TuiError::SessionEvents` | session event stream 被提前取走 |
| `TuiError::Shutdown` | App Server background driver 关闭失败 |
| `TuiError::Terminal` | terminal setup/event/draw failure |

`run` 接受一个已经初始化的 `AppServerSession`。Transport/embedded/local/remote 选择与
initialize/schema handshake 属于 CLI 和 app-server-client，不在 TUI 内重复实现；TUI 读取
request handle 保存的 immutable initialize result，在创建 Session 前拒绝非法/冲突的 server
slash snapshot，并且只取一次 connection event stream。

## 文件与职责

```text
src/
├── lib.rs                         # narrow public startup API; delegates to app::run
├── app/
│   ├── event_loop.rs              # terminal/client/background coordination
│   ├── state.rs                   # single-writer presentation state
│   ├── event.rs / command.rs      # completed facts / typed side-effect intents
│   ├── dispatch.rs                # built-in product command coordination
│   ├── bootstrap.rs / help.rs     # startup registry validation / help model
│   └── frame/                     # top-level frame and footer assembly
├── client/
│   ├── command_id.rs              # stable logical command identity allocation
│   ├── event_pump.rs              # terminal + AppServerEvents wakeup/multiplexing
│   └── notification.rs            # typed ServerNotification → ClientEvent mapping
├── components/
│   ├── composer/                  # editor, attachments, paste, slash/mention state and views
│   ├── interaction/               # composer-preserving temporary view stack
│   ├── selection/                 # reusable selection state and view
│   └── transcript/                # transcript projection rendering and row estimation
├── features/
│   ├── config/                    # config/MCP/model typed requests and presentation results
│   ├── sessions/                  # active Session/Thread selection and lifecycle requests
│   ├── thread/                    # canonical snapshot, requests, subscription and projection
│   ├── skills/                    # skill request and selection presentation mapping
│   ├── status_line/               # model/workspace model and pure view
│   └── workspace_files/           # bounded async file-search runtime
├── host/
│   └── clipboard.rs               # native file/RGBA clipboard adapter
├── terminal/
│   └── session.rs                 # transactional terminal acquisition and RAII restore
└── ui/
    ├── layout.rs                  # shared pure geometry
    ├── theme.rs                   # shared token subset and terminal capability projection
    └── theme_tests.rs             # TrueColor/ANSI/monochrome projection contract
```

实现 module 都是 private；crate 只导出启动 contract。

## 内部接口地图

| Symbol | 可见性 | 当前职责 | 方向约束 |
| --- | --- | --- | --- |
| `App` | crate-private | presentation `Status`、feature/局部交互协调和单写者 state transition | 不保存 worker/channel、不复制 feature state 或编辑器细节 |
| `AppCommand` | crate-private | execute/quit/interrupt/clipboard/skill/Turn 的 typed side-effect intent | 只描述待执行行为，不携带任意闭包或执行 I/O |
| `AppEvent` | crate-private | config、clipboard、file search、Thread/Turn 与 product command 的已完成事实 | 只能由 `App::update` 改变 presentation state |
| `TurnActivity` | crate-private | canonical Turn status 到 Working/waiting/Cancelling presentation state 的窄映射 | 不复制完整 Turn reducer |
| `ThreadFeatureState` | crate-private | active canonical `Thread` snapshot、transcript projection 与本地 optimistic/diagnostic overlay | 下一份 snapshot 替换 projection；不执行 RPC、不复制 product reducer |
| `ThreadPresentationEvent` | crate-private | snapshot/user/notice/failure/interrupted/clear 的 feature-local事实 | 只改变 active Thread presentation owner |
| `components::transcript::{Message,MessageRole,draw}` | crate-private | 定义 transcript-facing 展示值，并渲染 role chrome、empty state、wrapping 与 bottom scroll | 不依赖 feature/`App`、不保存 Thread/sequence、不处理输入 |
| `ui::layout` | private module | 跨 surface 复用的纯 geometry | 不读取 App/feature、不调用 terminal 或 RPC |
| `ui::theme` | private module | 将 `zeta-theme::ThemeSnapshot` 的明确子集投影到终端能力 | 不复制完整 Desktop token catalog、不拥有用户文件加载、不定义产品状态 |
| `Status` | crate-private | Ready/Working/waiting/Cancelling/Error display state | 只能由 canonical snapshot/result驱动 |
| `StatusLineModel` | crate-private | 直接把 config/workspace 接口结果变成长短展示值并执行宽度降级 | 不查询接口、不保存领域 authority、不渲染 |
| `App::update` | crate-private | 将一个 `AppEvent` 应用到唯一 presentation state owner | 不执行 I/O、不访问 runtime resource |
| `App::handle_key` | crate-private | 先委托局部输入，再处理未消费的全局键 | 不直接调用 client |
| `App::activate_slash_command` | crate-private | 将鼠标命中的 command index 委托给 composer 并复用 command dispatch | 不计算 terminal geometry |
| `App::quit_or_interrupt` | private | active state interrupt；idle/error quit | Cancelling 不重复发送 interrupt |
| `client::EventPump` | crate-private | 独立等待 terminal input、Unix termination signal 与 `AppServerEvents`，把三者汇入单写者 loop | 不应用 UI state、不执行领域 request |
| `client::map_event` / `ClientEvent` | crate-private | 把共享 connection event 映射为 skills changed、Thread update 与 connection failure | 不保存 transport、不应用 projection |
| `ThreadSubscription` | crate-private | 维护 active Thread scope 与最后确认的 snapshot sequence；新 update 只请求 snapshot resync | 不应用 `ThreadEvent` reducer、不保存 transient projection |
| `InteractionPane` | crate-private | 保留 composer、拥有 temporary view stack，并把 key/paste 路由到 active view 或 composer | 不保存 Plugin/Session 等产品 feature 状态 |
| `components::selection::SelectionViewState` | crate-private | tabs、搜索 query、过滤索引、选择与循环导航 | 不执行 action、不依赖产品 ID 或 App Server |
| `components::selection::draw` | crate-private | generic title/tabs/search/items/footer Ratatui surface | 只读 selection state、不解释产品 action |
| `ChatComposer` | private | blank/trim/submit、paste routing、slash completion application、参数结构化与 local dispatch | 不自行实现 slash grammar，不拥有 cursor、Vim state 或 RPC |
| `Attachments` | private | 图片 bytes/path、data URL 与原子占位符绑定、删除后重新编号 | 不直接读取系统 clipboard、不发 RPC、不渲染 |
| `host::clipboard::read_image` | crate-private | 从本机 clipboard 文件列表/RGBA image 读取并统一编码 PNG | 不改变 composer、不发 RPC、不持久化临时文件 |
| `FileSearchManager` | crate-private | event loop 持有的 workspace search runtime；非阻塞 drain snapshot 并丢弃旧 query 结果 | 不进入 `App` state、不解析输入、不保存 popup state |
| `Mentions` / `MentionPopup` | private | `@token` query/range、异步结果应用、选择/关闭和原子路径补全 | 不扫描 workspace、不拥有 worker、不构造结构化 app/plugin Mention |
| `PendingPastes` | private | 超过 1000 字符的 text-paste payload、唯一占位符与提交时展开 | 不识别或保存图片，不解释 slash、不渲染、不直接提交 |
| `zeta_slash_commands::SlashCommandsState` | shared public type | 拥有 cursor query、matches、selection、dismissal 与 completion | TUI 不保存第二份 Slash query/selection authority；可见范围与滚动仍由 Ratatui renderer 负责 |
| `zeta_slash_commands::{SlashCommandInput,SlashCommandCatalog}` | shared public types | 统一输入 grammar，并合并 built-in 与 server metadata | TUI 不重新校验名称、不执行 App Server operation |
| `SlashCommandInvocation` | crate-private | command identity、trimmed display arguments 与有序 text/image argument items | 不执行 RPC |
| `features::sessions::ActiveConversation` | crate-private | 当前 Session/Thread identity、sequence 与 typed create/fork/resume lifecycle | 不解析 composer text、不更新 `App`、不拥有 App Server |
| `TextArea` | private | UTF-8 buffer、byte-safe cursor、原子元素 insert/delete/movement；Vim 的扩展 owner | 不保存 paste payload，不解释 Enter submission 或 slash command |
| `features::thread::submit_prompt` | private | 从显式 `ThreadRequestScope` build typed `TurnStartParams` 并返回 typed result | 不引用或更新 `App`、不手写 method string/JSON |
| `app::event_loop::refresh_turn` | private | `thread/read`、校验 scope、更新 local sequence 并协调 active Turn mapping | 不 drain notification；snapshot 是 authoritative UI source |
| `features::thread::interrupt_turn` | private | 从显式 scope 执行 typed Turn interrupt 并返回结果 | 不引用或更新 `App` |
| `app::apply_active_turn_snapshot` | crate-private | canonical Turn presentation outcome → `AppEvent` | 不从 log/text 猜 terminal state |
| `present_turn_error` | private | stable Turn error code → user-facing recovery message | 不显示 Rust Debug/provider secret |
| `client::new_command_id` | private | process ID + wall-clock nanos 分配 `CommandId` | 一次逻辑 command 一个新 ID |
| `app::frame::draw` | crate-private | frame 分区并协调 feature/component renderer | 不改变 App state |
| `app::frame::{slash_command_index_at,mention_index_at}` | crate-private | 复用 popup geometry 映射可见行点击 | 不执行命令、不改变选择状态 |
| `components::transcript::row::estimated_wrapped_rows` | private | Unicode display-width based scroll estimate | width 0 不 panic |
| `TerminalSession::open` | crate-private | 进入 raw/alternate/paste/mouse mode 并创建 backend | partial failure 必须 rollback |
| `TerminalModeGuard::acquire` | private | 按顺序获取 terminal mode，并在任一步失败时逆序 rollback | 不创建 Ratatui backend、不处理产品状态 |
| `TerminalModeGuard::restore` | private | 幂等地逆序释放已经获取的 mode | cleanup error 不覆盖原始错误 |
| `Drop for TerminalSession` | private impl | 委托 guard 恢复 terminal modes，再显示 normal-screen cursor | 所有成功构造后的退出路径都依赖 RAII |

本地 `app::Status` 是 presentation state，不是 `zeta_protocol::TurnStatus` 的复制品。它可以包含
`Error(String)` 供显示，但不能被其他层当作 domain fact。

## 启动与事件循环

```text
run(session, options)
├─ session.client → cloneable typed request handle
├─ session.take_events → single-consumer AppServerEvents
├─ client.create_session / create_session_thread
├─ client.subscribe_thread → ThreadSubscription + initial canonical snapshot
├─ TerminalSession::open
├─ EventPump::start → terminal source + App Server source
├─ FileSearchManager::new
├─ App::for_workspace → StatusLineModel::for_workspace
├─ client.read_config → AppEvent::ConfigSnapshotReceived → App::update
└─ loop
   ├─ EventPump::recv
   │  ├─ terminal event → input routing
   │  └─ App Server event → typed notification mapping
   ├─ App::mention_query → FileSearchManager::{update_query,stop}
   ├─ FileSearchManager::poll → AppEvent::FileSearchSnapshotReceived → App::update
   ├─ skills changed → skills refresh
   ├─ newer active Thread update → one thread/read snapshot resync
   ├─ TerminalSession::draw → app::frame::draw
   └─ terminal event
      ├─ key → App::handle_key
      │  ├─ local input → InteractionPane
      │  │  ├─ active selection view → local view state
      │  │  └─ no active view → ChatComposer → TextArea
      │  ├─ ReadClipboardImage → clipboard::read_image → AppEvent → App::update
      │  ├─ Quit → return
      │  ├─ SubmitTurn → submit_prompt
      │  └─ Interrupt → refresh + interrupt_turn
      ├─ left mouse down → app::frame::{mention_index_at,slash_command_index_at}
      │  ├─ mention hit → App::activate_mention → atomic path completion
      │  └─ slash hit → App::activate_slash_command → existing command dispatch
      └─ Paste → App::handle_paste → InteractionPane
         ├─ active selection view → search query
         └─ no active view → ChatComposer
         ├─ image path → Attachments + TextArea atomic placeholder
         └─ text → PendingPastes + TextArea
```

Session create 和 Thread create 使用独立 `CommandId`。Turn start/interrupt 使用当前
`thread_sequence` 作为 expected sequence；client error 会进入 visible error message/status，不退出
terminal session。

创建后通过 `thread/read`/`thread/subscribe` 返回的 canonical snapshot 设置 initial sequence，
不存在硬编码的初始 sequence。切换 active Thread 时先更换 subscription，再以返回 snapshot
替换 presentation projection。

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

`ChatComposer` 只解析光标下 whitespace-delimited `@token`；event loop 从 `App` 读取当前
query 并同步给 `FileSearchManager`。manager 为 active token 保持一个 `PathSearchHandle`，后台 walker 使用
Git 作用域内的 ignore 语义、不跟随 symlink，并跳过 `.git`、`.zeta`、`node_modules` 与
`target`。完整 `nucleo` engine 在独立 matcher worker 中增量 reparse query；event loop 每轮通过
`FileSearchManager::poll` 非阻塞收取 snapshot，再以 `AppEvent` 投递给 `App::update`。manager
同时校验 query revision 与文本，
popup 再校验 query，因此包括 A → B → A 在内的旧结果都不会覆盖新输入。结果按 `nucleo` 分数
降序、路径升序稳定打破平局，最多保留 50 项，字符索引交给 renderer 高亮。

补全只替换当前 `@token`，不会把 email 中的 `@` 当作 mention；选择结果作为 `TextArea` 原子元素
插入，但提交时仍属于普通 Text item。关闭 token 会 drop handle；裸 `@` 也会启动空 pattern
搜索并随着 walker 发现文件逐步更新候选。

`zeta_slash_commands::SlashCommandInput::at_cursor` 只在光标位于第一行 `/name` token 内时提供
popup query；补全返回 `SlashCommandCompletion { range, replacement }`，因此
`/mod provider/model` 可变成
`/model provider/model` 而不会
清空后缀、图片或 paste bindings。完成且后接 whitespace 的命令名会被标记为 `TextArea`
原子元素；移除 separator 后会解除标记，从而允许重新编辑。

提交路径先生成完整 `ComposerSubmission`，再由共享 `SlashCommandInput::for_submission` 使用
同一个 `SlashCommandCatalog` 识别命令。支持 inline arguments 的命令会生成
`SlashCommandInvocation`：display arguments 已 trim，structured arguments 保持原有
`ComposerInput::Text` / `ComposerInput::Image` 顺序。未知命令以及不支持参数却带参数的命令仍是
普通 prompt。Catalog 可以合并已校验的 dynamic metadata，并拒绝非法名称、空描述和 built-in
冲突；App Server 在 initialize snapshot 中提供 host-composed dynamic command source。

Built-in command 进入 `ActiveConversation::execute`：Session/Thread lifecycle 使用 typed
Session/Thread API，查询命令读取 authoritative config，`/model` 通过 expected revision mutation
更新 preferred model。`/help` 和 `/skills` 复用 generic interaction selection surface；关闭
它们会恢复一直保留的 composer。`/skills` 映射 App Server 的 immutable catalog snapshot；
`Space` 产生 source-qualified `SkillId` enablement intent，成功写入 config 后重新读取页面。
catalog/file watcher 变化通过 `skills/changed` 触发同一刷新路径。TUI 不读取 Skill filesystem，
也没有正文 activation/context injection action。没有对应 typed contract 的产品命令不进入
registry，不显示占位提示，也不转成普通 prompt 冒充成功。

## 快照→ UI 映射

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

`refresh_turn` 每次成功 read 都用 snapshot sequence 覆盖 local expected sequence。
`client::map_event` 保留 typed `ThreadUpdateEnvelope`；`ThreadSubscription` 验证 active
Session/Thread scope，并在 durable sequence 新于最后确认 snapshot 时触发一次 `thread/read`。
重复、旧 scope update 不覆盖当前 state。当前 transient delta 不进入本地 reducer；snapshot
read 只由新 durable update 或显式 interrupt/resync 触发。

## 键盘状态机

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

`AppEvent::InterruptFailed` 把状态恢复到 Working，使用户可以再次请求 interrupt；ordinary
client failure 通过 `AppEvent::FailureReported` 进入 Error 并允许输入新 prompt。

## 终端生命周期

`TerminalSession::open` 按以下顺序获取资源：

```text
enable_raw_mode
→ EnterAlternateScreen
→ EnableBracketedPaste
→ EnableMouseCapture
→ Terminal::new
→ clear
```

`TerminalModeGuard::acquire` 在任一 mode 获取失败时只回滚已经成功获取的 mode。
`Terminal::new` 或 `clear` 失败时，已经构造的 guard 也执行同一路径。成功后
`TerminalSession::drop` 无条件尝试：

```text
DisableMouseCapture
→ DisableBracketedPaste
→ LeaveAlternateScreen
→ disable_raw_mode
→ show_cursor
```

cleanup 是幂等的；显式 restore 后 guard Drop 不会重复发出控制操作。cleanup error 被忽略是
Drop 路径的刻意选择，避免 panic during unwind。新增 terminal capability 时必须同时更新
acquisition flag、reverse cleanup 和 `session_tests.rs` 的 partial-failure case。

## 渲染

当前 layout 在 composer 模式是固定四段：

1. expandable、无外框的 transcript；空会话显示由 `components::welcome` 拥有的 responsive
   Welcome Banner，宽终端使用双栏，窄终端降级为单栏；
2. 一行右对齐 status line，显示现有接口提供的 model/workspace context；
3. 三行 composer：上下浅灰水平线，中间一行以浅灰 `❯` 开始；
4. 一行 recovery/help footer。

所有 interaction surface 都以 terminal 底部为锚点：composer/footer 固定在底部，slash/mention
popup 从 composer 上沿向上展开；temporary interaction view active 时替换 composer/footer 区域，
底边保持不动并按 view 的 desired height 只向上扩张，transcript 至少保留四行。
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
- `app::frame::draw` 或 component view 修改 state、subscription 或发 RPC：view 与 coordination 耦合；
- retry 逻辑生成新 CommandId 却重用同一 intent：idempotency semantic 被破坏；
- 把 `TerminalSession`、RPC connection 和 product `Session` 命名/生命周期混为一谈；
- docs 中把 planned Markdown/approval/streaming 写成当前能力。

## 同步修改关系

| 修改 | 必须同步检查 |
| --- | --- |
| 新 `Status` | `handle_key`/`quit_or_interrupt`、run polling guard、snapshot mapping、render status、tests |
| 新 `AppCommand` | keyboard mapping、run command match、I/O failure result event |
| 新 `AppEvent` | `App::update`、producer scope/ordering、state tests |
| 新 canonical Turn state/item | `apply_active_turn_snapshot`、render behavior、protocol compatibility |
| 新 terminal mode | `open` rollback、`Drop` cleanup、manual terminal recovery |
| Composer behavior | `accepts_input`、paste/key handling、cursor width、app tests |
| Incremental notifications | `features/thread` sequence/cursor state、gap/resync、client event pump、snapshot fallback |

## 测试、限制与演进

```text
cargo test -p zeta-tui
bazel test //zeta-rs/tui:tui-unit-tests
```

测试当前覆盖后台路径句柄的增量查询、Git 忽略规则、稳定排序、高亮索引与旧结果过滤，
以及局部键到全局键的 routing、trimmed/blank submit、slash registry validation、
cursor filtering、range completion、bare/inline submission、dynamic metadata、原子 command token、
structured text/image/paste arguments、popup render/mouse hit testing 与 local quit dispatch、Unicode
Thread notification decode、active scope/sequence resync 判定、
并覆盖游标/编辑、在游标处粘贴、大段粘贴占位符展开/绑定/删除、退出/中断、
keyboard semantics、duplicate interrupt suppression、图片路径识别/占位符删除重编号/结构化提交、input lock、
canonical Thread snapshot 替换 optimistic transcript、snapshot identity/sequence 保留、
非 message item 过滤、response lifecycle/error/interrupted transitions、interaction view 的
composer 保留、tabs wrap/左右循环切换、
搜索过滤/选择修复/Esc-Ctrl-C dismissal、selection render，以及 snapshot
terminal/wait/resume mapping，以及 transcript chrome、error 去重、role
label/Unicode/zero-width wrapping，以及 status-line 长短值降级、Unicode-safe truncation 和
composer 上方的右对齐渲染，以及 terminal mode acquisition failure、逆序 rollback 与幂等
restore。

Render tests 使用 Ratatui `TestBackend` 固定 empty/error surface，transcript component tests
固定 row estimation；但还没有完整 snapshot/golden terminal test，`run` 也没有完整的 fake
transport event-loop integration test。
按系统文档的阶段零固定行为与性能基线后，下一阶段优先级是 request completion 的非阻塞
command dispatch、transient item merge、interaction resolve UI 和 richer
transcript。具体 owner、数据面可靠性和退出条件由
[`docs/tui.md`](../../docs/tui.md#17-演进顺序) 统一定义；本 README 在每个切片落地后更新当前
symbol、调用路径和限制。
