# `git-turn-changes`

> 本 README 拥有按 Turn 归属的 Git 变更集领域契约；工作树和 Git 原语分别由 [`worktree`](../worktree/README.md) 与 [`zeta-git`](../git/README.md) 拥有，跨组件产品语义见 [`docs/chat-session-inspector.md`](../../docs/chat-session-inspector.md)。

1. `TurnChangeLedger` 为每个 `SessionId + ThreadId + TurnId + repository_id` 串行捕获不可变 Git tree/blob before/after 检查点；`GitTurnChangeWatcher` 消费文件事件并刷新打开的 ChangeSet。
2. `TurnChangeSet` 分别维护 capture/message/commit 状态、revision、工具读写归属、ChangeSet 依赖、初始工作区依赖与用户 draft；`Open`、`Incomplete`、`Discarded` 或依赖未满足的记录不能提交。
3. `TurnChangeStore` 定义完整记录 CAS；SQLite 实现还原子保存 mutation command receipt，确保相同 command/payload 重放首次响应，而不是重复排队摘要或提交任务。

非 Git Thread 不创建 ChangeSet。普通目录的物化、恢复和清理由 `worktree` 独立完成，不进入本 crate。
