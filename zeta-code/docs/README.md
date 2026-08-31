# Zeta Code 文档

> 本目录只保存 `zeta code` 产品宿主和 TUI 专属的系统文档。共享协议、后端、权限、配置和产品线边界仍由仓库根 [`docs/`](../../docs/README.md) 拥有；crate 实现细节由相邻 `README.md` 拥有。

## 快速理解

`zeta-code` 的文档按产品宿主、TUI 架构和交互视觉三类分开。已完成的迁移计划和历史讨论不保留为长期入口，其有效结论已经并入下面的当前文档。

| 想了解或修改什么 | 先读 | 再读 |
| --- | --- | --- |
| CLI 命令、输出、退出码和 App Server 接线 | [产品与 CLI 架构](architecture.md) | [`zeta-code` README](../README.md) |
| TUI 状态、事件、布局、生命周期和功能边界 | [TUI 架构](tui.md) | [TUI crate README](../tui/README.md) |
| 键盘、鼠标、焦点、选择和颜色语义 | [TUI 交互契约](tui-interaction.md) | [TUI 主题实现](../tui/README.md) |

共享系统只在根 `docs/` 维护。修改 App Server 契约时读 [App Server API](../../docs/zeta-app-server-api.md)，修改跨端快捷键时读 [三端快捷键系统](../../docs/keybindings.md)，判断产品归属时读 [产品线与宿主边界](../../docs/product-lines.md)。
