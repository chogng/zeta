# Zeta Code：CLI + Desktop 统一架构方案

> **文档状态：历史统一基线。** 团队协作入口已经拆分为
> [Desktop](zeta-desktop-architecture.md)、
> [CLI](zeta-cli-architecture.md)、
> [zeta-rs](zeta-rs-architecture.md) 和
> [App Server API v1](zeta-app-server-api-v1.md)、
> [API 接口文档规范](zeta-api-interface-requirements.md)。
> 后续职责与接口交付以拆分文档和已接受的 API 契约为准。

> 版本：0.3  
> 日期：2026-07-24  
> 状态：建议基线  
> 目标：Rust workspace 与 app-server 分层，让 Zeta CLI、TUI、Electron Desktop、未来 IDE 插件和后台服务共享同一套 Rust 产品内核，同时保持各客户端独立演进。

---

## 1. 核心结论

Zeta 顶层应按“产品技术域”划分，而不是使用一个泛化的 `crates/` 收纳所有 Rust crate：

```text
zeta/
├── zeta-rs/       # 完整 Rust 产品工作区：Core、CLI、TUI、App Server、协议、工具
├── desktop/       # Electron 富客户端
├── packages/      # 可选的 TypeScript SDK、UI 公共包
├── schemas/       # 生成的 JSON Schema 等协议产物
├── docs/
└── scripts/
```

`zeta-rs` 是广义上的 Zeta Rust 核心能力区；其中 `zeta-rs/core` 才是狭义的领域内核。

```text
广义 Rust 产品核心：zeta-rs/
狭义领域核心：    zeta-rs/core/
```

`app-server`、`app-server-protocol`、`cli`、`tui`、`protocol` 与 `core` 都应当是 `zeta-rs` Cargo workspace 下的同级 crate。

Desktop 不直接调用 CLI，也不解析 CLI 的终端输出；Desktop 启动或连接：

```bash
zeta app-server --listen stdio://
```

产品入口关系：

```text
                    zeta-rs/core
                   /            \
                  /              \
          zeta-rs/cli      zeta-rs/app-server
               |                    |
             TUI/CLI           JSON-RPC / stdio
                                    |
                              Electron Desktop
```

浏览器由 Electron Main 管理。Rust Agent 通过 app-server 的双向请求调用 Browser Capability，而不是让 Core 直接依赖 Electron 或 CDP。

---

## 2. 为什么采用 `zeta-rs/` 产品边界

`zeta-rs` 不只是一个普通 crate 文件夹，而是整个 Rust 产品运行时：

```text
zeta-rs/
├── 领域模型与 Agent 内核
├── Thread / Turn / Item 协议
├── CLI 与 TUI
├── App Server 与客户端协议
├── 配置、认证与状态存储
├── 工具执行、文件操作与搜索
├── 沙箱、审批与安全策略
└── 通用 Rust 工具库
```

这种组织方式有三个直接收益。

第一，Rust 能力形成一个完整、独立、可单独构建和测试的 workspace。Electron 与前端工具链不会污染 Cargo workspace 的边界。

第二，CLI、TUI、app-server 都被视为正式产品入口，而不是某个 Desktop 项目的内部 sidecar。

第三，未来增加 IDE 插件、远程客户端或 daemon 时，只需消费 app-server 协议，不需要直接链接 `zeta-core`。

---

## 3. 总体架构

```text
┌───────────────────────────────────────────────────────────────┐
│                         zeta-rs                               │
│                                                               │
│  Composition Roots                                             │
│  ┌─────────────┐                         ┌──────────────────┐   │
│  │  CLI / TUI  │                         │    App Server    │   │
│  └──────┬──────┘                         └───────┬──────────┘   │
│         └────────────────┬───────────────────────┘              │
│                          ▼                                      │
│                ┌──────────────────┐                             │
│                │ Zeta Core        │──→ internal protocol       │
│                │ use cases + ports│                             │
│                └────────┬─────────┘                             │
│                         ▲ ports implemented by                  │
│       ┌─────────────────┼────────────────────────────┐          │
│       │                 │                            │          │
│    Storage       Exec / Sandbox / Tools      Model / Credential│
│                                                               │
│  App Server ──→ app-server-protocol + transport                │
└────────────────────────────────┬───────────────────────────────┘
                                 │ JSON-RPC / stdio / socket
┌────────────────────────────────▼───────────────────────────────┐
│                       Electron Desktop                         │
│                                                                │
│  Main Process                                                   │
│  ├── AppServerClient                                            │
│  ├── BrowserManager / CDP                                       │
│  ├── WindowManager                                              │
│  └── Security Gateway                                           │
│           │                                                     │
│       Preload                                                   │
│           │                                                     │
│       Renderer                                                  │
│       ├── UI Command Registry                                   │
│       ├── View Stores                                           │
│       └── React/Vue/Svelte                                      │
└────────────────────────────────────────────────────────────────┘
```

上图的运行时调用方向不等于 Cargo 依赖方向：Core 调用 port，但具体 adapter 在编译时依赖
Core 定义的 trait；CLI 与 App Server 负责同时依赖两者并完成注入。完整 Cargo 依赖方向见
第 20 节。

---

## 4. 仓库目录规划

