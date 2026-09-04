# Zeta Code 文档

> 本目录只保存 `zeta code` 产品宿主和 TUI 专属的系统文档。共享协议、后端、权限、配置和产品线边界仍由仓库根 [`docs/`](../../docs/README.md) 拥有；crate 实现细节由相邻 `README.md` 拥有。

## 快速理解

`zeta-code` 的文档按产品宿主、TUI 架构、界面部位、交互和视觉样式分开。已完成的迁移计划和历史讨论不保留为长期入口，其有效结论已经并入下面的当前文档。

| 想了解或修改什么 | 先读 | 再读 |
| --- | --- | --- |
| CLI 命令、输出、退出码和 App Server 接线 | [产品与 CLI 架构](architecture.md) | [`zeta-code` README](../README.md) |
| TUI 状态、事件、布局、生命周期和功能边界 | [TUI 架构](tui.md) | [TUI crate README](../tui/README.md) |
| 进程资源何时采样、统计含义和内存诊断边界 | [进程资源观测与内存诊断](process-resources.md) | [TUI 架构](tui.md) |
| 界面区域叫什么、位于哪里 | [界面部位词典](LAYOUT.md) | [TUI 架构](tui.md) |
| 键盘、鼠标、焦点和选择如何变化 | [TUI 交互契约](tui-interaction.md) | [TUI 样式](styles.md) |
| 输入提示、状态字符、边线和颜色如何显示 | [TUI 样式](styles.md) | [TUI 主题实现](../tui/README.md) |
| Welcome 终端 Logo 如何设计、生成和验收 | [终端 Logo 开发](logo.md) | [界面部位词典](LAYOUT.md) |

共享系统只在根 `docs/` 维护。修改 App Server 契约时读 [App Server API](../../docs/zeta-app-server-api.md)，修改跨端快捷键时读 [三端快捷键系统](../../docs/keybindings.md)，判断产品归属时读 [产品线与宿主边界](../../docs/product-lines.md)。
