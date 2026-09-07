# Zeta Code 文档

## TUI 功能完整性梳理

先读目标和功能要求；判断当前完成情况时看验收记录。下面四份是本次梳理的正文，不是模板。

| 阅读顺序 | 文档 | 回答的问题 |
| --- | --- | --- |
| 1 | [intent.md · 目标](changes/tui-completeness/intent.md) | 用户最终应该能完成什么？ |
| 2 | [spec.md · 功能要求](changes/tui-completeness/spec.md) | 每个流程应该怎样工作，失败和返回怎么办？ |
| 3 | [plan.md · 检查与补齐计划](changes/tui-completeness/plan.md) | 先检查什么，缺口怎样补？ |
| 4 | [verification.md · 验收记录](changes/tui-completeness/verification.md) | 实际确认了什么，还缺什么证据？ |

当前已登记 27 条要求；产品实测尚未执行。Markdown、终端组合、未发送内容恢复和宠物点击播放的完成边界仍待确定。

## 详细参考

| 内容 | 文档 |
| --- | --- |
| 当前能力与已知限制 | [功能现状](capabilities.md) |
| 按键、返回与焦点 | [交互规格](spec/interaction.md) |
| 页面位置与输出示例 | [布局规格](spec/layout.md) |
| 字符、颜色与主题效果 | [样式规格](spec/styles.md) |
| 资源格式与动作要求 | [Welcome 宠物规格](spec/welcome-pet.md) |
| 状态、请求与模块分工 | [TUI 架构](design/tui.md) · [CLI 架构](design/cli.md) · [进程资源采样](design/process-resources.md) |
| 代码入口、配置格式与测试 | [TUI 开发指南](../tui/README.md) |

[正文绘制重构](spec/transcript-rendering.md)已有新的实现，尚待逐项验收；[内存诊断](spec/memory-diagnostics.md)已接入共享后端，完整验收见[记录](../../zeta-rs/docs/changes/memory-diagnostics/verification.md)。其他规格中尚未接入的要求在对应段落标明。

新功能沿用[开发流程与模板](../../docs/development-workflow.md)，在 `changes/<工作名>/` 保存自己的四份记录。本次梳理保留到范围和验收完成，不因正文已经写完而删除。

共享协议与配置见[工程文档](../../docs/README.md)；跨端键位见[快捷键系统](../../docs/keybindings.md)。
