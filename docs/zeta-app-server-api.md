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

Current [`zeta-mcp-server`](mcp-server.md) stdio/Streamable HTTP surface 是本 API 的外层
Agent-as-tool adapter；它通过 App Server client 复用这里定义的 Session/Thread/Turn contract，
不建立第二套 execution API。该 adapter 已将 Thread subscription/update 投影为 MCP progress，
将 approval/user-input 映射为 form elicitation，并在自己的 state root 中持久化外部 invocation
receipt。Receipt 只拥有 MCP correlation/recovery，不改变本 API 的 durable authority。

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
- connection 断开后 request ID、subscription、Resource 与 Terminal ownership 全部失效。

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

返回值包含 `serverInfo`、完整 schema 的 `schemaHash`、server capability，以及 composition
边界冻结的 `slashCommands` snapshot：

```json
{
  "capabilities": {
    "sessions": true,
    "threads": true,
    "turns": true,
    "resources": true,
    "fileSystem": true,
    "workspaceSearch": true,
    "terminal": true,
    "typst": true,
    "updateReplay": true
  },
  "slashCommands": [
    {
      "name": "diagnose",
      "description": "inspect the current workspace",
      "argumentMode": "optional"
    }
  ]
}
```

schema hash 不一致时客户端必须拒绝继续运行。
`slashCommands` 每项的 `name` 只能使用 lowercase ASCII letters、digits 与 interior hyphens，
description 不能为空，同一 snapshot 中 name 必须唯一。该 snapshot 负责 discoverability 与
inline argument parsing；提交仍通过 `turn/start.input`，并保留 `/name`、text/image 顺序。

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
| `session/model/set` | Session | 持久化当前 Session 的模型选择 |
| `model/list` | global model catalog | 列出当前已配置 provider 可选择的模型 |
| `thread/read` | Thread | 读取 canonical snapshot |
| `thread/subscribe` | connection | snapshot + `afterSequence` 之后的 durable gap |
| `thread/unsubscribe` | connection | 删除订阅 |
| `turn/start` | Thread | 接受并执行 Turn |
| `turn/interrupt` | Thread | 中断非终态 Turn |
| `turn/interaction/resolve` | Thread | 用 exact request identity 解决一个 outstanding interaction |
| `config/read` | config | 读取配置 |
| `config/update` | config | typed command 更新配置 |
| `skills/list` | global Skill catalog | 读取 cached projection 或请求完整 refresh |
| `skill/enablement/set` | config + Skill catalog | revision-checked 启用/禁用 exact `SkillId` |
| `resource/metadata` | Resource | 读取元数据 |
| `resource/read` | Resource | 分块读取 |
| `resource/release` | Resource | 释放 connection-owned resource |
| `fs/getMetadata` | workspace | 读取根相对路径的 metadata |
| `fs/readDirectory` | workspace | 枚举根相对目录的直接子项 |
| `fs/readFile` | workspace | 读取不超过 10 MiB 的 UTF-8 文件 |
| `git/status` | workspace | 读取 HEAD、upstream 和 index/worktree change snapshot |
| `git/stage` | workspace | stage 一组 workspace-relative path |
| `git/unstage` | workspace | 从 index 移除一组 path 的 staged change |
| `git/discardWorktree` | workspace | 恢复 tracked working-tree change，不删除 untracked 文件 |
| `git/commit` | workspace | 使用有界非空 message 创建 commit |
| `git/fetch` | workspace | non-interactive fetch all remotes 并 prune |
| `git/pull` | workspace | non-interactive fast-forward-only pull |
| `git/push` | workspace | 按当前 Git upstream/default 配置 push |
| `workspace/search/start` | connection + workspace | 启动有界内容搜索 |
| `workspace/search/read` | connection + search job | 按游标读取最多 200 条结果 |
| `workspace/search/cancel` | connection + search job | 取消并释放搜索 |
| `terminal/profile/list` | workspace | 列出 App Server 冻结的可信 Shell Profile |
| `terminal/create` | connection + workspace | 在可信 workspace root 启动 PTY |
| `terminal/write` | connection + Terminal | 写入有界 UTF-8 输入 batch |
| `terminal/resize` | connection + Terminal | 修改 PTY rows/cols |
| `terminal/read` | connection + Terminal | 按 sequence 拉取有界 Base64 输出 |
| `terminal/close` | connection + Terminal | 终止并释放 PTY |

