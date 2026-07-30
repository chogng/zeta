# Renderer UI 样式所有权规范

> 本文是 Zeta Desktop Renderer 中组件、组合控件与 Workbench Part 样式边界的 canonical 文档。
> 主题值与 token 注册仍以 [`design-tokens.md`](design-tokens.md) 为准；Desktop 的跨进程和产品所有权仍以 [`zeta-desktop-architecture.md`](zeta-desktop-architecture.md) 为准。
> Command、MenuId、Context Key 与菜单型 Toolbar 的组合语义以 [`menu-system.md`](menu-system.md) 为准；本文只拥有它们最终投影到控件后的视觉边界。

## 快速理解

每一种视觉规则只能有一个 owner。调用方选择组件、传入语义状态或选择公开的 presentation variant；调用方不得通过深层 selector 重写被调用组件的内部 item、hover、active、focus 或 disabled 样式。

判断 owner 时使用下面的顺序：

1. 状态由哪个控件定义，状态视觉就由哪个控件拥有。
2. DOM 由哪个控件创建，内部结构样式就由哪个控件拥有。
3. Part 只拥有区域布局、边界、背景和直接子组件的外部尺寸。
4. 颜色和尺寸值来自 design token；token 不拥有 selector、状态机或 DOM 结构。
5. 同一组件需要不同外观时，增加有名字的 presentation variant，不在 host CSS 中穿透覆盖。

| 想修改什么 | 样式所有者 | 正确做法 |
| --- | --- | --- |
| 组件内部间距和交互状态 | 创建该 DOM 和状态的组件 | 修改组件自己的样式 |
| Workbench 区域布局和边界 | 对应 Part | 只修改直接托管区域的外部盒子 |
| 同一组件的另一种正式外观 | 组件公开的展示变体 | 增加有名字的变体 |
| 主题颜色或标准尺寸 | Design Token | 修改语义 token，不修改 selector |
| 某个业务动作的显隐 | Command/Menu/Context Key | 不用 CSS 猜业务状态 |

## 分层所有权

| 层级 | 当前代表 | 必须负责 | 禁止负责 |
| --- | --- | --- | --- |
| Primitive | `Button`、`ActionBar`、`ToolBar`、`TabList` | 通用 DOM、键盘行为、ARIA、基础布局和 primitive 自身状态 | Workbench 区域语义、Panel/Sidebar 特例 |
| Composed control | `CompositeBar`、Editor/Chat tabs control、`MenubarControl` | 领域内 item 几何、presentation variant、hover/active/selected 视觉 | Part 的位置、区域背景和网格尺寸 |
| Part | `TitlebarPart`、`PanelPart`、`SidebarPart` | Part 根节点、标题区/内容区布局、边框、背景、直接子组件占位 | 深入修改组件内部 `.zeta-action-bar`、`.zeta-tab`、`.zeta-button` 状态 |
| Contribution/View | `TerminalViewPane`、Explorer、Search、Chat | 自有内容、命令、View 内部交互和私有子控件 | Workbench Part 的全局布局或其他 View 的皮肤 |
| Theme | color/size registries 与 CSS custom properties | 视觉值、别名、主题覆盖和快照投影 | selector、DOM、active/hover 判定 |

文件位置不是所有权的唯一证据，root class 和构造者才是。Part 私有子控件可以与 Part CSS 共置，但 selector 必须以该私有子控件的 root class 开始，不能借 Part root 任意穿透共享组件。

## 当前控件的准确职责

| 控件 | 当前职责 | 状态视觉 owner |
| --- | --- | --- |
| `ActionBar` | action 排列、方向键导航、roving tabindex、item shell；可显式启用 toggled 高亮上下文 | 不自动决定业务 selected/checked 视觉 |
| `Button` | button DOM、hover、focus-visible、disabled，以及 `.checked` 与 `aria-pressed` 的并行状态投影 | checked 的具体皮肤由使用上下文决定 |
| `ToolBar` | primary/secondary action 编排、icon action 尺寸、More Actions、可选 toggled 高亮 | Button 的状态投影仍归 `Button` |
| `WorkbenchToolBar` | 把 platform action representation 适配到 base `ToolBar`；actions 仍由调用方提供 | 视觉仍归 base `ToolBar`/`Button` |
| `MenuWorkbenchToolBar` | 从 `MenuId` 解析并刷新 `WorkbenchToolBar` actions | Menu 来源不改变 toolbar ARIA 与视觉 owner |
| `TabList` | `tablist/tab` ARIA、`aria-selected`、激活回调、`.checked` 状态投影 | 不定义 Workbench tab 皮肤 |
| `CompositeBar` | View Container 切换，以及 `icon`/`label` 两种 presentation | `compositebar.css` |
| `PaneComposite` | `tabpanel`、pane 生命周期、pane header presentation | `views.css` |
| `PanelPart` | 35px title control、CompositeBar 与 toolbar 的左右布局 | `panelpart.css` 只拥有外层布局 |
| `TitlebarPart` | 左区、应用菜单、标题和右区的窗口级布局 | 通用 toolbar/button 状态不归 Titlebar |

