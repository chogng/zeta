---
description: Zeta CLI and Ratatui product ownership, architecture, interaction, and validation boundaries.
applyTo: "zeta-code/**"
---

# Zeta Code CLI/TUI Guidelines

Follow [`zeta-code/docs/tui.md`](../../zeta-code/docs/tui.md) for the product architecture, state ownership, event flow, current implementation, and validation requirements.

`zeta-code` owns `zeta-cli`, `zeta-tui`, raw-mode lifecycle, Ratatui interaction, and CLI product composition. Do not move this product presentation or lifecycle into `zeta-rs`; shared backend semantics must first form a backend-neutral contract with a real non-TUI consumer.

Keep one writer for each product state, render from explicit state, isolate side effects, reject stale asynchronous results by request/revision identity, and keep host adapters narrow. Feature behavior belongs in vertical feature owners rather than a global application switch.

Prefer command-line-observable tests for state, events, terminal output, timing, and lifecycle. Do not use screenshots or terminal pixel baselines as the primary pass/fail signal.

## Learnings

* 固定高度面板需要页签时，直接复用通用 `TabList` 的状态、绘制和交互样式；能力代码只提供页签标签与业务身份，不自行拼接方括号、选中标记或颜色，避免同类面板产生视觉和交互漂移。
