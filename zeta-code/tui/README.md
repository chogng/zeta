# `zeta-tui`

> 文档所有权：本 README 是当前 crate 实现、真实调用路径、限制与修改影响的 canonical 文档。
> TUI 的跨 crate ownership、长期稳态架构基准和阶段退出条件见
> [`docs/tui.md`](../../docs/tui.md)；App Server client contract 见
> [`docs/app-server-client.md`](../../docs/app-server-client.md)。本文不把 Proposed 架构写成
> 当前能力。
> 三条产品线与宿主边界见 [`docs/product-lines.md`](../../docs/product-lines.md)。
> 三端快捷键语义与端侧输入边界见 [`docs/keybindings.md`](../../docs/keybindings.md)。

`zeta-tui` 是 `zeta code` 产品线的 TUI 实现。它当前是 `AppServerSession` 上的 presentation
shell：从 owned session 取得 cloneable typed request handle 与独立 `AppServerEvents`，创建或切换
Session/Thread，接受文本、本地图片路径与系统剪贴板图片输入，订阅 active Thread、启动或中断
Turn，并用 canonical Thread snapshot 加有界 transient projection 驱动 Ratatui 呈现。

它不拥有 Agent runtime、Session/Thread reducer、App Server connection composition、model、
Tool、approval policy 或 persistence。

## 当前能力

- 启动时创建一个 product Session 和 root Thread；
- 最多显示六行正文的多行 composer，支持 typing、Unicode-safe cursor/editing、Shift/Alt-Enter
  换行、按行 Home/End/Up/Down、Unicode display-cell 软换行与 bracketed paste；超过 1000
  个 Unicode scalar value 的 paste 会显示为原子占位符，并只在提交时展开；
- 粘贴 PNG/JPEG/GIF/WEBP 本地文件路径时立即读取最多 16 MiB 的图片，显示可原子编辑的
  `[Image #N]` 占位符，并以结构化图片项提交；
- `Ctrl-V` 从系统剪贴板读取图片；文件列表与原始 RGBA 位图都会统一进入同一附件占位符和
  结构化提交路径；
- `@` 在当前 workspace 打开 mention popup；`zeta-file-search::PathSearchHandle` 使用与 Codex file search 相同 revision 的完整 `nucleo` engine，在后台增量扫描和匹配路径，支持 fuzzy 排序、命中字符高亮、循环选择、鼠标 hover 跟随选中、Tab/Enter completion、Esc dismiss 和左键选择；选中路径作为原子文本插入；
- `/` 打开 command popup，支持 cursor-aware prefix filtering、循环选择、保留已有参数尾部的 Tab completion、Esc dismiss、鼠标 hover 跟随选中与左键单击可见命令；
- enabled、compatible、名称无歧义且不与已有命令冲突的 Skill 直接显示为 `/name`；提交时保留
  `/name …` 用户文本并附加 exact pinned `SkillRef`，完整 `SKILL.md` 只在 App Server 接受 Turn
  后按需加载；`skills/changed` 会刷新这部分动态命令；
- `/resume`、`/rewind`、`/clear`、`/add-dir`、`/fork`、`/model`、`/theme` 与 `/new` 可解析 inline arguments，并在执行前展开 large-paste placeholder；product command 明确拒绝 image arguments；
- command popup 只注册已有真实执行流的 built-ins：`/status`、`/statusline`、`/skills`、`/mcp`、`/connectors`、`/resume`、`/archive`、`/rewind`、`/clear`、`/config`、`/add-dir`、`/fork`、`/help`、`/shortcuts`、`/copy`、`/export`、`/model`、`/theme`、`/new` 与 `/quit`；
- `/status` 展示当前 Session 实际模型、完整上下文窗口、扣除 output reservation、safety margin 与 auto-compaction 边界后的可用窗口、最近一次同模型调用后的剩余窗口，以及 Session ID、Thread ID 和 Thread sequence；provider usage 不完整时剩余值标记为估算，尚无可信值时显示 unknown；
- `/help` 使用保留 composer 的 interaction view stack 打开可搜索的命令列表；Space 进入搜索模式，上下键循环选择，Esc/Ctrl-C 返回 composer；快捷键只由 `/shortcuts` 展示；
- `/skills` 通过 typed `skills/list` 打开同一 interaction surface，提供
  All/Enabled/Disabled/Manage/Errors tabs、数量、搜索和 source-qualified metadata；只有 Manage
  tab 的动作通过 revision-checked `skill/enablement/set` 修改 enablement；该页面是目录管理入口，
  不直接激活 Skill；
- `/connectors` 通过 typed `connector/list` 打开 Connector Pane；已连接项可以执行
  generation-checked disconnect，`connector/changed` 只在该 Pane 打开时触发 catalog refresh；
  API token/OAuth 连接仍由 Desktop Settings 完成；
- `/add-dir <path>` 通过 typed App Server RPC 把 canonical 目录加入当前 Session 的访问作用域，默认打开读取与修改文件；本地 `read_file`、`write_file`、`edit`、`grep` 与 `glob` 随即按各自能力接受该目录中的绝对路径，下一 Turn 的 environment snapshot 只列出仍允许读取的目录。不带参数时打开可搜索列表，Enter 撤销所选目录；快照放在模型请求尾部且不写入对话历史，主 Workspace、cwd、相对路径和项目配置均不改变；
- 同一 profile 中的 `marketplace/changed` 与 `plugin/changed` 会触发 Skill catalog 重读，并在
  Connector Pane 已打开时重读 Connector projection；TUI 当前不提供独立 Marketplace 浏览/安装界面；
- `/rewind` 或主界面 500 ms 内连续按两次 Esc 打开可搜索的历史消息 checkpoint Pane；Enter
  通过 typed `session/request` 的 `RewindThread` operation，创建具有 Rewind lineage 的子 Thread，只导入所选消息之前的
  terminal Turns。原 Thread 保持不变，TUI 切换订阅并以 `/rewind <turn-id>` 记录结果；
