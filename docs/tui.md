# TUI 架构与演进方案

> 物理位置：`zeta-rs/tui/`  
> 宿主：`zeta-rs/cli/`  
> 当前实现接口与事件循环：[`zeta-rs/tui/README.md`](../zeta-rs/tui/README.md)
> 产品接口基线：[`zeta-app-server-api.md`](zeta-app-server-api.md)  
> App Server 启动与连接基线：[`app-server-client.md`](app-server-client.md)  
> Workspace 边界基线：[`zeta-rs-architecture.md`](zeta-rs-architecture.md)

## 1. 结论

TUI 采用“稳定核心模块 + 垂直功能模块”，但不能直接复制 Codex TUI 的模块划分。

Zeta 已经在 TUI 外部拥有：

- `zeta-core` 中权威的 `SessionCoordinator` 和 `ThreadController`；
- `zeta-app-server-protocol` 中唯一的 wire contract；
- `zeta-app-server-client` 中共享的 App Server 启动、初始化、请求/事件连接与关闭层；
- CLI 交付的启动配置与产品入口参数。

因此 TUI 必须是可丢弃、可重新同步的 presentation shell，而不是第二个 Agent runtime 或
App Server facade。目标依赖链固定为：

```text
zeta-cli
  → zeta-tui
  → zeta-app-server-client
  → App Server dispatcher
  → zeta-core
```

进程内模式只是一种 transport 优化。TUI 仍然经过 initialize、typed request/response、
dispatcher 和 notification decode，不得直接依赖 Core、Storage、Exec、Sandbox 或 Model
Provider。

## 2. 与原讨论结构的关键差异

原讨论中的八类结构不能原样落到 Zeta：

| 原模块 | Zeta 中的处理 |
| --- | --- |
| `app/` | 保留，但只拥有 TUI 状态、事件协调和退出流程 |
| `app_server/` | 不建立；typed RPC 已由 `zeta-app-server-client` 和 protocol crate 拥有 |
| `thread/` | 改为 `projection/`；TUI 只维护 Session/Thread 客户端投影 |
| `chat/` | 改为 `conversation/`；Turn 还包含 reasoning、plan、tool item，不只是聊天消息 |
| `ui/` | 保留，只放可复用 Ratatui 原语 |
| `terminal/` | 保留，只负责真实终端生命周期和能力 |
| `features/` | 保留，但只添加 Zeta 已有或已接受产品契约支持的功能 |
| `platform/` | 不建立泛化 facade；窄 OS adapter 放入 `host/` 的明确子模块 |

尤其不能在 TUI 中定义第二个 `ThreadController`。`zeta-core::ThreadController` 是 Thread 执行、
持久化、顺序和恢复的 authority；TUI 的 Thread 数据只是由 snapshot 和 update 构成的
read model。

同样，TUI 内部不应出现另一个聚合 account、session、thread、turn、config 等 RPC 域的
facade。TUI 使用 `zeta-app-server-client::AppServerSession` 启动并拥有本地 App Server
运行会话，再围绕其 cloneable request handle 与 event stream 补充交互客户端所需的请求调度、
订阅和错误映射。该 `AppServerSession` 是 connection/runtime owner，不是产品 `Session`。

## 3. 产品状态与 TUI 状态

权威产品模型与 sequence/cursor 语义统一见 [`protocol.md`](protocol.md)。TUI 只消费这些
canonical snapshot/update，不在本地重新定义产品实体。

TUI 可以保存以下客户端状态：

- 当前选中的 Session、Thread、Turn 或 Item；
- canonical Session/Thread snapshot 的本地投影；
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

## 4. 目标目录

目标结构如下：

