# Memories

- 拥有长期 Memory 的身份、作用域、用户来源、revision、分页、搜索、引用读取和删除语义。
- 首版只接受用户显式 add、list、read、search 和 delete；不自动读取、写入或调用模型。
- `MemoryStore` 是持久化 port；SQLite 实现由 `zeta-state` 提供，App Server 只做权限与协议转换。
- Profile、Project 和 Dir 是互不替代的作用域；Dir 使用稳定 `DirId`，不持久化路径。
- 删除后正文不再进入 live record、command receipt 或 tombstone，旧 add command 不会恢复已经删除的 Memory；SQLite page、WAL 和外部备份的物理清理由存储维护策略负责。
- 分页 cursor 绑定 catalog revision、作用域和查询；目录变化后旧 cursor 明确失效。
- 验证：`just test zeta-memories`、`just test zeta-state memory_store` 和 App Server Memory 集成测试。
