# `zeta-rollout`

> 本 README 解释本地 durable repository 的组合与恢复顺序。Session/Thread store contract 分别见
> [`zeta-session-store`](../session-store/README.md) 和
> [`zeta-thread-store`](../thread-store/README.md)；Core ownership 见
> [`docs/core.md`](../../docs/core.md)，workspace recovery 方向见
> [`docs/zeta-rs-architecture.md`](../../docs/zeta-rs-architecture.md)。

`zeta-rollout` 是本地 state composition crate：在一个 profile root 下打开共享
`state.sqlite3` 的 typed Session/Thread stores 与 writer lease，并按 dependency order 恢复
`SessionCoordinator`。它不定义 event、SQLite schema、reducer 或 trace format；crate 名是迁移
SQLite 前的历史名称。

## 公共与内部接口

| Symbol | 可见性 | 职责 |
| --- | --- | --- |
| `LocalStateRepository` | public | 持有同一数据库的 `SqliteSessionStore`、`SqliteThreadStore` 与同一 profile 的 `LeaseDirectory` |
| `LocalStateRepository::open` | public | 使用同一个 `StateRuntime` 打开 `state.sqlite3` 与 writer lease 目录 |
| `database_path` | public | 暴露同一 SQLite path，供 Config authority 加入同一 profile 数据库 |
| `session_store` | public | 以 `Arc<dyn SessionStore>` 暴露 typed history port |
| `thread_store` | public | 以 `Arc<dyn ThreadStore>` 暴露 typed history port |
| `recover_coordinator` | public | 唯一受支持的 Core runtime recovery construction |
| `LocalStateError` | public | 保留 Core、SessionStore、ThreadStore error domain |
| repository fields | private | 防止调用方拆开 recovery generation 或替换其中一个 store |

```text
LocalStateRepository::open(state_runtime)
├─ SqliteSessionStore::open(state_runtime.database_path)
├─ SqliteThreadStore::open(state_runtime.database_path)
└─ LeaseDirectory::open(state_runtime.writer_leases_root)

LocalStateRepository::recover_coordinator
├─ ThreadController::with_store_and_lease
├─ ThreadStore::list_thread_ids
├─ ThreadController::recover_thread(each)
├─ SessionCoordinator::with_store_and_lease
├─ SessionStore::list_session_ids
├─ SessionCoordinator::recover_session(each)
└─ return Arc<SessionCoordinator>
```

Thread 必须先恢复，因为 Session recovery 可能 reconcile 一个 durable
`ThreadCreationPlanned` saga。交换这两个循环不是重构，而是 recovery semantic change。

方向偏差：

- App Server 自行打开 stores/leases：composition ownership 被复制；
- Session 在 Thread 前恢复：create/fork saga 可能错误补偿；
- `LocalStateRepository` 定义新 event envelope 或 SQLite table：侵入 store/protocol ownership；
- trace/export write 回 repository：read-only diagnostics 变成第二 authority；
- 一个 store 或 lease 使用不同 root generation：runtime 不再代表一致 repository。

## 失败与生命周期

`LocalStateError` 不扁平化来源：open/recovery caller 可以区分 Core recovery、Session store 与 Thread
store failure。`open` 只创建 storage handles；只有 `recover_coordinator` 构造并恢复 runtime。

Repository 返回 cloned `Arc<dyn ...Store>`，它们属于同一 opened database generation。profile
变化应创建新的 repository，不应原地替换内部 backend。普通 Config 变化不重开 repository；
`ConfigStore` 使用该 repository 暴露的 `database_path` 打开自己的 authority tables。

## 测试与修改路径

```text
cargo test -p zeta-rollout
bazel test //zeta-rs/rollout:rollout-unit-tests
```

测试使用真实临时 root，验证创建 Session/Thread 后重新打开 repository 可以恢复 membership 与
Thread history。修改恢复顺序时必须增加 interrupted create/fork saga case，而不只测试 happy path。

## 当前限制与演进

当前只组合本地 SQLite `zeta-state` backend，没有 remote repository、encryption policy、
multi-process coordination protocol、旧 JSONL import 或 lazy recovery。未来 backend 可以变化，
但 typed store ports、shared lease generation 和 Thread-before-Session recovery 是稳定边界。
