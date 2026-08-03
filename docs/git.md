# Git 与 Desktop SCM：系统边界和当前状态

> 本文拥有 Git 跨进程 ownership、用户可见语义和演进状态。`zeta-git` 的命令、解析、
> timeout 与失败细节以 [`zeta-rs/git/README.md`](../zeta-rs/git/README.md) 为准；
> external wire shape 以 [`zeta-app-server-api.md`](zeta-app-server-api.md) 为准。

## 快速理解

Desktop Renderer、Native 和 TUI 不启动 Git 进程，也不解析 Git 输出。Workspace-scoped App
Server 接收 typed Git intent，调用 `zeta-git`，再把 client-safe DTO 返回产品入口。

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

| 用户操作 | 当前行为 | 关键限制 |
| --- | --- | --- |
| 查看更改 | 自动读取并投影工作区范围内的 Git 状态 | 当前只支持单工作区 |
| 暂存或取消暂存 | 使用明确的工作区相对路径 | 不能越过工作区边界 |
| 丢弃更改 | 只恢复已跟踪文件，并在界面确认 | 不删除未跟踪文件 |
| 切换本地分支 | Native 点击底栏当前分支，在菜单中选择另一个本地分支；请求通过 App Server | 冲突时 Git 拒绝切换并保留当前工作树 |
| 拉取远端 | 只允许 fast-forward | 需要交互认证时失败 |
| 提交和推送 | 使用系统 Git 的当前仓库配置 | 尚无凭据提示和进度 UI |

## 所有权

| 层级 | 当前职责 | 不拥有 |
| --- | --- | --- |
| Desktop SCM | 分支/upstream、Merge/Staged/Working Tree 分组，提交输入，Git intent，以及按 revision 接收自动状态更新 | Git process、porcelain parser、任意 host path authority |
| Electron bridge | 校验 workspace-relative path、commit message 和空参数，再把 typed Git intent 转发给 App Server | Git domain semantics、最终路径授权 |
| App Server `GitRuntime` | 串行化 operation、维护 workspace projection/revision、消费 watcher hint、去重并发布状态 | Git command/parsing、Renderer state |
| App Server `GitService` | 冻结 workspace root、映射 workspace/repository path、持有 Tokio runtime并调用 `zeta-git` | live projection、notification |
| `zeta-app-server-protocol` | Git query/mutation、`git/statusChanged`、DTO、capability 和 stable error name | process/runtime state |
| `zeta-git` | system Git identity、仓库发现、porcelain-v2 snapshot、HEAD/worktree 文本 Diff 与增删行统计、typed mutation 与结构化 parsing | App Server lifecycle、workspace product boundary、Renderer state |

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
  rename original path、conflict 和 submodule flags，并带 repository-relative `workspacePath`、
  Git runtime `streamInstanceId` 与在该实例内单调递增的 workspace status revision；协议不暴露
  repository 的 host 绝对路径；
- `git/stage`、`git/unstage` 和 `git/discardWorktree` 使用明确 path set；discard 只恢复 tracked
  working-tree 内容，不删除 untracked 文件，Desktop 在执行前要求确认；
- `git/commit` 从 stdin 传入经过校验的 message，并返回新 commit object ID；
- `git/textDiff` 返回 workspace-scoped status、受限 UTF-8 HEAD/worktree 文本与增删行统计；
- `git/branch/list` 返回现有本地分支，`git/branch/switch` 只接受 branch name，并在 host 重新解析为
  当前仓库真实分支后执行切换；
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

Native 通过 `zeta-app-server-client` 消费 `git/textDiff`，在 Composer 底栏展示
`Changes files • +additions -deletions`，并从协议中的原始/修改文本重建 presentation-only
`DiffDocument`。文件内容读取、replacement 计数和 binary/size skip 规则仍由 `zeta-git` 统一拥有；
Native 只负责标签、侧栏状态和 MultiDiff presentation。点击 Changes action 会请求刷新 Git
projection、展开右栏并选择 Changes Pane。cwd picker 使用 `workspacePath` 从当前可信工作区还原
repository root 快捷项；选择该项会把工作区切换到 repository root，使 Changes 投影覆盖整个仓库。

Native 的底栏分支按钮复用通用 `ContextMenu`，候选项来自 `git/branch/list`，切换通过
`git/branch/switch`。Git 对脏工作树或 linked worktree 冲突保持权威：失败时不重试、不丢弃改动，
菜单保留并显示失败；成功后使用新的 typed projection 刷新 Files、HEAD、Changes 和 MultiDiff。
`zeterm/zeterm` 不再依赖 `zeta-git`。

稳定失败边界为 `GitUnavailable`、`GitNotRepository` 和 `GitOperationFailed`。内部 executable、
stderr、磁盘绝对路径和非 UTF-8 path 不进入 Renderer。

## 当前限制

- 当前是单 workspace `GitRuntime`，尚无 multi-repository registry；
- operation 由 runtime mutex 串行化，但尚无可观测 queue、progress、caller cancellation 或 retry；
- App Server 与 Native 已支持切换现有本地分支；系统仍无 branch 新建/删除/重命名、
  tag/worktree mutation 或 credential prompt；TUI 尚未投影 Git UI；
- pull 固定为 fast-forward only；discard 不删除 untracked 文件；
- 当前是单 workspace root contract，不是 multi-root repository collection；
- SCM change row 尚未接入 editor diff/open workflow。

这些限制必须在 UI 中保持可见：未实现的 mutation 不注册空命令，也不显示会误导用户的按钮。

## 分阶段演进

近期扩展顺序：

1. 增加显式 repository identity 与 multi-root registry；
2. 为长时间 remote operation 增加 progress、queue state 和 caller cancellation；
3. 接入 diff/open 与更细粒度的错误 UI；
4. 按明确产品语义增加 branch lifecycle 等额外 mutation，并让 TUI 消费同一协议。

长期不变量是：Desktop、Native 与 TUI 不直接执行 Git；App Server adapter 不复制 Git
command/parsing；watch event 只触发重新确认，不能自身成为 repository truth。
