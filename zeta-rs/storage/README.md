# `zeta-storage`

> 本 README 负责 SQLite 物理存储契约。Session/Thread envelope、batch validator 与 trait 分别由
> [`zeta-session-store`](../session-store/README.md) 和
> [`zeta-thread-store`](../thread-store/README.md) 拥有；跨 crate 恢复顺序见
> [`zeta-rollout`](../rollout/README.md)。

`zeta-storage` 是本地 Session/Thread authority 的 SQLite adapter。它不定义领域 event、
reducer、Config schema、trace/export 格式或 UI query model。

## 当前接口

| Symbol | 职责 |
| --- | --- |
| `SqliteSessionStore` | 实现 `SessionStore`，读写 `session_*` tables |
| `SqliteThreadStore` | 实现 `ThreadStore`，读写 `thread_*` tables |
| `LeaseDirectory` | 为 Core aggregate writer 提供 profile-scoped advisory lease |

两个 store 必须打开同一 `<profile_root>/state.sqlite3`。每个 adapter 使用独立 connection；
SQLite WAL 协调它们的并发事务。

## Schema 与提交

`sqlite::connection::open` 是物理 schema 的唯一安装入口：

```text
zeta_schema_migrations(component = "event-store")
thread_streams / thread_batches / thread_events
session_streams / session_batches / session_events
```

每次 append 固定执行：

```text
BEGIN IMMEDIATE
→ materialize aggregate stream row
→ read current sequence
→ run domain batch validator
→ reject duplicate batch/event identity
→ insert full typed envelopes
→ compare-and-set current sequence
→ COMMIT
```

任一步失败都会回滚整批，不暴露部分 event。完整 typed envelope 以 JSON 保存，同时将 aggregate
identity、sequence、event ID 和 schema version 放入稳定列，用于唯一性和顺序查询。加载后仍会
调用领域 validator，数据库内容不能绕过 envelope contract。

连接统一启用：

- foreign keys；
- WAL；
- `synchronous=FULL`；
- 5 秒 busy timeout；
- component schema version fail-closed。

SQLite 在这里是 authority，不是可删除 projection。旧 JSONL rollout、tail recovery、checksum
frame 和双写路径已移除；当前开发期不自动导入旧格式。

## 所有权与漂移信号

- `sqlite::connection` 拥有物理表、PRAGMA 与 schema version；
- `sqlite::session` 只做 `SessionStore` 映射；
- `sqlite::thread` 只做 `ThreadStore` 映射；
- batch legality 继续委托 `validate_session_append_batch` / `validate_append_batch`；
- recovery 顺序继续由 `LocalStateRepository` 拥有。

若本 crate 开始解释 Session/Thread lifecycle、生成 event、组合 Core，或让 Config 表依赖
Session/Thread reducer，即表示所有权漂移。若 App Server 直接写这些表，同样是绕过 store port。

## 验证

```text
cargo test -p zeta-storage
```

测试覆盖共享数据库重开、typed event recovery、sequence conflict、原子 batch 与 writer lease。
新增 schema migration 时还必须增加旧 schema fixture、失败中断和重开测试。