- `/resume` 提供 Session picker；`/archive` 归档当前 Session 并建立 replacement Session。所有 mutation 都通过 typed `session/request`，随后在后台切换 subscription；
- `/config` 异步读取服务端配置、供应商目录和当前 Session 的附加目录权限；Directory permissions 标签页按目录提供读取、修改、执行、文件监听与项目配置加载开关，使用 Workspace access revision 防止旧页面覆盖新选择。Config 标签页包含本地 Mouse interactions 开关，Providers 标签页展示后端注册的完整供应商目录，并通过隐藏输入框把 API key 交给 profile SecretStore；`/model` 使用 expected revision 更新 preferred model；
- 启动时读取 client 保存的 `initialize.slashCommands` snapshot，通过
  [`zeta-slash-commands`](../../zeta-rs/slash-commands/README.md) 与 built-ins 做防冲突合并；
  server-advertised command 保留 `/name`、inline text/image/large-paste 参数并作为普通 Turn
  input 提交；slash popup 不清空或铺设独立背景，透明继承当前 TUI 主题 surface，选中项使用候选 highlight 色粗体且不添加行首标记；
- Enter 按 composer 顺序提交由 text/image items 组成的 Turn；active Turn 执行期间仍可编辑并提交
  follow-up，Core 的 per-Thread mailbox 按接受顺序串行执行这些 Turn；
- 启动及 `/new`、`/fork`、`/resume`、`/rewind` 后维护 active Thread subscription；typed update 按
  Session/Thread scope 和 durable sequence 过滤，并通过 `session/thread/read` 触发权威 snapshot
  resync，不在 TUI 内复制 Thread reducer；
- `AppServerEvents` 与 terminal input 由独立、有界 event source 主动唤醒单写者 loop；typed request
  由 `RequestTask` 在后台执行，完成结果回到 event loop，排队的用户 intent 不会静默丢失；active
  Turn 不再使用 25 ms `session/thread/read` polling fallback；
- transcript 投影完整显示 user text/image、所有 agent message、reasoning、plan、ToolCall 与
  ToolResult；`ItemDelta`、`PlanUpdated` 与 `ToolOutputDelta` 按 stream instance/cursor 增量更新，
  gap 会清除不可信 transient row 并读取权威 snapshot；单个 transient row 限 256 KiB、最多保留
  1024 个 transient identity；Tool stdout/stderr 只在 Ratatui render boundary 通过
  [`zeta-ansi-escape`](../ansi-escape/README.md) 将 ANSI SGR 转为 styled spans，并把 tab 投影为四个
  空格，protocol 与 Thread presentation state 继续保留原始输出；
- owner-directed `agent/request` 支持 approval（approve once/decline）和多问题 user input；只有
  App Server 选中的、声明对应 capability 且订阅该 Thread 的 connection 能 resolve。交互不可用
  Esc 关闭，但可 Ctrl-C interrupt；deadline 由 App Server 执行并投影为稳定 Turn failure；
- footer 按“权限模式、模型、Git 分支、Git 变更”的固定顺序显示 `/statusline` 启用的项目，不显示 Turn 运行状态，也不常驻展示快捷键；Shift-Tab 通过 typed Session mutation 在 `ask permissions on`、`auto review on` 与 `bypass permissions on` 之间切换后端保存的下一次模式。运行中 Turn 的冻结模式与下一次模式不同时，footer 同时标出 `current` 和 `next`；TUI 不解释策略结果，也不自行签发执行授权；
- 根级 `keymap.rs` 只保留运行时入口和 `AppKeymap`，`keymap/bindings.rs`、`keymap/chords.rs` 与 `keymap/input.rs` 分别拥有动作绑定、Chord 生命周期和 Crossterm 转换；共享 Resolver 处理 Shift-Tab、根级 Esc 与 Ctrl-C/D/O/V/Z，并生成设置界面只读快照。`features/shortcuts.rs` 读取 `<profile>/zeta-code/keybindings.json`，每秒热重载 User command/blocker、平台覆盖与 `when`，并为 `/shortcuts` 汇总可配置绑定和固定操作键，提供搜索、诊断、单键/两段 Chord 录制、revision 校验和原子保存；坏更新或保存失败保留上一份有效规则。composer 编辑、selection 导航和 transcript 滚动仍由各 component 拥有；
- composer 保存最近 100 条纯文本提交，Up/Down 可召回并恢复原 draft；transcript 支持
  PageUp/PageDown 与 Ctrl-Home/Ctrl-End。初始 Thread snapshot 只读取最近 50 个 Turn，Ctrl-Home
  通过 App Server 的 durable Turn cursor 请求更早的 50 个 Turn，并在 presentation projection 中
  合并页面；TUI 不保存 Thread history；
- `/copy` 或 Ctrl-O 把最后一条 Agent response 写入系统剪贴板；`/export [relative-path]` 以
  Markdown 导出当前已加载的 transcript history window，路径限制在 workspace 内且绝不覆盖已有文件；
- 空会话 Welcome Banner 在 `Ready when you are` 下方显示以 `~` 缩写用户主目录的 workspace 路径；composer 上方不常驻 workspace 信息。composer 只使用上下两条浅灰分隔线，底部 footer 区域由 status line 按配置显示权限模式、preferred model、typed Git branch 与变更数，并按宽度降级；Chord 等必须立即处理的提示临时覆盖 status line；
- Ctrl-C 或 Ctrl-D（空输入）在 idle 时退出，active 时请求 interrupt；单次 Esc 在根界面保持
  inert，连续两次 Esc 打开 Rewind Pane；
- Unix `SIGINT`/`SIGTERM` 进入同一个 event loop 退出路径，确保 watcher 重启和 host termination
  仍执行 session shutdown 与 terminal RAII cleanup；
- Ctrl-Z 在 Unix 上先恢复当前启用的鼠标捕获、bracketed paste、alternate screen 和 raw mode，再发送 `SIGTSTP`；`fg` 恢复后按原顺序重新获取所有 terminal mode 并清屏重绘；
- raw mode、alternate screen、bracketed paste 与 cursor cleanup；鼠标捕获只在 slash/file-mention popup 或包含可执行候选项的 Selection Pane 可见时启用，相关界面关闭后立即释放，使终端恢复拖拽文本选择；
- 启动时通过 `zeta-theme` 读取共享用户主题；chrome 投影 accent/error/success/warning/muted/highlight，
  Theme Pane preview 投影有限的 syntax/diff token，并按 TrueColor、ANSI-256、ANSI-16 或
  Monochrome 能力确定性降级；`features/theme` 拥有 `/theme` 的固定八项 Zeta Code Pane、`Theme` 标题及其上下各一行间距、编号、
  active 标记、候选 frame highlight、仅带上下较高对比度长节虚线的 diff preview、palette 来源说明和选择动作，Pane
  不启用搜索，Enter 原子保存、立即重绘并关闭整个 Theme flow 返回主界面，失败时保留 Pane；成功时以状态圆点、`/theme <id>` 和以 `└─` 归属且与命令文字对齐的 `Theme set to …` transcript 记录执行结果，`/theme <id>` 保留直接切换；
  Auto 在 terminal raw mode 建立后查询一次 OSC 11 实际背景 RGB，据此选择 Light/Dark；查询超时
  后依次回退 `COLORFGBG` 和 Dark。结果在会话内缓存，后续打开 Theme Pane 不重复查询；
