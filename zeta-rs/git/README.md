# `zeta-git`

> 本 README 是 Zeta 本地 Git 实现的 crate-level canonical contract。它说明当前代码、关键
> private symbol、失败语义与安全修改路径。未来 Desktop/App Server Git 产品语义需要独立系统
> 文档；模型 Tool 和 approval 边界仍由 [`docs/tools.md`](../../docs/tools.md) 与
> [`docs/sandboxing.md`](../../docs/sandboxing.md) 维护。

`zeta-git` 是 Zeta 中“如何调用 Git、如何解释 Git 结果”的唯一实现 owner。完整 owner 不等于
当前已经实现完整 SCM：本阶段提供仓库打开、结构化状态快照、本地 branch、remote、最近 commit
和 patch check/apply；持续监听、状态缓存、stage/commit/fetch/push/worktree mutation 与前端协议
尚未实现。

## 为什么不是普通 `git utils`

本 crate 吸收了 Codex `git-utils` 中值得保留的 agent/host 侧优化，但把这些能力组织在一个明确的
Git domain owner 下，而不是建立平级的 `zeta-git-utils`：

| 容易被低估的能力 | 当前实现价值 |
| --- | --- |
| 有界进程 | query 默认 5 秒、mutation 默认 30 秒；stdout/stderr 各自有 8 MiB 上限，达到上限后仍继续 drain child，最终返回明确错误 |
| 非交互执行 | 禁用 terminal prompt 和 credential-manager 交互，固定 `LC_ALL=C`，避免后台调用等待 UI/input |
| repository config 隔离 | 所有内部命令禁用 configured hooks；query 禁用 optional locks |
| fsmonitor 安全与性能 | 不执行 repository-selected fsmonitor helper；仅当配置确实是 boolean true 且 Git 声明 built-in daemon capability 时保留 daemon |
| 机器可读 status | 使用 porcelain v2 + NUL 分隔，保留 index/worktree、rename original path、conflict、submodule 和 upstream ahead/behind |
| 结构化 patch | 用 enum 区分 check/apply 与 forward/reverse；区分 `AppliedWithConflicts` 和未应用的 `Rejected`，不与 spawn/timeout/输出损坏混为一谈 |
| 路径边界 | `git apply` 不启用 `--unsafe-paths`；diff header path 做排序、去重和 quoted-path 解析 |

因此，新增 Git 能力时应扩展本 crate，而不是在 App Server、Desktop adapter 或 Tool 中直接新增
`Command::new("git")`。

## 当前 ownership

| 文件 | 当前职责 | 关键 symbol |
| --- | --- | --- |
| `src/client.rs` | Git executable identity、process profile、timeout、bounded capture、non-interactive config | `GitClient`、`GitExecutionLimits`、private `GitInvocation`、`GitCommandProfile`、`read_bounded` |
| `src/repository.rs` | 从已有 path 打开 working tree，解析 worktree/git/common metadata path | `GitRepository`、`GitRepositoryKind`、`existing_directory` |
| `src/status.rs` | porcelain-v2 snapshot 与 HEAD/change/submodule model | `GitRepositorySnapshot`、`GitHead`、private `parse_status` |
| `src/info.rs` | local branches、fetch/push remote URLs、bounded recent history | `GitBranch`、`GitRemote`、`GitCommitSummary` |
| `src/patch.rs` | patch request/result、stdin apply、path extraction 和 diagnostics 分类 | `GitPatchRequest`、`GitPatchResult`、private `parse_apply_diagnostics` |
| `src/fsmonitor.rs` | effective config 与 built-in daemon capability 探测 | private `detect_fsmonitor_override` |
| `src/path.rs` | porcelain path bytes 到 platform `PathBuf` | private `path_from_git_bytes` |
| `src/error.rs` | transport、timeout、limit、Git exit 和 parse failure 的稳定区分 | `GitError` |

所有实现 module 均为 private，`src/lib.rs` 显式导出 crate API。把 App Server repository registry、
watch subscription、operation queue 或 wire DTO 放进上述 module，意味着 ownership 已经漂移。

## Public API 与真实调用路径

