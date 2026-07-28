# App Server Client 架构与演进方案

> 物理位置：`zeta-rs/app-server-client/`  
> 主要消费者：`zeta-exec` 非交互执行宿主、`zeta-tui`  
> Wire contract：[`zeta-app-server-api.md`](zeta-app-server-api.md)  
> Canonical 产品模型：[`protocol.md`](protocol.md)  
> Headless 与远程调度：[`exec.md`](exec.md)

## 1. 结论

`zeta-app-server-client` 存在的原因，不只是复用一组 typed RPC method。`zeta-exec` 与
`zeta-tui` 都需要完成同一套本地 App Server 宿主流程：

1. 根据启动配置创建并启动 App Server；
2. 建立 client 到 App Server 的请求通道；
3. 建立 App Server 到 client 的结果与事件通道；
4. 通过请求通道完成 `initialize` 与 schema/capability 校验；
5. 只在初始化成功后向调用方交付 ready client；
6. 运行期间正确配对请求结果，并持续转发 server notification；
7. 退出时关闭 connection、结束 pending 请求并等待后台任务停止。

这套启动、连接和关闭逻辑不能在 `zeta-exec` 与 `zeta-tui` 中各写一份，因此抽成共享 crate。

```text
zeta-exec ─┐
           ├─► zeta-app-server-client ─► start App Server
zeta-tui ──┘             │
                         ├─ request channel ──► App Server
                         └─ result/event  ◄──── App Server
```

直接依赖和启动 `zeta-app-server` 是 embedded backend 的职责，不是依赖方向错误。长期可以在
同一 public facade 下增加 remote App Server backend，但它只连接相同的 App Server contract，
不是 scheduler protocol，也不是 remote process executor。Desktop 的 JSONL/stdio client
仍不要求复用这个 Rust crate。

`zeta-exec` 是无交互界面的 Agent 执行宿主。当前它与 TUI 一样启动 embedded App Server；
后续远程调度系统以它作为 headless execution entry。Job/Attempt/lease/event cursor 属于
[`exec.md`](exec.md) 定义的 scheduler adapter，不进入 App Server Client。

## 2. 抽象单位：一个运行中的 App Server Session

共享层的顶层抽象应是一个有明确所有权的运行会话，而不是裸 transport：

```rust
pub struct AppServerSession {
    client: AppServerClient,
    events: Option<AppServerEvents>,
    shutdown: ShutdownHandle,
    tasks: AppServerTasks,
}
```

`AppServerSession` 拥有：

- embedded App Server runner，或一个 remote App Server connection；
- 唯一 App Server connection；
- request channel 的 server 端；
- server message/event channel 的 client 端；
- connection driver 与 server runner 的后台任务；
- initialize/ready/closing/closed 生命周期；
- 显式 shutdown 与 task join。

它向宿主提供两个运行时端点：

```rust
impl AppServerSession {
    pub fn client(&self) -> AppServerClient;
    pub fn take_events(&mut self) -> Result<AppServerEvents, TakeEventsError>;
    pub async fn shutdown(self) -> Result<(), ShutdownError>;
}
```

- `AppServerClient` 是可克隆的请求 handle，供 feature/task 发送 typed request；
- `AppServerEvents` 是单消费者事件流，供 app event loop 持续接收 notification 与 connection
  lifecycle event；
- `AppServerSession` 保持所有权直到显式 shutdown，避免最后一个 client clone 的偶然 drop
  决定 server 生命周期。

结果与事件必须区分：

- request result 通过该 request 自己的 completion/oneshot 返回；
- server notification 通过独立 `AppServerEvents` 流返回；
- connection closed、protocol failure 等连接事件也通过 `AppServerEvents` 告知宿主；
- notification 不得依附某次 request，不能通过 `drain_notifications()` 拉取。

## 3. 为什么由这个 crate 统一连接 App Server

`zeta-exec` 和 `zeta-tui` 都是本地 App Server 的宿主，而不是已经存在的外部 server 的普通
调用方。两者需要相同的 composition：