```text
zeta/
├── README.md
├── LICENSE
├── package.json
├── pnpm-workspace.yaml
├── tsconfig.base.json
├── rust-toolchain.toml
│
├── zeta-rs/
│   ├── Cargo.toml
│   ├── Cargo.lock
│   │
│   ├── core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── thread_manager.rs
│   │       ├── error.rs
│   │       ├── agent.rs
│   │       ├── agent/
│   │       │   ├── service.rs
│   │       │   ├── state.rs
│   │       │   ├── planner.rs
│   │       │   ├── tool_loop.rs
│   │       │   └── cancellation.rs
│   │       ├── thread.rs
│   │       ├── thread/
│   │       │   ├── model.rs
│   │       │   ├── manager.rs
│   │       │   └── repository.rs
│   │       ├── turn.rs
│   │       ├── turn/
│   │       │   ├── model.rs
│   │       │   ├── runner.rs
│   │       │   └── state.rs
│   │       ├── item.rs
│   │       ├── item/
│   │       │   ├── model.rs
│   │       │   └── stream.rs
│   │       ├── tools.rs
│   │       ├── tools/
│   │       │   ├── registry.rs
│   │       │   ├── executor.rs
│   │       │   └── approval.rs
│   │       ├── workspace.rs
│   │       ├── workspace/
│   │       │   ├── model.rs
│   │       │   ├── service.rs
│   │       │   └── index.rs
│   │       ├── capabilities.rs
│   │       ├── capabilities/
│   │       │   ├── browser.rs
│   │       │   ├── user_input.rs
│   │       │   └── host.rs
│   │       ├── events.rs
│   │       └── events/
│   │           └── publisher.rs
│   │
│   ├── protocol/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── thread.rs
│   │       ├── turn.rs
│   │       ├── item.rs
│   │       ├── tool.rs
│   │       ├── event.rs
│   │       ├── approval.rs
│   │       └── ids.rs
│   │
│   ├── app-server-protocol/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── common.rs
│   │       ├── v1.rs
│   │       ├── v1/
│   │       │   ├── initialize.rs
│   │       │   ├── account.rs
│   │       │   ├── config.rs
│   │       │   ├── thread.rs
│   │       │   ├── turn.rs
│   │       │   ├── item.rs
│   │       │   ├── browser.rs
│   │       │   ├── approvals.rs
│   │       │   ├── resources.rs
│   │       │   ├── requests.rs
│   │       │   ├── notifications.rs
│   │       │   └── errors.rs
│   │       └── schema.rs
│   │
│   ├── app-server-transport/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── connection.rs
│   │       ├── jsonl.rs
│   │       ├── stdio.rs
│   │       └── socket.rs             # 阶段 6 再实现
│   │
│   ├── app-server/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── server.rs
│   │       ├── dispatcher.rs
│   │       ├── message_processor.rs
│   │       ├── session.rs
│   │       ├── outbound_requests.rs
│   │       ├── notification_sink.rs
│   │       ├── capability_registry.rs
│   │       ├── resource_store.rs
│   │       ├── handlers.rs
│   │       └── handlers/
│   │           ├── initialize.rs
│   │           ├── thread.rs
│   │           ├── turn.rs
│   │           ├── config.rs
│   │           └── account.rs
│   │
│   ├── app-server-client/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── client.rs
│   │       ├── connection.rs
│   │       ├── subscriptions.rs
│   │       └── error.rs
│   │
│   ├── cli/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── lib.rs
│   │       ├── args.rs
│   │       ├── commands.rs
│   │       ├── commands/
│   │       │   ├── ask.rs
│   │       │   ├── exec.rs
│   │       │   ├── login.rs
│   │       │   ├── config.rs
│   │       │   └── app_server.rs
│   │       ├── output.rs
│   │       └── output/
│   │           ├── human.rs
│   │           ├── json.rs
│   │           └── jsonl.rs
│   │
│   ├── tui/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── app.rs
│   │       ├── chat.rs
│   │       ├── input.rs
│   │       ├── events.rs
│   │       └── render.rs
│   │
│   ├── config/
│   ├── storage/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── rollout.rs
│   │       ├── rollout/
│   │       │   ├── reader.rs
│   │       │   ├── writer.rs
│   │       │   └── recovery.rs
│   │       ├── state_db.rs
│   │       ├── state_db/
│   │       │   ├── migrations.rs
│   │       │   └── projection.rs
│   │       └── lease.rs
│   ├── credentials/
│   ├── exec/
│   ├── built-in-tools/
│   ├── file-search/
│   ├── file-watcher/
│   ├── git-utils/
│   ├── model-provider/
│   ├── sandboxing/
│   └── utils/
│       ├── absolute-path/
│       ├── image/
│       ├── jsonl/
│       ├── stream-parser/
│       └── task/
│
├── desktop/
│   ├── package.json
│   ├── electron-builder.yml
│   ├── vite.config.ts
│   ├── src/
│   │   ├── main/
│   │   │   ├── index.ts
│   │   │   ├── lifecycle/
│   │   │   │   ├── desktop-lifecycle.ts
│   │   │   │   └── shutdown.ts
│   │   │   ├── app-server/
│   │   │   │   ├── app-server-process.ts
│   │   │   │   ├── app-server-client.ts
│   │   │   │   ├── json-rpc-connection.ts
│   │   │   │   ├── capability-host.ts
│   │   │   │   ├── resource-store.ts
│   │   │   │   └── protocol-version.ts
│   │   │   ├── browser/
│   │   │   │   ├── browser-manager.ts
│   │   │   │   ├── browser-view.ts
│   │   │   │   ├── browser-session.ts
│   │   │   │   ├── cdp-client.ts
│   │   │   │   ├── page-observer.ts
│   │   │   │   ├── page-actions.ts
│   │   │   │   ├── pdf-capture.ts
│   │   │   │   └── permissions.ts
│   │   │   ├── windows/
│   │   │   │   ├── window-manager.ts
│   │   │   │   ├── main-window.ts
│   │   │   │   └── layout.ts
│   │   │   ├── ipc/
│   │   │   │   ├── renderer-gateway.ts
│   │   │   │   ├── sender-validation.ts
│   │   │   │   ├── event-forwarder.ts
│   │   │   │   └── handlers/
│   │   │   │       ├── thread.ts
│   │   │   │       ├── turn.ts
│   │   │   │       ├── browser.ts
│   │   │   │       └── window.ts
│   │   │   └── security/
│   │   │       ├── navigation-policy.ts
│   │   │       ├── permission-policy.ts
│   │   │       └── origin-policy.ts
│   │   ├── preload/
│   │   │   ├── index.ts
│   │   │   ├── api.ts
│   │   │   └── events.ts
│   │   └── renderer/
│   │       ├── index.html
│   │       ├── main.tsx
│   │       ├── app/
│   │       ├── commands/
│   │       │   ├── command-registry.ts
│   │       │   ├── command-context.ts
│   │       │   └── command-contributions.ts
│   │       ├── app-server/
│   │       │   ├── proxy.ts
│   │       │   └── hooks.ts
│   │       ├── features/
│   │       │   ├── agent/
│   │       │   ├── browser/
│   │       │   ├── conversation/
│   │       │   ├── workspace/
│   │       │   └── settings/
│   │       ├── shared/
│   │       │   ├── components/
│   │       │   ├── state/
│   │       │   └── utilities/
│   │       └── styles/
│   └── generated/
│       └── app-server/
│           └── v1/
│               ├── types.ts
│               ├── methods.ts
│               └── schema-hash.ts
│
├── packages/
│   ├── app-server-client-ts/     # 多个 TS 客户端复用时再建立
│   └── ui/                       # 有共享 UI 需求时再建立
│
├── schemas/
│   ├── app-server/
│   │   └── v1.schema.json
│   └── protocol-version.json
│
├── scripts/
│   ├── generate-app-server-types.ts
│   ├── build-zeta-rs.ts
│   ├── package-desktop.ts
│   └── verify-protocol.ts
│
└── docs/
    ├── architecture.md
    ├── app-server-protocol.md
    ├── browser-capability.md
    ├── command-system.md
    ├── state-ownership.md
    └── security-model.md
```

Rust 模块使用 `foo.rs` 加 `foo/` 子目录，不新建 `foo/mod.rs`。`lib.rs` 中逐项显式声明
私有模块和公开导出。新增测试模块放在实现文件的同级 `*_tests.rs` 中，并通过显式
`#[path = "..._tests.rs"]` 引入。新 trait 必须用 doc comment 说明职责、实现约束和调用方
预期；公开 API 避免含义不清的 bool 或 `Option` 参数，优先使用 enum、newtype 或命名方法。

第一版按当前已知的稳定边界建立独立 crate。阶段 1 先建立：

```text
zeta-rs/
├── core/
├── protocol/
├── config/
├── credentials/
├── storage/
├── built-in-tools/
├── exec/
├── sandboxing/
├── model-provider/
├── cli/
└── tui/
```

App Server 相关 crate 在阶段 2 直接按 `app-server-protocol`、`app-server-transport`、
`app-server-client` 和 `app-server` 四个 crate 创建。文件搜索、watcher、git-utils 或新的
utility 能力可以等到功能进入开发范围时再创建，但首次实现就必须落在独立 crate 中，
不能先作为 Core、Exec、Config 或 Built-in Tools 的内部模块，之后再迁移。

---

## 5. `protocol` 与 `app-server-protocol` 的区别

这两个 crate 不重复。

### 5.1 `zeta-rs/protocol`

它表示 Zeta 内部可序列化的稳定值、事实和运行事件：

```text
Thread
Turn
Item
ToolCall
Approval
AgentEvent
WorkspaceEvent
```

它不是完整 Domain Layer，不包含 Manager、Repository 实现、可变 aggregate 或状态转换策略。
Thread/Turn/Tool 的不变量和状态机属于 Core；`protocol` 只提供跨 crate 共享所需的 ID、
不可变 Item/Event 和稳定值对象，避免把业务行为做成贫血的公共数据结构。

它可以被以下 crate 复用：

```text
core
cli
tui
app-server
storage
```

它不应该知道 JSON-RPC、Electron、TypeScript 代码生成或客户端版本。

