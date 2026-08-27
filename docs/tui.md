# `zeta code` TUI 架构与产品支持边界

> 修改 `zeta-code` 时同时遵守 scoped [`tui.instructions.md`](../.github/instructions/tui.instructions.md)；该 instruction 只保留任务期规则，完整产品架构和当前状态仍由本文拥有。

> 物理位置：`zeta-code/tui/`；ANSI/Ratatui adapter：`zeta-code/ansi-escape/`
> 宿主：`zeta-code/cli/`
> 文档所有权：本文是 TUI 跨 crate ownership、长期不变量、产品支持边界与已接受架构迁移顺序的 canonical 文档。
> 当前实现接口与事件循环：[`zeta-code/tui/README.md`](../zeta-code/tui/README.md)
> 产品接口基线：[`zeta-app-server-api.md`](zeta-app-server-api.md)  
> App Server 启动与连接基线：[`app-server-client.md`](app-server-client.md)  
> Workspace 边界基线：[`zeta-rs-architecture.md`](zeta-rs-architecture.md)
> 产品线归属基线：[`product-lines.md`](product-lines.md)

## 快速理解

`zeta code` 的 TUI 把 App Server 的权威状态转换成终端中的可交互呈现；它拥有单写者的界面状态和事件循环，
不拥有产品事实或后台执行。

| 发生的事情 | TUI 负责 | 不负责 |
| --- | --- | --- |
| 用户按键或提交命令 | 转成类型化界面命令并发给客户端 | 直接修改 Session 或 Thread |
| 服务端产生更新 | 归约为单写者 presentation state 并请求重绘 | 重新解释领域事件 |
| 高频流式文本到达 | 通过专用临时数据面更新显示 | 把每个片段伪装成持久事实 |
| 检测到序列缺口 | 暂停推断并请求权威快照 | 用本地状态填补缺口 |
| 请求完成、过期或取消 | 按关联身份丢弃过期结果并更新交互 | 决定工具重试或 Agent 恢复 |
| 渲染一帧 | 纯读取当前呈现状态 | 在绘制过程中触发业务副作用 |
| 加载主题 | 消费共享 snapshot 的明确子集并按终端能力降级 | 复制 Desktop 的全部主题能力或默认色目录 |
| 外部 Agent 来源已经进入统一 Skill catalog | 浏览、启用或禁用已有条目 | 导入外部 Agent 配置、选择目录或扫描用户主目录 |

## 1. 结论

TUI 正式采用以下长期稳态架构基准：

> **窄而稳定的终端基础设施 + 单写者的 presentation state + 分域 event/command +
> 垂直功能切片 + 高频流式事件专用数据面 + 无业务副作用的渲染。**

这套基准约束新功能和迁移方案，但不是对 Codex TUI 目录、feature catalog 或领域状态的复制。
Zeta 的产品 authority、typed contract 和 crate dependency direction 仍由本仓库现有架构决定。

状态必须按下表阅读：

| 内容 | 状态 | Canonical owner |
| --- | --- | --- |
| 当前 wakeable event loop、typed Thread subscription、snapshot/transient resync 与交互 | Current | [`zeta-code/tui/README.md`](../zeta-code/tui/README.md) |
| 本文的 ownership 与长期不变量 | Accepted architecture baseline | 本文 |
| independent request driver、wakeable event pump | Current | [`app-server-client.md`](app-server-client.md) 与 crate README |
| 非阻塞 request completion dispatch、有界 transient data plane、Session/Thread/interaction 垂直切片 | Current | [`zeta-code/tui/README.md`](../zeta-code/tui/README.md) |
| plain-text transcript、必要 popup mouse hit 与 brokered local session | Current product support boundary | 本文与 crate README |
| active-Turn follow-up queue、多行 composer、copy/export、分页历史与 suspend/resume | Current | [`zeta-code/tui/README.md`](../zeta-code/tui/README.md) |
| 尚无 `zeta code` 产品要求或 canonical contract 的 feature | 非目标或 Potential；不构成实现承诺 | 对应产品线、领域与 App Server API 文档 |

本文只为已经接受的 `zeta code` 能力规定架构与迁移顺序。“某能力在 TUI 中不存在”不自动产生
产品 backlog。Native Agent Timeline 的 Markdown、table、selection、折叠与虚拟化由
[`app/docs/native-agent-console.md`](../app/docs/native-agent-console.md) 规划，具体 Native
Markdown 组件由 [`app/markdown`](../app/markdown/README.md) 拥有；TUI 不追求与其 feature
parity。Vim、remote selector/reconnect、通用 structured Mention 和 durable blob 只有在产品文档
接受需求并确定 canonical owner 后，才可能成为某条产品线的实施项。

Zeta 已经在 TUI 外部拥有：

- `zeta-core` 中权威的 `SessionCoordinator` 和 `ThreadController`；
- `zeta-app-server-protocol` 中唯一的 wire contract；
- `zeta-app-server-client` 中共享的 App Server 启动、初始化、请求/事件连接与关闭层；
- CLI 交付的启动配置与产品入口参数。
- `zeta-theme` 中与 Desktop/Native 共享的 manifest、用户主题解析和 device preference loader。
- `zeta-ansi-escape` 中独立的 ANSI SGR → Ratatui presentation adapter；它不拥有 PTY/terminal state。

主题边界是“部分接入”而不是“尚未复制完成”：TUI chrome 读取 accent、composer chrome、错误、
成功、警告、弱化文字和选择高亮；Theme Pane preview 额外读取有限的 syntax/diff token。选择高亮由
`tui.highlightForeground` 独立表达，不借用编辑器关键字色。`ui/theme.rs` 将透明色先合成到 terminal
background，再按 TrueColor、ANSI-256、ANSI-16、Monochrome 投影；其他 Desktop/Native token
不进入 TUI API。
主题选择保存在共享 profile `configuration.json` 的 `tui.colorTheme`，由 `zeta-theme` 严格读取并
原子写回；它不进入 App Server Config API。无参数 `/theme` 打开由 `features/theme` 拥有、不可搜索的固定
Zeta Code Theme Pane 以 `Theme` 为标题，顶部分隔线与标题、标题与第一个候选项之间各保留一行；固定选项为 Auto、Dark/Light、对应 colorblind-friendly 与 ANSI-only 模式，以及 Custom
color theme。候选行编号展示，cursor 选择色和 syntax/diff preview 随候选主题变化；Enter 原子保存、
即时切换并关闭整个 Theme flow 返回主界面；保存成功后 transcript 以独立的状态圆点显示实际执行的 `/theme <id>`，下一行通过 `└─` 结构连接符归属结果说明，两行正文保持同列对齐；保存失败时则保留当前 Pane 以显示错误。移动 cursor 时，Theme Pane 分隔线、上方 welcome banner
框线使用候选 highlight；独立 `Diff preview` 区域不画左右边框，只用候选 muted token 绘制上下
较高对比度的长节虚线。主题列表与 preview 间保留两行，palette 来源说明与操作提示间保留一行。preview 下方标明
GitHub、GitHub Colorblind、ANSI 16 colors 或 User-defined 配色来源。`/theme <id>` 保留直接切换。通用 Selection Pane 的搜索是独立、
可配置的底座；启用搜索的 feature 必须先按 Space 进入 search mode，footer 明示 `Space search`。
所有本地 command 都可使用同一个“命令 + 结果” transcript 形式：命令本身不带箭头符号，独立的状态圆点为 Running 显示 `◉`，为 Succeeded 显示 `●`；结果行使用与状态位结构相连的 `└─` 表达归属，正文与命令文字同列对齐并使用弱化色，之后才空行。当前没有折叠交互；待有多行 command output 时再基于这个分组添加展开/收起。
Auto 在终端 raw mode 建立后、输入事件线程启动前发出一次 OSC 11 背景色查询，并按实际 RGB
亮度选择 Light/Dark；120 ms 内没有有效响应时读取 `COLORFGBG`，仍无法识别才安全回退 Dark。
检测结果在当前 TUI 会话内缓存，主题面板与再次选择 Auto 不会重复查询；显式模式不受该判断影响。

因此 TUI 必须是可丢弃、可重新同步的 presentation shell，而不是第二个 Agent runtime 或
App Server facade。产品权威状态的依赖链固定为：

```text
zeta-cli
  → zeta-tui
    ├─ zeta-ansi-escape → ansi-to-tui / ratatui
    └─ zeta-app-server-client
       → App Server dispatcher
       → zeta-core
```

本地只读能力不必统一绕行 App Server：composer 的 workspace path mention 直接调用
`zeta-file-search`；需要 workspace authority、跨进程一致性或 watcher revision 的目录浏览与 Git
状态通过 typed App Server filesystem/Git contract。原则不是“所有数据经过一个 facade”，而是
“每个 feature 消费事实 owner 已提供的 public typed interface”。

