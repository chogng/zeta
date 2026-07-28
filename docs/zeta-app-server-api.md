# Zeta App Server API

```yaml
title: Zeta App Server API
status: development
owner: zeta-rs
consumers:
  - desktop
  - cli
lastUpdated: 2026-07-25
```

本文描述当前开发期的唯一 App Server 契约。项目不保留旧 wire API、旧 DTO 或旧持久化格式
的兼容入口；Rust DTO、生成的 TypeScript 和 JSON Schema 必须始终一致。

具体 method registry、artifact generator 与 schema fixture 见
[`zeta-app-server-protocol` README](../zeta-rs/app-server-protocol/README.md)；JSON-RPC dispatch、
subscription broker、resource store 与 local composition 见
[`zeta-app-server` README](../zeta-rs/app-server/README.md)。本文拥有跨客户端 API 语义与演进方向，
两个 README 拥有当前实现接口与修改路径。

## 1. 产品模型

Canonical 产品实体和内部契约的详细定义见 [`protocol.md`](protocol.md)。本 API 直接暴露
其中的 readable Session/Thread/Turn/ThreadItem view，不维护第二份领域定义。

- App Server connection/session 只是传输生命周期，不能与产品 Session 混用。

Session 不嵌入 Thread 历史，只保存 membership、lineage 和 lifecycle。Fork 的 lineage 固定为
`parentThreadId + parentSequence`，因此父 Thread 后续继续执行不会改变已创建分支的起点。

## 2. 一致性模型

每个修改命令都使用：

- `commandId`：客户端生成的稳定命令身份；
- `expectedSequence`：客户端观察到的目标 aggregate sequence；
- typed command payload：参与重放与冲突判断。

同一 `commandId + typed payload` 重试返回原结果；同一 `commandId` 携带不同 payload 返回
`CommandConflict`。JSON-RPC request `id` 只做当前 connection 的 response pairing，不能替代
`commandId`。

Session 与每个 Thread 拥有独立 durable sequence：

- Session sequence 只排序 topology 与 lifecycle；
- Thread sequence 只排序该分支内的 Turn 与 Item；
- 修改一个 Thread 不会占用 Session 或其他 Thread 的 sequence；
- `expectedSequence` 始终针对 method 所修改的 aggregate。

## 3. Transport

当前外部 transport 是 UTF-8 JSONL/stdio：

- 每行一个完整 JSON-RPC 2.0 message；
- 单条 message 最大 1,048,576 bytes；
- stdout 只允许协议 message，stderr 只用于诊断；
- 同一 request 的 response 先于由它产生的 causal notifications；
- connection 断开后 request ID、subscription 和 Resource ownership 全部失效。

In-process client 使用 protocol-owned typed request/event channel，可以省略 JSON string
编解码，但必须经过相同 initialize gate、method dispatcher、result/error envelope 与
notification contract，不能拥有隐藏业务接口。JSONL/stdio、WebSocket 等外部 transport 才在
边界执行 wire encoding。

## 4. Initialize