```text
zeta-rs/tui/
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
│   │   ├── driver.rs
│   │   ├── operation.rs
│   │   ├── notification.rs
│   │   ├── subscription.rs
│   │   ├── command_id.rs
│   │   └── error.rs
│   ├── projection/
│   │   ├── mod.rs
│   │   ├── store.rs
│   │   ├── session.rs
│   │   ├── thread.rs
│   │   ├── transient.rs
│   │   └── resync.rs
│   ├── conversation/
│   │   ├── mod.rs
│   │   ├── composer/
│   │   ├── transcript/
│   │   ├── streaming/
│   │   └── turn_flow.rs
│   ├── ui/
│   │   ├── mod.rs
│   │   ├── layout/
│   │   ├── widgets/
│   │   ├── picker/
│   │   ├── overlay/
│   │   ├── keymap/
│   │   └── theme/
│   ├── terminal/
│   │   ├── mod.rs
│   │   ├── session.rs
│   │   ├── event_stream.rs
│   │   ├── frame_scheduler.rs
│   │   ├── scrollback.rs
│   │   ├── reflow.rs
│   │   ├── cursor.rs
│   │   └── capabilities.rs
│   ├── features/
│   │   ├── mod.rs
│   │   ├── session_browser/
│   │   ├── thread_navigation/
│   │   ├── config/
│   │   ├── resources/
│   │   └── status/
│   ├── host/
│   │   ├── mod.rs
│   │   ├── clipboard.rs
│   │   ├── external_editor.rs
│   │   ├── notification.rs
│   │   └── ide.rs
│   └── lib.rs
├── assets/
├── tests/
├── Cargo.toml
└── README.md
```

这是随实现增长的目标边界，不要求先创建所有空目录。只有当一个模块拥有明确职责、测试和
调用方时才落地；不能把目录树本身当作架构完成度。

## 5. `app/`：应用协调

`app/` 负责：

- 接收 terminal event、client result、server notification 和 background completion；
- 把输入转换为语义明确的 `AppCommand`；
- 协调 projection、conversation、feature 和 overlay；
- 管理焦点、顶层模式、退出与恢复终端；
- 决定何时请求重绘。

建议把可测试状态与副作用 driver 分开：

