# `zeta-app-server-client`

> 当前 crate 实现、调用路径、failure/shutdown 语义由本文说明。跨 consumer 的目标架构、
> backend 取舍与后续阶段见 [`docs/app-server-client.md`](../../docs/app-server-client.md)；
> wire contract 见 [`docs/zeta-app-server-api.md`](../../docs/zeta-app-server-api.md)。

## 结论

`AppServerSession` 是 CLI/TUI 的 canonical embedded connection owner。它完成 App Server
composition、connection 创建、正式 initialize/schema gate、request driver、wakeable
notification pump 与显式 shutdown。Consumer 不直接拥有 `AppServer` 或
`ConnectionState`。

同步适配面 `AppServerClient<T>`、`InProcessTransport`、`start_in_process_client` 和
`drain_notifications` 仍供 MCP adapter、rust-app 与 contract tests 使用；它们不是新交互
consumer 的启动入口。新的 CLI/TUI 交互统一通过 `AppServerSession` 获取 request handle
与独立 event stream。

`InProcessClientOptions` 默认选择 `SessionStateMode::Durable`。需要进程内生命周期的 host
必须显式调用 `with_session_state_mode(SessionStateMode::Ephemeral)`；该模式只替换
Session/Thread coordinator 的存储，不改变 Config、Workspace、Tool 或 protocol contract。

| 能力 | Owned session | Synchronous adapter |
| --- | --- | --- |
| ready initialize/schema gate | ✅ `start_embedded` | `start_in_process_client` 时具备 |
| cloneable request handle | ✅ | 仅 `T: Clone` 时可克隆 |
| 独立 wakeable event stream | ✅ | ❌，通过 `drain_notifications` 读取 |
| 空闲/长 Turn notification | ✅ | 需要 consumer polling |
| 有界 event/backpressure | ✅，1024 项 event channel | 由 consumer drain cadence 决定 |
| 显式 connection shutdown/task join | ✅ | 由 host 管理 |
| remote backend | 尚未完成 | ❌ |

## 公共契约

| Symbol | Owner 与 contract |
| --- | --- |
| `AppServerSession::start_embedded` | 创建 embedded composition；initialize/schema 校验失败时关闭已启动 driver |
| `AppServerSession::client` | 返回共享 request ID allocator 的 cloneable `AppServerRequestHandle` |
| `AppServerSession::take_events` | 单次取出 `AppServerEvents`；第二次返回 `TakeEventsError` |
| `AppServerSession::shutdown` | 拒绝后续 request、关闭 connection、唤醒 event pump 并 join tasks |
| `AppServerEvents::{recv,recv_timeout,try_recv}` | 接收 typed notification 与最终 connection close event |
| `AppServerEvent::Notification` | 已由 client boundary 解码的 `ServerNotification` |
| `AppServerEvent::ConnectionClosed` | 明确的 shutdown、driver stop 或 protocol failure |
| `AppServerClient<T>` | typed JSON-RPC client；method 与 DTO source 仍来自 protocol crate |
| `InProcessClientOptions::with_session_state_mode` | 明确选择 profile durable history 或 process-local ephemeral Session/Thread state |
| `InProcessClientOptions::with_model_operation_client` | embedded host/test 注入离线或自定义 model transport；不改变 protocol/model semantics |
| `AppServerClient::request_session` | Session aggregate 的 canonical typed mutation request；所有 Session mutation 统一由此进入 |

正常入口示意（`options`/`params` 由 host 与 protocol DTO 构造）：

```rust
let mut session = AppServerSession::start_embedded(options)?;
let mut client = session.client();
let events = session.take_events()?;

let result = client.request_session(params)?;
let event = events.recv();

session.shutdown()?;
```

Typed method 当前同步等待该 request 的 completion，但实际 dispatch 位于独立 driver。
Notification 不依附 request completion；consumer 不得对 session handle 调用
`drain_notifications()`。

## 文件与职责

| 文件 | 当前 owner |
| --- | --- |
| `src/session.rs` | owned session、session transport、request/event threads、shutdown 与 connection lifecycle |
| `src/in_process.rs` | embedded composition 与 initialized connection |
| `src/profile.rs` | `ZETA_PROFILE_ROOT` 与 host-wide default profile state path |
| `src/lib.rs` | generic typed JSON-RPC methods、request ID/result pairing 与 public exports |
| `src/notification.rs` | wire notification method 到 `ServerNotification` 的 typed decode |
| `src/session_tests.rs` | owned lifecycle、idle wakeup、clone identity 与 shutdown contract |
| `src/client_tests.rs` | JSON-RPC pairing、schema、Session contract、Skill catalog/watcher contract |

