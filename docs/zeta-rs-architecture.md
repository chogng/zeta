# zeta-rs 产品内核与统一对外层

> 当前基线：[Zeta 长期架构](zeta-code-architecture-codex-style-v2.md) 与
> [App Server API](zeta-app-server-api.md)。
> 各 crate 的当前 public/private interface、调用图与修改路径以对应 README 为准。

## 快速理解

`zeta-rs` 是完整的 Rust 产品内核，Core 只是其中的执行控制面；协议、存储、配置、模型、工具和
产品接口各有独立边界，不能都塞回 Core。

| 想知道什么 | 直接答案 | 从哪里继续 |
| --- | --- | --- |
| 哪些状态由 Rust 权威拥有？ | Session、Thread、Turn、工具生命周期、配置与持久化事实 | [核心](#4-核心) |
| Desktop 和 CLI 如何调用？ | 统一经过 App Server API，不链接 Core 或解析终端输出 | [对外接口](#8-app-server) |
| protocol、Core 和 storage 有什么区别？ | 分别拥有共享语义、状态协调和物理持久化机制 | [Protocol 边界](#3-protocol-边界) |
| 为什么有这么多 crate？ | 按可独立验证的责任拆分，不按功能名称堆成通用 service | [Workspace 边界](#2-workspace-边界) |
| 具体函数和修改路径在哪里？ | 进入对应 crate README，系统文档不复制私有实现 | [文档规范](documentation-guidelines.md) |

## 1. Workspace 职责

`zeta-rs/` 是完整 Rust 产品实现，`zeta-rs/core/` 是领域运行时。它负责：

- Session、Thread、Turn、ThreadItem 的 reducer、命令与恢复；
- Agent/model/tool 执行编排；
- typed SessionStore/ThreadStore 与共享 event-stream adapter；
- Config、Credential、sandbox 与 resource；
- App Server、client、transport；
- Rust、TypeScript 与 JSON Schema contract tests。

Desktop UI、Electron IPC、终端渲染和第三方网页 UI 不属于 Rust Core。

## 2. Workspace 边界

```text
zeta-rs/
├── protocol/             # canonical shared domain contract
├── agent-import/         # Codex/Claude known-path discovery + metadata-only import plan
├── tools/                # target host-side tool types, interfaces and pure adapters
├── shell-command/        # concrete approved-process executor
├── file-system/          # concrete read-only filesystem executor
├── file-search/          # workspace path fuzzy search + CLI
├── file-watcher/         # shared filesystem invalidation hints
├── git/                  # bounded Git repository operations and structured parsing
├── diff/                 # shared bounded line/inline diff mapping for Native and TUI
├── editor/               # Native CodeEditor/DiffEditor/MultiDiffEditor presentation；不拥有文件或产品宿主
├── markdown/             # bounded CommonMark/GFM parsing、layout 与 Native presentation
├── install-context/      # runtime install method, package layout and resource candidates
├── apply-patch/          # concrete validated write executor
├── session-store/        # Session persistence port + envelope
├── thread-store/         # Thread persistence port + envelope
├── core/                 # reducers, coordinators, execution policy and recovery
├── storage/              # one physical event-stream engine + typed adapters
├── rollout/              # local rollout repository + recovery composition
├── rollout-trace/        # read-only export, diagnostics and evaluation artifact
├── app-server-protocol/  # external RPC wire contract + generators
├── app-server-transport/
├── app-server-client/
├── app-server/
├── mcp-server/           # current stdio/HTTP Agent-as-tool App Server adapter
├── config/
├── secrets/              # provider-neutral secret persistence primitives
├── login/                # target interactive account-login control plane
├── codex-app-server/     # target adapter for the external Codex App Server
├── model-provider-config/
├── model-provider/
├── zeta-api/
├── http-client/           # shared proxy/TLS/unary HTTP substrate；stream/WebSocket 尚未实现
├── zeta-client/           # API operation retry 与 SSE framing layer
├── winit/                 # App Server 下方的底层 event-loop/native-window crate
├── wgpu/                  # App Server 下方的底层 GPU/surface crate；不拥有 product identity
├── exec/                  # target headless Agent runner
├── tool-executor/         # target local process execution boundary
├── tui/
└── cli/
```

当前 `exec/` 仍实现 process `ToolExecutor`。它迁移为 `tool-executor/` 后，`exec/` 名称用于
[`exec.md`](exec.md) 定义的 headless Agent runner；迁移完成前不能把目标目录注释理解为现状。

不建立职责含糊的 `common`、`service` 或总括式执行 crate。Agent loop 先在 Core 内按模块
分层；只有具备第二个真实消费者、独立 typed port 与测试 vertical slice 时才提取 crate。

`zeta-git` 当前拥有系统 Git 进程身份、仓库发现、porcelain-v2 快照、分支/远端/历史查询、
类型化暂存/取消暂存/丢弃/提交/获取/拉取/推送以及补丁检查与应用；不拥有实时仓库注册表、
监听状态、App Server 线协议或 Desktop SCM。
Git 状态/修改适配器、工作区范围运行时、监听失效/版本通知与 SCM View 已在各自层实现；
它们不改变 `zeta-git` 的 crate 所有权。当前 API 和失败语义见
[`git/README.md`](../zeta-rs/git/README.md)，跨层状态见 [`git.md`](git.md)。

`zeta-diff` 当前拥有 Native 与 TUI 共享的 bounded text diff：精确 line ending、Myers 行映射、
Unicode 字素级内联范围、Git-style hunk、比较策略、取消和资源上限；不读取文件、不调用 Git，
也不拥有 Editor/TUI presentation。当前 API、失败和修改影响见
[`diff/README.md`](../zeta-rs/diff/README.md)。

`zeta-editor` 当前拥有 Native 使用的多行编辑、caret/selection、undo/redo、IME/syntax
projection、代码视口绘制、复用两个 CodeEditor pane 的 side-by-side DiffEditor，以及纵向组合
多个文件 section 的 MultiDiffEditor；它依赖
`zeta-ui` 和 `zeta-diff`，但不依赖 `zeta-native`，也不拥有文件 Tab、平台事件、EditorHost 或
TUI presentation。当前 API、接入义务和限制见
[`editor/README.md`](../zeta-rs/editor/README.md)。

`zeta-markdown` 当前拥有有资源上限的 CommonMark/GFM parsing、只读文档 snapshot、富文本与
block layout 和 Native presentation，并消费 `zeta-ui::ScrollState`；它依赖 `zeta-ui`，但不依赖
`zeta-native`，也不拥有消息 identity、网络图片、链接激活、平台输入或持久化。当前 API、
信任边界和限制见 [`markdown/README.md`](../zeta-rs/markdown/README.md)。

`zeta-ui::ScrollState`、`ScrollMetrics`、`ScrollView` 与 `ScrollbarController` 提供
domain-agnostic logical-pixel offset、clamp、viewport clip、内容坐标、同源 scrollbar
paint/hit/track-page/thumb-drag geometry，以及 hover/active/fade deadline。MultiDiffEditor
复用这套基座；平台 wheel normalization、pointer capture，以及 Terminal scrollback 的距底部
行偏移、输出增长锚定和 alternate-screen 分流仍由 `zeta-native` 拥有，不能迁入通用 ScrollView。

Direct-provider credential ownership 由 [`model-provider.md`](model-provider.md) 维护；通用 secret
persistence 由 [`secrets.md`](secrets.md) 维护；interactive login control plane 由
[`login.md`](login.md) 维护。Workspace 不创建统一 credential/OAuth crate，也不让 Core、API 或
network client 读取 secret store。

ChatGPT/Codex subscription 通过 [`codex-app-server.md`](codex-app-server.md) 接入：
`zeta-codex-app-server` 启动并适配上游 `codex app-server`，由上游拥有 PKCE、callback、token
persistence/refresh 和 Codex backend compatibility。Zeta App Server 只组合其 redacted login/account
状态；`model-provider` 只选择 injected subscription backend。Zeta 不复制 OAuth 实现，也不读取
`~/.codex/auth.json`。

## 3. Protocol 边界

Canonical 产品模型、command/event/update/request 的分类、ID/cursor 语义、当前缺口和后续迁移
统一由 [`protocol.md`](protocol.md) 维护。本文件只规定 workspace 依赖关系：
`zeta-protocol` 是纯共享值层，Core、store、provider adapter 与 App Server wire 可以依赖它，
它不能反向依赖这些执行或 I/O crate。

工具 host contract、registry/binding、executor interface、MCP/dynamic conversion、tool search、
Plugin discovery、code mode 与图片精度由 [`tools.md`](tools.md) 维护。`zeta-tools` 复用 protocol
identity/content，不拥有 Core 调度、MCP session、Plugin authority 或 provider wire。

## 4. 核心

Core 的完整 ownership、执行组件、ports、并发与恢复规则统一由
[`core.md`](core.md) 维护。本节只保留 workspace 级约束。

SessionCoordinator 与 ThreadController 都通过纯 reducer 维护可重建 projection：

```text
stored event + previous snapshot → next snapshot
```

live commit 与 recovery 必须调用同一 reducer。副作用顺序固定为：

```text
validate command
→ build typed events
→ append atomic batch
→ update in-memory projection
→ publish update
```

append 失败时不能暴露未提交状态。

SessionCoordinator 只序列化 membership、lineage 与 lifecycle。ThreadController 只序列化一个
Thread 的执行历史。不同 Thread 可并行，不受 Session sequence 阻塞。

## 5. 存储端口与物理存储

`zeta-session-store` 和 `zeta-thread-store` 分别拥有 storage-neutral trait、stored envelope、
typed command receipt 与 atomic batch validator。Core 依赖这些 port，不依赖本地文件实现。
实现细节分别见 [`session-store/README.md`](../zeta-rs/session-store/README.md) 与
[`thread-store/README.md`](../zeta-rs/thread-store/README.md)。

`zeta-storage` 只有一个物理 event-stream engine，统一负责：

- typed JSONL batch；
- format/kind discriminator；
- checksum；
- append + fsync；
- 未终止 tail recovery。

`SessionRolloutStore` 与 `ThreadRolloutStore` 是薄的领域 adapter，不是两套 rollout 引擎。
它们分别选择 stream kind、path、ID 与 batch validator。

`zeta-rollout` 组合上述 adapters 与 writer lease，负责从一个 state root 恢复可运行的
SessionCoordinator；它先恢复 Thread，再恢复 Session 以便继续 create/fork saga。App Server 的本地
composition root 只依赖该 repository，不重复这套恢复流程。
具体打开与恢复顺序见 [`rollout/README.md`](../zeta-rs/rollout/README.md)。

`zeta-rollout-trace` 以两个 store port 为输入，生成只读、可序列化的 Session trace。它适合
诊断、导出和评测，但不是 authority，也不能成为执行输入。它保留独立 Session/Thread
sequence，而不把并发 aggregate 拼成伪全局顺序。trace 可能携带用户输入、工具参数和结果，因此
crate 不提供默认文件写入；持久化或上传必须由调用方显式施加脱敏、访问控制和保留期策略。
实现与 privacy obligation 见
[`rollout-trace/README.md`](../zeta-rs/rollout-trace/README.md)。

当前开发期只读当前 schema。旧记录、implicit Session、kind/payload upcast 和 sidecar ledger
不进入执行路径。

SQLite 仅为可删除、可重建的查询 projection，不是 authority。

## 6. Sequence 与并发

Sequence、cursor、ID 和 optimistic concurrency 的领域语义统一见
[`protocol.md`](protocol.md#5-sequencecursor-与-id)。Workspace 实现必须为每个 aggregate
提供独立 writer lease，使不同 Thread 可以并发且不会占用 Session revision。

Fork 在 Session lineage 中保存 `parentThreadId + parentSequence`。该 parent sequence 是一个
不可变历史锚点，不是另一套物理日志计数。

创建/fork Thread 使用可恢复 saga：

```text
Session plan(creating)
→ Thread create
→ Session attach(active)
```

## 7. 类型化命令重放

Command identity、receipt 和 replay 规则统一见
[`protocol.md`](protocol.md#41-command请求改变状态)。在 Workspace 内，store adapter 负责把
typed receipt 与首个业务 event 原子提交，reducer recovery 恢复稳定结果；Config authority
沿用相同模式。Config、Plugin、MCP 与 Skill 的 authority 分布、snapshot reconcile 和 safe-point
组合由 [`config.md`](config.md) 统一规定。

## 8. App Server

`zeta-app-server-protocol` 直接引用语义完全一致的 canonical Session/Thread/Turn/ThreadItem、
events 和 updates，只为真正 wire-specific 的 params/result/error 定义 DTO。
Registry/generator 实现见
[`app-server-protocol/README.md`](../zeta-rs/app-server-protocol/README.md)。

`zeta-app-server` 负责：

- initialize 与 schema hash gate；
- method dispatch；registry 已声明 Session/Thread serialization scope，但当前同步 server 尚未
  接入 per-scope scheduler；
- connection subscription cursor；
- `session/update` / `thread/update`；
- Resource ownership；
- Core error 到 stable RPC error 的映射。

它不重建旧事件、不运行 reducer、不推断领域状态，也不拥有持久化模型。
当前 dispatch、broker、resource 与 local composition 见
[`app-server/README.md`](../zeta-rs/app-server/README.md)。

## 9. 客户端与传输

```text
zeta-exec / TUI
  → app-server-client::AppServerSession
  → request handle + event stream
  → App Server dispatcher

Desktop
  → generated TypeScript contract
  → JSONL / stdio
  → App Server dispatcher
```

Rust `app-server-client` 是本地 App Server 的共享宿主层：负责启动、initialize、请求/事件
channel wiring 和显式 shutdown，详细边界见
[`app-server-client.md`](app-server-client.md)。进程内 typed channel 是性能优化，不是语义
捷径；Rust 本地路径与 Desktop JSONL 路径都必须经过 typed request/response、initialize、
dispatcher 和 notification contract。长期可增加相同 contract 的 remote App Server backend。

`zeta-mcp-server` 当前通过该 client 将 stdio/Streamable HTTP MCP `zeta` / `zeta-reply` tool
call 映射到 canonical Session/Thread/Turn，并提供 bounded progress、approval/user-input form
interaction 与 principal-scoped durable invocation recovery。它是外层 adapter，不直接依赖
Core 或 stores；HTTP security、SSE recovery、remote App Server 和 remote Agent bridge 边界见
[`mcp-server.md`](mcp-server.md)。

产品 Session、App Server connection session 与 terminal session 是三种不同生命周期，命名
时必须带领域限定。

## 10. 无界面 exec 与远程执行

`zeta-exec` 是无交互 Agent runner，负责 run-once、机器输出、terminal outcome 和未来 scheduler
worker adapter。它只通过 App Server Client 工作，不依赖 Core、store、provider、sandbox 或
process executor。完整架构见 [`exec.md`](exec.md)。

底层 process execution 从当前同名 crate 迁移为 `zeta-tool-executor`。未来
`zeta-exec-server` 只负责远程 process/PTY/filesystem execution，不能接收 Agent
`turn/start`。Remote Agent scheduling 与 remote process execution 使用不同 protocol、
identity、lease 和 disconnect 语义。

## 11. 验证

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
corepack pnpm --dir desktop test:main
corepack pnpm --dir desktop typecheck:renderer
```

协议变更还必须重新生成并提交 JSON Schema、TypeScript 与 Desktop 同步产物。
