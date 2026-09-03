---
description: Zeta Rust API, module, import, documentation, ownership, and file-size conventions.
applyTo: "**/*.rs"
---

# Rust Coding Guidelines

- Newly added traits include doc comments explaining their role and how implementations use them.
- Avoid boolean and ambiguous `Option` parameters. Prefer enums, named methods, or newtypes.
- Default modules and implementation details to private and explicitly re-export the public crate API.
- Prefer one Rust import per line over brace-grouped imports.
- Use file-based module roots: `foo.rs` and `foo/bar.rs`. Do not introduce `foo/mod.rs` without an external constraint.
- Prefer production modules below 500 lines. Near 800 lines, add new responsibility in another module and move its tests and documentation with it.

For new test modules, use a separate sibling file and an explicit descriptive path:

```rust
#[cfg(test)]
#[path = "parser_tests.rs"]
mod tests;
```

## Learnings

* 不要按 `state` / `view` 机械拆分小型 Rust 组件。只有子模块拥有独立职责、生命周期或依赖边界时才拆分；否则让同名组件文件直接承载状态、行为和绘制逻辑，避免只含模块声明与重新导出的空壳文件。
* 合并或退场 Rust 模块后，必须同步删除旧模块声明、更新调用方、测试和文档，并在交付中明确列出旧路径对应的新归属；看到 IDE 旧标签页报错时，先确认文件是否已退场，再判断是否为编译问题。
