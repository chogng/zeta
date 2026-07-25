# zeta-rs 产品内核与统一对外层

> 负责人：zeta-rs 开发者  
> 客户端：Zeta Desktop、Zeta CLI、未来 IDE 插件和 daemon

## 1. 责任

`zeta-rs/` 是完整 Rust 产品实现；`zeta-rs/core/` 是领域内核。

zeta-rs 负责：

- Thread、Turn、Item、Tool Call 状态机；
- Agent、模型调用和工具循环；
- sandbox、审批、超时、取消和输出上限；
- rollout 权威日志、SQLite 投影、恢复和 writer lease；
- Config 和 Credential Store；
- App Server 产品 API、dispatcher、client 和 transport；
- Rust、TypeScript、JSON Schema 生成和 contract tests。

zeta-rs 不负责：

- Desktop UI、窗口或 CDP 具体实现；
- CLI 参数解析、终端渲染或 shell completion；
- Renderer 状态和 Electron IPC；
- 第三方网页 UI。

## 2. Workspace 边界

```text
zeta-rs/
├── core/
├── protocol/
├── app-server-protocol/
├── app-server-transport/
├── app-server-client/
├── app-server/
├── config/
├── credentials/
├── storage/
├── exec/
├── sandboxing/
├── built-in-tools/
├── model-provider/
├── zeta-api/
├── tui/
└── cli/
```

不建立 `zeta-runtime`。也不建立以 `runtime`、`service`、`common` 或 `platform` 命名的泛化
聚合 crate。

应用用例由 App Server dispatcher 暴露；本地 adapter 组合位于
`app-server/src/local.rs` 这一明确的宿主组合入口。该文件只负责装配，不拥有领域规则。

## 3. 唯一产品契约

Zeta 只有一个跨产品业务接口：

```text
zeta-app-server-protocol
```

它定义版本化：

- Client → Server requests；
- Server → Client requests；
- Server notifications；
- Params、Result、Resource 和稳定错误；
- capability 与协议版本协商。

CLI、Desktop、daemon 和远程客户端不得另外建立语义重复的产品 API。

当前 accepted 基线是
[`zeta-app-server-api-v1.md`](zeta-app-server-api-v1.md)。

## 4. 客户端与 transport

```text
CLI/TUI
  → zeta-app-server-client
  → InProcess transport
  → App Server dispatcher

Desktop
  → generated TypeScript client
  → JSONL / stdio
  → App Server dispatcher

daemon / remote
  → Unix socket / WebSocket
  → App Server dispatcher
```

进程内 transport 是性能优化，不是语义捷径。它必须经过协议编码、request ID pairing、
initialize、dispatcher、typed response 和 notification 解码。

## 5. 依赖方向

```text
zeta-core
  → zeta-protocol

zeta-api
  → normalized Zeta model protocol + provider-specific wire adapters

model-provider
  → zeta-api + zeta-core + credentials

storage/config/credentials/exec
  → zeta-core

app-server-protocol
  → zeta-protocol 的稳定叶子类型

app-server
  → zeta-core + adapters + app-server-protocol + transport

app-server-client
  → app-server-protocol
  → app-server（仅进程内宿主实现）

CLI
  → app-server-client + app-server-protocol + tui
  → app-server（仅 `zeta app-server` 宿主子命令）

Desktop
  → JSON-RPC → app-server
```

Core 不依赖 Storage、Exec、Model Provider、App Server、CLI 或 Desktop。

## 6. 防止职责越界

- Core 保存领域规则，不了解 JSON-RPC、终端或 Electron；
- App Server handler 负责用例编排、DTO mapper、路由和协议错误；
- `local.rs` 只选择 adapter 和恢复持久化状态；
- Client 负责请求配对、通知解码、连接和重连；
- Transport 只负责有界消息传输；
- CLI 与 Desktop 只负责各自 presentation 和 host capability；
- 协议 DTO 不直接复用内部 aggregate；
- 不能用新的泛化 facade 绕开 App Server。

某段代码如果同时依赖 Core、Storage、Exec、Config、Model Provider 和 UI 类型，应先判断它
是否确实是 App Server 的组合入口；否则视为边界设计问题。

## 7. App Server API 实现流程

Desktop 或 CLI 开发者提交符合
[`zeta-api-interface-requirements.md`](zeta-api-interface-requirements.md) 的产品接口需求。
zeta-rs 是已接受 App Server 契约的最终 owner。

实现顺序：

1. 审核客户端覆盖、方向、所有权、安全和生命周期；
2. 在 `app-server-protocol/v1` 定义冻结 DTO；
3. 定义内部模型与 DTO 的显式 mapper；
4. 实现 dispatcher 和 handler；
5. 实现 connection、subscription、turn owner 和 capability owner 路由；
6. 实现 idempotency、deadline、取消和错误码；
7. 同时扩展进程内 client 和外部 transport contract tests；
8. 生成 TypeScript 与 JSON Schema；
9. 交付二进制、生成文件、schema hash 和变更说明。

接口文档未说明的行为不能由 Rust 实现自行猜测。

## 8. 持久化

每个 Thread 使用独立 rollout，记录至少包含 schema version、Thread ID、单调 sequence、
event ID、recorded time、event kind、payload 和 checksum。

完成 Item、审批决定和 Turn 终态必须先 durable commit，再更新内存投影和通知客户端。

SQLite 只做可重建查询投影。启动时检测不完整尾记录，并把未完成 Turn 标为 Interrupted。
同一持久化 Thread 同时只允许一个 writer lease。

## 9. App Server 必须保证

- `initialize` 是首个请求；
- 版本区间无交集时拒绝连接；
- 同一 Thread 修改串行，不同 Thread 可并发；
- `thread/read` 不订阅，`thread/resume` 原子返回 snapshot 并订阅；
- 同一 Thread 可有多个订阅 connection；
- Server → Client 请求按 turn owner 或 capability owner 路由；
- 副作用方法使用持久化 idempotency key；
- durable sequence 和 streamSeq 可检测空洞；
- transport 队列有界；
- ResourceRef 有 owner、TTL、digest、quota 和 chunk 上限；
- 连接断开时清理订阅、pending request 和 resource；
- 进程内与外部 transport 的可见行为一致。

## 10. Browser Capability

Core 只定义语义 trait 和类型，不依赖 Electron 或 CDP。

App Server 将 Core capability 调用转换为 Server → Client JSON-RPC；Desktop Electron Main
实现 Browser host。CLI 没有浏览器时通过 initialize 声明 capability 不可用。

## 11. 测试与交付门

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

协议变更还必须验证：

```text
进程内 App Server Client contract test
stdio/JSONL contract test
TypeScript 重新生成且可编译
JSON Schema 与 schema hash 更新
当前与前一兼容版本 fixtures 通过
Desktop 和 CLI fixture 通过
```

## 12. 当前优先级

1. 维护 accepted App Server API v1 与实现一致；
2. 通过新契约 revision 完成 typed notification、双向请求和异步 Turn；
3. 完成多连接订阅和资源生命周期；
4. 按 Desktop Browser API 文档实现 host capability；
5. 扩展 CLI JSON/JSONL 映射与 approval UI；
6. 再扩展 daemon、WebSocket、PDF 和后台 Turn。
