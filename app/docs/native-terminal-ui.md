# app：Terminal compatibility 结构与演进

> 状态：Compatibility。Agent 开发能力、Thread authority、统一 Composer 与 direct Shell Turn 以 [`native-agent-console.md`](native-agent-console.md) 为 canonical；Agent Terminal 会话流、按需检查、最大化与主窗口组合由 [`native-layout.md`](native-layout.md) 拥有。本文只维护 Terminal Surface、PTY、grid、screen mode、selection 与 terminal protocol 的兼容性边界。

> 本文是 `app` terminal compatibility 和分阶段演进的 canonical 文档。三条公开产品线与宿主边界见 [`product-lines.md`](../../docs/product-lines.md)；本文只负责 `app` 的纯 Rust Desktop 终端实现。当前源码所有权、调用路径和测试入口见 [`app` README](../README.md)；terminal grid 与 BlockList 的实现契约见 [`zeta-terminal` README](../../zeta-rs/terminal/README.md)；文本输入、IME 与 caret 的跨 crate 所有权见 [`native-text-input.md`](native-text-input.md)；窗口 chrome 与控件占位的实现契约见 [`zui` 开发文档](../zui/README.md)。Workbench 模型、布局、界面和通用 binding 的边界见 [`zeta-workbench`](../workbench/README.md)、[`zeta-workbench-layout`](../workbench-layout/README.md)、[`zeta-workbench-ui`](../workbench-ui/README.md) 和 [`zeta-workbench-host`](../workbench-host/README.md)。

## 快速理解

Terminal session 是 `app` 主窗口的执行上下文和交互基座，但 PTY transcript 不是 Thread authority。默认布局把 Terminal command block、用户消息和 Agent response 投影到同一会话流；普通有界命令继续使用 durable Shell Turn 或 execution result，只有 terminal protocol、持续输入或用户直接接管需要时才在原位置进入完整 grid。底层拥有 PTY capability 不要求用户观察 Agent 的每次后台命令。

当前视觉采用浅色扁平界面：Top Bar、Block 输出画布与底部输入面板只用背景层级和一像素分隔线建立结构，不使用悬浮卡片、厚描边或大圆角。未接入真实状态的搜索、Agent、Git 和 Session action 不以静态装饰出现。

