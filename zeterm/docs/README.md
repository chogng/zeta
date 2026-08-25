# zeterm 产品文档

> 状态：Current。本文是 `zeterm` 专属系统文档的导航和边界说明；跨产品线、共享后端和通用
> crate 契约分别由 [`product-lines.md`](../../docs/product-lines.md)、
> [`zeta-rs-architecture.md`](../../docs/zeta-rs-architecture.md) 和各 crate README 维护。

## 快速理解

`zeterm/docs` 只记录纯 Rust Desktop 产品的用户语义、跨 crate 组合、终端兼容性、输入与发布边界。
它不替代 `zeterm/README.md` 或各 presentation crate README，也不把产品状态机复制进文档。

| 你要理解什么 | 先读哪份文档 | canonical owner |
| --- | --- | --- |
| Agent 开发能力、机器反馈、人类观测、功能准入与 Thread/Composer 语义 | [`native-agent-console.md`](native-agent-console.md) | `zeterm` 产品宿主 + App Server contract |
| 主窗口信息架构、Agent Terminal 会话流、按需检查与响应式布局 | [`native-layout.md`](native-layout.md) | `zeterm` product presentation + `zeta-layout` geometry contract |
| Terminal Surface、PTY、grid 与兼容性 | [`native-terminal-ui.md`](native-terminal-ui.md) | `zeta-terminal` + `zeterm` terminal host |
| 键盘、IME、caret 与输入路由 | [`native-text-input.md`](native-text-input.md) | `zui` / `zeta-ui` / `zeta-winit` / `zeterm` adapter |
| 稳定产品命令身份、请求与注册式执行 | [`native-terminal-ui.md`](native-terminal-ui.md) + [`zeta-commands`](../commands/README.md) | `ZetermCommandId` + `CommandRequest` + `CommandRegistry` |
| UI scene 到 GPU 的依赖方向 | [`rendering-architecture.md`](rendering-architecture.md) | `zui` → `zeta-renderer` → `zeta-wgpu` |
| 通用 icon asset 与产品 icon catalog 的边界 | [`rendering-architecture.md`](rendering-architecture.md) + [`zeta-icon`](../icon/README.md) | `zeta-icon` contract + optional `zeta-icons` catalog |
| 通用 Pane geometry 与 resize contract | [`zeta-layout`](../layout/README.md) | `zeta-layout` + host state adapter |
| Composer state、input、routing、interaction 与 panel/list geometry | [`zeta-composer`](../composer/README.md) | `zeta-composer` + Native product/scene adapter |
| 通用 UI 脱离产品宿主的最小验证 | [`zui-demo`](../zui-demo/README.md) | `zui` / `zeta-ui` / `zeta-renderer` |
| Native UI 组件和宿主迁移边界 | [`ui-component-migration-plan.md`](ui-component-migration-plan.md) | zeterm-owned UI crates |
| 从旧 Native workspace 的迁移状态 | [`zeterm-app-migration-plan.md`](zeterm-app-migration-plan.md) | `zeterm/` + root Cargo workspace |
| Native 弃用与长期 owner | [`native-deprecation-plan.md`](native-deprecation-plan.md) | `zui` / `zeta-ui` / `zeterm` host boundary |
| 构建、签名和发布输入 | [`zeterm-release-graph.md`](zeterm-release-graph.md) | root Cargo/Bazel graph + `zeterm/packaging` |

## 文档边界

- `zeterm/docs` 解释产品行为、跨 crate 数据流、ownership、兼容性和阶段性迁移；
- `zeterm/README.md` 解释产品宿主当前源码路径、接入义务和 crate 组合；
- `zeterm/<crate>/README.md` 解释单个 crate 的 public contract、关键 private symbol、测试和修改影响；
- `docs/` 只保留跨产品、共享后端、通用 UI 规范和不属于某一产品宿主的系统文档。

如果产品行为和 crate 实现发生冲突，先修正拥有该行为的实现契约，再同步本目录的系统文档；不要在
`zeterm/src`、`zui` 或共享后端之间建立第二份权威状态。
