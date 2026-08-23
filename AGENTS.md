`../vscode` and `../codex` and `../zed` and `../warp` and `../pi`, `../marketplace`, `../deepseek-harness`

# Zeta Agent Instructions

Before modifying this repository:

1. Read [`.github/copilot-instructions.md`](.github/copilot-instructions.md) completely for repository ownership, dependency direction, workflow, and scoped-instruction routing.
2. Read every file under [`.github/instructions`](.github/instructions) whose `applyTo` pattern matches any target file. Scoped instructions add to the repository instructions and cannot override a higher-level ownership or safety rule.
3. Follow the nearest additional `AGENTS.md` when a subtree provides one. Keep detailed rules in their canonical scoped instruction or architecture document rather than copying them into this entry file.

# Communication

- Lead with the conclusion and use surrounding prose only for important boundaries or caveats.
- When comparing responsibilities, capabilities, implementation status, or design options, prefer a compact conclusion-oriented table when it makes the distinction clearer.
- Use `✅` and `❌` only for genuinely binary judgments. Use explicit labels such as `部分具备`, `尚未完成`, `协调`, or `委托` for nuanced states.
