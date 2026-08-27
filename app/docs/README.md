# app 产品文档

> 状态：Current。本文是 `app` 专属系统文档的导航和边界说明；跨产品线、共享后端和通用
> crate 契约分别由 [`product-lines.md`](../../docs/product-lines.md)、
> [`zeta-rs-architecture.md`](../../docs/zeta-rs-architecture.md) 和各 crate README 维护。

## 快速理解

`app/docs` 只记录纯 Rust Desktop 产品的用户语义、跨 crate 组合、输入与发布边界。
它不替代 `app/README.md` 或各 presentation crate README，也不把产品状态机复制进文档。

| 你要理解什么 | 先读哪份文档 | canonical owner |
| --- | --- | --- |
| Agent 开发能力、机器反馈、人类观测、功能准入与 Thread/Composer 语义 | [`native-agent-console.md`](native-agent-console.md) | `app` 产品宿主 + App Server contract |
| 主窗口 Tab/Pane 层级、PaneInput 与响应式布局 | [`LAYOUT.md`](../LAYOUT.md) | `zeta-workbench` model/layout contract + `app` product presentation |
| 外部 AI CLI、Terminal Pane、PTY、grid 与兼容性 | [`TERMINAL.md`](../TERMINAL.md) | AI CLI adapter + `zeta-terminal` + `app` terminal host |
| 键盘、IME、caret 与输入路由 | [`native-text-input.md`](native-text-input.md) | `zui` / `zeta-ui-components` / `app` adapter |
| 稳定产品命令身份、请求与注册式执行 | [`zeta-commands`](../commands/README.md) | `AppCommandId` + `CommandRequest` + `CommandRegistry` |
| UI scene 到 GPU 的依赖方向 | [`rendering-architecture.md`](rendering-architecture.md) | `zui::ui` → `zui::render` contract → private `render/wgpu` |
| Native UI 编写、布局、样式与主题投影边界 | [`native-ui-authoring.md`](native-ui-authoring.md) | `zui` / `zeta-ui-components` / `app` host |
| 通用 application/window、renderer 与平台能力 | [`zui`](../zui/README.md) | 单一 `zui` crate，内部按 `app/window/input/ui/runtime/render/services` 能力目录隔离 |
| 通用 icon asset 与产品 icon catalog 的边界 | [`rendering-architecture.md`](rendering-architecture.md) + [`zui`](../zui/README.md) | `zui::Icon` contract + optional `zeta-icons` catalog |
| Workbench 模型、布局、外壳 UI 与 Pane binding | [`zeta-workbench`](../workbench/README.md) | 一个 Workbench crate；产品能力 crate 负责各自内容 runtime 与 scene |
| 可复用 UI 组件与样式边界 | [`zeta-ui-components`](../ui-components/README.md) | `zeta-ui-components` → `zui`；不包含 Workbench 布局或业务状态 |
| Workbench 导航与标题栏界面 | [`zeta-workbench`](../workbench/README.md) | `zeta-workbench` → `zeta-ui-components` / `zui` |
| Composer state、input、routing、interaction 与 panel/list geometry | [`zeta-composer`](../composer/README.md) | `zeta-composer` + Native product/scene adapter |
| 通用 UI 脱离产品宿主的最小验证 | [`zui-demo`](../zui-demo/README.md) | `zui` / `zeta-ui-components` |
| 从旧 Native workspace 的迁移状态 | [`app-migration-plan.md`](app-migration-plan.md) | `app/` + root Cargo workspace |
| Native 弃用与长期 owner | [`native-deprecation-plan.md`](native-deprecation-plan.md) | `zui` / `zeta-ui-components` / `app` host boundary |
| 构建、签名和发布输入 | [`app-release-graph.md`](app-release-graph.md) | root Cargo/Bazel graph + `app/packaging` |

## 文档边界

- `app/docs` 解释产品行为、跨 crate 数据流、ownership、兼容性和阶段性迁移；
- `app/README.md` 解释产品宿主当前源码路径、接入义务和 crate 组合；
- `app/<crate>/README.md` 解释单个 crate 的 public contract、关键 private symbol、测试和修改影响；
- `docs/` 只保留跨产品、共享后端、通用 UI 规范和不属于某一产品宿主的系统文档。

如果产品行为和 crate 实现发生冲突，先修正拥有该行为的实现契约，再同步本目录的系统文档；不要在
`app/src`、`zui` 或共享后端之间建立第二份权威状态。