- basic Unicode-aware wrapped-row estimation、显式 transcript scroll 和默认 follow-latest。

## 产品支持边界

`zeta code` 是键盘优先、低带宽的终端产品，不以复刻 `app` Native rich UI 为完成条件。
transcript 当前采用 plain-text wrapping；Native Agent Timeline 的 Markdown block、table、selection、
折叠与虚拟化由
[`native-agent-console.md`](../../app/docs/native-agent-console.md) 和
[`zeta-markdown`](../../app/markdown/README.md) 拥有，不构成 TUI backlog。TUI 的 Mouse support 服务 slash/file-mention popup、多标签 Selection Pane 的左键切换和带可执行候选项的 Selection Pane；hover 复用选中态，左键复用 Enter 动作。Config 标签页中的 Mouse interactions item 可关闭鼠标交互，关闭后这些页面不捕获鼠标，任意屏幕文本框选由终端负责。
`TextArea` 保留局部 keymap 扩展边界，但 Vim mode/motion/operator 不是当前 `zeta code` 产品要求。

TUI 当前连接 CLI 提供的 profile/Workspace-scoped local App Server authority，不提供 remote
connection selector 或自动 reconnect；Desktop 与 app 在相同 authority partition 下可以实时读取
同一份 Session catalog 和 Thread event。若未来产品要求远程运行，必须先由
`zeta-app-server-client` 接受 connection/recovery contract，TUI 只消费其 typed state，不能自建
transport retry。workspace mention 当前插入 workspace-relative 原子文本路径；通用
`app://`/`plugin://` Mention 尚无已接受产品契约，也不是 TUI 自行扩展的领域模型。

图片 bytes 的持久化由共享 `zeta-attachments` content-addressed store 拥有；TUI 只在草稿期间保留
本地 data URL，并在 `StartTurn` 前通过 App Server 分块上传或安全导入远程 URL，最终只提交 typed
`ImageAttachmentRef`。`/status` 只消费 typed model capacity 与 Turn `contextUsage`，不从 transcript 推导上下文占用。缺少 typed backend contract 的 login、compact、service tier 等命令不会进入 registry。`/statusline` 使用 `<profile>/zeta-code/statusline.json` 保存权限、模型、Git 分支和 Git 变更四个显示开关；Config 页面展示 Config、Directory permissions、Providers 与 Language servers，其中 Mouse interactions 保存在 `<profile>/zeta-code/terminal.json`，关闭后 `App::mouse_mode` 始终把拖选留给终端。Directory permissions 是当前 Session 的 Workspace access authority，不写入 terminal 设置或 User Config；执行、监听和项目配置加载目前只保存权限，相关 consumer 尚未接入附加目录。Providers 来自后端注册表，API key 只通过 `provider/apiKey/set` 写入 SecretStore，不进入普通配置或展示状态。MCP、Skill、Plugin 和 Hook 不在 Config 中重复展示。

从 repository root 启动当前 TUI：

```bash
just zeta
```

等价的 Cargo 命令是：

```bash
cargo run --manifest-path Cargo.toml -p zeta-cli
```

## 公共契约

| Symbol | 职责 |
| --- | --- |
| `TuiOptions::new` | 提供 Session/Thread title，并默认以当前目录作为 file mention root |
| `TuiOptions::with_workspace_root` | 显式覆盖有界 file mention root |
| `TuiOptions::with_profile_root` | 启用 host-local、产品作用域的 `zeta-code/keybindings.json`、`zeta-code/statusline.json` 与 `zeta-code/terminal.json` 资源 |
| `run` | 接管 ready `AppServerSession`，校验 initialize snapshot、驱动 terminal/client events，并在退出时显式 shutdown |
| `TuiExit::UserRequested` | 用户通过按键或 command 请求正常退出 |
| `TuiExit::TerminationRequested` | Unix host termination signal 请求正常退出 |
| `TuiError::Client` | typed App Server client failure |
| `TuiError::SessionEvents` | session event stream 被提前取走 |
| `TuiError::Shutdown` | App Server background driver 关闭失败 |
| `TuiError::Terminal` | terminal setup/event/draw failure |

`run` 接受一个已经初始化的 `AppServerSession`。Transport/brokered-local/embedded/remote 选择与
initialize/schema handshake 属于 CLI 和 app-server-client，不在 TUI 内重复实现；TUI 读取
request handle 保存的 immutable initialize result，在创建 Session 前拒绝非法/冲突的 server
slash snapshot，并且只取一次 connection event stream。

## 文件与职责

