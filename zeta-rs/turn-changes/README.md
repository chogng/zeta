# `zeta-turn-changes`

> 本 README 拥有 Turn 变更集的领域契约；工作树和 Git 操作分别由 [`zeta-worktree`](../worktree/README.md) 与 [`zeta-git`](../git/README.md) 拥有，跨组件产品语义见 [`docs/chat-session-inspector.md`](../../docs/chat-session-inspector.md)。

1. `TurnChangeLedger` 为每个 `SessionId + ThreadId + TurnId + repository_id` 串行捕获不可变 before/after 检查点；Git 使用 tree/blob，非 Git 使用内容寻址 manifest/blob，并保留文本、二进制、删除、重命名、符号链接与执行位。
2. `TurnChangeSet` 分别维护 capture/message/commit 状态、revision、工具读写归属、ChangeSet 依赖、初始工作区依赖与用户 draft；`Open`、`Incomplete`、`Discarded` 或依赖未满足的记录不能提交。
3. `TurnChangeStore` 定义完整记录 CAS；SQLite 实现还原子保存 mutation command receipt，确保相同 command/payload 重放首次响应，而不是重复排队摘要或提交任务。
