# `zeta-workbench-host`

`zeta-workbench-host` 是 Workbench 模型与产品内容运行状态之间的通用协调层。它持有一个 `Workbench` 和一个泛型 `PaneHost<B>`，但不解释 `B`，因此 Terminal、Agent、Editor 和其他产品运行状态可以复用同一套 binding contract。

## 边界

每个 Tab 在 `Workbench` 中一对一拥有一个 `PaneContainer`，容器再拥有 PaneGroup 拓扑与 PaneInput。`PaneHost` 使用 `PaneHostScope::Tab(TabInputKey)` 和 `PaneKey` 查找 Pane binding，并以 `PaneMount` 返回 Pane 输入、逻辑 Pane identity 和不透明 `PaneBindingId`。Tab 关闭由 `WorkbenchHost::close_tab` 原子地移除逻辑 Tab、对应 PaneContainer 以及该容器的所有 binding；返回的 binding 由产品宿主负责释放具体运行状态。

`WorkbenchHost::workbench`、`workbench_mut`、`pane_host` 和 `pane_host_mut` 是显式访问边界，不通过 `Deref` 隐藏模型所有权。`layout` 只委托给 `zeta-workbench-layout`，不参与渲染。

本 crate 不包含 Terminal session、App Server、窗口事件、产品颜色、图标、Shell ID、UI dispatch、retained runtime 或 frame scheduler。

## 依赖方向

```text
zeta-workbench-host
    ├── zeta-workbench
    └── zeta-workbench-layout
```

产品层可以把 `PaneBindingId` 映射到具体 runtime，但不能把具体 runtime 类型下沉到本 crate。

## 验证

运行 `cargo test -p zeta-workbench-host` 验证 binding 创建、查询、挂载、Tab 清理和布局委托。
