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

* 带页签的列表中，`Tab` / `Shift+Tab` 切换页签时，焦点必须同步移到页签栏，不能留在列表或搜索框；即使只有一个页签也要移动焦点。页签栏内用左右键切页，上下键按页签栏、搜索框（若有）、列表的视觉顺序移动焦点；列表内部上下键仍逐项移动，到边界才离开列表。

* 行首用于放置 `>`、状态圆点等符号的区域统一称为“状态标识列”，简称“标识列”；标识列及其分隔留白之外才是内容区。标签、正文、输入框（包括边框）都从内容区开始，不能占用标识列；没有符号时标识列仍保留，不能让内容随状态左右移动。组件必须复用所在面板已有的标识列，不能在内容区内再预留一列或叠加缩进；布局、命中测试和渲染测试使用同一边界。涉及此边界的变更，必须通过完整面板 snapshot 和 buffer 列位置断言验证符号在标识列、内容在内容区，不能只测试组件内部的相对对齐。

* 修改 TUI 布局、可见文案、焦点标识或临时状态时，必须使用 [zeta-code-snapshot-testing](../../.agents/skills/zeta-code-snapshot-testing/SKILL.md)，更新或补充受影响状态的终端文本 snapshot；替换界面时同步替换对应覆盖，不能只删除旧基线。行为断言验证操作，文本 snapshot 验证完整可见内容和布局，二者不能相互替代。颜色、修饰符和动画等文本快照无法表达的反馈，必须另用实际渲染 buffer 的属性断言；动画注入固定时刻，不能靠 sleep 等待取样。

* 固定高度面板需要页签时，直接复用通用 `TabList` 的状态、绘制、命中和页签焦点按键；能力代码只提供页签标签与业务身份，并根据 `TabList` 返回的切页或进入正文结果执行业务动作。嵌入内容不保留未绘制的内部页签，只向父面板报告焦点到达边界；父面板不能检查子组件内部索引来猜边界，避免出现不可见焦点和同类面板交互漂移。
* 设计 StatusLine 数据需求时，先确认同一数据是否服务其他能力。Git 状态必须持续跟随 ChangeTurn；关闭 StatusLine 的 Git branch 或 Git changes 只能停止对应显示与 StatusLine 专属的额外计算，不能停止基础 Git 状态跟随。
* Status 只展示状态和证据，不拥有功能启停；需要跨启动保留的 TUI 诊断开关由 Config 写入 `[tui]`，运行层按配置管理诊断会话，不能把会话身份或采样结果写回配置。