```rust
struct TuiApp {
    connection: ConnectionViewState,
    projections: ProjectionStore,
    active_view: ActiveView,
    conversation: ConversationState,
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

`bootstrap.rs` 只负责 TUI 启动流程，例如打开 terminal、建立初始 projection、进入指定
Session。CLI 可以构造 start options，但 App Server composition、channel 建立、initialize、
schema gate 和 shutdown 属于 `zeta-app-server-client`，TUI 不复制这些步骤。

## 6. `client/`：TUI 到 typed client 的窄适配

`client/` 不是新的 App Server facade。它只负责交互客户端特有的：

- 将 `AppCommand` 映射到 cloneable `AppServerClient` request handle 的 typed 方法；
- 生成并保存一次逻辑命令的 `CommandId`；
- 跟踪 pending request、取消意图和结果；
- 持续消费 `AppServerEvents`，将 `ServerNotification` 转换为内部事件；
- 建立、切换和释放 Session/Thread subscription；
- 将 stable server error 映射为可操作的 TUI 错误类别。

RPC Params、Result、Notification 和错误码仍由：

```text
zeta-app-server-protocol
zeta-app-server-client
```

唯一拥有。`client/` 不得复制 DTO、兼容旧 method、直接序列化 JSON-RPC 或暴露
`execute(method: &str, ...)`。

一次逻辑写操作在超时或响应丢失后重试时必须复用原 `CommandId` 和 exact typed payload。
用户再次点击或再次提交是新命令，必须生成新 ID。`expectedSequence` 来自目标 aggregate
最后确认的 canonical projection，不能使用 JSON-RPC request ID、stream cursor 或另一个
aggregate 的 sequence。

当前同步 `round_trip + drain_notifications` 是待替换实现，不能作为 TUI 目标基线。共享 client
层必须提供独立的 request completion 与 notification event pump，使 `turn/start` 期间 TUI
仍能处理键盘、重绘和 server update。TUI 不能通过直连 Core、读取日志或私有 transport method
绕过。

## 7. `projection/`：可丢弃的客户端 read model

`projection/` 保存：

```text
ProjectionStore
├─ SessionId → SessionProjection
└─ ThreadId  → ThreadProjection
```

`SessionProjection` 包含 canonical `Session` snapshot、durable sequence、订阅状态和局部 UI
metadata。`ThreadProjection` 包含 canonical `Thread` snapshot、durable sequence、transient
buffer、stream cursor 和订阅状态。

这里可以有“应用 update 到显示投影”的小型 reducer，但它不是 Core reducer：

- 输入只允许 typed snapshot/update；
- 不执行业务校验、持久化或副作用；
- 不补造缺失 event；
- 不推断未收到的 Turn/Item 终态；
- 遇到未知 update、sequence 空洞或不一致时进入 `Desynced` 并重新订阅。

### 7.1 Durable sequence

Session 与每个 Thread 的 durable sequence 必须分别跟踪。收到 committed update 时：

1. `durableSequence <= localSequence`：作为重复 delivery 忽略；
2. `durableSequence == localSequence + 1`：应用到对应 projection；
3. `durableSequence > localSequence + 1`：停止增量合并并重新 subscribe；
4. update identity 与当前 aggregate 不一致：视为协议错误，不路由到当前视图。

subscribe result 必须作为一个完整的 resync package 处理：

1. 校验 snapshot identity，以及返回 gap 在 `afterSequence` 之后连续；
2. 直接把 snapshot 安装为当前 canonical 基线；
3. 不把已经包含在 snapshot 中的 gap event 再应用一次；
4. 丢弃已经排队且 sequence 不大于 snapshot sequence 的重复 notification；
5. 从 snapshot sequence 开始继续应用新的连续 live notification。

这套规则应由一个共享的 projection helper 实现，不能由每个 feature 各写一遍。

### 7.2 Transient stream cursor

transient update 只影响低延迟显示：

- cursor 只在同一 `streamInstanceId` 内连续；
- `streamInstanceId` 改变时清空旧 transient buffer；
- cursor 出现空洞时丢弃不可信 transient 内容并重新同步；
- committed Item 到达后替换相同 Item 的 transient 版本；
- transient 文本永远不能决定 Turn completed、failed 或 interrupted。

## 8. `conversation/`：所有 Thread 共用的交互机制

`conversation/` 只保存每条 Thread 都需要的机制：

- composer、textarea、paste 和输入历史；
- Turn input 提交、interrupt 意图和 draft restore；
- transcript item model；
- Markdown、diff、table 和 wrapping；
- Agent message、reasoning、plan、tool call/result 的流式展示；
- committed Item 与 transient Item 的视觉合并。

选择 `conversation/` 而不是 `chat/`，是因为 Zeta 的 Thread transcript 不只有 User/Agent
message，还包含 Turn、reasoning、plan 和 tool lifecycle。

transcript 的核心 trait 只描述布局、渲染和稳定 identity。特定业务 Item 的交互行为应由
对应 feature adapter 提供，不能不断扩大一个总括式 `history_cell`。

`conversation/` 禁止：

- 直接调用 JSON-RPC；
- 保存 Session membership 或 Thread lineage；
- 决定 tool 是否允许执行；
- 根据展示文本判断 Turn 终态；
- 包含 session picker、config popup 等完整业务界面。

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

一个通用 picker 可以在 `ui/picker/`，但“恢复哪个 Session”的 state、command 和 row model
属于 `features/session_browser/`。

## 10. `terminal/`：真实终端基础设施

`terminal/` 负责：

- raw mode、alternate screen 和 bracketed paste；
- Crossterm event 读取；
- Ratatui backend 和 frame scheduling；
- terminal resize、reflow、cursor 和 scrollback；
- terminal capability 探测和控制序列；
- panic、错误和正常退出时恢复终端。

`TerminalSession` 必须使用 RAII 恢复 raw mode、alternate screen、paste mode 和 cursor。
启动中途任一步失败，也必须回滚之前成功启用的能力。

`terminal/` 不知道产品 Session、Thread、Turn、feature 或 App Server。它可以产生
`TerminalEvent`，但不能发送 `turn/start`、打开 approval popup 或根据 Agent 状态决定文案。

## 11. `features/`：Zeta 功能的垂直切片

一个完整 feature 可以包含：

```text
features/<name>/
├── mod.rs
├── state.rs
├── event.rs
├── command.rs
├── view.rs
├── transcript.rs
└── <name>_tests.rs
```

不是每个 feature 都需要全部文件。`command.rs` 表达交给 `app/` 执行的 typed 意图，不建立
`service.rs` 或直接持有 client。

当前 App Server contract 可以支持的首批 feature：

| Feature | 职责 |
| --- | --- |
| `session_browser` | list、create、resume、complete、archive Session |
| `thread_navigation` | create、fork、切换、archive Thread，展示 lineage |
| `config` | typed config read/update UI |
| `resources` | metadata、分块读取、校验、release 和内容展示 |
| `status` | connection、subscription、Turn 和错误状态的产品化呈现 |

Turn submit/interrupt 和 transcript 属于所有 Thread 的共同机制，先留在
`conversation/`；当某种 Turn 工作流拥有独立状态和 UI 时再提取 feature。

以下能力不能仅因为 Codex TUI 已经存在就提前创建：

- approval 与 request-user-input；
- model、skills、plugins 和 connectors；
- MCP、hooks、goals、usage 和 review；
- file search、feedback、updates 和 visualization；
- sub-agent 或 side conversation 导航。

这些能力必须先进入 Zeta canonical domain 和 App Server API，具备 typed
request/response/notification、顺序、取消、错误与恢复语义，然后 TUI 才添加对应垂直切片。
例如 approval 在 App Server 尚未提供 server-to-client request 和 typed response 前，TUI
不能靠检查 ToolCall 名称或 arguments JSON 自行弹窗并决定策略。

Feature 之间不能依赖彼此的私有模块。共享机制上移到稳定核心；跨功能协作通过
`AppEvent`、`AppCommand` 或公开的小型 value type 完成。

## 12. `host/`：窄宿主能力

`host/` 只放非终端 OS adapter，例如 clipboard、external editor、desktop notification 和
IDE IPC。每个模块必须暴露窄能力，不能形成一个无所不包的 `PlatformService` 或
`HostContext`。

职责按“何时”与“如何”拆开：

```text
features/status：何时通知用户
host/notification：如何调用某个 OS 通知后端