```text
src/
├── lib.rs                         # narrow public startup API; delegates to app::run
├── keymap.rs                      # runtime owner and module entry
├── keymap/
│   ├── bindings.rs               # root action declarations, conditions, resolver snapshots
│   ├── chords.rs                 # chord validation, pending state, timeout and dispatch
│   └── input.rs                  # Crossterm event normalization and config serialization
├── app/
│   ├── event_loop.rs              # terminal/client/background coordination
│   ├── state.rs                   # single-writer presentation state
│   ├── event.rs / command.rs      # completed facts / typed side-effect intents
│   ├── dispatch.rs                # built-in product command coordination
│   ├── bootstrap.rs / help.rs     # startup registry validation / help model
│   ├── frame.rs                   # top-level frame assembly
│   └── frame/                     # footer content selection and frame tests
├── client/
│   ├── command_id.rs              # stable logical command identity allocation
│   ├── event_pump.rs              # terminal + AppServerEvents wakeup/multiplexing
│   └── notification.rs            # typed ServerNotification → ClientEvent mapping
├── components/
│   ├── composer/                  # editor, attachments, paste, slash/mention state and views
│   ├── interaction/               # composer-preserving temporary view stack
│   ├── selection/                 # reusable selection state and view
│   ├── tab_list.rs                # reusable horizontal tab state, mouse/keyboard input, wrapping and view
│   └── transcript/                # transcript projection rendering and row estimation
├── features/
│   ├── config.rs                  # config feature module root
│   ├── config/                    # config snapshot, enhanced-terminal resource/view and model update
│   ├── sessions/                  # active Session/Thread selection and lifecycle requests
│   ├── thread/                    # canonical snapshot, requests, subscription and projection
│   ├── skills/                    # skill request and selection presentation mapping
│   ├── shortcuts.rs               # shortcut catalog, profile polling and atomic edits
│   ├── shortcuts/                 # searchable view, action menu and key/chord capture
│   ├── status_line.rs             # status-line module root
│   ├── status_line/               # item settings, profile resource, setup view and pure footer view
│   ├── workspace_files.rs         # file-mention search module root
│   └── workspace_files/           # bounded async file-search runtime
├── mouse.rs                        # shared mouse-mode contract for pages and terminal lifecycle
├── host/
│   ├── clipboard.rs               # native text output plus file/RGBA image input
│   └── transcript_export.rs       # workspace-bounded, no-overwrite Markdown export
├── test_support.rs                 # test-only canonical aggregate fixture defaults
├── terminal/
│   ├── session.rs                 # transactional terminal acquisition and RAII restore
│   └── terminal_probe.rs          # bounded OSC query before the crossterm event reader starts
└── ui/
    ├── layout.rs                  # shared pure geometry
    ├── theme.rs                   # shared token subset and detected color-level projection
    └── theme_tests.rs             # TrueColor/ANSI/monochrome projection contract
```

实现 module 都是 private；crate 只导出启动 contract。

## 内部接口地图

