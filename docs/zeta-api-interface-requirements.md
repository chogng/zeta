# Zeta API 接口文档规范

> 本文规定 Desktop、CLI 或其他客户端提交 Zeta App Server 产品 API 需求时必须包含的内容。  
> zeta-rs 是已接受契约的 owner，并据此实现 Rust 协议、handler、typed client、生成类型和测试。

当前已经接受的产品契约见
[`zeta-app-server-api-v1.md`](zeta-app-server-api-v1.md)。

## 1. 文档头

每份 API 文档必须先写：

```yaml
title: Browser Capability API
status: draft | review | accepted | deprecated
protocolVersion: 1
capabilityVersion: 1
owner: zeta-rs
rustOwner: zeta-rs
requestedBy: desktop
consumers: [desktop, cli]
lastUpdated: YYYY-MM-DD
```

还需说明：

- 解决的问题；
- 不在范围内的能力；
- 权威状态由哪一端持有；
- 哪些客户端需要该能力；
- 兼容性与废弃策略。

## 2. 方法清单

先提供一张完整清单：

| Method | Direction | Consumers | Side effect | Idempotent | Capability | Summary |
|---|---|---|---:|---:|---|---|
| `thread/start` | Client → Server | Desktop, CLI | yes | required | threads/v1 | 创建 Thread |
| `browser/observe` | Server → Client | Desktop host | no | n/a | browser/v1 | 观察目标 |

Direction 只能使用：

- Client → Server request；
- Server → Client request；
- Server → Client notification；
- 双向 Resource request。

同一 method 在 in-process、stdio、Unix socket 和 WebSocket 上必须保持相同语义。若某个
transport 不支持该能力，必须通过 initialize capability 明确声明，不能提供绕过 dispatcher
的隐藏 Rust 方法。

## 3. 每个 Request 必填内容

每个 request 单独一节，包含：

1. method 字符串；
2. direction；
3. 前置条件；
4. capability 与版本；
5. Params 的字段表；
6. Result 的字段表；
7. 所有错误码；
8. deadline 和 timeout；
9. cancellation 行为；
10. idempotency 规则；
11. owner/connection/thread/turn 路由；
12. 成功 JSON fixture；
13. 每类错误 JSON fixture。

字段表格式：

| Field | Type | Required | Nullable | Constraints | Meaning |
|---|---|---:|---:|---|---|
| `threadId` | `ThreadId` | yes | no | existing | 目标 Thread |
| `maxBytes` | integer | yes | no | 1..262144 | 最大 chunk |

`required` 和 `nullable` 必须分开说明。不能用“可选”同时表达字段缺失和 JSON `null`。

## 4. JSON fixture

请求：

```json
{
  "jsonrpc": "2.0",
  "id": 42,
  "method": "turn/start",
  "params": {
    "idempotencyKey": "req_01...",
    "threadId": "thread_123",
    "input": [
      { "type": "text", "text": "分析当前网页" }
    ]
  }
}
```

成功响应：

```json
{
  "jsonrpc": "2.0",
  "id": 42,
  "result": {
    "turnId": "turn_456",
    "sequence": 18
  }
}
```

错误响应：

```json
{
  "jsonrpc": "2.0",
  "id": 42,
  "error": {
    "code": -32004,
    "message": "IdempotencyConflict",
    "data": {
      "method": "turn/start",
      "idempotencyKey": "req_01..."
    }
  }
}
```

Fixtures 是测试输入，必须是可解析的完整 JSON，不能只写 TypeScript 示例对象。

## 5. 生命周期和状态机

涉及状态的接口必须给出：

- 状态全集；
- 初始状态；
- 合法转换；
- 终态；
- 失败和取消的区别；
- connection 断开时的行为；
- process crash/restart 后的行为。

示例：

```text
Created → Running → Completed
                  → Failed
                  → Cancelling → Interrupted
```

## 6. 顺序与一致性

必须说明：

- 哪个字段提供 durable sequence；
- 哪个字段提供 stream sequence；
- response 与 notification 的因果顺序；
- snapshot 与实时订阅如何避免事件空洞；
- 客户端发现空洞后如何 resync；
- 哪些通知可以合并，哪些不能丢弃。