`initialize` 必须是 connection 的首个 request。

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "clientInfo": { "name": "zeta-desktop", "version": "0.1.0" },
    "capabilities": { "notifications": true }
  }
}
```

返回值包含 `serverInfo`、完整 schema 的 `schemaHash`，以及：

```json
{
  "sessions": true,
  "threads": true,
  "turns": true,
  "resources": true,
  "updateReplay": true
}
```

schema hash 不一致时客户端必须拒绝继续运行。

## 5. Method inventory

| Method | Aggregate | Effect |
| --- | --- | --- |
| `session/create` | new Session | 创建任务 |
| `session/read` | Session | 读取 canonical snapshot |
| `session/list` | global | 列出 Session |
| `session/subscribe` | connection | snapshot + `afterSequence` 之后的 durable gap |
| `session/unsubscribe` | connection | 删除订阅 |
| `session/thread/create` | Session + new Thread | 创建 root Thread |
| `session/thread/fork` | Session + new Thread | 从固定 parent sequence 创建分支 |
| `session/thread/archive` | Session | archive membership |
| `session/complete` | Session | 完成任务 |
| `session/archive` | Session | archive 任务 |
| `thread/read` | Thread | 读取 canonical snapshot |
| `thread/subscribe` | connection | snapshot + `afterSequence` 之后的 durable gap |
| `thread/unsubscribe` | connection | 删除订阅 |
| `turn/start` | Thread | 接受并执行 Turn |
| `turn/interrupt` | Thread | 中断非终态 Turn |
| `turn/interaction/resolve` | Thread | 用 exact request identity 解决一个 outstanding interaction |
| `config/read` | config | 读取配置 |
| `config/update` | config | typed command 更新配置 |
| `resource/metadata` | Resource | 读取元数据 |
| `resource/read` | Resource | 分块读取 |
| `resource/release` | Resource | 释放 connection-owned resource |

长期 account control plane 另见[第 11 节](#11-account-与登录)。它尚未进入当前 registry/schema，
加入时必须和 Rust DTO、TypeScript 与 JSON Schema 同步提交。

## 6. Session commands

### Create

```json
{
  "method": "session/create",
  "params": {
    "commandId": "command_session_1",
    "title": "Investigate repository"
  }
}
```

返回 `{ "session": Session }`。首次 durable event 为 `sessionCreated`，并在同一 atomic batch
保存 typed command receipt。

### Create Thread

```json
{
  "method": "session/thread/create",
  "params": {
    "commandId": "command_thread_1",
    "sessionId": "session_1",
    "expectedSequence": 1,
    "title": "Main"
  }
}
```

创建采用可恢复 saga：

1. Session 写入 `threadCreationPlanned`，membership 状态为 `creating`；
2. 创建带相同 `sessionId` 的 Thread stream；
3. Session 写入 `threadAttached`，membership 状态变为 `active`。

恢复时发现 `creating` membership 会继续完成后两步，而不是创建另一个 Thread。

### Fork Thread

`session/thread/fork` 比 create 多一个 `parentThreadId`。Server 在执行命令时读取父 Thread 的
当前 sequence，并把它持久化进 `ThreadOrigin::Fork`。Fork 只复制 lineage 起点；它不让两个
Thread 共享后续 sequence。

### Lifecycle

`session/thread/archive`、`session/complete` 和 `session/archive` 都要求 `commandId`、
`sessionId` 与 `expectedSequence`。Archived Session 不允许再修改。

## 7. Thread 与 Turn

`thread/read` 返回：

```text
Thread {
  sessionId,
  threadId,
  title,
  status,
  sequence,
  turns: Turn[]
}
```

每个 Turn 始终包含完整的 `items: ThreadItem[]`、可选 `pendingInteraction` metadata 与可选稳定
错误。`pendingInteraction` 不含 interaction payload；完整请求只能通过 owner-directed delivery
获得。客户端不得从日志文本或瞬态 delta 推断权威终态。

`turn/start` 参数：

```json
{
  "commandId": "command_turn_1",
  "sessionId": "session_1",
  "threadId": "thread_1",
  "expectedSequence": 1,
  "input": [{ "type": "text", "text": "Explain this repository" }]
}
```

acceptance、user items 与 started facts 作为一个 atomic Thread batch 提交。最终 Agent item
与 completed fact 也作为一个 atomic batch 提交。Provider 失败时持久化稳定
`StableTurnError`；持久化失败时内存投影不得伪造终态。

`turn/interrupt` 同样携带 `commandId`、Session/Thread/Turn identity 与
`expectedSequence`，成功返回新的 Thread sequence。

`turn/interaction/resolve` 携带同样的 aggregate identity、`commandId`、`expectedSequence`，
以及 outstanding interaction 的 `requestId` 和 typed response。它只接受该 Turn 当前 pending
interaction 的同一 request kind；相同 `commandId + typed payload` 会重放原结果，错误的
`requestId` 或 response kind 会被拒绝。该 method 解决已 durable 的 interaction，不用于创建
新的 Agent request。

当前同步 App Server 还没有实现 Server → Client request owner selection 或主动投递；未来 runtime
必须将 connection ownership 作为短暂 delivery state，而不是写入 Thread snapshot 或 event。

## 8. Update stream

只有两个 notification method：

- `session/update`，payload 为 `SessionUpdateEnvelope`；
- `thread/update`，payload 为 `ThreadUpdateEnvelope`。

durable update 使用 `durableSequence`。Thread 的低延迟非 durable update 可额外携带
`streamCursor { streamInstanceId, sequence }`，两者不能混为一个计数器：

- durable sequence 可用于恢复、重放和 optimistic concurrency；
- stream cursor 只用于检测当前 runtime 的瞬态 update 空洞；
- streamInstanceId 变化时客户端丢弃旧瞬态 cursor，并以 durable snapshot/gap 重新同步。

`session/subscribe` 与 `thread/subscribe` 原子建立订阅并返回当前 snapshot 以及
`afterSequence` 之后的 committed update gap。客户端应先应用 snapshot/gap，再接收实时
notification；发现 durable 空洞时重新 subscribe。

## 9. Config 与 Resource

`config/update` 使用 `commandId`。Patch 字段三态语义为：

- 缺失：不修改；
- `null`：清除；
- value：替换。

Resource bytes 使用标准 RFC 4648 Base64；`decodedLength` 是原始 byte 数，单 chunk 最大
262,144 bytes。客户端用 `decodedLength` 推进 offset，并在结束后校验 size 与 SHA-256。

## 10. Stable errors

标准 JSON-RPC errors 为 `ParseError`、`InvalidRequest`、`MethodNotFound` 和
`InvalidParams`。产品稳定错误包括：

- `NotInitialized`
- `AlreadyInitialized`
- `ServerOverloaded`
- `CommandConflict`
- `CoreOperationFailed`
- `ResourceNotFound`
- `ResourceNotOwner`
- `ResourceTooLarge`
- `InvalidResourceChunkSize`
- `InvalidResourceOffset`
- `ConfigUnavailable`

当前 `error.data` 为 `null`。客户端必须匹配稳定 code/name，不能解析人类错误文本。

## 11. Account 与登录

Account 是 App Server 暴露给客户端的 redacted 控制面，不是 secret/token authority。长期 method：

```text
account/read
account/login/start
account/login/cancel
account/logout

