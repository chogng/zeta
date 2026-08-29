## 快速理解

`zeta-codebase-store` 只负责三件事：

- 实现 Codebase 定义的源码、符号和向量存储接口。
- 拥有这些 SQLite 表、迁移、查询和领域记录映射。
- 使用 `zeta-state` 提供的 Profile 路径、数据库权限和 Workspace 占用锁。