| Symbol | 可见性 | 当前职责 | 方向约束 |
| --- | --- | --- | --- |
| `App` | crate-private | presentation `Status`、feature/局部交互协调和单写者 state transition | 不保存 worker/channel、不复制 feature state 或编辑器细节 |
| `AppCommand` | crate-private | execute/quit/interrupt/suspend/copy/export/history/skill/Turn 的 typed side-effect intent | 只描述待执行行为，不携带任意闭包或执行 I/O |
| `AppEvent` | crate-private | config、clipboard、file search、Thread/Turn 与 product command 的已完成事实 | 只能由 `App::update` 改变 presentation state |
| `TurnActivity` | crate-private | canonical Turn status 到 Working/waiting/Cancelling presentation state 的窄映射 | 不复制完整 Turn reducer |
| `ThreadFeatureState` | crate-private | active canonical `Thread` snapshot、transcript projection 与本地 optimistic/diagnostic overlay | 下一份 snapshot 替换 projection；不执行 RPC、不复制 product reducer |
| `ThreadPresentationEvent` | crate-private | snapshot/transient/reset/user/notice/failure/interrupted/clear 的 feature-local 事实 | 只改变 active Thread presentation owner |
| `components::transcript::{Message,MessageRole,draw}` | crate-private | 定义 transcript-facing 展示值，并渲染 role chrome、empty state、wrapping 与 bottom scroll | 不依赖 feature/`App`、不保存 Thread/sequence、不处理输入 |
| `zeta_ansi_escape::ansi_text` | dependency public API | 把 Tool stdout/stderr 的 ANSI SGR 和 tab 转为 Ratatui-owned styled text | 实现归 [`zeta-ansi-escape`](../ansi-escape/README.md)；TUI 不复制 parser、不修改 protocol/Thread 原始输出 |
| `ui::layout` | private module | 跨 surface 复用的纯 geometry | 不读取 App/feature、不调用 terminal 或 RPC |
| `ui::theme` | private module | 将 `zeta-theme::ThemeSnapshot` 的明确子集投影到终端能力 | 不复制完整 Desktop token catalog、不拥有用户文件加载、不定义产品状态 |
| `Status` | crate-private | Ready/Working/waiting/Cancelling/Error display state | 只能由 canonical snapshot/result驱动 |
| `StatusLineModel` | crate-private | 按配置顺序把当前权限、preferred model、Git 分支和变更映射为长短展示值并执行宽度降级 | 不接收完整 config aggregate、不查询接口、不保存权限或 Turn authority、不渲染 |
| `StatusLineResource` | crate-private | 有界读取、revision 校验并原子保存 `<profile>/zeta-code/statusline.json` | 只保存四个显示开关，不进入 App Server 配置、不拥有被显示的数据 |
| `ConfigResource` | crate-private | 有界读取、revision 校验并原子保存 `<profile>/zeta-code/terminal.json` | 只保存终端本地设置；服务端配置仍来自 `config/read` |
| `components::welcome::WelcomeModel` | crate-private | 在 App 构造阶段把 workspace 路径缩写为 `~/...`，供空会话 Welcome Banner 使用 | 不在 draw 中读取环境，不把路径复制到 status line |
| `App::update` | crate-private | 将一个 `AppEvent` 应用到唯一 presentation state owner | 不执行 I/O、不访问 runtime resource |
| `App::handle_key` | crate-private | 先路由 Chord prefix；其他键先委托局部输入，再处理未消费的应用级键 | 不直接调用 client |
| `AppKeymap` | private | 把 Crossterm key 转为共享 `KeyStroke`，解析应用级 action，并拥有 Chord pending/超时/取消/提示生命周期 | 不处理 composer 编辑、selection 导航、滚动、I/O 或命令副作用 |
| `features::shortcuts::ShortcutResource` | private | 有界读取产品 profile JSON、检测外部修改、revision 校验、原子保存，并在完整编译后替换 User rules | 不解析按键语法、不执行 action、不读取 Remote Workspace 文件 |
| `features::shortcuts::view` | private | 从 `AppKeymap` 快照和固定操作目录生成 `/shortcuts` 的搜索/诊断列表、动作菜单和临时按键录制状态 | 不执行快捷键、不建立第二套 Resolver |
| `App::activate_slash_command` | crate-private | 将鼠标命中的 command index 委托给 composer 并复用 command dispatch | 不计算 terminal geometry |
| `App::quit_or_interrupt` | private | active state interrupt；idle/error quit | Cancelling 不重复发送 interrupt |
| `client::EventPump` | crate-private | 独立等待 terminal input、Unix termination signal 与 `AppServerEvents`，通过 1024 项有界队列汇入单写者 loop | Tick 可合并；control/input 不静默丢失；不应用 UI state |
| `client::RequestTask<T>` | crate-private | 在独立 worker 执行一个 typed request 并以单槽 completion 非阻塞回投 | 不修改 `App`、不解释领域结果 |
| `app::request_completion` | private module | 校验 request scope、安装 subscription/snapshot 并把 typed completion 映射为 `AppEvent` | 不执行 renderer、不复制 reducer |
| `client::map_event` / `ClientEvent` | crate-private | 把共享 connection event 映射为 agent request、skills/Git changed、Thread update 与 connection failure | 不保存 transport、不应用 projection |
| `ThreadSubscription` | crate-private | 分开维护 durable sequence、stream-instance cursor 与 history Turn cursor，分类 duplicate/gap/runtime switch，消费 bounded snapshot 和 older-page resync | 不应用 `ThreadEvent` reducer、不保存 Thread history 或 transient projection |
| `features::interactions` | crate-private | full agent request → approval/user-input view state → exact typed response | 不决定 policy、不选择 owner、不支持未声明的 dynamic Tool |
| `InteractionPane` | crate-private | 保留 composer、拥有 temporary view stack，并把 key/paste 路由到 active view 或 composer | 不保存 Plugin/Session 等产品 feature 状态 |
| `components::tab_list::TabListState<T>` | crate-private | 拥有 tab 集合和当前项，处理左右/Tab 循环切换与鼠标命中，并由同模块按 Unicode 宽度统一换行和绘制 | 不拥有 pane 内容、搜索、选择或产品 action |
| `components::selection::SelectionViewState` | crate-private | 可配置 search/titled preview、Space search mode、过滤索引、候选 presentation highlight、选择与循环导航，并组合 `TabListState<SelectionTab>` 切换候选集合 | 不执行 action、不依赖产品 ID 或 App Server |
| `components::selection::draw` | crate-private | generic title/search/items、可配置间距的水平分隔 preview、caption/footer Ratatui surface，并把 tab 区域委托给 `components::tab_list::draw` | 只读 selection state、不解释产品 action |
| `ChatComposer` | private | blank/trim/submit、多行换行、paste routing、slash completion application、参数结构化与 local dispatch | 不自行实现 slash grammar，不拥有 cursor、Vim state 或 RPC |
| `Attachments` | private | 图片 bytes/path、共享格式识别/data URL helper 与原子占位符绑定、删除后重新编号 | 不解码或缩放图片、不替代 Core 权威校验、不直接读取系统 clipboard、不发 RPC、不渲染 |
| `host::clipboard::read_image` | crate-private | 从本机 clipboard 文件列表/RGBA image 读取并统一编码 PNG | 不改变 composer、不发 RPC、不持久化临时文件 |
| `host::clipboard::write_text` / `host::transcript_export::write` | crate-private | command-based response copy 与 workspace-bounded Markdown export | 不拥有 transcript、不覆盖文件、不实现任意屏幕文本 selection |
| `FileSearchManager` | crate-private | event loop 持有的 workspace search runtime；非阻塞 drain snapshot 并丢弃旧 query 结果 | 不进入 `App` state、不解析输入、不保存 popup state |
| `Mentions` / `MentionPopup` | private | `@token` query/range、异步结果应用、选择/关闭和原子路径补全 | 不扫描 workspace、不拥有 worker、不构造结构化 app/plugin Mention |
| `PendingPastes` | private | 超过 1000 字符的 text-paste payload、唯一占位符与提交时展开 | 不识别或保存图片，不解释 slash、不渲染、不直接提交 |
| `zeta_slash_commands::SlashCommandsState` | shared public type | 拥有 cursor query、matches、selection、dismissal 与 completion | TUI 不保存第二份 Slash query/selection authority；可见范围与滚动仍由 Ratatui renderer 负责 |
| `zeta_slash_commands::{SlashCommandInput,SlashCommandCatalog}` | shared public types | 统一输入 grammar，并合并 built-in 与 server metadata | TUI 不重新校验名称、不执行 App Server operation |
| `SlashCommandInvocation` | crate-private | command identity、trimmed display arguments 与有序 text/image argument items | 不执行 RPC |
| `features::sessions::ActiveConversation` | crate-private | 当前 Session/Thread identity、sequence、后端保存的下一次批准模式与 typed create/fork/resume/rewind/archive lifecycle | 不解析 composer text、不拥有批准策略或 App Server |
| `TextArea` | private | UTF-8 多行 buffer、byte-safe line/cursor movement、原子元素 insert/delete 与局部 keymap 扩展边界 | 不保存 paste payload，不解释 Enter submission 或 slash command；当前不承诺 Vim mode |
| `features::thread::submit_prompt` | private | 从显式 `ThreadRequestScope` build typed `session/request` `StartTurn` operation 并返回 typed result | 不引用或更新 `App`、不手写 method string/JSON |
| `App::approval_mode_status` | crate-private | 缓存 Session 的下一次模式与 active Turn 的冻结模式，供 footer 展示；Shift-Tab 只产生 Session mutation intent | 不成为权威状态、不判断或绕过批准策略 |
| `app::request_completion::apply_thread_snapshot` | private | 安装 canonical snapshot、恢复最早 nonterminal Turn 作为执行队首并协调 presentation mapping | 不 drain notification；snapshot 是 authoritative UI source |
| `features::thread::interrupt_turn` | private | 从显式 scope 执行 typed Turn interrupt 并返回结果 | 不引用或更新 `App` |
| `app::apply_active_turn_snapshot` | test-visible | canonical Turn presentation outcome → `AppEvent` | 不从 log/text 猜 terminal state |
| `present_turn_error` | private | stable Turn error code → user-facing recovery message | 不显示 Rust Debug/provider secret |
| `client::new_command_id` | private | process ID + wall-clock nanos 分配 `CommandId` | 一次逻辑 command 一个新 ID |
| `app::frame::draw` | crate-private | frame 分区并协调 feature/component renderer | 不改变 App state |
| `app::frame::{slash_command_index_at,mention_index_at}` | crate-private | 复用 popup geometry 映射可见行点击 | 不执行命令、不改变选择状态 |
| `components::transcript::row::estimated_wrapped_rows` | private | Unicode display-width based scroll estimate | width 0 不 panic |
| `TerminalSession::open` | crate-private | 进入 raw/alternate/paste mode 并创建 backend | partial failure 必须 rollback；默认不捕获鼠标 |
| `MouseMode` | crate-private | 页面声明 `TerminalSelection` 或 `UiClick`，供 App 与终端共享同一鼠标模式契约 | 不执行终端副作用、不保存页面身份 |
| `TerminalSession::set_mouse_mode` | crate-private | 应用当前页面声明的鼠标模式并切换终端全屏鼠标捕获 | 不判断具体页面、不处理点击坐标；重复调用保持幂等 |
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
├─ client.create_session / client.request_session(CreateThread)
├─ client.subscribe_session → Session snapshot + child projections
├─ client.subscribe_session_thread → ThreadSubscription + initial canonical snapshot
├─ TerminalSession::open
├─ EventPump::start → terminal source + App Server source
├─ FileSearchManager::new
├─ App::for_workspace → WelcomeModel::for_workspace + StatusLineModel::new
├─ ConfigResource::refresh → AppEvent::ConfigSettingsReceived → App::update
├─ StatusLineResource::refresh → AppEvent::StatusLineSettingsReceived → App::update
├─ client.read_config / git_status → AppEvent → App::update
└─ loop
   ├─ EventPump::recv
   │  ├─ terminal event → input routing
   │  └─ App Server event → typed notification mapping
   ├─ App::mention_query → FileSearchManager::{update_query,stop}
   ├─ FileSearchManager::poll → AppEvent::FileSearchSnapshotReceived → App::update
   ├─ RequestTask::poll → request completion → app::request_completion → App::update
   ├─ skills changed → queued background skills refresh
   ├─ newer active Thread durable update → queued session/thread/read snapshot resync
   ├─ transient update → cursor validation → bounded Thread projection
   ├─ App::mouse_mode → TerminalSession::set_mouse_mode
   ├─ TerminalSession::draw → app::frame::draw
   └─ terminal event
      ├─ key → App::handle_key
      │  ├─ local input → InteractionPane
      │  │  ├─ active selection view → local view state；可搜索模型先用 Space 进入 search mode
      │  │  └─ no active view → ChatComposer → TextArea
      │  ├─ ReadClipboardImage → clipboard::read_image → AppEvent → App::update
      │  ├─ Quit → return
      │  ├─ SubmitTurn → RequestTask(submit_prompt + canonical read)
      │  └─ Interrupt → RequestTask(interrupt_turn + canonical read)
      ├─ left mouse down → generic Selection / mention / slash hit testing
      │  ├─ Selection item hit → App::activate_visible_item → existing feature action mapping
      │  ├─ mention hit → App::activate_mention → atomic path completion
      │  └─ slash hit → App::activate_slash_command → existing command dispatch
      ├─ mouse moved → same hit testing → existing selected item
      └─ Paste → App::handle_paste → InteractionPane
         ├─ active searchable selection view in search mode → search query
         └─ no active view → ChatComposer
         ├─ image path → Attachments + TextArea atomic placeholder
         └─ text → PendingPastes + TextArea