## 状态归属矩阵

| 状态 | 判定 owner | 样式 owner | Host 可以做什么 |
| --- | --- | --- | --- |
| `hover` / `focus-visible` / `disabled` | 原生交互 primitive | primitive CSS | 选择 primitive；不得重写内部状态 |
| tab `selected` / `active` | `TabList` 投影状态，组合控件解释语义 | 具体 tabs control 或 `CompositeBar` CSS | 选择 presentation variant |
| toggle `checked` / `pressed` | command/action model | 控件并行投影 `.checked` 与 ARIA；具体 composed control CSS 决定皮肤 | 提供 checked 值或显式启用 toggled 高亮；不得在 Part 中猜状态 |
| Part visible/hidden | Workbench layout | Part/layout CSS | contribution 只能请求命令 |
| theme light/dark/high contrast | Theme service | token snapshot | 组件消费 token，不硬编码主题分支 |

`ActionBar` 是行为与排列基座，不因为它包裹了 item 就自动拥有业务 selected 皮肤。需要通用 toggled 背景时，由调用方显式启用 `highlightToggledItems`，ActionBar 再通过 `.highlight-toggled .checked` selector 应用自己的公开高亮 presentation。

和 VS Code 一致，ARIA attribute 用于无障碍语义，稳定 class 用于视觉 selector。不要使用 `[aria-pressed="true"]` 或 `[aria-selected="true"]` 作为皮肤 selector。

需要让选中状态覆盖 hover 时，先写一般 `:hover` 规则，再写同等或更高 specificity 的 `.checked` 规则；不要使用 `:not(.checked):hover` 表达状态优先级。

## 行为身份与视觉身份

行为身份和视觉身份必须使用不同契约：

| 身份 | 表达方式 | 允许用途 | 禁止用途 |
| --- | --- | --- | --- |
| 行为身份 | `IAction.id`、`data-action-id` | TypeScript 路由、动作查找、诊断和测试 | CSS selector、颜色、显隐和布局 |
| 视觉身份 | 组件拥有的稳定 class，例如 `.zeta-tab-close-action` | 组件 CSS、交互状态和 presentation | 命令分派、持久化身份和业务查找 |

组合控件定义某个 item 的语义时，也负责把该语义投影为稳定的视觉 class。底层控件可以继续保留行为 ID，但 CSS 不得把 ID 当作公开样式 API：

```css
/* 禁止：行为身份泄漏为视觉契约 */
.zeta-tab-actions [data-action-id="zeta.tab.close"] {
  visibility: hidden;
}

/* 允许：TabList 投影并拥有视觉身份 */
.zeta-tab-close-action {
  visibility: hidden;
}
```

添加视觉 class 不等于需要新的 representation。只有 DOM 结构、交互行为、ARIA 语义或生命周期确实不同，才新增 `ActionViewItem` 等专用表示。仅有颜色、显隐、间距或状态皮肤差异时，由语义 owner 在现有直接托管 shell 上投影 class。只有多个真实消费者需要同一项通用能力时，才扩展 `ActionBar`、`Button` 等基座接口；不要为了单个样式差异增加转发层、模糊选项或具体实现继承。

## Selector 规则

允许组件修改自己的内部结构：

```css
.zeta-composite-bar-label .zeta-tab {
  padding: 0 5px;
}
```

允许 Part 修改直接托管组件 root 的外部尺寸或位置：

```css
.zeta-panel-title-control > .zeta-composite-bar {
  flex: 1 1 auto;
  min-width: 0;
}
```

允许 Part 修改自己的直接内容区：

```css
.zeta-workbench-panel > .zeta-composite-content {
  overflow: hidden;
}
```

禁止 Part 穿透共享组件内部结构：

```css
/* 禁止 */
.zeta-panel-title-control
  .zeta-tab-list-scroll-content
  > .zeta-action-bar
  > .zeta-tab.checked {
  background: var(--some-color);
}
```

如果 host 需要改变 item 的 padding、radius、最小尺寸、hover 或 active 视觉，应回到该组件实现，增加语义明确的 variant。variant 使用枚举或字符串联合表达，不使用含义模糊的布尔参数。

当前示例：

```ts
export type CompositeBarPresentation = "icon" | "label";
```

Sidebar 使用默认 `icon` presentation；Panel 显式选择 `label` presentation。两种 item 几何及 hover/active 视觉均由 `CompositeBar` 拥有。

## Panel 规范

Panel 顶部的 Problems、Output、Terminal、Ports 是同一 View Container location 中的互斥目的地，因此语义是 `tablist`，实现为 `CompositeBar`。右侧是当前选中 Panel View 的上下文命令，因此语义是 `toolbar`，实现为 `ToolBar`。