进程内模式只是一种 transport 优化。TUI 仍然经过 initialize、typed request/response、
dispatcher 和 notification decode，不得直接依赖 Core、Storage、Exec、Sandbox 或 Model
Provider。

## 2. 基准采用与 Zeta 修正

长期基准中的通用结构按下表落到 Zeta：

| 基准概念 | Zeta 中的处理 |
| --- | --- |
| `app/` | 保留，但只拥有 TUI 状态、事件协调和退出流程 |
| `app_server/` | 不建立；typed RPC 已由 `zeta-app-server-client` 和 protocol crate 拥有 |
| `thread/` | 作为 `features/thread/` 保留；拥有 active Thread 的可重建展示状态和交互流程 |
| `chat/` | 不建立总括目录；Turn flow 归 `features/thread/`，composer/transcript 归 `components/` |
| `ui/` | 保留，只放可复用 Ratatui 原语 |
| `terminal/` | 保留，只负责真实终端生命周期和能力 |
| `features/` | 保留，但只添加 Zeta 已有或已接受产品契约支持的功能 |
| `platform/` | 不建立泛化 facade；窄 OS adapter 放入 `host/` 的明确子模块 |

尤其不能在 TUI 中定义第二个 `ThreadController`。`zeta-core::ThreadController` 是 Thread 执行、
持久化、顺序和恢复的 authority；TUI 的 Thread 数据只是由 snapshot 和 update 构成的
read model。

同样，TUI 内部不应出现另一个聚合 account、session、thread、turn、config 等 RPC 域的
facade。CLI 使用 `zeta-app-server-client` 建立并初始化 `AppServerSession`，再把该运行会话
的所有权交给 TUI；TUI 围绕其 cloneable request handle 与 event stream 补充交互客户端所需的
请求调度、订阅和错误映射。该 `AppServerSession` 是 connection/runtime owner，不是产品
`Session`。

## 3. 产品状态与 TUI 状态

权威产品模型与 sequence/cursor 语义统一见 [`protocol.md`](protocol.md)。TUI 只消费这些
canonical snapshot/update，不在本地重新定义产品实体。

TUI 可以在使用相应数据的 feature 中保存以下可重建状态：

- 当前选中的 Session、Thread、Turn 或 Item；
- 当前页面需要的 canonical Session/Thread snapshot；
- 每个 aggregate 最后确认的 durable sequence；
- 当前 runtime 的 transient stream cursor；
- composer 草稿、光标、选择区和输入历史；
- scroll、折叠、tab、overlay 和 picker 状态；
- 正在发送的 typed command 及其稳定 `CommandId`；
- connection、subscription、resync 和可展示错误状态。

TUI 不得保存或推导：

- Session membership、fork lineage 或 lifecycle 的第二份权威状态机；
- Thread、Turn、ThreadItem 或 Tool Call 的执行状态机；
- writer lease、command receipt 或持久化恢复状态；
- approval、sandbox 或 tool execution policy；
- 从日志、stderr 或人类文本解析出的产品终态。

命名时必须区分三种生命周期：

- `ProductSession` 或 protocol 中的 `Session`：产品任务；
- `AppServerConnection`：RPC connection；
- `TerminalSession`：raw mode、alternate screen 等终端资源。

禁止用无修饰的本地 `SessionState` 同时表达上述多个概念。

## 长期架构不变量

以下约束是架构评审中的默认拒绝条件，而不是可选风格。若实现确实需要偏离，必须先在本文记录
原因、影响范围、恢复策略与退出条件。

### 单写者

TUI semantic/presentation state 只能由主事件循环写入。后台 task、transport callback、host
adapter 和 renderer 只能产生 event 或完成结果，不能直接修改共享 feature state。

单写者不表示所有工作都在主线程同步执行。RPC、文件搜索、大型 transcript projection 和其他昂贵工作应
异步执行，但其结果必须重新进入有序 event loop，并由当前 state owner 判断是否仍然有效。

### 状态分类

| 类别 | 示例 | Owner 与约束 |
| --- | --- | --- |
| 可重建 presentation state | active Thread snapshot、pending command、connection/resync、可展示错误 | 对应 feature 或 `app/`；可由 typed input 重建 |
| 局部交互状态 | draft、cursor、selection、scroll、overlay 展开 | 明确生命周期的 component；不提升为全局 store |
| runtime resource | terminal handle、channel、task、clock、client、cache | driver/client/terminal；不得进入可 replay state |
| 产品权威状态 | Thread/Turn reducer、writer lease、approval policy、durable recovery | TUI 外的 canonical owner；TUI 不复制 |

### 纯渲染

`view`、layout 和 Ratatui renderer 只能读取 state 与只读环境信息。禁止在 draw path 中：

- 发起 RPC、读取文件或写配置；
- spawn task 或改变 subscription；
- 推进 Turn、animation 或 paste 等语义状态；
- 根据 frame 次数决定业务结果；
- 查询 Git、usage 或其他可能阻塞的接口。

时间驱动变化由明确的 timer event 推进；render cache 属于 runtime resource，并使用稳定
identity、revision、width、theme/capability revision 等显式 key 失效。

### 显式副作用与局部协议

跨边界行为必须先成为 typed command/intent，再由 driver、feature request module 或窄 host
adapter 执行，完成结果重新成为 event。顶层 `AppEvent`/`AppCommand` 只负责领域路由；具体
变体留在 feature 内，不能重新形成覆盖所有 RPC 的大枚举。

不建立高度泛型的 `Feature` trait、全局 `Services` facade 或任意闭包 callback 总线。普通
Rust struct/enum/function 和一致约定是默认方案；只有多个真实调用方证明语义相同时才提取
共享抽象。

### 请求关联与过期结果

任何可能乱序完成的读取或计算必须携带 generation、目标 scope 和必要的 request identity。
写操作还必须遵守 canonical `CommandId` 与 expected sequence 语义。旧 generation、错误 scope
或已取消请求的结果不得覆盖当前 state。

### 控制面、数据面与恢复

用户意图、退出、interrupt、写请求结果、committed update、错误和 subscription lifecycle
属于有序控制面；token delta、process output 和 tool progress 等高频事件进入按 aggregate
隔离的 bounded 数据面。所有 lag、overflow、cursor gap、identity mismatch 和已经实现的
connection recovery 路径必须定义 resync contract，不能只记录 warning 后继续猜测状态。

### 内部优先

目标边界先在 `zeta-tui` crate 内通过 private module 和 `pub(crate)` 验证。只有 runtime 或
component 被至少两个真实消费者使用、API 变化频率下降且抽取能减少依赖时，才评估独立 crate
或公共插件 API。窄的第三方类型适配层可以按依赖隔离提前成为产品内 crate，但必须保持单向、
无产品状态且有明确 failure semantics；`zeta-ansi-escape` 是当前唯一此类例外。

## 4. 目标目录

目标结构如下：