```

Session create、Thread create 和下一次批准模式变更使用独立 `CommandId`。Turn start/interrupt 使用当前 `thread_sequence` 作为 expected sequence；批准模式变更使用 Session sequence。client error 会进入 visible error message/status，不退出 terminal session。

创建后通过 `session/thread/read`/`session/thread/subscribe` 返回的 canonical snapshot 设置 initial sequence，
不存在硬编码的初始 sequence。所有可能等待 App Server 的 product command、Turn mutation、文件
浏览和配置 mutation 都在 `RequestTask` 中执行；同一时刻只 dispatch 一个 request task，后续用户
intent 保序排队，Quit 和纯本地 theme/clipboard 操作不被阻塞。Thread/Session 变更在后台先建立新
subscription，再把新 `ActiveConversation`、`ThreadSubscription` 与 snapshot 作为一个 completion
安装；旧 scope completion 不能直接修改 `App`。

`Event::Paste` 与普通 key editing 使用不同入口。`PendingPastes` 先把 CRLF/CR 规范化为 LF，
再以 Rust `char`（Unicode scalar value）数量判断大小：不超过 1000 时直接写入 `TextArea`；超过阈值时写入
`[Pasted Content N chars]` 原子元素并在内部保留原文。相同字符数的多个待提交 paste 使用
`#2`、`#3` 后缀避免绑定歧义。移动光标会整体跨过占位符，删除占位符会同时丢弃对应 payload；
`ChatComposer::submit` 在 trim、slash recognition 和 user-message recording 之前展开仍然存在的
占位符。

图片 paste 先尝试把完整字符串解释为本地文件路径；支持引号包裹和 shell 风格反斜杠转义。
`Attachments` 通过 `zeta-utils-image` 的共享签名识别与 data URL helper 处理
PNG/JPEG/GIF/WEBP，拒绝超过 16 MiB 的文件，并在草稿期编码为 base64 data URL，避免提交时路径
失效；提交时 `RequestTask` 使用 bounded chunk RPC 上传，真正的解码、资源限制、规范化和 durable
identity 由共享 backend 接受边界执行。占位符绑定到稳定的
`TextElementId`，光标移动和删除保持原子性，删除后剩余图片会重新编号。提交时
`ChatComposer` 按草稿顺序生成 text/image items；
展示记录保留 `[Image #N]`，App Server/Core 持久化 attachment digest、格式、长度和尺寸，而不是
本地路径或 data URL。

`Ctrl-V` 是独立的 clipboard-image intent，不依赖 terminal `Event::Paste` 是否能携带位图。
adapter 优先读取 clipboard file list 中可解码的图片，否则读取 RGBA image data，并统一编码为
PNG bytes；`App` 再把 bytes 交给 `Attachments`，因此系统剪贴板和本地路径共享大小校验、
占位符绑定、删除和提交语义。active Turn 期间同样可把图片加入 follow-up draft。

data URL 不进入 command receipt、durable Thread history 或 snapshot。当前 16 MiB 单图上限、192
KiB upload chunk、connection-owned upload session 与共享缓冲上限共同构成 RPC 保护边界；TUI
不能建立私有附件 authority，也不能绕过共享远程 URL 安全导入。

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

