# zeta-app-server-protocol

## 总体职责

`zeta-app-server-protocol` 是 App Server 对外 RPC 协约的唯一事实来源。它定义统一的业务
DTO、RPC 方法和通知，提供当前 JSON-RPC wire envelope，并从同一份定义确定性地导出 JSON
Schema、TypeScript 类型和协商用的 Schema 哈希。

它负责“双方交换什么数据、如何包装为 RPC 消息、如何导出并验证协约”，但不负责：

- WebSocket、stdio 或 JSON Lines 等连接和帧传输；
- RPC 方法的业务执行、调度、资源所有权和持久化；
- Core-private runtime message、actor state、模型运行时数据和 rollout 耐久化格式。

canonical `Session/Thread/Turn/ThreadItem`、durable event 和 live update 由
`zeta-protocol` 所有，持久化 envelope 分别由 `zeta-session-store` 与
`zeta-thread-store` 所有。当前开发契约直接复用语义完全一致的 canonical public view；
只有 method params/result、RPC error 或真正独立的 wire 语义才定义 DTO。Core-private
aggregate、command receipt 和 App Server connection owner state 永不进入 wire。作为产品状态的
`Turn.pendingInteraction` 则是 canonical view 的一部分，但只包含 wait metadata，不携带
connection ID 或需要定向交付的 request payload。

## 目录与职责

```text
app-server-protocol/
├── schema/
│   ├── schema.json
│   └── types.ts
├── src/
│   ├── bin/
│   │   ├── export.rs
│   │   └── write_schema_fixtures.rs
│   ├── protocol/
│   │   ├── common.rs
│   │   ├── session.rs
│   │   ├── thread.rs
│   │   ├── turn.rs
│   │   ├── registry.rs
│   │   └── ...
│   ├── export.rs
│   ├── rpc.rs
│   ├── schema_fixtures.rs
│   └── lib.rs
└── README.md
```

| 文件或目录 | 职责 |
| --- | --- |
| `protocol/` | 传输和编码无关的统一业务协约：基础值、方法注册表，以及 Session、Thread、Turn、Config 等 RPC DTO。新增 RPC 方法或通知时，必须先在此处定义 DTO，再在 `registry.rs` 注册。 |
| `rpc.rs` | 通用 RPC 消息模型，以及当前 JSON-RPC 2.0 的 request、success response、error response 和 notification 包装规则。它不含 App Server 业务方法。 |
| `export.rs` | 纯函数导出层：将 `protocol/` 的 registry 与 DTO 套入 RPC wire contract，生成 JSON Schema、TypeScript 与 Schema 哈希；不读写文件。 |
| `bin/export.rs` | 命令行导出工具：调用 `export.rs` 并将指定格式写到标准输出，便于其他工具或流水线消费。 |
| `bin/write_schema_fixtures.rs` | 显式维护工具：调用 `export.rs`，覆盖写入 `schema/` 下受版本控制的 JSON 与 TypeScript golden fixtures。正常测试绝不运行它。 |
| `schema/` | 统一协议的权威、受版本控制的导出产物。JSON 用于 wire payload 校验，TypeScript 供 Desktop 同步后编译使用。 |
| `schema_fixtures.rs` | 防回归测试：在内存中重新生成产物并与 `schema/` 逐字节比较；还校验 RPC envelope、注册表和关键 DTO 约束。产物过期或生成结果变化会直接失败。 |
| `lib.rs` | crate 根模块：只暴露 RPC、业务协议和导出 API。 |

需要将单一格式交给外部工具时，`export` 只写标准输出：

```sh
cargo run -p zeta-app-server-protocol --bin export -- json
cargo run -p zeta-app-server-protocol --bin export -- typescript
```

## 生成与校验流程

修改 DTO、registry 或 RPC envelope 后：

1. 运行 `cargo run -p zeta-app-server-protocol --bin write_schema_fixtures` 更新 `schema/`；
2. 审阅 JSON/TypeScript diff，确认它是有意的 wire contract 变更；
3. 运行 crate 测试。`schema_fixtures.rs` 会拒绝未更新或非确定性的产物；
4. Desktop 的构建准备步骤从 `schema/types.ts` 同步其本地编译输入，不拥有第二份权威定义。

`initialize` 返回由完整 JSON Schema 计算出的 `sha256:` 哈希。客户端携带的 Schema 哈希必须
与服务端一致，避免字段定义不一致的连接继续运行。

## 关键建模约定

- Canonical 产品语义、ID 和 sequence/cursor 定义统一见
  [`docs/protocol.md`](../../docs/protocol.md)；本 crate 只规定它们如何进入 external wire。
- 当前编码为 JSON-RPC 2.0，可通过 WebSocket 或 JSON Lines 承载；连接/帧层不属于本 crate。
- `rpc.rs` 的业务无关模型是未来接入 Protobuf 等编码的边界；不要让 JSON-RPC 字段泄漏到 `protocol/` DTO。
- 所有业务 DTO 同时派生 Serde、JSON Schema 和 TypeScript 定义，使 Rust、JSON Schema 与客户端类型来自同一个源头。
- `Thread.turns` 是分支的交互层级；每个 `Turn.items` 始终是该 Turn 的有序转录，空 Turn 使用空数组而非省略字段。
- durable side effect 使用 `commandId` 与目标 aggregate 的 `expectedSequence`；typed command receipt 属于 store/Core，不进入 wire snapshot。
- 只有 `session/update` 与 `thread/update` 两种业务 notification；durable sequence 与瞬态 stream cursor 保持独立。
- `ConfigUpdateParams` 用 `Patch<T>` 表示三态：字段缺失是不修改，`null` 是清除，具体值是设置。
- `ResourceReadResult.dataBase64` 使用标准 RFC 4648 Base64；`decodedLength` 是原始 bytes 数，客户端以它推进 offset 和校验解码结果，不能使用编码字符串长度。
- TypeScript fixture 同时导出 `APP_SERVER_METHODS` 与 `APP_SERVER_NOTIFICATIONS` typed definitions；Desktop client 以它们约束 params/result，不接受任意业务 method string。