conversation：何时复制或打开编辑器
host/clipboard、host/external_editor：如何访问宿主能力
```

宿主 adapter 不得反向依赖 conversation、projection 或 feature workflow。

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
当前焦点和 keymap 转换为用户意图，再进入业务 command。

单个 event-loop iteration 应有界，长请求、resource 读取和昂贵 Markdown layout 不得阻塞
terminal event pump。重绘可以合并，但 committed update、输入和退出事件不能因为 frame
throttle 丢失。

## 14. 公共 API 与 CLI 所有权

`zeta-tui` 的公开 API 应保持很小：

```rust
pub async fn run(
    start: AppServerStartOptions,
    options: TuiOptions,
) -> Result<TuiExit, TuiError>;
```

`run` 通过 `AppServerSession::start(start)` 获得 request handle 与 event stream，并在退出路径
显式等待 `shutdown()`。它不接受一个生命周期不明确的裸 transport/client。

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
- initialize 并校验 schema hash；
- 构造 `TuiOptions` 和已初始化 client；
- 把 `TuiExit` 映射为进程退出码。

TUI 负责 terminal 打开后的交互，不应反向读取 CLI arguments 或自行打开本地 App Server。

## 15. 依赖规则

| 模块 | 可以依赖 | 禁止依赖 |
| --- | --- | --- |
| `app` | client、projection、conversation、features、ui、terminal、host | Core、Storage、具体 widget 实现、JSON-RPC 字符串 |
| `client` | app-server-client、app-server-protocol、内部 value type | Ratatui、view、Core、私有 wire DTO |
| `projection` | canonical protocol model/update | Ratatui、client transport、Core reducer、feature popup |
| `conversation` | projection read model、ui、canonical Item value | JSON-RPC、Session lifecycle、tool policy |
| `ui` | Ratatui 和纯展示 value | 产品 ID、App Server、业务状态 |
| `terminal` | Crossterm、Ratatui backend、纯 terminal value | AppEvent、产品 ID、feature |
| `features` | projection、conversation、ui、canonical value | 其他 feature 私有模块、Core、任意 RPC |
| `host` | 窄 OS library | app、conversation、projection、feature workflow |

整体方向：

```text
app
├─ client ─────────────────────► zeta-app-server-client
├─ projection ─────────────────► canonical protocol values
├─ conversation ───────────────► projection + ui + narrow host adapters
├─ features ───────────────────► projection + conversation + ui
├─ ui ─────────────────────────► Ratatui
├─ terminal ───────────────────► Crossterm + Ratatui backend
└─ host ───────────────────────► narrow OS libraries
```

`client` 定义 `ClientOperation`，feature 定义自己的公开 intent，`app` 负责把 intent 转成
operation，因此不形成 `app ↔ feature` 或 `app ↔ client` 的反向依赖。view 产生 Ratatui
render data，由 `app` 在 terminal frame 中装配；conversation、feature 和 ui 都不依赖
terminal。

禁止新增含义模糊的 `runtime`、`service`、`common` 或 `platform` 聚合层。共享代码必须根据
其真正职责进入 projection、conversation、ui、terminal 或某个窄 host module。

## 16. 当前实现与目标差距

当前 `zeta-rs/tui/src/` 是保持同步行为的 bootstrap 分层：

```text
app.rs
chatwidget/
└── mod.rs
clipboard.rs
toppane/
├── mod.rs
├── attachments.rs
├── chat_composer.rs
├── pending_pastes.rs
├── slash_command_popup.rs
├── slash_input.rs
├── slash_commands.rs
└── textarea.rs
render/
├── mod.rs
├── header.rs
├── history.rs
├── composer.rs
├── slash_command_popup.rs
├── footer.rs
├── layout.rs
└── theme.rs
terminal.rs
lib.rs
```

它已经满足以下正确边界：

- 通过 `zeta-app-server-client` 的 typed method 工作；
- 明确声明权威 Thread/Turn 状态留在 App Server 后面。
- `App` 处理全局状态/键，`ChatWidget` 协调 transcript 与 sibling `TopPane`；
- `ChatComposer` 协调提交、popup keys、range completion application 与 structured local
  command dispatch，`SlashInput` 解释 cursor 下的 slash composer text，`TextArea` 拥有 UTF-8
  编辑状态、原子 command element 和未来 Vim keymap 边界；
- bracketed paste 使用独立事件路径；超过 1000 个 Unicode scalar value 的内容由 `PendingPastes`
  绑定到 `TextArea` 原子占位符，并在提交前展开；
- 粘贴 PNG/JPEG/GIF/WEBP 本地文件路径会由 `Attachments` 立即读取并绑定为 `[Image #N]`
  原子占位符；提交保持 text/image 顺序并通过 typed `turn/start` 进入 durable Thread history；
- `Ctrl-V` 产生独立 `PasteImage` action，由 native clipboard adapter 读取文件列表或 RGBA 位图、
  编码 PNG，并复用 `Attachments` 的校验、占位符与结构化提交；
- `FileSearchManager` 通过 `zeta-file-search::PathSearchHandle` 在后台增量遍历 workspace 并
  使用完整 `nucleo` engine 更新 `@token` fuzzy results；`Mentions` 只拥有 token/popup 状态、
  高亮、keyboard/mouse selection 和原子文本路径 completion。旧 query snapshot 会在 manager
  和 popup 边界被丢弃；两者都不读取候选文件内容，也不构造结构化 app/plugin Mention；

但目前仍是同步最小实现：

- 每次启动创建新的 Session 和 root Thread；`/new`、`/clear`、`/fork` 与
  `/resume <session-id>` 可以切换当前 conversation，但尚无 picker/browser；
- `ChatWidget` 只保存扁平 `Vec<Message>`，没有 canonical projection；
- `turn/start` 执行期间暂停输入；
- 请求结束后一次性 drain notification；
- 没有 Session/Thread subscribe、gap detection 或 resync；
- 只提取最终 AgentMessage，忽略 Turn/Item 的完整 typed lifecycle；
- 没有 archive UI、resume picker 或多 Thread navigation surface；interrupt、exact-ID resume
  与当前 Thread fork 已接通 typed API；
- local slash popup 已支持共享 validated registry、cursor-aware prefix filtering、保留 argument
  tail 的 range completion、keyboard/mouse selection、原子 command token，以及 inline
  text/image/large-paste arguments；App Server 的 `initialize.slashCommands` snapshot 会在创建
  Session 前合并进 registry，非法名称、空描述、重复项和 built-in shadowing 都会使启动失败；
  dynamic command 恢复完整 `/name` 与 ordered arguments 后作为普通 Turn input 提交。
  Built-in registry 只保留真实执行流：Session/Thread lifecycle、status/config/MCP/Skill 查询、
  revision-checked model selection、help 与退出；缺少 backend contract 的命令不显示。
  workspace file mention popup 也支持
  keyboard/mouse selection，两者之外的 mouse surface 尚未接通；结构化 app/plugin Mention
  仍无 catalog 与执行流；
- 图片输入已形成“本地路径/系统 clipboard → data URL → `UserImage` → provider image block”
  的 vertical slice；native clipboard 在远程 SSH/tmux 环境尚无 terminal-mediated fallback，
  data URL 也会放大 command receipt、Thread store 与 snapshot，长期仍需 resource/blob 引用
  contract；
- `lib.rs` 同时承担 public API、启动编排、Turn 请求执行和通知解释；built-in product commands
  已抽到 `slash_command_dispatch.rs`。

这些限制可以作为 bootstrap 阶段存在，但不应在其上继续堆叠 feature。

## 17. 演进顺序

### 阶段一：建立边界

1. 将 terminal RAII 移入 `terminal/session.rs`；
2. 将纯 TUI state/event/command 移入 `app/`；
3. 将 typed request 与 notification 适配移入 `client/`；
4. 建立 `projection/`，用 Session/Thread snapshot 替换扁平 message authority；
5. 随 projection 落地，把 bootstrap `chatwidget/` 与 `toppane/` 迁入 `conversation/`，保留
   `ChatWidget → TopPane → ChatComposer → TextArea` 的局部 ownership；
6. 保持现有同步行为和 public `run` API 可用。

### 阶段二：订阅与恢复

1. 使用 `session/subscribe` 和 `thread/subscribe`；
2. 实现 durable gap、duplicate、aggregate mismatch 和 resync；
3. 实现 transient cursor 和 committed Item 替换；
4. 把稳定 `CommandId` 生命周期集中到 `client/command_id.rs`；
5. 增加 reconnect 后重新订阅，禁止自动重放结果未知的新副作用。

### 阶段三：核心交互

1. Session list/resume；
2. Thread create/fork/switch/archive；
3. Turn start/interrupt；
4. 完整 ThreadItem transcript；
5. resize/reflow、scroll、selection、copy 和 composer history。

### 阶段四：垂直功能

按已接受的 App Server contract 逐个添加 config、resources、approval 等 feature。每个 feature
同时交付 state、typed command、view、错误/恢复行为和测试，不采用先建一个全局
`services/` 再逐步塞逻辑的方式。

不进行一次性 `git mv` 大重排。每一步都应保持 crate 可编译、现有入口可运行，并让新 owner
和测试一起迁移。

## 18. 测试

测试按 owner 放置：

- 新单元测试模块使用 sibling `*_tests.rs` 和显式 `#[path = "..._tests.rs"]`；
- projection 测试覆盖连续 update、重复 delivery、durable gap、runtime 切换和 resync；
- client 测试使用 fake/mock transport 验证 typed request、稳定 CommandId 和错误映射；
- conversation 测试覆盖 transient/committed 合并、Unicode width、Markdown 和 resize；
- terminal 测试覆盖部分初始化失败与 Drop 恢复；
- feature 测试覆盖 key intent → command → result event → view state；
- crate 级 `tests/` 覆盖 create/resume/fork/interrupt 和 subscription recovery；
- Ratatui `TestBackend` 与 snapshot 只验证稳定布局，不替代状态断言。

测试支持代码也按 owner 拆分。只有确实跨多个模块且 API 稳定的 fixture 才进入 crate 级
`tests/support/`，不能建立全局 `test_utils.rs` 杂物箱。

Rust 模块目标保持在 500 行以内；文件接近 800 行时，新功能必须进入新模块。新增 public
trait 必须有 doc comment，说明其职责、实现约束和调用方预期。

## 19. 验收

- TUI crate 不直接依赖 Core、Storage、Exec、Sandbox 或 Model Provider；
- CLI 向 TUI 传入已初始化 typed client，TUI 不自行建立第二套 composition root；
- Session/Thread projection 可以从 snapshot 完整重建；
- durable sequence 和 transient cursor 分开处理；
- sequence gap、runtime 切换和未知 update 会触发 resync，而不是猜测状态；
- `expectedSequence` 来自正确 aggregate，逻辑重试复用原 `CommandId`；
- transcript 展示完整 typed Turn/ThreadItem lifecycle；
- Ctrl-C 在 Turn 运行时优先发出 `turn/interrupt`，空闲时才退出；
- terminal 在正常退出、错误和 panic 路径均可恢复；
- feature 只通过已接受的 App Server contract 工作；
- UI 原语、terminal 基础设施和 host adapter 不依赖产品状态；
- 单元、projection、transport、render 和端到端测试覆盖主要恢复路径。
