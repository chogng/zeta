# Zeta App Server API v1

```yaml
title: Zeta App Server API
status: accepted
protocolVersion: 1
owner: zeta-rs
requestedBy:
  - desktop
  - cli
consumers:
  - desktop
  - cli
lastUpdated: 2026-07-24
```

本文是 Desktop、CLI 和 zeta-rs 共同使用的 v1 权威契约。生成的 TypeScript 与 JSON
Schema 必须与本文一致；发生冲突时停止接入并修正契约或实现，不能由客户端猜测。

## 1. v1 范围

v1 当前冻结：

- connection initialize 和版本协商；
- Thread 创建、读取、恢复、列表和取消订阅；
- Turn 启动和中断；
- completed Agent message；
- 普通配置读取和更新；
- connection-owned Resource 读取与释放；
- stable JSON-RPC error envelope；
- in-process 与 JSONL/stdio 的一致语义。

以下能力尚未进入 accepted v1：

- Agent message delta；
- Item 和 Tool Call 完整事件；
- approval 双向请求；
- `turn/steer`；
- Browser Capability requests；
- 多 connection 广播；
- WebSocket、Unix socket 和 daemon lifecycle；
- 后台异步 Turn、deadline 和远程取消；
- Resource 创建业务方法。

客户端可以为这些能力预留 UI，不得发送未定义 method 或把预留字段写入已冻结 DTO。

## 2. 权威状态与职责

| State | Authority | Client responsibility |
|---|---|---|
| Thread/Turn 状态 | App Server | 维护只读投影，发现冲突时重新读取 |
| durable `sequence` | App Server | 只接受单调不减的 snapshot/notification |
| request ID | 发起 connection | 只做当前 connection response pairing |
| `idempotencyKey` | App Server ledger | 每次业务操作生成稳定且非空的 key |
| Config | App Server Config Store | 显示返回值，不维护第二份权威配置 |
| Resource bytes | App Server | 校验 size、digest、offset 和 `eof` |
| Desktop UI 状态 | Desktop | 不写回 Thread/Turn 权威字段 |
| CLI presentation 状态 | CLI | 不从日志文本推断业务状态 |

## 3. Transport 与 envelope

### 3.1 JSONL/stdio

- UTF-8；
- 每行一个完整 JSON-RPC 2.0 message；
- 单条 message 最大 1,048,576 bytes；
- stdout 只能包含协议 message；
- stderr 只能用于诊断；
- response 与随后因该 request 产生的 notifications 按行依次写出。

### 3.2 In-process

In-process client 必须经过相同的：

```text
serialize request
→ request ID pairing
→ initialize gate
→ App Server dispatcher
→ deserialize response
→ typed notification decode
```

禁止提供仅 in-process 可用的隐藏业务方法。

### 3.3 Request

```json
{
  "jsonrpc": "2.0",
  "id": 42,
  "method": "thread/read",
  "params": {
    "threadId": "thread_123"
  }
}
```

`id` 在一个 connection 内必须唯一，类型为正整数。

### 3.4 Success response

```json
{
  "jsonrpc": "2.0",
  "id": 42,
  "result": {}
}
```

无业务结果的方法返回 `"result": null`。

### 3.5 Error response

```json
{
  "jsonrpc": "2.0",
  "id": 42,
  "error": {
    "code": -32602,
    "message": "InvalidParams",
    "data": null
  }
}
```

### 3.6 Notification

```json
{
  "jsonrpc": "2.0",
  "method": "turn/completed",
  "params": {
    "threadId": "thread_123",
    "turnId": "turn_456",
    "sequence": 5
  }
}
```

Notification 没有 `id`，客户端不得发送 response。

## 4. Connection lifecycle

```text
Connected
  → initialize
  → Ready
  → requests / notifications
  → Disconnected
```

规则：

1. `initialize` 必须是首个 request；
2. Ready 前的其他 request 返回 `NotInitialized`；
3. 同一 connection 重复 initialize 返回 `AlreadyInitialized`；
4. 版本区间必须与 server 当前版本相交；
5. 客户端必须保存返回的 `protocolVersion` 和 `schemaHash`；
6. connection 断开后，所有 request ID、subscription 和 Resource ownership 失效；
7. v1 不支持断线自动恢复；客户端重连后重新 initialize，再 `thread/resume`。

## 5. Common types

### 5.1 IDs