| 用户场景 | 界面行为 | 当前状态 | 深入阅读 |
| --- | --- | --- | --- |
| 打开 `app` | 目标布局默认进入绑定 Workspace、Thread 与 Terminal session 的统一会话流 | 部分具备；Agent ThreadTimeline、共享 Composer 和独立 Terminal Surface 已接入，统一会话流尚未完成 | [当前实现](#4-当前实现) |
| 输入有界 shell 命令 | Composer 提交 direct Shell Turn，结果进入 Thread 并可恢复 | durable Shell Turn 已实现 | [`native-agent-console.md`](native-agent-console.md) |
| 使用 `vim`、`top` 等交互式 TUI | alternate screen 临时接管 Terminal Workspace，切回 primary 后恢复 BlockList 与底部输入 | 部分具备；scroll region、常见 query 与主流 mouse modes 已接通 | [当前实现](#4-当前实现) |
| 浏览较早的主屏输出 | 在终端内容区滚轮上翻 Block transcript 或 cell history，新输出不抢走当前阅读位置 | 会话内有界回滚已实现；跨重启持久化尚无 | [当前实现](#4-当前实现) |
| 复制或粘贴终端文本 | 主屏优先复制 composer selection，再复制 Block 输出 selection；macOS 使用 Cmd+C/V，Windows/Linux 的 alternate terminal 使用 Ctrl+Shift+C/V | 快捷键已通过统一 command resolver；未加 Shift 的 terminal Ctrl+C 继续透传 | [当前实现](#4-当前实现) |
| 自定义快捷键 | 按 `Cmd/Ctrl+,` 打开设置并点击命令录制，或编辑 `<ZETA_PROFILE_ROOT>/keybindings.json` | 最多四段 Chord；设置写入采用原子替换，错误更新保留上一份有效规则并显示诊断 | [当前实现](#4-当前实现) |
| 查看当前会话导航 | Top Bar 按钮展开垂直 Session TabList；拖动右边界可调整宽度 | App Server Session Tab 的创建、投影和切换已实现；不显示 fixture | [当前实现](#4-当前实现) |
| 浏览工作区文件或变更 | 当前实现由 Top Bar 展开右栏；目标布局改为从相关 block 打开单一临时检查对象 | 根目录、上游领先/落后数和文本 MultiDiff 已接入；检查界面收敛尚未完成 | [当前实现](#4-当前实现) |
| 操作当前会话 | 右键 Session Tab，在锚点附近打开 Pin、Close、Rename、Fork 菜单 | 菜单呈现、定位和键盘交互已实现；Close 会请求 App Server Stop 并清理本地 Tab/Pane/runtime，Pin/Rename/Fork mutation 尚未执行 | [当前实现](#4-当前实现) |
| 切换多个会话 | 在同一垂直 TabList 选择另一个 App Server Session | 已实现；每个 Session Tab 绑定并保留一个独立本地 PTY | [当前实现](#4-当前实现) |
| 在 macOS 使用 Top Bar | 左侧 action 避开系统红绿灯占位并保留组件间距 | 70px host 占位 + 8px Titlebar 间距已实现 | [尺寸语义](#5-尺寸语义) |
| 调整窗口尺寸 | 从同一 viewport 重算 rows/columns，并同步 resize grid 与 PTY | 已实现 | [尺寸语义](#5-尺寸语义) |
| 拆分终端 | 只在当前 `SessionTab` 的 `PaneContainer` 内拆成多个 PaneGroup，并调整比例 | 已实现；支持 Pane Tree、独立 runtime、active Pane、Sash 比例调整、焦点切换和关闭 | [TabPart、TabGroup 与 PaneGroup](#22-tabparttabgroup-与-panegroup)、[Terminal Pane 分屏](#62-terminal-pane-分屏) |

## 1. 产品命名

命名规则只有一条：仓库内部统一属于 Zeta，只有跨过发布边界、真正交付给用户的 Native 终端
应用使用全小写名称 `app`。它来自 “Zeta Terminal”，但完整形式不作为第二个品牌。

| 边界 | 规范名称 | 示例 |
| --- | --- | --- |
| 共享仓库、crate 和内部标识 | `zeta` / `zeta-*` | `zeta-ui-components`、`zeta-terminal-*` |
| `app` Cargo package、发布的终端应用及其用户可见表面 | `app` | package、executable、app bundle、窗口标题、Top Bar 和输入提示 |

因此不把内部 crate、CSS、协议或测试标识重命名为 `app-*`。公开发布前仍需单独核查 `app`
的商标、域名、应用商店和软件包名称可用性；核查结果只影响发布层名称。

## 2. 产品结构

当前产品结构：

```text
app
├─ TopBar
│  ├─ window drag region
│  ├─ Tab Container toggle ActionBar
│  └─ workspace pane toggle ActionBar
├─ WorkbenchNavigator (collapsible)
│  └─ WorkbenchTabList
│     ├─ SessionTab
│     │  ├─ App Server Session → active Thread
│     │  └─ PaneContainer → PanePart → split tree → PaneGroup → PaneInput → content/runtime
│     └─ SettingsTab
│        └─ PaneContainer → PanePart → Settings PaneInput → feature-owned settings sections
├─ SessionContextMenu (transient overlay)
│  └─ Pin / Close / Rename / Fork
├─ Main Workbench
│  └─ active Tab.PaneContainer.PanePart
│     ├─ Pane → PaneInput::Agent → Agent timeline / composer
│     ├─ Pane → PaneInput::Terminal → TerminalSession → dedicated PTY
│     ├─ Pane → PaneInput::Files → FilesPane
│     └─ Pane → PaneInput::Diff → EditorPane → MultiDiffEditor
└─ Inspector (collapsible InspectorPart)
   └─ FileEditorPane → FileEditorHost
```

primary screen 的结构固定为“上方输出、底部输入”。键盘、IME 和 paste 先进入
`CommandInputEditor`；Enter 通过 host-owned command boundary 建立 Block，再把命令写入 PTY。
shell prompt、命令回显和行编辑不再作为 primary screen 的产品界面。输出仍由真实 PTY 产生，
BlockList 只投影已提交命令及其 printable output，不使用静态 transcript。

alternate screen 是协议兼容的明确例外：`vim`、`top` 等程序请求 alternate buffer 后，全幅 grid
临时接管 Terminal Workspace，输入直接交给该程序；退出 alternate screen 后恢复固定底部
composer。这个切换不能改变 primary screen 的 Block 输入语义。

Top Bar 不是独立工作区，也不拥有终端 Session。它只提供窗口拖动、会话入口和少量全局操作。
Workbench Navigation 当前使用可折叠、可通过右边界 Sash 调整宽度的垂直 TabList，但不构成可注册任意区域的通用 Sidebar Part。每个 TabInput 一对一拥有一个 `PaneContainer`，选择 TabInput 时切换整个容器；容器内的 `PanePart` 保存 PaneGroup 拓扑，Agent、Terminal、Files、Diff 和 Settings 都是不同的 `PaneInput`。Settings Tab 不创建或切换 App Server Session。右侧 `InspectorPart` 只保留文件编辑器 Inspector 的显隐和宽度生命周期；横向和纵向导航可以读取同一 `TabPart`，但使用各自的 presentation geometry 和 UI identity scope。

### 2.1 NavBar 导航容器

`NavBar` 是计划中的导航容器，而不是新的状态层。它统一描述横向或纵向的导航排列、可选的 leading/trailing slot、滚动或 overflow 几何，以及与宿主布局的连接；它不识别 Session、Settings、Agent 或其他具体内容。

```text
Titlebar
└─ NavBar(horizontal)
   └─ TabList
      └─ TabInputView

Body mount
└─ NavBar(vertical)
   └─ TabList
      └─ TabInputView
```

| 层 | 责任 | 当前 owner / 状态 |
| --- | --- | --- |
| `NavBar` | 横向/纵向容器、slot、滚动或 overflow 的 presentation geometry | 计划由 `zeta-ui-components` 提供 presentation-only contract；尚未作为独立 public component 实现 |
| `TabList` | Tab surface 的排列、item bounds 和状态绘制 | `zeta-ui-components::TabList`；不拥有 identity、active transition、关闭动作或 tabpanel |
| `TabPart` | 保存多个 browser-style `TabGroup`、全局 active Tab 与跨组移动/合并 | `zeta-workbench::TabPart`；不保存方向、bounds 或 UI identity |
| `TabGroup` | 保存一组有序 `TabInput` 及 label/collapsed 元数据 | `zeta-workbench::TabGroup`；不拥有独立内容 viewport 或 renderer node |
| `TabInput` | 一个可被 Workbench 导航的逻辑内容输入，例如 Session 或 Settings | `zeta-workbench::TabInput`；不分配 UI identity、不绘制、不直接执行激活副作用 |
| Tab presentation | 将 `TabPart`/`TabGroup`/`TabInput` 映射为方向、标题、状态、图标、`ElementId` 和 accessibility node | `zeta-workbench-ui::TabContainer` 同时承载 body 与 Titlebar 两种排列，两处使用独立 UI identity scope |
| Controller / provider | 处理选择后的 Session、Settings、Agent 或外部资源行为 | 当前由 `app` host 和各自 feature crate 负责；尚无通用 Provider API |

这里的 `TabInput` 采用 VS Code `EditorInput` 类似的含义，表示“被打开的内容”，不是文本输入框。搜索或创建入口继续使用 `SearchBox`、`InputBox` 等 UI 组件，避免与逻辑 `TabInput` 混淆。

逻辑导航位于 `zeta-workbench`：`TabPart → TabGroup → TabInput` 只保存稳定 identity、分组和选择。`TabContainer`、Titlebar、Toolbar、interaction identity 和 resize state 位于 `zeta-workbench-ui`；产品 app 只提供 palette、Session 可用性和命令接线。

### 2.2 TabPart、TabGroup 与 PaneGroup

`TabPart` 管理整个 Workbench 的 browser-style TabGroup 和全局 active Tab；TabGroup 只负责归组、顺序与折叠元数据，不产生独立内容 viewport。`TabInput` 是可被导航和激活的逻辑内容身份；它不等于视觉 Tab，也不直接等于一个 Pane。每个 `TabInput` 一对一关联一个 Workbench-owned `PaneContainer`，选择 TabInput 时切换整个容器。PaneContainer 持有当前 Tab 的 PanePart 与恢复状态；PanePart 管理 split tree 和 active PaneGroup；每个 PaneGroup 对应一个可见矩形区域，并可拥有多个 PaneInput，其中一个 active。右侧 `InspectorPart` 是文件编辑器 Inspector，不拥有这些 PaneGroup。

```text
Workbench
├─ TabPart
│  └─ TabGroup → TabInput
├─ NavBar(horizontal/vertical presentation)
│  └─ TabList → TabInputView
└─ TabInput → PaneContainer
   └─ PanePart
      └─ split tree
         ├─ PaneGroup → PaneInput tabs
         └─ Split(direction, ratio)
            ├─ PaneGroup → PaneInput tabs
            └─ PaneGroup → PaneInput tabs
```

| 层 | 责任 | 当前 owner / 状态 |
| --- | --- | --- |
| `TabPart` | 保存 TabGroup 顺序、全局 active Tab、跨组 move/merge 与 Session 选择回退 | `zeta-workbench::TabPart`；由 Workbench 唯一持有 |
| `TabGroup` | 浏览器式归组多个 TabInput，不创建新的内容区域 | `zeta-workbench::TabGroup`；label/collapsed 是投影元数据，active 仍属于 TabPart |
| `TabInput` | 描述“打开了什么”，保存稳定逻辑 identity、标题和状态摘要 | `zeta-workbench::TabInput`；不保存 Pane geometry 或 runtime handle |
| `PaneContainer` | 保存一个 TabInput 的完整 pane container，并作为 Tab 切换和关闭的单位 | `zeta-workbench::PaneContainer`；一对一归属于 TabInput，持有 PanePart 和 Tab 级恢复状态 |
| `PanePart` | 保存 PaneGroup split tree、active group、split/close/focus transition 与 ratio | `zeta-workbench::PanePart`；只负责容器内部拓扑，不拥有 Tab 级状态 |
| `PaneGroup` | 一个可见矩形区域，保存多个 PaneInput 和其中一个 active input | `zeta-workbench::PaneGroup`；不保存 renderer node 或 feature runtime |
| `Pane` | 描述一个内容承载位置；`PaneGroupId` 标识布局叶子，不等于内容类型 | `zeta-workbench::Pane`；通过 host binding 关联一个 `PaneInput` 和可丢弃 view state |
| `PaneInput` | 描述 Pane 当前承载的内容类型与逻辑 identity，不是 UI widget，也不保存 geometry | `zeta-workbench::PaneInput`；当前定义 `Terminal`、`Agent`、`Files`、`Diff`、`Settings` descriptor，具体 payload 由对应 feature owner 解释 |
| `PaneInputKind` | 供 host 做能力路由和组合策略判断的稳定内容类型 | `zeta-workbench::PaneInputKind`；`Agent-1`、`Agent-2`、`Agent-3` 是同一 kind 下的不同 thread/session identity，不是三种 Pane type |
| `Pane binding` | 把 `(PaneHostScope, PaneGroupId)` 映射到 `PaneInput`，再连接到具体 runtime/view state | `zeta-workbench-host::PaneHost`；产品侧 `PaneBinding` 通过 `PaneBindingId` 连接 Terminal、Agent、Files、Diff 或其他 runtime，Inspector 不属于该 PaneGroup |
| `PaneView` | 按 PaneInput 绘制内容并消费 feature-owned state 的 view contract | 各自 feature crate / Native host；不要求所有内容共用 TerminalSession |
| `PanePart` layout projection | 把 Workbench-owned split tree 投影为 PaneGroup bounds、owning split 和 sash geometry | `zeta-workbench-layout::PaneGroupLayout` 组合 `zui::GridLayout`；只返回几何，不修改 PanePart |
| `PaneHost` | 按 Pane bounds 组装具体内容并路由 keyboard、pointer、resize 与 accessibility | `zeta-workbench-host::PaneHost`；按 `PaneHostScope` 和 `PaneKey` 产生含不透明 `PaneBindingId` 的 mount，产品层再解释具体 runtime |

分屏的边界是“一个 TabInput 对应一个 PaneContainer”，PaneContainer 内的 PanePart 再包含一个或多个 PaneGroup。TabGroup 只改变 Tab 的归组与顺序，不改变 PaneContainer 拓扑。Settings 可以暂时只有一个 PaneGroup，但不需要为它建立另一套 Tab/Pane 模型。

`PanePart` 的几何输入由 `zeta-workbench-layout` 消费，`zui::GridLayout` 负责递归计算 SplitView bounds；PaneContainer 和 PanePart mutation 留在 `zeta-workbench`，TabInput 到 PaneContainer 的一对一关系由 Workbench 持有，通用 Pane binding 由 `zeta-workbench-host` 协调。PaneGroup/PaneInput 到 Session/PTY 或其他 feature runtime 的 binding，以及每个 Pane 的 scroll、selection、input 和 runtime 状态留在产品 host 或具体内容 crate。

本次实现先建立异构 `PaneInput` 的 host contract，并让 `TerminalPaneInput` 接入现有 Session PaneContainer 的 PanePart：split 创建独立 terminal Pane，active Pane 接收输入，Pane 各自 resize、绘制和保存 view state。Changes/Files 选择会把当前 Session PaneContainer 的 root binding 更新为 `PaneInput::Diff`/`PaneInput::Files`，并通过现有 feature-owned `EditorPane`/`FilesPane` 绘制；titlebar action 可以在 workspace pane 与 Agent pane 之间切换。右侧 `zeta-workbench-ui::InspectorPartState` 只作为文件编辑器 Inspector 的 shell 状态，不参与 Files/Diff 的 Pane topology。

选择 `PaneInput` 而不是 `EditorInput`，是因为 Terminal、Agent、Files、Diff 和 Settings 并不都是 editor。`PaneInput` 表示“要在这个 Pane 位置打开什么”；`Pane` 表示“这个内容当前位于哪一个布局叶子”；`PaneView` 才负责把输入转换成具体 UI。一个 `PaneGroup` 可以有多个 `PaneInput` kind，但一个可见 Pane 只绑定一个 PaneInput；同一个逻辑 input 若需要同时打开多次，应由不同的 `PaneGroupId` 或功能实例标识区分。

PaneGroup 拥有 UI-neutral 的 PaneInput descriptor 与 active input identity，但不拥有对应的 renderer、TerminalSession 或 feature runtime。host 可以使用 `PaneInputKind` 做允许组合、快捷键和输入路由判断，再把内容绘制委托给对应 crate；`zeta-ui-components` 只接收 PanePart 的几何 spec，不识别这些 kind。

PaneGroup 的纯结构模型位于 `zeta-workbench`。产品 pane binding 由 `zeta-workbench-controller` 对接，Session 到 terminal runtime 的生命周期由 `zeta-terminal-workspace` 管理，产品 command 和 scene lifecycle 留在 app composition；稳定的通用递归几何位于 `zeta-workbench-layout` 并复用 `zui`，不把 feature state 或产品 runtime 下沉到 Workbench crate。

## 3. 所有权

| 能力 | 最终 owner | 职责边界 |
| --- | --- | --- |
| Window 与 Terminal Workspace 外部布局 | `app` product host | 决定窗口区域和活动会话，不进入 UI 组件或 Workbench UI crate |
| 单轴 Pane 尺寸约束、Sash track、feedback geometry 与通用 resize snapshot drag | `zui::SplitViewLayout` / `zeta-ui-components::{Sash,SashController,Resizable}` | 不持有产品显隐、preferred width、pointer capture 或持久化；`Resizable` 只负责通用 drag 计算 |
| 递归 Pane geometry 与 owning-split Sash 路由 | `zui::GridLayout` | 递归组合 SplitView；不持有 Terminal Session、Agent content、Pane Tree mutation 或 active Pane |
| Product PaneContainer/PanePart/PaneGroup topology 与选择状态 | `zeta-workbench` | Workbench 保存 TabInput → PaneContainer；PaneContainer 持有 PanePart，PanePart 保存 split tree、active PaneGroup 与 ratio，PaneGroup 保存 PaneInput tabs；不进入 UI crate |
| PaneInput descriptor 与 Pane binding | `zeta-workbench::PaneInput` + `zeta-workbench-host::{PaneHost,PaneHostScope}` | 区分 Pane layout identity、Terminal/Agent/Files/Diff/Settings content kind 与可选 runtime；产品层使用 `PaneBindingId` 连接具体 runtime |
| PaneGroup 的递归叶子 bounds 与 owning-split sash | `zeta-workbench-layout::PaneGroupLayout` + `zui::GridLayout` | 消费 Workbench 提供的 `PaneNode`；只计算 bounds/sash，不拥有 PaneTree mutation 或 active Pane |
| Tab Container 显隐与 preferred width；产品 resize gesture | `zeta-workbench-ui::TabContainerState` | 使用通用 Split/Sash geometry；状态保存尺寸、显隐和 drag snapshot，产品 app 负责命中与 pointer 路由；不拥有 TabInput 或 Session lifecycle |
| InspectorPart 显隐与 preferred width；产品 resize gesture | `zeta-workbench-ui::InspectorPartState` | 使用通用 Split/Sash geometry；产品 app 负责命中与 pointer 路由；默认 520px、限制为 360–800px，并为 main Pane 保留至少 400px；不拥有 Files/Diff feature state |
| Main workspace Files/Diff composition | `zeta-workbench-host::PaneHost` + [`features/workspace`](../src/features/workspace/workspace_panes.rs) 的 `WorkspacePaneNavigation` / `FilesPane` / `EditorPane` | 产品 host 按 `PaneInputKind` 挂载主工作区 Pane；workspace feature 只拥有 Files/SCM state 与具体 view，不拥有外层 Workbench geometry |
| Files 树、模糊搜索与领先/落后显示 | [`features/workspace`](../src/features/workspace/workspace_panes.rs) / `zeta-file-search` / `zeta-git` | `features/workspace::files::FilesState` 保存可丢弃 UI 状态；app 适配目录 DTO 并执行动作，Git 命令解析和模糊匹配器仍由各自 crate 拥有 |
| 多文件差异内容与视口 binding | `features/workspace::scm::EditorPane` / `zeta-editor::MultiDiffEditor` | SCM feature 保存 changed-file collection 与每文件 `DiffEditorState`；Native 只提供 SCM 投影 |
| 通用 UI 滚动 geometry、交互映射与状态 transition | `zeta-ui-components::ScrollView` / `ScrollState` / `ScrollbarController` | MultiDiff 复用完整 logical-pixel 状态和交互映射；BlockOutputViewport 通过 Native adapter 复用 clip、内容坐标和 scrollbar paint；Terminal 仍保留底部相对行锚定与输出增长策略 |
| Top Bar 内部 action 排列 | `zeta-ui-components::ActionBar` | 后续有真实 action 时使用；只拥有 representation geometry 和 paint |
| 通用 Tab surface 与横/纵排列 | `zeta-ui-components::Tab` / `TabList` | 只拥有 presentation state、item size/gap、surface paint 和同源 bounds；不拥有 product content 或 tabpanel |
| NavBar 导航容器 | 计划中的 `zeta-ui-components` presentation contract | 尚未实现；若落地，只拥有方向、slot、滚动/overflow geometry，不拥有产品 identity 或状态 |
| Workbench Tabs 与活动 Session/Settings presentation | `zeta-workbench::{TabPart,TabGroup,TabInput}` + `zeta-workbench-ui::TabContainer` + app activation wiring | `TabPart` 统一维护分组和全局 active selection；两种排列各自分配 `ElementId`，只通过 `TabInputKey` 汇合，Settings 不复制 Session lifecycle |
| 锚点浮层定位、viewport 约束与 layer 合成 | `zeta-ui-components::ContextView` | 不拥有显示生命周期、输入路由或产品 action |
| 无边框下拉 surface、可选 header、纵向 item geometry 与默认选择 | `zeta-ui-components::Dropdown` | 组合 ContextView/ActionBar；不拥有产品查询、选择 identity、关闭或 command |
| 柔和阴影、2px menu padding、4px radius、纵向 item geometry 与默认选择 | `zeta-ui-components::ContextMenu` | 组合 ContextView/ActionBar；不拥有 Session identity、关闭或 command |
| Session Tab 右键菜单生命周期与 command identity | `app/src/session/session_context_menu` | 保存目标、锚点与恢复焦点；菜单关闭后不保留第二份 Session 状态 |
| Product command identity 与注册式执行 | [`zeta-commands`](../commands/README.md) / `app/src/app/command_dispatch` | pointer、menu 和 shortcut 只提供入口，业务行为汇合到同一 `CommandRequest`，再由宿主注册的 handler 执行 |
| 平台无关按键、规则顺序与冲突解析 | [`zeta-keybinding`](../../zeta-rs/keybinding/README.md) | 不读取 winit event、focus、terminal state 或用户配置，不执行产品 command |
| winit 按键转换、Native context 与 Chord 生命周期 | `app/src/platform/keybindings` | 内建 Copy/Paste；1.5 秒超时，失焦或 IME 取消；保持 alternate terminal Control 序列透传 |
| Native 用户快捷键资源 | `app/src/platform/keybindings` | 读取 `<ZETA_PROFILE_ROOT>/keybindings.json`；完整校验成功才替换，坏更新保留上一份规则 |
| 快捷键设置、录制和提示 | [`app-keybinding-ui`](../keybinding-ui/README.md) | 拥有浮层 lifecycle、录制 deadline、诊断呈现和组件样式；app 提供产品 command、事件 adapter 与保存接线 |
| Terminal Session product state | App Server/terminal session runtime | 拥有进程、cwd、环境、输出与退出状态 |
| Terminal grid、screen/mode state、基础 escape sequence 与 BlockList | `zeta-terminal::TerminalCore` | 不由 `UiScene` 或 `InputBox` 推断 |
| PTY process、write、resize 与 exit | `app/src/features/terminal/terminal_session` + `zeta-utils-pty` | process mechanism 与 terminal model 分离；创建在后台 worker 完成 |
| Session Tab 到本地 PTY 的一对一 binding、活动/非活动 runtime 切换 | `zeta-terminal-workspace::TerminalWorkspace` | 管理 pending key、ready 顺序和非活动 runtime；不拥有 App Server Session/Thread authority，也不依赖产品事件类型 |
| cell scrollback retention | `zeta-terminal::TerminalGrid` | 会话内最多保留 10,000 行；不负责跨重启持久化 |
| scroll position | `app/src/features/terminal/terminal_scrollback` | 可丢弃的 presentation state，不写回 terminal model |
| terminal output selection | `app/src/features/terminal/terminal_selection` | 可丢弃的 viewport state；文本来自 terminal/Block projection |
| 跨重启历史持久化与完整 terminal compatibility | 后续 terminal runtime | 尚未完成 |
| BlockList / TerminalOutput presentation | Native terminal session view | 呈现 runtime output；不能成为第二份权威输出存储 |
| Primary Block Input Editor 与 IME candidate area | `zeta-composer::Composer` + `app/src/features/terminal/terminal_input` + `app/src/platform/input_method` | 编辑 host-owned `TextInput`；Enter 才提交真实 command boundary |
| 命中、指针状态、focus、键盘导航与 accessibility semantics | `zui` | 只分发稳定控件身份和 activation intent，不保存 Session、文件、对话或文档状态 |
| 平台 accessibility publication | `zui` private AccessKit adapter | 已发布现有 tree/focus/selection/expansion 并回流 Focus/Click；平台读屏 smoke coverage 仍需补充 |
| alternate-screen direct input | `app/src/features/terminal/terminal_input` + `app/src/platform/input_method` + `TerminalCore` | 仅在 TUI 接管期间编码 key/IME/paste 并写入 PTY |
| shell command completion boundary | `app/src/features/terminal/terminal_session` bootstrap + `zeta-terminal::TerminalCore` | 当前 zsh 使用 OSC 133 `D`；其他 shell 只有基础 prompt/echo suppression |
| Rect、icon、text scene 与 GPU draw | `zui::ui` / private `zui::render/wgpu` | 不拥有 Session、PTY、窗口布局或产品 reducer |

`app` 可以保存活动 Tab、hover、focus、scroll position 等可丢弃 presentation state，但
Session、Thread、Turn、PTY process 和 durable output 必须来自对应 runtime。

## 4. 当前实现

| 当前实现 | 当前事实 | 目标映射 |
| --- | --- | --- |
| `titlebar::Titlebar` | 绘制 32px 窗口顶区、拖拽区、横向 `TabContainer`、Tab Container toggle 和 Workspace Pane toggle `ActionBar`；不显示独立标题文案 | Top Bar |
| `zui::WindowControlInsets` | 按 native chrome policy 提供覆盖产品内容的左右逻辑占位；macOS full-size titlebar 当前为左侧 70px | 原生窗口控件安全区 |
| `zeta-workbench-ui::TabContainer` | 按 `TabGroup` 把 `TabPart` 组合成一个或多个 `zeta-ui-components::TabList`；body 使用两行纵向项，Titlebar 使用单行横向项，Settings 绘制 gear icon | 通用 TabList 支持双向排列；App Server Session 切换、browser-style group model、singleton Settings selection 和双挂载 identity 已接入 |
| `session::session_context_menu::SessionContextMenu` | 右键任一真实 Session Tab 后，用通用 `ContextMenu` 基座绘制 Pin、Close、Rename、Fork；目标保存逻辑 `TabInputKey`；基座提供 renderer 柔和阴影、2px padding 与 4px radius，默认选择 Pin；菜单子树打开时成为 modal interaction scope，hover 同步 roving focus 并在移出后保留最后一项，同时支持菜单外点击、Escape、上下键、Tab、Enter/Space 与焦点恢复 | 下层控件在菜单打开期间不接收 pointer、focus 或 activation；Close 通过 App Server Stop contract 执行并清理本地 Tab/Pane/runtime，Pin/Rename/Fork transition 尚未执行 |
| `ShellLayout` | 组合扁平 titlebar、可选 Tab Container，并把剩余区域交给 `WorkspaceLayout` | primary screen 窗口外层布局 |
| `zeta-workbench-layout::PaneGroupLayout` / `zui::GridLayout` | 把 Workbench `PaneNode` 投影为递归 Grid Leaf 和 owning-split Sash；alternate screen 使用每个 Pane 自己的完整 Leaf | Terminal PanePart 的 host model、逐 Pane runtime、焦点路由、Sash 比例调整和逐 Pane resize 已接入 |
| `pane_input::{PaneInput,PaneInputKind,PaneBinding}` / `zeta-workbench-host::{PaneHost,PaneHostScope}` | 把 Pane leaf 映射到 Terminal、Agent、Files、Diff 或 Settings descriptor，并在 frame 组装阶段产生含 `PaneBindingId` 的 view mount | Session Tab 的 Agent/Terminal/Files/Diff Pane 已挂载；Settings 仍待具体 view/state 接入 |
| `TabContainerState` / `zeta-ui-components::Resizable + Sash` | host 保存 preferred width 与 visibility；Resizable 保存 drag snapshot/relative delta，Sash 只绘制 8px track / 2px feedback | 垂直投影宽度限制为 160–480px，并始终为 main Pane 保留至少 240px |
| `InspectorPartState` / `zeta-ui-components::Resizable + Sash` | host 保存 Inspector Part 的 preferred width 与 visibility；Resizable 保存 drag snapshot/relative delta，Sash 只绘制 8px track / 2px feedback | 默认 520px、限制为 360–800px，并始终为 main Pane 保留至少 400px；只承载文件编辑器 Inspector |
| `features/workspace::WorkspacePaneNavigation` | 跨功能 Changes/Files ActionBar 与导航语义 | 不拥有 Files/SCM 功能布局 |
| `features/workspace::files::{FilesLayout, FilesToolbar, FilesPane}` / `zeta-file-search` | Files 自己拥有 36px 功能 toolbar、根目录文件树与工作区路径模糊匹配结果 | Search 输入已接键盘、剪贴板和 IME；目录展开、滚动和文件打开动作已由 Native adapter 接线 |
| `features/workspace::scm::{ScmLayout, EditorPaneState, EditorPane}` / `zeta-editor::MultiDiffEditor` | SCM 自己拥有 Changes toolbar slot 与整体纵向视口；Native 将 `zeta-git` snapshot 映射为 `ScmDiff` | 启动、Refresh 与 command completion 更新；binary、非 UTF-8 或单侧超过 2 MiB 的文件跳过 |
| `zeta-commands::{AppCommandId, CommandRequest, CommandRegistry}` / `keybindings::NativeKeybindings` / `keybindings_resource::KeybindingsResource` / `keyboard_shortcuts` | pointer/menu 与标准化键盘事件汇合到同一 command request，再由宿主注册的 handler 执行；resolver 支持 `when`、Builtin/User precedence、blocker 和最多四段 Chord；资源轮询外部编辑，设置录制采用原子写入 | ✅；内建 Copy/Paste、1.5 秒 Chord timeout、失焦/IME 取消、冲突诊断、Chord 提示与设置 UI 已实现 |
| `TerminalCore` / `TerminalGrid` | 增量解析 ANSI，维护 cell、cursor、wrap、erase 与基础 SGR | 当前最小 terminal emulator core |
| Unicode terminal text | CJK 按双 cell 保存；组合符、ZWJ Emoji 与 flag 序列保留在 leading cell；renderer 使用系统 outline fallback | macOS 已规避不可栅格化的 `GB18030 Bitmap`；复杂 BiDi 行级布局尚未完成 |
| primary/alternate screen | 解析 `47/1047/1048/1049`，切换 active grid，并在 resize 时同步两块 grid | 已实现基础 buffer lifecycle |
| DEC mode 与 terminal input | 记录 cursor key、cursor visibility、bracketed paste 和 mouse request state；编码基础 key | 常见 input mode 已形成闭环 |
| scroll region 与 terminal query | 支持 margin scrolling、origin-relative cursor、line insert/delete，并把 DA/DSR/CPR reply 写回 PTY | 常见纵切已实现，尚非完整 query family |
| terminal mouse report | alternate screen 内把 pointer cell、button/motion/wheel 和 modifiers 编码为 1000/1002/1003 legacy 或 1006 SGR report | 不接管 titlebar；1005/1015 尚未实现 |
| 主屏 scrollback | full-screen scroll 把 cell rows 保留到 10,000 行有界历史；局部 scroll region 和 alternate screen 不进入历史 | `CSI 3 J` 清理历史；当前没有磁盘持久化 |
| Native 回滚浏览 | 主屏滚轮浏览 Block transcript/cell history，并在阅读旧输出时保持新输出锚定；`TerminalOutputScrollView` 把行窗口映射到通用 `ScrollView` | alternate screen 的应用鼠标报告优先于产品滚动；Terminal 行锚定不下沉到 UI 基座 |
| resize reflow | 主屏按 soft-wrap metadata 重排 history/live rows，并映射 cursor、pending wrap 与 wide cells | alternate screen 和自定义 scroll region 保持 fixed-grid resize |
| selection / clipboard | 主屏复制 composer 或 Block 输出 selection，paste 编辑 composer；alternate screen 按 terminal mode 写入 PTY | 尚无双击词、三击行和 selection auto-scroll |
| OSC title | 解析 OSC 0/2 并同步 native window 与 Session Tab | 标题去 control characters，限制为 256 字符 |
| `BlockList` | host submit 建立 Block，保存有界 printable output，过滤 PTY echo，并在 OSC 133 `D` 上完成当前 Block | primary screen 的权威 output projection |
| `terminal_session::TerminalSession` | 启动默认 shell、抑制原生 prompt/echo、提交整条命令、转发 PTY output/exit、处理 resize | zsh 已有最小 completion hook；其他 POSIX shell 只有基础 bootstrap |
| `zeta-composer::Composer` / `app/src/features/terminal/terminal_input` | primary screen 编辑 `TextInput` 并在 Enter 时提交 | 当前为单行 |
| `app/src/platform/input_method` | 根据 window、screen 与 focus 选择 Disabled/Composer/TerminalGrid，转换 IME 事件并同步 candidate area | preedit 状态由共享 `TextInput` 模型维护 |
| 输入上下文工具栏 | Bottom Widget 最底部用 `ActionBar` 排列四个带图标标签的 `Button`：Local、当前工作区目录、Git branch 与 diff count | 目录按钮复用带 header slot 的 `Dropdown`，分支按钮复用带同类 header 的 `ContextMenu`；两者第一行均默认聚焦 Search Box，并分别实时过滤当前层级子目录与本地分支。成功后替换 Files 根、搜索索引和 Git/Changes 投影；环境选择器尚未绑定 |
| 统一 UI 分发 | `zui` 的 `ElementId`、父子 `UiNode`、反向 hit-test、focus order、同组导航、`UiIntent` 与每帧 accessibility snapshot | 当前 Titlebar、Session TabList、Session Menu/MenuItem、Sash separator、terminal output、composer、toolbar 和 Button 已接入，并由 AccessKit adapter 发布 |
| primary/alternate Native presentation | primary 绘制 BlockList + 固定底部 composer；alternate 绘制全幅 active grid/cursor | Warp 式主屏与 TUI compatibility 已分流 |
| `ActionBar` / `Button` | presentation-only action 与 icon button | 保持通用 primitive，不接收 terminal domain state |
| `TabList` / `Tab` | presentation-only Tab 排列与 surface | 当前用于 Session navigation；changed-file diff 不再使用 Tab |
| 完整 DEC/query/mouse family、跨重启历史持久化 | 尚未实现 | 后续 terminal compatibility / Session durability 纵切 |
| Session restoration | 尚未实现 | 后续产品能力；Terminal PaneContainer 的当前内存状态不跨重启持久化 |
| Terminal PanePart / split panes | 本次实现 | 当前只在 Terminal Surface 内提供产品 split；每个 Terminal Pane 有独立 runtime，Agent/Files/Diff 仍可保持单 Pane |

### 4.1 用户快捷键资源

`<ZETA_PROFILE_ROOT>/keybindings.json` 是严格 JSON 数组。未设置 `ZETA_PROFILE_ROOT` 时，Native 与
App Server 使用同一个 `<home>/.zeta` profile root；切换工作区不会切换用户快捷键或
Config/Session/Thread authority。每条规则必须包含 `key` 和 `command`；
`command: null` 表示 blocker。`mac`、`linux`、`win` 可以用平台专属按键覆盖 `key`，设为
`null` 表示该平台禁用该条规则。

```json
[
  {
    "key": "primary+k primary+b",
    "command": "workbench.action.toggleTabContainer",
    "when": "textInputFocus",
    "mac": "primary+k primary+b",
    "linux": "ctrl+k ctrl+b",
    "win": "ctrl+k ctrl+b"
  },
  {
    "key": "ctrl+v",
    "command": null,
    "when": "terminalFocus"
  }
]
```

| 字段或语法 | 当前支持 | 失败语义 |
| --- | --- | --- |
| modifier | `primary`、`ctrl`、`meta`、`alt`、`shift` | 重复 modifier，或 `primary` 与显式 `ctrl`/`meta` 组合时拒绝整份资源 |
| key identity | 逻辑键名或 `[PhysicalCode]` | 空键、单个 Chord 多键、超过四段时拒绝整份资源 |
| `when` 语法 | 省略表示 always；支持 `!`、`&&`、`||`、括号、`==`、`!=`、布尔值与字符串值 | 语法错误或未知 context key 时拒绝整份资源 |
| context key | `textInputFocus`、`terminalFocus`、`agentSurfaceVisible`、`terminalSurfaceVisible`、`tabContainerVisible`、`inspectorVisible`、`fileSearchVisible`、`composerRoute` | 布尔 key 参与真假组合；`composerRoute` 可与 `agent` 或 `shell` 比较 |
| command | Copy/Paste、Tab Container、Workspace Pane、Changes/Files、Refresh、File Search 与打开快捷键设置的当前稳定 command ID | 未知或尚未执行真实 transition 的 command ID 拒绝整份资源 |
| 更新 | 每秒比较资源内容，完整编译后替换 Builtin + User rule set | 文件过大、读取失败、JSON 错误、未知字段或任一规则无效时继续使用上一份完整规则 |

设置页使用深灰色 keycap 分别呈现 modifier 和按键字符；同一 Chord 紧密排列，多段 Chord 使用
更大间距。点击命令进入录制，暂停一秒后保存；Escape 取消录制或关闭浮层，窗口失焦取消录制。
用户规则优先于内建规则；同来源、同 priority 的冲突由资源中靠后的规则获胜，并在设置页显示
诊断。等待第二段按键时，底部提示已经输入的 keycap；错误按键会消费并退出 Chord，1.5 秒
超时、窗口失焦或 IME 事件也会退出。

当前可绑定的 command ID 为：

- `editor.action.clipboardCopyAction`、`editor.action.clipboardPasteAction`；
- `workbench.action.toggleTabContainer`、`workbench.action.toggleAuxiliaryBar`；
- `workbench.action.showAgentChanges`、`workbench.action.showAgentFiles`；
- `workbench.action.refreshAgentFiles`、`workbench.action.toggleAgentFileSearch`；
- `workbench.action.pickExecutionLocation`、`workbench.action.manageRemoteTunnels`；
- `workbench.action.openKeyboardShortcuts`；
- `workbench.action.splitTerminalHorizontal`、`workbench.action.splitTerminalVertical`、
  `workbench.action.focusNextPane`、`workbench.action.focusPreviousPane`、`workbench.action.closePane`。

当前“terminal core 纵切完成”指以下端到端路径已经同时成立：

- 当前 Session Tab 的默认 shell PTY 的 command submit、output、reply、resize 和 exit 都经过同一
  `TerminalSession`，非活动 Tab 的 PTY 事件按 terminal key 路由到自己的 `TerminalCore`；
- primary grid、alternate grid、BlockList、scrollback、reflow 与 command echo filtering 各有单一
  权威 owner；
- primary composer 与 alternate direct input、IME、应用鼠标、产品滚轮、selection 和
  clipboard 按当前 screen/mode 正确分流；
- 当前 state 能直接生成 scene、cursor、title 和可复制文本，不从 renderer 反推 terminal state。
- 简中、日文、韩文、组合音标、阿拉伯文字形与 Emoji 已覆盖 shaping/raster regression；terminal
  model 另覆盖 CJK cell width 和 extended grapheme ownership。
- 当前交互节点从同一份 bounds 生成 paint state、hit-test、cursor、focus navigation 与
  accessibility semantics；不存在另一份按坐标猜测控件身份的 hover 表。

Session Tab 到 PTY 的创建不在 native event loop 内同步等待：`TerminalWorkspace` 先分配
`TerminalSessionKey` 并记录 pending binding，`TerminalSession::spawn_async` 在后台 worker
完成 shell/PTY 启动后发送 `NativeEvent::TerminalReady`；UI 线程只在 ready event 到达时接管
已创建的 `TerminalSession`。ready 之前到达的 output/exit event 按 key 暂存，避免 PTY 启动和
事件投递的竞态把首批输出丢掉。切换到仍在创建中的 Session 会立即返回，完成顺序不决定最终
active tab，最终选择由最近一次 activation request 决定。

这一定义不把“完整 xterm compatibility”或跨重启 Session durability 伪装成本阶段能力；Terminal
Pane split 已在当前内存生命周期内接通，但 PaneGroup、Pane view state 与 Session binding 尚未跨
app 重启恢复。

当前 Top Bar 会显示左右两个入口。左侧展开后投影 App Server Session/Thread 列表，新增 Session
通过 worker 创建并在 snapshot 到达后加入同一垂直 TabList；切换 tab 重新订阅对应 Thread。右侧
workspace action 会把当前 Session PaneContainer 的 root Pane 切换为 Files 或恢复 Agent；Files/Changes
直接占据主工作区，文件编辑器打开时才使用右侧 Inspector Part。
Files 默认投影工作区
根目录，Search 使用模糊路径索引；Changes 把当前 Git 状态快照的全部文本变更作为
`MultiDiffEditorItem` 放入同一个滚动文档，每项保留独立的 DiffEditor viewport。没有文件或
变更时仍显示真实空态，不用 fixture 冒充 file tree、plan 或 diff。
任一真实 Session Tab 可通过右键打开真实浮层菜单；菜单保存逻辑 `TabInputKey`，Close 通过
App Server 的权威 Session Stop contract 执行并清理本地 Tab、Pane 和 runtime，Pin、Rename、Fork
仍必须通过对应的权威 Session contract 执行。

## 5. 尺寸语义

窗口 resize 的长期执行顺序是：

1. `NativeApp` 接收 physical extent 与 scale factor；
2. `NativeApp` 从 `NativeWindow` 读取窗口控件左右占位，product layout 计算 logical Top Bar、
   可选 Tab Container 与剩余 Workspace；`SplitViewLayout` 约束外部 effective width，
   `PaneGroupLayout` 再把 PaneGroup 的每个叶子和 owning-split Sash 投影为 Grid bounds，
   Titlebar action 在占位外另加 8px 组件间距；
3. 每个 terminal Pane 用自己的 output viewport 计算 rows/columns，固定底部 composer 不计入
   primary screen 的输出行数；alternate screen 用该 Pane 的完整 bounds；
4. `TerminalWorkspace` 逐 Pane 调用 `TerminalSession::resize`，同步对应 grid、PTY 和 view state，
   不再把一个活动 Session 的尺寸广播给所有 Pane；
5. host 从同一份 PaneTree geometry 与各 Pane runtime state 构造下一帧 scene。

Tab Container 默认宽度为 200 logical pixels，可在 160–480px 范围内调整，并始终为
Terminal Workspace 保留至少 240px。`TabContainerState` 保存 preferred width；窗口临时变窄
只改变当前 effective width，收起、重新展开或恢复窗口尺寸后仍使用用户首选值。Sash 拖动触发
同一条 Shell bounds → terminal grid → PTY resize 链路，pointer cell mapping、output 与
composer 共享调整后的 workspace。当前不增加 `PanelHeight`、任意区域拖拽或通用 Workbench
Part 系统。

文件编辑器 Inspector（`InspectorPart`）默认宽度为 520 logical pixels，可在 360–800px 范围内调整；工作区总宽度不足
760px 时，即使显隐状态为展开也会临时隐藏。右侧 Sash 拖动触发同一条 Shell bounds →
terminal grid → PTY resize 链路，不能绕过这条 Terminal Workspace 最小宽度约束。

窗口控件占位由 `zui::window` 的 private chrome adapter 统一拥有，不属于通用 `ActionBar` 样式。
macOS 当前使用集中且受测试的 70 logical pixel policy；由于 `winit` 尚无安全的 system button
geometry API，RTL 换边和未来 Windows controls overlay 仍是 adapter 扩展点，不能描述为当前
能力，也不能在 `titlebar::Titlebar` 再引入平台常量。实现契约见
[`zui` 开发文档](../zui/README.md)。

当前 Session Tab 的白色状态圆形只投影 native runtime 可确认的通用 `Active` 语义，尚未绘制
状态 SVG。Planning、Thinking、Editing 等 Agent 阶段尚无权威 Session projection，不能由 UI
从 terminal 输出推断；后续应由 App Server 提供 typed 状态，再把对应 SVG 映射到同一圆形容器。

## 6. 分阶段演进

### 6.1 近期结构整理

- 把当前 zsh 最小 bootstrap 演进为可协商版本的 shell integration，可靠产生 command
  start/end、cwd、exit status，并覆盖更多 shell；
- 把当前单行 composer 演进为支持换行、历史、补全和建议的 Block Input Editor；
- shell integration 提供 cwd 更新后，协调 input context toolbar 选中的工作区与 shell 内的
  `cd`；当前目录标签表示用户选择的 Files/Git 工作区，不能据此推断 PTY 内部 cwd；
- 为 Session action menu 接入 App Server 的 pin/close/rename/fork mutation，并为每个 tab 保留
  独立的完整本地 presentation state；
- file tree、tabs、chat 和 editor 接入时复用 `zui`：各组件只注册稳定 identity、
  父子关系、语义和 intent，业务模型仍由各自 owner 保存；
- 为现有 AccessKit adapter 增加 VoiceOver/Narrator/Orca smoke coverage，并扩展 text selection/edit action；
- 让 root layout 收敛为 Top Bar、Agent Terminal 会话流和单一临时检查对象；Session 列表改为按需打开，通用单轴约束继续委托给 `SplitViewLayout`；
- 保持 `zeta-ui-components` 通用、`zeta-workbench-ui` 只负责 Workbench presentation，不把 Session 或 terminal reducer 下沉到 UI crate。

### 6.2 Terminal Pane 分屏

Terminal Workspace 的分屏模型是 `TabInput → PaneContainer → PanePart → PaneGroup → PaneInput`。Session Tab 切换的是整个 PaneContainer；split 创建同一 PanePart 内的 sibling PaneGroup，不创建新的 Workbench Tab。

```text
TabInput(Session)
└─ PaneContainer
   └─ PanePart
      └─ split tree
         ├─ PaneGroup(PaneInput → PaneBinding(TerminalSessionKey))
         └─ Split {
              direction,
              ratio,
              first: PaneGroup,
              second: PaneGroup
            }
```

Terminal Pane Splitter 复用当前 `GridLayout`、`SplitViewLayout` 与 `Sash` geometry，但 PaneTree、PaneInput binding、active Pane、ratio、逐 Pane scroll/selection/input 和 Terminal Session lifecycle 仍只属于 Session PaneContainer owner。当前 command surface 包括 split horizontal、split vertical、focus next/previous Pane 和 close Pane；Sash 拖动会把受约束的尺寸写回对应 split ratio，最后一个 Pane 不能被关闭。当前每个 TerminalPaneInput 都绑定一个独立 TerminalSession runtime；Files/Diff root Pane 也使用同一套 PaneHost binding 机制，右侧 `zeta-workbench-ui::InspectorPartState` 仅保存文件编辑器 Inspector 的 shell 状态。
通用 presentation primitive 不等于可以注册任意产品区域的 Workbench Part/Sash 系统。

## 7. 明确不做什么

- 不构建任意区域注册系统；当前只保留 `zeta-workbench` 明确定义的 `TabPart`、`PaneContainer`、`PanePart` 与产品宿主映射；
- 不让每个视觉区域都具有可拖拽尺寸；
- 不把当前 immutable `GridLayout` 扩张成持有产品状态的 retained Workbench Grid，也不增加
  动态 Part 注册或任意区域 resize；
- 不用静态 session fixture 冒充真实 Block 输出；composer 必须提交到真实 PTY；
- 不把 terminal grid、PTY、scrollback 或 Session 生命周期放入 renderer；
- 不让一个视觉 Tab 同时承担 TabInput、PaneGroup、Pane runtime 和 Part topology 的全部职责；
- 不把当前静态 transcript/message fixture 描述成真实 terminal output；
- 不把当前基础 ANSI subset 描述成完整 xterm/VT compatibility；
- 不因 alternate screen 和基础 direct input 已接通，就声称完整支持 `vim`、`top` 等交互式 TUI；
- 不把当前 alternate-screen 1000/1002/1003/1006 支持描述成所有 screen/mouse protocol 均已完成；
- 不因参考现代终端产品而复制某个外部产品的内部模型或视觉细节。

## 8. 长期不变量

- 终端会话是窗口主体，chrome 和导航服务于终端而不是与终端平级；
- primary screen 始终由上方 BlockOutputViewport 与固定底部 CommandInputEditor 组成；
- product host 决定活动 Session、布局和事件路由，`zeta-workbench-ui` 只消费 Workbench presentation state；
- pointer、menu、shortcut 与后续 command palette 必须执行同一个 product command identity；
- hit-test、hover/press/capture、focus、键盘导航、cursor 和 accessibility semantics 必须共享
  同一个 `ElementId`，不能由各组件建立彼此不一致的状态表；
- terminal viewport、grid rows/columns 和 PTY size 必须来自同一条尺寸链路；
- Workbench Navigation 不拥有 Session lifecycle 或 durable output；
- 主工作区的每个 Tab 使用一个 PaneContainer；容器内每个可见 PaneGroup 投影一个 active PaneInput，`PaneHostScope::Tab` 标识其 owner；PaneInput 不拥有 Pane geometry、view state 或 runtime；
- PaneContainer/PanePart/PaneGroup topology 属于 `zeta-workbench`；通用 Grid/Split/Sash geometry 不拥有产品内容拓扑，文件编辑器 InspectorPart 也不创建第二套 Files/Diff PaneGroup；
- 不让 PaneGroup 为了复用而直接依赖 `zui` 的 UI identity、pointer capture 或 renderer；
- 当前实现、计划迁移和潜在能力必须在文档中保持明确分离。
