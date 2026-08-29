## 快速理解

`zeta-state` 只负责三件事：

- 统一产生 Profile 的持久数据库路径，并负责 SQLite 打开参数、权限和迁移。
- 提供 Session、Thread、Turn Changes 的 SQLite 存储实现，但不解释这些领域对象。
- 管理 Workspace 可重建索引的目录、跨进程占用锁和显式删除。

Codebase 的表结构和查询不在这里，它们由 `zeta-codebase-store` 负责；`zeta-state` 只提供数据库运行环境。

```text
cargo test -p zeta-state
```