Built-in command 进入 `app::dispatch::execute_product_command`：dispatcher 从 clone 的
`ActiveConversation` 调用 typed Session/Thread API，只返回 `ProductCommandOutput`；主循环不在
dispatcher 内等待 RPC。查询命令读取 authoritative config，`/model` 通过 expected revision
mutation 更新 preferred model。`/help` 和 `/skills` 复用 generic interaction selection surface；关闭
它们会恢复一直保留的 composer。`/skills` 映射 App Server 的 immutable catalog snapshot；Manage
tab 的 `Enter` 产生 source-qualified `SkillId` enablement intent，成功写入 config 后重新读取页面。
catalog/file watcher 变化通过 `skills/changed` 同时刷新管理页面和动态 Skill commands。TUI 不读取
Skill filesystem；`/name` 只提交 typed `SkillRef`，正文 activation/context injection 仍由 App
Server 与 Core 拥有。没有对应 typed contract 的产品命令不进入
registry，不显示占位提示，也不转成普通 prompt 冒充成功。

## 快照→ UI 映射

`apply_active_turn_snapshot` 观察当前执行队首；队首进入 terminal state 后选择最早的下一个
non-terminal Turn，因此 follow-up queue 不会把 interrupt/status 错绑到队尾：

| Canonical `TurnStatus` | UI effect |
| --- | --- |
| `Created` / `Running` | `Status::Working` |
| `WaitingForApproval` | waiting status；owner-directed approval Pane；仍可 interrupt |
| `WaitingForUserInput` | waiting status；owner-directed multi-question Pane；仍可 interrupt |
| `WaitingForCapability` | waiting status；当前不能 resolve |
| `Cancelling` | `Status::Cancelling`，抑制重复 interrupt |
| `Completed` | canonical transcript 已包含该 Turn 所有 Item；返回 Ready |
| `Failed` | 显示 stable Turn error，清除 active turn |
| `Interrupted` | 添加 notice，返回 Ready |

Completed Turn 没有 Agent message 会被显示为 error。已知 stable Turn error（包括 interaction
deadline）由
`present_turn_error` 映射成面向用户的恢复提示，错误详情只在 transcript 出现一次；footer 只说明
可以 retry 或退出。`features/thread/projection.rs` 按 canonical Item 顺序投影所有 user/agent、
reasoning、plan 与 Tool row；它不从展示内容判断 Turn terminal state。

`client::map_event` 保留 typed `ThreadUpdateEnvelope`；`ThreadSubscription` 验证 active
Session/Thread scope，durable sequence 新于最后确认 snapshot 时触发一次后台
`session/thread/read`。transient cursor 在每个 stream instance 内必须连续；duplicate 被忽略，
gap/runtime switch 会移除旧 transient row 并 resync。canonical snapshot 替换全部 projection，
transient 永远不决定 completed/failed/interrupted。

## 键盘状态机

下列根级组合由 `keymap.rs` 的单一静态声明注册到共享 `zeta-keybinding` Resolver，并由同一声明生成 `/shortcuts` 的可配置项。运行时结构叫 `AppKeymap`：多段 Chord prefix 在 component 前匹配，普通单键仍先经过当前 interaction/component，只有未消费事件进入应用级 fallback。组合精确匹配修饰键，因此 `Ctrl-Shift-V` 不会触发只声明为 `Ctrl-V` 的动作。

`AppKeymap` 支持一至四段 Chord，pending 后在 footer 显示已输入前缀和 Esc cancel；1 秒超时、上下文变化、Esc 或 blocker 会清空 pending，错误后续键清空 pending 后继续作为普通输入透传。当前内建表仍只声明单段组合。`Esc Esc` rewind 是独立的根级状态，不属于通用 Chord，因此 Esc 可无歧义地取消 pending。

用户配置不是 `GlobalKeymap`。它以 `BindingSource::User` 合并进同一个 `AppKeymap`；省略 `when` 只表示该规则在 Zeta Code 的所有上下文中适用。`/shortcuts` 以“快捷键、职责、default/user 来源”三列展示固定操作键和应用级绑定，内部 command ID 不进入界面；选择可配置 action 后可替换该 action 的 User 项、追加单键或两段 Chord、清除 User 项，但不会移除 default 键位或 `command: null` blocker。直接编辑 JSON 仍支持一至四段 Chord、平台覆盖、`when` 和 blocker。保存先检查界面打开时的资源 revision，再完整编译临时规则并原子替换文件与运行时映射；失败不改变当前映射。完整契约见 [`docs/keybindings.md`](../../docs/keybindings.md)。

```text
Ready / Error
├─ Enter(non-empty) → Submit → Working
├─ Shift-Tab → cycle next-Turn approval mode
├─ Shift/Alt-Enter 或 Ctrl-J → insert newline
├─ Enter(/quit) → Quit
├─ Enter(其他 built-in command) → structured invocation → typed command dispatcher
├─ Enter(server dynamic command) → preserve /name + ordered arguments → Submit
├─ /query → cursor-aware popup；↑/↓ select；Tab range completion；Esc dismiss
├─ @query → workspace file popup；↑/↓ select；Tab/Enter complete；Esc dismiss
├─ popup 可见行左键单击 → 补全 mention 或执行 slash command
├─ Esc / Ctrl-C / empty Ctrl-D → Quit
└─ typing/paste/cursor movement/editing accepted

Working / Waiting*
├─ Esc / Ctrl-C / empty Ctrl-D → Interrupt → Cancelling
├─ Working: Shift-Tab → cycle mode for later submissions
├─ Working: typing/paste/editing accepted；Enter → queue follow-up Turn
└─ Waiting*: owner-directed interaction Pane owns input until resolved/interrupted

Cancelling
└─ further quit/interrupt keys ignored until snapshot terminal state
```

`AppEvent::InterruptFailed` 把状态恢复到 Working，使用户可以再次请求 interrupt；ordinary
client failure 通过 `AppEvent::FailureReported` 进入 Error 并允许输入新 prompt。

## 终端生命周期

`TerminalSession::open` 按以下顺序获取基础资源：

```text
enable_raw_mode
→ EnterAlternateScreen
→ EnableBracketedPaste
→ Terminal::new
→ clear
```

首帧前由 `App::mouse_mode` 决定是否另行执行 `EnableMouseCapture`。Config 中的鼠标交互关闭时，该方法始终返回 `TerminalSelection`；页面即使声明可点击，也不会捕获鼠标。

`TerminalModeGuard::acquire` 在任一 mode 获取失败时只回滚已经成功获取的 mode。
`Terminal::new` 或 `clear` 失败时，已经构造的 guard 也执行同一路径。成功后
`TerminalSession::drop` 无条件尝试：

```text
DisableMouseCapture（仅在已经捕获时）
→ DisableBracketedPaste
→ LeaveAlternateScreen
→ disable_raw_mode
→ show_cursor
```