```text
zeta-code/tui/
├── src/
│   ├── app/
│   │   ├── mod.rs
│   │   ├── state.rs
│   │   ├── event.rs
│   │   ├── command.rs
│   │   ├── event_loop.rs
│   │   ├── dispatch.rs
│   │   ├── bootstrap.rs
│   │   └── shutdown.rs
│   ├── client/
│   │   ├── mod.rs
│   │   ├── event_pump.rs
│   │   ├── pending.rs
│   │   ├── subscription.rs
│   │   ├── command_id.rs
│   │   ├── notification.rs
│   │   └── error.rs
│   ├── features/
│   │   ├── mod.rs
│   │   ├── thread/
│   │   │   ├── mod.rs
│   │   │   ├── state.rs
│   │   │   ├── update.rs
│   │   │   ├── command.rs
│   │   │   ├── request.rs
│   │   │   ├── view.rs
│   │   │   └── thread_tests.rs
│   │   ├── sessions/
│   │   │   ├── mod.rs
│   │   │   ├── state.rs
│   │   │   ├── command.rs
│   │   │   ├── request.rs
│   │   │   ├── view.rs
│   │   │   └── sessions_tests.rs
│   │   ├── config/
│   │   │   ├── mod.rs
│   │   │   ├── state.rs
│   │   │   ├── command.rs
│   │   │   ├── request.rs
│   │   │   ├── view.rs
│   │   │   └── config_tests.rs
│   │   ├── skills/
│   │   │   ├── mod.rs
│   │   │   ├── state.rs
│   │   │   ├── command.rs
│   │   │   ├── request.rs
│   │   │   ├── view.rs
│   │   │   └── skills_tests.rs
│   │   ├── workspace_files/
│   │   │   ├── mod.rs
│   │   │   ├── search.rs
│   │   │   ├── completion.rs
│   │   │   └── workspace_files_tests.rs
│   │   └── status_line/
│   │       ├── mod.rs
│   │       ├── model.rs
│   │       ├── refresh.rs
│   │       ├── layout.rs
│   │       ├── view.rs
│   │       └── status_line_tests.rs
│   ├── components/
│   │   ├── mod.rs
│   │   ├── interaction/
│   │   │   ├── mod.rs
│   │   │   ├── state.rs
│   │   │   ├── view_stack.rs
│   │   │   └── interaction_tests.rs
│   │   ├── composer/
│   │   │   ├── mod.rs
│   │   │   ├── state.rs
│   │   │   ├── editor.rs
│   │   │   ├── attachments.rs
│   │   │   ├── pending_pastes.rs
│   │   │   ├── slash_commands.rs    # TUI-local command execution metadata only
│   │   │   ├── view.rs
│   │   │   └── composer_tests.rs
│   │   ├── transcript/
│   │   │   ├── mod.rs
│   │   │   ├── row.rs
│   │   │   ├── layout.rs
│   │   │   ├── view.rs
│   │   │   └── transcript_tests.rs
│   │   └── selection/
│   │       ├── mod.rs
│   │       ├── state.rs
│   │       ├── view.rs
│   │       └── selection_tests.rs
│   ├── ui/
│   │   ├── mod.rs
│   │   ├── layout.rs
│   │   ├── scroll.rs
│   │   ├── overlay.rs
│   │   ├── keymap.rs
│   │   ├── text.rs
│   │   └── theme.rs
│   ├── terminal/
│   │   ├── mod.rs
│   │   ├── session.rs
│   │   ├── event_stream.rs
│   │   ├── frame_scheduler.rs
│   │   ├── scrollback.rs
│   │   ├── reflow.rs
│   │   ├── cursor.rs
│   │   └── capabilities.rs
│   ├── host/
│   │   ├── mod.rs
│   │   ├── clipboard.rs
│   │   ├── external_editor.rs
│   │   ├── notification.rs
│   │   └── ide.rs
│   └── lib.rs
├── tests/
│   ├── event_loop.rs
│   ├── subscription_recovery.rs
│   └── support/
│       ├── mod.rs
│       ├── fake_client.rs
│       └── terminal_harness.rs
├── Cargo.toml
└── README.md
```

上图只列当前已有能力和已确定的下一阶段 owner，不罗列尚无 contract 的未来 feature。每个
子目录都给出首批具体文件；实现初期不足以形成独立 owner 时继续使用单文件，不能只创建一个
空 `mod.rs` 或 slash-only 占位目录。文件名表达职责，不要求一次性生成整棵目录树。

## 5. `app/`：应用协调

`app/` 负责：

- 接收 terminal event、client result、server notification 和 background completion；
- 把输入转换为语义明确的 `AppCommand`；
- 协调 active feature、component、overlay 和请求结果；
- 管理焦点、顶层模式、退出与恢复终端；
- 决定何时请求重绘。

建议把可测试状态与副作用 driver 分开：

```rust
struct TuiApp {
    connection: ConnectionViewState,
    active_view: ActiveView,
    thread: ThreadFeatureState,
    sessions: SessionsFeatureState,
    status_line: StatusLineModel,
    overlays: OverlayStack,
    pending_commands: PendingCommands,
}

struct AppDriver<C> {
    app: TuiApp,
    client: C,
    terminal: TerminalSession,
}
```

`TuiApp` 处理事件并产生 command；`AppDriver` 执行 I/O，再把结果作为事件送回。这样 reducer
式状态测试不需要打开终端或 App Server。

`app/` 可以协调其他模块，但禁止：

- 实现具体 Ratatui widget；
- 拼接 JSON-RPC method string 或手写 wire JSON；
- 执行 Session/Thread 领域 reducer；
- 直接运行 shell、模型或 Agent tool；
- 成为每个 feature 私有状态的杂物桶。

`bootstrap.rs` 只负责 TUI 启动流程，例如打开 terminal、读取入口所需 snapshot、进入指定
Session。CLI 可以构造 start options，但 App Server composition、channel 建立、initialize、
schema gate 和 shutdown 属于 `zeta-app-server-client`，TUI 不复制这些步骤。

## 6. `client/`：TUI 到 typed 客户端的窄适配

`client/` 不是新的 App Server facade。它只负责交互客户端特有的：

- 持有 cloneable `AppServerClient` request handle 和 `AppServerEvents`；
- 驱动 pending typed request completion；
- 持续消费 `AppServerEvents`，将 `ServerNotification` 转换为内部事件；
- 维护 subscription transport lifecycle；
- 为一次逻辑写操作分配并保留稳定 `CommandId`；
- 将 stable server error 映射为可操作的 TUI 错误类别。

RPC Params、Result、Notification 和错误码仍由：

```text
zeta-app-server-protocol
zeta-app-server-client
```

唯一拥有。`client/` 不得复制 DTO、兼容旧 method、直接序列化 JSON-RPC 或暴露
`execute(method: &str, ...)`。

具体领域请求不汇总成 `ClientOperation` 大枚举。`features/thread/request.rs`、
`features/config/request.rs` 等模块直接调用 `AppServerClient` 的 typed method，并把完成结果
转换为自己的 feature event；Session/Thread/Turn mutation 统一使用 `request_session`，
`client/` 只提供执行与投递机制，不复制 App Server 的领域 contract。

一次逻辑写操作在超时或响应丢失后重试时必须复用原 `CommandId` 和 exact typed payload。
用户再次点击或再次提交是新命令，必须生成新 ID。`expectedSequence` 来自目标 aggregate
在对应 feature 中最后确认的 canonical snapshot，不能使用 JSON-RPC request ID、stream
cursor 或另一个 aggregate 的 sequence。

这些 identity 不得互换：

| Identity | 解决的问题 | 生命周期与规则 |
| --- | --- | --- |
| JSON-RPC request ID | 匹配一次 transport request/response | 每次 transport attempt 独立；不表示产品幂等性 |
| `CommandId` | 标识一次逻辑写意图 | exact retry 必须复用；新的用户意图必须新建 |
| expected sequence | 防止基于过期 aggregate state 写入 | 来自目标 aggregate 最后确认的 durable sequence |
| request generation | 丢弃旧搜索、读取或计算结果 | scope 内单调；旧 generation 不改变当前 state |
| durable sequence | 排序 committed aggregate update | 按 Session/Thread 分别连续验证 |
| stream instance/cursor | 排序一次 transient stream | 不能作为 durable completion 或 expected sequence |

pending request 至少记录 result route、scope、generation、cancellation state 和 timeout policy。
取消只表示 TUI 不再接受结果；除非 typed contract 明确支持取消，不能假定远端副作用没有发生。
写请求在结果未知时不得自动生成新 `CommandId` 重放。

共享 client 使用 bounded request/event channel、per-request completion 与独立 wakeable
`AppServerEvents` 替换 TUI 的 `round_trip + drain_notifications` 路径。TUI 的
`client::RequestTask` 执行 typed request，`app::request_completion` 在单写者 loop 校验 scope 并
安装结果；用户 request intent 在单槽执行器前保序排队，Quit 和纯本地操作仍可立即处理。TUI
不能通过直连 Core、读取日志或私有 transport method 绕过。

## 7. `features/thread/`：active Thread 的唯一 TUI 所有者

Zeta 的主要交互对象是 Thread，因此不再建立通用 `projection/` 和 `conversation/` 两层。
`features/thread/` 是 active Thread 在 TUI 内的唯一状态 owner：

| 文件 | 职责 |
| --- | --- |
| `state.rs` | 保存当前 `ThreadId`、canonical snapshot、最后确认 sequence、transient items 和 view-local selection |
| `update.rs` | 校验并应用 typed snapshot/update，处理 committed/transient 替换和 resync intent |
| `command.rs` | 定义 start、interrupt、fork 等用户意图及 pending command 状态 |
| `request.rs` | 直接调用 `AppServerClient` 的 Thread/Turn typed methods |
| `view.rs` | 将 `ThreadFeatureState` 与 transcript/composer component 组装成当前页面 |
| `thread_tests.rs` | 覆盖 sequence、resync、Turn flow 和可见 item 顺序 |

这里保存的是可由 server snapshot 重建的客户端状态，不是第二个 `ThreadController`：

- 输入只允许 typed snapshot/update 和明确的本地 UI intent；
- 不执行业务校验、持久化或 Tool 副作用；
- 不补造缺失 event；
- 不从展示文本推断 Turn/Item 终态；
- 遇到未知 update、sequence 空洞或 identity 不一致时产生 resubscribe/resync intent。

Session 列表和 Session 页面状态归 `features/sessions/`，不会进入一个跨领域
`ProjectionStore`。其他 feature 同样只缓存自己页面真正需要的 typed result。

### 7.1 持久化序列

Session 与 Thread 的 durable sequence 由各自 feature 分别跟踪。Thread 收到 committed update
时：

