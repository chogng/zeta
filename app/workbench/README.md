# `zeta-workbench`

`zeta-workbench` 是 Workbench 的纯逻辑模型。它负责 Tab、PaneContainer、PanePart、PaneGroup、PaneInput、激活、分割和关闭等状态转换，不依赖 `zeta-workbench-ui`、`zeta-ui-components`、`zui`、renderer、窗口事件或具体产品运行状态。

## Tab 与 Pane 的层级

这里有两层容易混淆的页签：Workbench 顶层页签由 `TabInput` 表示；一个 Pane 内部可切换的内容页签由 `PaneInput` 表示。顶层页签切换的是整套 Pane 布局，Pane 内部页签只切换当前 `PaneGroup` 显示的内容。

```text
Workbench
├── TabPart
│   ├── TabGroup
│   │   └── TabInput (Session 或 Settings)
│   └── active: TabInputKey
└── pane_containers: HashMap<TabInputKey, PaneContainer>
    └── PaneContainer
        └── PanePart
            ├── PaneNode
            │   ├── Split
            │   └── Leaf(PaneGroupId)
            ├── PaneGroup（每个 Leaf 对应一个）
            │   ├── PaneInput + PaneInputId
            │   └── active: PaneInputId
            └── active: PaneGroupId
```

| 层级 | 数量关系 | 负责什么 |
| --- | --- | --- |
| `TabPart` → `TabGroup` | 一对多 | `TabPart` 保存所有顶层页签分组和全局唯一的 active `TabInputKey`；`TabGroup` 只负责分组标签、折叠状态和组内顺序。 |
| `TabGroup` → `TabInput` | 一对多 | 一个 `TabInput` 就是一个 Workbench 顶层页签，当前类型为 Session 或 Settings。`TabGroup` 不拥有该页签的 Pane 内容。 |
| `TabInputKey` → `PaneContainer` | 一对一 | `Workbench` 创建顶层页签时同时创建它的 `PaneContainer`；切换顶层页签时，整套容器一起切换。 |
| `PaneContainer` → `PanePart` | 一对一 | `PaneContainer` 是顶层页签拥有整套 Pane 布局的边界，不保存具体功能的导航历史。 |
| `PanePart` → `PaneGroup` | 一对多 | `PanePart` 用 `PaneNode` 保存递归拆分树；树中的每个 `Leaf(PaneGroupId)` 必须对应一个 `PaneGroup`。它还保存当前获得焦点的 `PaneGroupId`。 |
| `PaneGroup` → `PaneInput` | 零到多个 | 一个 `PaneGroup` 对应屏幕上的一个矩形 Pane 区域。组内每个内容都有稳定的 `PaneInputId`，但同一时刻只有 active `PaneInput` 可见。新拆出的组允许暂时为空。 |

因此，当前内容的选择链是 `TabPart::active_tab_key()` → `PaneContainer` → `PanePart::active_group()` → `PaneGroup::active_input_id()`。顶层 active 只允许一个；同一个 `PanePart` 中可以同时显示多个拆分后的 `PaneGroup`，每个组各自显示自己的 active `PaneInput`。

`Pane` 不是额外的容器层。它是 `PanePart::pane()` 和关闭操作返回的值，打包一个 `PaneGroupId`、一个 `PaneInputId` 和对应的 `PaneInput`。Pane 区域统一使用 `PaneGroupId` 标识；`PaneInputId` 标识组内内容，`TabInputKey` 标识整个顶层页签及其 `PaneContainer`。

## 边界

`Workbench` 持有 `TabPart` 和以 `TabInputKey` 为键的 `PaneContainer`。每个 `TabInput` 必须同时拥有一个容器；新增 Session 由 `upsert_session_input` 或 `upsert_catalog_session_input` 原子创建两者，关闭页签时 `close_tab` 原子移除两者。调用方只能只读访问 `TabPart`，不能绕过 `Workbench` 单独修改 Tab。

`PanePart::tree()` 返回只读的 `PaneNode` 拓扑；`set_split_ratio` 只修改逻辑拆分比例，不接收 UI 布局类型，也不处理鼠标拖拽。一个 PaneContainer 可以包含多个 PaneGroup，一个 PaneGroup 可以包含多个 PaneInput；同一 Group 只有 active input 对应的 Pane 可见。

`TabInputMetadata` 保存 Tab 界面当前使用的标题、工作区和状态文本，但本 crate 不接收或解析完整 `Session`。`zeta-workbench-controller` 负责把 Session 数据转换为元数据、选择 Session 的初始 PaneInput，并保存 Files、Diff、Agent 与 Terminal 切换所需的返回状态。

`Titlebar` 和 `InspectorPartState` 属于 `zeta-workbench-ui`。标题栏动作只调用 `Workbench` 的 Pane 状态转换；Inspector 的展开状态、首选宽度、sash、拖拽和指针状态都不进入本 crate。

## 依赖方向

```text
zeta-workbench
    └── zeta-protocol
```

`zeta-workbench-layout` 消费本 crate 的 `PaneNode` 并生成几何快照；`zeta-workbench-host` 持有本 crate 的模型并协调通用 binding；`zeta-workbench-controller` 负责产品命令、Session 转换和跨功能返回状态。三者都不能把 UI 或产品 runtime 反向放回模型层。

## 验证

运行 `cargo test -p zeta-workbench` 验证 Tab/Pane 状态转换、split ratio 和 PaneNode 快照；运行 `cargo test -p zeta-workbench-controller` 验证 Session 转换、默认 Pane 策略和跨功能返回状态。