cleanup 是幂等的；显式 restore 后 guard Drop 不会重复发出控制操作。cleanup error 被忽略是
Drop 路径的刻意选择，避免 panic during unwind。新增 terminal capability 时必须同时更新
acquisition flag、reverse cleanup 和 `session_tests.rs` 的 partial-failure case。

Ctrl-Z 复用同一个 `restore → SIGTSTP → reacquire` 生命周期；reacquire 任一步失败时再次逆序
回滚已经恢复的 mode，避免 `fg` 后留下半初始化 terminal。

## 渲染

当前 layout 在 composer 模式是底部锚定的三段：

1. expandable、无外框的 transcript；空会话显示由 `components::welcome` 拥有的 responsive Welcome Banner，宽终端使用双栏，窄终端降级为单栏，并在 `Ready when you are` 下方显示 home-relative workspace 路径；
2. 三至八行 composer：上下浅灰水平线，正文随逻辑行增长、最多显示六行，首行以浅灰 `❯` 开始；超过可见高度时跟随光标纵向滚动；
3. 一行 footer 布局区域；普通状态由 `features::status_line` 从左到右显示已启用的权限、模型、Git 分支和 Git 变更，宽度不足时使用短值并从右侧省略。Chord 等操作提示由 `app/frame/footer.rs` 临时覆盖普通 status line。

所有 interaction surface 都以 terminal 底部为锚点：composer/footer 固定在底部，slash/mention popup 从 composer 上沿向上展开；temporary interaction view active 时替换 composer/footer 区域，底边保持不动并按 view 的 desired height 只向上扩张，transcript 至少保留四行。temporary view active 时 footer 不占行。普通 composer 模式下，status line 只消费 `StatusLineModel` 与当前 `ApprovalMode`，不显示本地 `Status`，也不在 draw 中调用 config、Git 或 Thread 接口。
Selection surface 当前包含顶部分隔线、可配置上下间距的标题、可换行 Tabs、搜索框、可滚动窗口和 view-local
footer；关闭后恢复一直保留的 composer state。

Transcript marker 使用 user/agent/reasoning/plan/tool/error 等 role-specific color，正文和 Tool
detail 仍是 plain text。PageUp/PageDown 按五行移动，Ctrl-Home/End 到首尾；新提交默认恢复
follow-latest。`estimated_wrapped_rows` 使用
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
- docs 中把 `app` rich presentation roadmap 或尚未接受的共享 contract 写成 TUI backlog；
- docs 中把 planned approval/streaming 写成当前能力。

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
| `ConfigReadResult` 新字段 | `test_support::empty_config_snapshot` 与真实消费该字段的 feature tests；无关 view tests 不复制完整 aggregate |
| `ThreadItem` variant 字段 | 构造该 variant 的 projection tests 必须显式更新，不能由通用 fixture 隐藏领域不变量 |

## 测试与支持边界

```text
cargo test -p zeta-tui
bazel test //zeta-code/tui:tui-unit-tests
```

测试当前覆盖后台路径句柄的增量查询、Git 忽略规则、稳定排序、高亮索引与旧结果过滤，
以及 Chord prefix/局部键到应用级键的 routing、trimmed/blank submit、slash registry validation、
cursor filtering、range completion、bare/inline submission、dynamic metadata、原子 command token、
structured text/image/paste arguments、popup render/mouse hit testing 与 local quit dispatch、Unicode
Thread notification decode、active scope/sequence resync 判定、
并覆盖游标/编辑、在游标处粘贴、大段粘贴占位符展开/绑定/删除、退出/中断、
keyboard semantics、duplicate interrupt suppression、active-Turn follow-up queue、图片路径识别/占位符删除重编号/结构化提交、多行编辑、
canonical Thread snapshot 替换 optimistic transcript、snapshot identity/sequence 保留、完整
ThreadItem projection、transient identity/UTF-8/容量上限、stream duplicate/gap/runtime switch、
response lifecycle/error/interrupted transitions、interaction view 的
composer 保留、tab list 换行/左右循环切换、
approval 与多问题 option/free-form user input、blocked Esc/Ctrl-C semantics、搜索过滤/选择修复、
selection render，以及 snapshot
terminal/wait/resume mapping，以及 transcript chrome、error 去重、role
label/Unicode/zero-width wrapping、bounded scroll/history window、copy/export，以及 status-line item 顺序/开关、profile 保存、Git 长短值降级、Unicode-safe truncation、welcome home-relative 路径，以及 terminal mode acquisition failure、逆序 rollback、suspend/reacquire 与幂等
restore；还覆盖 request task 非阻塞 completion、request intent 保序、Session picker/archive 与 Thread recovery、
workspace directory/preview 和 interaction deadline。

跨 feature 复用的完整配置快照只由 `test_support::empty_config_snapshot` 构造；各测试随后只修改
自己拥有的字段。该 helper 仅存在于 `cfg(test)`，不会给生产协议增加 `Default`，也不会让真实
`ConfigReadResult` 新字段在 App Server 或消费该字段的 feature 中被静默忽略。相反，直接构造
`ThreadItem` variant 的测试必须明确填写其全部字段，因为这些字段属于被测试对象本身的领域语义。

生产路径同样按能力收窄：`config/read` 的完整聚合只停留在 request adapter 和 `/config` 的 Config、Providers、Language servers 页面；`provider/list` 只投影供应商名、API key 策略与是否已配置，不返回密钥。Model Pane 只接收 preferred model，MCP Pane 只接收 server map，status line 通过 `AppEvent::PreferredModelReceived` 与 `AppEvent::GitStatusReceived` 接收展示数据，通过本地 `StatusLineResource` 接收显示开关。新增 Tool Search 或 CodeIndex 配置字段不会扩散到这些不拥有该能力的展示组件。

Render tests 使用 Ratatui `TestBackend` 固定 empty/error surface，transcript component tests
固定 row estimation；命令行状态测试是通过依据，没有截图/像素基线。完整 fake-transport `run`
event-loop integration 可以继续加强当前 brokered-local 路径，但 remote reconnect trace、Native
Markdown/diff/table、任意鼠标文本框选和完整 pointer parity 都不是当前 TUI 验收项；命令式
copy/export 已由 TUI host adapter 提供。产品要求与
owner 判断以 [`docs/tui.md`](../../docs/tui.md#17-已接受的架构迁移顺序) 和
[`docs/product-lines.md`](../../docs/product-lines.md) 为准。