## 执行路径

```text
AppServerSession::start_embedded
├─ resolve/receive profile root
├─ open_in_process_app_server
├─ AppServer::connection
├─ AppServer::connection_notifications
├─ start connection driver
├─ start notification pump
├─ AppServerClient::initialize
│  └─ SessionTransport → request channel → AppServer::handle_json
├─ validate schema hash
└─ return ready session

AppServerRequestHandle typed method
├─ shared atomic request ID
├─ bounded DriverCommand queue
├─ connection driver → AppServer::handle_json
└─ per-request completion → typed decode

App Server background update
├─ UpdateBroker → NotificationQueue::push/extend
├─ condition variable wake
├─ notification pump → typed decode
└─ bounded AppServerEvents (1024)
```

Connection driver 在 request response completion 发送之前持有 delivery barrier；event pump
通过同一 barrier drain queue，因此同一 request 的 response 先于 causal notification 交付。

## 失败与关闭

- request channel 已关闭：返回 `ClientError::Transport`；
- response ID/envelope/decode 不合法：返回 `ClientError::Protocol`；
- server failure：保留 code/message 为 `ClientError::Server`；
- known notification decode 失败：event stream 发送 `ProtocolFailure` 并关闭 connection；
- event receiver 被 drop：pump 关闭 connection，surviving client clone 的后续 request 失败；
- event channel 满时 pump 背压 App Server 的 4096 项 connection queue；该 queue 先清可重建
  transient，control-only overflow 关闭 connection，不静默丢 durable/control fact；
- 正常退出必须调用 `shutdown`；`Drop` 只做 best-effort signal，不等待 task；
- `shutdown` 后仍存在的 client clone 不延长 connection 生命周期。

当前 event channel 已有界；transient purge 依赖 consumer 的 stream cursor gap → snapshot resync。
显式 `Lagged` event 与 remote reconnect 尚未实现，connection/control overflow 后 consumer 仍需按
`ConnectionClosed` 处理恢复。

## 内部所有权与漂移信号

- `session::drive_connection` 唯一拥有 `ConnectionState` 与 request dispatch；
- `session::pump_notifications` 唯一解码 session outbound notification；
- `server::notification_queue::NotificationQueue`（App Server crate）拥有 wake/close queue primitive；
- `AppServerClient::call` 拥有 request ID、JSON-RPC pairing 与 typed result decode；
- `in_process::initialize_client` 只服务 embedded startup。

以下变化表示 architecture drift，需要同步系统文档与测试：

- CLI/TUI 恢复调用 `start_in_process_client` 或 `drain_notifications`；
- consumer 直接创建 `AppServer`/`ConnectionState`；
- notification 再次只在 request 后被 drain；
- session lifetime 由最后一个 client clone 的 drop 决定；
- request driver 绕过 `AppServer::handle_json` 直连 Core/store；
- shutdown 不再 join session 启动的 tasks。

常见修改影响：

| 修改 | 必须同步检查 |
| --- | --- |
| 新 typed RPC method | protocol registry/DTO、`AppServerClient` method、wire contract tests |
| notification variant | protocol registry、`notification::decode`、session/TUI mapping tests |
| request/event queue policy | causal barrier、shutdown unblock、durable loss/lag contract 与系统文档 |
| connection-owned resource | `AppServer::close_connection` cleanup、cross-connection rejection tests |
| 新 backend | initialize/schema gate、同一 session contract suite、failure/reconnect ownership |

## 验证

```bash
cargo test --manifest-path Cargo.toml -p zeta-app-server-client
cargo test --manifest-path Cargo.toml -p zeta-tui
cargo test --manifest-path Cargo.toml -p zeta-cli
```

Session tests覆盖空闲连接 notification、Turn completion event、有界 channel 关闭不死锁、显式
shutdown、最终 `ConnectionClosed` 与 surviving clone rejection。Contract tests继续覆盖 JSON-RPC
pairing、schema gate、canonical Session mutation、Agent request、Skill watcher 和同步 client surface。