`GitClient` 冻结 executable path 和 `GitExecutionLimits`。`GitClient::system()` 使用进程启动环境
解析 `git`；需要 bundled/显式 executable identity 的 host 应在启动阶段调用
`GitClient::with_executable`，并长期复用返回值。

```text
GitClient::open_repository
└─ GitClient::run_query_unchecked
   └─ git rev-parse --path-format=absolute
      ├─ worktree root
      ├─ git dir
      └─ common git dir

GitClient::snapshot
├─ detect_fsmonitor_override
│  ├─ git config --null --get core.fsmonitor
│  └─ git version --build-options
└─ GitClient::run_query_with_fsmonitor
   └─ git status --porcelain=v2 --branch -z
      └─ parse_status

GitClient::local_branches / remotes / recent_commits
└─ GitClient::run_query
   └─ strict parser for the command-specific output

GitClient::apply_patch
├─ extract_patch_paths
├─ GitClient::run_mutation_with_stdin
│  └─ git apply --recount [--check | --3way] [-R] -
└─ parse_apply_diagnostics
```

`GitRepository` 只能由 `open_repository` 构造，后续 API 依赖其中已解析的 worktree root。
`GitRepositoryKind::LinkedWorktree` 表示 per-worktree `git_dir` 与 shared `common_dir` 不同；
`Standard` 不进一步声称 repository 一定不是 submodule。

## Status contract

`snapshot` 是一次有界 query，不是 live state。返回：

- `GitHead::Branch`、`Detached` 或 `Unborn`；
- branch upstream 与 ahead/behind；
- 每个 path 独立的 index/worktree `GitChangeStatus`；
- rename/copy 的 `original_path`；
- porcelain submodule flags；
- untracked path。

tracked status 使用 NUL-delimited porcelain-v2，Unix 上 path bytes 可保留为非 UTF-8 `PathBuf`。
branch、remote URL 和 commit subject 当前要求 UTF-8。Snapshot 按 Git 输出顺序返回；当前实现不再
额外排序，因为 rename record 的相邻 NUL payload 由 parser 顺序消费。

`snapshot` 使用 `GIT_OPTIONAL_LOCKS=0`。它是 observation，不获得阻止并发 Git mutation 的锁；
调用方不能把一次 snapshot 当作后续 mutation 的 compare-and-swap 前提。App Server service
未来必须使用 revision/invalidation 重新确认状态。

## Patch contract

`GitPatchRequest` 用 `GitPatchExecution` 和 `GitPatchDirection` 避免 `apply(false, true)` 一类不透明
callsite：

| Request | Git invocation | Working tree |
| --- | --- | --- |
| `Check + Forward` | `git apply --recount --check -` | 不修改 |
| `Check + Reverse` | `git apply --recount --check -R -` | 不修改 |
| `Apply + Forward` | `git apply --recount --3way -` | 可能修改并产生 conflict |
| `Apply + Reverse` | `git apply --recount --3way -R -` | 可能修改并产生 conflict |

成功 check 返回 `Applicable`，成功 apply 返回 `Applied`。当 Git 明确报告三方应用已经写入
conflict 时返回 `AppliedWithConflicts`；其他 nonzero exit 返回 `Rejected`。后两者都表示进程已经
启动并完成，调用方必须根据 disposition 判断 working tree 是否可能改变；spawn、stdin、timeout
或 output limit failure 返回 `GitError`。`referenced_paths` 来自 diff header；check 不会把这些
path 错称为 `applied_paths`。Diagnostics path 是对 Git 文本输出的 best-effort 分类，
`exit_code` 与 disposition 比分类集合 authoritative。

本实现不会像 Codex reverse helper 一样预先 stage existing path，因为前端 Git service 不应让
“反向应用 patch”隐式改变 index。若未来需要该 workflow，必须建模成显式 operation。

## Process、失败与取消

private `GitCommandProfile` 固定三类执行：

| Profile | Timeout | Optional locks | fsmonitor |
| --- | ---: | --- | --- |
| Query | query timeout | disabled | explicit safe override |
| Configuration probe | query timeout | disabled | 不覆盖被探测值 |
| Mutation | mutation timeout | Git 默认 | disabled |

