# `zeta-workbench`

`zeta-workbench` 是 Workbench 的纯逻辑模型。它负责 Tab、Pane、PaneGroup、PaneInput、激活、分割、关闭和 workspace 恢复等状态转换，不依赖 `zeta-ui`、`zui`、renderer、窗口事件或具体产品 runtime。

## 边界

`Workbench` 持有 `TabPart`、`TitlebarPart` 和按 Tab 分配的 `PanePart`。`PanePart::tree()` 返回只读的 `PaneNode` 拓扑；`set_split_ratio` 只修改逻辑 split ratio，不接收 UI 布局类型，也不处理鼠标拖拽。

`InspectorPartState` 只保存 Inspector 的展开状态和首选宽度。布局约束、sash、拖拽和指针状态由产品宿主与 `zeta-workbench-layout`、`zeta-ui` 协作完成。

## 依赖方向

```text
zeta-workbench
    └── zeta-protocol
```

`zeta-workbench-layout` 消费本 crate 的 `PaneNode` 并生成几何快照；`zeta-workbench-host` 持有本 crate 的模型并协调通用 binding。两者都不能把 UI 或产品 runtime 反向放回模型层。

## 验证

运行 `cargo test -p zeta-workbench` 验证 Tab/Pane 状态转换、split ratio 和 PaneNode 快照。
