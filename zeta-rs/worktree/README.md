# `zeta-worktree`

> 本 README 拥有已有 Git worktree 的切换目标解析和 Codex 兼容归属元数据实现契约；跨 crate 产品状态见 [`docs/git.md`](../../docs/git.md)。

1. `WorktreeManager::list` 和 `WorktreeManager::resolve` 通过 `zeta-git` 读取同一仓库的 Git worktree 清单，拒绝不可用目标，并把源工作区在仓库内的相对子目录映射到目标 checkout；crate 不启动 Git、不替换产品工作区、不启动 Session，也不拥有选择器 UI。
2. `WorktreeSettings::defaults` 和 `WorktreeSettings::from_desktop_config` 保留 Codex Desktop 的 `git-worktree-root`、`worktree-auto-cleanup-enabled`、`worktree-keep-count` 语义；`bind_thread` 和 `owner` 只接受 `<root>/<4 hex>/<checkout>` 下的 linked worktree，并以不可覆盖的原子写入维护 `codex-thread.json`。
3. `just test zeta-worktree` 覆盖 primary/linked 清单、nested cwd 映射、按分支和绝对路径解析、损坏 owner 隔离、并发归属、managed layout 拒绝与配置校验；新增创建、清理或保留策略前必须同时实现明确的文件变更和删除失败语义，不能把设置字段当成已经执行的清理能力。