示例：

```rust
pub struct ThreadId(pub String);
pub struct TurnId(pub String);

pub enum Item {
    UserMessage(UserMessage),
    AgentMessage(AgentMessage),
    ToolCall(ToolCall),
    ToolResult(ToolResult),
    FileChange(FileChange),
}
```

### 5.2 `zeta-rs/app-server-protocol`

它表示面向富客户端的外部 RPC 契约：

```text
initialize
thread/start
thread/read
turn/start
turn/interrupt
config/read
browser/observe（server → client）
browser/perform（server → client）
```

它可以依赖 `zeta-protocol`，但只复用经过明确审核的稳定叶子类型：

```text
ThreadId
TurnId
ItemId
ToolCallId
Timestamp
稳定标量与资源标识
```

以下类型必须在 `app-server-protocol/v1` 中定义独立 RPC DTO，并通过显式转换与内部模型互转：

```text
Thread
Turn
Item
ToolCall
Approval
Error
所有状态 enum
```

```text
app-server-protocol
        ↓
    protocol
```

而不能反向：

```text
protocol
    ×
app-server-protocol
```

外部协议常常需要：

- 稳定字段；
- 版本兼容；
- 可选字段；
- 客户端能力协商；
- 数据裁剪；
- TypeScript / JSON Schema 生成；
- 废弃策略。

内部领域模型则可以更自由地演进。

因此，不能因为两个类型当前字段相同就直接公开内部 struct 或 enum。特别是内部 enum
新增 variant、改变默认值或调整字段语义时，不得隐式改变已发布的 RPC 契约。

第一版即使用 `v1` 模块固定 wire shape：

```text
app-server-protocol/
├── common.rs   # 跨版本稳定叶子类型
├── v1.rs
└── v1/         # v1 Params / Result / Notification / Error DTO
```

---

## 6. CLI、TUI 与 App Server 的关系

Core 定义业务状态机、应用服务以及它所消费的 Repository、Model、Tool 和 Host Capability
port。Storage、Model Provider、Exec、Sandbox、Browser Bridge 等具体实现反向依赖这些 port。
第一版不建立 `core-api` 或泛化 `runtime` facade。App Server 是唯一产品用例边界；CLI 和
Desktop 都通过同一版本化协议使用它。

建议发布一个统一的 `zeta` 二进制：

```bash
zeta
zeta ask "解释当前仓库"
zeta exec "修复测试失败"
zeta login
zeta config
zeta app-server --listen stdio://
```

`zeta-rs/cli` 是顶层命令入口，普通业务路径依赖：

```text
tui
app-server-client
app-server-protocol
```

`zeta app-server` 宿主子命令可以依赖 `app-server`。CLI 的普通交互路径使用进程内 client，
但仍经过同一个 dispatcher：

```text
Terminal
  ↓
zeta CLI / TUI
  ↓
InProcess App Server Client
  ↓
App Server dispatcher
  ↓
zeta Core
```

Desktop 路径通过 app-server：

```text
Electron Desktop
  ↓ JSON-RPC
zeta app-server
  ↓
zeta Core
```

Desktop 不应执行：

```bash
zeta ask ...
```

然后解析彩色终端文本。它应直接使用 app-server 的机器协议。

---

## 7. Command 与领域 RPC 的分工

Command 没有被放弃。它只应存在于适合它的层。

### 7.1 Renderer 内部使用 Command Registry

按钮、菜单、快捷键和命令面板都调用同一动作：

```ts
commands.execute("zeta.startTurn");
commands.execute("browser.reload");
commands.execute("workspace.open");
```

注册示例：

```ts
commands.register("zeta.startTurn", async () => {
  const threadId = threadStore.activeThreadId;
  const input = composerStore.consumeInput();

  await appServer.turn.start({
    threadId,
    input,
  });
});
```

它解决的是 UI 入口解耦：

```text
发送按钮 ─┐
快捷键 ───┼─→ zeta.startTurn
命令面板 ─┤
菜单 ─────┘
```

### 7.2 跨进程使用领域化 RPC

UI Command Handler 调用明确的 RPC：

```text
UI Command：zeta.startTurn
         ↓
RPC Method：turn/start
```

不建议跨进程协议只有一个万能入口：

```json
{
  "method": "command/execute",
  "params": {
    "commandId": "turn.start",
    "payload": {}
  }
}
```

更推荐：

```json
{
  "id": 42,
  "method": "turn/start",
  "params": {
    "idempotencyKey": "req_01J...",
    "threadId": "thread_123",
    "input": [
      { "type": "text", "text": "分析当前网页" }
    ]
  }
}
```

这样可以为每个方法生成独立的 Params 和 Result 类型。

### 7.3 Rust Core 内部使用强类型方法

```rust
turn_service.start_turn(params).await?;
thread_manager.resume(thread_id).await?;
```

Core 内部不需要通过字符串命令总线调用本地服务，除非后续实现动态插件命令系统。

---

## 8. App Server 协议模型

### 8.1 初始化握手

客户端连接后必须首先调用：

```json
{
  "id": 1,
  "method": "initialize",
  "params": {
    "clientInfo": {
      "name": "zeta-desktop",
      "version": "0.1.0"
    },
    "protocolVersions": {
      "min": 1,
      "max": 1
    },
    "capabilities": {
      "browser": {
        "version": 1,
        "observe": true,
        "input": true,
        "network": true,
        "pdf": true
      },
      "userInput": true,
      "notifications": true,
      "resources": {
        "version": 1,
        "read": true,
        "maxChunkBytes": 262144
      }
    }
  }
}
```

服务端返回协商后的能力：

```json
{
  "id": 1,
  "result": {
    "serverInfo": {
      "name": "zeta-app-server",
      "version": "0.1.0"
    },
    "protocolVersion": 1,
    "schemaHash": "sha256:...",
    "capabilities": {
      "threads": true,
      "turns": true,
      "browser": {
        "version": 1,
        "tools": true
      },
      "resources": {
        "version": 1,
        "read": true,
        "maxChunkBytes": 262144
      }
    }
  }
}
```

### 8.2 Connection 与 Thread 订阅模型

`threadId` 负责标识业务会话，但 App Server 还需要使用仅存在于服务端内部的
`connectionId` 区分不同客户端连接。`connectionId` 不进入公开协议，也不需要客户端生成。

建议由 `ThreadStateManager` 维护双向索引：

```text
liveConnections
  ConnectionId → ConnectionCapabilities

threads
  ThreadId → ThreadState + Set<ConnectionId>

threadIdsByConnection
  ConnectionId → Set<ThreadId>

turnOwners
  (ThreadId, TurnId) → ConnectionId

capabilityBindings
  CapabilityHandle → ConnectionId

pendingServerRequests
  (ConnectionId, RequestId) → ResponseSender
```

完整标识层次是：

```text
ConnectionId
  └── ThreadId
        └── TurnId
              └── ItemId / ToolCallId / RequestId
```

规则如下：

1. 每个 transport 连接独立执行 `initialize` 并获得服务端内部 `connectionId`；
2. `thread/start`、`thread/resume` 和 `thread/fork` 成功后，自动把当前连接订阅到该 Thread；
3. `thread/read` 只读取持久化快照，不加载 Thread，也不自动订阅；
4. Thread、Turn 和 Item 通知只广播给该 Thread 的订阅连接；
5. 同一 Thread 可以有多个订阅连接，同一连接也可以订阅多个 Thread；
6. 同一 Thread 的状态修改请求按 `threadId` 串行化，不同 Thread 可以并发执行；
7. `thread/resume` 应把“一致性快照响应”和“接入后续实时事件”放在同一 Thread
   listener 顺序中；snapshot 返回最后一个 durable `sequence`，避免读取快照与开始订阅
   之间丢失事件；