| 区域 | 语义/实现 | 样式 owner |
| --- | --- | --- |
| Problems / Output / Terminal / Ports | `TabList` + `CompositeBar` | tab 状态和 label presentation 归 `compositebar.css` |
| 当前 View 命令 | `WorkbenchToolBar` 或 `MenuWorkbenchToolBar` | action 编排归 toolbar；button 状态归 button |
| title control 左右布局 | `PanelPart` | `panelpart.css` |
| terminal 实例列表 | Terminal View 内第二级 `TabList` | Terminal contribution/view |
| pane header 是否显示 | `PaneComposite` presentation | `views.css` |

Panel 不拥有 tab 的 active 下划线，也不拥有 active/hover 背景。选中态应填满 `CompositeBar` item 的命中区域，具体视觉由 `CompositeBar` 的 token 与 CSS 决定。

## Titlebar 规范

Titlebar 是 Workbench Part，不是一个巨型 toolbar。它负责窗口拖拽区、左中右区域编排、应用菜单与窗口控件的占位；其中嵌入的 Button、ToolBar、ActionBar 仍保留各自状态所有权。

| Titlebar 内容 | Owner | Titlebar 可以负责 | Titlebar 不负责 |
| --- | --- | --- | --- |
| 窗口级网格、拖拽区、左右区域 | `TitlebarPart` | 高度、排列、间距、背景、边界 | 子控件 active/hover 状态 |
| 应用菜单 | `MenubarControl` | Titlebar 只放置其 root | 菜单 item 内部状态 |
| 通用图标命令 | `ToolBar` + `Button` | toolbar root 的位置 | button hover/focus/disabled |
| 窗口控制按钮 | Electron/native integration | 预留布局与主题投影 | 模拟通用 Workbench action 状态 |

Titlebar 可以给直接托管的公共组件 root 设置适配当前背景所需的继承色，但不应改变组件内部状态规则。如果某种前景色必须在不同背景下变化，优先注册语义 token 或公开 presentation，而不是新增深层 selector。

## Token 与 CSS 的边界

Design token 回答“值是什么”，组件 CSS 回答“何时使用这个值”。

```text
Theme registry
  → --zeta-button-active-background
  → compositebar.css 的 .zeta-tab.checked
```

Theme 不判断某个 tab 是否 active，Part 也不选择 active token。`CompositeBar` 在自己的状态 selector 中消费相应 token。

新增视觉规则时：

1. 先确认已有语义 token 是否准确。
2. 没有准确 token 时，在实际消费语义的 domain 注册 token。
3. 在拥有状态的组件 CSS 中消费 token。
4. 不为一次 host 覆盖创建位置型 token，例如 `panel-tab-hover-background`；只有产品语义确实不同才拆分。

## 当前实现状态

| 项目 | 状态 |
| --- | --- |
| Panel 一级目的地使用 `CompositeBar`/`TabList` | ✅ |
| Panel 右侧上下文命令使用 `ToolBar` | ✅ |
| `CompositeBar` 提供 `icon` / `label` presentation | ✅ |
| Panel tab 几何与状态已从 `panelpart.css` 移回 `compositebar.css` | ✅ |
| Panel pane header 显示策略由 `PaneComposite` 拥有 | ✅ |
| tab 与 tabpanel 使用 `aria-controls` / `aria-labelledby` 配对 | ✅ |
| Workbench Part CSS 共享交互控件 selector 门禁 | ✅ |
| CSS 禁止使用 ARIA state attribute 作为视觉 selector 的门禁 | ✅ |
| CSS 禁止使用 `data-action-id` 作为视觉 selector 的门禁 | ✅ |
| CSS 禁止通过 `:not(.checked)` 等否定投影状态表达优先级 | ✅ |
| 全仓所有组合控件的深层 selector 自动判定 | 尚未完成 |
| Button 并行投影 `.checked` 与 `aria-pressed`，CSS 不依赖 ARIA selector | ✅ |
| ActionBar/ToolBar 通过 `highlightToggledItems` 选择通用 checked 背景 | ✅ |
| `WorkbenchToolBar` 与 `MenuWorkbenchToolBar` 不改变 base toolbar 样式所有权 | ✅ |

历史 CSS 中可能仍有不符合本规范的穿透 selector。它们是待迁移实现，不构成新的先例；修改相关区域时应就地迁移到 owner 或公开 variant。

## 审查清单

- 这个 selector 的 root class 是否由当前模块创建？
- 这个状态是否由当前模块定义？
- Part 是否只触及子组件 root 的外部尺寸和位置？
- 相同组件的差异是否通过有名字的 presentation 表达？
- 是否错误地把 `ActionBar` 当成所有 action 状态的 owner？
- 是否使用语义 token，而不是硬编码颜色或位置型 token？
- ARIA 状态、逻辑状态和视觉状态是否由同一个控件链路投影？
- selector 是否把 `data-action-id` 等行为身份误当成视觉身份？
- 新增专用 `ActionViewItem` 是否真的改变了 DOM、行为、ARIA 或生命周期，而不只是添加样式 class？
- 对基座的扩展是否已有多个真实消费者，而不是为单个 host 的皮肤差异预留？
- 文档描述的是当前行为，还是尚未实现的计划？