account/login/completed
account/updated
```

第一阶段只支持：

```rust
pub enum AccountLoginMethod {
    ApiKey { provider: ProviderId },
    OpenAiChatGptBrowser,
    OpenAiChatGptDeviceCode,
}
```

Provider 是否支持 interactive login、credential 的实际所有者和 refresh 语义由
[`zeta-login`](login.md) 的 exact driver 决定。对 ChatGPT/Codex subscription，该 driver 是
[`zeta-codex-app-server`](codex-app-server.md)：它委托上游 Codex App Server 完成浏览器/设备码
登录及 refresh。Zeta App Server 只编排和映射 redacted control plane：

```text
app-server-protocol/src/protocol/account.rs
  └─ login/account request、redacted result、notification DTO

app-server/src/server/account/
  ├─ mod.rs       ── start/cancel/read/logout dispatch 与 notification
  ├─ login.rs     ── LoginService composition 与 redacted RPC mapping
  └─ account_tests.rs

login/
  └─ user-visible login lifecycle 与 redacted account projection

codex-app-server/
  └─ upstream process、JSON-RPC、managed login 与 subscription runtime adapter

zeta-secrets
  └─ direct-provider/API-key 或 exact OAuth-owner 的 opaque secret bytes
```

Browser 打开、URL 展示和 device code UI 属于 Desktop/CLI/TUI。Renderer 可以接收 `loginId`、
authorization URL、一次性 user code 和以下 redacted metadata：

- opaque account ID；
- email/display name（Provider 返回且 UI 需要时）；
- workspace/organization display metadata；
- plan/status；
- credential revision；
- reauthentication required 状态。

禁止进入 RPC/schema：

- access token、refresh token、API key；
- authorization/cookie header map；
- PKCE verifier、authorization code；
- secret-store key 的内部 namespace；
- Codex internal/unstable `chatgptAuthTokens` login variant。

ChatGPT/Codex login 的顺序固定为：

1. App Server 调用 `LoginService::begin`；
2. Codex adapter 向上游发出 `account/login/start`，返回 authorization URL 或 device-code UI 指令；
3. client 打开 URL 或展示 code；
4. 上游 Codex 绑定 callback listener、验证 callback、持久化并刷新 credential；
5. Codex adapter 转发 redacted completed/account-updated event；
6. `zeta-login` 发布 account revision，Zeta App Server 返回 completed 并通知客户端。

Zeta App Server 绝不接触 callback code、OAuth state、PKCE verifier、access/refresh token、
`~/.codex/auth.json` 或上游 keychain entry。`account/login/cancel` 必须取消 exact upstream login；
logout 转发 upstream logout，并将失败映射为稳定的 redacted diagnostic。

## 12. Source of truth

- Rust DTO 与 registry：`zeta-rs/app-server-protocol/src/protocol/`
- JSON Schema：`zeta-rs/app-server-protocol/schema/schema.json`
- TypeScript：`zeta-rs/app-server-protocol/schema/types.ts`
- Desktop 同步产物：`desktop/generated/app-server/types.ts`

修改契约后执行：

```bash
cargo run --manifest-path zeta-rs/Cargo.toml \
  -p zeta-app-server-protocol --bin write_schema_fixtures
node desktop/scripts/sync-app-server-protocol.mjs
```

生成产物、Rust contract tests 和 Desktop TypeScript 编译必须同时通过。

## 13. Typst document compilation

`initialize.capabilities.typst` indicates support for
`document/typst/compile`. The method accepts `{ "source": string }` and returns
either a connection-owned `application/pdf` resource plus warnings, or typed
source diagnostics. Source size is limited to 1 MiB of UTF-8 bytes.

The current compiler exposes only an in-memory `/main.typ`; it does not expose
host files, network access, package downloads, system fonts, or the current
date. PDF bytes use the existing `resource/metadata`, `resource/read`, and
`resource/release` lifecycle. Cross-process ownership and planned evolution
are documented in [`typst.md`](typst.md).