- state root 与本地 rollout/store；
- config、credentials、model provider 和 tool runtime；
- App Server dispatcher；
- connection-scoped subscription 与 Resource ownership；
- client/server channel；
- initialize 参数、schema hash 与 capabilities；
- shutdown 与错误回收。

如果这些步骤留在各自 consumer，会形成两套不同的启动语义：一个可能忘记 schema gate，另一个
可能没有事件 pump，第三个关闭时遗留后台任务。共享 crate 必须保证它们拿到的是同一种 ready
connection。

依赖方向应是：

```text
zeta-exec / zeta-tui
          │
          ▼
zeta-app-server-client
   ├─► zeta-app-server
   ├─► zeta-app-server-protocol
   └─► async/channel runtime
```

Consumer 不直接创建 `AppServer`、`ConnectionState`、dispatcher 或 notification broker。
这些类型可以在 client crate 内部使用，但不能泄漏到 `zeta-exec` 或 TUI 的业务代码。

目标架构中的 `zeta-exec` 是非交互 Agent 宿主和未来远程调度执行入口。当前
`zeta-rs/exec` 仍实现底层 tool process execution，这是现状与目标的职责偏差；迁移时必须把
底层命令执行能力保留为独立边界，不能让“执行一个 tool process”和“宿主化完整 App Server”
继续共用同一个含义不清的模块。

长期 backend 选择必须是显式 enum：

```rust
pub enum AppServerTarget {
    Embedded(EmbeddedAppServerOptions),
    Remote(RemoteAppServerOptions),
}
```

- `Embedded` 由本 crate 组合并启动本地 App Server；
- `Remote` 连接 daemon/remote App Server，完成相同 initialize 与 event wiring；
- 两者暴露相同 typed request handle、event stream 与 shutdown contract；
- backend 差异不能泄漏成 Core 私有方法；
- remote scheduler 仍位于 `zeta-exec` 上层，不等于 `RemoteAppServerOptions`。

## 4. 启动流程

建议暴露两个自解释入口，并可由 target enum 统一选择：

```rust
let mut session = AppServerSession::start_embedded(options).await?;
// 或 AppServerSession::connect_remote(options).await?;
let client = session.client();
let events = session.take_events()?;
```

Embedded start 必须按以下顺序执行：

```text
validate options
  → build App Server composition
  → create one server connection
  → create bounded request/result-event channels
  → start server runner and connection driver
  → send initialize through the normal request path
  → validate schema hash and required capabilities
  → return Ready AppServerSession
```

必须保持的 invariant：

- `initialize` 是 connection 的首个 request；
- initialize 也经过正式 request channel、dispatcher 和 result pairing，不能直接调用内部
  server method；
- schema hash 不一致或缺少 required capability 时，`start` 失败；
- `start` 失败必须关闭已创建的 connection/channel 并 join 已启动的 task；
- 调用方永远拿不到半初始化的 `AppServerClient`；
- start options 完整描述 composition，不依赖 consumer 在启动前偷偷修改全局状态。

Remote connect 必须：

```text
validate endpoint/auth
  → connect bounded transport
  → send initialize as first request
  → validate schema/capabilities
  → start request/event driver
  → return Ready AppServerSession
```

Remote connection failure 必须结束 pending request 并发出 connection event；自动 reconnect 只有
在能重新建立 subscription、Resource ownership 和 transient cursor 时才能增加，不能隐藏状态
丢失。

启动配置应使用自解释类型，而不是 bool：

```rust
pub struct AppServerStartOptions {
    pub state_root: PathBuf,
    pub client_info: ClientInfo,
    pub required_capabilities: RequiredCapabilities,
}
```

若 `zeta-exec` 与 TUI 的 capability 不同，可以分别构造 typed
`RequiredCapabilities`，不能用 `start(..., true, false)` 表达。

## 5. 请求通道与 result pairing

