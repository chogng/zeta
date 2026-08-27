# `zeta-workbench`

`zeta-workbench` 是整个应用的 Workbench 边界，负责三件事：

1. `tabpart/` 保存 TabPart 的分组、选中状态、Tab/Titlebar/Toolbar UI 和 Tab Container 的宽度状态。
2. `panepart/` 保存 PanePart 的分割树几何投影；根目录的 `layout.rs` 组合 Titlebar、Tab Container、主工作区和 Inspector 的布局。
3. `host.rs` 保存 Pane 与外部能力运行时之间的绑定，并在 Tab/Pane 变化后给出统一的挂载、激活、停用和释放边界。

## 状态层级

```text
Workbench
├── TabPart
│   └── TabGroup → TabInput
└── PaneContainer (one per TabInput)
    └── PanePart
        └── PaneNode → PaneGroup → PaneInput
```

`TabInputKey` 标识顶层 Tab，`PaneGroupId` 标识可见矩形，`PaneInputId` 标识一个 Pane 内的内容。Workbench
保证每个 Tab 都有对应的 PaneContainer；关闭 Tab 会一次性移除它的 PaneContainer，并由 `PaneHost` 返回需要
释放的外部 binding。

## 交互与生命周期

Tab 切换、关闭、创建和 Pane 分割都先通过 `Workbench` 形成完整的逻辑变更。`WorkbenchLayout` 只从当前
状态计算 bounds，不修改状态；`PaneHost` 只保存不透明 binding，不解释 Session、Terminal、Settings 或
Editor 的具体状态。具体内容的状态和 UI 由对应能力 crate 持有。

典型顺序如下：

```text
用户输入
  → Workbench 状态变更
  → 计算需要 detach/attach/activate/dispose 的 binding
  → 宿主调用对应能力 crate 的生命周期方法
  → Workbench UI 和内容 UI 使用同一帧快照重绘
```

`zeta-ui-components` 提供通用控件，`zui` 提供 UI 框架和帧调度；它们不反向依赖 Workbench。

## 验证

```text
cargo test -p zeta-workbench
cargo test -p zeta-terminal-workspace
```
