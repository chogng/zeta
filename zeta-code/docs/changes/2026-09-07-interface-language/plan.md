# 界面语言设置：计划

规格：[spec.md](spec.md) v1。代码基线：`7ee335295`，工作区开始时无未提交差异。

| 步骤 | AC / INV | 实现与接入位置 | 验证命令 | 状态 |
| --- | --- | --- | --- | --- |
| 1 | AC-2、AC-3、AC-4、INV-1 | `tui/src/nls.rs`、`config/settings.rs`：有界类型、文案接口、完整设置读写 | `just test zeta-tui --lib config::` | 已实现 |
| 2 | AC-1、AC-2、INV-2 | `tui/src/config/editor.rs`、`app/state.rs`：删除摘要，增加语言行和统一输入动作 | `just test zeta-tui --lib language` | 已实现 |
| 3 | AC-3、AC-4、INV-1 | 请求完成链、启动配置与持久化集成覆盖 | `just test zeta-cli --test tui_real_scenarios actual_tui_switches_language_and_persists_it -- --exact` | 已实现 |
| 4 | 全部 | 更新现行规格、能力、开发指南和验收记录 | 文档链接检查；`just check zeta-tui` | 已实现 |

下一步：无；本次规格已经完成。
