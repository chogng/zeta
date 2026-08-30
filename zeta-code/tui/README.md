# `zeta-tui`

> 文档所有权：本 README 是当前 crate 实现、真实调用路径、限制与修改影响的 canonical 文档。
> TUI 的跨 crate ownership、长期稳态架构基准和阶段退出条件见
> [`docs/tui.md`](../../docs/tui.md)；App Server client contract 见
> [`docs/app-server-client.md`](../../docs/app-server-client.md)。本文不把 Proposed 架构写成
> 当前能力。
> 三条产品线与宿主边界见 [`docs/product-lines.md`](../../docs/product-lines.md)。
> 三端快捷键语义与端侧输入边界见 [`docs/keybindings.md`](../../docs/keybindings.md)。
> 源码分层入口见 [`src/README.md`](src/README.md)，当前界面区域见 [`src/layout.md`](src/layout.md)。
> Session、Thread、正文单元和请求布局决定见
> [`docs/tui-chat-architecture-discussion-v15.md`](../../docs/tui-chat-architecture-discussion-v15.md)。

`zeta-tui` 是 `zeta code` 产品线的 TUI 实现。它当前是 `AppServerSession` 上的 presentation
shell：从 owned session 取得 cloneable typed request handle 与独立 `AppServerEvents`，创建或切换
Session/Thread，接受文本、本地图片路径与系统剪贴板图片输入，订阅 active Thread、启动或中断
Turn，并用 canonical Thread snapshot 加有界 transient projection 驱动 Ratatui 呈现。

它不拥有 Agent runtime、Session/Thread reducer、App Server connection composition、model、
Tool、approval policy 或 persistence。

## 当前能力

- 启动时创建一个 product Session 和 root Thread；
- 最多显示六行正文的多行 `ChatInput`，支持 typing、Unicode-safe cursor/editing、Shift/Alt-Enter
  换行、按行 Home/End/Up/Down、Unicode display-cell 软换行与 bracketed paste；Config 的 Input mode 可在 Standard 与局部 Vim 编辑间切换，Vim 状态只存在于每个 Thread 的 ChatInput 草稿，补全弹层优先消费按键；超过 1000
  个 Unicode scalar value 的 paste 会显示为原子占位符，并只在提交时展开；
- 粘贴 PNG/JPEG/GIF/WEBP 本地文件路径时立即读取最多 16 MiB 的图片，显示可原子编辑的
  `[Image #N]` 占位符，并以结构化图片项提交；
- `Ctrl-V` 从系统剪贴板读取图片；文件列表与原始 RGBA 位图都会统一进入同一附件占位符和
  结构化提交路径；
- `@` 打开 File/Plugin 混合 Mention；File 候选由 `zeta-file-search::PathSearchHandle` 在后台增量扫描和 fuzzy 匹配，Plugin 候选来自 `plugin/list` 的 effective package。两者共用循环选择、鼠标 hover、Tab/Enter completion 和 Esc dismiss；File 以当前目录的相对路径插入，Plugin 以原子 `@plugin-id` 插入；
- `/` 打开 command popup，支持 cursor-aware prefix filtering、循环选择、保留已有参数尾部的 Tab completion、Esc dismiss、鼠标 hover 跟随选中与左键单击可见命令；
- `$` 打开独立 Skill selector；enabled、compatible 且名称无歧义的 Skill 显示为 `$name`，Tab/Enter 或鼠标选中后作为原子文本插入。提交时保留 `$name …` 用户文本并附加 exact pinned `SkillRef`，完整 `SKILL.md` 只在 App Server 接受 Turn 后按需加载；Skill 与 `/name` 命令不冲突，`skills/changed` 会刷新候选；
- `/resume`、`/rewind`、`/clear`、`/add-dir`、`/fork`、`/model`、`/theme` 与 `/new` 可解析 inline arguments，并在执行前展开 large-paste placeholder；product command 明确拒绝 image arguments；
- command popup 只注册已有真实执行流的 built-ins；除原有产品命令外，`/agents` 与 `/sessions` 进入覆盖完整 Session catalog 的唯一 Session Manager，`/subagents` 聚焦常驻 SubagentPane，`/queue` 管理当前 Thread 的 Queue；Manager 保留顶部 Welcome，按 Pinned、Needs input、Working、Ready for review、Failed、Stopped、Completed、Idle 分组，每行以状态图标开头并显示名称、当前操作/问题和状态时长；Approval 和 Query 仍由各自请求直接打开，不提供总括页面；
- `/status` 使用不改变正常布局高度的 QuickView，展示当前 Thread 最近一次 Turn 使用的模型、上下文窗口以及 Session ID、Thread ID 和 Thread sequence；provider usage 不完整时剩余值标记为估算，尚无可信值时显示 unknown；
- `/help` 从 `ThreadPresentationStore` 保存的合并 `SlashCommandCatalog` 构造可搜索的 `ListSelection` Pane，本地与服务端命令使用和 `/` 补全一致的名称、描述及顺序；上下键在 item、SearchBox 和 Tab 栏间移动输入焦点，Tab 栏用左右键或 Tab 切换，焦点不再额外绘制紫色状态列；Esc/Ctrl-C 逐层返回保留原草稿的 `ChatInput`；Pane 打开期间自身占用 ChatInput 的可见高度，底栏只保留 `←/→/Tab to switch · Esc to close`；快捷键只由 `/shortcuts` 展示；
- `/skills` 通过 typed `skills/list` 打开同一 interaction surface，提供
  All/Enabled/Disabled/Manage tabs、数量、搜索和 source-qualified metadata；只有 Manage
  tab 的动作通过 revision-checked `skill/enablement/set` 修改 enablement；该页面是目录管理入口，
  不直接激活 Skill；Skill 错误由 App Server 判断并通过 `skills/list` 返回，TUI 将新出现的诊断写成
  Notice，同一条持续存在的诊断不重复提示，消失后再出现或内容变化时重新提示；
- `/connectors` 通过 typed `connector/list` 打开 Connector Pane；已连接项可以执行
  generation-checked disconnect，`connector/changed` 只在该 Pane 打开时触发 catalog refresh；
  API token/OAuth 连接仍由 Desktop Settings 完成；
- `/add-dir <path>` 通过 typed App Server RPC 把 canonical 目录加入当前 Session 的访问作用域，并使用 Config → Add-dir 中始终可见的十二项默认授权；初始默认只打开读取与修改文件。本地 `read_file`、`write_file`、`edit`、`grep` 与 `glob` 随即按各自能力接受该目录中的绝对路径，下一 Turn 的 environment snapshot 只列出仍允许读取的目录。不带参数时打开可搜索列表，Enter 撤销所选目录；快照放在模型请求尾部且不写入对话历史，cwd 与相对路径均不改变；
- 同一 profile 中的 `marketplace/changed` 与 `plugin/changed` 会触发 Skill catalog 重读，并在
  Connector Pane 已打开时重读 Connector projection；TUI 当前不提供独立 Marketplace 浏览/安装界面；
- `/rewind` 或主界面 500 ms 内连续按两次 Esc 打开可搜索的历史消息 checkpoint Pane；Enter
  通过 typed `session/request` 的 `RewindThread` operation，创建具有 Rewind lineage 的子 Thread，只导入所选消息之前的
  terminal Turns。原 Thread 保持不变，TUI 切换订阅并以 `/rewind <turn-id>` 记录结果；
- `/resume` 提供 Session picker；`/archive` 通过 typed `session/request` 归档当前 Session，成功后退出 TUI，不创建新 Session；失败时保留当前会话并显示错误；
- `/config` 异步读取服务端配置、供应商目录和当前 Session 的目录权限；Config 标签页包含本地 Mouse interactions、Follow-up messages 与 Input mode，后两项分别选择 Queue/Steer 和 Standard/Vim；Add-dir 标签页提供文件读取/修改/执行/监听、文件浏览/搜索、Instructions、Config、Skills、MCP、Language services、Hooks、Plugins 和 repository Permission，并在已有目录时追加目录级开关，使用 permissions revision 防止旧页面覆盖新选择。Providers 标签页展示后端注册的完整供应商目录，并通过隐藏输入框把 API key 交给 profile SecretStore；`/model` 使用 expected revision 更新 preferred model；
- 启动时读取 client 保存的 `initialize.slashCommands` snapshot，通过
  [`zeta-slash-commands`](../../zeta-rs/slash-commands/README.md) 与 built-ins 做防冲突合并；
  server-advertised command 保留 `/name`、inline text/image/large-paste 参数并作为普通 Turn
  input 提交；slash popup 不清空或铺设独立背景，透明继承当前 TUI 主题 surface，选中项使用候选 highlight 色粗体且不添加行首标记；
