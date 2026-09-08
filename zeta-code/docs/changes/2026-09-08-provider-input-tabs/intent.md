# Provider 输入与禁用页签

继续 issue [#3](https://github.com/chogng/zeta/issues/3) 的部分实现，并完成 [#7](https://github.com/chogng/zeta/issues/7)。用户进入 OpenAI 后先选择字段，明确进入编辑、保存或取消，再主动刷新模型列表；被禁用的页签不能进入。

本次取代 [OpenAI 面板](../openai-panel/plan.md) 的“保存后自动编辑下一字段”要求，保留连接、凭据与模型目录的既有归属。

| 变化 | 长期规格 | 设计 |
| --- | --- | --- |
| 字段选择、保存取消、发现结果 | [供应商](../../spec/providers.md) | [TUI](../../design/tui.md) |
| 页签启用状态 | [交互](../../spec/interaction.md) | [TUI](../../design/tui.md) |

## 2026-09-08 · 通用输入控件

用户进一步确认：取消自动下移，并要求调用方直接复用输入框基座，不能在各页面重新实现 Enter/Esc 和输入模式。本次范围增加 `TextField`，由它持有编辑、提交等待、确认值和取消规则；Provider 页面只保留字段业务校验、保存与页签/字段导航。
