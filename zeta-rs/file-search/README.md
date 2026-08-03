# `zeta-file-search`

> 本 README 是 workspace 文件路径 fuzzy search 的实现契约。TUI `@file` 交互由
> [`zeta-code/tui/README.md`](../../zeta-code/tui/README.md) 维护。模型侧的文件内容搜索通过
> [`zeta-rs/search/README.md`](../search/README.md) 完成；可执行文件的
> discovery 与冻结边界见 [`zeta-rs/shell-command/README.md`](../shell-command/README.md)。

`zeta-file-search` 只拥有 workspace-relative 文件路径的后台索引、增量 fuzzy matching 和独立
CLI。它不读取候选文件内容，不注册模型 Tool，也不拥有 TUI popup/token 状态。

## 文件与职责

```text
src/
├── lib.rs                    # PathSearchHandle、worker、snapshot 与 public exports
├── file_search_tests.rs      # 增量 query、ignore、排序和高亮索引
├── main.rs                   # zeta-file-search 的薄进程入口与 exit status
├── cli.rs                    # 参数、snapshot wait 与 stdout/stderr 输出
└── cli_tests.rs              # CLI 参数、JSON 与 plain-text output
```

## 公共契约

`PathSearchHandle::start(root, options)` 验证 root 是目录，返回后台搜索 handle 和
`Receiver<PathSearchSnapshot>`：

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

## `zeta-file-search` CLI

独立 binary 是 `PathSearchHandle` 的开发者/脚本入口，不是 TUI 启动的子进程：

```bash
cargo run --manifest-path Cargo.toml -p zeta-file-search -- src -C .
cargo run --manifest-path Cargo.toml -p zeta-file-search -- \
  --json --compute-indices --limit 20 mention -C zeta-rs
```

| 参数 | 语义 |
| --- | --- |
| `[PATTERN]` | fuzzy pattern；省略时列出 workspace 文件 |
| `-C, --cwd <DIR>` | 搜索 root；默认当前目录 |
| `-l, --limit <N>` | 输出上限；默认 64 |
| `--threads <N>` | walker 和 Nucleo worker 数；默认 2 |
| `--json` | 每个 match 输出一行 JSON |
| `--compute-indices` | JSON 包含 indices；TTY plain output 对命中字符加粗 |

CLI 等待当前 `query_revision` 的 `search_complete` snapshot 后输出，因此不会显示中间结果。结果被
limit 截断时，warning 写入 stderr；JSON match 仍写入 stdout，方便逐行消费。省略 pattern 时仍
通过 `PathSearchHandle` 列出文件，不回退执行 `ls` 或其他 shell 命令。

## 内部接口地图

| Symbol | 职责 | 不承担 |
| --- | --- | --- |
| `SearchInner` | 搜索 root、worker 配置、shutdown 与进度共享状态 | popup/query 生命周期 |
| `walker_worker` | 遍历并向 Nucleo 注入 relative file path | 读取文件内容、排序结果 |
| `matcher_worker` | 合并 query/walker/notify signal，驱动增量 tick | UI stale-result policy |
| `build_snapshot` | 生成有界、稳定排序且带高亮索引的 immutable snapshot | 发送 UI event |
| `cli::execute` | 等待 final snapshot 并选择 stdout/stderr 编码 | 扫描或 fuzzy matching |

如果该 crate 开始读取候选文件内容、实现模型 Tool binding，或保存 TUI popup state，说明 ownership
已经漂移；这些职责分别属于 `zeta-search`、Tool registry 和 `zeta-tui`。

## 失败与取消

- root 不存在或不是目录：`PathSearchHandle::start` 返回 `std::io::Error`，不启动 worker；
- CLI 参数非法、root 不可用或 worker 在 completion 前退出：binary 输出带
  `zeta-file-search:` 前缀的错误并返回非零状态；
- 单个 walker entry 错误或非 UTF-8 path：跳过该 entry，搜索继续；
- snapshot receiver 被丢弃：matcher 停止发送并退出；
- handle 被丢弃：matcher 立即收到 shutdown，walker 在下一次 entry callback 停止。

## 验证

```bash
cargo test --manifest-path Cargo.toml -p zeta-file-search
cargo clippy --manifest-path Cargo.toml -p zeta-file-search --all-targets --no-deps -- -D warnings
cargo run --manifest-path Cargo.toml -p zeta-file-search -- --help
bazel test //zeta-rs/file-search:file-search-unit-tests
bazel build //zeta-rs/file-search:zeta-file-search
```

修改 Nucleo 配置、ignore 规则、排序或 snapshot completion 语义时，必须同步检查
`file_search_tests.rs`、TUI file-search manager、mention renderer 和两层文档。
