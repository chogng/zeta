# `zeta-workbench`

1. `tabpart/` 管顶层 Tab 的分组、顺序、选中、搜索、右键菜单和切换，以及 Titlebar、Tab Container、Toolbar 的基础 UI；`panepart/` 管 Pane 拆分树、组内输入、活动 Pane、sash UI 和拖拽周期。
2. 根级 `layout.rs` 管窗口 Part 的显示、尺寸和几何计算；`WorkbenchHost` 是唯一公开的状态变更入口，将 Tab、Pane、布局和 `PaneInput` binding 作为一个周期提交。
3. Session、Terminal、Files、Changes、Settings 等内容由对应能力 crate 管自己的状态和 UI；Workbench 只保存 `PaneInput` 描述与不透明 binding，并负责组合键等待提示等工作界面反馈，在关闭 Pane/Tab 时返回需要释放的 binding。

```text
WorkbenchHost
├── Workbench
│   ├── TabPart → TabGroup → TabInput
│   └── PaneContainer per TabInput
│       └── PanePart → PaneGroup → PaneInput
├── WorkbenchLayoutState
└── Pane binding: TabInputKey + PaneGroupId + PaneInputId
```

`TabInputKey` 标识顶层 Tab，`PaneGroupId` 标识拆分叶子，`PaneInputId` 标识组内内容。切换到已有输入会复用原 binding；新建输入和拆分 Pane 只在结构验证后创建 binding；关闭 Pane 或 Tab 会一次返回该边界内的全部 binding。

验证：`cargo test -p zeta-workbench --lib`。
