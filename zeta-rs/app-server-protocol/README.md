# `zeta-app-server-protocol`

> 本 README 解释 App Server external RPC contract、method registry 与 artifact generator。
> 面向客户端的 API 语义见 [`docs/zeta-app-server-api.md`](../../docs/zeta-app-server-api.md)，
> canonical product values 见 [`docs/protocol.md`](../../docs/protocol.md)，workspace 搜索的跨层
> ownership 见 [`docs/search.md`](../../docs/search.md)，Terminal external 语义见
> [`docs/zeta-app-server-api.md`](../../docs/zeta-app-server-api.md#integrated-terminal)，Git/SCM
> ownership 见 [`docs/git.md`](../../docs/git.md)，Desktop 浏览器宿主的跨进程语义见
> [`docs/zeta-desktop-architecture.md`](../../docs/zeta-desktop-architecture.md#7-浏览器能力)。

`zeta-app-server-protocol` 是 App Server 对外 wire contract 的唯一 Rust source。它定义类型化
参数、结果、错误、JSON-RPC 2.0 envelope，以及客户端方法、服务端通知和宿主方法注册表；同一套
定义还会生成 canonical `ServerNotification`、payload decoder、JSON Schema、TypeScript binding、
协议主版本、能力契约版本和诊断用 schema hash。

它不拥有 connection、framing、dispatcher、业务执行、Core state、resource bytes 或 persistence。

## Crate 边界

```text
zeta-protocol canonical values
          │
          ▼
zeta-app-server-protocol
├─ protocol DTO
├─ client method / server notification / host method registry
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
│   │   ├── notification.rs   # canonical notification API re-export
│   │   ├── common.rs         # handshake/shared wire values
│   │   ├── browser.rs        # client-hosted semantic browser request/result DTO
│   │   ├── initialize.rs
│   │   ├── slash_commands.rs
│   │   ├── session.rs
│   │   ├── model.rs
│   │   ├── language.rs       # language-server Config 与 Marketplace DTO
│   │   ├── turn.rs
│   │   ├── config.rs
│   │   ├── connectors.rs     # redacted connection DTO + inbound-only secret
│   │   ├── resources.rs
│   │   ├── search.rs
│   │   ├── code_index.rs
│   │   ├── terminal.rs
│   │   └── error.rs
│   ├── rpc.rs                # generic JSON-RPC 2.0 envelopes
│   ├── export.rs             # pure deterministic generators
│   ├── schema_fixtures.rs    # contract/golden tests
│   └── bin/
│       └── generate_protocol.rs
├── scripts/
│   └── write_schema_fixtures.py
└── schema/
    ├── json/
    │   └── schema.json
    └── typescript/
        └── types.ts
```

## 公共契约

### 注册表

| Symbol | 职责 |
| --- | --- |
| `ClientMethod` | typed enum of every client-callable method |
| `client_method` | exact string → `ClientMethod` lookup |
| `client_method_definition` | exact string → method metadata lookup |
| `CLIENT_METHODS` | method name、params/result type function、serialization scope |
| `ServerNotificationMethod` | typed enum of server notifications |
| `server_notification_method` | exact string → notification enum |
| `SERVER_NOTIFICATIONS` | notification name 与 params type |
| `ServerNotification` | 跨 crate 非穷尽的 canonical typed notification；consumer 只投影自己拥有的 capability |
| `decode_server_notification` | registry-generated method/payload decoder；未知 method 无损保留为 `Unknown` |
| `HostMethod` | App Server 可以向已声明能力的 client 发出的 typed request enum |
| `HOST_METHODS` | host request name 与 params/result type 的唯一 registry |
| `SerializationScopeDefinition` | static dispatcher serialization requirement and declared key field |
| `ClientRequestSerializationScope` | 从 request params 解析出的 global / Session / connection-resource runtime key |

`ClientCapabilities.browser` 是 connection-local browser host 声明。版本 1 用 `observe` 和 `input`
显式表示 client 能处理的语义操作；它不授予 Rust 任意 CDP、Node 或浏览器对象访问权。
`browser/create`、`browser/observe`、`browser/perform` 和 `browser/close` 是当前全部 host method。
每个 action 都携带精确 `targetId`；点击和输入只使用观察结果中的 backend DOM node ID。

方法注册表当前覆盖初始化、Session 生命周期/聚合订阅、canonical `session/request` mutation、
带 Session scope 的 Thread 读取/订阅、模型目录、Connector list/API-token connect/disconnect、
配置/供应商/MCP/Skill/Plugin request/Hook declaration 修改、signed language Marketplace list/install、
digest-pinned `skill/resource/open`、
Turn 与 Resource metadata/read/release、filesystem metadata/read/write、workspace search start/read/cancel、
workspace code-index status/search/rebuild，以及 cloud code-index status/preview/authorize/sync/revoke。
Workspace registry 还包含 Session-scoped `workspace/additionalDirectories/list|add|remove|permissions/set`。目录 DTO 携带完整权限集合，list 与 mutation result 携带 Workspace access revision；权限替换必须提交 `expectedRevision`。四个 method 都携带 `sessionId`，mutation method 使用 Session-exclusive serialization，主 Workspace identity 不进入 mutation result。App Server 要求 connection 声明 `workspaceTrustHost`；DTO 本身不是授权。
Notification 包含 `session/update`、Session-owned child 的 `session/thread/update`、owner-directed
`agent/request`、`connector/changed`、`skills/changed`、`git/statusChanged` 与 `fs/changed`；Terminal 当前使用
profile/list 与 create/write/resize/read/close 的有界 pull
contract，不伪装成主动 notification stream。

`SessionSubscribeResult` 是产品宿主的 aggregate port：`session/subscribe` 返回 Session snapshot、
Session durable gap 和 `SessionThreadProjection` 列表；App Server 同时为这些 child Thread 建立
connection-local update delivery。需要单独读取或追赶一个 child Thread 时使用
`session/thread/read` / `session/thread/subscribe`，两者都必须携带 `sessionId`。
两种 Thread snapshot 请求都可携带 bounded history：`ThreadSnapshotHistory::Latest { turnLimit }`
（范围为 `1..=MAX_THREAD_SNAPSHOT_TURNS`）取得连续的最新 Turn 窗口；
`ThreadSnapshotHistory::Before { turnId, turnLimit }` 取得指定 durable Turn 之前的一页历史。两者的
result 都通过 `ThreadHistoryBoundary` 明确报告是否仍有更早 Turn 以及当前页最老 Turn identity，
因此客户端可以使用 durable Turn identity 继续向前翻页，而不需要自行保存或重建 Thread history。
`Before` 页返回的 `Thread.sequence` 仍是读取时的聚合序号，不表示该旧页覆盖了这一序号对应的最新
Turns；客户端不得用旧页确认 durable update cursor，也不得用旧页替换已加载的最新窗口。
省略 history 保持完整 snapshot 语义，供 rewind、MCP Agent 等需要完整历史的调用方使用。subscribe
的 durable gap 从返回 snapshot 的 sequence 之后开始，不重复传输已经包含在 snapshot 中的事件；
省略 history 的 reconnect 调用仍从客户端提供的 `afterSequence` 返回完整 gap。
`SessionRequestParams` / `SessionRequest` 是 mutation 的 canonical Session contract：公共请求统一
携带 `CommandId`、Session sequence 和 typed operation，结果通过 `SessionRequestResult` 的 tagged
union 返回。旧的独立 Session/Thread/Turn mutation methods 不在 registry 中。
`SessionRequest::SetNextApprovalMode` 修改 Session 保存的下一次批准模式；`StartTurn` 和 `StartShellTurn` 不接收批准模式。App Server 接受新 Turn 时读取该 Session 字段，并把结果冻结到 canonical `Turn.approvalMode`，因此同一 Session 的多个 Thread 各自保留准确的当前模式。

`ClientCapabilities.agentInteractions` 用 version + explicit kinds 声明 connection 能实际处理的
Agent interaction；当前 App Server version 为 1。普通 Thread snapshot 只公开
`PendingInteraction` metadata，full `TurnInteraction` 仅通过 `agent/request` 发送给一个同时满足
capability 和 Session-owned Thread subscription 的 connection。响应继续使用 canonical
`SessionRequest::ResolveInteraction` + exact `RequestId`；非 owner/过期响应分别返回稳定
`AgentInteractionNotOwner` / `AgentInteractionExpired`。owner、重选和 timer 都是 App Server runtime
状态，不进入 DTO；deadline/cancel reason 则是 durable canonical fact。
Git 注册 `git/status`、`git/textDiff`、`git/branch/list` 查询，以及分支切换、暂存、取消暂存、
丢弃工作树修改、提交、抓取、拉取和推送等全局互斥变更方法；
status 带 revision 和 repository-relative `workspacePath`，投影变化通过 `git/statusChanged` 发送
完整 snapshot；该相对关系允许本地 host 提供 repository-root 导航，但不会把 host 绝对路径放进
共享协议。
Filesystem 注册 `fs/writeFile` global-exclusive mutation；`fs/changed` 只携带 workspace-relative
invalidation hint 或 rescan request，不成为 durable 文件事件。

`ConnectorSecretDto` 只实现 inbound `Deserialize`，`Debug` 永远脱敏并在 Drop 时 zeroize；它不能增加
`Clone`、`Serialize` 或普通字符串 getter。新增 Connector response 字段必须继续证明 schema/result 中
没有 credential reference 或 value。修改 registry 后必须同步生成两份 schema fixture。

### JSON-RPC

`JsonRpcRequest<P>`、`JsonRpcNotification<P>`、`JsonRpcSuccess<R>`、`JsonRpcFailure<E>` 和
`JsonRpcResponse<R,E>` 对 envelope 使用 `deny_unknown_fields`。`JsonRpcId` 保留 number、string
与 null 的 parse shape；App Server 对 Client → Server request 实施更窄的 positive-integer ID
policy，Server → Client browser request 使用与其隔离的非空字符串 ID。

`JsonRpcVersion` 只接受 `"2.0"`。

### 导出

| Symbol | 输出 |
| --- | --- |
| `json_schema()` | pretty JSON schema + trailing newline |
| `typescript()` | DTO declarations、client/host method map、notification union、协议版本与 schema hash |
| `schema_hash()` | `sha256:` + generated schema 的 deterministic digest |
| `JSON_SCHEMA_FIXTURE` / `TYPESCRIPT_FIXTURE` | checked-in artifact 相对路径 |

Library generator 只返回字符串，不读写 filesystem。`generate_protocol` binary 负责把指定 artifact
直接写入调用方给出的目录；`scripts/write_schema_fixtures.py` 只声明 checked-in fixture 的目标目录。

## 内部接口地图

| Symbol | 可见性 | 当前职责 | 方向约束 |
| --- | --- | --- | --- |
| `client_methods!` | private macro | 一次定义 method enum、lookup、metadata、request/result schema enums | method 不得在 dispatcher 建第二份表 |
| `ClientMethodDefinition::serialization_scope` | public metadata resolver | 按 registry 声明从 params 生成 runtime scope key | 不加入 connection identity、不执行或排队 request |
| `host_methods!` | private macro | 一次定义 Server → Client method enum、lookup、metadata 与 request/result schema enums | Desktop handler 不手写平行 method name/type map |
| `server_notifications!` | private macro | 一次定义 method enum、lookup、canonical runtime enum、payload decoder、metadata 与 schema enum | notification list 与 decode mapping 的唯一来源 |
| `typescript_bindings!` | private macro | 建立显式 DTO declaration list | canonical/re-exported nested types 也必须可生成 |
| `ClientRequestSchema` | crate-private generated enum | 聚合所有 request params 到 root schema | 不是 runtime dispatcher payload |
| `ClientResultSchema` | crate-private generated enum | 聚合所有 method result | 与 method registry lockstep |
| `HostRequestSchema` / `HostResultSchema` | crate-private generated enum | 聚合全部 reverse request params/result | 与 `HOST_METHODS` lockstep |
| `ServerNotificationSchema` | crate-private generated enum | 聚合 notification params | 与 notification registry lockstep |
| `TypeScriptBinding` | crate-private | 延迟调用每个 `TS::decl` | declaration order 必须 deterministic |
| `type_name<T>` / `declaration<T>` | private | ts-rs adapter functions | 不手写 DTO field shape |
| `ProtocolSchema` | private | JSON Schema root：client/host request/response、notification/error | artifact coverage 的唯一 root |
| `protocol_schema` | private | `schema_for!(ProtocolSchema)` | hash 与 JSON 必须调用同一函数 |
| `Command::parse` | binary-private | 校验 artifact 和 `--out` 目录 | 不影响 library contract |
| `write_artifact` | binary-private | 将单个 artifact 写入调用方目录 | 不推断 Desktop 路径 |

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
├─ ServerNotification / decode_server_notification
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
├─ HOST_METHODS → AppServerHostMethodMap + constants
└─ CLIENT_METHODS → AppServerMethodMap + constants
```

Runtime dispatch 使用 `client_method(method)` 得到 typed method，再通过
`client_method_definition(method).serialization_scope(params)` 解析 executable scope。Session scope
要求 canonical `sessionId`；resource scope 在 registry 条目中显式声明 `resourceId`、`uploadId`、
`skillId` 或 `extensionId`，不得在 App Server 用字段猜测补第二张表。App Server 消费该结果实施
FIFO/shared-read 调度；connection-resource key 由 runtime 再加入 connection identity，防止跨 owner
资源互相串行或被错误共享。

## DTO 建模约束

- Canonical `Session`、`Thread`、`Turn`、`ThreadItem`、events、updates 与 typed IDs 直接
  re-export/reuse `zeta-protocol`；不要复制相同语义 DTO。
- RPC-only params/result 留在本 crate，例如 `SessionRequestParams`、`ConfigUpdateParams`、
  `ResourceReadResult`。
- `SyntaxAnalyzeResult` 只包含通用 snapshot facts；structural selections 使用独立的
  `SyntaxSelectionRangesParams/Result`，避免普通分析序列化全树节点。
- `WorkspaceDocumentOverlay*` DTO 只传 Editor-authoritative full snapshot 或相对 path，响应只返回
  content-free generation/count；Symbol/CodeIndex store identity 不进入 wire。
- `SymbolIndexSearchHitDto` 投影 revision-bound UTF-16 declaration/selection range，不承诺 stable semantic
  symbol ID；retrieval origin 显式区分 local symbol/lexical/semantic/cloud。
- Durable mutation params 带 `CommandId` 与 expected sequence/revision；JSON-RPC ID 不替代它。
- `ConfigUpdateParams` 使用 `Patch<T>` 表达 missing/no-op、null/clear、value/set 三态。
- `SessionRequest::ResolveInteraction` 使用 canonical `AgentResponse` 与 exact `RequestId`。
- `AgentRequestEnvelope` 携带 Session/Thread/Turn aggregate context 和 full durable request，但不携带
  connection owner；`ClientCapabilities.agentInteractions.kinds` 必须与可产生的 response kind 一致。
- `thread/goal/get|set|clear` 管理 Thread 唯一的持久化 Goal；Goal 的状态和跨 Turn token 用量由 Core 从 Thread event log 恢复，App Server 不维护旁路预算状态。
- `SessionRequest::StartTurn.input` 是有序、非空的 tagged union：`text { text }` 或
  `image { url }` 或 `skill { skill: SkillRef }`；图片在进入 wire contract 前必须已经从本地路径
  规范化为 HTTP(S)/data URL，Skill 只能携带 source-qualified ID 与 version selector，不能携带
  raw filesystem path。
- canonical `Session.nextApprovalMode` 表示下一次 Turn 使用的模式，canonical `Turn.approvalMode` 表示该 Turn 接受时冻结的模式；客户端内部名称和显示文案可以不同，但不得重新从本地设置推导这两个值。
- `SessionRequest::CompactContext` 创建独立的压缩 Turn，并携带可选 retention prompt；它不是
  `StartTurn.input`，也不能与 Thread 上的非终态 Turn 并发。prompt 在 Core command receipt 中冻结，
  command replay 不得重新触发压缩后端。
- `ProviderConfigDto.model_context` 是按模型 ID 索引的 context-window metadata，供 Core 决定
  是否启用 deterministic budget/compaction；它不改变 endpoint normalization。
- `InitializeResult.slash_commands` 是 server composition 在 handshake 时冻结的完整动态命令
  snapshot；每项声明 canonical name、description 与 inline argument mode。client 必须按命令契约
  分发：Skill 和 server prompt command 进入普通 ordered Turn input，内置 `/compact` 进入
  `SessionRequest::CompactContext`，不能只按 origin 猜测统一分发。
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
5. 运行 scripts/write_schema_fixtures.py
6. 审阅 schema/json/schema.json 与 schema/typescript/types.ts
7. 运行 contract tests
8. 更新 docs/zeta-app-server-api.md 与 client
```

新增 notification 使用同一流程，但修改 `server_notifications!` 和 broker/connection publish path。
由于该宏同时生成 runtime enum 与 decoder，不得再在 client crate 增加平行 decode match；只有实际
拥有该 capability 的产品 projection 才需要增加消费逻辑。

新增 Server → Client request 时改用 `host_methods!`，并同步实现 App Server broker 和 capable client
handler。host method 不填写 `SerializationScopeDefinition`，因为其顺序、owner、deadline 和取消由
调用该 capability 的 broker 管理，不能借用 Client → Server dispatcher scope。

Fixture 更新必须显式执行：

```bash
python -B zeta-rs/app-server-protocol/scripts/write_schema_fixtures.py
```

生成到指定调用方目录：

```bash
cargo run -p zeta-app-server-protocol --bin generate_protocol -- \
  typescript --out zeta-ts/generated/app-server
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
- Client crate 出现按 method 维护的第二份 notification decoder：registry 不再是唯一事实源；
- 产品穷尽枚举未拥有的 notification：wire capability 变化重新扩散到无关 consumer；
- Canonical value 在本 crate 被复制：`zeta-protocol` source 分叉；
- Connection owner、actor handle 或 resource bytes 进入 DTO：runtime state 泄漏到 wire；
- `AppServerError` 传递内部 error string：安全与兼容性 contract 漂移；
- Client 使用 arbitrary method string 而非 generated method map：compile-time contract 被绕过。
- Desktop 手写 host method 字符串或结果 shape：reverse RPC artifact 与 Rust registry 分叉。

## 测试、限制与演进

```text
cargo test -p zeta-app-server-protocol
bazel test //zeta-rs/app-server-protocol:app-server-protocol-unit-tests
```

## Typst 文档契约

`protocol::document` 拥有 `document/typst/compile` 的请求，以及对渲染进程安全的诊断和结果
数据结构。该方法注册为全局独占，并通过 `ServerCapabilities::typst` 公布。PDF 内容由
`ResourceMetadataResult` 引用，字节不会嵌入编译响应。

修改这些数据结构时，必须重新生成 `schema/json/schema.json`、`schema/typescript/types.ts` 和
`zeta-ts/generated/app-server/types.ts`，并更新
[`docs/typst.md`](../../docs/typst.md) 中的跨进程契约。

测试验证 method/notification 唯一性、JSON-RPC 2.0 envelope、TypeScript model/patch shape、root
schema coverage、host method 名称与 request/result coverage、agent interaction capability/request/error、
config 三态、MCP/Skill round trip、schema
hash 和 checked-in fixtures exact match。

运行时兼容性由 `ProtocolVersion.major` 和 client 所需的版本化 capability contract 决定；主版本
不同、能力缺失或能力版本不在 client 支持窗口内时 fail closed。revision 和 schema hash 用于诊断，
exact schema hash 不再是启动门禁，因此同一主版本内增加可选字段或能力不会无条件阻断启动。

checked-in JSON Schema 与 TypeScript fixture 仍必须在 CI、build 和完整 Desktop dev 启动前 exact
匹配生成器。这个构建一致性门禁与运行时兼容性协商是两条独立边界；未来增加 Protobuf 或 migration
layer 时，应复用 `protocol::*` 语义和 registry，不把第二套 method ownership 带进 transport adapter。
