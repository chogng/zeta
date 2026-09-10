# `zeta-state`

- 产生 Profile 数据库与索引路径，并统一 SQLite 打开参数、文件权限和迁移。
- 提供本机输入历史的 SQLite 存储，独立迁移、原子追加与裁剪、稳定游标和文字搜索。
- 提供 Thread、Git Turn Changes 与 Project 的 SQLite 存储；各领域使用独立 migration component，Session tree 仍由 Thread 的 `session_id` 聚合。
- 管理可重建目录索引和跨进程占用锁；不解释 Core 生命周期，也不拥有 Codebase 表结构。