| Type | JSON | Rules |
|---|---|---|
| `ThreadId` | string | App Server 生成；客户端视为 opaque |
| `TurnId` | string | App Server 生成；客户端视为 opaque |
| `ResourceId` | string | App Server 生成；绑定 owner connection |
| `idempotencyKey` | string | 客户端生成；同一业务操作重试时保持不变 |

### 5.2 Turn status

```text
created
running
waitingForApproval
waitingForUserInput
waitingForCapability
cancelling
completed
failed
interrupted
```

终态为 `completed`、`failed`、`interrupted`。

### 5.3 Thread

| Field | Type | Required | Nullable | Meaning |
|---|---|---:|---:|---|
| `threadId` | `ThreadId` | yes | no | Thread identity |
| `title` | string | yes | no | Display title |
| `sequence` | integer | yes | no | Durable Thread sequence |
| `turns` | `Turn[]` | yes | no | Ordered Turn projection |

### 5.4 Turn

| Field | Type | Required | Nullable | Meaning |
|---|---|---:|---:|---|
| `turnId` | `TurnId` | yes | no | Turn identity |
| `status` | `TurnStatus` | yes | no | Current authoritative state |

## 6. Method inventory

| Method | Consumers | Side effect | Idempotency | Subscription |
|---|---|---:|---|---|
| `initialize` | Desktop, CLI | connection | no | no |
| `thread/start` | Desktop, CLI | durable | required | subscribes |
| `thread/read` | Desktop, CLI | no | n/a | no change |
| `thread/resume` | Desktop, CLI | connection | n/a | subscribes |
| `thread/list` | Desktop, CLI | no | n/a | no change |
| `thread/unsubscribe` | Desktop, CLI | connection | n/a | removes |
| `turn/start` | Desktop, CLI | durable | required | requires subscription for notifications |
| `turn/interrupt` | Desktop, CLI | durable | no | requires subscription for notification |
| `config/read` | Desktop, CLI | no | n/a | no |
| `config/update` | Desktop, CLI | durable | required | no |
| `resource/metadata` | Desktop, CLI | no | n/a | owner only |
| `resource/read` | Desktop, CLI | no | n/a | owner only |
| `resource/release` | Desktop, CLI | resource | no | owner only |

## 7. `initialize`

### Params

| Field | Type | Required | Nullable | Constraints |
|---|---|---:|---:|---|
| `clientInfo.name` | string | yes | no | non-empty |
| `clientInfo.version` | string | yes | no | non-empty |
| `protocolVersions.min` | integer | yes | no | `>= 1` |
| `protocolVersions.max` | integer | yes | no | `>= min` |
| `capabilities.notifications` | boolean | no | no | default `false` |
| `capabilities.browser` | object | no | yes | reserved; no accepted Browser methods |

### Request fixture

```json
{
  "jsonrpc": "2.0",
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
      "notifications": true
    }
  }
}
```

### Success fixture

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "serverInfo": {
      "name": "zeta-app-server",
      "version": "0.1.0"
    },
    "protocolVersion": 1,
    "schemaHash": "fnv1a64:<generated-hash>",
    "capabilities": {
      "threads": true,
      "turns": true,
      "resources": true
    }
  }
}
```

Errors: `InvalidParams`, `AlreadyInitialized`, `ProtocolVersionUnsupported`.

## 8. Thread methods

### 8.1 `thread/start`

Creates a durable Thread, subscribes the current connection, then emits `thread/started`.

Params:

| Field | Type | Required | Nullable | Constraints |
|---|---|---:|---:|---|
| `idempotencyKey` | string | yes | no | non-empty |
| `title` | string | yes | no | may be empty |

Request:

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "thread/start",
  "params": {
    "idempotencyKey": "desktop-thread-01",
    "title": "New conversation"
  }
}
```