所有 profile 禁用 configured hooks、terminal prompts 和 color。stdout/stderr reader 在保留上限
后继续 drain，避免 child 因 pipe backpressure 卡死；最终以 `OutputLimitExceeded` 失败，不返回
截断后可能被误解析的数据。

`GitError` 保留以下边界：

- `InvalidConfiguration` / `InvalidStartPath`：spawn 前拒绝；
- `NotAWorkingTree`：`rev-parse` 明确报告不在 working tree；
- `Io` / `Runtime`：spawn、pipe、wait 或 Tokio task failure；
- `TimedOut`：child 已启动，crate 发出 kill 并等待退出；
- `OutputLimitExceeded`：child 已完成或已被完整 drain，但结果不用于 domain parsing；
- `CommandFailed`：strict query 的 nonzero Git exit；
- `InvalidOutput`：Git success output 不满足 parser contract。

当前 API 没有调用方 cancellation token。Timeout 是唯一中断机制；未来 service cancellation 必须
进入 `GitClient` 的 process lifecycle，不能只在 App Server 丢弃 future。

## Integration boundary

Current：

```text
host
└─ zeta-git
   └─ system git
```

Proposed，但尚未实现：

```text
Desktop
  ↕ app-server-protocol Git commands/snapshots/events
App Server Git service
  ├─ repository registry + revision
  ├─ zeta-file-watcher invalidation hints
  ├─ operation serialization/progress/cancellation
  └─ zeta-git
      └─ system git
```

未来 App Server service 拥有 live repository lifecycle；`zeta-file-watcher` 只提供 invalidation
hint；protocol 拥有 wire DTO；Desktop 只展示状态和发送 intent；agent Git Tool 还必须经过
policy/approval。它们都不能复制本 crate 的 command/parsing 实现。

## 测试与修改

实现测试全部位于独立 sibling 文件：

- `client_tests.rs`：system Git runner 与 limits；
- `repository_tests.rs`：nested start、non-repository、linked worktree；
- `status_tests.rs`：index/worktree/untracked、rename 与 unmerged/unborn parser；
- `info_tests.rs`：branch、remote fetch/push URL、history limit；
- `patch_tests.rs`：quoted paths、check/apply、unapplied rejection 与 three-way conflict；
- `fsmonitor_tests.rs`：NUL config value 与 boolean spelling。

正常 workspace 状态下运行：

```bash
cargo test --manifest-path zeta-rs/Cargo.toml -p zeta-git
cargo clippy --manifest-path zeta-rs/Cargo.toml -p zeta-git \
  --all-targets --no-deps -- -D warnings
bazel test //zeta-rs/git:git-unit-tests
```

修改 `GitCommandProfile`、env/config override、timeout 或 capture 时同步检查所有 command family；
修改 porcelain parser 时增加 raw NUL fixture 和 live repository test；修改 patch diagnostics 时
同时保留 disposition/exit 的 authoritative 边界；新增 public operation 时更新本 README 的
Current capability 和 failure contract。

## 当前限制与扩展点

当前限制：

- 尚无 stage/unstage/discard/commit/branch mutation/worktree mutation/fetch/pull/push；
- 尚无 live cache、watch、operation queue、progress、caller cancellation 或 protocol DTO；
- 不支持 bare repository，`open_repository` 要求 working tree；
- repository discovery 依赖支持 `rev-parse --path-format=absolute` 的 Git；
- patch diagnostics parser 是 best effort，不承诺复现所有 Git 版本的自然语言；
- 未移植 Codex internal baseline，因为 resettable internal directory 不是用户 repository SCM；
- remote URL 当前保留 Git 配置原值，尚无 canonical repository identity API。

扩展顺序应优先保持 ownership：

1. 在本 crate 增加明确 typed operation 与 parser；
2. 为 mutation 定义 index/worktree side effect、failure 和 cancellation；
3. 在 App Server service 中增加 registry、revision、watch invalidation 与 operation manager；
4. 最后增加 protocol/desktop/agent adapter。

只有出现多个独立底层 Git owner 且有稳定共享 primitive 时，才重新评估 `zeta-git-utils`；当前不应
新增该 crate。
