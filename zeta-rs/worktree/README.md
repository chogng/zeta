# `zeta-worktree`

> 本 README 拥有已有 Git worktree 的切换目标解析和 Codex 兼容归属元数据实现契约；跨 crate 产品状态见 [`docs/git.md`](../../docs/git.md)。

1. `WorktreeManager::list/resolve` 读取同仓库 worktree 清单；`provision_thread` 在 Thread 执行前持久化独占工作区：Git 使用 detached linked worktree，非 Git 使用内容寻址快照支持的受管目录，创建失败不会转回共享工作区。
2. `ThreadWorktreeBinding` 固定来源工作区、目标 branch/HEAD、初始 baseline 与受管目录身份；恢复只接受 `<root>/<4 hex>/<digest>` 的有效绑定，清理必须收到全部 ChangeSet 已 settled 的证明。
3. `WorktreeSettings` 保留 `git-worktree-root`、自动清理开关和保留数量语义；本 crate 只负责隔离目录和生命周期，不拥有 Turn 归属、提交信息、Git 提交或 Session Inspector。
