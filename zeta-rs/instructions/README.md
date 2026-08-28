# `zeta-instructions`

> 本 README 拥有 Zeta 原生 Instruction artifact 的当前实现契约。跨系统对象划分、`.zeta`
> 命名空间与外部导入边界由
> [`docs/agent-customizations.md`](../../docs/agent-customizations.md) 维护。

`zeta-instructions` 对一个 Workspace 的 `.zeta/instructions/*.md` 执行有界、非递归发现，校验
YAML frontmatter、三态加载策略和 UTF-8 Markdown 正文，并发布不可变 catalog snapshot。它不解析
Codex/Claude 格式，不组装模型请求，也不拥有 watcher、Workspace trust 或 UI。

## 快速理解

| 文件状态 | 结果 |
| --- | --- |
| `.zeta/instructions` 不存在 | 空 catalog，无错误 |
| 合法 `.md` | 进入确定性 catalog |
| 单文件格式错误 | 产生隔离 diagnostic，其他文件继续 |
| `load: global` | 可由 runtime 注入后续模型调用 |
| `load: contextual` / `on-demand` | 保留为类型化 artifact，等待显式上下文选择 |

## 边界与公共契约

`InstructionCatalog::discover` 固定原生 Workspace 相对路径；`refresh` 只在 entries 或 diagnostics
变化时推进 generation。`InstructionCatalogSnapshot::global_content` 只渲染 `Global` 条目，并带
artifact name 与相对路径 provenance。

frontmatter 必须显式声明：

```yaml
---
name: rust-style
load: contextual
patterns:
  - "**/*.rs"
---
```

`load` 只接受 `global`、`contextual`、`on-demand`。只有 contextual 可以且必须声明非空
`patterns`。文件名使用小写字母、数字和连字符；可选 `name` 存在时必须与文件名一致。

实现限制为 128 个直接条目、每个文件 32 KiB。source/entry symlink、目录、非 Markdown、非法
UTF-8、空正文和未知 frontmatter 字段均不会进入 catalog。

## 内部所有权与调用路径

| 文件 / symbol | 职责 |
| --- | --- |
| `catalog.rs::scan` | 固定路径、entry limit、排序与隔离诊断 |
| `catalog.rs::load_entry` | metadata、类型、大小、UTF-8、frontmatter 和正文校验 |
| `catalog.rs::load_policy` | 三态加载策略的不变量 |
| `model.rs::InstructionCatalogSnapshot` | immutable entries/diagnostics 与 Global 渲染 |

```text
InstructionCatalog::discover / refresh
  → scan
  → load_entry
  → load_policy
  → InstructionCatalogSnapshot
```

若本 crate 开始扫描 `AGENTS.md`、`.codex`、`.claude`，执行 glob matching，读取当前 editor 状态，
或直接修改 `ModelRequest`，表示原生 authority 与 compatibility/runtime 边界已经漂移。

## 验证与当前限制

```bash
cargo test -p zeta-instructions
cargo clippy -p zeta-instructions --all-targets --no-deps -- -D warnings
```

当前已实现原生 Workspace 发现、格式校验、不可变 snapshot 与 Global 内容渲染；App Server 的
`WorkspaceCustomizations` 在 Workspace 激活时发现 catalog，由 filesystem invalidation refresh，并在
下一次 model invocation 通过 `HarnessContextProvider` 提供 Global 内容。
当前限制是 contextual pattern matching、显式 on-demand selection、user/built-in/Plugin source
composition，以及 catalog/diagnostic list API 尚未实现。