- Enter 按 `ChatInput` 草稿顺序提交由 text/image items 组成的 Turn；active Turn 执行期间仍可编辑并提交
  follow-up，Core 的 per-Thread mailbox 按接受顺序串行执行这些 Turn；
- 启动及 `/new`、`/fork`、`/resume`、`/rewind` 后维护 active Thread subscription；`/fork` 的历史继承由 Core 完成，TUI 只切换订阅并安装子 Thread 的权威 snapshot；typed update 按 Session/Thread scope 和 durable sequence 过滤，并通过 `session/thread/read` 触发权威 snapshot resync，不在 TUI 内复制 Thread reducer；具体 fork 语义见 [App Server API](../../docs/zeta-app-server-api.md#分叉-thread)；
- `AppServerEvents` 与 terminal input 由独立、有界 event source 主动唤醒单写者 loop；typed request
  由 `RequestTask` 在后台执行，完成结果回到 event loop，排队的用户 intent 不会静默丢失；active
  Turn 不再使用 25 ms `session/thread/read` polling fallback；
- `app::RedrawScheduler` 把连续服务端更新、请求完成、资源刷新和可见计时变化合并到首个 16 ms frame deadline，后续请求不能把 deadline 向后推迟；终端输入立即画，空 Tick 不画，Submit/Queue/Steer 不再额外插入重复帧；
- `app::TranscriptBatch` 在同一 frame deadline 内只归约同 Session/Thread/durable sequence、同 stream instance 且 cursor/revision 连续的 transient 完整 `Upsert`；批次最多 256 个 identity、1024 次更新和 1 MiB 正文，同 identity 使用最后完整值，committed、Remove、ClearTransient、gap 和任意输入/控制事件都会结束批次且不被吞掉；
- transcript 投影完整显示 user text/image、所有 agent message、reasoning、plan、ToolCall 与
  ToolResult；`ItemDelta`、`PlanUpdated` 与 `ToolOutputDelta` 按 stream instance/cursor 增量更新，
  gap 会清除不可信 transient row 并读取权威 snapshot；单个 transient row 限 256 KiB、最多保留
  1024 个 transient identity；Tool stdout/stderr 只在 Ratatui render boundary 通过
  [`zeta-ansi-escape`](../ansi-escape/README.md) 将 ANSI SGR 转为 styled spans，并把 tab 投影为四个
  空格，protocol 与 Thread presentation state 继续保留原始输出；
- owner-directed `agent/request` 支持 approval（approve once/decline）和多问题 user input；只有
  App Server 选中的、声明对应 capability 且订阅该 Thread 的 connection 能 resolve。交互不可用
  Esc 关闭，但可 Ctrl-C interrupt；deadline 由 App Server 执行并投影为稳定 Turn failure；
- StatusLine 最多两行：第一行显示 working/waiting、Plan、Queue 与当前 Session 的后台 Subagent 数量，配置行按 `[tui].statusLine` 的顺序显示 `/statusline` 启用的权限模式、模型、Git 分支和 Git 变更；Manager、Pane 和其他需要明确操作键的表面由固定一行 KeyHints 直接替换。Shift-Tab 切换下一次 Turn 的权限模式；
- 根级 `keymap.rs` 只保留运行时入口和 `AppKeymap`，`keymap/bindings.rs`、`keymap/chords.rs` 与 `keymap/input.rs` 分别拥有动作绑定、Chord 生命周期和 Crossterm 转换；共享 Resolver 处理 Shift-Tab、根级 Esc 与 Ctrl-C/D/O/V/Z，并生成设置界面只读快照。`features/keymap.rs` 解释 `[tui].keybindings` 的 User command/blocker、平台覆盖与 `when`，并为 `/shortcuts` 汇总可配置绑定和固定操作键，提供搜索、诊断、单键/两段 Chord 录制、config revision 校验和完整规则校验；保存通过 App Server 替换完整 `[tui]` 表，坏更新或保存失败保留上一份有效规则。`ChatInput` 编辑、`ListSelection` 导航和 `ChatHistory` 滚动仍由各 component 拥有；
- `ChatInput` 保存最近 100 条纯文本提交，Up/Down 可召回并恢复原 draft；`ChatHistory` 支持
  PageUp/PageDown 与 Ctrl-Home/Ctrl-End。初始 Thread snapshot 只读取最近 50 个 Turn，Ctrl-Home
  通过 App Server 的 durable Turn cursor 请求更早的 50 个 Turn，并在 presentation projection 中
  合并页面；TUI 不保存 Thread history；
- `/copy` 或 Ctrl-O 把最后一条 Agent response 写入系统剪贴板；`/export [relative-path]` 以
  Markdown 导出当前已加载的 transcript history window，路径限制在当前目录内且绝不覆盖已有文件；
- Mouse interactions 开启时，所有页面统一捕获左键：拖动按当前 Ratatui frame 的字符网格形成跨行选区，双击选择连续 Unicode 单词、词间空白或符号，三击选择当前可视行，完成选择后立即写入系统剪贴板；单击继续执行 Pane、ChatInput completion、Approval、Query 或 transcript marker 的原有动作。连续点击要求 500 ms 内落在同一行相邻字符，拖动或超时会重新开始计数。选区按 Unicode 字符宽度跳过宽字符占用的后续单元格；只有选区延伸到屏幕右边界时才裁掉行尾终端填充空格，明确选中的词间空白原样保留；
- 空会话和 Manager 顶部的 Welcome Banner 在 `Ready when you are` 下方显示以 `~` 缩写用户主目录的当前目录路径；底部直接使用 StatusLine 或固定一行 KeyHints，不再套额外容器；
- Ctrl-C 或 Ctrl-D（空输入）在 idle 时退出，active 时请求 interrupt；单次 Esc 在根界面保持
  inert，连续两次 Esc 打开 Rewind Pane；
- Unix `SIGINT`/`SIGTERM` 进入同一个 event loop 退出路径，确保 watcher 重启和 host termination
  仍执行 session shutdown 与 terminal RAII cleanup；
- Ctrl-Z 在 Unix 上先恢复当前启用的鼠标捕获、bracketed paste、alternate screen 和 raw mode，再发送 `SIGTSTP`；`fg` 恢复后按原顺序重新获取所有 terminal mode 并清屏重绘；
- raw mode、alternate screen、bracketed paste 与 cursor cleanup；Mouse interactions 开启时在整个 TUI 会话捕获鼠标，关闭时释放捕获并把拖拽文本选择交还终端；
- 启动时通过 App Server 从 `<profile>/config.toml` 的 `[tui]` 读取主题和终端设置，并从 `<profile>/zeta-code/themes/*.json` 读取 TUI 专用用户主题；内置调色板、语义颜色与 TrueColor、ANSI-256、ANSI-16、Monochrome 降级均由本 crate 拥有，不读取 TypeScript token registry、`resources/design-tokens`、`configuration.json` 或 `zeta-theme`；`features/theme` 拥有 `/theme` 的固定八项 Zeta Code Pane、挂在顶部分隔线上的反色 `Theme` 标题、编号、
  active 标记、候选 frame highlight、仅带上下较高对比度长节虚线的 diff preview、palette 来源说明和选择动作，Pane
  不启用搜索，Enter 原子保存、立即重绘并关闭整个 Theme flow 返回主界面，失败时保留 Pane；成功时以状态圆点、`/theme <id>` 和以 `└─` 归属且与命令文字对齐的 `Theme set to …` transcript 记录执行结果，`/theme <id>` 保留直接切换；
  Auto 在 terminal raw mode 建立后查询一次 OSC 11 实际背景 RGB，据此选择 Light/Dark；查询超时
  后依次回退 `COLORFGBG` 和 Dark。结果在会话内缓存，后续打开 Theme Pane 不重复查询；
- transcript 的行生成、首行/续行前缀、Ratatui 实际折行高度、scroll 与鼠标命中共用同一份派生结果；显式 transcript scroll 默认 follow-latest，逻辑偏移不受 `u16` 限制。每个 Thread 持有独立的 `ChatHistoryRenderCache`：轻量高度覆盖当前正文，Ratatui buffer 只为视口内 cell 生成并有界复用，切走 Thread 时释放重型缓存。`render::highlight` 使用 bundled syntax 定义和当前 Zeta syntax token 生成代码行，transcript fenced code block 通过 `StreamingCodeHighlighter` 只延续以换行结束的完整新增行；未知语言、解析失败或超限源码保持可见原文，Theme Pane 的 Rust diff preview 使用同一入口。

## 产品支持边界

`zeta code` 是键盘优先、低带宽的终端产品，不以复刻 `app` rich UI 为完成条件。
transcript 当前采用 plain-text wrapping 并识别 fenced code block 做代码高亮，但不实现完整 Markdown；桌面 Agent Timeline 的 Markdown block、table、selection、折叠与虚拟化由 [`native-agent-console.md`](../../app/docs/native-agent-console.md) 和 [`zeta-markdown`](../../app/markdown/README.md) 拥有，不构成 TUI backlog。TUI 的鼠标交互覆盖所有页面：拖动选择当前 frame 中可见的字符，双击选择字符类别连续的词、空白或符号，三击选择当前可视行，完成选择后自动复制；单击才进入 ChatInput 的 Slash/File/Plugin completion、Pane、Approval、Query 与 transcript marker 的命中路径。Config 标签页中的 Mouse interactions item 可关闭全部 TUI 鼠标捕获，关闭后选择与复制行为由终端负责。
Vim 只改变 `ChatInput` 的文字编辑行为，不把 Normal/Visual 状态扩散到 Pane、正文选择或应用级快捷键。

TUI 当前连接 CLI 提供的 profile/Directory-scoped App Server authority，不拥有 connection selector 或 transport retry。连接中断时，TUI 丢弃本代 pending request 和 queued action，只向 CLI 交还持久化的 Session/Thread 身份；本地和 Remote CLI 宿主都在 30 秒有界窗口内重建连接，再让 TUI 从权威 snapshot 恢复。重连失败时，CLI 分别输出 `zeta resume SESSION_ID THREAD_ID` 或绑定原 Remote 连接的 `zeta remote connect ... --resume SESSION_ID THREAD_ID`，不会丢掉可恢复身份。Desktop 与 app 在相同 authority partition 下可以实时读取同一份 Session catalog 和 Thread event。File mention 插入当前目录的相对路径，Plugin mention 插入 effective package 的原子 `@plugin-id`；TUI 不另造 `app://`/`plugin://` 协议身份。

图片 bytes 的持久化由共享 `zeta-attachments` content-addressed store 拥有；TUI 只在草稿期间保留
本地 data URL，并在 `StartTurn` 前通过 App Server 分块上传或安全导入远程 URL，最终只提交 typed
`ImageAttachmentRef`。`/status` 只消费 typed model capacity 与 Turn `contextUsage`，不从 transcript 推导上下文占用。缺少 typed backend contract 的 login、compact、service tier 等命令不会进入 registry。`/statusline` 编辑 `[tui].statusLine` 中有顺序的权限、模型、Git 分支和 Git 变更项；Config 页面展示 Config、Add-dir、Providers 与 Language servers。快捷键、状态栏、主题、Mouse interactions、Follow-up messages、Input mode 和 Add-dir 的新增目录默认授权都保存在 `<profile>/config.toml` 的根级 `[tui]` 表；配置后端只保存完整键值表，字段默认值和校验由 TUI 负责。默认授权不保存路径；当前 Session 的目录授权也不写入 `[tui]`。每项开关只授予标题所指的能力，MCP 连接和 Plugin 安装仍由各自流程确认。Providers 来自后端注册表，API key 只通过 `provider/apiKey/set` 写入 SecretStore，不进入普通配置或展示状态。

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
| `TuiOptions::with_dir_root` | 显式覆盖有界 file mention root |
| `TuiOptions::with_profile_root` | 启用产品作用域的 `zeta-code/themes/*.json` 主题文档；其他 TUI 设置来自 App Server 配置 authority 的 `[tui]` |
| `TuiRecoveryState::new` | 从 CLI 参数构造需要重新读取的持久化 Session/Thread 身份，不携带 connection 或待执行请求 |
| `run` | 接管 ready `AppServerSession`，校验 initialize snapshot、驱动 terminal/client events，并在退出时显式 shutdown |
| `TuiExit::UserRequested` | 用户通过按键或 command 请求正常退出 |
| `TuiExit::TerminationRequested` | Unix host termination signal 请求正常退出 |
| `TuiError::Client` | typed App Server client failure |
| `TuiError::EventStream` | App Server event stream 被提前取走 |
| `TuiError::Shutdown` | App Server background driver 关闭失败 |
| `TuiError::Terminal` | terminal setup/event/draw failure |

`run` 接受一个已经初始化的 `AppServerSession`。Transport/brokered-local/embedded/remote 选择与
initialize/schema handshake 属于 CLI 和 app-server-client，不在 TUI 内重复实现；TUI 读取
request handle 保存的 immutable initialize result，在创建 Session 前拒绝非法/冲突的 server
slash snapshot，并且只取一次 connection event stream。

## TUI 主题文件

键盘焦点、键盘当前项、已生效项、鼠标悬停/按下、禁用和文字框选的统一含义及内置颜色由 [TUI 交互与颜色语义](../../docs/tui-interaction.md) 定义；本节只拥有用户主题文件格式。

TUI 设置保存在 `<profile>/config.toml` 的根级 `[tui]` 表：

```toml
[tui]
theme = "graphite"
mouseInteractions = true
followUpMode = "queue"
inputMode = "standard"
```

新增目录默认授权写入 `[tui.dirPermissions]`，只保存权限集合，不保存目录或 Session。用户主题内容仍保存为 `<profile>/zeta-code/themes/*.json`。每个文件最多 1 MiB；目录最多读取 128 个常规 JSON 文件；`id` 必须是小写 kebab-case，`label` 为 1–80 个已去除首尾空格的字符，`appearance` 只能是 `dark` 或 `light`，`colors` 最多覆盖 64 项且颜色必须是 `#RRGGBB`。未知字段、未知颜色名、重复/保留 ID 和不支持的版本都会使该主题文件单独失效。

```json
{
  "schemaVersion": 2,
  "id": "graphite",
  "label": "Graphite",
  "appearance": "dark",
  "colors": {
    "background": "#101010",
    "quickViewBackground": "#303030",
    "actionForeground": "#58a6ff",
    "focus": "#8b80f9",
    "hoverBackground": "#25233a",
    "hoverForeground": "#f0edff"
  }
}
```

可覆盖字段为 `accent`、`accentSurfaceBackground`、`accentSurfaceForeground`、`actionForeground`、`background`、`border`、`chatInputChrome`、`danger`、`disabledForeground`、`focus`、`foreground`、`function`、`hoverBackground`、`hoverForeground`、`insertedBackground`、`insertedMarker`、`keyword`、`muted`、`pressedBackground`、`pressedForeground`、`quickViewBackground`、`removedBackground`、`removedMarker`、`selectionBackground`、`selectionForeground`、`screenSelectionBackground`、`screenSelectionForeground`、`string`、`success`、`type`、`variable` 与 `warning`。未写字段继承所选 `appearance` 的内置调色板；该格式不接受图形界面 token、别名、透明色或颜色变换。

## 文件与职责

```text
src/
├── README.md                       # source ownership and dependency direction
├── layout.md                       # current frame, height-entry and overlay geometry
├── lib.rs                         # narrow public startup API; delegates to app::run
├── keymap.rs                      # runtime owner and module entry
├── keymap/
│   ├── bindings.rs               # root action declarations, conditions, resolver snapshots
│   ├── chords.rs                 # chord validation, pending state, timeout and dispatch
│   └── input.rs                  # Crossterm event normalization and config serialization
├── app/
│   ├── event_loop.rs              # runtime coordination and command scheduling
│   ├── event_pump.rs              # bounded merge of terminal/client/termination sources
│   ├── redraw.rs                  # dirty frame deadline and input-priority redraw scheduling
│   ├── transcript_batch.rs        # bounded coalescing of contiguous transient transcript values
│   ├── state.rs                   # single-writer presentation state
│   ├── event.rs / command.rs      # completed facts / typed side-effect intents
│   ├── dispatch.rs                # built-in product command coordination
│   ├── bootstrap.rs / help.rs     # startup registry validation / help model
│   ├── layout.rs / navigation.rs  # page geometry / global focus and root movement
│   └── frame.rs / frame_tests.rs  # top-level frame assembly and behavior tests
├── app.rs                         # application module root and public crate surface
├── client/
│   ├── command_id.rs              # stable logical command identity allocation
│   ├── notification.rs            # typed ServerNotification → ClientEvent mapping
│   └── notification_source.rs     # AppServerEvents wait lifecycle
├── components/
│   ├── chat_history.rs / chat_history/ # transcript rendering, per-Thread buffer cache and scroll
│   ├── chat_composer.rs / chat_composer/ # ChatInput/Pane priority and submission-target routing
│   ├── chat_input.rs / chat_input/ # draft, editor, attachments, paste and completion
│   │   └── completion.rs / completion/ # Slash, Mention and Skill state and drawing
│   ├── pane.rs / pane/             # PaneSpec, Pane, PaneId and readonly PaneView
│   ├── list_selection.rs / list_selection/ # reusable list selection state and drawing
│   ├── approval.rs / query.rs      # one-shot temporary-layer interactions
│   ├── quick_view.rs               # read-only overlay with shared geometry
│   ├── steer.rs                    # pending Steer delivery identity; no separate surface
│   ├── tab_list.rs                # reusable horizontal tab state, mouse/keyboard input, wrapping and view
│   └── detail_list.rs / text_prompt.rs / key_capture.rs # QuickView detail and concrete Pane bodies
├── features/
│   ├── config.rs                  # config feature module root
│   ├── config/                    # [tui] parsing, server config requests and pane mapping
│   ├── sessions/                  # active Session/Thread selection and lifecycle requests
│   ├── thread/                    # canonical snapshot, requests, subscription and presentation
│   ├── queue.rs / queue/          # stable Queue identity, inline rows and management Pane
│   ├── goal.rs / plan.rs          # one-row Thread-derived content
│   ├── skills/                    # skill request and selection presentation mapping
│   ├── keymap.rs                  # [tui].keybindings parsing, requests and validated edits
│   ├── keymap/                    # searchable view, action menu and key/chord capture
│   ├── status_line.rs             # status-line module root
│   ├── status_line/               # runtime facts, ordered item settings, config requests, setup and bounded view
│   ├── file_search.rs              # file-mention search module root
│   └── file_search/                # bounded async file-search runtime
├── mouse.rs                        # session-wide mouse mode and hover state
├── screen_selection.rs             # drag/multi-click selection, visible-cell boundaries and text extraction
├── host/
│   ├── browser.rs                 # validated HTTPS handoff to the system browser
│   ├── clipboard.rs               # system text output plus file/RGBA image input
│   ├── termination.rs             # process termination signal registration
│   └── transcript_export.rs       # directory-bounded, no-overwrite Markdown export
├── test_support.rs                 # test-only canonical aggregate fixture defaults
├── terminal/
│   ├── event_source.rs            # Crossterm input and bounded Tick source
│   ├── session.rs                 # transactional terminal acquisition and RAII restore
│   └── terminal_probe.rs          # bounded OSC query before the crossterm event reader starts
├── render.rs                      # Renderable measurement/draw contract and module root
└── render/
    ├── highlight.rs               # bounded syntax highlighting with Zeta theme tokens
    ├── highlight_streaming.rs     # complete-line incremental parser state
    ├── layout.rs                  # shared inset and pure geometry
    ├── text.rs                    # line ownership, prefixing and wrapped height
    └── theme.rs                   # immutable RenderTheme and terminal color-level mapping
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
| `components::chat_history::{Message,MessageRole,ChatHistoryView}` | crate-private | 定义 transcript-facing 展示值，并通过 `Renderable` 渲染 role chrome、empty state、wrapping 与 bottom scroll | 不依赖 feature/`App`、不保存 Thread/sequence、不处理输入 |
| `components::chat_history::ChatHistoryRenderCache` | crate-private | 每个 Thread 独立持有的有界派生 buffer cache；height、draw 与 pointer hit test 复用同一结果 | 不保存正文或主题事实、不跨 Thread 共享、不缓存无 revision 或超大 cell |
| `zeta_ansi_escape::ansi_text` | dependency public API | 把 Tool stdout/stderr 的 ANSI SGR 和 tab 转为 Ratatui-owned styled text | 实现归 [`zeta-ansi-escape`](../ansi-escape/README.md)；TUI 不复制 parser、不修改 protocol/Thread 原始输出 |
| `render::{Renderable,RenderContext}` | private | 统一 surface 的宽度测量、绘制入口与只读主题传递 | 不读取 App/feature、不调用 terminal 或 RPC |
| `render::layout` | private module | 跨 surface 复用的纯 geometry | 不读取 App/feature、不调用 terminal 或 RPC |
| `render::text` | private module | 行的借用/持有转换、批量复制、首行/续行前缀和 Ratatui 实际折行高度 | 不解释 transcript role、代码语言或滚动状态 |
| `render::{highlight,StreamingCodeHighlighter}` | private | 在 512 KiB、10,000 行、单行 4 KiB 上限内把明确语言的完整源码或以换行结束的完整新增行映射为主题 syntax span | 不解析 Markdown、不接受半行、不选择或保存主题、不修改源文本 |
| `render::theme::{ThemePalette,RenderTheme}` | private | 定义完整 TUI 语义调色板，并转换为终端能力支持的颜色 | 不读取或保存用户配置、不依赖图形界面 token |
| `features::theme::ThemeResource` | private | 从 TUI 产品目录加载用户主题、解析选择并生成预览 | 不读取或保存配置；draw path 不执行文件 I/O |
| `Status` | crate-private | Ready/Working/waiting/Cancelling/Error display state | 只能由 canonical snapshot/result驱动 |
| `StatusLineModel` | crate-private | 按配置顺序把当前权限、preferred model、Git 分支和变更映射为长短展示值并执行宽度降级 | 不接收完整 config aggregate、不查询接口、不保存权限或 Turn authority、不渲染 |
| `StatusLineSettings` | crate-private | 解释和校验 `[tui].statusLine` 的项目、开关与顺序 | 不拥有被显示的数据；写入时保留 `[tui]` 的其他键 |
| `features::config::TerminalSettings` | crate-private | 解释和校验 `[tui]` 中的终端设置，并在更新已知键时保留该表的其他键 | App Server 只做 revision 校验和完整表替换，不解释 TUI 字段 |
| `components::welcome::WelcomeModel` | crate-private | 在 App 构造阶段把 directory 路径缩写为 `~/...`，供空会话 Welcome Banner 使用 | 不在 draw 中读取环境，不把路径复制到 status line |
| `App::update` | crate-private | 将一个 `AppEvent` 应用到唯一 presentation state owner | 不执行 I/O、不访问 runtime resource |
| `App::handle_key` | crate-private | 先路由 Chord prefix；其他键先委托局部输入，再处理未消费的应用级键 | 不直接调用 client |
| `AppKeymap` | private | 把 Crossterm key 转为共享 `KeyStroke`，解析应用级 action，并拥有 Chord pending/超时/取消/提示生命周期 | 不处理 `ChatInput` 编辑、`ListSelection` 导航、滚动、I/O 或命令副作用 |
| `features::keymap::KeymapSettings` | private | 解释 `[tui].keybindings`，完整编译后构造 User rules 与诊断 | 不执行 action；写入时保留 `[tui]` 的其他键 |
| `features::keymap::pane` | private | 从 `AppKeymap` 快照和固定操作目录生成 `/shortcuts` 的 Pane 创建数据、动作菜单和按键录制状态 | 不执行快捷键、不建立第二套 Resolver |
| `App::activate_slash_command` | crate-private | 将鼠标命中的 command index 委托给 `ChatComposer` 并复用 command dispatch | 不计算 terminal geometry |
| `App::quit_or_interrupt` | private | active state interrupt；idle/error quit | Cancelling 不重复发送 interrupt |
| `app::EventPump` | crate-private | 合并 terminal、client 与 termination 三种独立来源；终止请求、终端输入、服务端事件使用明确优先级和各自有界车道唤醒单写者 loop | Tick 可合并；control/input 不静默丢失；不读取终端或解释服务端通知 |
| `app::RedrawScheduler` | crate-private | 保留首个 16 ms 批量 deadline，并允许终端输入把待绘制帧提前到当前时刻 | 不读取 App、不绘制、不合并或丢弃状态事件 |
| `app::TranscriptBatch` | crate-private | 在当前 frame deadline 内按 scope、stream instance 和连续 cursor/revision 归约 transient 完整 `Upsert`，同 identity 保留最后值，并限制 identity、更新数与文本字节 | 不拼接 token、不跨 committed/Remove/ClearTransient/input/control barrier、不推迟 deadline |
| `terminal::TerminalEventSource` | crate-private | 轮询 Crossterm input，并产生 input、Tick 或 terminal failure | 不依赖 `app`、feature、client 或产品 ID |
| `client::ClientEventSource` | crate-private | 持续等待 `AppServerEvents`，通过 `map_event` 输出 `ClientEvent` | 不读取终端、不注册进程信号、不应用 UI state |
| `host::TerminationSource` | crate-private | 注册进程终止信号并提供一次性消费的 termination request | 不管理终端 suspend/reacquire，不依赖 `app` 或 feature |
| `client::RequestTask<T>` | crate-private | 在独立 worker 执行一个 typed request 并以单槽 completion 非阻塞回投 | 不修改 `App`、不解释领域结果 |
| `app::request_completion` | private module | 校验 request scope、安装 subscription/snapshot 并把 typed completion 映射为 `AppEvent` | 不执行 renderer、不复制 reducer |
| `client::map_event` / `ClientEvent` | crate-private | 把共享 connection event 映射为 agent request、skills/Git changed、Thread update 与 connection failure | 不保存 transport、不应用 projection |
| `ThreadSubscription` | crate-private | 分开维护 durable sequence、transcript revision 与 history Turn cursor，分类 duplicate/gap/runtime switch，阻止旧快照覆盖新流式正文并触发缺口重读 | 不应用 `ThreadEvent` reducer、不保存 Thread history 或 transient 内容 |
| `features::approval::Approval` | crate-private | 保存一次 Approval 的请求身份、选择、提交和错误状态，并生成准确响应 | 不拥有 ChatInput 草稿、不决定 policy 或 owner |
| `features::query::Query` | crate-private | 保存一次 Query 的问题、选择、自定义文本、提交和错误状态，并生成准确响应 | 不借用 ChatInput 编辑答案、不决定 owner |
| `features::thread::TranscriptProjection` | crate-private | 用稳定 `TranscriptCellId` 维护有序 `TranscriptCell`，单条 entry 和 Exec 分组采用确定性身份，并为每次可见内容变化分配单调 render revision | 不成为持久化层、不把 TUI 身份写成 Core 领域 ID、不从显示文字推断产品事实 |
| `features::thread::ExecCell` | crate-private | 按 `ToolCallId` 路由调用、流式输出和结果，执行稳定分组与有界保留 | 不把输出接到“当前命令”、不推测缺失的退出码或时长 |
| `ChatComposer` | crate-private | 在 caller-owned `ChatInput` 与 stacked Pane 之间路由输入，并按 Start/Queue/Steer 选择提交目标 | 不保存补全、Approval、Query、Turn、Plan 等状态，不执行外部副作用 |
| `components::pane::PaneStack` + 内部 `Pane` | crate-private | 分配 `PaneId`、管理页面栈，统一绘制顶边和反色标题，并把封闭 `PaneBody` 的输入、高度、按键提示和命中映射为 `PaneOutcome` | 不拥有 feature action、RPC、Tab/Search 等正文专属能力 |
| `components::tab_list::TabListState<T>` | crate-private | 拥有 tab 集合和当前项，处理 Tab/Shift-Tab 与左右键循环切换、鼠标命中，并由同模块按 Unicode 宽度统一换行和绘制 | 不拥有 pane 内容、搜索、选择或产品 action |
| `components::list_selection::ListSelectionState` | crate-private | 可选 search/preview、过滤索引、候选高亮、选择，并组合 `TabListState<ListSelectionGroup>` 管理 item/SearchBox/Tab 焦点与候选集合 | 只承载真正的列表选择，不执行产品 action |
| `components::list_selection::view` | crate-private | 用与 ChatInput 正文对齐的两列状态位绘制 search/items/preview/caption，并把 tab 区域委托给 `components::tab_list::draw` | 只读 `ListSelectionState`，不绘制 Pane 标题或底栏，不解释产品 action |
| `ChatInput` | private | 草稿、多行编辑、Standard/Vim 局部模式、paste routing、附件、输入历史、原子绑定、Slash/Mention/Skill 补全和结构化提交组装 | 不发现候选数据、不打开 Pane、不执行产品动作、不把 Vim 或补全状态提升到 App |
| `Attachments` | private | 图片 bytes/path、共享格式识别/data URL helper 与原子占位符绑定、删除后重新编号 | 不解码或缩放图片、不替代 Core 权威校验、不直接读取系统 clipboard、不发 RPC、不渲染 |
| `host::clipboard::read_image` | crate-private | 从本机 clipboard 文件列表/RGBA image 读取并统一编码 PNG | 不改变 `ChatInput`、不发 RPC、不持久化临时文件 |
| `host::clipboard::write_text` / `host::transcript_export::write` | crate-private | response/屏幕选区文字写入系统剪贴板，以及目录边界内的 Markdown export | 不拥有 transcript 或屏幕选区状态、不覆盖文件 |
| `FileSearchManager` | crate-private | event loop 持有的目录搜索 runtime；非阻塞 drain snapshot 并丢弃旧 query 结果 | 不进入 `App` state、不解析输入、不保存 popup state |
| `Mentions` / `MentionPopup` | private | `@token` query/range、File/Plugin 混合结果、选择/关闭和原子补全 | 不扫描目录、不读 Plugin 文件系统、不拥有 worker |
| `SkillCompletionState` | private | `$token` query/range、metadata 过滤、选择/关闭、原子 `$name` 与 exact `SkillRef` 绑定 | 不读取 Skill filesystem、不加载 `SKILL.md`、不占用 `/` 或 `@` |
| `PendingPastes` | private | 超过 1000 字符的 text-paste payload、唯一占位符与提交时展开 | 不识别或保存图片，不解释 slash、不渲染、不直接提交 |
| `zeta_slash_commands::SlashCommandsState` | shared public type | 拥有 cursor query、matches、selection、dismissal 与 completion | TUI 不保存第二份 Slash query/selection authority；可见范围与滚动仍由 Ratatui renderer 负责 |
| `zeta_slash_commands::{SlashCommandInput,SlashCommandCatalog}` | shared public types | 统一输入 grammar，并合并 built-in 与 server metadata | TUI 不重新校验名称、不执行 App Server operation |
| `SlashCommandInvocation` | crate-private | command identity、trimmed display arguments 与有序 text/image argument items | 不执行 RPC |
| `features::sessions::ActiveConversation` | crate-private | 当前 `session_id`、选中 Thread identity、Thread sequence 与 typed create/fork/resume/rewind/archive lifecycle | 不把 Session 变成独立事件聚合，不解析 `ChatInput` 文本，不拥有批准策略或 App Server |
| `TextArea` | private | UTF-8 多行 buffer、byte-safe line/cursor movement、原子元素 insert/delete | 不保存 paste payload，不解释 Enter submission、slash command 或应用级导航；Vim 状态由相邻 `vim.rs` 拥有 |
| `features::thread::{submit_prompt,steer_prompt}` | private | 从显式 `ThreadRequestScope` 构造 typed `StartTurn` 或 `SteerTurn` 请求并返回 typed result | 不引用或更新 `App`、不手写 method string/JSON |
| `Steer` | private | 按稳定本地身份跟踪尚未收到服务端确认的 Steer | 不创建独立布局区域；消息和交付结果由 Thread 流程维护 |
| `Queue` | private | 保存本地未发送的完整 ChatInput 草稿、稳定身份和 queued/sending 状态；支持 `↑` 取回及 FIFO 自动发送 | 不提前创建 canonical Turn、不在请求拒绝时丢弃草稿、不跨 Thread 搬运条目 |
| `App::approval_mode_status` | crate-private | 保存 TUI 下一次 Turn 要提交的模式与 active Turn 的冻结模式，供 StatusLine 展示 | 不把模式写进 Session，不判断或绕过批准策略 |
| `app::request_completion::apply_thread_snapshot` | private | 安装 canonical snapshot、恢复最早 nonterminal Turn 作为执行队首并协调 presentation mapping | 不 drain notification；snapshot 是 authoritative UI source |
| `features::thread::interrupt_turn` | private | 从显式 scope 执行 typed Turn interrupt 并返回结果 | 不引用或更新 `App` |
| `app::apply_active_turn_snapshot` | test-visible | canonical Turn presentation outcome → `AppEvent` | 不从 log/text 猜 terminal state |
| `present_turn_error` | private | stable Turn error code → user-facing recovery message | 不显示 Rust Debug/provider secret |
| `client::new_command_id` | private | process ID + wall-clock nanos 分配 `CommandId` | 一次逻辑 command 一个新 ID |
| `app::frame::draw` | crate-private | frame 分区并协调 feature/component renderer，最后把当前屏幕选区样式应用到完整 buffer | 不改变 App state、不写剪贴板 |
| `app::frame::input_pointer_target_at` | crate-private | 复用当前 Pane、ChatInput completion、Approval、Query 与 ChatInput 区域映射可见行点击 | 不执行命令、不改变选择状态 |
| `TerminalSession::open` | crate-private | 进入 raw/alternate/paste mode、创建 backend，并保存最后完成的 Ratatui buffer 供松手复制读取 | partial failure 必须 rollback；鼠标捕获仍由 `set_mouse_mode` 决定 |
| `ScreenSelection` / `ScreenSelectionRange` | crate-private | 在 `App` 中拥有左键拖动与连续点击生命周期、屏幕字符范围，并对完整 frame 绘制反色选区 | 不写剪贴板、不解释页面内容 |
| `screen_selection::token_range_at` / `line_range_at` / `text_in_range` | crate-private | 从最后完成的 Ratatui buffer 计算词、空白、符号或可视行范围，并提取宽字符安全的文字 | 不保存手势状态、不读取正文模型或滚出屏幕的内容 |
| `MouseMode` | crate-private | `App` 按本地 Mouse interactions 设置声明 `TerminalSelection` 或 `TuiCapture` | 不由局部页面决定、不执行终端副作用 |
| `TerminalSession::set_mouse_mode` / `selected_text` | crate-private | 切换全屏鼠标捕获，并从最后完成的 frame 读取选区字符 | 不保存手势状态、不解释页面或产品动作；mode 切换保持幂等 |
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
├─ client.create_session → Session view + root Thread
├─ client.subscribe_session_thread → ThreadSubscription + initial canonical snapshot
├─ TerminalSession::open
├─ app::EventPump::start → TerminalEventSource + ClientEventSource + TerminationSource
├─ FileSearchManager::new
├─ App::for_dir → WelcomeModel::for_dir + StatusLineModel::new
├─ client.read_config → `[tui]` terminal/keybindings/statusLine parse + preferred model → AppEvent → App::update
├─ client.git_status → AppEvent → App::update
└─ loop
   ├─ EventPump::recv / recv_timeout(redraw deadline)
   │  ├─ terminal event → input routing
   │  ├─ App Server event → typed notification mapping
   │  └─ termination request → orderly TUI exit
   ├─ App::mention_query → FileSearchManager::{update_query,stop}
   ├─ FileSearchManager::poll → AppEvent::FileSearchSnapshotReceived → App::update
   ├─ RequestTask::poll → request completion → app::request_completion → App::update
   ├─ skills changed → queued background skills refresh
   ├─ newer active Thread durable update → queued session/thread/read snapshot resync
   ├─ transient update → cursor validation → bounded Thread projection
   ├─ state/resource change → RedrawScheduler::request
   ├─ frame deadline due → App::mouse_mode → TerminalSession::set_mouse_mode
   ├─ frame deadline due → TerminalSession::draw → app::frame::draw
   └─ terminal event
      ├─ key → App::handle_key
      │  ├─ Approval → feature-owned selection / response
      │  ├─ Query → feature-owned selection / custom editor / response
      │  ├─ selected TranscriptCell → select / expand / QuickView
      │  ├─ local input → ChatComposer
      │  │  ├─ top Pane → ListSelection / DetailList / TextPrompt / KeyCapture
      │  │  └─ ChatInput → completion 优先，再进入 Vim/普通编辑或提交
      │  ├─ ReadClipboardImage → clipboard::read_image → AppEvent → App::update
      │  ├─ Quit → return
      │  ├─ SubmitTurn → RequestTask(submit_prompt + canonical read)
      │  ├─ SubmitQueuedTurn → 保留 Queue 条目 → RequestTask(submit_prompt + canonical read)
      │  ├─ SteerTurn → RequestTask(steer_prompt + canonical read)
      │  └─ Interrupt → RequestTask(interrupt_turn + canonical read)
      ├─ left mouse down → `ScreenSelection::begin`
      ├─ left mouse drag → 更新全屏字符选区 → 下一 frame 统一反色绘制
      ├─ left mouse up
      │  ├─ 已拖动 → 从最后完成的 frame 提取字符 → `clipboard::write_text`
      │  └─ 未拖动 → frame 共享几何命中 → Pane / Approval / Query / Transcript / ChatInput completion 原有动作
      ├─ mouse moved → same hit testing → existing selected item
      └─ Paste → App::handle_paste
         ├─ Approval → consumed without changing hidden draft
         ├─ Query → feature-owned custom answer
         ├─ otherwise → ChatComposer
         ├─ ListSelection / TextPrompt Pane → page-local input
         └─ ChatInput
            ├─ image path → Attachments + TextArea atomic placeholder
            └─ text → PendingPastes + TextArea
```

Session create 和 Thread branch mutation 使用独立 `CommandId`。Turn start/interrupt 使用当前 `thread_sequence` 作为 expected sequence；下一次批准模式由 TUI 放进 `StartTurn`，不修改 Session。client error 会进入 visible error message/status，不退出 terminal session。

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
`ChatInput::submit` 在 trim、slash recognition 和 user-message recording 之前展开仍然存在的
占位符。

图片 paste 先尝试把完整字符串解释为本地文件路径；支持引号包裹和 shell 风格反斜杠转义。
`Attachments` 通过 `zeta-utils-image` 的共享签名识别与 data URL helper 处理
PNG/JPEG/GIF/WEBP，拒绝超过 16 MiB 的文件，并在草稿期编码为 base64 data URL，避免提交时路径
失效；提交时 `RequestTask` 使用 bounded chunk RPC 上传，真正的解码、资源限制、规范化和 durable
identity 由共享 backend 接受边界执行。占位符绑定到稳定的
`TextElementId`，光标移动和删除保持原子性，删除后剩余图片会重新编号。提交时
`ChatInput` 按草稿顺序生成 text/image items；
展示记录保留 `[Image #N]`，App Server/Core 持久化 attachment digest、格式、长度和尺寸，而不是
本地路径或 data URL。

`Ctrl-V` 是独立的 clipboard-image intent，不依赖 terminal `Event::Paste` 是否能携带位图。
adapter 优先读取 clipboard file list 中可解码的图片，否则读取 RGBA image data，并统一编码为
PNG bytes；`App` 再把 bytes 交给 `Attachments`，因此系统剪贴板和本地路径共享大小校验、
占位符绑定、删除和提交语义。active Turn 期间同样可把图片加入 follow-up draft。

data URL 不进入 command receipt、durable Thread history 或 snapshot。当前 16 MiB 单图上限、192
KiB upload chunk、connection-owned upload session 与共享缓冲上限共同构成 RPC 保护边界；TUI
不能建立私有附件 authority，也不能绕过共享远程 URL 安全导入。

`ChatInput` 内的 `Mentions` 解析光标下 whitespace-delimited `@token`，先从 `plugin/list` 的 effective package catalog 生成 Plugin 候选，event loop 再从 `App` 读取当前
query 并同步给 `FileSearchManager` 生成 File 候选。manager 为 active token 保持一个 `PathSearchHandle`，后台 walker 使用
Git 作用域内的 ignore 语义、不跟随 symlink，并跳过 `.git`、`.zeta`、`node_modules` 与
`target`。完整 `nucleo` engine 在独立 matcher worker 中增量 reparse query；event loop 每轮通过
`FileSearchManager::poll` 非阻塞收取 snapshot，再以 `AppEvent` 投递给 `App::update`。manager
同时校验 query revision 与文本，
popup 再校验 query，因此包括 A → B → A 在内的旧结果都不会覆盖新输入。结果按 `nucleo` 分数
降序、路径升序稳定打破平局，最多保留 50 项，字符索引交给 renderer 高亮。

补全只替换当前 `@token`，不会把 email 中的 `@` 当作 mention；File 与 Plugin 选择结果都作为 `TextArea` 原子元素
插入，但提交时仍属于普通 Text item。关闭 token 会 drop handle；裸 `@` 会立即显示 Plugin catalog，也会启动空 pattern
文件搜索并随着 walker 发现文件逐步更新候选。

`zeta_slash_commands::SlashCommandInput::at_cursor` 只在光标位于第一行 `/name` token 内时提供
popup query；补全返回 `SlashCommandCompletion { range, replacement }`，因此
`/mod provider/model` 可变成
`/model provider/model` 而不会
清空后缀、图片或 paste bindings。完成且后接 whitespace 的命令名会被标记为 `TextArea`
原子元素；移除 separator 后会解除标记，从而允许重新编辑。

提交路径先生成完整 `ChatSubmission`，再由共享 `SlashCommandInput::for_submission` 使用
同一个 `SlashCommandCatalog` 识别命令。支持 inline arguments 的命令会生成
`SlashCommandInvocation`：display arguments 已 trim，structured arguments 保持原有
`ChatInputItem::Text` / `ChatInputItem::Image` 顺序。未知命令以及不支持参数却带参数的命令仍是
普通 prompt。Catalog 可以合并已校验的 dynamic metadata，并拒绝非法名称、空描述和 built-in
冲突；App Server 在 initialize snapshot 中提供 host-composed dynamic command source。

需要服务端数据或修改请求的内置命令进入 `app::dispatch::execute_product_command`：dispatcher 从克隆的 `ActiveConversation` 调用类型化的 Session/Thread API，只返回 `ProductCommandOutput`；主循环不在 dispatcher 内等待 RPC。查询命令读取权威配置，`/model` 通过预期版本修改 preferred model。`/help` 由 `App` 直接从当前合并命令目录构造，`/help` 和 `/skills` 复用 `ListSelection` Pane；关闭它们后一直保留的 `ChatInput` 重新获得焦点。`/skills` 映射 App Server 的不可变目录快照；Manage
tab 的 `Enter` 产生 source-qualified `SkillId` enablement intent，成功写入 config 后重新读取页面。
catalog/file watcher 变化通过 `skills/changed` 同时刷新管理页面和 `$` Skill 候选。TUI 不读取
Skill filesystem；`$name` 只提交 typed `SkillRef`，正文 activation/context injection 仍由 App
Server 与 Core 拥有。没有对应 typed contract 的产品命令不进入
registry，不显示占位提示，也不转成普通 prompt 冒充成功。

## 快照→ UI 映射

`apply_active_turn_snapshot` 观察当前 canonical 执行队首；如果其他客户端已经创建后续 Turn，
队首进入 terminal state 后继续选择最早的 non-terminal Turn。TUI 自己的本地 Queue 在当前
执行队首 terminal 后才提交，因此不会提前出现在 canonical Turn 列表中：

| Canonical `TurnStatus` | UI effect |
| --- | --- |
| `Created` | `Status::Working`；Enter 只保存本地 Queue，不发送 Steer 或创建后续 Turn |
| `Running` | `Status::Working`；Enter 按 Follow-up messages 设置进入 Queue 或立即 Steer，默认 Queue |
| `WaitingForApproval` | waiting status；owner-directed approval Pane；仍可 interrupt |
| `WaitingForUserInput` | waiting status；owner-directed multi-question Pane；仍可 interrupt |
| `WaitingForCapability` | waiting status；当前不能 resolve |
| `Cancelling` | `Status::Cancelling`，抑制重复 interrupt |
| `Completed` | canonical transcript 已包含该 Turn 所有 Item；返回 Ready |
| `Failed` | 显示 stable Turn error，清除 active turn |
| `Interrupted` | 添加 notice，返回 Ready |

Completed Turn 没有 Agent message 会被显示为 error。已知 stable Turn error（包括 interaction
deadline）由
`present_turn_error` 映射成面向用户的恢复提示，错误详情只在 transcript 出现一次；StatusLine 只说明
可以 retry 或退出。`features/thread/presentation.rs` 按 canonical Item 顺序映射所有 user/agent、
reasoning、plan 与 Tool row；它不从展示内容判断 Turn terminal state。

`client::map_event` 保留 typed `ThreadUpdateEnvelope`；`ThreadSubscription` 验证 active
Session/Thread identity，Thread durable sequence 新于最后确认 snapshot 时触发一次后台
`session/thread/read`。transient cursor 在每个 stream instance 内必须连续；duplicate 被忽略，
gap/runtime switch 会移除旧 transient row 并 resync。canonical snapshot 替换全部 projection，
transient 永远不决定 completed/failed/interrupted。

## 键盘状态机

下列根级组合由 `keymap.rs` 的单一静态声明注册到共享 `zeta-keybinding` Resolver，并由同一声明生成 `/shortcuts` 的可配置项。运行时结构叫 `AppKeymap`：多段 Chord prefix 在 component 前匹配，普通单键仍先经过当前 interaction/component，只有未消费事件进入应用级 fallback。组合精确匹配修饰键，因此 `Ctrl-Shift-V` 不会触发只声明为 `Ctrl-V` 的动作。

`AppKeymap` 支持一至四段 Chord，pending 后用一行 KeyHints 显示已输入前缀和 Esc cancel；1 秒超时、上下文变化、Esc 或 blocker 会清空 pending，错误后续键清空 pending 后继续作为普通输入透传。当前内建表仍只声明单段组合。`Esc Esc` rewind 是独立的根级状态，不属于通用 Chord，因此 Esc 可无歧义地取消 pending。

用户配置不是 `GlobalKeymap`。它以 `BindingSource::User` 合并进同一个 `AppKeymap`；省略 `when` 只表示该规则在 Zeta Code 的所有上下文中适用。`/shortcuts` 打开 Keymap 设置界面，以“快捷键、职责、default/user 来源”三列展示应用级绑定和少量固定操作键，内部 command ID 不进入界面；通用方向键由各 component 拥有，不作为快捷键条目；选择可配置 action 后可替换该 action 的 User 项、追加单键或两段 Chord、清除 User 项，但不会移除 default 键位或 `block = true` 规则。直接编辑 `config.toml` 仍支持一至四段 Chord、平台覆盖、`when` 和 blocker。保存先检查界面打开时的配置 revision，再完整编译临时规则并更新 `[tui]` 与运行时映射；失败不改变当前映射。完整契约见 [`docs/keybindings.md`](../../docs/keybindings.md)。

```text
Ready / Error
├─ Enter(non-empty) → Submit → Working
├─ Shift-Tab → cycle next-Turn approval mode
├─ Shift/Alt-Enter 或 Ctrl-J → insert newline
├─ Enter(/quit) → Quit
├─ Enter(其他 built-in command) → structured invocation → typed command dispatcher
├─ Enter(server dynamic command) → preserve /name + ordered arguments → Submit
├─ /query → cursor-aware popup；↑/↓ select；Tab range completion；Esc dismiss
├─ $query → Skill popup；↑/↓ select；Tab/Enter exact binding；Esc dismiss
├─ @query → File/Plugin completion；↑/↓ select；Tab/Enter complete；Esc dismiss
├─ popup 可见行左键单击 → 补全 Skill/mention 或执行 slash command
├─ Esc / Ctrl-C / empty Ctrl-D → Quit
└─ typing/paste/cursor movement/editing accepted

Working / Waiting*
├─ Esc / Ctrl-C / empty Ctrl-D → Interrupt → Cancelling
├─ Working: Shift-Tab → cycle mode for later submissions
├─ Running + Queue mode: typing/paste/editing accepted；Enter → local Queue（默认）
├─ Running + Steer mode: Enter → steer active Turn immediately
├─ Created: Enter → local Queue，不提前创建下一 Turn
├─ Queue + empty input: ↑ → move latest complete draft back to ChatInput
├─ Running + Steer mode + Queue + empty input: Enter → steer latest queued message now
├─ restored Queue draft: Enter → follow current Queue/Steer mode
├─ active Turn terminal: FIFO send first queued draft；server reject → keep it editable
└─ Waiting*: owner-directed interaction Pane owns input until resolved/interrupted

Cancelling
└─ further quit/interrupt keys ignored until snapshot terminal state
```

Running 状态下 ChatInput completion 可见时，Tab 仍完成 Slash、Mention 或 Skill 候选；候选关闭后 Tab 不提交消息。当前 Turn 的 Skill 已在开始时冻结，因此包含 `$skill` 绑定的草稿不能通过 Enter Steer，界面会保留草稿并提示切到 Queue 模式排队。

Queue 保存 `TextArea`、附件、长粘贴占位绑定和 exact `SkillRef`，并由稳定 `QueueId` 标识。普通 Up 只进入 ChatInput 历史，不再拦截 Queue；`/queue` Pane 使用 `r` 恢复所选项、`d` 删除、Alt-Up/Down 调序、Ctrl-Enter 立即发送，Enter 打开完整内容 QuickView。恢复不会覆盖非空草稿；自动发送期间条目显示为 sending，服务端接受后才移除，请求拒绝后恢复为 queued。

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

首帧前由 `App::mouse_mode` 决定是否另行执行 `EnableMouseCapture`。Config 中的 Mouse interactions 开启时，该方法在任何页面都返回 `TuiCapture`，让最外层事件循环区分拖动选择与单击；关闭时始终返回 `TerminalSelection`，TUI 不接收鼠标事件。

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

Session 页面固定按 `Transcript → Goal → Plan → Queue → Query → ChatInput/Approval → StatusLine/KeyHints → 空行 → SubagentPane` 排列。Goal 与 Plan 各最多一行，Queue 默认最多三行，Query 最多一行；Approval 替换 ChatInput 区域，StatusLine 最多两行，KeyHints 固定一行，SubagentPane 最多四行。StatusLine/KeyHints 与存在内容的 SubagentPane 之间固定保留一行；几何由 `app/screen_layout.rs` 统一分配并优先保留正文。Manager 页面按 `Welcome → 分组 Session rows → ChatInput → KeyHints` 排列，并至少为列表保留四行。

QuickView 和 ChatInput completion 覆盖现有内容；stacked Pane 打开时保留 ChatInput 状态但接管其全部可见高度，Pane KeyHints 在底栏替换 StatusLine。Pane 标题挂在顶部分隔线上并使用当前高亮色底、白色字；列表第一个两字符状态列与 ChatInput 正文起点对齐。每个 Thread 的草稿、补全状态、Queue、Plan 展示、正文滚动、稳定选择和展开集合按 `ThreadId` 独立保存；正文选择、展开集合和滚动锚点都使用 `TranscriptCellId`，不依赖绘制后的行号。

正文由有序 `TranscriptCell` 构成，live/final 生命周期不改变单元种类。单条正文单元从 canonical entry identity 确定 `TranscriptCellId`，ExecCell 从分组中的首个 `ToolCallId` 确定，后续分组增长不改身份。ExecCell 按 `ToolCallId`
接收调用、输出和结果，命令输出按 byte、行数和单行长度有界保留；折叠态、展开态与 QuickView
详情都读取同一单元数据。QuickView 的“完整”指 TUI 可获得的完整保留表示，上游省略标记不会被隐藏。Space 切换展开，Enter 打开可滚动详情，鼠标只响应明确的展开或详情标记。
PageUp/PageDown 按五行移动，Ctrl-Home/End 到首尾；新提交默认恢复 follow-latest。`estimated_wrapped_rows` 使用
`unicode_width::UnicodeWidthStr`，把 label width 计入首行，然后计算 bottom scroll。它是估算，
不处理完整 grapheme/reflow/Markdown layout。

`SessionThread` 提供标题和创建时间；SubagentPane 将 Main 与子代理名称统一为小写，以实心圆表示当前项、空心圆表示其他项，在右侧按 Codex 状态时长格式展示从 Thread 创建至今的时间，并隐藏 Thread ID。`SessionManagerInfo` 由 App Server 从完整 Thread snapshot 推导明确的 Idle、Needs input、Working、Ready for review、Completed、Failed、Stopped 状态、状态变更时间与当前操作/问题/失败；TUI 不从正文猜状态。Manager 的 Working 图标只由 Tick 推进动画；Completed 显示完成至今的 `… ago`，其他行显示进入当前状态后的时长。`summary` 目前保持空值，等独立配置的摘要模型与请求生命周期存在后再填充，不会暗用当前聊天模型。协议也没有为每次工具执行提供可持久恢复的最终时长与退出码；ExecCell 不会从输出文字猜这些字段。

v15 接受 `inline_visualization` 的终端 fallback，但当前 protocol 没有 visualization artifact、结构化 fallback 或安全引用。TUI 因此不会解析任意 HTML；上游契约到位后，它进入普通 TranscriptCell/Expansion/QuickView 路径，而不是新增一套覆盖层。

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
| Incremental notifications | `features/thread` sequence/cursor state、gap/resync、client notification source、app event pump、snapshot fallback |
| `ConfigReadResult` 新字段 | `test_support::empty_config_snapshot` 与真实消费该字段的 feature tests；无关 view tests 不复制完整 aggregate |
| `ThreadItem` variant 字段 | 构造该 variant 的 presentation tests 必须显式更新，不能由通用 fixture 隐藏领域不变量 |

## 测试与支持边界

```text
just test zeta-tui
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
response lifecycle/error/interrupted transitions、Pane 打开期间的聊天草稿保留、tab list 换行/左右循环切换、
approval 与多问题 option/free-form user input、blocked Esc/Ctrl-C semantics、搜索过滤/选择修复、
selection render、全屏拖拽选择、松手复制、反向范围、宽字符与点击/拖动分流，以及 snapshot
terminal/wait/resume mapping，以及 transcript chrome、error 去重、role
label/Unicode/zero-width wrapping、bounded scroll/history window、copy/export，以及 status-line item 顺序/开关、config 保存、Git 长短值降级、Unicode-safe truncation、welcome home-relative 路径，以及 terminal mode acquisition failure、逆序 rollback、suspend/reacquire 与幂等
restore；还覆盖 request task 非阻塞 completion、request intent 保序、Session picker/archive 与 Thread recovery、
directory directory/preview 和 interaction deadline。

跨 feature 复用的完整配置快照只由 `test_support::empty_config_snapshot` 构造；各测试随后只修改
自己拥有的字段。该 helper 仅存在于 `cfg(test)`，不会给生产协议增加 `Default`，也不会让真实
`ConfigReadResult` 新字段在 App Server 或消费该字段的 feature 中被静默忽略。相反，直接构造
`ThreadItem` variant 的测试必须明确填写其全部字段，因为这些字段属于被测试对象本身的领域语义。

生产路径同样按能力收窄：`config/read` 的完整聚合只停留在 request adapter；各 feature 只解释自己拥有的字段。`provider/list` 只投影供应商名、API key 策略与是否已配置，不返回密钥。Model Pane 只接收 preferred model，MCP Pane 只接收 server map，status line 通过 `AppEvent::PreferredModelReceived` 与 `AppEvent::GitStatusReceived` 接收展示数据，通过 `StatusLineSettings` 接收 `[tui].statusLine` 项目。新增 Tool Search 或 Codebase 配置字段不会扩散到这些不拥有该能力的展示组件。

Render tests 使用 Ratatui `TestBackend` 固定 empty/error surface，并覆盖 transcript 折行高度、prefix、scroll、pointer hit test、cell revision、cache key 失效与资源上限、完整源码和逐个完整新增行高亮的一致性，以及 batch deadline 不后移、输入提前 deadline、到期帧只消费一次、transient latest-value、identity 顺序、cursor/scope/barrier 和批量容量边界。命令行状态测试是通过依据，没有截图/像素基线。完整 fake-transport `run`
event-loop integration 可以继续加强当前 brokered-local 路径；连接恢复验证属于 CLI，不进入 TUI transport 测试。桌面端 Markdown/diff/table 和完整 pointer parity 都不是当前 TUI 验收项；屏幕框选只复制当前 Ratatui frame 的可见字符，不把 Markdown 结构或滚出屏幕的内容伪装成语义选区。产品要求与
owner 判断以 [`docs/tui.md`](../../docs/tui.md#17-已接受的架构迁移顺序) 和
[`docs/product-lines.md`](../../docs/product-lines.md) 为准。

渲染测试遵守以下边界；这里的测试快照只指渲染基线，与运行时的权威 Thread 快照无关：

- 默认断言状态、事件、命令、语义身份以及关键文字、颜色、焦点和坐标；完整画面不能代替这些行为依据。
- 当前不引入快照测试依赖。只有完整字符布局本身是稳定契约、逐格断言会遮蔽测试意图时，才在拥有该布局的组件测试中加入文本快照，并只固定代表性宽度以及可复现的时间、路径、版本和 ID。
- 不为整个应用帧建立快照，也不使用终端截图或像素基线；快照变更必须逐项审查，不能批量接受后跳过行为核对。