## 7. 幂等

所有持久化副作用请求必须说明：

- `idempotencyKey` 是否必填；
- ledger scope；
- request hash 包含哪些字段；
- 相同 key + 相同参数的响应；
- 相同 key + 不同参数的错误；
- retention deadline；
- 重启后是否仍可去重。

JSON-RPC `id` 只做当前连接的 response pairing，不能代替 idempotency key。

## 8. 超时、取消和迟到响应

Server → Client 请求必须定义：

- deadline 字段和单位；
- Turn 结束时如何取消；
- connection 断开时如何清理；
- `serverRequest/resolved` 通知；
- deadline 后的迟到 response 是否忽略；
- UI 如何关闭等待状态。

## 9. Connection 与 owner

必须分别说明：

- Thread subscription；
- Turn owner；
- CapabilityHandle owner；
- Resource owner；
- Browser Target owner。

不能用“当前活动客户端”或“当前活动 Tab”这种可变全局概念代替稳定 owner。

## 10. Resource

大对象不得在普通业务消息中使用 Base64 或暴露本地路径。接口文档需要定义：

- `resourceId`；
- MIME type；
- size；
- SHA-256；
- owner connection；
- TTL；
- 单资源与单连接 quota；
- chunk 最大值；
- offset 范围；
- release、过期和断连清理。

## 11. 安全

每个涉及 Browser、文件、命令、网络、下载或外部 URL 的接口必须说明：

- 信任边界；
- 输入验证；
- origin/路径/目标 allowlist；
- secret 和敏感字段脱敏；
- approval 是否需要；
- approval 与 action digest 如何绑定；
- host policy 的二次校验；
- 审计记录。

## 12. 错误码

每个错误码必须稳定，并包含：

| Error | Retryable | Client action | Data |
|---|---:|---|---|
| `ProtocolVersionUnsupported` | no | 升级客户端 | 支持区间 |
| `ServerOverloaded` | yes | jitter backoff | `retryAfterMs` |
| `BrowserTargetUnavailable` | no | 刷新目标列表 | `targetId` |

不要只写“失败时返回 error”。

## 13. 兼容性

文档必须标注：

- 新增字段缺失时的默认行为；
- enum 是否允许 unknown fallback；
- 未知 notification 是否可忽略；
- 未知 request 的错误；
- v1 是否向后兼容；
- 何种变更必须发布 v2。

## 14. Rust 实现验收

API 被视为完成，必须同时具备：

- Rust v1 DTO；
- internal model ↔ DTO mapper；
- handler 与路由；
- capability/version 校验；
- error code；
- timeout/cancellation；
- idempotency；
- Rust tests；
- JSON fixtures；
- in-process App Server Client contract test；
- stdio transport contract test；
- TypeScript 生成；
- JSON Schema 生成；
- schema hash；
- Desktop 与 CLI contract fixtures。

缺少其中任何一项，接口状态仍是 `draft` 或 `review`，不能标记为 `accepted`。

## 15. 可复制模板

建议从独立文件
[`zeta-api-interface-template.md`](zeta-api-interface-template.md)
复制一份作为新接口文档。以下内容与该模板的要求一致：

````markdown
# <Capability / Domain> API

## Metadata

- Status:
- Protocol version:
- Capability version:
- Owner:
- Rust owner:
- Requested by:
- Consumers:
- Last updated:

## Scope

### In scope

### Out of scope

### State owner

## Method inventory

| Method | Direction | Consumers | Side effect | Idempotent | Capability | Summary |

## `<method/name>`

### Semantics

### Preconditions

### Params

| Field | Type | Required | Nullable | Constraints | Meaning |

### Result

| Field | Type | Required | Nullable | Constraints | Meaning |

### Errors

| Error | Retryable | Client action | Data |

### Routing and ownership

### Idempotency

### Deadline and cancellation

### Ordering

### Security

### Request fixture

```json
{}
```

### Success fixture

```json
{}
```

### Error fixtures

```json
{}
```

## State machine

## Compatibility

## Acceptance tests
````