1. `durableSequence <= localSequence`：作为重复 delivery 忽略；
2. `durableSequence == localSequence + 1`：应用到 active Thread state；
3. `durableSequence > localSequence + 1`：停止增量合并并重新 subscribe；
4. update identity 与当前 aggregate 不一致：视为协议错误，不路由到当前视图。

subscribe result 必须作为一个完整的 resync package 处理：

1. 校验 snapshot identity，以及返回 gap 在 `afterSequence` 之后连续；
2. 直接把 snapshot 安装为当前 canonical 基线；
3. 不把已经包含在 snapshot 中的 gap event 再应用一次；
4. 丢弃已经排队且 sequence 不大于 snapshot sequence 的重复 notification；
5. 从 snapshot sequence 开始继续应用新的连续 live notification。

只有 Session 与 Thread 确实出现相同的连续序列算法后，才提取一个只处理
`aggregate identity + sequence` 的小型 helper；不能重新演变成保存所有领域状态的 store。

### 7.2 临时流游标

transient update 由 `features/thread/update.rs` 处理，只影响低延迟显示：

- cursor 只在同一 `streamInstanceId` 内连续；
- `streamInstanceId` 改变时清空旧 transient buffer；
- cursor 出现空洞时丢弃不可信 transient 内容并重新同步；
- committed Item 到达后替换相同 Item 的 transient 版本；
- transient 文本永远不能决定 Turn completed、failed 或 interrupted。

## 8. `components/`：有局部状态的交互与呈现组件

`components/` 与 `ui/` 的区别是：component 可以拥有局部交互状态，也可以理解少量产品展示
value；但不能调用领域接口或保存 canonical aggregate。

| Component | 拥有 | 不拥有 |
| --- | --- | --- |
| `interaction/` | composer 与 temporary view stack 的焦点、push/pop 和 routing | feature catalog、RPC、Thread lifecycle |
| `composer/` | draft、Unicode cursor、attachments、paste bindings、slash parsing | Turn start、config mutation、App Server client |
| `transcript/` | plain-text visible row、wrapping 与 scroll | canonical Thread snapshot、sequence、transient cursor；不复制 Native Markdown/diff/table 组件 |
| `selection/` | tabs、query、filtered indices、selection 和通用列表渲染 | Session/Skill identity 的业务 action |

`features/thread/update.rs` 完成 committed/transient item 合并并暴露有稳定 identity 的可见
items；`components/transcript/` 只负责把这些 items 布局和渲染。composer 提交时只产生
`ComposerSubmission`，由 `features/thread/command.rs` 转成 Turn intent。这样同一份 Thread
状态不会同时由 feature 和 component 保存。

component 可以依赖 `ui/` 原语和必要的 canonical value type，但禁止：

- 直接调用任何 crate/App Server 领域接口；
- 保存 Session membership、Thread lineage 或 durable sequence；
- 决定 Tool 是否允许执行；
- 根据展示文本判断产品终态；
- 通过 callback/闭包把任意业务副作用藏进通用组件。

## 9. `ui/`：可复用 Ratatui 原语

`ui/` 可以包含：

- layout helper；
- selection list、tabs、picker 和 overlay 容器；
- scroll state；
- key hint 和 keymap 展示；
- theme、color、spacing；
- 通用 loading、error 和 empty-state widget。

`ui/` 不得出现：

- `SessionId`、`ThreadId`、`TurnId`；
- `AppServerClient`、RPC Params 或 `ServerNotification`；
- approval、model、plugin 等产品状态；
- 完整的 Session browser 或 Thread transcript。

通用 tabs、过滤和选择状态放在 `components/selection/`；“恢复哪个 Session”的 row model、
typed ID 和 action 属于 `features/sessions/`。`ui/` 只提供这两个上层模块共同需要的纯布局与
样式函数。

## 10. `terminal/`：真实终端基础设施

宿主终端身份、multiplexer、色彩等级与背景回退解释的 crate contract 见
[`zeta-terminal-detection`](../zeta-rs/terminal-detection/README.md)；本节只定义 TUI 对真实终端
I/O 和 crossterm 生命周期的所有权。

`terminal/` 负责：

- raw mode、alternate screen 和 bracketed paste；
- Crossterm event 读取；
- Ratatui backend 和 frame scheduling；
- terminal resize、reflow、cursor 和 scrollback；
- 在独占 input window 中执行 terminal response probe 和控制序列；host terminal 身份、色彩等级及
  background fallback 解释由 `zeta-terminal-detection` 提供；
- panic、错误和正常退出时恢复终端。

`TerminalSession` 必须使用 RAII 恢复 raw mode、alternate screen、paste mode 和 cursor。
启动中途任一步失败，也必须回滚之前成功启用的能力。

`terminal/` 不知道产品 Session、Thread、Turn、feature 或 App Server。它可以产生
`TerminalEvent`，但不能发送 Agent `session/request`、打开 approval popup 或根据 Agent 状态决定文案。

## 11. `features/`：Zeta 功能的垂直切片

一个完整 feature 可以包含：

```text
features/<name>/
├── mod.rs
├── state.rs
├── command.rs
├── request.rs
├── view.rs
└── <name>_tests.rs
```

不是每个 feature 都需要全部文件。`command.rs` 表达用户意图，`request.rs` 直接调用数据
owner 的公开 typed interface，`view.rs` 组装 component。没有请求的 feature 不创建空
`request.rs`，也不建立 `service.rs`、provider registry 或 feature-local facade。

目标树中已经确定的首批 feature：

| Feature | 职责 |
| --- | --- |
| `thread` | active Thread snapshot/update、Turn start/interrupt、transient merge 与页面组装 |
| `sessions` | Session list/create/resume/archive 与 Thread create/fork/rewind/switch/archive |
| `interactions` | owner-directed approval 与 structured user-input view/response mapping |
| `config` | typed config read/update UI |
| `skills` | typed catalog、enablement intent 和 selection row model |
| `workspace_files` | `zeta-file-search` mention completion + typed filesystem browser/preview |
| `status_line` | 汇集既有接口结果并执行 item 排列、降级与渲染 |

resources 等已经有 typed contract、但尚无 `zeta code` 用户场景的能力不提前出现在
目录树中，也不构成 TUI backlog。只有产品文档先接受具体场景后，才按同一文件契约
添加 feature。

approval、request-user-input 与 MCP 已在 canonical contract 完整后作为垂直切片接入；TUI 只声明
实际支持的 interaction kind，App Server 选择唯一 owner，deadline/cancellation 不由 view 决定。
以下能力仍不能仅因为其他产品存在就提前创建：

- plugins、connectors 和 account/login surface；
- hooks、goals、usage 和 review；
- feedback、updates 和 visualization；
- sub-agent 或 side conversation 导航。

这些能力必须先进入 Zeta canonical domain 和 App Server API，具备 typed
request/response/notification、顺序、取消、错误与恢复语义，然后 TUI 才添加对应垂直切片。
未来 interaction kind 也必须先扩展同一个 capability/owner/request/response/deadline contract；
TUI 不能靠检查 ToolCall 名称或 arguments JSON 自行弹窗并决定策略。