8. Server → Client 请求使用 `(connectionId, requestId)` 关联响应。不同连接可以使用相同的
   JSON-RPC `id`，服务端不能只按裸 `requestId` 建立全局索引；
9. 连接断开时，移除它的全部 Thread 订阅，并取消或清理发给该连接但尚未完成的请求；
10. `thread/unsubscribe` 只取消当前连接对指定 Thread 的订阅，不等于删除或归档 Thread。
11. `turn/start` 将新 Turn 绑定到发起请求的连接；审批和用户输入默认回到该 Turn owner；
12. Browser Target 等客户端资源句柄绑定到创建它的连接，资源句柄的 owner 优先于 Turn owner。

Thread 订阅只决定哪些连接接收通知，不自动授予客户端能力的所有权。Server → Client
能力请求默认发送给发起当前 Turn、且声明了对应 capability 的连接；如果请求引用了
Browser Target 等客户端资源句柄，则必须发送给创建该句柄的连接。只有在协议明确允许、
目标连接也确认接管时，才迁移能力请求。

```text
Desktop Connection A ─┐
                      ├─→ Thread 123 ─→ Turn / Item notifications
IDE Connection B ─────┘

Thread 123 outbound capability request
  → Turn owner or CapabilityHandle owner: Connection A
  → ConnectionRequestId(A, 81)
```

### 8.3 Client → Server 方法

建议第一版包含：

```text
initialize
thread/start
thread/read
thread/list
thread/resume
thread/fork
thread/unsubscribe
thread/archive
turn/start
turn/interrupt
config/read
config/update
account/read
account/login
workspace/open
workspace/search
```

会产生持久化副作用的方法必须接受客户端生成的 `idempotencyKey`。JSON-RPC `id` 只关联当前
连接上的 response，不能用于断线重试去重。服务端按 method 与资源 scope 保存幂等结果；
同一个 key 和相同参数返回原结果，同一个 key 配不同参数返回 `IdempotencyConflict`。

第一版至少覆盖：

```text
thread/start
thread/fork
thread/archive
turn/start
config/update
account/login 的状态提交步骤
```

### 8.4 Server → Client 请求

Agent 需要调用客户端能力：

```text
browser/observe
browser/perform
browser/getPdf
userInput/request
approval/request
host/openExternal
```

例如：

```json
{
  "id": 81,
  "method": "browser/observe",
  "params": {
    "threadId": "thread_123",
    "turnId": "turn_456",
    "toolCallId": "tool_789",
    "targetId": "browser_target_abc",
    "timeoutMs": 30000,
    "include": {
      "accessibilityTree": true,
      "domSnapshot": true,
      "screenshot": true,
      "networkSummary": false
    }
  }
}
```

`requestId` 由 JSON-RPC envelope 的 `id` 承担；业务上下文字段用于校验请求仍属于活动的
Thread、Turn 和 Tool Call，不能替代连接内的 JSON-RPC 请求关联。

所有 Server → Client 请求都定义 deadline、取消和迟到响应规则。Turn 完成、中断、owner
连接断开或 deadline 到期时，App Server 清理 pending request，并发送带
`{ threadId, requestId, reason }` 的 `serverRequest/resolved` 通知。客户端收到 resolved
后关闭相应 UI；之后到达的 response 作为 stale response 忽略并记录诊断。

### 8.5 Server → Client 通知

```text
thread/started
thread/updated
turn/started
turn/completed
turn/failed
item/started
item/agentMessage/delta
item/toolCall/delta
item/completed
approval/pending
serverRequest/resolved
system/warning
```

流式回答示例：

```json
{
  "method": "item/agentMessage/delta",
  "params": {
    "threadId": "thread_123",
    "turnId": "turn_456",
    "itemId": "item_789",
    "streamSeq": 14,
    "delta": "当前页面主要说明了"
  }
}
```

对应 Rollout 记录的 durable 通知携带 `eventId` 和 Thread `sequence`。流式 delta 尚未形成
durable Item 时使用 Item 内单调递增的 `streamSeq`。Renderer 发现 sequence 或 streamSeq
空洞时停止继续归并该实体，并通过 `thread/read` 或 `thread/resume` 获取权威 snapshot。
第一版不要求提供任意时间范围的通知 replay 服务。

### 8.6 双向资源方法

Rust 和客户端都可能生产大对象，因此 Resource RPC 是对称能力。`ResourceRef` 的创建方是
owner，另一端只能通过 owner connection 调用：

```text
resource/metadata
resource/read
resource/release
```

Resource 方法不属于某个 Thread 的业务事件，但创建记录应关联可选的 Thread、Turn 和
Tool Call 审计上下文。资源句柄不能跨连接复用或在 capability 未协商时发送。

---

## 9. 浏览器能力边界

浏览器由 Electron Main 持有：

```text
Electron Main
├── WebContentsView
├── Session / Cookie
├── CDP debugger
├── 下载与权限
└── PDF 网络捕获
```

Rust Core 只定义抽象能力：

```rust
/// Host-provided browser operations used by the Agent.
///
/// Implementations translate these semantic requests into a local or remote browser backend while
/// enforcing the host's target ownership, security policy, deadlines, and cancellation behavior.
#[async_trait]
pub trait BrowserCapability: Send + Sync {
    async fn observe(
        &self,
        request: BrowserObserveRequest,
    ) -> Result<BrowserObservation, BrowserError>;

    async fn perform(
        &self,
        action: BrowserAction,
    ) -> Result<BrowserActionResult, BrowserError>;

    async fn get_pdf(
        &self,
        request: GetPdfRequest,
    ) -> Result<PdfResource, BrowserError>;
}
```

Desktop 中的实现链路：

```text
Rust Core
  ↓ BrowserCapability
App Server outbound request
  ↓ JSON-RPC
Electron Main CapabilityHost
  ↓
BrowserManager / CDP
```

Electron Main 为每个 Browser View 创建不可伪造的 `BrowserTargetHandle`，并通过协议中的
`targetId` 传递其稳定表示。App Server 将目标句柄绑定到提供该能力的连接；另一个连接不能
使用这个句柄操作浏览器。

`"active"` 只能作为单 Browser Target 模式下的输入便利值。存在多个 Tab 时，应在 Tool Call
开始前把 `"active"` 解析成具体句柄，并在整个 Tool Call 期间保持不变。目标关闭后返回
`BrowserTargetUnavailable`，不能静默切换到另一个当前活动 Tab。

CLI 直接调用 Core 且没有浏览器能力时，可以：

- 在 App Server 组合阶段安装显式的 `UnsupportedBrowserCapability`；
- 返回 `CapabilityUnavailable`；
- 后续支持外部 Chrome CDP；
- 后续支持远程 Browser Service。

如果 CLI 作为 app-server 客户端运行，则在 `initialize` 中声明 `browser = false`。

不能让 `zeta-core` 直接依赖 Electron 类型或 `webContents.debugger`。

---

## 10. 状态所有权

### 10.1 运行时状态

| 状态 | 权威持有者 | Desktop 角色 |
|---|---|---|
| Thread、Turn、Item | Rust Core | 订阅和展示投影 |
| Agent 运行状态 | Rust Core | 展示、取消、审批 |
| 工具调用状态 | Rust Core | 展示、用户确认 |
| 普通配置 | Rust Core / Config | 编辑表单与调用 RPC |
| 认证状态与长期凭据 | Rust Core / Credential Store | 触发登录、展示脱敏状态 |
| 工作区与索引 | Rust Core | 展示搜索结果 |
| Browser View、Tab、Session | Electron Main | 实际持有与操作 |
| DOM/AX/截图 | Electron Main 临时采集 | 返回给 Rust |
| 输入框、弹窗、侧栏 | Renderer | 纯 UI 状态 |
| 窗口尺寸与位置 | Electron Main | 可本地持久化 |