`AppServerClient` 的 typed method 将请求送入后台 connection driver：

```text
typed params
  → allocate connection-local request ID
  → build typed ClientRequest
  → request channel
  → App Server dispatcher
  → response channel
  → match pending request ID
  → typed result / typed server error
```

Embedded hot path 推荐使用 protocol-owned typed request enum：

```rust
enum ClientRequest {
    Initialize(InitializeParams),
    SessionCreate(SessionCreateParams),
    ThreadSubscribe(ThreadSubscribeParams),
    TurnStart(TurnStartParams),
    // generated/registered remaining methods
}
```

Client worker 在 request 外附加 connection-local request ID 与 oneshot completion。In-process
channel 不需要先序列化为 JSON string，但仍必须经过相同 initialize gate、method dispatcher、
result/error envelope 和 notification contract。JSON/JSONL/WebSocket backend 在 transport
边界执行 wire encoding。

Public API 仍然是 typed method：

```rust
let result = client.start_turn(TurnStartParams { /* ... */ }).await?;
```

调用方不拼 method string、不处理 JSON-RPC request ID，也不直接操作 request channel。

请求 driver 负责：

- 分配只在当前 connection 内有效的 request ID；
- 保存 `requestId → completion` pending table；
- 校验 response ID、envelope 与 typed result；
- 允许多个调用方持有 `AppServerClient` clone；
- connection 关闭时一次性结束全部 pending request；
- 将 server stable error 保留为 typed `AppServerError`，包括 code、name 和 data；
- 对 unknown、duplicate、retired request ID 做明确 protocol error 处理。

JSON-RPC request ID 不能代替产品 `CommandId`。修改操作的 `CommandId`、
`expectedSequence` 和 exact typed payload 由调用方创建并传入，client 只负责无损发送。超时或
response 丢失后的业务重试策略不属于 connection driver。

## 6. 事件通道

App Server notification 与 request result 是两条独立交付路径：

```text
                         ┌─ matching response ─► request completion
App Server outbound ─────┤
                         └─ notification ──────► AppServerEvents
```

`AppServerEvents` 至少表达：

```rust
pub enum AppServerEvent {
    SessionUpdate(SessionUpdateEnvelope),
    ThreadUpdate(ThreadUpdateEnvelope),
    ServerRequest(ServerRequest),
    Lagged { skipped: usize },
    Desynced { aggregate: AggregateRef },
    ConnectionClosed(ConnectionCloseReason),
}
```

当前 App Server API 只包含 `session/update` 与 `thread/update` notification；
`ServerRequest` 是 approval/user-input 等双向交互落地后的目标 variant，在 protocol method
registry 接受前不能提前声称可用。

事件 driver 负责：

- 在没有新 request 时仍持续接收 notification；
- 解码 `session/update` 与 `thread/update`；
- 保留 durable sequence 与 transient stream cursor；
- 保证同一 request 的 response 先于它产生的 causal notification 交付；
- 已知 notification 在 remote decode 或 typed validation 失败时关闭 connection 并报告
  protocol failure；
- server request 无 consumer/handler 时及时返回 stable rejection，不能让 approval 永久等待；
- event receiver 被 consumer drop 时执行明确 policy，而不是无限积压；
- bounded channel 满时不静默丢弃 durable update。

Event channel 不负责：

- 把 update 应用到 Session/Thread projection；
- 判断 durable sequence gap 后如何 resubscribe；
- 推导 Turn 或 Item 的权威终态；
- 将 notification 变成日志文本；
- 合并不同 aggregate 的 sequence。

这些属于 `zeta-exec` 的输出/终态协调或 TUI projection。

## 7. Driver 与 App Server 的连接方式

共享层需要同时驱动请求与事件，不能保留“调用一次、顺便 drain 一次”的模型：

```text
AppServerClient clones
        │
        ▼
bounded request channel
        │
        ▼
connection driver
  ├─ dispatch request ─────────► App Server
  ├─ pair response ────────────► request completion
  ├─ receive notification ─────► AppServerEvents
  └─ observe shutdown ─────────► close + join
```