Response:

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "threadId": "thread_123",
    "sequence": 1
  }
}
```

Errors: `InvalidParams`, `IdempotencyConflict`, `CoreOperationFailed`.

相同 key 和相同 params 返回原 result，并把当前 connection 订阅到该 Thread。不同 params
返回 `IdempotencyConflict`。

### 8.2 `thread/read`

读取 snapshot，不改变 subscription。

```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "thread/read",
  "params": {
    "threadId": "thread_123"
  }
}
```

```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "result": {
    "thread": {
      "threadId": "thread_123",
      "title": "New conversation",
      "sequence": 1,
      "turns": []
    }
  }
}
```

Errors: `InvalidParams`, `CoreOperationFailed`.

### 8.3 `thread/resume`

原子读取当前 snapshot 并订阅当前 connection。Result 与 `thread/read` 相同。

```json
{
  "jsonrpc": "2.0",
  "id": 4,
  "method": "thread/resume",
  "params": {
    "threadId": "thread_123"
  }
}
```

Errors: `InvalidParams`, `CoreOperationFailed`.

### 8.4 `thread/list`

```json
{
  "jsonrpc": "2.0",
  "id": 5,
  "method": "thread/list",
  "params": {}
}
```

```json
{
  "jsonrpc": "2.0",
  "id": 5,
  "result": {
    "threads": [
      {
        "threadId": "thread_123",
        "title": "New conversation",
        "sequence": 1,
        "turns": []
      }
    ]
  }
}
```

Errors: `CoreOperationFailed`.

v1 不承诺排序、分页或完整历史搜索。

### 8.5 `thread/unsubscribe`

```json
{
  "jsonrpc": "2.0",
  "id": 6,
  "method": "thread/unsubscribe",
  "params": {
    "threadId": "thread_123"
  }
}
```

```json
{
  "jsonrpc": "2.0",
  "id": 6,
  "result": null
}
```

不存在的本地 subscription 也返回成功。该方法不删除 Thread。

## 9. Turn methods

### 9.1 `turn/start`

Params:

| Field | Type | Required | Nullable | Constraints |
|---|---|---:|---:|---|
| `idempotencyKey` | string | yes | no | non-empty |
| `threadId` | `ThreadId` | yes | no | existing Thread |
| `input` | `InputItem[]` | yes | no | at least one item |
| `input[].type` | `"text"` | yes | no | v1 only |
| `input[].text` | string | yes | no | UTF-8 |

```json
{
  "jsonrpc": "2.0",
  "id": 7,
  "method": "turn/start",
  "params": {
    "idempotencyKey": "desktop-turn-01",
    "threadId": "thread_123",
    "input": [
      {
        "type": "text",
        "text": "Explain this repository"
      }
    ]
  }
}
```

```json
{
  "jsonrpc": "2.0",
  "id": 7,
  "result": {
    "turnId": "turn_456",
    "sequence": 3
  }
}
```

Current accepted behavior:

1. App Server durably creates and starts the Turn；
2. 当前 model adapter 完成一次非流式响应；
3. App Server durably completes the Turn；
4. transport 先发送 response；
5. 首次执行按顺序发送 `turn/started`、`item/agentMessage/completed`、`turn/completed`。

重放相同 idempotency request 只返回原 result，不重放 Turn notifications。

Errors: `InvalidParams`, `IdempotencyConflict`, `CoreOperationFailed`.

### 9.2 `turn/interrupt`

```json
{
  "jsonrpc": "2.0",
  "id": 8,
  "method": "turn/interrupt",
  "params": {
    "threadId": "thread_123",
    "turnId": "turn_456"
  }
}
```

```json
{
  "jsonrpc": "2.0",
  "id": 8,
  "result": null
}
```

成功后发送 `turn/interrupted`。如果 Turn 已经进入终态，返回
`CoreOperationFailed`。当前非流式 Turn 通常在 `turn/start` 返回前已经完成，因此 v1
前端不能假设一定存在可中断窗口。

## 10. Config methods

### 10.1 Config shape

| Field | Type | Required | Nullable | Meaning |
|---|---|---:|---:|---|
| `preferredModel` | string | yes | yes | `null` 表示未设置 |
| `theme` | `"light" \| "dark" \| "system"` | yes | yes | `null` 表示未设置 |

### 10.2 `config/read`

```json
{
  "jsonrpc": "2.0",
  "id": 9,
  "method": "config/read",
  "params": {}
}
```

```json
{
  "jsonrpc": "2.0",
  "id": 9,
  "result": {
    "preferredModel": null,
    "theme": "system"
  }
}
```

Errors: `ConfigUnavailable`.

### 10.3 `config/update`

字段缺失表示“不修改”，显式 `null` 表示“清除”。

```json
{
  "jsonrpc": "2.0",
  "id": 10,
  "method": "config/update",
  "params": {
    "idempotencyKey": "desktop-config-01",
    "theme": "dark"
  }
}
```

Result 是更新后的完整 Config shape。

Errors: `InvalidParams`, `IdempotencyConflict`, `ConfigUnavailable`.

## 11. Resource methods

Resource 由未来业务方法返回 `resourceId`；v1 不提供客户端创建 Resource 的方法。

当前限制：

- owner 是创建 Resource 的 connection；
- TTL 为 300 seconds；
- 单 Resource 最大 16 MiB；
- 单 chunk 最大 262,144 bytes；
- bytes 使用 JSON integer array，不使用 Base64；
- SHA-256 格式为 `sha256:<lowercase-hex>`。

### 11.1 `resource/metadata`

```json
{
  "jsonrpc": "2.0",
  "id": 11,
  "method": "resource/metadata",
  "params": {
    "resourceId": "resource_0000000000000001"
  }
}
```

```json
{
  "jsonrpc": "2.0",
  "id": 11,
  "result": {
    "resourceId": "resource_0000000000000001",
    "mimeType": "application/pdf",
    "size": 1024,
    "sha256": "sha256:<lowercase-hex>"
  }
}
```

### 11.2 `resource/read`

```json
{
  "jsonrpc": "2.0",
  "id": 12,
  "method": "resource/read",
  "params": {
    "resourceId": "resource_0000000000000001",
    "offset": 0,
    "maxBytes": 262144
  }
}
```

```json
{
  "jsonrpc": "2.0",
  "id": 12,
  "result": {
    "resourceId": "resource_0000000000000001",
    "offset": 0,
    "data": [37, 80, 68, 70],
    "eof": false
  }
}
```

客户端将下一次 offset 设置为 `offset + data.length`，直到 `eof: true`，最后校验 size 和
SHA-256。

### 11.3 `resource/release`

```json
{
  "jsonrpc": "2.0",
  "id": 13,
  "method": "resource/release",
  "params": {
    "resourceId": "resource_0000000000000001"
  }
}
```

成功返回 `null`。release 后再次访问返回 `ResourceNotFound`。

Resource methods 可能返回：`ResourceNotFound`、`ResourceNotOwner`、`ResourceTooLarge`、
`InvalidResourceChunkSize`、`InvalidResourceOffset`、`ServerOverloaded`。

## 12. Notifications

### 12.1 `thread/started`

```json
{
  "jsonrpc": "2.0",
  "method": "thread/started",
  "params": {
    "threadId": "thread_123",
    "sequence": 1
  }
}
```

### 12.2 `turn/started`

```json
{
  "jsonrpc": "2.0",
  "method": "turn/started",
  "params": {
    "threadId": "thread_123",
    "turnId": "turn_456",
    "sequence": 3
  }
}
```

### 12.3 `item/agentMessage/completed`

```json
{
  "jsonrpc": "2.0",
  "method": "item/agentMessage/completed",
  "params": {
    "threadId": "thread_123",
    "turnId": "turn_456",
    "text": "Zeta response",
    "sequence": 4
  }
}
```

### 12.4 `turn/completed`

```json
{
  "jsonrpc": "2.0",
  "method": "turn/completed",
  "params": {
    "threadId": "thread_123",
    "turnId": "turn_456",
    "sequence": 4
  }
}
```

### 12.5 `turn/interrupted`

```json
{
  "jsonrpc": "2.0",
  "method": "turn/interrupted",
  "params": {
    "threadId": "thread_123",
    "turnId": "turn_456",
    "sequence": 5
  }
}
```

### 12.6 Ordering

- 同一 request 产生的 response 先于 notifications；
- 同一 connection 内 notifications 保持生成顺序；
- `sequence` 是 Thread durable sequence，不是 notification ordinal；
- 多个 notifications 可以引用同一个 durable sequence；
- 当前 v1 只保证向发起 request 的已订阅 connection 发送因果 notifications；
- v1 不保证跨 connection 广播；
- 客户端发现 notification sequence 小于已知 sequence 时忽略该 notification；
- 客户端发现未知状态或无法合并时调用 `thread/read`；
- v1 尚未提供独立 `streamSeq`。

## 13. Idempotency

适用方法：

- `thread/start`
- `turn/start`
- `config/update`

规则：

1. ledger scope 是 App Server state root；
2. key 与 method 共同构成 identity；
3. params 的完整 canonical JSON 参与比较；
4. 相同 method + key + params 返回原 result；
5. 相同 method + key + 不同 params 返回 `IdempotencyConflict`；
6. ledger 在进程重启后仍然有效；
7. v1 尚未定义自动过期，客户端不得复用旧 key 代表新操作；
8. JSON-RPC `id` 不能代替 `idempotencyKey`。

## 14. Stable errors

| Code | Message | Retryable | Client action |
|---:|---|---:|---|
| `-32700` | `ParseError` | no | 修复 JSON framing |
| `-32600` | `InvalidRequest` | no | 修复 envelope |
| `-32601` | `MethodNotFound` | no | 检查 protocol version |
| `-32602` | `InvalidParams` | no | 修复 Params |
| `-32000` | `InternalError` | maybe | 记录诊断，允许一次重试 |
| `-32000` | `ServerOverloaded` | yes | bounded backoff |
| `-32001` | `NotInitialized` | no | 先 initialize |
| `-32002` | `AlreadyInitialized` | no | 不重复 initialize |
| `-32003` | `ProtocolVersionUnsupported` | no | 升级客户端或服务端 |
| `-32004` | `IdempotencyConflict` | no | 新操作使用新 key |
| `-32010` | `CoreOperationFailed` | context | 重新读取 Thread 或显示失败 |
| `-32020` | `ResourceNotFound` | no | 丢弃 ResourceRef |
| `-32020` | `ResourceNotOwner` | no | 只在 owner connection 读取 |
| `-32020` | `ResourceTooLarge` | no | 降低 Resource 大小 |
| `-32020` | `InvalidResourceChunkSize` | no | 使用 `1..=262144` |
| `-32020` | `InvalidResourceOffset` | no | 重新读取 metadata |
| `-32030` | `ConfigUnavailable` | maybe | 显示配置不可用 |

v1 的 `error.data` 固定为 `null`。客户端必须同时检查 code 和 message，不能只匹配人类文本。

## 15. Error fixtures

### Not initialized

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "error": {
    "code": -32001,
    "message": "NotInitialized",
    "data": null
  }
}
```