Renderer Store 只是服务端状态的客户端投影：

```text
App Server Notification
        ↓
Event Reducer
        ↓
Renderer View Store
        ↓
UI
```

不要在 Renderer 复制 Rust 的完整业务状态机。

### 10.2 持久化事实来源

采用追加式 Thread Rollout 作为对话与执行历史的唯一持久化事实来源，SQLite State DB
作为可重建的查询投影：

```text
Thread Rollout / Event Log（权威）
  ├── User / Agent Items
  ├── Tool Call / Result
  ├── Approval Decision
  ├── Turn State Transition
  └── Recovery Marker
           │
           ▼
SQLite State DB（投影）
  ├── thread/list 与搜索索引
  ├── 名称、归档、置顶等元数据
  ├── 最近状态与统计
  ├── schema migration 版本
  ├── 副作用 RPC idempotency ledger
  └── Thread writer lease
```

每条 Rollout 记录至少包含：

```text
schemaVersion
threadId
sequence
eventId
recordedAt
eventKind
payload
checksum
```

持久化规则：

1. 每个持久化 Thread 使用独立的追加日志，`sequence` 在该 Thread 内严格递增；
2. 已完成 Item、审批决定和 Turn 终态必须先持久化，随后才能发送对应完成通知；
3. 流式 delta 可以是暂态通知；恢复以已持久化 Item snapshot 为准；
4. Rollout 写入成功但 SQLite 投影更新失败时，后台或下次启动从 Rollout 重建投影；
5. SQLite 不保存一份与 Rollout 竞争的完整对话事实；
6. 写入使用临时文件、原子 rename 或等价的追加完整性机制，启动时检测并截断不完整尾记录；
7. Rollout 和 SQLite schema 分别版本化，迁移必须支持旧 fixture，并保留失败回滚或重建路径；
8. `thread/read` 从权威记录构建结果，`thread/list` 可以读取 SQLite 投影。

Idempotency ledger 保存 method、principal/state-root scope、key、request hash、结果引用和
retention deadline。创建 Thread 或 Turn 的幂等记录与对应首条 Rollout/metadata 在同一
逻辑提交边界内建立，不能只保存在连接内存中。

Config 文件和认证凭据不进入 Thread Rollout。普通配置由 Config 模块持有；长期凭据使用
操作系统凭据存储或等价安全设施，日志和协议错误不得包含明文 secret。

### 10.3 进程所有权

上述权威性首先成立于单个 App Server 进程内部。第一版即使不引入 daemon，也必须保证：

```text
同一个持久化 Thread
  → 同一时刻最多由一个 App Server 进程以可写方式加载
```

CLI 直连 Core 与 Desktop 启动的 app-server 是两个独立进程，进程内的 `connectionId` 和
Thread 订阅表不能解决它们之间的并发写入。Storage 层应提供 Thread 写入 lease 或进程锁。
发生冲突时，CLI 可以连接现有 App Server，或者明确返回 `ThreadAlreadyOwned`，不能让两个
App Server 静默修改同一个 Thread。`archive`、`delete`、rollback 和会改变 Rollout 的 metadata
操作也必须取得相同 lease；普通 Config 文件写入使用独立文件锁和原子替换。引入 daemon 后，
可以由 daemon 统一持有这些 lease。

---

## 11. Desktop 内部结构

### 11.1 Main Process

职责：

```text
启动和监督 zeta app-server
维护 JSON-RPC 连接
创建窗口和 WebContentsView
管理 Browser Session
执行 CDP 操作
处理系统权限
验证 Renderer IPC 来源
把 app-server 通知转发给 Renderer
```

Main 不负责 Agent 规划、Thread 状态机或数据库业务。

App Server 进程监督规则：

- 只启动应用包内经过签名或构建校验的精确二进制路径，不通过 shell 或 `PATH` 查找；
- 使用环境变量 allowlist，清除可能改变动态链接、代理、日志目标或运行目录的非必要变量；
- 设置启动、initialize 和 graceful shutdown deadline，超时后回收整个子进程树；
- initialize 后校验 server build、协议版本和 schema hash，不匹配时禁止进入 Ready；
- stdout 只进入协议解析器，stderr 进入带大小上限和 secret 脱敏的日志管线；
- 自动重启使用有上限的指数退避，避免崩溃循环。

### 11.2 Preload

Preload 只暴露有限接口：

```ts
contextBridge.exposeInMainWorld("zeta", {
  thread: {
    start: (params: ThreadStartParams): Promise<ThreadStartResult> =>
      ipcRenderer.invoke("zeta:thread:start", params),
    read: (params: ThreadReadParams): Promise<ThreadReadResult> =>
      ipcRenderer.invoke("zeta:thread:read", params),
  },
  turn: {
    start: (params: TurnStartParams): Promise<TurnStartResult> =>
      ipcRenderer.invoke("zeta:turn:start", params),
    interrupt: (params: TurnInterruptParams): Promise<void> =>
      ipcRenderer.invoke("zeta:turn:interrupt", params),
  },
  events: {
    subscribe: (listener: (event: DesktopEvent) => void) => {
      const handler = (_: Electron.IpcRendererEvent, event: DesktopEvent) => {
        listener(event);
      };
      ipcRenderer.on("zeta:event", handler);
      return () => ipcRenderer.removeListener("zeta:event", handler);
    },
  },
});
```

这些 Params 和 Result 从生成的 App Server Protocol 类型导入。Preload 内部可以使用统一的
IPC helper，但不能把 `execute(id: string, args?: unknown)` 这种万能入口暴露给 Renderer。
Renderer 的 UI Command Registry 仍然保留；纯 UI Command 不穿过 IPC，需要 Main 权限的
Command 才调用上述强类型接口。

不要暴露：

```text
ipcRenderer 原对象
fs
child_process
webContents
debugger
任意 app-server method 调用
```

### 11.3 Renderer

Renderer 负责：

```text
UI Command Registry
页面路由
纯 UI 状态
服务端状态投影
流式消息展示
虚拟长列表
审批与用户输入界面
```

Renderer 不直接控制第三方网页，也不直接启动 Rust 进程。

---

## 12. 安全模型

### 12.1 WebContents 信任边界

至少分离两个 WebContents：

```text
Trusted App UI
Untrusted Browser Page
```

第三方页面必须：

```text
nodeIntegration: false
contextIsolation: true
sandbox: true
无特权 preload
无应用 IPC
独立 Session / partition
严格导航与新窗口策略
```

App UI 也只通过 contextBridge 暴露窄 API。

Electron Main 对每个 Renderer 请求验证：

- sender 是否为可信 App UI；
- frame URL 和 origin；
- command 是否在白名单；
- 参数是否通过 schema 验证；
- 当前会话是否有能力；
- 需要审批的操作是否携带仍然有效且与动作完全匹配的授权。

### 12.2 审批权威

是否需要审批由 Rust Core 的 `ApprovalPolicy` 决定。Renderer 只负责展示请求和收集用户决定，
Electron Main 不能把 Renderer 返回的裸 `accept` 当作独立授权。

```text
Core ApprovalPolicy
  → 构造不可变 ProposedAction
  → 计算 actionDigest
  → approval/request
  → Renderer 展示并返回 decision
  → Core 校验 request、digest、scope 和 expiry
  → Tool Executor 执行
```

审批请求和响应至少绑定：

```text
threadId
turnId
toolCallId
approvalRequestId
actionDigest
decision
scope
expiresAt
```

