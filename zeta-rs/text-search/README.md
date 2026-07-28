# `zeta-text-search`

> 本 README 是模型可见文本内容搜索 Tool 的实现契约。跨 crate Tool 架构由
> [`docs/tools.md`](../../docs/tools.md) 维护。workspace 路径 fuzzy search 属于 sibling
> [`zeta-file-search`](../file-search/README.md)。

`zeta-text-search` 只提供只读的 `text-search` ToolExecutor。它在 workspace 边界内递归读取
UTF-8 文件内容并返回有界行匹配；不做文件名 fuzzy matching，不跟随 symlink，不调用 shell，
也不修改文件。

## 文件与职责

```text
src/
├── lib.rs                    # TextSearchTool、limits、validation 与内容扫描
└── text_search_tests.rs      # binding、内容匹配与结果上限
```

## Public contract

| Symbol | 职责 |
| --- | --- |
| `TextSearchTool` | 验证 frozen binding/environment 并执行一次同步内容搜索 |
| `TextSearchLimits` | 限制单文件 bytes、模型可见 matches 和扫描文件数 |
| `TextSearchError` | 表达 limit 配置或 Tool definition 构造失败 |

`TextSearchTool::new` 固定 `ToolEnvironmentId + WorkspaceRoot + TextSearchLimits`。`execute` 只接受：

```json
{
  "query": "non-empty text",
  "path": "relative/workspace/path",
  "case": "sensitive | insensitive"
}
```

执行顺序是：

```text
validate environment + frozen binding + cancellation
→ WorkspaceRoot::resolve_existing
→ SearchState::search
→ skip symlink / oversized / binary / non-UTF-8
→ bounded JSON ToolOutput
```

## 内部接口地图

| Symbol | 职责 | 不承担 |
| --- | --- | --- |
| `TextSearchInput` | deny-unknown-fields 参数 shape | workspace 解析 |
| `SearchState` | 递归读取、内容匹配、limit 与 cancellation checkpoint | 路径 fuzzy ranking |
| `text_search_definition` | authoritative Tool name、description 与 schema | runtime binding |
| `validate_invocation` | environment、definition digest 与预启动 cancellation | 执行搜索 |

`column_byte` 是 1-based byte offset，不是 Unicode character column。ASCII-insensitive 模式通过
ASCII lowercase 匹配；它不是完整 Unicode case folding。

## Failure 与 cancellation

- 空 query 或 schema 错误：返回模型可见 error output；
- environment/binding 不匹配或预启动 cancellation：返回 `NotStarted`；
- 遍历期间 cancellation：返回 `NotStarted`，不暴露部分匹配；
- I/O、workspace escape：返回模型可见 error output；
- 达到任一 limit：停止继续收集并返回 `truncated: true`。

## 验证

```bash
cargo test --manifest-path zeta-rs/Cargo.toml -p zeta-text-search
cargo clippy --manifest-path zeta-rs/Cargo.toml -p zeta-text-search --no-deps -- -D warnings
bazel test //zeta-rs/text-search:text-search-unit-tests
```
