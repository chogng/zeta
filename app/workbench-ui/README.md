# `zeta-workbench-ui`

`zeta-workbench-ui` 负责 Rust Desktop 产品的 Workbench 导航和标题栏界面。跨 crate 的 Workbench 行为与所有权见 [`native-terminal-ui.md`](../docs/native-terminal-ui.md)，纯逻辑模型由 [`zeta-workbench`](../workbench/README.md) 负责。

## 边界

本 crate 把 `zeta-workbench::{TabPart, TabGroup, TabInput}` 映射为横向或纵向界面。它负责 Workbench 交互标识、无障碍节点、`TabContainer`、`TabContainerToolbar`、`Titlebar`、`WorkbenchUiStyle`、可调整宽度的 `TabContainerState`，以及 Inspector 的展开状态和首选宽度 `InspectorPartState`。

本 crate 不负责 Session 或 Settings 生命周期、命令执行、窗口事件路由、帧调度和持久 Workbench 状态。产品宿主提供模型快照、主题值、当前交互状态和命令接线。

## 依赖方向

```text
app
  → zeta-workbench-ui
      → zeta-ui-components
      → zeta-workbench
      → zeta-workbench-layout
      → zui
```

`zeta-ui-components` 不得依赖本 crate 或任何 Workbench 模型。`zeta-workbench` 必须保持与渲染无关，不得反向导入本 crate。

## 文件职责

| 文件 | 职责 |
| --- | --- |
| `workbench/tabs.rs` | 把 Workbench 分组和输入转换成横向或纵向 Tab 界面。 |
| `workbench/toolbar.rs` | 组合垂直 Tab 上方的 Session 搜索和创建入口。 |
| `workbench/titlebar.rs` | 组合可拖动标题栏、横向 Tab 和 Workbench 开关。 |
| `workbench/tabs_state.rs` | 保存垂直 Tab 容器的显隐、首选宽度和调整宽度状态。 |
| `workbench/inspector_state.rs` | 保存 Inspector 的显隐和首选宽度。 |
| `workbench/identity.rs` | 定义稳定的 Workbench 交互标识，不把 UI identity 泄漏进逻辑模型。 |
| `workbench/style.rs` | 接收产品宿主解析后的颜色和图标。 |

## 验证

修改 Workbench 投影、交互标识、标题栏、工具栏或宽度调整行为后，运行 `cargo test -p zeta-workbench-ui`。若同时修改逻辑 Tab 模型，还需运行 `cargo test -p zeta-workbench`。
