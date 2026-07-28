# `zeta-file-search`

> 本 README 是该 crate 的实现契约。模型可见本地工具的跨 crate 架构由
> [`docs/tools.md`](../../docs/tools.md) 维护；TUI `@file` 交互由
> [`zeta-rs/tui/README.md`](../tui/README.md) 维护。

`zeta-file-search` 拥有两条彼此独立的只读搜索路径：

| 能力 | Public entry point | 调用方 | 搜索对象 |
| --- | --- | --- | --- |
| 模型文本搜索 | `FileSearchTool` | Tool registry / Core | UTF-8 文件内容 |
| 交互式路径搜索 | `PathSearchHandle` | TUI 等 presentation adapter | workspace-relative 文件路径 |

路径搜索不是一个新的模型 Tool；它不会读取候选文件内容，也不会改变 `file-search` Tool 的
schema、binding、sandbox 或结果语义。

## 文件与职责

```text
src/
├── lib.rs                    # FileSearchTool 与显式 public exports
├── file_search_tests.rs      # 模型文本搜索 Tool contract
├── path_search.rs            # 后台 walker、Nucleo handle 与 snapshot contract
└── path_search_tests.rs      # 增量 query、ignore、排序和高亮索引
```

## Public contract

### `FileSearchTool`

`FileSearchTool::new` 固定 `ToolEnvironmentId + WorkspaceRoot + FileSearchLimits`。执行时验证
environment、冻结 binding digest、结构化参数和 cancellation，然后递归读取 UTF-8 文本。它跳过
symlink、二进制和超限文件，并以有界 JSON Tool output 返回行号、byte column 和截断状态。

### `PathSearchHandle`

`PathSearchHandle::start(root, options)` 验证 root 是目录，返回后台搜索 handle 和
`Receiver<PathSearchSnapshot>`。handle 启动：

```text
ignore::WalkBuilder worker
  └─ workspace-relative file path → Nucleo injector

Nucleo matcher worker
  ├─ QueryChanged → incremental pattern reparse
  ├─ injected path notification → tick
  └─ PathSearchSnapshot → caller-owned receiver
```

`PathSearchHandle::update_query` 只更新 matcher pattern，不重启目录遍历。丢弃 handle 会设置
shutdown 并唤醒 matcher；walker 在遍历 callback 中观察 shutdown。worker 是 detached thread，
调用方不等待 join。

`PathSearchSnapshot` 携带单调递增的 `query_revision`、query、按 score 降序且按 path 升序打破
平局的前 N 项、匹配字符索引、扫描文件数以及 scan/search completion 状态。调用方必须同时检查
revision 和 query，避免输入从 A 变成 B 再回到 A 时接受第一次 A 的过期结果。

路径 walker：

- 使用 Git 作用域内的 `.gitignore`/ignore 语义；
- 不跟随 symlink；
- 显式跳过 `.git`、`.zeta`、`node_modules` 与 `target`；
- 跳过非 UTF-8 relative path；
- 只注入普通文件，不返回目录。

## 内部接口地图

| Symbol | 职责 | 不承担 |
| --- | --- | --- |
| `SearchState` | 模型文本搜索的递归读取、内容匹配和 limit/cancellation checkpoint | fuzzy path ranking |
| `SearchInner` | 路径搜索 root、worker 配置、shutdown 与进度共享状态 | popup/query 生命周期 |
| `walker_worker` | 遍历并向 Nucleo 注入 relative file path | 读取文件内容、排序结果 |
| `matcher_worker` | 合并 query/walker/notify signal，驱动增量 tick | UI stale-result policy |
| `build_snapshot` | 生成有界、稳定排序且带高亮索引的 immutable snapshot | 发送 UI event |

如果路径搜索开始读取候选文件内容，或 `FileSearchTool` 开始依赖交互 handle/popup 状态，说明两条
职责边界发生了漂移。

## Failure 与 cancellation

- root 不存在或不是目录：`PathSearchHandle::start` 返回 `std::io::Error`，不启动 worker；
- 单个 walker entry 错误或非 UTF-8 path：跳过该 entry，搜索继续；
- snapshot receiver 被丢弃：matcher 停止发送并退出；
- handle 被丢弃：matcher 立即收到 shutdown，walker 在下一次 entry callback 停止；
- 模型 Tool cancellation 和错误语义仍由 `FileSearchTool`/`SearchFailure` 独立拥有。

## 验证

```bash
cargo test --manifest-path zeta-rs/Cargo.toml -p zeta-file-search
cargo clippy --manifest-path zeta-rs/Cargo.toml -p zeta-file-search --no-deps -- -D warnings
```

修改 Nucleo 配置、ignore 规则、排序或 snapshot completion 语义时，必须同步检查
`path_search_tests.rs`、TUI file-search manager、mention renderer 和两层文档。
