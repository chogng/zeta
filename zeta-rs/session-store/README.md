# `zeta-session-store`

> 本 README 解释 Session durability port、stored envelope 和 batch validation。Canonical
> Session domain 见 [`docs/protocol.md`](../../docs/protocol.md)，跨 Session/Thread 的恢复与
> saga 见 [`docs/core.md`](../../docs/core.md)。

`zeta-session-store` 定义 storage-neutral Session event-stream contract。它保存
`SessionEvent` 和 exact typed `SessionCommandReceipt`，但不实现文件格式、reducer、runtime
mailbox、Thread transcript 或 RPC。

## 边界与 public contract

| Symbol | 职责 | 关键约束 |
| --- | --- | --- |
| `SessionStore` | `list_session_ids / load / append_batch` port | 每个 Session 独立 sequence；batch 原子可见 |
| `SessionEventBatch` | 一次 append intent | exact `session_id + expected_sequence + events` |
| `AppendSessionBatchResult` | 已提交 batch 摘要 | 返回 committed sequence 和 event count |
| `StoredSessionEvent` | storage-owned durable envelope | schema、event ID、sequence、timestamp、optional command receipt |
| `SessionCommandReceipt` | accepted command 的 exact typed copy | 用于 idempotency/conflict recovery，不是日志文本 |
| `SessionStoreError` | stable boundary error | 区分 invalid batch、sequence conflict 和 backend storage |

本 crate 不拥有 `SessionEvent` 语义；它来自 `zeta-protocol`。Backend 实现必须复用这里的
envelope 与 validator，不能发明第二套 Session event schema。

## 内部接口地图与调用路径

核心函数只有一个，但它是所有 backend 的写入前置条件：

| Symbol | 当前职责 | 不得迁移的语义 |
| --- | --- | --- |
| `validate_session_append_batch` | 对照 backend actual sequence 校验完整 batch | identity、sequence、schema version、batch/event ID 约束必须在 durable append 前成立 |
| `CURRENT_SESSION_EVENT_SCHEMA_VERSION` | 新写入 event 的 schema version | migration reader 可以支持旧版本，新写入不能任意选择版本 |

```text
backend append_batch
├─ read current Session sequence
├─ validate_session_append_batch(batch, actual_sequence)
│  ├─ expected sequence exact match
│  ├─ non-empty batch ID
│  ├─ at least one event
│  ├─ current schema version
│  ├─ envelope/event/batch Session ID agreement
│  ├─ contiguous sequence
│  └─ unique event ID within batch
├─ atomically persist complete batch
└─ expose AppendSessionBatchResult
```

方向偏差：

- backend 跳过 `validate_session_append_batch`：不同 backend 会产生不同 durability 语义；
- partial event 可被 `load` 观察：batch atomicity 已破坏；
- `ThreadEvent` 或 transcript 进入 `StoredSessionEvent`：aggregate ownership 已漂移；
- command 只保存 ID、不保存 exact `SessionCommand`：replay conflict 无法可靠判断；
- Session 与 Thread 共用逻辑 sequence：独立并发边界已被错误合并。

## Validation 与错误

`validate_session_append_batch` 按 deterministic 顺序返回：

- `SequenceConflict { expected, actual }`：caller snapshot 已过期；
- `InvalidBatch`：空 ID、空 events、schema、identity、sequence 或 duplicate event ID 错误；
- `Storage` 只能由 backend 表达 I/O、corruption 或 durable facility failure。

Validator 不检查 reducer-level 合法状态迁移。Core 负责产生合法 `SessionEvent`；store 只验证
storage envelope 与 append contract。

## Backend 实现要求

一个 `SessionStore` backend 必须：

1. 对每个 Session 串行比较 actual sequence；
2. 在同一原子边界内提交整个 batch；
3. success 前完成所声明的 durability；
4. `load` 不返回未提交 tail；
5. `list_session_ids` 不虚构空 aggregate；
6. 保留 exact event/command bytes 的语义；
7. 将 backend detail 映射为 sanitized `SessionStoreError::Storage`。

物理 JSONL framing、checksum、tail recovery 和 filesystem layout 当前由 `zeta-storage`
实现，不应反向进入这个 port。

## 测试与修改路径

```text
cargo test -p zeta-session-store
bazel test //zeta-rs/session-store:session-store-unit-tests
```

修改 stored field 或 schema version 时，同时审查 serde compatibility、`zeta-storage`
adapter、rollout recovery、trace export 和 fixtures。修改 batch validation 时，SessionStore 与
ThreadStore 的共同原则应保持一致，但不要抽象掉各自的 ID、event 和 sequence 类型。

## 当前限制与演进

当前只有 schema version `1` 和 validation contract；migration registry、compaction、
snapshot/checkpoint 与 corruption diagnostics 尚不属于本 crate 的已实现能力。未来可以增加
versioned reader/migration policy，但不能削弱 append atomicity、exact command receipt 或
per-Session sequence。