`acceptForSession` 只能生成受限 grant，grant 必须记录允许的工具、参数范围、工作区或网络
目标，不能退化成整个进程的全局布尔开关。Electron Main 对 Browser、下载、外部 URL 和
系统权限继续执行宿主侧强制策略，形成 Core policy 与 host policy 的双重约束。

### 12.3 工具执行与沙箱

所有命令、文件写入和网络访问都必须经过统一 Tool Executor，不能由 CLI、App Server handler
或 Desktop 绕过。第一版的最低执行边界包括：

- 默认 sandbox policy；
- canonical workspace root 校验；
- 命令超时、取消和子进程回收；
- stdout、stderr 和单条事件大小上限；
- 环境变量白名单与敏感值脱敏；
- 执行前审批，执行后记录实际 argv、cwd、权限和结果；
- 被批准动作与实际执行动作的 digest 一致性检查。

### 12.4 Browser Capability

Browser Capability 不能接受 Rust 传来的任意 CDP method。Rust 应输出语义动作：

```rust
pub enum TextInputTarget {
    Element(ElementTarget),
    FocusedElement,
}

pub enum BrowserAction {
    Navigate { url: String },
    Click { target: ElementTarget },
    TypeText { target: TextInputTarget, text: String },
    Scroll { delta_x: f64, delta_y: f64 },
    GoBack,
    Reload,
}
```

Electron Main 再转换为受控的 CDP 调用。

进程隔离不能阻止 Agent 把已登录网页中的敏感信息发送给模型。Browser policy 还必须定义：

- 允许观察和操作的 origin；
- Cookie、密码框、支付字段和个人数据的脱敏；
- 文件上传、下载、剪贴板和跨 origin 导航策略；
- 页面内容视为不可信输入，不能把网页指令提升为系统或用户授权；
- 截图、DOM、AX Tree 和网络摘要的大小限制与保留期限。

---

## 13. 协议类型生成

Rust 是协议事实来源：

```text
zeta-rs/app-server-protocol
          ↓
      TypeScript
      JSON Schema
```

建议命令：

```bash
zeta app-server generate-ts \
  --protocol-version 1 \
  --out desktop/generated/app-server/v1

zeta app-server generate-json-schema \
  --protocol-version 1 \
  --out schemas/app-server/v1.schema.json
```

生成后在 CI 检查工作树是否干净：

```bash
pnpm generate:protocol
git diff --exit-code
```

协议类型应包含：

```text
Request methods
Request params
Request results
Notifications
Server-to-client requests
Error codes
Capability types
Protocol version
```

初始化时客户端发送支持的 `[min, max]`，服务端选择交集中的一个版本。没有交集时立即返回
`ProtocolVersionUnsupported`，不能进入部分初始化状态。

协议兼容规则：

1. wire 类型在 `v1` 模块中冻结；字段不能改名、复用或改变既有语义；
2. 只有“字段缺失时保持旧行为”的新增可选字段才属于向后兼容；
3. 新增 enum variant 默认视为不兼容；只有类型明确设计了 `unknown` fallback 时才能在同一版本增加；
4. Error code 和 terminal status 是协议的一部分，不能只保持 JSON 字段兼容；
5. 未协商的 capability 不得调用；具有独立演进速度的 capability 携带自己的版本；
6. 未知通知可以忽略，未知请求必须返回 `MethodNotFound`；
7. 客户端必须先 `initialize`，服务端返回选定版本、协商能力和当前 schema hash；
8. Desktop 与内置 `zeta` 二进制优先同版本打包，但 CI 仍维护当前版本与前一兼容版本的 fixtures；
9. 破坏性变更新增 `v2` DTO 和 mapper，`v1` 在明确的废弃窗口内继续受支持。

---

## 14. App Server 传输

第一阶段：

```text
stdio
JSON Lines
双向 JSON-RPC
```

约束：

```text
stdout：只输出协议消息
stderr：日志
每行一个完整 JSON 消息
消息体限制最大字节数
大文件和截图使用资源引用
transport ingress、request queue 和 outbound queue 全部有界
```

队列饱和时，新请求返回可重试的 `ServerOverloaded`，并可携带 `retryAfterMs`；客户端使用带
jitter 的指数退避。Durable 状态通知、RPC response 和 approval request 不能静默丢弃。
允许合并的高频暂态事件必须逐类定义，例如相邻 token delta 或进度百分比；不能使用
unbounded channel 把慢客户端造成的压力转移到进程内存。

不要在业务消息中直接传输大尺寸 PNG、PDF Base64 或任意本地路径。协议只传输：

```json
{
  "resourceId": "resource_123",
  "mimeType": "application/pdf",
  "size": 4200000,
  "sha256": "..."
}
```

资源生产方在自己的 Resource Store 中保存内容，消费方通过双向 RPC 分块读取：

```text
resource/metadata
resource/read
resource/release
```

```json
{
  "id": 92,
  "method": "resource/read",
  "params": {
    "resourceId": "resource_123",
    "offset": 0,
    "maxBytes": 262144
  }
}
```

Resource Store 的本地实现可以使用应用私有临时目录，但文件路径不进入公开协议。每个资源
必须记录 owner connection、TTL、大小、digest、读取权限和引用计数。连接断开、显式 release
或 TTL 到期时回收；达到单资源或单连接 quota 时拒绝创建。`resource/read` 必须限制 chunk
大小并验证 offset，不能借资源接口读取任意文件。

未来 socket 或远程 transport 可以在 capability 协商后增加二进制帧或对象存储实现，
`ResourceRef` 的业务协议形状保持不变。

后续需要多个客户端或后台常驻时，再引入：

```text
zeta app-server-daemon
Unix Domain Socket
Windows Named Pipe
```

演进后：

```text
                zeta daemon
               /           \
          zeta CLI       Desktop
```

但不要第一阶段就增加 daemon 的进程治理复杂度。

---

## 15. 状态机建议

### 15.1 Turn

```text
Created ───────────────→ Running
                           ├──→ WaitingForApproval ──┐
                           ├──→ WaitingForUserInput ─┤
                           ├──→ WaitingForCapability ┤
                           │                         │
                           │←────────────────────────┘
                           ├──→ Completed
                           ├──→ Failed
                           └──→ Cancelling ──→ Interrupted

Stage 6 background Turn:
Running / Waiting* ──→ Orphaned ──→ Recovering ──┬──→ Running
                                                 └──→ Failed
```

`Completed`、`Failed` 和 `Interrupted` 是互斥终态。`Waiting*` 可以在一次 Turn 中重复进入；
等待期间发生取消时先进入 `Cancelling`，待相关 Tool、Model Stream 和子进程停止后进入
`Interrupted`。`Orphaned` 和 `Recovering` 为 Stage 6 预留，Stage 1 不恢复实际后台执行。
等待原因使用明确 enum 表示，不使用多个可能互相冲突的 bool。

### 15.2 Item

```text
Created ──→ InProgress ──┬──→ Completed
                         ├──→ Failed
                         └──→ Cancelled
```

Streaming 是 `InProgress` Item 的输出方式，不是所有 Item 都必须经历的独立状态。

### 15.3 Tool Call

```text
Proposed ──┬──→ AwaitingApproval ──┬──→ Running ──┬──→ Succeeded
           │                        │               ├──→ Failed
           │                        ├──→ Declined   └──→ Cancelled
           │                        └──→ Cancelled
           └───────────────────────────→ Running
```

`Declined` 表示用户拒绝执行，`Cancelled` 表示请求或运行中的动作被取消，两者不能合并。
进入 `Running` 前必须固定实际执行参数和 action digest。

### 15.4 App Server Connection

```text
StartingProcess
  → Connecting
  → Initializing
  → Ready
      ├──→ Disconnected ──→ Restarting ──→ Connecting
      └──→ Closing ──→ Closed

任一非终态 ──→ Failed
```

