---
description: Zeta documentation ownership, current/proposed status, cross-linking, and verification rules.
applyTo: "**/*.md"
---

# Documentation Guidelines

Follow [`docs/documentation-guidelines.md`](../../docs/documentation-guidelines.md). It is the canonical writing and information-architecture standard; this instruction is only the always-loaded entry for Markdown changes.

- Give each fact one detailed owner and link to it elsewhere. Do not copy implementation rules between `AGENTS.md`, scoped instructions, architecture documents, and README files.
- Keep current implementation, current limitations, proposed work, and potential direction explicitly separated.
- Architecture documents explain cross-component behavior, ownership, trust/durability boundaries, tradeoffs, and evolution. They do not inventory private helpers or reproduce source.
- Implementation READMEs explain exact contracts, execution paths, failure semantics, integration obligations, tests, modification impact, limitations, and the private symbols that carry important ownership.
- Durable documents describe the current system, not the chronology of commits, PRs, or a single refactor.
- Commands, paths, type names, status claims, and links must be verifiable. Do not claim a check passed unless it ran successfully.
- When a rule is mechanical, enforce it in a formatter, linter, generator, or repository gate; documentation explains its meaning and non-mechanical boundary.

## Learnings

* 规范文档使用 VS Code 式的直接写法：优先写短而可执行的规则和最小示例；删除动机铺陈、重复边界、迁移叙事和不影响当前决策的解释。