### Unsupported version

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32003,
    "message": "ProtocolVersionUnsupported",
    "data": null
  }
}
```

### Idempotency conflict

```json
{
  "jsonrpc": "2.0",
  "id": 7,
  "error": {
    "code": -32004,
    "message": "IdempotencyConflict",
    "data": null
  }
}
```

### Core operation failed

```json
{
  "jsonrpc": "2.0",
  "id": 8,
  "error": {
    "code": -32010,
    "message": "CoreOperationFailed",
    "data": null
  }
}
```

### Resource ownership violation

```json
{
  "jsonrpc": "2.0",
  "id": 12,
  "error": {
    "code": -32020,
    "message": "ResourceNotOwner",
    "data": null
  }
}
```

## 16. Security

- Desktop 只从打包目录启动确定的 `zeta` binary；
- Desktop 使用 `shell: false` 和 environment allowlist；
- Renderer 不接触任意 method 调用器，只接触 typed Preload API；
- Electron Main 校验 sender frame URL 和 params；
- stdout 不得包含日志或 secret；
- Resource 不暴露本地路径；
- Resource 只能由 owner connection 读取；
- Browser、文件、命令和网络能力在进入 v1 前必须单独完成安全契约；
- CLI 不得以“本地调用”为由绕过 App Server、sandbox 或 approval。

## 17. Compatibility

- 新增 optional 字段：允许在 v1 内兼容增加；
- 新增 required 字段：发布新协议版本；
- 改变字段含义、默认值或 owner：发布新协议版本；
- 未知 notification：记录诊断并忽略；
- 未知 request：返回 `MethodNotFound`；
- 未知 enum：当前 v1 客户端停止合并对应实体并 `thread/read`；
- 客户端必须使用 initialize 返回的版本；
- schema hash 不匹配时，Desktop 不进入业务 UI。

## 18. Frontend development baseline

Desktop 可以立即实现：

- App Server process lifecycle；
- initialize/version/schema gate；
- Thread list/start/read/resume；
- Turn start；
- completed Agent message 展示；
- Config read/update；
- typed notification router；
- Resource reader 基础设施。

Desktop 暂时不要实现或模拟：

- streaming cursor；
- Tool Call timeline；
- approval modal 协议；
- Browser action RPC；
- Turn steer；
- multi-connection presence；
- background Turn。

这些能力必须通过本契约的新 accepted revision 或 protocol v2 后再接入。

## 19. Acceptance

v1 变更必须同时满足：

- Rust DTO 编译；
- App Server handler tests；
- in-process client contract tests；
- JSON fixtures 可解析；
- TypeScript 重新生成并 strict compile；
- JSON Schema 可解析；
- schema hash 更新；
- Desktop 不使用 `unknown` 代替 accepted DTO；
- CLI 不直接依赖 Core、Storage、Exec、Sandbox 或 Model Provider。
