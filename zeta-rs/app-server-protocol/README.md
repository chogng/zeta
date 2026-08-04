# `zeta-app-server-protocol`

> 本 README 解释 App Server external RPC contract、method registry 与 artifact generator。
> 面向客户端的 API 语义见 [`docs/zeta-app-server-api.md`](../../docs/zeta-app-server-api.md)，
> canonical product values 见 [`docs/protocol.md`](../../docs/protocol.md)，workspace 搜索的跨层
> ownership 见 [`docs/search.md`](../../docs/search.md)，Terminal external 语义见
> [`docs/zeta-app-server-api.md`](../../docs/zeta-app-server-api.md#integrated-terminal)，Git/SCM
> ownership 见 [`docs/git.md`](../../docs/git.md)。

`zeta-app-server-protocol` 是 App Server 对外 wire contract 的唯一 Rust source。它定义 typed
params/results/errors、JSON-RPC 2.0 envelopes、method/notification registry，并从同一套定义生成
JSON Schema、TypeScript binding 和协商用 schema hash。

它不拥有 connection、framing、dispatcher、业务执行、Core state、resource bytes 或 persistence。

## Crate 边界

```text
zeta-protocol canonical values
          │
          ▼
zeta-app-server-protocol
├─ protocol DTO
├─ method/notification registry
├─ generic JSON-RPC envelope
└─ schema / TypeScript / hash generator
          │
          ├─ zeta-app-server dispatcher
          └─ Desktop / CLI clients
```

语义与 JSON-RPC wrapper 分开：

- `protocol::*` 定义 method params/result 与 App Server-specific DTO；
- `rpc::*` 只定义 transport-neutral generic RPC envelope；
- `export::*` 把 registry、DTO 和 envelope 聚合成 artifact。

未来换编码时可以复用业务 DTO；不要把 `jsonrpc`、wire request ID 或 connection state放进
`protocol::*` DTO。

## 目录与职责

```text
zeta-rs/app-server-protocol/
├── src/
│   ├── protocol/
│   │   ├── registry.rs       # method、notification、serialization scope
│   │   ├── common.rs         # handshake/shared wire values
│   │   ├── initialize.rs
│   │   ├── slash_commands.rs
│   │   ├── session.rs
│   │   ├── model.rs
│   │   ├── turn.rs
│   │   ├── config.rs
│   │   ├── resources.rs
│   │   ├── search.rs
│   │   ├── terminal.rs
│   │   └── error.rs
│   ├── rpc.rs                # generic JSON-RPC 2.0 envelopes
│   ├── export.rs             # pure deterministic generators
│   ├── schema_fixtures.rs    # contract/golden tests
│   └── bin/
│       ├── export.rs
│       └── write_schema_fixtures.rs
└── schema/
    ├── schema.json
    └── types.ts
```

## 公共契约

### 注册表

| Symbol | 职责 |
| --- | --- |
| `ClientMethod` | typed enum of every client-callable method |
| `client_method` | exact string → `ClientMethod` lookup |
| `CLIENT_METHODS` | method name、params/result type function、serialization scope |
| `ServerNotificationMethod` | typed enum of server notifications |
| `server_notification_method` | exact string → notification enum |
| `SERVER_NOTIFICATIONS` | notification name 与 params type |
| `SerializationScopeDefinition` | dispatcher serialization requirement |

方法注册表当前覆盖初始化、Session 生命周期/聚合订阅、canonical `session/request` mutation、
带 Session scope 的 Thread 读取/订阅、模型目录、配置/供应商/MCP/Skill/Plugin request/Hook declaration 修改、
Turn 与 Resource
metadata/read/release、filesystem metadata/read/write，以及 workspace search start/read/cancel。
Notification 包含 `session/update`、Session-owned child 的 `session/thread/update`、`skills/changed`、`git/statusChanged` 与
`fs/changed`；Terminal 当前使用 profile/list 与 create/write/resize/read/close 的有界 pull
contract，不伪装成主动 notification stream。

`SessionSubscribeResult` 是产品宿主的 aggregate port：`session/subscribe` 返回 Session snapshot、
Session durable gap 和 `SessionThreadProjection` 列表；App Server 同时为这些 child Thread 建立
connection-local update delivery。需要单独读取或追赶一个 child Thread 时使用
`session/thread/read` / `session/thread/subscribe`，两者都必须携带 `sessionId`。
`SessionRequestParams` / `SessionRequest` 是 mutation 的 canonical Session contract：公共请求统一
携带 `CommandId`、Session sequence 和 typed operation，结果通过 `SessionRequestResult` 的 tagged
union 返回。旧的独立 Session/Thread/Turn mutation methods 不在 registry 中。
Git 注册 `git/status`、`git/textDiff`、`git/branch/list` query，以及
branch/switch、stage/unstage/discardWorktree/commit/fetch/pull/push global-exclusive mutation；
status 带 revision 和 repository-relative `workspacePath`，投影变化通过 `git/statusChanged` 发送
完整 snapshot；该相对关系允许本地 host 提供 repository-root 导航，但不会把 host 绝对路径放进
共享协议。
Filesystem 注册 `fs/writeFile` global-exclusive mutation；`fs/changed` 只携带 workspace-relative
invalidation hint 或 rescan request，不成为 durable 文件事件。

### JSON-RPC

`JsonRpcRequest<P>`、`JsonRpcNotification<P>`、`JsonRpcSuccess<R>`、`JsonRpcFailure<E>` 和
`JsonRpcResponse<R,E>` 对 envelope 使用 `deny_unknown_fields`。`JsonRpcId` 保留 number、string
与 null 的 parse shape；App Server dispatcher 再实施更窄的 positive-integer request-ID policy。

`JsonRpcVersion` 只接受 `"2.0"`。

### 导出

| Symbol | 输出 |
| --- | --- |
| `json_schema()` | pretty JSON schema + trailing newline |
| `typescript()` | DTO declarations、method map、notification union、schema hash |
| `schema_hash()` | `sha256:` + generated schema 的 deterministic digest |
| `JSON_SCHEMA_FIXTURE` / `TYPESCRIPT_FIXTURE` | checked-in artifact 相对路径 |

Generator 只返回字符串，不读写 filesystem。只有 `write_schema_fixtures` binary 拥有 fixture 写入。

## 内部接口地图

| Symbol | 可见性 | 当前职责 | 方向约束 |
| --- | --- | --- | --- |
| `client_methods!` | private macro | 一次定义 method enum、lookup、metadata、request/result schema enums | method 不得在 dispatcher 建第二份表 |
| `server_notifications!` | private macro | 一次定义 notification enum、lookup、metadata、schema enum | notification list 唯一来源 |
| `typescript_bindings!` | private macro | 建立显式 DTO declaration list | canonical/re-exported nested types 也必须可生成 |
| `ClientRequestSchema` | crate-private generated enum | 聚合所有 request params 到 root schema | 不是 runtime dispatcher payload |
| `ClientResultSchema` | crate-private generated enum | 聚合所有 method result | 与 method registry lockstep |
| `ServerNotificationSchema` | crate-private generated enum | 聚合 notification params | 与 notification registry lockstep |
| `TypeScriptBinding` | crate-private | 延迟调用每个 `TS::decl` | declaration order 必须 deterministic |
| `type_name<T>` / `declaration<T>` | private | ts-rs adapter functions | 不手写 DTO field shape |
| `ProtocolSchema` | private | JSON Schema root：request/response/notification/error | artifact coverage 的唯一 root |
| `protocol_schema` | private | `schema_for!(ProtocolSchema)` | hash 与 JSON 必须调用同一函数 |
| `Artifact::{parse,contents}` | binary-private | stdout exporter format selection | 不影响 library contract |
| `write_fixture` | binary-private | explicit golden fixture overwrite | tests 不能隐式调用 |

`TYPESCRIPT_BINDINGS` 是一个容易遗漏的同步点。例如新增 canonical enum 被 RPC DTO 引用时，若
ts-rs 无法从 outer declaration 自动内联，就必须显式加入这里。编译通过不代表 TypeScript fixture
完整。

## 注册表→ artifact 调用图

```text
client_methods!
├─ ClientMethod / client_method
├─ CLIENT_METHODS
├─ ClientRequestSchema
└─ ClientResultSchema

server_notifications!
├─ ServerNotificationMethod / lookup
├─ SERVER_NOTIFICATIONS
└─ ServerNotificationSchema

json_schema()
└─ protocol_schema()
   └─ schema_for!(ProtocolSchema)

schema_hash()
└─ serialize protocol_schema()
   └─ SHA-256

typescript()
├─ schema_hash()
├─ TYPESCRIPT_BINDINGS → TypeScriptBinding::declaration
├─ SERVER_NOTIFICATIONS → ServerNotification union/map
└─ CLIENT_METHODS → AppServerMethodMap + constants
```

Runtime dispatch 当前使用 `client_method(method)` 得到 typed method；`serialization` 已在 registry
中声明，但现有同步 `AppServer::handle_json` 尚未读取它来建立 per-scope scheduler。它是待接入
dispatcher 的 executable metadata，不能被误写成已经落实的并发保证。

## DTO 建模约束

- Canonical `Session`、`Thread`、`Turn`、`ThreadItem`、events、updates 与 typed IDs 直接
  re-export/reuse `zeta-protocol`；不要复制相同语义 DTO。
- RPC-only params/result 留在本 crate，例如 `SessionRequestParams`、`ConfigUpdateParams`、
  `ResourceReadResult`。
- Durable mutation params 带 `CommandId` 与 expected sequence/revision；JSON-RPC ID 不替代它。
- `ConfigUpdateParams` 使用 `Patch<T>` 表达 missing/no-op、null/clear、value/set 三态。
- `SessionRequest::ResolveInteraction` 使用 canonical `AgentResponse` 与 exact `RequestId`。
- `SessionRequest::StartTurn.input` 是有序、非空的 tagged union：`text { text }` 或
  `image { url }`；图片在进入 wire contract 前必须已经从本地路径规范化为 HTTP(S)/data URL。
- `InitializeResult.slash_commands` 是 server composition 在 handshake 时冻结的完整动态命令
  snapshot；每项声明 canonical name、description 与 inline argument mode。动态命令提交仍是
  普通 ordered Turn input，不引入第二套 command RPC。
- Resource chunk 的 `data_base64` 是标准 Base64，`decoded_length` 是原始 bytes 数，最大 chunk
  262,144 bytes。
- Terminal raw output 使用 `TerminalOutputChunk.data_base64`；sequence 字段在 Rust 使用 `u64`，
  TypeScript artifact 显式生成为 `number`，server 只产生 safe-lifetime 的单调值。`output_gap`
  是 ring eviction 的显式信号，不能由客户端忽略。
- `TerminalProfile` 只暴露稳定 ID、标题和 default 标记。Terminal create 只接受
  `TerminalProfileSelection`，不包含 program、args、cwd 或 environment；这些 authority 留在
  local App Server composition。Terminal params 与 profile selection 拒绝 unknown field，不能把
  client-authored environment 藏在生成 DTO 之外。
- `AppServerError` 的 `message` 是 stable `AppServerErrorName`，`data` 当前为 unit，不传任意
  internal error text。
- DTO 默认 camelCase；required/optional/nullability 要同时符合 serde、schemars 与 ts-rs。

## 修改一个 RPC 的准确路径

```text
1. 在 protocol/<domain>.rs 定义/修改 Params 与 Result
2. 在 client_methods! 中注册 method + types + serialization scope
3. 若新增 TS dependency，更新 typescript_bindings!
4. 更新 zeta-app-server dispatcher exhaustive match
5. 运行 write_schema_fixtures
6. 审阅 schema/schema.json 与 schema/types.ts
7. 运行 contract tests
8. 更新 docs/zeta-app-server-api.md 与 client
```

新增 notification 使用同一流程，但修改 `server_notifications!` 和 broker/connection publish path。

Fixture 更新必须显式执行：

```text
cargo run -p zeta-app-server-protocol --bin write_schema_fixtures
```

只查看生成结果：

```text
cargo run -p zeta-app-server-protocol --bin export -- json
cargo run -p zeta-app-server-protocol --bin export -- typescript
```

## 模式哈希与兼容性

`initialize` 返回 `SchemaHash`。当前 hash 是 `protocol_schema()` JSON serialization 的 SHA-256；
JSON Schema generator/order 的任何确定性变化都可能改变 hash，不只 breaking API change。

因此 hash 是“artifact exact match”信号，不是 semantic version。改变 DTO 后必须审阅 artifact
diff，明确客户端是否能同步升级；不要在测试中为了保留旧 hash 而隐藏真实 schema change。

## 方向偏差检查

- Dispatcher 中出现平行 method string/scope table：registry 不再唯一；
- DTO 在 App Server implementation 内定义：schema/TS 生成无法覆盖；
- `rpc.rs` 引用具体 Session/Config DTO：generic envelope 边界被破坏；
- `export.rs` 读取/写入文件或环境：generator 不再 pure/deterministic；
- Rust DTO 更新但 fixture 未变：binding list/root schema 可能漏项；
- Canonical value 在本 crate 被复制：`zeta-protocol` source 分叉；
- Connection owner、actor handle 或 resource bytes 进入 DTO：runtime state 泄漏到 wire；
- `AppServerError` 传递内部 error string：安全与兼容性 contract 漂移；
- Client 使用 arbitrary method string 而非 generated method map：compile-time contract 被绕过。

## 测试、限制与演进

```text
cargo test -p zeta-app-server-protocol
bazel test //zeta-rs/app-server-protocol:app-server-protocol-unit-tests
```

## Typst 文档契约

`protocol::document` 拥有 `document/typst/compile` 的请求，以及对渲染进程安全的诊断和结果
数据结构。该方法注册为全局独占，并通过 `ServerCapabilities::typst` 公布。PDF 内容由
`ResourceMetadataResult` 引用，字节不会嵌入编译响应。

修改这些数据结构时，必须重新生成 `schema/schema.json` 和 `schema/types.ts`，同步
`desktop/generated/app-server/types.ts`，并更新
[`docs/typst.md`](../../docs/typst.md) 中的跨进程契约。

测试验证 method/notification 唯一性、JSON-RPC 2.0 envelope、TypeScript model/patch shape、root
schema coverage、config三态、MCP/Skill round trip、schema hash 和 checked-in fixtures exact match。

当前只有 JSON-RPC 2.0 artifact，schema compatibility 依赖 exact hash + synchronized client build；
尚无 semantic protocol version、migration layer 或 multi-version server。未来增加 Protobuf 或
compatibility window 时，应复用 `protocol::*` 语义和 registry，不把第二套 method ownership 带进
transport adapter。
