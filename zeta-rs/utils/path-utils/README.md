# zeta-utils-path

> 本 README 是本 crate 当前实现契约的 canonical owner。跨进程文件位置 identity 的契约由
> [`zeta-utils-path-uri`](../path-uri/README.md) 拥有；Session cwd 与文件系统系统语义仍由对应
> `docs/*.md` 文档拥有。

`zeta-utils-path` 只处理当前 host 文件系统上的路径比较、symlink 写入目标解析和原子替换。
它不定义远程文件身份、workspace 授权边界或 Session 恢复策略。

## 边界与公共契约

| API | 当前职责 | Failure semantics |
| --- | --- | --- |
| `normalize_for_path_comparison` | canonicalize existing path，并应用 WSL mount case 规则 | 路径不存在或无法 canonicalize 时返回 `io::Error` |
| `paths_match_after_normalization` | 比较两个可能具有 symlink/host alias 的路径 | 任一路径无法规范化时退回原始 `Path` 相等 |
| `normalize_for_native_workdir` | Windows 上移除 verbatim path 语法 | 不访问文件系统 |
| `resolve_symlink_write_paths` | 跟随相对或绝对 symlink chain | cycle/metadata failure 返回原始 `write_path`，且 `read_path = None` |
| `write_atomically` / `write_text_atomically` | 同目录临时文件写入、flush、rename、目录 sync | rename 前失败保留旧 destination；rename 后目录 sync 失败会在新内容已可见时返回错误 |

`paths_match_after_normalization` 适合 Resume cwd 或本地 session filter，但本 crate 不决定何时
恢复会话或提示用户。`write_atomically` 也不拥有上层配置 schema、revision 或 locking。

## 文件、内部所有权与调用关系

| 文件 / private symbol | Ownership |
| --- | --- |
| `comparison.rs::normalize_for_wsl_on` | WSL `/mnt/<drive>` ASCII case normalization |
| `comparison.rs::is_wsl_case_insensitive_path` | 精确识别 Windows drive mount，不把普通 Linux path 当成 case-insensitive |
| `environment.rs::is_wsl` | 环境变量与 `/proc/version` detection |
| `persistence.rs::unresolved` | symlink resolution 的 conservative fallback |
| `persistence.rs::sync_parent` | rename 后 durability checkpoint |

```text
paths_match_after_normalization
  → normalize_for_path_comparison
      → canonicalize
      → normalize_for_wsl_on

resolve_symlink_write_paths
  → symlink_metadata/read_link loop
  → resolved target | unresolved fallback

write_atomically
  → create parent
  → NamedTempFile + write + sync
  → persist(rename)
  → sync_parent
```

如果这里开始保存 Session 状态、解析 URI、实施 workspace containment，表示 ownership 已经漂移。

## 集成与测试

consumer 只应依赖本 crate 导出的函数，不应依赖 private module。需要跨 RPC 序列化的路径应使用
`zeta-utils-path-uri` 或所属 protocol 的 root-relative path contract。

```text
cargo test -p zeta-utils-path
bazel test //zeta-rs/utils/path-utils:path-utils-unit-tests
```

修改 WSL、symlink 或 atomic-write failure semantics 时必须同步更新 `path_utils_tests.rs` 和本
README。当前没有 consumer；首次接入时应在 consumer crate 增加领域级测试，而不只依赖本 crate
的 primitive tests。

## 当前限制与扩展点

- Current：只比较当前 host 上可 canonicalize 的真实路径；不存在路径只做原始相等比较。
- Current：atomic replace 不提供跨进程 locking，caller 必须自行拥有并发控制。
- Current：Windows 不尝试 sync directory handle。
- Extension point：Session cwd filter 可复用比较 API，但其产品语义不能进入本 crate。
