# `zeta-thread-store`

> 本 README 解释 Thread durability port、stored envelope 和 batch validation。Canonical Thread
> lifecycle 见 [`docs/protocol.md`](../../docs/protocol.md)，Core execution/recovery 见
> [`docs/core.md`](../../docs/core.md)。

`zeta-thread-store` 定义 storage-neutral authoritative Thread history。它只持久化
`zeta_protocol::ThreadEvent` 与 exact `ThreadCommandReceipt`；`ThreadUpdate`、token delta、
actor state、RPC payload 和 Session membership 不进入该 stream。

## 公共契约

| Symbol | 职责 | 关键约束 |
| --- | --- | --- |
| `ThreadStore` | `list_thread_ids / load / append_batch` port | per-Thread sequence 与 atomic batch |
| `ThreadEventBatch` | exact append intent | thread ID、expected sequence、ordered events |
| `AppendBatchResult` | committed batch 摘要 | committed sequence 与 event count |
| `StoredEvent` | storage-owned Thread envelope | schema、event ID、sequence、timestamp、optional command |
| `ThreadCommandReceipt` | accepted `ThreadCommand` exact copy | 支持 idempotent replay 与 payload conflict |
| `ThreadStoreError` | stable store error | invalid batch、sequence conflict、storage failure |

Session membership/lineage 由 `zeta-session-store` 独立保存：

```text
SessionStore  → membership / lineage / Session lifecycle
ThreadStore   → Turn / ThreadItem / interaction / Tool lifecycle
```

两者可以共享物理 storage engine，但不能共享 logical sequence 或 envelope。

## 内部接口地图与调用图

| Symbol | 当前职责 | 方向约束 |
| --- | --- | --- |
| `validate_append_batch` | backend-independent append validation | 所有 implementation 必须在 durable write 前调用等价逻辑 |
| `CURRENT_STORED_EVENT_SCHEMA_VERSION` | 新 Thread event version | reader migration 与 new-write version 必须区分 |
| `StoredEvent::thread_id` | 提供 envelope identity borrow | 不从 inner event 重新推断 aggregate |

```text
ThreadStore::append_batch
├─ obtain actual Thread sequence
├─ validate_append_batch
│  ├─ expected sequence exact match
│  ├─ non-empty batch ID/events
│  ├─ current schema version
│  ├─ batch/envelope/event Thread ID agreement
│  ├─ contiguous event sequence
│  └─ unique event IDs within batch
├─ atomically persist all events
└─ return AppendBatchResult
```

方向偏差：

- 持久化 `ThreadUpdate`：把 transient delivery state 错当成 durable fact；
- command receipt 丢失 exact `ThreadCommand`：无法检测 same ID/different payload；
- start marker 后缺少 terminal result时自动重放：Core recovery 安全边界被绕过；
- backend 自行接受 gap sequence：单写者与 reducer ordering 被破坏；
- `SessionId` 在 Thread 创建后可变：Thread immutable ownership 已被破坏。

## 错误与后端义务

`SequenceConflict` 是正常 concurrency signal，不应被转换为 generic I/O failure。
`InvalidBatch` 表示 caller/backend contract bug；`Storage` 表示 backend failure。

Backend 必须 complete-or-none commit、success-before-durability 禁止、tail 不可见，并保留每条
event 的 exact sequence。Reducer legality 属于 Core，物理 framing/checksum/tail recovery 属于
`zeta-storage`。

## 测试与修改路径

```text
cargo test -p zeta-thread-store
bazel test //zeta-rs/thread-store:thread-store-unit-tests
```

新增 `ThreadEvent` 通常只修改 `zeta-protocol` 与 Core；只有 durable envelope 变化才修改本 crate。
修改 schema/version 时同步审查 storage adapter、rollout recovery、rollout trace 和 protocol
contract tests。

## 当前限制与演进

当前实现提供 version `1` envelope 与 append validator，没有 snapshot、compaction、event migration
registry 或 partial-history query。未来可以增加这些能力，但 `ThreadStore::load` 的 authoritative
ordering、exact command receipt 与 per-Thread atomic append 仍是长期不变量。
