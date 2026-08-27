`../vscode` and `../codex` and `../zed` and `../warp` and `../pi`, `../marketplace`, `../deepseek-harness`

# Zeta Agent Instructions

Before modifying this repository:

1. Read [`.github/copilot-instructions.md`](.github/copilot-instructions.md) completely for repository ownership, dependency direction, workflow, and scoped-instruction routing.
2. Read every file under [`.github/instructions`](.github/instructions) whose `applyTo` pattern matches any target file. Scoped instructions add to the repository instructions and cannot override a higher-level ownership or safety rule.
3. Follow the nearest additional `AGENTS.md` when a subtree provides one. Keep detailed rules in their canonical scoped instruction or architecture document rather than copying them into this entry file.
- Read [`.github/rust.instructions.md`](.github/rust.instructions.md) when edit the rust files.

# Communication

- Lead with the conclusion and use surrounding prose only for important boundaries or caveats.
- When comparing responsibilities, capabilities, implementation status, or design options, prefer a compact conclusion-oriented table when it makes the distinction clearer.
- Use `✅` and `❌` only for genuinely binary judgments. Use explicit labels such as `部分具备`, `尚未完成`, `协调`, or `委托` for nuanced states.

- 禁止往app/src中写入代码
- 回复时请说人话, 避免抽象语言描述
- 禁止 native, projection name
- crate 主要负责能力和依赖隔离
- 禁止兜底写法，过渡设计，过渡思考
- 当你思考项目架构时，请考虑从长期架构的终极形态去设计，而不是基于当前架构的优化方向
- 使用playwright测试web and electron-ui 以及electron
- 写 Readme.md 描述 crate 职责时，请用不超过3句话来完整描述，尽可能分点