长期 account control plane 另见[第 11 节](#11-account-与登录)。它尚未进入当前 registry/schema，
加入时必须和 Rust DTO、TypeScript 与 JSON Schema 同步提交。

### Filesystem

Filesystem method 的 `path` 是配置 workspace root 下的相对路径；空字符串表示 root。
绝对路径、父目录逃逸和解析后越过 root 的 symlink 会在可信 Rust 边界被拒绝。Desktop 的
IPC 层会先做同形状校验以便快速失败，但不承担最终授权。

当前 contract 提供 `fs/getMetadata`、`fs/readDirectory` 和 `fs/readFile`，用于单根
Folder 的 Explorer 与文本文件打开。`fs/readFile` 只接受不超过 10 MiB 的 UTF-8 文件。
Filesystem contract 本身不包含写入、重命名、删除、watcher、多根 Workspace，也不保证
跨请求 snapshot 一致性。客户端应把目录展开状态和重新加载策略视为可丢弃 UI 状态。

### Git SCM

`initialize.capabilities.git` 表示 server 已冻结可信 workspace root 并安装 Git backend。
`git/status` 不接受路径参数，返回 `GitStatusResult`：标识当前 Git runtime incarnation 的
`streamInstanceId`、在该实例内单调递增的 workspace status revision、
HEAD 的 branch/detached/unborn
状态、可选 upstream ahead/behind，以及每个 workspace-relative path 的 index/worktree status、
rename original path、conflict 和 submodule flags。Revision 表示 App Server 观察到的投影版本，
不是 durable CAS token，客户端不能把一次 snapshot 当作后续 mutation 的 compare-and-swap 前提。
当 workspace 是更大 repository 的子目录时，server 会过滤 workspace 外 change，并将保留路径
重新映射为 workspace-relative。

App Server 通过 `zeta-file-watcher` 接收 workspace、Git metadata 和相关 ancestor `.gitignore`
invalidation hint，100ms debounce 后重新读取 authoritative Git status。投影内容变化时 revision
递增并向支持 notification 的连接发送 `git/statusChanged { status }`；内容未变时不发送。
Watcher 初始化失败时显式 `git/status` 与 mutation 仍可用。客户端只在相同
`streamInstanceId` 内比较 revision 并忽略不大于当前 revision 的通知或响应；实例变化表示
App Server 已重启，客户端必须接受新 snapshot，并在连接重新 ready 时主动执行 `git/status`
恢复权威状态。

Mutation contract 提供 `git/stage`、`git/unstage`、`git/discardWorktree`、`git/commit`、
`git/fetch`、`git/pull` 和 `git/push`。Path mutation 接受 1–5000 个 workspace-relative path；
Rust service 负责最终边界校验和 repository-relative 映射。Commit message 必须非空、无 NUL，
且不超过 64 KiB UTF-8。每个成功 mutation 都返回新的 status；commit 另外返回 object ID。

Remote operation 禁用 terminal/credential prompt，pull 固定使用 fast-forward only。Discard 只恢复
tracked working tree，不删除 untracked 文件。Operation 在单 workspace runtime 内串行执行；当前
没有可观测 queue、progress 或 caller cancellation。跨层 ownership、当前 UI 和演进顺序见
[`git.md`](git.md)。

### Workspace Search

`initialize.capabilities.workspaceSearch` 表示 server 已安装 workspace 内容搜索 backend。
客户端通过 `workspace/search/start` 获得 connection-owned `searchId`，用
`workspace/search/read` 的 `afterMatch` cursor 分批读取结果，最后调用
`workspace/search/cancel` 释放作业。每个结果包含 workspace-relative path、1-based line
number、单行 preview 和 UTF-16 match ranges。

查询、glob、batch 和总结果都有协议上限；Rust backend 重新校验 workspace 边界并直接启动
冻结的 ripgrep executable。未知 ID、跨 connection 访问和并发超限使用稳定的
`SearchNotFound`、`SearchNotOwner` 与 `SearchBusy` error name。执行失败作为 terminal
read result 的脱敏 `error` 返回。完整 ownership 与当前 UI 限制见
[`search.md`](search.md)。

### Integrated Terminal

`initialize.capabilities.terminal` 表示 local composition 已提供可信 workspace root 和 PTY
runtime。`terminal/profile/list` 只返回稳定 `profileId`、显示标题与 default 标记，不暴露
program、args 或 environment。`terminal/create` 接受 rows/cols 和 `default | profileId`
tagged selection；Rust owner 把 ID 解析到冻结的本机 Shell Profile，并以显式 environment
allowlist 在 workspace root 启动。客户端不能提交任意 executable、environment 或绝对 cwd。
Terminal ID 绑定创建它的 App Server connection，跨 connection 操作返回 `TerminalNotOwner`。

当前同步 JSONL transport 不支持独立于 request 的高频主动输出。客户端通过
`terminal/read { terminalId, afterSequence, maxChunks }` 拉取最多 128 个 raw-byte chunk；
每个 chunk 使用标准 Base64，并以单调 sequence 排序。Server 保留最多 1 MiB 输出，cursor
落后于 ring 时返回 `outputGap: true`，客户端必须显式显示截断而不能把缺口当作连续输出。
`exited` 只在 authoritative process exit 且尾部输出流关闭后为 true。

当前 terminal 不持久化、不跨 App Server 重启恢复，也不支持环境变量修改或远程 attach。
正常客户端在实例关闭后调用 `terminal/close`；connection 结束时 server 终止该 connection
拥有的剩余 PTY。App Server 重启后的显式 Relaunch 会创建新 PTY，不能冒充原进程恢复。

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

### Session model

`session/model/set` 使用 `commandId`、`sessionId`、`expectedSequence` 和 provider-scoped
`ModelRef`。选择结果写入 Session event stream，只影响该 Session。`turn/start` 将 Session
当前模型复制到新 Turn；后续 Session 或全局配置变化不会改变已经启动的 Turn。

`model/list` 只返回 App Server 当前已配置 provider 的静态目录。远端账号 entitlement 和模型
实际可用性仍在调用时验证。

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
  "input": [
    { "type": "text", "text": "Describe this image" },
    { "type": "image", "url": "data:image/png;base64,..." }
  ]
}
```

acceptance、user items 与 started facts 作为一个 atomic Thread batch 提交。最终 Agent item
与 completed fact 也作为一个 atomic batch 提交。Provider 失败时持久化稳定
`StableTurnError`；持久化失败时内存投影不得伪造终态。

`input` 是保持顺序的非空 tagged union。文本项必须非空；图片项接受受限 HTTP(S) URL，或
PNG/JPEG/GIF/WEBP base64 data URL。App Server 不接受本地路径；客户端必须在 RPC 边界前读取并
规范化本地文件。Core 会重新校验 MIME、签名、base64 和 16 MiB decoded-size 上限，并把图片
作为 durable `UserImage` 保存。

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

当前有三个 notification method：

- `session/update`，payload 为 `SessionUpdateEnvelope`；
- `thread/update`，payload 为 `ThreadUpdateEnvelope`；
- `skills/changed`，payload 为新的 catalog `generation`。

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

`skills/list` 返回 source-qualified `SkillId`、description、source kind、content digest、
compatibility、effective enablement 和 isolated diagnostics。`reload: "cached"` 可复用当前
projection；`reload: "refresh"` 要求 server 重扫受控 roots。`skill/enablement/set` 必须携带
config `expectedRevision` 与 exact discovered `SkillId`，结果使用标准 config command receipt。
enablement 或 filesystem/config invalidation 导致可见 projection 变化时发布
`skills/changed`；notification 是重新 list 的提示，不包含 catalog body，也不表示 Skill 已注入
正在运行的 Turn。

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
- `TerminalUnavailable`
- `TerminalNotFound`
- `TerminalNotOwner`
- `TerminalBusy`
- `TerminalOperationFailed`
- `GitUnavailable`
- `GitNotRepository`
- `GitOperationFailed`
- `ConfigUnavailable`
- `SkillsUnavailable`
- `SkillNotFound`
- `SkillOperationFailed`

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
