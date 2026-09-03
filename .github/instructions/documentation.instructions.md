---
description: Zeta task-time rules for Markdown documentation ownership, status, indexing, and verification.
applyTo: "**/*.md"
---

# Markdown 文档规则

- 修改 Markdown 前完整阅读 [`docs/documentation-guidelines.md`](../../docs/documentation-guidelines.md)，并先确定文档的长期 owner。
- 新增或删除根 `docs/` 下的跨产品文档时同步更新 [`docs/README.md`](../../docs/README.md)；产品专属文档更新对应项目的文档索引。
- 当前实现、当前限制和计划设计必须明确分开。实现状态以源码、协议和测试为准，不能只依据另一份本地文档。
- 阶段计划结束或失去独立价值后，把仍有效的契约、限制和待办移入长期 owner，删除计划及全部引用。
- 提交前检查相对链接、文件路径、类型名、命令和状态说明，确保删除或移动文档后没有悬空引用。
