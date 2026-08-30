# `zeta-state`

- 产生 Profile 数据库与索引路径，并统一 SQLite 打开参数、文件权限和迁移。
- 提供 Thread 与 Turn Changes 的 SQLite 存储；schema version 4 删除旧 Session 事件表，Session tree 由 Thread 的 `session_id` 聚合。
- 管理可重建目录索引和跨进程占用锁；不解释 Core 生命周期，也不拥有 Codebase 表结构。