进程内连接仍应经过同一个 App Server protocol dispatcher。Typed channel 可以避免 JSON 编解码、
stdio 与子进程开销，但不能直接调用 Core、Store 或私有 App Server operation。它复用相同
request/result/error/notification 语义，而不是复制第二份 in-process response contract。

App Server 必须向 runner 提供可唤醒的 outbound notification source。当前
`drain_notifications` 只在 client 发 request 后检查队列，无法支持空闲连接上的事件，也无法
支持长 Turn 运行期间的实时 update。修正需要让 runner 能同时等待：

- 新 client request；
- 新 server notification；
- shutdown signal。

这属于 client session 与 App Server runner 共同遵守的 channel contract。

## 8. 正确关闭

显式 `AppServerSession::shutdown()` 是正常退出路径。它必须：

1. 原子地把 session 状态从 `Ready` 改为 `Closing`；
2. 拒绝新 request；
3. 结束或明确失败所有尚未完成的 request completion；
4. 关闭 App Server connection，使 subscription 与 Resource ownership 失效；
5. 停止 server runner 和 connection driver；
6. 关闭 event stream，并给 consumer 一个最终 `ConnectionClosed` 原因；
7. join 所有由 session 启动的后台 task；
8. 返回 shutdown 期间发生的错误。

关闭 connection 不等于 `turn/interrupt`。若产品要求退出前中断某个 Turn，`zeta-exec` 或 TUI
必须先发送 typed `turn/interrupt` 并等待所需终态，再调用 session shutdown。Client session
不能猜测要中断哪个 Thread/Turn。

`Drop` 只能做 best-effort cancellation，不能替代显式 shutdown，因为 `Drop` 无法可靠等待
后台任务结束。测试和正常 consumer 路径必须调用并等待 `shutdown()`。

需要定义以下边界情况：

- event receiver 先于 session 被 drop；
- 所有 client handle 被 drop，但 session owner 仍存在；
- session shutdown 时仍有 client clone；
- initialize 期间 consumer 取消启动；
- App Server runner panic；
- request completion receiver 被调用方取消；
- shutdown 被重复触发。

## 9. Consumer 使用方式

### 9.1 `zeta-exec`

非交互宿主：

1. 启动 `AppServerSession`；
2. 从 client handle 发送 Session/Thread/Turn typed request；
3. 同时消费 request result 与 `AppServerEvents`；
4. 根据 canonical Turn terminal update 产生 human/JSON/JSONL 输出和退出码；
5. Ctrl-C 时按产品策略发送 `turn/interrupt`；
6. 最后显式 shutdown session。

它不自行创建 App Server、初始化 connection 或实现 notification pump。

后续 scheduler-facing protocol、worker registration、lease、heartbeat、Job/Attempt mapping 和
event ack 属于 [`exec.md`](exec.md)。App Server Client 只提供 embedded/remote App Server
connection，不理解 scheduler Job。

### 9.2 `zeta-tui`

TUI：

1. 启动同一种 `AppServerSession`；
2. 把 client handle 交给 command dispatcher；
3. 把 `AppServerEvents` 接入 terminal/app event loop；
4. request 执行期间继续处理键盘、重绘和 server update；
5. 退出前恢复 terminal，并显式 shutdown session。

TUI 不再接收一个同步 `&mut AppServerClient<T>`，也不调用 `drain_notifications()`。

## 10. 当前实现审计

当前实现中可以保留的部分：

- `start_in_process_client` 已经体现“由共享 crate 创建本地 App Server”的正确方向；
- typed client methods；
- protocol method registry；
- external JSON-RPC request/response 编解码；
- response ID 校验；
- schema hash 校验；
- successful `InitializeResult` 保存在 `AppServerClient::initialization` snapshot 中，consumer
  可读取 server capabilities 与动态 slash catalog，而无需重复 handshake；
