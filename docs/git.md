# Git 与 Desktop SCM：系统边界和当前状态

> 本文拥有 Git 跨进程 ownership、用户可见语义和演进状态。`zeta-git` 的命令、解析、
> timeout 与失败细节以 [`zeta-rs/git/README.md`](../zeta-rs/git/README.md) 为准；
> external wire shape 以 [`zeta-app-server-api.md`](zeta-app-server-api.md) 为准。

## 决策摘要

Desktop Renderer 不启动 Git 进程，也不解析 Git 输出。Workspace-scoped App Server 接收 typed
Git intent，调用 `zeta-git`，再把 renderer-safe DTO 返回 TypeScript。

```text
Desktop SCM View
  ↔ Electron typed IPC + generic notification
  ↔ App Server GitRuntime
  → GitService
  → zeta-git
  → system Git
```

这条依赖方向让 workspace path authority、Git executable identity、process limits 和 output
parsing 留在 Rust host。Renderer 只拥有展示状态和用户 intent。

## 所有权

| 层级 | 当前职责 | 不拥有 |
| --- | --- | --- |
| Desktop SCM | 分支/upstream、Merge/Staged/Working Tree 分组，提交输入，Git intent，以及按 revision 接收自动状态更新 | Git process、porcelain parser、任意 host path authority |
| Electron bridge | 校验 workspace-relative path、commit message 和空参数，再把 typed Git intent 转发给 App Server | Git domain semantics、最终路径授权 |
| App Server `GitRuntime` | 串行化 operation、维护 workspace projection/revision、消费 watcher hint、去重并发布状态 | Git command/parsing、Renderer state |
| App Server `GitService` | 冻结 workspace root、映射 workspace/repository path、持有 Tokio runtime并调用 `zeta-git` | live projection、notification |
| `zeta-app-server-protocol` | Git query/mutation、`git/statusChanged`、DTO、capability 和 stable error name | process/runtime state |
| `zeta-git` | system Git identity、仓库发现、porcelain-v2 snapshot、typed mutation 与结构化 parsing | App Server lifecycle、workspace product boundary、Renderer state |

## 当前状态

当前已经实现 status 与常用用户 mutation 的完整纵向切片：

- App Server 只在 local composition 收到可信 workspace root 时声明
  `initialize.capabilities.git = true`；
- `git/status` 不接受路径；mutation 只接受 workspace-relative path，Electron 和 Rust 边界都会
  拒绝绝对路径、空路径和父目录逃逸；
- workspace 位于更大 repository 内时，App Server 会过滤 repository 其他目录并把 path 重新映射为
  workspace-relative；mutation 则把合法 workspace path 映射回 repository-relative path；
- 每次请求重新打开仓库并读取 authoritative snapshot，不把旧 snapshot 当作 mutation 前提；
- response 保留 HEAD branch/detached/unborn、upstream ahead/behind、index/worktree 状态、
  rename original path、conflict 和 submodule flags，并带 Git runtime `streamInstanceId` 与在
  该实例内单调递增的 workspace status revision；
- `git/stage`、`git/unstage` 和 `git/discardWorktree` 使用明确 path set；discard 只恢复 tracked
  working-tree 内容，不删除 untracked 文件，Desktop 在执行前要求确认；
- `git/commit` 从 stdin 传入经过校验的 message，并返回新 commit object ID；
- `git/fetch` 执行 all-remotes prune，`git/pull` 仅允许 fast-forward，`git/push` 使用 Git 当前
  upstream/default 配置；所有 remote operation 都是 non-interactive；
- 每个成功 mutation 都返回新的 `GitStatusResult`，Desktop 立即重绘；首次打开 View 也会自动刷新；
- App Server 监听 workspace、Git metadata 和 workspace 上层 repository `.gitignore`，以 100ms
  debounce 合并 burst；事件只触发重新查询，不直接成为 Git 状态；
- 新 snapshot 按 workspace 投影去重；HEAD/change 未改变时 revision 不推进也不发通知，变化时通过
  `git/statusChanged` 推送完整 snapshot；
- SCM View 只在相同 `streamInstanceId` 内按 revision 拒绝旧 notification/response；连接重新
  ready 时主动刷新，并接受新 runtime 从较小 revision 开始的 snapshot。已退役实例的迟到通知
  不能覆盖新状态。Watcher 初始化失败不会关闭 Git RPC，用户仍可手动 Refresh。

稳定失败边界为 `GitUnavailable`、`GitNotRepository` 和 `GitOperationFailed`。内部 executable、
stderr、磁盘绝对路径和非 UTF-8 path 不进入 Renderer。

## 当前限制

- 当前是单 workspace `GitRuntime`，尚无 multi-repository registry；
- operation 由 runtime mutex 串行化，但尚无可观测 queue、progress、caller cancellation 或 retry；
- 尚无 branch/tag/worktree mutation，也没有 credential prompt；需要交互认证的 remote operation 会失败；
- pull 固定为 fast-forward only；discard 不删除 untracked 文件；
- 当前是单 workspace root contract，不是 multi-root repository collection；
- SCM change row 尚未接入 editor diff/open workflow。

这些限制必须在 UI 中保持可见：未实现的 mutation 不注册空命令，也不显示会误导用户的按钮。

## 分阶段演进

近期扩展顺序：

1. 增加显式 repository identity 与 multi-root registry；
2. 为长时间 remote operation 增加 progress、queue state 和 caller cancellation；
3. 接入 diff/open 与更细粒度的错误 UI；
4. 按明确产品语义增加 branch 等额外 mutation。

长期不变量是：Desktop 不直接执行 Git；App Server adapter 不复制 Git command/parsing；watch
event 只触发重新确认，不能自身成为 repository truth。
