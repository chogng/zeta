# `zeta-agents`

> 本 README 拥有 Zeta 原生 Agent definition artifact 的当前实现契约。跨系统对象划分、`.zeta`
> 命名空间与运行时边界由
> [`docs/agent-customizations.md`](../../docs/agent-customizations.md) 维护；Thread execution identity
> 与 multi-agent gate 由 [`docs/core-multi-agent.md`](../../docs/core-multi-agent.md) 维护。

`zeta-agents` 对一个 Workspace 的 `.zeta/agents/*.md` 执行有界、非递归发现，校验 definition
frontmatter 和 role instructions，并发布不可变 catalog snapshot。它不创建 Thread、不调用模型、
不解析外部 Agent 格式，也不把 tool/model references 解释成权限或可用性保证。

## 快速理解

| 文件状态 | 结果 |
| --- | --- |
| `.zeta/agents` 不存在 | 空 catalog，无错误 |
| 合法 `.md` | 进入确定性 definition catalog |
| 单文件格式错误 | 产生隔离 diagnostic，其他文件继续 |
| definition 被发现 | 只证明声明有效，不表示 Agent 已启动或引用可用 |

## 边界与格式

```yaml
---
name: reviewer
description: Reviews code changes for correctness and regressions.
model: openai/gpt-5
tools:
  - read_file
skills:
  - code-review
instructions:
  - rust-style
---

Review the requested change and report actionable findings.
```

`name` 必须与小写文件名一致；`description` 和非空 Markdown role body 必填。`model`、`tools`、
`skills`、`instructions` 是经过基本语法校验的引用声明，最终解析继续由对应 authority 完成。
重复或非法引用使单个 definition 失效，不能通过发现阶段授予工具或模型能力。

实现限制为 64 个直接条目、每个文件 32 KiB。目录、symlink、非 Markdown、非法 UTF-8、未知
frontmatter 字段和空正文不会进入 catalog。

## 内部所有权与调用路径

| 文件 / symbol | 职责 |
| --- | --- |
| `catalog.rs::scan` | 固定 `.zeta/agents` 路径、entry limit、排序与隔离诊断 |
| `catalog.rs::load_entry` | 文件、frontmatter、正文与引用校验 |
| `catalog.rs::validate_references` | reference syntax 与重复拒绝 |
| `model.rs::AgentDefinitionCatalogSnapshot` | immutable entries/diagnostics generation |

```text
AgentDefinitionCatalog::discover / refresh
  → scan
  → load_entry
  → validate_references
  → AgentDefinitionCatalogSnapshot
```

若本 crate 开始创建 Thread、决定模型、解析 Tool registry、授予权限、扫描 `.codex/.claude` 或
管理 multi-agent lifecycle，表示 definition authority 已经越界。

## 验证与当前限制

```bash
cargo test -p zeta-agents
cargo clippy -p zeta-agents --all-targets --no-deps -- -D warnings
```

当前已实现 Workspace 原生 definition 发现、格式校验、不可变 snapshot 与 refresh；App Server
的 `WorkspaceCustomizations` 在 Workspace 激活时拥有 catalog，并响应 filesystem invalidation。
当前限制是 App Server list API、definition selection、跨 authority 引用解析和 multi-agent
runtime consumption 尚未实现。