- known notification typed decode。

需要替换的部分：

| 当前实现 | 问题 |
| --- | --- |
| `JsonRpcTransport::round_trip` | 请求、响应和事件被绑成同步调用，不能形成持续 connection driver |
| `drain_notifications` | notification 只能在 request 完成后批量拉取 |
| `AppServerClient<T>` 要求 `&mut self` | TUI 被阻塞，多个 feature 不能共享请求 handle |
| `start_in_process_client` 只返回 client | 没有 session owner、event receiver 或显式 shutdown |
| `InProcessTransport` 隐式拥有 server | 生命周期藏在 transport drop 中，不能 join runner |
| initialize gate 只存在于一个 helper | 裸 `AppServerClient::new` 可以在未初始化时发送业务请求 |
| server error 被压成 code/string | 丢失 typed error name/data |
| App Server notification queue 依靠 drain | server event 无法主动唤醒 client/TUI |

因此当前实现不是“client crate 不该启动 App Server”，而是“启动后没有把请求、事件与关闭组成
一个完整的 owned session”。

## 11. 目标模块

建议按所有权拆分，而不是按 RPC domain 复制 protocol：

```text
app-server-client/src/
├── lib.rs
├── session.rs
├── start.rs
├── client.rs
├── events.rs
├── driver.rs
├── embedded.rs
├── remote.rs
├── pending.rs
├── shutdown.rs
├── error.rs
├── session_tests.rs
└── driver_tests.rs
```

- `session.rs`：顶层 owner 与 ready/closing/closed lifecycle；
- `start.rs`：App Server composition、channel 创建和 initialize；
- `client.rs`：cloneable typed request handle；
- `events.rs`：单消费者 event stream；
- `driver.rs`：request/result/notification/shutdown multiplexing；
- `embedded.rs`：typed in-process channel 与 local App Server runner；
- `remote.rs`：相同 contract 的 remote App Server transport；
- `pending.rs`：request ID 与 completion pairing；
- `shutdown.rs`：关闭顺序与 task join；
- `error.rs`：startup/client/protocol/server/shutdown error。

不创建第二套 Session/Thread/Turn DTO，也不按 method 建大量重复 wrapper 模块。Wire Params、
Result、Notification 与 error 的 source of truth 仍是 `zeta-app-server-protocol`。
新增 test module 使用显式 `#[path = "..._tests.rs"]` 引入相邻测试文件。

## 12. 验证要求

共享层的测试重点是完整 session 生命周期：

- `start` 创建 App Server 并只在 initialize/schema gate 成功后返回；
- initialize 是 dispatcher 收到的首个 request；
- startup 任一步失败都会关闭 channel 并 join 已启动 task；
- `zeta-exec` 与 TUI 使用相同 start path；
- embedded 与 remote backend 通过相同 request/event contract suite；
- 多个 client clone 可以并发发送 request，result 按 request ID 正确配对；
- request 执行期间 event stream 可以持续收到 notification；
- 空闲时由其他 runtime activity 产生的 notification 无需 polling request 即可到达；
- response 先于其 causal notification 交付；
- event channel 满不会静默丢失 durable update；
- connection failure 会结束全部 pending request；
- shutdown 后新 request 明确失败；
- shutdown 会关闭 subscription/Resource ownership 并 join 全部 task；
- event receiver、client handle 和 session owner 以不同顺序 drop 时都不会泄漏；
- in-process typed channel 仍经过 protocol dispatcher，不存在 Core 私有旁路；
- remote backend reconnect 不会偷偷复用旧 subscription、Resource ownership 或 transient
  cursor。

验证入口：

```bash
cargo test --manifest-path zeta-rs/Cargo.toml -p zeta-app-server-client
```

修改 wire contract 时，还必须按
[`zeta-app-server-api.md`](zeta-app-server-api.md#11-source-of-truth) 重新生成 schema 与
TypeScript，并验证所有 consumer。
