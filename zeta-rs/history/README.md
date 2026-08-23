# `zeta-history`

> 本 README 解释模型历史与持久化 Thread record 的数据契约。Canonical `ThreadEvent` 语义见
> [`docs/protocol.md`](../../docs/protocol.md)，恢复与追加接口见
> [`zeta-thread-store`](../thread-store/README.md)，物理 SQLite 映射见
> [`zeta-storage`](../storage/README.md)。

`zeta-history` 是纯数据层：它定义“一个 Thread 事实被长期保存时长什么样”。它不打开文件或
数据库，不做分页、事务、恢复、归约、模型调用，也不拥有 TUI 展示。

## 职责边界

| 问题 | Owner | 本 crate 是否负责 |
| --- | --- | --- |
| 哪些事实可以持久化？ | `zeta-protocol::ThreadEvent` | ❌，只引用 canonical fact |
| 事实落盘时携带哪些稳定元数据？ | `zeta-history` | ✅ |
| 如何完整读取、追加和报告 sequence conflict？ | `zeta-thread-store` | ❌ |
| 如何返回有界 Turn 窗口？ | Core projection / App Server | ❌ |
| 如何写入 SQLite、提交事务和恢复？ | `zeta-storage` / `zeta-core` | ❌ |
| 如何显示旧消息？ | App Server client / TUI | ❌ |

因此它与 Codex 的 `codex-history` 一样属于 persisted-history domain types，而不是第二个
Thread Store。Zeta 仍只有一份 authoritative Thread stream。

## 公共契约

| Symbol | 职责 | 关键约束 |
| --- | --- | --- |
| `StoredEvent` | 一个 durable `ThreadEvent` 的 canonical envelope | 保留 thread、sequence、event ID、时间戳与可选 command receipt |
| `ThreadCommandReceipt` | 被接受命令的 exact typed copy | 支持恢复后的幂等结果与同 ID/different payload 冲突检测 |
| `EventId` | 单条历史记录 identity | 同一 stream 内不得重复 |
| `Timestamp` | Unix milliseconds | 只描述记录时间，不参与 aggregate ordering |
| `CURRENT_STORED_EVENT_SCHEMA_VERSION` | 新记录使用的 schema | writer 只产生 current version |
| `MINIMUM_SUPPORTED_EVENT_SCHEMA_VERSION` | recovery 可接受的下界 | reducer 显式拒绝区间外版本 |
| `supports_stored_event_schema_version` | 判断 persisted record 是否可读 | 读取兼容区间与新写 current version 分开 |

```text
zeta-protocol::ThreadEvent
          │
          ▼
zeta-core constructs StoredEvent
          │
          ▼
zeta-thread-store validates recovery/append contract
          │
          ▼
zeta-storage serializes exact record to SQLite
          │
          ├─► zeta-core replays it during recovery
          └─► zeta-rollout-trace exports a read-only copy
```

关键私有实现是 `record::StoredEvent` 及其 serde shape。`StoredEvent::thread_id` 返回 envelope
identity；Store 校验还会确认 inner `ThreadEvent::thread_id()` 与它一致，不能从 payload 猜测或
覆盖 envelope identity。

## 兼容性、失败与修改影响

本 crate 不定义 I/O error。反序列化失败由 storage adapter 映射成 storage failure；schema
版本不受支持由 Core recovery 或 Store write validation 明确拒绝。`record` 的 field name、serde
tag、默认值或类型变化都是持久化兼容性变化，必须同步审查：

- `zeta-core` 的 record 构造与 reducer recovery；
- `zeta-thread-store::validate_append_batch`；
- `zeta-storage::sqlite::thread`；
- `zeta-rollout-trace` export 与 fixtures。

```text
cargo test -p zeta-history
bazel test //zeta-rs/history:history-unit-tests
```

## 当前限制与演进

当前新写入使用 schema version `8`，reader 仍接受 minimum version `1`；version 8 覆盖稳定
`ToolRepetition` Turn failure，version 7 覆盖运行中 `SteerTurn` 的 durable Item binding 与 backend
delivery fact，version 6 覆盖 Turn 级供应商上下文溢出恢复 checkpoint，早期版本覆盖 Agent context seed、delegation、message/result facts、自动
Skill activation command snapshot 和结构化工具绑定。本 crate
只抽取已经在生产路径中使用的 Thread history record；Session envelope 仍由
`zeta-session-store` 拥有。它没有照搬 Codex 尚无真实消费者的 `InitialHistory`、rollout JSONL
line 或 harness metadata。未来只有当 Core、import/fork 或另一种持久化后端需要共同理解新的
history-only 数据时，才在这里增加类型；查询、缓存和物理写入仍留在 Store/Storage。
