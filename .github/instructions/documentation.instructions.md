---
description: Zeta task-time rules for Markdown documentation ownership, status, indexing, and verification.
applyTo: "**/*.md"
---

# Markdown 文档规则

- 修改 Markdown 前阅读[文档写作规范](../../docs/documentation-guidelines.md)。按读者任务确定内容和篇幅，沿用已有文档位置。
- 新增或删除根 `docs/` 下的跨产品文档时同步更新 [`docs/README.md`](../../docs/README.md)；产品专属文档更新对应项目的文档索引。
- 当前实现、当前限制和计划设计必须明确分开。实现状态以源码、协议和测试为准，不能只依据另一份本地文档。
- 阶段计划结束或失去独立价值后，把仍有效的契约、限制和待办移入长期 owner，删除计划及全部引用。
- 提交前检查相对链接、文件路径、类型名、命令和状态说明，确保删除或移动文档后没有悬空引用。

## Learnings

* 围绕读者当前要完成的任务写文档，开头直接给出用途、操作或关键结论。删除重复导航、职责宣言、泛化提醒和源码清单，不强制套用章节、表格或流程图；只保留影响使用、实现或判断的具体信息。
