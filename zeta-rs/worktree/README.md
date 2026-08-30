# `zeta-worktree`

> 本 README 拥有已有 Git worktree 的切换目标解析和 Codex 兼容归属元数据实现契约；跨 crate 产品状态见 [`docs/git.md`](../../docs/git.md)。

1. `WorktreeManager::list/resolve` 读取同仓库 worktree 清单；`provision` 为 Thread 或 WorkAttempt 根建立独占受管目录，Git 使用 detached linked worktree，非 Git 使用内容寻址目录快照。
2. `ManagedDirBinding` 固定执行所有者、来源 `DirId`、仓库身份、精确目标、不可变 baseline 和绑定摘要；一个 WorkAttempt 可为同一 Thread 持有多个互不共享的根。
3. 本 crate 只负责受管目录的物化、恢复和清理；Turn 归属、工作契约、验证、提交与产品状态由上层领域拥有。
