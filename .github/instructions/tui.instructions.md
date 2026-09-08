---
description: Zeta CLI and Ratatui product ownership, architecture, interaction, and validation boundaries.
applyTo: "zeta-code/**"
---

# Zeta Code CLI/TUI Guidelines

Do not add feature overviews, UI behavior specifications, design notes, change records, plans, or verification reports under `zeta-code/docs`; keep implementation guidance and targeted test commands with the owning crate, and keep cross-client methods, parameters, results, notifications, errors, and machine-output contracts in their owning API documents. See [`zeta-code/README.md`](../../zeta-code/README.md) for the product entry point. A specification or existing test file is not evidence that behavior passed acceptance.

`zeta-code` owns `zeta-cli`, `zeta-tui`, raw-mode lifecycle, Ratatui interaction, and CLI product composition. Do not move this product presentation or lifecycle into `zeta-rs`; shared backend semantics must first form a backend-neutral contract with a real non-TUI consumer.

Keep one writer for each product state, render from explicit state, isolate side effects, reject stale asynchronous results by request/revision identity, and keep host adapters narrow. Feature behavior belongs in vertical feature owners rather than a global application switch.

Prefer command-line-observable tests for state, events, terminal output, timing, and lifecycle. Do not use screenshots or terminal pixel baselines as the primary pass/fail signal.

## Learnings

* 固定高度面板需要页签时，直接复用通用 `TabList` 的状态、绘制、命中和页签焦点按键；能力代码只提供页签标签与业务身份，并根据 `TabList` 返回的切页或进入正文结果执行业务动作。嵌入内容不保留未绘制的内部页签，只向父面板报告焦点到达边界；父面板不能检查子组件内部索引来猜边界，避免出现不可见焦点和同类面板交互漂移。
* 设计 StatusLine 数据需求时，先确认同一数据是否服务其他能力。Git 状态必须持续跟随 ChangeTurn；关闭 StatusLine 的 Git branch 或 Git changes 只能停止对应显示与 StatusLine 专属的额外计算，不能停止基础 Git 状态跟随。
* Status 只展示状态和证据，不拥有功能启停；需要跨启动保留的 TUI 诊断开关由 Config 写入 `[tui]`，运行层按配置管理诊断会话，不能把会话身份或采样结果写回配置。