外部 Agent 配置导入是明确的 Desktop-only 产品边界，不属于上述“等待 canonical contract
后再进入 TUI”的潜在功能。TUI 不提供 `/add-dir`、`/import-agent`、目录选择器或等价的
配置 mutation，也不主动扫描 `~/.codex`、`~/.claude` 等目录。Desktop 已经导入的外部 Skill
仍可通过 App Server 统一 catalog 出现在 TUI `/skills` 中；这只是消费既有来源，不使 TUI
成为导入或文件访问授权 owner。Desktop 工作流见
[`zeta-desktop-architecture.md`](zeta-desktop-architecture.md#22-外部-agent-配置导入仅限-desktop)，
Skill 来源边界见 [`skills.md`](skills.md#151-外部-agent-skill-导入仅限-desktop)。

Feature 之间不能依赖彼此的私有模块。跨功能结果由 `app/` 协调，交互复用通过
`components/`，纯布局复用通过 `ui/`；只有重复已经出现且语义一致时才提取公开的小型 value
type。

### 11.1 `status_line/`：接口结果的展示模型

长期应把 status line 作为完整的 presentation subsystem，但“完整”只表示它独立拥有 item
选择、排列、宽度降级和渲染，不表示它接管数据来源。各项数据仍由相应领域 crate 或 App
Server contract 的公开接口拥有，TUI 在事件/更新阶段直接调用这些接口，再把结果映射为可丢弃
的 `StatusLineModel`。禁止建立通用 `StatusProvider`、`StatusStore`，也禁止在 draw 路径查询
Git、配置或 Thread。

| Item | 权威接口 | TUI 的职责 | 当前状态 |
| --- | --- | --- | --- |
| preferred model | `AppServerClient::read_config` | 把 `ConfigReadResult::preferred_model` 映射为长/短文案 | 已实现 |
| workspace | `TuiOptions::workspace_root` | 保留完整路径和 basename 两种展示值 | 已实现 |
| Git branch/state | App Server `git/status` + `git/statusChanged`，其 owner 调用 `zeta-git` | startup/read 与 notification 映射 branch/dirty/count | 已实现 |
| Thread/Turn/usage | App Server typed snapshot/update | 消费 contract 已提供的字段，不从 transcript 推导 | Thread usage contract 已提供；status line 尚未接入 |
| connection/runtime state | `client/` 与 `app/` 本地状态 | 只在已接受的用户场景中映射 | Potential；embedded TUI 当前无独立 connection UI 需求 |

依赖方向固定为：

```text
owning crate interface / typed App Server result
                    │
                    ▼
            app update coordination
                    │
                    ▼
              StatusLineModel
                    │
                    ▼
          pure layout + Ratatui render
```

`status_line/` 未来可以定义稳定的 item identity、用户选择与顺序、separator、alignment 和
overflow policy；配置持久化必须先进入 typed config contract，不能由 renderer 私存。昂贵或
异步接口在后台完成后以 event 更新模型；失败只影响对应 item，并保留其明确的 unavailable/
stale 语义。任何新 item 都应先回答“哪个 crate/interface 拥有这个事实”，再添加展示映射和
宽度测试。

当前实现由 `features/status_line/model.rs` 拥有 model、Git projection 与宽度降级，
`features/status_line/view.rs` 只负责右对齐渲染。usage 和可配置 item/order 缺少已接受的
typed contract，因此不是当前 TUI 完成项。

## 12. `host/`：窄宿主能力

`host/` 只放非终端 OS adapter，例如 clipboard、external editor、desktop notification 和
IDE IPC。每个模块必须暴露窄能力，不能形成一个无所不包的 `PlatformService` 或
`HostContext`。

职责按“何时”与“如何”拆开：

```text
features/thread：何时通知用户 Turn 已结束
host/notification：如何调用某个 OS 通知后端

components/composer、components/transcript：产生 copy/open-editor intent
host/clipboard、host/external_editor：如何访问宿主能力
```

宿主 adapter 不得反向依赖 component 或 feature workflow。

## 13. 事件与命令流

顶层数据流固定为：

```text
TerminalEvent / ClientResult / ServerNotification
                      │
                      ▼
                   AppEvent
                      │
                      ▼
              TuiApp state transition
                │                 │
                ▼                 ▼
           AppCommand          redraw
                │
                ▼
        typed client / host adapter
                │
                └──────────────► AppEvent
```

建议区分：

- `AppEvent`：已经发生的事实；
- `AppCommand`：等待执行的副作用意图；
- feature event/command：feature 内部语义；
- `TerminalEvent`：原始 key、paste、resize、tick；
- `ServerNotification`：App Server typed notification。

不要把所有类型压成一个包含任意闭包、JSON 或字符串 method 的总线。原始 key event 应先由
当前焦点和 keymap 转换为用户意图，再进入业务 command。三端共享语法和 Zeta Code 的根级
Keymap 边界见 [`keybindings.md`](keybindings.md)。

单个 event-loop iteration 应有界，长请求、resource 读取和大型 transcript projection 不得阻塞
terminal event pump。重绘可以合并，但 committed update、输入和退出事件不能因为 frame
throttle 丢失。

### 13.1 输入、焦点与浮层

输入传播顺序固定为：

```text
system capture
      ↓
top overlay
      ↓
active feature
      ↓
focused component
      ↓
parent bubble
      ↓
application fallback
```

局部 handler 返回 `Handled`、`Propagate` 或 typed intent；不能通过默认空 hook 把 approval、
plugin 或 Session 等领域行为继续塞进通用 view trait。内置 overlay 优先使用可穷举的 enum 和
显式 stack，只有真实外部扩展场景才使用动态 view object。

Ctrl-C 等冲突键按当前交互层级解释：可取消 overlay 优先关闭 overlay，局部搜索优先取消搜索，
active Turn 转成 interrupt，只有空闲顶层状态才退出。相同语义必须由键盘、鼠标和 command
surface 复用同一 intent，不各自执行副作用。

### 13.2 控制面与流式数据面

普通控制总线不承担全部 streaming 流量：

```text
App Server streaming update
        ↓
typed protocol decode
        ↓
per-Thread bounded stream buffer
        ↓
batch / coalesce / fair scheduling
        ↓
features/thread semantic update
        ↓
coalesced frame request
```

| 事件类别 | Delivery class | 规则 |
| --- | --- | --- |
| 退出、用户提交、interrupt、approval response | Must deliver / ordered | 最高控制优先级，不被后台流量饿死 |
| committed update、command completion、fatal error | Ordered lossless | 进入控制面；不得被 frame throttle 合并掉 |
| assistant/process text delta | Ordered + recoverable gap | 正常路径连续应用；overflow 可丢 transient，但必须由 cursor gap 清除 projection 并 resync |
| tool/progress preview | Latest wins / coalescable | 允许保留最新 revision |
| draw request | Coalescable | 多次请求合并为一次 frame |
| resize | Latest wins | 保留最新尺寸并触发必要 reflow |
| 搜索建议等后台查询 | Latest generation wins | scope 或 generation 不匹配即丢弃 |
| telemetry | Best effort | 不得反压用户交互 |

每轮先处理高优先级控制事件，再按固定数量或时间预算消费数据面；一批数据只请求一次 frame。
completion 是控制面 barrier：应用 completion 前必须消费或明确作废属于该 Turn 的先前数据。
buffer overflow、receiver lag 或 cursor gap 立即把对应 projection 标为需要 resync，不能静默
丢弃 Must-deliver 语义。

当前实现的三层上限是 App Server 每 connection 4096 条 notification、共享 client 1024 条 event、
TUI EventPump 1024 条 runtime event；App Server 满载时先清 transient，control-only overflow 关闭
connection。Thread projection 每个 transient row 限 256 KiB、最多 1024 个 identity。当前 TUI
产品支持边界不包含自动 reconnect；connection overflow 会显示 failure 并要求重启。这与已有
cursor-gap snapshot resync 是两个不同恢复层级，不能把前者写成已承诺的 TUI 恢复阶段。

### 13.3 展示映射

TUI 不应长期保存原始 transport envelope。protocol crate 已经提供稳定 canonical
snapshot/update 时，feature 直接消费这些 typed value；只有满足以下至少一项时才增加
presentation mapping：

- wire object 字段庞大或变化频繁，而界面只需要稳定子集；
- Desktop/TUI 等多个前端确实共享同一种 presentation semantic；
- replay、脱敏 trace 或多种 render mode 需要稳定 identity/revision；
- server-originated agent request 需要转换成不会泄漏 transport 细节的交互模型。

不机械包装稳定 ID、简单 enum 或已经 canonical 的产品 value。mapping 只能降低耦合，不能
成为第二套领域模型或兼容私有 wire DTO 的长期层。

## 14. 公共 API 与 CLI 所有权

`zeta-tui` 的公开 API 应保持很小：

```rust
pub fn run(
    session: AppServerSession,
    options: TuiOptions,
) -> Result<TuiExit, TuiError>;
```

`run` 从已经初始化的 `AppServerSession` 获得 request handle 与 event stream，并在退出路径
显式调用 `shutdown()`、等待 background driver join。它不接受启动参数后自行选择 transport，
也不接受生命周期不明确的裸 transport/client。当前 Rust host 保持同步入口，connection
notification 与 terminal input 已由独立 source 唤醒；共享 typed method 保持同步返回，但 TUI
通过 `RequestTask → app::request_completion` 将等待和结果应用分开。

入口建议使用 enum 表达，避免含义不明的 bool 或 `Option`：

```rust
pub enum TuiEntry {
    CreateSession { title: String },
    OpenSession { session_id: SessionId },
    OpenThread {
        session_id: SessionId,
        thread_id: ThreadId,
    },
}
```

CLI 负责：

- 判断 stdin/stdout 是否为 TTY；
- 解析命令行和选择 interactive 模式；
- 加载 config 和 state root；
- 选择 in-process、daemon 或 remote transport；
- initialize 并校验 protocol major 与 required capability versions，记录 schema hash 诊断；
- 构造 `TuiOptions` 和已初始化 `AppServerSession`；
- 把 `TuiExit` 映射为进程退出码。

TUI 负责 terminal 打开后的交互，不应反向读取 CLI arguments 或自行打开本地 App Server。

## 15. 依赖规则

| 模块 | 可以依赖 | 禁止依赖 |
| --- | --- | --- |
| `app` | client、features、components、ui、terminal、host | Core、Storage、具体 widget 实现、JSON-RPC 字符串 |
| `client` | app-server-client、app-server-protocol、内部 value type | Ratatui、view、Core、私有 wire DTO |
| `features` | owner crate 的 public interface、typed App Server client、components、ui、canonical value | 其他 feature 私有模块、Core、手写 RPC |
| `components` | ui、Ratatui、必要的 canonical presentation value | client、领域请求、canonical aggregate authority |
| `ui` | Ratatui 和纯展示 value | 产品 ID、App Server、业务状态 |
| `terminal` | Crossterm、Ratatui backend、纯 terminal value | AppEvent、产品 ID、feature |
| `host` | 窄 OS library | app、components、feature workflow |

整体方向：

```text
app
├─ client ─────────────────────► zeta-app-server-client
├─ features ───────────────────► owning crate public interfaces
│  └─ components ──────────────► ui + Ratatui
├─ components ─────────────────► ui + Ratatui
├─ ui ─────────────────────────► Ratatui
├─ terminal ───────────────────► Crossterm + Ratatui backend
└─ host ───────────────────────► narrow OS libraries
```

feature 定义自己的 intent，并由同目录 `request.rs` 直接映射到 owner interface；`app` 只调度
intent 和 result event，不维护一份跨领域 operation enum。view 产生 Ratatui render data，
由 `app` 在 terminal frame 中装配；feature、component 和 ui 都不依赖 terminal。

禁止新增含义模糊的 `runtime`、`service`、`common` 或 `platform` 聚合层。共享代码必须根据
其真正职责进入 feature、component、ui、client、terminal 或某个窄 host module。

## 16. 当前实现与产品支持边界

当前 `zeta-code/tui/src/` 已完成第一阶段物理 ownership 重排，并迁到 owned session/event
contract：

```text
app/
├── mod.rs
├── bootstrap.rs
├── command.rs
├── dispatch.rs
├── event.rs
├── event_loop.rs
├── frame/
├── help.rs
├── keymap.rs
├── keybindings_resource.rs
├── request_completion.rs
├── state.rs
└── state_tests.rs
client/
├── mod.rs
├── command_id.rs
├── event_pump.rs
├── notification.rs
├── request_task.rs
└── *_tests.rs
components/
├── mod.rs
├── composer/
├── interaction/
├── selection/
└── transcript/
features/
├── mod.rs
├── config/
├── interactions.rs + interactions/
├── sessions/
├── skills/
├── status_line/
├── thread/
│   ├── presentation.rs
│   ├── projection.rs
│   ├── request.rs
│   ├── state.rs
│   ├── subscription.rs
│   └── update.rs
└── workspace_files/
host/
└── clipboard.rs
terminal/
├── mod.rs
├── session.rs
└── session_tests.rs
ui/
├── mod.rs
├── layout.rs
├── layout_tests.rs
└── theme.rs
lib.rs
lib_tests.rs
```

已经落地的边界：

- 通过 `zeta-app-server-client` 的 typed method 工作；
- 明确声明权威 Thread/Turn 状态留在 App Server 后面；
- `App` 不再持有 file-search worker/channel；`AppCommand` 描述待执行副作用，外部结果统一以
  `AppEvent` 进入 `App::update`，`FileSearchManager` 由 event loop 持有；
- `client/event_pump.rs` 独立等待 terminal 与 `AppServerEvents`，并把两者汇入单写者 loop；
  `client/notification.rs` 把共享 connection event 映射成 typed `ClientEvent`，保留
  `agent/request`、`skills/changed`、Git、`ThreadUpdateEnvelope` 和 connection failure；event
  channel 有界，Tick 可丢弃而 input/control 不静默丢失；
- `client/RequestTask` 在 worker 执行 typed request，`app/request_completion.rs` 校验 scope 并把
  completion 安装到单写者 state；同一 request slot 前的用户 intent 保序排队，全部 product
  command、Turn mutation、文件浏览与 subscription switch 均不在 draw/input 线程等待；
- `features/thread/ThreadSubscription` 在启动和 active Thread 切换时调用 typed
  `session/thread/subscribe`/`session/thread/unsubscribe`，验证 Session/Thread scope，并用最后确认的 snapshot
  sequence 丢弃重复或旧 scope update；stream instance/cursor 单独排序 transient，gap/runtime
  switch 清除不可信 row 并请求 snapshot；
- newer durable sequence（包括 gap）只触发 `session/thread/read` resync；TUI 不执行 `ThreadEvent`
  reducer。`refresh_turn` 不再二次 drain notification，因此不会吞掉订阅事件；
- `features/thread/ThreadFeatureState` 已成为 active Thread snapshot 与当前 transcript projection
  的唯一 TUI owner；本地 optimistic user message、notice 与 failure 也通过 feature event
  进入同一 owner，下一份 canonical snapshot 会替换 projection；
- `features/thread/projection.rs` 显示完整 ThreadItem，并有界保存最多 1024 个 transient identity、
  每个 row 256 KiB；`components/transcript` 只负责 role/detail layout 和 scroll；
- `components/transcript` 已拥有 transcript row wrapping、role chrome、empty state 与只读
  Ratatui view，以及 component-facing `Message`/`MessageRole`；它不依赖 feature、`App` 或保存
  canonical Thread；
- `features/thread/request.rs` 只构造并执行 typed Thread/Turn request，返回 typed result；
  request module 不引用或更新 `App`。event loop 把结果转换为 `AppEvent`，presentation module
  只把 canonical Turn snapshot 分类为可展示 outcome；
- `features/sessions/ActiveConversation` 拥有当前 product Session/Thread identity 与 sequence，
  create/fork/rewind/resume/switch/archive 返回 conversation change/notice，不直接写 `App`；新的
  canonical snapshot 由后台 subscription completion 安装。Session picker、Thread active/archived
  tabs 与 replacement lifecycle 由同一 feature 拥有；
- `features/interactions` 把 owner-directed full request 转成 approval 或多问题 user-input Pane，
  只返回 exact typed response；owner selection、deadline 与 cancellation 留在 App Server；
- `features/config/request.rs` 与 `features/skills/request.rs` 分别拥有已有 typed config/MCP/model
  与 Skill catalog/enablement 调用，App 不再内联这些领域 payload；
- `components/selection` 已同时拥有 generic tabs/query/filter/selection state、输入 outcome 与
  Ratatui view；`InteractionPane`、App 和产品 view builder 只消费该 component contract；
- `ui/layout.rs` 拥有跨 presentation surface 复用的纯 geometry；`ui/theme.rs` 只拥有共享主题
  snapshot 到终端色彩能力的窄投影，用户文件解析与完整 token catalog 留在 `zeta-theme`；
  component 不反向依赖 frame coordinator；
- `app/keymap.rs` 已通过产品无关 `zeta-keybinding` 注册 Shift-Tab、根级 Esc 与
  Ctrl-C/D/O/V/Z，并从同一静态声明生成 Resolver 规则和 `/help` 项；Crossterm event 单向转换为
  标准 `KeyStroke`，修饰键精确匹配。运行时结构 `AppKeymap` 已拥有一至四段 Chord 的 pending、
  1 秒超时、上下文变化/Esc 取消、错误后续键透传和 footer 提示；当前内建表仍只声明单段组合。
  普通单键保持 component-first，只有 Chord prefix 在 component 前路由；composer 编辑、selection
  导航与 transcript 滚动继续由局部 component 拥有；
- `app/keybindings_resource.rs` 已读取 CLI 显式提供的 active profile 下
  `zeta-code/keybindings.json`，在 event-loop Tick 中有界热重载 User command/blocker、平台覆盖与
  `when`。完整编译和 TUI Chord 安全校验成功后才替换 `AppKeymap`；坏更新保留上一份有效映射并
  产生可见诊断。资源不进入 App Server，也不从 Remote Workspace 读取客户端按键配置；
- `App` 处理 presentation coordination 与 Keymap action，并直接委托 `InteractionPane` 的
  composer/temporary-view 输入；`ChatWidget` 与过渡目录 `toppane/` 已移除，不再存在第二份
  transcript 或模糊的 top-pane owner；
- composer、editor、attachment、paste、slash/mention state 与各自纯 view 已迁入
  `components/composer/`；temporary stack 已迁入 `components/interaction/`；
- update-driven snapshot resync 先应用完整 canonical Thread，再把
  completed/waiting/failed/interrupted 映射为 presentation lifecycle；active Turn 的定时
  snapshot polling 已移除，Turn completion 不再单独追加 agent 文本；
- `InteractionPane` 保留 composer 并拥有 temporary view stack；generic selection view 已支持
  tabs、直接输入搜索、过滤、循环选择、左右/Tab 切页和 Esc/Ctrl-C 出栈；`/help` 提供
  Commands/Keys，`/skills` 从 typed `skills/list` 提供
  All/Enabled/Disabled/Errors catalog tabs；
- `/skills` 只消费 App Server catalog snapshot，不读取 `zeta-skills` filesystem；
  `Space` 将 exact `SkillId` 转成 revision-checked `skill/enablement/set`，成功后刷新页面；
  `skills/changed` 也会刷新前台页面。enablement 不等于正文 activation，TUI 当前没有
  Skill context injection；
- `app/frame/` 只装配 frame；各 component/feature view 拥有自己的 surface。layout 把所有
  interaction surface 显式锚定在 terminal 底部：composer/footer
  固定到底部，slash/mention popup 从 composer 上沿向上展开，temporary view 保持底边不动并
  只向上占用 transcript 空间；
- `TerminalModeGuard` 在任一 mode 获取失败时按逆序恢复已经获取的 terminal mode，显式
  restore 和 Drop 共享幂等清理路径；
- `StatusLineModel` 直接映射 typed config/Git result 与 `TuiOptions::workspace_root`；
  `features/status_line/view.rs` 只读取模型，并按可用宽度从完整 model/workspace/Git 降级到短值
  或省略号；
- `ChatComposer` 协调提交、popup keys、range completion application 与 structured local
  command dispatch；`zeta-slash-commands` 拥有 slash grammar、catalog、matches、selection 与
  dismiss，Ratatui popup renderer 根据自身 viewport 投影可见范围，`TextArea` 只拥有 UTF-8
  编辑状态、原子 command element 和局部 keymap 扩展边界；当前没有 Vim 产品要求；
- bracketed paste 使用独立事件路径；超过 1000 个 Unicode scalar value 的内容由 `PendingPastes`
  绑定到 `TextArea` 原子占位符，并在提交前展开；
- 粘贴 PNG/JPEG/GIF/WEBP 本地文件路径会由 `Attachments` 立即读取并绑定为 `[Image #N]`
  原子占位符；提交保持 text/image 顺序并通过 typed `session/request` 进入 durable Thread history；
- `Ctrl-V` 产生独立 `AppCommand::ReadClipboardImage`，由 `host/clipboard.rs` 读取文件列表
  或 RGBA 位图、编码 PNG，并复用 `Attachments` 的校验、占位符与结构化提交；
- event loop 持有的 `FileSearchManager` 通过 `zeta-file-search::PathSearchHandle` 在后台增量
  遍历 workspace，并使用完整 `nucleo` engine 更新 `@token` fuzzy results；snapshot 作为
  `AppEvent` 回到单写者。`Mentions` 只拥有 token/popup 状态、高亮、keyboard/mouse selection
  和原子文本路径 completion。旧 query snapshot 会在 manager 和 popup 边界被丢弃；两者都不
  读取候选文件内容，也不构造结构化 app/plugin Mention；
- crate root `lib.rs` 只保留 public startup contract 与错误类型；事件循环、bootstrap、
  built-in dispatch、Thread request 和 frame coordination 都有明确的 private owner；
- 所有写请求通过 `client::new_command_id` 分配 typed `CommandId`，不再在 crate root 复制
  command ID 拼装逻辑。

当前产品支持边界与非目标：

- transcript 采用 bounded plain-text wrapping、分页键盘滚动、最后回复 copy 与当前已加载 history
  window 的 Markdown export。
  Native Agent Timeline 的 Markdown/table、任意 pointer selection、折叠与虚拟化属于 `app`，
  不是 TUI 的“尚未完成”；
- Mouse 只覆盖 slash/file-mention popup 的必要左键命中。完整 pointer/selection 交互不属于当前
  `zeta code` 要求；Vim mode/motion/operator 也没有被产品文档接受；
- 当前入口通过 `AppServerSession` 消费 profile/Workspace-scoped local authority，不提供 remote
  selector 或自动 reconnect。若未来接受
  远程产品需求，connection/recovery contract 必须先进入 `zeta-app-server-client`；
- workspace mention 当前只插入原子文本路径。通用 app/plugin Mention、login、compact、service
  tier、usage、review 等没有已接受 contract 的 surface 不注册；
- 图片输入已形成“本地路径/系统 clipboard → 草稿 data URL → App Server 分块上传 →
  `ImageAttachmentRef` → durable `UserImageAttachment` → provider 临时 image block”纵切。TUI
  不建立私有 blob store；Thread history 与 command receipt 不持久化 data URL；
- status line 已有 model/workspace/Git；usage 与稳定 item/order 没有 typed contract，因此不是通过
  transcript 推导的 TUI 缺口；
- Config surface 可读 provider、MCP、Skill source、Plugin request、Hook、language server 状态；
  当前只有已有 typed mutation 的 model/MCP/Skill 可修改，TUI 不接管 Desktop-only 外部 Agent
  导入或凭据配置。

新增能力必须先证明是 `zeta code` 产品要求，再按 canonical contract 和垂直 feature 接入；不能
因为 Native 已有 richer component，或某能力技术上可实现，就把它复制成 TUI backlog。

## 17. 已接受的架构迁移顺序

当前阶段状态：

| 边界 | 状态 |
| --- | --- |
| terminal session RAII 与 partial-failure rollback | Current |
| `app/state.rs`、`app/event.rs`、`app/command.rs` | Current |
| runtime file-search worker 与 `App` state 分离 | Current |
| typed notification 适配、active Thread subscription 与 snapshot gap/resync | Current |
| independent request driver 与 wakeable notification pump | Current |
| request completion 的非阻塞 app command dispatch | Current |
| canonical `features/thread` snapshot 与 transcript projection owner | Current |
| `components/transcript` 与 `ui` layout/theme 原语 | Current |
| `components/selection` state/view 边界 | Current |
| composer/interaction component 物理边界 | Current |
| Thread transient merge、cursor recovery 与 bounded data plane | Current |
| Session/Thread picker、archive 与恢复 | Current |
| owner-directed approval / user input / deadline | Current |
| 多行 composer 与 active-Turn follow-up queue | Current |
| bounded Thread history window 与 Ctrl-Home 增量加载 | Current |
| command copy/export 与 Ctrl-Z suspend/resume | Current |

### 阶段零：固定行为与性能基线

在移动 owner 或改变 event loop 前：

1. 固定 terminal open/partial-failure/Drop restore、输入、submit/interrupt、resize 和 popup 的
   行为测试；
2. 记录 startup first frame、输入到 frame、snapshot/update 到 frame、render duration 和
   resize/reflow duration；
3. 清点 `lib.rs`、`App`、`InteractionPane` 的状态 owner、I/O 调用、task/channel 与 protocol
   type 引用；
4. 为 dependency direction 建立可在 CI 执行的检查；
5. 确认每个迁移切片只有一个 source of truth，旧层只允许单向转换。

退出条件：关键行为可自动验证，性能回退可比较，当前 implementation limitation 已在 crate
README 中记录。

### 阶段一：建立边界

1. 将 terminal RAII 移入 `terminal/session.rs`；（Current）
2. 将纯 TUI state/event/command 移入 `app/`；（Current）
3. 将 notification adapter、`CommandId` 与 subscription tracking 移入明确 owner；
   （wakeable event pump、notification adapter、`CommandId` 与 Thread subscription tracking
   以及 request completion 的 app-level 非阻塞调度均为 Current）
4. 建立 `features/thread/`，用 canonical Thread snapshot 替换扁平 message authority；
   （snapshot/projection owner、typed request、durable sequence/gap snapshot resync 为 Current；
   transient merge 和完整 ThreadItem projection 也为 Current）
5. 把 bootstrap `toppane/` 与各 presentation surface 按职责迁入
   `components/transcript/`、`components/interaction/`、`components/selection/` 和
   `components/composer/`；（Current；旧 `toppane/` 与顶层 `render/` 已移除）
   保留 `InteractionPane → ChatComposer → TextArea` 的局部 ownership；
6. 将 public `run` 收敛为接收 owned `AppServerSession`，并在所有正常/错误退出路径显式
   shutdown。（Current）

退出条件：terminal 不依赖产品状态；可测试 TUI state 不含 client、terminal、channel 或 task；
现有入口与用户可见行为不变。

### 阶段二：订阅与恢复

1. 使用 `session/subscribe` 和 `session/thread/subscribe`；（Current）
2. 实现 durable gap、duplicate、aggregate mismatch 和 resync；（active Thread snapshot
   resync 为 Current）
3. 实现 transient cursor、runtime switch、gap reset 和 committed snapshot 替换；（Current）
4. 把稳定 `CommandId` 生命周期集中到 `client/command_id.rs`；（Current）

退出条件：Turn 执行期间输入和 redraw 不被 request completion 阻塞；duplicate、gap、lag、
connection close 和 runtime 切换都有确定结果；当前 local authority client 不会把结果未知的写入当成
失败后新命令重放。Remote reconnect 不在本阶段范围内。

### 阶段三：核心交互

1. 在 `features/sessions/` 完成 Session list/resume/archive；（Current）
2. 完成 Thread create/fork/rewind/switch/archive；（Current）
3. 让 Turn start/interrupt 全部经过 `features/thread/request.rs`；（Current）
4. 让 Thread projection 与 `components/transcript/` 展示完整 ThreadItem；（Current）
5. 在对应 component 内完成当前产品要求的 scroll 与 composer history；（Current）

退出条件：active Thread state 只有一个 TUI owner；完整 typed Item lifecycle 可呈现；transient
流量不会决定 durable terminal state；render 不推进 semantic state。

### 阶段四：垂直功能

按已接受的 App Server contract 逐个添加 config、resources、approval 等 feature。config、MCP、
Skill、workspace file browser、Git status、approval 与 user input 已按该规则接入；每个后续 feature
同时交付 state、typed command、view、错误/恢复行为和测试，不采用先建一个全局
`services/` 再逐步塞逻辑的方式。

不进行一次性 `git mv` 大重排。每一步都应保持 crate 可编译、现有入口可运行，并让新 owner
和测试一起迁移。

每个新增 feature 的退出条件相同：功能主要修改集中在自己的目录，不向顶层大 enum 或
`App` 或 component 增加完整领域状态，所有异步结果携带 scope/generation，failure 与 resync
行为和 feature 一起交付。

### 潜在阶段：抽取公共能力

独立 runtime/component crate 或第三方 UI API 不是当前 roadmap。只有至少两个真实消费者已经
使用同一内部边界、API 稳定多个版本且抽取能减少依赖与转换时才重新评估；否则继续在单 crate
内使用 private module。当前 `zeta-ansi-escape` 是已经完成的窄 dependency adapter，不是可复用
component 或第三方 UI API，也不放宽其他 presentation state 的抽取门槛。

## 18. 测试

测试按 owner 放置：

- 新单元测试模块使用 sibling `*_tests.rs` 和显式 `#[path = "..._tests.rs"]`；
- `features/thread` 测试覆盖连续 update、重复 delivery、durable gap、runtime 切换、
  transient/committed 合并和 resync；
- feature request 测试使用 fake/mock typed client 验证 payload、稳定 CommandId 和错误映射；
- component 测试覆盖 Unicode width、plain-text wrapping、resize、局部交互和纯渲染；
- client 测试覆盖 event pump、pending completion 和 subscription transport lifecycle；
- terminal 测试覆盖部分初始化失败与 Drop 恢复；
- feature 测试覆盖 key intent → command → result event → view state；
- crate 级 `tests/` 覆盖 create/resume/fork/interrupt 和 subscription recovery；
- Ratatui `TestBackend` 与 snapshot 只验证稳定布局，不替代状态断言。

测试支持代码也按 owner 拆分。只有确实跨多个模块且 API 稳定的 fixture 才进入 crate 级
`tests/support/`，不能建立全局 `test_utils.rs` 杂物箱。

Rust 模块目标保持在 500 行以内；文件接近 800 行时，新功能必须进入新模块。新增 public
trait 必须有 doc comment，说明其职责、实现约束和调用方预期。

目标测试矩阵（Proposed）：

| 层级 | 输入与断言 | 不应依赖 |
| --- | --- | --- |
| feature state/update | state + event → state + command + invalidation | terminal、真实网络、真实时间 |
| client contract | typed request/result/notification、identity 与 error mapping | Ratatui、Core private API |
| component | local intent、Unicode/grapheme、layout 与 view model | RPC、canonical reducer |
| render golden | 40/80/120 列、CJK/emoji、长 URL、capability、resize | 业务副作用 |
| terminal behavior | cursor、scroll region、clear、suspend/resume、partial open failure | 产品 feature |
| trace replay | 初始 presentation state + 脱敏 event trace → digest/key frames | 原始敏感 payload |
| end-to-end | create/resume/fork/interrupt、subscription recovery、shutdown | 私有 transport shortcut |

property test 优先覆盖 generation 单调、stale result 不改变状态、overlay stack 合法、
transcript revision/cache invalidation、cursor 保持在 viewport、shutdown 后无活动 task，以及
bounded buffer 不丢 Must-deliver event。

CI 应以简单、可解释的检查固定依赖方向：

- `terminal/` 不依赖 `app`、feature 或产品 ID；
- `ui/` 不依赖 App Server 与产品状态；
- `components/` 不依赖 client 或领域请求；
- renderer/view 不调用 I/O 或 `tokio::spawn`；
- feature 不手写 JSON-RPC，不依赖其他 feature 的 private module；
- 只有 client/protocol boundary 解释 transport notification。

## 评估、可观测性与发布

架构迁移必须以行为与性能不回退为前提。至少跟踪：

- startup first-frame duration；
- terminal input 到 visible feedback 的 P50/P95；
- stream/update 到 frame 的 P50/P95；
- render 与 resize/reflow duration；
- frame request、实际 frame、coalesced frame 数量；
- control/data queue depth、batch size、overflow/lag；
- stale result 丢弃、request timeout/cancel 和 resync 原因；
- shutdown task drain/cancel 结果。

日志记录 event/command 名称、feature、request identity metadata、scope、duration 与结果类别，
不记录用户输入、模型输出、文件内容或 secret。可重放 trace 默认脱敏 payload，并明确区分
metadata-only 诊断与包含测试 fixture 的受控 trace。

每个迁移 PR 必须：

1. 只迁移一个明确 owner 或一条完整垂直切片；
2. 保持旧入口可用并可独立回滚；
3. 给出行为测试以及相关性能对比；
4. 删除旧 source of truth，或明确兼容 adapter 的单向性与删除条件；
5. 同步 crate README 的当前事实和本文的阶段状态。

## 主要风险与防线

| 风险 | 识别信号 | 防线 |
| --- | --- | --- |
| Action/command 形式化过度 | cursor movement 等局部操作也穿过全局总线 | 只有跨 component/feature、异步或需 trace 的行为进入顶层 |
| 重新形成巨型 `AppEvent`/`AppCommand` | 新 feature 必须持续修改全局变体 | 顶层只路由 feature-local enum |
| 机械 protocol mapping | 稳定 ID/enum 被逐一包装 | 只转换高变、展示语义不同或 replay 所需对象 |
| 控制面与数据面双状态源 | buffer 和 feature 都决定 active Turn | buffer 只存未合并事件，主循环仍是唯一 semantic writer |
| 迁移期双 owner | old presentation container 与新 feature 双向同步 | 每个切片明确唯一 source of truth，adapter 只能单向 |
| 过早拆 crate | public API 和类型转换快速增长 | 默认先用 private module；窄 dependency adapter 必须保持无产品状态并单独固定 failure contract |
| terminal 行为回退 | 架构重排同时改变 cursor/viewport | 先固定 behavior test，terminal 与 feature 分阶段迁移 |
| 数据面饿死输入 | delta backlog 提高按键延迟 | 控制优先级、每轮 budget、batch/coalesce 与 resync |

明确拒绝只拆文件、全面重写、每个组件一个 actor、全局响应式 signal、ECS，以及在产品 contract
稳定前开放第三方 UI API。这些方案不能改善 Zeta 当前最关键的 authority、顺序和恢复边界，
却会增加新的生命周期与同步成本。

## 19. 验收

- TUI crate 不直接依赖 Core、Storage、Exec、Sandbox 或 Model Provider；
- CLI 向 TUI 传入已初始化 typed client，TUI 不自行建立第二套 composition root；
- 每个 feature 的本地状态可以从相应 typed snapshot/result 完整重建；
- 不存在跨 Session/Thread/config 等领域的通用 `ProjectionStore`；
- active Thread snapshot、sequence 和 transient merge 只有 `features/thread` 一个 TUI owner；
- durable sequence 和 transient cursor 分开处理；
- sequence gap、runtime 切换和未知 update 会触发 resync，而不是猜测状态；
- `expectedSequence` 来自正确 aggregate，逻辑重试复用原 `CommandId`；
- transcript 展示完整 typed Turn/ThreadItem lifecycle；
- Ctrl-C 在 Turn 运行时优先发出 `session/request` 的 `InterruptTurn`，空闲时才退出；
- terminal 在正常退出、错误、panic 和 Unix termination signal 路径均可恢复；
- feature 只通过 owner crate 的 public interface 或已接受的 App Server contract 工作；
- UI 原语、terminal 基础设施和 host adapter 不依赖产品状态；
- control plane 不被 streaming data plane 饿死，overflow/lag 会触发确定 resync；
- stale request result 不能覆盖新的 scope/generation；
- render 不执行 I/O、不 spawn、不推进 semantic state；
- migration 不保留双向同步的第二 source of truth；
- 单元、feature-state、transport、render 和端到端测试覆盖主要恢复路径。