### 15.5 Browser Target

```text
Created
  → Navigating
  → Ready
      ├──→ Navigating
      ├──→ Crashed
      └──→ Closing ──→ Closed
```

`Crashed` 的目标句柄不重新变为 `Ready`；恢复时创建新 Target 和新句柄。UI 只展示这些状态
的投影，非法状态转换由 Rust Core 或 Electron Main 的权威模块拒绝。

每次领域状态转换都产生 Rollout 记录。内存状态更新、持久化和完成通知的顺序固定为：

```text
验证转换
  → 追加权威 Rollout
  → 更新内存状态和 SQLite 投影
  → 发出通知
```

---

## 16. 测试策略

### 16.1 Core 单元测试

通过 mock capability 测试 Agent：

```rust
struct MockBrowserCapability;
struct MockApprovalCapability;
struct MockModelProvider;
```

覆盖：

- Turn 状态转换；
- 工具调用和审批；
- action digest 与审批 grant 校验；
- `Declined`、`Cancelled` 和 `Interrupted` 的区别；
- 取消与超时；
- 浏览器观察结果处理；
- 崩溃后的恢复；
- Thread 持久化。

### 16.2 Storage 与恢复测试

```text
Rollout sequence 严格递增且 eventId 不重复
尾部记录只写入一半时能够检测和恢复
Rollout 已提交但 SQLite 未更新时能够重建投影
SQLite 完全删除后能够从 Rollout 重建 thread/list
旧 schema fixtures 能迁移或以只读方式打开
两个 App Server 争用同一 Thread writer lease 时只有一个成功
进程崩溃后的过期 lease 能按规则回收
Turn 终态通知不会早于对应 Rollout durable commit
```

### 16.3 App Server 协议测试

```text
initialize 必须为首个请求
协议版本区间无交集
协商结果包含版本、capabilities 和 schema hash
未知 method
错误参数
有界队列饱和时返回 ServerOverloaded，且不会丢 durable 通知
双向 browser 请求
流式通知顺序
同一 Thread 的修改请求按连接到达顺序串行执行
不同 Thread 的请求可以并发执行
同一 Thread 的多个订阅连接收到正确通知
不同连接使用相同 JSON-RPC id 时响应不会串线
turn/start 响应丢失后使用相同 idempotencyKey 不会创建第二个 Turn
相同 idempotencyKey 携带不同参数时返回冲突
审批和用户输入请求返回到发起 Turn 的连接
Browser Target 句柄只能路由到创建它的连接
thread/resume 的快照与实时订阅之间没有事件空洞
durable sequence 或 streamSeq 空洞会触发 snapshot resync
thread/unsubscribe 只移除当前连接
客户端断开时任务处理
客户端断开时移除订阅并清理 pending server requests
deadline 或 Turn 结束后发出 serverRequest/resolved，迟到响应不恢复请求
resource/read 的 offset、chunk 上限和 owner 校验
resource/release、TTL 和断连清理
当前版本与前一兼容版本 fixtures
```

建议维护一个 Rust test client 和一个 TypeScript contract client。

### 16.4 Desktop 集成测试

```text
启动 Desktop
启动内置 zeta app-server
完成 initialize
创建 thread
开始 turn
接收 delta
打开浏览页面
Agent 观察网页
返回回答
中断 turn
恢复 thread
Preload 拒绝未生成或参数不匹配的 IPC
审批响应无法用于不同 action digest
Browser Target 关闭后不会静默操作其他 Tab
```

### 16.5 生成代码一致性

CI 必须验证：

```text
Rust 协议变更后 TS 是否重新生成
Schema 是否更新
Desktop 是否能编译
旧 fixtures 是否仍能读取
```

---

## 17. 开发阶段

### 阶段 1：建立 Rust 产品内核

创建：

```text
zeta-rs/core
zeta-rs/protocol
zeta-rs/config
zeta-rs/credentials
zeta-rs/storage
zeta-rs/built-in-tools
zeta-rs/exec
zeta-rs/sandboxing
zeta-rs/model-provider
zeta-rs/cli
zeta-rs/tui
```

完成：

```bash
zeta ask "解释当前仓库"
zeta exec "检查测试失败"
```

此阶段不依赖 Electron，但必须同时完成：

```text
Rollout 权威日志与 SQLite 可重建投影
Thread writer lease
Turn / Item / Tool Call 状态机
统一 Tool Executor
默认 sandbox、workspace root 校验、审批、超时和输出上限
OS Credential Store 与日志 secret 脱敏
崩溃恢复与旧 fixture 测试
```

### 阶段 2：建立 App Server

创建：

```text
zeta-rs/app-server-protocol
zeta-rs/app-server-transport
zeta-rs/app-server-client
zeta-rs/app-server
```

完成：

```text
initialize
thread/start
thread/read
turn/start
turn/interrupt
item delta notifications
Connection / Thread subscriptions
副作用 RPC idempotency ledger
durable sequence、streamSeq 与 snapshot resync
resource/metadata
resource/read
resource/release
```

建立 v1 DTO mapper、协议生成、版本协商和 Rust/TypeScript 测试客户端。
`app-server-client` 是可复用的 Rust 客户端与契约测试入口，后续 daemon-aware CLI/TUI
也复用它，不在各入口重新实现连接、订阅和重连逻辑。

### 阶段 3：Desktop 基础壳

完成：

```text
Electron Main / Preload / Renderer
启动 zeta app-server
initialize
thread/list
turn/start
流式消息 UI
长对话虚拟列表
生成的强类型 Preload API
审批与 action digest 展示
```

此阶段先不做浏览器 Agent。

### 阶段 4：浏览器 Capability

完成：

```text
WebContentsView
独立 Session
CDP attach
browser/observe
browser/perform
截图、DOM、AX Tree
Target handle 与 connection/turn 绑定
origin、敏感字段、上传下载和保留期限策略
```

### 阶段 5：PDF 与网络能力

完成：

```text
PDF URL/响应捕获
原始 PDF 下载
文档资源引用
Agent PDF 工具
下载和权限策略
```

### 阶段 6：恢复与后台化

在实际需要时增加：

```text
app-server-daemon
本地 socket
任务租约
CLI 与 Desktop 同时连接
后台任务恢复
```

虽然 daemon 延后实现，但边界现在固定：

- 每个用户和 state root 最多一个 daemon，使用持有者权限受限的 local socket；
- CLI 与 Desktop 继续使用同一 App Server Protocol，不新增第二套 daemon 业务协议；
- daemon 成为 Thread writer lease 的统一持有者，客户端断开不自动终止已声明为后台的 Turn；
- 前台 Turn 在 owner 连接断开时按策略取消或进入 orphaned recovery，不静默继续执行高风险动作；
- Stage 1 的“崩溃恢复”只重建历史并把未完成 Turn 标为中断；Stage 6 才允许恢复实际后台执行；
- ResourceRef、版本协商和 Connection/Thread 订阅模型保持不变，transport 可以从 stdio
  切换到 Unix Domain Socket 或 Windows Named Pipe。

---

## 18. 第一版最小目录

不要一开始建立完整目录树。实际可从下面开始：

```text
zeta/
├── zeta-rs/
│   ├── Cargo.toml
│   ├── core/
│   ├── protocol/
│   ├── app-server-protocol/
│   ├── app-server-transport/
│   ├── app-server-client/
│   ├── app-server/
│   ├── cli/
│   ├── tui/
│   ├── config/
│   ├── credentials/
│   ├── storage/
│   ├── built-in-tools/
│   ├── exec/
│   ├── sandboxing/
│   └── model-provider/
│
├── desktop/
│   ├── src/main/
│   ├── src/preload/
│   ├── src/renderer/
│   └── generated/app-server/
│
├── schemas/
├── scripts/
└── docs/
```

