# `zeta-app-server-protocol`

> 本 README 解释 App Server external RPC contract、method registry 与 artifact generator。
> 面向客户端的 API 语义见 [`docs/zeta-app-server-api.md`](../../docs/zeta-app-server-api.md)，
> canonical product values 见 [`docs/protocol.md`](../../docs/protocol.md)，workspace 搜索的跨层
> ownership 见 [`docs/search.md`](../../docs/search.md)。

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
│   │   ├── thread.rs
│   │   ├── turn.rs
│   │   ├── config.rs
│   │   ├── resources.rs
│   │   ├── search.rs
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

## Public contract

### Registry

| Symbol | 职责 |
| --- | --- |
| `ClientMethod` | typed enum of every client-callable method |
| `client_method` | exact string → `ClientMethod` lookup |
| `CLIENT_METHODS` | method name、params/result type function、serialization scope |
| `ServerNotificationMethod` | typed enum of server notifications |
| `server_notification_method` | exact string → notification enum |
| `SERVER_NOTIFICATIONS` | notification name 与 params type |
| `SerializationScopeDefinition` | dispatcher serialization requirement |

Method registry 当前覆盖 initialize、Session lifecycle/subscription、Thread read/subscription、
Config/provider/MCP/Skill mutation、Turn start/interrupt/interaction resolve 与 Resource
metadata/read/release，以及 workspace search start/read/cancel。Notification 只有
`session/update` 与 `thread/update`。

### JSON-RPC

`JsonRpcRequest<P>`、`JsonRpcNotification<P>`、`JsonRpcSuccess<R>`、`JsonRpcFailure<E>` 和
`JsonRpcResponse<R,E>` 对 envelope 使用 `deny_unknown_fields`。`JsonRpcId` 保留 number、string
与 null 的 parse shape；App Server dispatcher 再实施更窄的 positive-integer request-ID policy。

`JsonRpcVersion` 只接受 `"2.0"`。

### Export

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

## Registry → artifact 调用图

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
- RPC-only params/result 留在本 crate，例如 `TurnStartParams`、`ConfigUpdateParams`、
  `ResourceReadResult`。
- Durable mutation params 带 `CommandId` 与 expected sequence/revision；JSON-RPC ID 不替代它。
- `ConfigUpdateParams` 使用 `Patch<T>` 表达 missing/no-op、null/clear、value/set 三态。
- `TurnInteractionResolveParams` 使用 canonical `AgentResponse` 与 exact `RequestId`。
- `TurnStartParams.input` 是有序、非空的 tagged union：`text { text }` 或
  `image { url }`；图片在进入 wire contract 前必须已经从本地路径规范化为 HTTP(S)/data URL。
- `InitializeResult.slash_commands` 是 server composition 在 handshake 时冻结的完整动态命令
  snapshot；每项声明 canonical name、description 与 inline argument mode。动态命令提交仍是
  普通 ordered Turn input，不引入第二套 command RPC。
- Resource chunk 的 `data_base64` 是标准 Base64，`decoded_length` 是原始 bytes 数，最大 chunk
  262,144 bytes。
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

## Schema hash 与 compatibility

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

## Typst document contract

`protocol::document` owns the request and renderer-safe diagnostic/result DTOs
for `document/typst/compile`. The method is registered as global-exclusive and
advertised through `ServerCapabilities::typst`. PDF content is referenced by
`ResourceMetadataResult`; bytes are not embedded into the compile response.

Any change to these DTOs requires regenerating `schema/schema.json` and
`schema/types.ts`, syncing `desktop/generated/app-server/types.ts`, and
updating the cross-process contract in
[`docs/typst.md`](../../docs/typst.md).

Tests 验证 method/notification 唯一性、JSON-RPC 2.0 envelope、TypeScript model/patch shape、root
schema coverage、config三态、MCP/Skill round trip、schema hash 和 checked-in fixtures exact match。

当前只有 JSON-RPC 2.0 artifact，schema compatibility 依赖 exact hash + synchronized client build；
尚无 semantic protocol version、migration layer 或 multi-version server。未来增加 Protobuf 或
compatibility window 时，应复用 `protocol::*` 语义和 registry，不把第二套 method ownership 带进
transport adapter。