拆 crate 的标准不是“文件多了”，而是至少满足一项：

- 有独立稳定职责；
- 有明确依赖边界；
- 可以单独测试；
- 被多个入口复用；
- 需要控制编译 feature；
- 有独立发布或版本语义。

---

## 19. 关键设计规则

1. `zeta-rs` 是完整 Rust 产品工作区，不只是领域 Core。
2. `zeta-rs/core` 是狭义领域内核，不能依赖 Electron、TUI 或 app-server。
3. Core 定义自己消费的 capability/repository port；第一版不建立重复的 `core-api` crate。
4. Config、Credentials、Storage、Exec、Sandboxing、Built-in Tools 和 Model Provider
   从第一天就是独立 crate；它们实现 Core port，由组合入口注入，不暂存进 Core 等待以后拆分。
   File Search、File Watcher、Git Utils 与 utility 能力也在首次实现时直接建立独立 crate。
5. Desktop 不直接链接 Core，只连接 app-server；CLI 可以在进程内组合 Core 与 adapter。
6. `protocol` 是内部领域协议；`app-server-protocol/v1` 是冻结的外部 RPC DTO。
7. 外部协议只复用经过审核的稳定 ID 和标量，不直接公开内部领域 struct 或 enum。
8. 协议从 Rust 生成 TypeScript 和 JSON Schema，不在两边手写重复类型。
9. Thread Rollout 是对话与执行历史的持久化事实来源，SQLite 是可重建查询投影。
10. 已完成 Item、审批决定和 Turn 终态先 durable commit，再发送完成通知。
11. 同一个持久化 Thread 同一时刻只能由一个 App Server 进程以可写方式加载。
12. Thread、Turn、Item 和 Agent 运行状态的权威来源在 Rust。
13. Browser View、Session 和 Browser Target 的权威来源在 Electron Main。
14. 是否需要审批由 Core ApprovalPolicy 决定；Renderer 只采集决定，Host 继续执行本地策略。
15. 所有命令、写文件和网络动作必须经过统一 Tool Executor，不能绕过 sandbox 与审批。
16. 长期凭据只通过 Credential Store port 访问，不能写入普通配置、Rollout 或日志。
17. UI 内部保留 Command Registry；跨 Preload 和跨进程使用生成的强类型 API。
18. 第三方网页无 Node、无特权 preload、无应用 IPC。
19. 不把任意 CDP 命令作为公开 Agent 工具，网页内容始终视为不可信输入。
20. App Server 使用内部 `connectionId` 和公开 `threadId` 共同完成订阅与请求路由。
21. 同一 Thread 的修改请求串行化，不同 Thread 可以并发执行。
22. Browser Target 绑定到提供能力的连接和具体 Tool Call，不依赖可变化的全局 active Tab。
23. Thread 订阅与客户端能力所有权是两个概念；Turn owner 和 CapabilityHandle owner
    决定 Server → Client 请求发往哪个连接。
24. 副作用 RPC 使用 idempotency key；durable 通知和流式 delta 使用各自单调序号检测空洞。
25. 大对象通过有 owner、TTL、digest 和 quota 的 `ResourceRef` 传输，公开协议不暴露路径。
26. Transport 队列全部有界；durable 通知和 RPC response 不得因慢客户端静默丢失。
27. 第一阶段使用 stdio；确认有多客户端需求后再引入 daemon。
28. Desktop 打包的 `zeta` 二进制与生成协议必须来自同一构建，并兼容前一协议 fixture。

---

## 20. 最终依赖图

箭头表示“左侧依赖右侧”：

```text
zeta-core ───────────────────────────→ zeta-protocol

zeta-storage ────────────────────────→ zeta-core + zeta-protocol
zeta-exec ───────────────────────────→ zeta-core + zeta-sandboxing
zeta-built-in-tools ─────────────────→ zeta-core + zeta-exec
zeta-model-provider ─────────────────→ zeta-core
zeta-config ─────────────────────────→ zeta-core
zeta-credentials ────────────────────→ zeta-core

zeta-app-server-protocol ────────────→ zeta-protocol（仅稳定叶子类型）
zeta-app-server-transport ───────────→ app-server-protocol::common
zeta-app-server-client ──────────────→ app-server-protocol + app-server-transport

zeta-cli / zeta-tui ─────────────────→ zeta-core + adapters
zeta-app-server ─────────────────────→ zeta-core + adapters
                                        + app-server-protocol
                                        + app-server-transport

Electron Desktop ── JSON-RPC ────────→ zeta-app-server
```

`zeta-core` 不反向依赖 Storage、Exec、Model Provider 或 App Server。`zeta-cli` 和
`zeta-app-server` 是 composition root，负责选择 adapter、构建 Thread manager 并注入 capability。
`core::tools` 只包含 Tool 状态机、registry、policy 和 port；顶层 `zeta-built-in-tools` 提供具体
内置 Tool adapter，不能复制 Agent tool loop。

更完整的方向：

```text
Desktop Renderer
   ↓ UI Command
Desktop AppServerClient
   ↓ domain RPC
zeta app-server
   ↓ typed service call
zeta-core
   ↓ outbound capability request
zeta app-server
   ↓ JSON-RPC
Electron CapabilityHost
   ↓ CDP
Browser WebContentsView
```

---

## 21. 推荐决策

采用：

```text
zeta/
├── zeta-rs/
└── desktop/
```

而不是：

```text
zeta/
├── crates/
└── apps/
```

这不是因为后者技术上错误，而是前者更准确地表达产品架构：

- `zeta-rs` 是可独立开发、测试和发布的 Rust 产品运行时；
- CLI/TUI 与 app-server 都是该运行时的正式入口；
- Desktop 是外部富客户端；
- App Server 是 Desktop 与 Rust 产品能力之间的稳定边界；
- Browser Capability 是 Desktop 提供给 Agent 的宿主能力。

最终目标不是“CLI 被 Desktop 复用”，也不是“Desktop 包装 CLI”，而是：

> CLI 与 Desktop 分别通过适合自己的适配层消费同一个 Zeta Rust 产品内核。

---

## 22. 参考实现

本方案主要参考 OpenAI Codex 当前 Rust workspace 的以下设计特征：

- `codex-rs` 是独立 Cargo workspace；
- `core`、`protocol`、`app-server`、`app-server-protocol`、`app-server-client`、`cli` 和 `tui` 是同级 crate；
- `app-server` 同时依赖 Core、内部 Protocol 和 App Server Protocol；
- CLI 可以组合 Core、TUI 与 App Server；
- App Server Protocol 可以复用内部 Protocol 的稳定类型，并承担 TypeScript / Schema 输出职责；
- App Server 为每个连接维护内部 `ConnectionId`，并维护 Connection 与 Thread 的双向订阅关系；
- 同一 Thread 的状态修改请求按 Thread scope 串行化，不同 Thread 可以并发执行。

Zeta 不机械复制所有实现细节：本方案进一步限制内部领域 enum 向外部协议泄漏，并从第一版
定义 ResourceRef 生命周期、Thread writer lease 和 action digest 审批绑定。

官方仓库：

- <https://github.com/openai/codex>
- <https://github.com/openai/codex/tree/main/codex-rs>
- <https://github.com/openai/codex/tree/main/codex-rs/app-server>
- <https://github.com/openai/codex/tree/main/codex-rs/app-server-protocol>
- <https://github.com/openai/codex/blob/main/codex-rs/app-server/src/thread_state.rs>
- <https://github.com/openai/codex/blob/main/codex-rs/app-server/src/request_serialization.rs>
