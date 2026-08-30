# `zeta-ui-components`

> 本 README 是 app 可复用 UI 组件的当前实现说明。Element、scene、inspection、font 与基础
> layout contract 由 [`zui`](../zui/README.md) 维护；跨 crate 渲染边界见
> [`docs/rendering-architecture.md`](../docs/rendering-architecture.md)，
> renderer contract 与私有 wgpu 实现见 [`zui`](../zui/README.md)；native 文本输入的跨 crate ownership 见
> [`docs/native-text-input.md`](../docs/native-text-input.md)；product icon system 见
> [`docs/icons.md`](../../docs/icons.md)；Native UI 编写和样式边界见
> [`docs/native-ui-authoring.md`](../docs/native-ui-authoring.md)。`Keycap` 的快捷键设置组合由
> [`zeta-settings`](../settings/README.md) 管，工作界面的组合键提示由 [`zeta-workbench`](../workbench/README.md) 管。

`zeta-ui-components` 基于 `zui` 提供 Button、Switch、Checkbox、ActionBar、Menu、ContextMenu、Dropdown、Picker、TabList、Keycap、Sash、Resizable、HorizontalScrollbar、VerticalScrollbar、ScrollView 和输入框等可复用组合控件。调用方必须直接从 `zui::ui` 引用框架类型；本 crate 不转发 `zui` API。
GPU pipeline、atlas、shader 和 surface 全部委托给 renderer backend。

## 1. 边界与依赖方向

| 能力 | 当前 owner | 状态 |
| --- | --- | --- |
| Component、Element、scene 与 inspection contract | [`zui`](../zui/README.md) | 委托；调用方直接依赖 `zui` |
| Text、symbolic-icon 与 icon-only button 的状态、样式和内部布局 | `zeta-ui-components::Button` | ✅ |
| 二态控件的共享交互状态，以及 Switch/Checkbox 的独立几何和样式 | `zeta-ui-components::{ToggleState,Switch,Checkbox}` | ✅；Checkbox 支持 unchecked/checked/mixed，值、输入路由和 accessibility 归 host |
| Button/Separator action 排列、绘制和可查询命中几何 | `zeta-ui-components::ActionBar` | ✅ |
| Tab surface 状态与横/纵 TabList 排列 | `zeta-ui-components::Tab` / `TabList` | ✅；product content 与 tabpanel 不在本 crate |
| NavBar 导航容器 | 计划中的 `zeta-ui-components` presentation composition | 尚未作为独立 public component 实现；若落地，只拥有方向、slot、滚动/overflow geometry，不拥有 product identity、active state 或 provider |
| 单轴 Pane 与递归 Grid layout | `zui::{SplitViewLayout,GridLayout}` | 委托；算法和 constraints 归 `zui` |
| Terminal/heterogeneous PaneGroup geometry projection | [`zeta-workbench`](../workbench/README.md) + `zui::GridLayout` | 委托；Workbench 消费 `PaneNode`，返回 leaf bounds 和 owning-split sash，不拥有 PaneInput 对应的具体 runtime |
| Workbench 的 Titlebar、Sessions、Main、Inspector 结构几何 | [`zeta-workbench`](../workbench/README.md) | 委托；本 crate 不拥有 Workbench 拓扑、TabInput state 或产品布局 |
| Workbench Titlebar、TabContainer、Toolbar、交互标识和界面状态 | [`zeta-workbench`](../workbench/README.md) | 委托；本 crate 只提供被其组合的通用控件 |
| Workbench 模型与 Pane binding | [`zeta-workbench`](../workbench/README.md) | 委托；本 crate 不拥有业务状态或 runtime |
| Workbench TabPart、TabGroup、TabInput 的逻辑身份、分组和 active selection | `zeta-workbench` + product host | 委托；模型不含方向和 `ElementId`，横向/纵向 Tab surface 与具体内容由 host 的 projection/scene 负责 |
| PaneInput 类型、逻辑 identity 与 Pane binding | `zeta-workbench` | 委托；具体 Terminal/Agent/Files/Diff/Settings runtime 仍由产品模块负责 |
| Settings、Files、SCM 和 Editor pane content | `zeta-settings` / `zeta-files` / `zeta-scm` / `zeta-editor` | 委托；各 feature/crate 负责自己的 view/presentation contract，domain state 与 adapter 由对应 host 保留，不能下沉到 `zeta-ui-components` |
| Sash 命中几何、hover/active presentation 与通用 resize gesture | `zeta-ui-components::{Sash,SashController,Resizable}` | ✅；pointer capture、identity、preferred size 与产品 resize transition 归 host |
| 单轴滚动条几何、绘制和交互映射 | `zeta-ui-components::{HorizontalScrollbar,VerticalScrollbar}` | ✅；两个方向由类型确定，共享标量 metrics、hover/active/fade、thumb drag 和 track paging；pointer capture 与调度归 host |
| 通用像素滚动状态、viewport 裁剪与内容坐标 | `zeta-ui-components::ScrollState` / `ScrollView` | ✅；`ScrollView` 组合启用方向对应的滚动条；平台事件路由、pointer capture 与产品内容归 host |
| 固定/可变高度列表测量、可见/overscan range、item bounds、hit-test 与虚拟化绘制 | `zeta-ui-components::VirtualListLayout` / `ListView` | ✅；固定高度直接计算，可变高度使用写时复制的平衡分块树，单项更新、按偏移定位和区间 splice 不重建无关分支，并支持稀疏展示覆盖和 item-relative scroll anchor；identity、selection、键盘语义与产品数据归 host |
| 虚拟 Tree 行、层级缩进、disclosure/content geometry 与命中 | `zeta-ui-components::TreeView` | ✅；普通文件树复用固定高度 ListView，展开式编辑器可接入保留的可变高度布局；hierarchy、稳定节点 identity、展开状态和 child loading 归 host |
| 锚点浮层布局、viewport 翻转/约束、通用外壳与浮层合成 | `zeta-ui-components::ContextView` / `zui::ui::UiScene::with_overlay` | ✅；显示生命周期、关闭和输入路由归 host |
| 菜单外壳、纵向菜单项、选择、命中、键盘导航和无障碍结构 | `zeta-ui-components::Menu` | ✅；产品 identity 由 host 提供，打开状态、关闭与 command 归 host |
| 锚定菜单的 viewport 翻转、约束与浮层组合 | `zeta-ui-components::ContextMenu` | ✅；组合 ContextView/Menu，不重复菜单内容和交互结构 |
| 无边框、无外层 padding 的锚定下拉项布局、可选 header 与默认选择 | `zeta-ui-components::Dropdown` | ✅；可滚动项复用 ListView 可见范围投影，选中 identity、header 内容、关闭与 command 归 host |
| 带搜索框的锚定候选列表、滚动、选择展示与 accessibility | `zeta-ui-components::Picker` | ✅；调用界面保留打开状态、查询、过滤、输入路由和选择结果执行 |
| Icon+text label 的内部布局 | `zeta-ui-components::IconLabel` | ✅ |
| 单个按键与多段快捷键的 keycap 几何和绘制 | `zeta-ui-components::Keycap` / `KeycapSequence` | ✅；按键语义与平台 label 归 caller |
| Renderer-independent icon identity、SVG definition 与 rendering mode | `zui::{Icon,IconDefinition}` | 委托 |
| 非 component 单行编辑基座与 shaping | `zui::{TextInput,TextInputLayoutEngine}` | 委托 |
| Input-box chrome、状态与 scene composition | `InputBox` | ✅ |
| 带左侧语义图标的单行搜索框 composition | `SearchBox` | ✅；过滤策略与输入状态仍归 host |
| Scene primitive、ordered batch、text layout 与 font catalog | `zui` | 委托；Markdown 语义归 `zeta-markdown` |
| shaping 与 renderer-compatible text measurement | `zui` → `cosmic-text` | 委托 |
| 后端无关 frame execution contract | `zui::render::Renderer` | ❌；由 framework 拥有 |
| GPU pipeline、atlas、shader、surface 与 present | private `zui::render/wgpu` | ❌ |
| Focus、input routing 与 accessibility semantics | `zui` + product host | ❌；Button 只消费 host 投影的 focused presentation |

依赖方向：

```text
product host → zeta-workbench → zui
product host → zeta-ui-components → zui
product → zui public facade → private framework modules

zeta-ui-components → zui::Icon
product catalog → zui::Icon
zui(macOS font catalog) → coretext-rs → CoreText

zeta-ui-components -X→ wgpu / Metal / Vulkan / winit
zeta-ui-components -X→ App Server / workspace / product state
```

`zeta-icons` 是可选的产品语义目录；组件只接收 caller 提供的 `zui::Icon`，因此本 crate
不需要依赖 app 的产品 artwork。若本 crate 开始拥有 scene primitive、font adapter、GPU API、窗口、workspace 或产品 reducer，
说明 ownership 已经漂移。基础 framework 的内部符号、验证与扩展点以 `zui/README.md` 为准。

导航和 Pane 组合的跨 crate contract 由 [`LAYOUT.md`](../LAYOUT.md) 维护。当前 `zeta-ui-components` 只提供 `Tab`/`TabList` 和其他 presentation component；Workbench 模型、结构布局、外壳 UI 和 binding 都位于 [`zeta-workbench`](../workbench/README.md)。`TabInput`、`PaneInput`、`PaneGroup`、active selection、provider/controller 和具体 tab/pane content 不得下沉到本 crate。

## 2. 文件与接口地图

| Symbol | 可见性 | 精确职责 |
| --- | --- | --- |
| `zui::ui::{Component,Element,ComputedElement,UiScene}` | external | Framework contract；调用方和本 crate 均直接依赖 `zui` |
| `components::button::{Button, ButtonState, ButtonSelection}` | public | 根据 host 投影的交互、disabled 与 selected 状态绘制 text、icon+text 或 icon-only button |
| `components::toggle::ToggleState` | public | 表达 Switch 与 Checkbox 共享的 resting、hovered、focused、pressed、disabled 交互状态 |
| `components::toggle::{Switch, SwitchSelection, SwitchStyle}` | public | 根据 on/off 状态和动画采样进度绘制 centered track 与 thumb；不拥有值、时钟或输入 |
| `components::toggle::{Checkbox, CheckboxSelection, CheckboxStyle}` | public | 绘制 unchecked/checked/mixed 方框与调用方提供的语义图标；不拥有值或输入 |
| `components::action_bar::ActionBar` | public | 在 caller bounds 内排列和绘制 action representation，并公开同源 visual/interactive bounds 与 hit-test |
| `components::action_bar::{ActionBarItem, ActionViewItem}` | public | `ActionBarItem` 组合可执行项与 Separator；`ActionViewItem` 表达单个动作的展示、状态和可由界面指定的主轴尺寸 |
| `components::action_bar::{ActionBarStyle, ActionBarSeparatorStyle, ActionBarOrientation}` | public | 定义 item size、gap、separator metrics、共享 Button style 与排列轴 |
| `components::tab_list::{Tab, TabState, TabSelection}` | public | 表达无产品 identity/content 的 Tab surface 交互与选中 presentation |
| `components::tab_list::{TabList, TabListStyle, TabListOrientation}` | public | 横向或纵向排列 Tab surface，拥有 item size/gap，并公开同源 tab bounds |
| `NavBar` 导航容器 | proposed composition boundary | 组合横向/纵向导航 shell 与 `TabList`；尚未形成 public API，具体方向见 [`LAYOUT.md`](../LAYOUT.md) |
| `components::tab_list::{TabStyle, TabBackgrounds}` | public | 定义 border、corner radii 及普通/selected 的状态背景 |
| `components::sash::{Sash, SashStyle, SashState}` | public | 从零面积 separator track 推导共享 drag target 与 feedback line，并绘制 host 投影的 hover/active 状态 |
| `components::resizable::{SashController, SashPointerPresence, Resizable}` | public | 延迟 hover、active presentation、deadline 与基于 `SplitViewResizeSnapshot` 的 drag-start-relative resize；不拥有 pointer capture、产品 identity 或 pane state |
| `components::context_view::ContextView` | public | 计算锚点附近的浮层 bounds/content bounds，并把通用外壳与调用方内容画入独立浮层 |
| `components::context_view::{ContextViewPlacement, ContextViewStyle}` | public | 分别定义锚定轴/方向/对齐/gap/viewport margin，以及 background/radius/padding；ContextView 天然无 border |
| `components::context_view::ContextViewLayout` | public | 暴露实际 bounds、content bounds 及翻转后的方向/对齐，供 host 注册命中和组合内容 |
| `components::context_view::{place_beside, align_with_anchor}` | private | 分别执行主轴侧边翻转/贴边与交叉轴对齐翻转/贴边；只计算 logical geometry，不读取窗口状态 |
| `components::scroll_view::{ScrollState, ScrollMetrics, ScrollCommand}` | public | 保存 logical-pixel offset，根据 viewport/content metrics 执行按像素、首尾和 ensure-visible transition |
| `components::scroll_view::{ScrollView, ScrollViewport}` | public | 约束有效 offset，裁剪调用方内容，并公开 translated content origin 与 visible content bounds |
| `components::scrollbar::{HorizontalScrollbar, VerticalScrollbar, ScrollbarMetrics}` | public | 以类型固定方向，消费单轴 viewport/content/offset，拥有 track/thumb 几何、绘制、命中、轨道翻页和拖动映射 |
| `components::scrollbar::{ScrollbarController, ScrollbarPresentation, ScrollbarStyle}` | public | 计算 hover/active 与 fade-in/hold/fade-out deadline 并选择语义颜色；不安装 timer 或持有平台 pointer capture |
| `components::list_view::{VirtualListLayout, ListView}` | public | 固定 extent 使用 O(1) geometry；可变 extent 使用平衡分块树，以 O(log n) 定位范围和更新单项高度，并通过 `splice_item_extents` 保留区间外的共享分支；稀疏高度覆盖不复制 retained index；组合 ScrollView 且只调用可见及 overscan item paint |
| `components::list_view::extent_tree::VariableExtentTree` | private | 以写时复制的平衡分块树保存叶子高度、子树 item count 与总高度，负责 O(log n) 查询/单点更新和 O(log n + k) 区间 splice；不拥有 item 内容或 identity |
| `components::list_view::extent_overrides::ListItemExtentOverrides` | private | 保存少量展示期高度覆盖及累计差值，用于动画而不改动或复制保留的高度树 |
| `components::list_view::ListScrollAnchor` | public | 记录 viewport 相对某个 item 起点的距离；调用方可按稳定 identity 解析新 index 后恢复滚动位置 |
| `components::list_view::ListContentPadding` | public | 显式表达列表 item sequence 前后的内容留白，不把 padding 混入首尾 item identity 或高度 |
| `components::list_view::ListItemLayout` | public | 为一个 projected index 暴露 translated item bounds；不携带产品 identity 或内容 |
| `components::tree_view::{TreeView, TreeViewStyle}` | public | 在 host-flattened visible node sequence 上组合 ListView；`new` 保留固定高度快速路径，`from_layout` 接收可变高度保留布局；拥有 depth indentation、disclosure/content geometry 与虚拟化 |
| `components::tree_view::{TreeItem, TreeItemExpansion, TreeItemLayout}` | public | 分别表达可见节点的 depth/Leaf/Collapsed/Expanded 结构状态，以及同源 row/disclosure/content bounds |
| `components::menu::{Menu, MenuItem, MenuIds}` | public | 绘制带柔和 BoxShadow 的无边框菜单外壳，以 host 提供的稳定 identity 建立菜单与菜单项的同一交互/无障碍树，并公开同源 item bounds/hit-test |
| `components::menu::{MenuSelection, MenuStyle}` | public | 默认选择首个 enabled item；定义 surface color、item size/style 和可选 header height，padding 固定为 2px、radius 固定为 4px |
| `components::context_menu::{ContextMenu, ContextMenuStyle}` | public | 组合 ContextView 与 Menu，只定义锚点 placement、viewport 翻转/约束和浮层合成，并把菜单几何与 header 组合入口原样公开给 host |
| `components::dropdown::{Dropdown, DropdownItem}` | public | 组合锚定浮层与纵向 label item；可滚动模式只为 visible/overscan range 构建 ActionBar，同时以 O(1) 固定高度几何公开全部 item/interactive bounds，供 host 保留键盘与 accessibility identity |
| `components::dropdown::{DropdownSelection, DropdownStyle}` | public | 默认选择首个 enabled item，并定义 borderless surface、item size/style、可选 header height、圆角和锚点 placement |
| `components::dropdown::DropdownScrollConfiguration` | public | 让 host 以 retained `ScrollState`、最大可见项数与 `ScrollViewStyle` 为 Dropdown 的 item region 启用独立滚动；header 保持固定 |
| `components::picker::{Picker, PickerIds, PickerItem, PickerStyle}` | public | 组合 Dropdown 与 SearchBox，统一锚定 picker 的浮层几何、候选行、滚动、选择展示和 accessibility；不拥有业务候选、查询状态或 action |
| `components::icon_label::{IconLabel, IconLabelStyle}` | public | 对齐 semantic icon 与单行 text；不选择产品 icon |
| `components::keycap::{Keycap, KeycapSequence, KeycapStyle}` | public | 绘制 caller 提供 label 的按键块，并区分同一 Chord 内按键间距与多段 Chord 间距；不解析快捷键或选择平台 label |
| `components::input_box::InputBox` | public | 组合 base layout 与 input-box chrome/style，并实现 `Component` |
| `components::search_box::{SearchBox, SearchBoxStyle}` | public | 复用 `InputBox` 的 chrome/text layout，在组件内拥有左侧 search icon 占位与几何 |
| `zeta-workbench::{TabContainerLayoutSpec,TabContainerLayout}` | external crate | 解析 Tab Container 与 main Part 的 split geometry；不进入本组件库 |
| `zeta-workbench::{WorkbenchLayoutSpec,WorkbenchLayout,WorkbenchPart}` | external crate | 组装 Titlebar、Sessions、Main、Inspector 的结构 geometry；不进入本组件库 |
| `zeta-workbench::PaneGroupLayout` | external crate | 将 `PaneNode` 投影为 leaf bounds 和 split sash；不进入本组件库 |
| `zeta-workbench::{TabPart,TabGroup,TabInput}` | external crate | 保存 Workbench 逻辑状态；不进入本组件库 |
| `zeta-workbench::{PanePart,PaneGroup,PaneInput}` | external crate | 保存 Pane 内容描述与递归 split topology；不进入本组件库 |

`Color` 的 RGB channel 是 sRGB、alpha 为 straight alpha。`Point`、`Size`、font size 与 line
height 都使用 logical UI pixels；只有 renderer backend 可以执行 logical-to-physical 转换。

## 3. 当前执行路径

```text
host
  → GridLayout::new
      → each GridNode::Split → SplitViewLayout::new
          → fit_sizes / distribute_delta
          → pane bounds + SplitViewSashLayout
      → recursive leaf bounds + GridSashLayout
  → Resizable::presentation
  → Sash::new
      → interaction_bounds (host hit registration)
      → Component::paint (hover/active feedback)
  → TextInput::apply / apply_composition (editing only)
  → CaretBlinkController::focus / activity / advance
  → InputBox::new
      → TextInputLayoutEngine::layout
  → UiScene::new
  → UiScene::draw_component
      → Component::element → ComponentElement::compute
      → ComputedElement → automatic InspectionNode
      → Component::paint_element(same ComputedElement)
          ├─ ContextView → anchored layout → overlay layer
          │   ├─ floating shell rect
          │   └─ caller content inside content-bounds clip
          ├─ Menu → shadow/menu surface + vertical ActionBar + interaction regions
          │   └─ MenuIds/MenuItem → Menu/MenuItem accessibility + vertical navigation
          ├─ ContextMenu → ContextView + Menu
          │   └─ anchored placement + viewport constraint + overlay composition
          ├─ Dropdown → ContextView + ListView projected range + vertical ActionBar
          │   └─ visible/overscan selected item → Button selection presentation
          ├─ Picker → Dropdown + SearchBox + host-owned item identities
          │   └─ anchored searchable candidates + Menu accessibility
          ├─ ScrollView → viewport clip + translated content geometry
          │   ├─ HorizontalScrollbar → horizontal track/thumb component
          │   └─ VerticalScrollbar → vertical track/thumb component
          ├─ ListView → ScrollView + fixed/variable-extent visible/overscan item projection
          ├─ TreeView → fixed/variable ListView + depth/disclosure/content item geometry
          ├─ ActionBar → item bounds
          │   ├─ ActionViewItem → Button → icon/text primitives
          │   └─ Separator → rect primitive
          ├─ TabList → Tab bounds → state/selection surface rect
          ├─ Button state/style → IconLabel → icon/text primitives
          ├─ Files row presentation → IconLabel → icon/text primitives
          ├─ ToggleState → Switch track/thumb 或 Checkbox box/mark
          └─ InputBox → rect/text primitives
  → UiScene::draw_rect / UiScene::draw_image / UiScene::draw_icon / UiScene::draw_text
  → UiScene::batches (layer order + exact cross-kind paint order)
  → zui::render::Renderer::render_scene
      → selected backend
```

`UiScene` 是本 crate 依赖的 `zui` 输出边界；组件只通过 `zui::Component` 写入它，不拥有 scene
协议。当前 wgpu 如何消费这些数据和替换规则由 [`zui`](../zui/README.md) 与
[`docs/rendering-architecture.md`](../docs/rendering-architecture.md) 统一定义。

## 4. 字体与 CoreText

字体实现由 `zui` 拥有，以下是组件必须遵守的 delegated contract。`FontCatalog::system` 在 macOS
调用 `coretext::FontCollection::available`，只返回 canonicalized
family-name snapshot。其他平台从 cosmic-text 的 font database 枚举 family。

当前文本测量路径在所有平台统一使用 cosmic-text；具体 backend 可以复用
`renderer_support` 提供的 font policy：

- cosmic-text 负责 shaping、line breaking、font matching 与 fallback；
- `TextLayoutEngine` 负责向组件返回 logical geometry；
- glyph raster、atlas 与 draw 归具体 renderer backend。

`zui::font::system::new_font_system` 是 `TextInputLayoutEngine` 与 backend bridge 的私有共同构造入口。
它加载系统字体、保留系统 locale，并在 macOS 排除 `GB18030 Bitmap`：该 face 能被 cosmic-text
选为 CJK fallback，但 swash 不能把其 bitmap glyph 栅格化，若不排除会出现 cell/caret 已推进而
字形透明。排除后 CJK 继续由可栅格化的系统 outline font fallback 承担。平台 filter 只处理
已验证的 backend incompatibility，不承担产品字体偏好。

因此“已经接入 CoreText”只表示 macOS 原生 font catalog 已接入，不能解读为 CTLine shaping、
CoreGraphics raster 或原生 typographic metrics 已经成为绘制事实。

## 5. 校验、失败与接入义务

- `ImageData::from_rgba8` 拒绝零 dimensions、乘法溢出与不匹配的 RGBA byte length；
- scene 保存 logical geometry 和显式 clip/layer，不执行 GPU capability 或 atlas 校验；
- 不换行的 code/input row 必须选择 `TextBlockWrap::None`，不能依赖 renderer 默认断行；
- primitive validation、raster、atlas、scale factor 与 surface failure 属于具体 renderer backend；
- backend-neutral outcome 与 error wrapping 属于 `zui::render::Renderer` contract。

Host 必须先根据当前 logical layout 构造完整 `UiScene`，再把同一帧的 physical extent 与 scale
factor 交给 renderer。不要预先把 text coordinates 乘 DPI，否则会发生二次缩放。
`Component` implementation 只能消费 caller 已投影好的 presentation state 并发出 primitives；
component bounds、hit registration、event dispatch 和 authoritative state transition 仍由 host
拥有。每个组件通过 `Component::element` 返回声明式 `ComponentElement`，并在
`Component::paint_element` 中消费框架计算的同一个 `ComputedElement`；`UiScene::draw_component`
只计算一次，再自动把 name、authored style、computed bounds、resolved padding、gap、gap regions、
radius 与 Element 声明位置写入检查快照。Caller 必须使用 `UiScene::draw_component`，
由它在当前 nested clip 内自动注册 inspection parent 并同步 paint；这不引入 retained component
instance、隐式 identity 或 lifecycle。`Button` 拥有 control 内部 padding 和 state-specific
background selection，并把 icon/text placement 委托给 `IconLabel`；`Button::icon` 保留不参与
绘制的 accessible label，供 host 的 ZUI accessibility adapter 使用。Files pane 同样把文件行
的 icon/text 几何委托给 `IconLabel`，但由 product host 先选择具体 semantic file icon。Caller
必须显式提供
`ButtonState`、`ButtonStyle`、bounds 与具体 content constructor。`ButtonState::Focused`
让 host 明确投影键盘 focus，不让组件自行监听键盘；selected presentation 通过
`ButtonSelection` 独立投影。

`SplitViewLayout` 是每帧重算的 immutable geometry，不是 retained widget。Host 保存每个
Pane 的 preferred size 与 visibility，传入 `SplitViewPane`；布局只计算当前 viewport 下的
effective size，因此窗口临时缩小不能覆盖用户首选尺寸。`SplitViewSashLayout` 的
`resize_snapshot` 固定一次拖动开始时相邻 Pane 的尺寸与约束；`Resizable` 保存 drag-start
pointer 与 snapshot，并始终以相对 delta 调用 `resize`，不能逐 pointer move 累加 delta。
`SashController` 负责 host 投影的 hover/active presentation 与 deadline；`Sash` 使用同一个
zero-area track 推导 interaction bounds 和 feedback bounds。Host 用前者注册 identity、cursor、
accessibility 与 pointer capture，再把 `Resizable` 返回的 pane size 写回产品状态。若 Host
另行计算命中区域或直接在产品 scene 中画反馈线，说明 Sash geometry ownership 已漂移。

`GridLayout` 在这层单轴能力上递归解析 caller-owned `GridNode`。每个 Split 的 identity、
orientation、children 与 preferred sizes 都来自 Host；布局只输出当前帧的 Leaf/Split bounds
和带 owning split identity 的 `GridSashLayout`。产品层必须把 resize 结果写回对应 Split，
并自行处理 add/remove/move、active Pane、Session binding 与序列化。若 `zeta-ui-components` 开始创建
Terminal Session、决定 split command 或跨帧修改树，说明 Grid ownership 已漂移。

`ScrollState` 是 logical-pixel offset primitive，不读取 `winit::MouseScrollDelta`。Host 把平台 wheel、键盘或 scrollbar drag 归一化为 `ScrollCommand`，再使用同一 `ScrollMetrics` 更新 retained state。`ScrollView::draw` 把调用方内容裁剪到 viewport，并通过 `ScrollViewport` 返回 translated content origin 和 content-coordinate visible bounds；启用横向或纵向滚动时分别组合 `HorizontalScrollbar` 或 `VerticalScrollbar`，滚动条用自己的 `ScrollbarMetrics` 计算同源 track/thumb paint 与交互几何。

`ListView` 在这层基座上为固定或可变高度数据提供 content height、可见/overscan range、item bounds、point hit-test 和 ensure-visible command；固定高度不分配逐项 geometry，可变高度使用写时复制的平衡分块树。`VirtualListLayout::update_item_extent` 以 O(log n) 更新一个已索引高度，`splice_item_extents` 以 O(log n + k) 替换连续区间并共享无关分支，按滚动偏移定位 item 为 O(log n)，`with_item_extent_overrides` 为少量动画项叠加稀疏高度而不复制整张索引，`ListScrollAnchor` 在高度变化或调用方按稳定 identity 解析新 index 后恢复 item-relative scroll position。固定高度布局插入相同高度仍为 O(1)；第一次写入不同高度时会一次性建立可变索引，因此确定包含编辑器的列表应从 `VirtualListLayout::variable` 开始。ListView 只请求可见及 overscan index，不拥有 item、identity、selection、键盘策略或产品数据。
`TreeView` 把 host 已按展开状态扁平化的 visible node sequence 映射为 ListView items，计算 depth indentation 与 disclosure/content bounds；普通文件树通过 `TreeView::new` 保留固定高度快速路径，承载 CodeEditor 或 DiffEditor 的展开节点通过 `TreeView::from_layout` 使用可变高度，宿主在展开或收起子树时同步 splice 节点序列和 `VirtualListLayout`。TreeView 不读取 children、不持有展开状态，也不生成产品节点 identity。
Terminal 从底部计数和输出增长锚定不属于通用 `ScrollState`；Native 的
`TerminalOutputScrollView` 只负责把该产品状态适配为 `ScrollView` 的顶部相对内容坐标。

`ActionBar` 接收 caller-provided outer bounds，用 `Element::row/column` 声明 `ActionViewItem`/Separator 的方向、间距和 item fixed size；zui element layout 生成的 `ComputedElement` 同时驱动 paint、`item_bounds`/hit-test 与自动 inspection。默认 item extent 来自调用界面提供的共享 style；需要不同尺寸的正式 Toolbar 可以通过 `ActionViewItem::with_main_axis_extent` 覆盖单项主轴尺寸。`ActionBar::item_bounds` 暴露 visual bounds；`ActionBar::interactive_item_bounds` 与 `ActionBar::hit_test` 复用相同几何并排除 disabled action 和 Separator。Host 必须把返回的 item index 映射到自己的 action identity 和命令。ActionBar 不持有 callback、命令、hover/focus state 或 product action registry。
`TabList` 同样用 Element pipeline 消费 caller-provided bounds、排列轴、Tab presentation 和 style。
`TabList::tab_bounds` 是 host 注册命中范围和组合 label/icon/status content 的唯一几何来源；
`TabList` 不持有 tab identity、activation、focus、accessibility、关闭动作或对应 tabpanel。
Session navigation 和后续 Editor tabs 可以复用同一 surface/排列 primitive，但各自保留内容
布局与 active panel 生命周期。
`ContextView::new` 接收同一逻辑坐标空间中的 viewport、anchor 和期望 content size。它先把 padding 加入外壳尺寸，再按 `ContextViewPlacement` 尝试首选侧和对齐；首选位置不适合时先翻转，仍无法完整放入时贴紧 inset viewport 并约束外壳和内容尺寸。`ContextView::draw` 把外壳与调用方 closure 发出的任意 primitive 放入同一个新浮层；该层不继承 host component 的 clip，因此可以越过锚点所在控件的边界，调用方内容再单独裁剪到 `content_bounds`。Host 必须使用同一 `ContextViewLayout` 注册命中区域，并自行管理 open/close、outside click、Escape、focus restoration 和锚定内容的领域交互；这些 retained lifecycle 不进入 scene component。当前 `ContextViewStyle` 不暴露 border：浮层天然无边框；若某个具体浮层需要描边，应由其内容组件拥有并绘制，不能改变 ContextView 的定位几何。

`Menu` 拥有无边框菜单外壳、与 macOS 原生弹出菜单同量级的单层下落阴影、2px padding、4px radius、纵向 `ActionBar`、选择展示和 item bounds/hit-test。`MenuIds` 与 `MenuItem` 接收 host 提供的稳定 identity，`Menu::compose` 用同一几何建立 `Menu`/`MenuItem` 无障碍节点、激活动作和纵向导航；host 只保留打开状态、关闭、焦点恢复与命令执行。`MenuStyle::with_header_height` 保留调用界面拥有的 header geometry，`Menu::{paint_with_header,draw_components_with_header}` 保证 header 与菜单项处于同一组件树。

`ContextMenu` 使用 `ContextView` 定位一个完整 `Menu`，只增加锚定、viewport 翻转/约束和浮层层级。标签菜单与 SCM 工具栏菜单直接提供 `MenuIds`/`MenuItem`，不再分别创建菜单项交互节点；标签菜单的重命名输入仍通过 header 入口组合，状态和输入语义保留在 Workbench。
`Dropdown` 是另一层 ContextView 组合：它使用无外层 padding 的浮层外壳，
用垂直 ActionBar 排列 label item，并默认选择第一个 enabled item。Host 可以用
`DropdownSelection::Item` 投影 hover/focus/pressed 对应的唯一选择，用 Dropdown 返回的同源
bounds 注册交互。`DropdownStyle::with_header_height` 与 `Dropdown::paint_with_header` 为
选择器保留 product-owned header，但不拥有查询或输入状态；Native 的工作区目录 picker 通过
这条契约组合 SearchBox。`Dropdown::new_scrollable` 进一步把 item region 组合进 ScrollView，
并以 `DropdownScrollConfiguration` 限制可见行数；header 不随内容滚动。selected identity、
open/close、平台滚轮路由和 command 不进入组件。
`TextInput` 拥有 local editing state 和 composition，但不拥有 focus、platform IME lifecycle、
component chrome 或产品 reducer。`InputBox::new` 使用 `TextInputLayoutEngine` 从 base state
生成 immutable layout，再组合 background、border、placeholder、selection、caret 和 preedit
presentation。`InputBoxState::Focused(CaretVisibility)` 显式投影 blink phase；组件不读取时钟。
IME 候选框定位读取同一个 `InputBox::caret_bounds`，即使 blink phase 隐藏也不能按字符数量另行
估算。

## 6. 测试与修改路径

```bash
just test zeta-ui-components
bazel test //app/ui-components:ui-unit-tests
```

`zui` 单元测试覆盖检查节点、Element、scene、font/text input 与 Split/Grid；`zeta-ui-components` 单元测试覆盖
组件裁剪与浮层合成、Sash 命中/反馈几何、SashController deadline 与 Resizable drag 结果，
ScrollState 的 axis clamp、绝对 offset、首尾和
ensure-visible transition，水平/垂直 Scrollbar 的独立类型、比例 thumb geometry、track paging、thumb drag 映射、hover/active 颜色与 fade deadline，ScrollView 的内容坐标、裁剪和 visibility policy，ListView 的固定/可变高度 visible/overscan range、平衡分块高度索引、O(log n) 单项更新、O(log n + k) 区间 splice、稀疏高度覆盖、scroll anchor、gap/padding、translated bounds、hit-test、ensure-visible 与 visible/overscan-only paint，TreeView 的固定/可变 item、depth/disclosure geometry、命中与 visible/overscan-only paint，
ContextView 的纵/横锚定、
翻转、对齐、viewport 约束、外壳/内容裁剪，Dropdown 的默认/显式选择、无外层 inset 与命中，
Menu 的柔和阴影、2px padding、4px radius、默认/显式选择、命中、纵向导航与无障碍父子关系，ContextMenu 的锚定浮层组合，ActionBar 排列与命中、
TabList 横纵排列与
surface 状态、按钮、图标标签和输入框的状态/样式/布局。GPU conversion、shader、atlas 与 input
validation 测试属于具体 backend crate。

- 扩展 text style/span、path、rect/clip、font 或 scene：修改 `zui`、backend 与其 canonical README，
  再检查本 crate 的组件和 `zeta-markdown` projection；
- 更换 shaping backend：保持 `zui::ui::UiScene` 平台无关，并更新字体语义与 backend bridge；
- 修改 DPI 转换、shader、atlas 或 glyph raster：只修改具体 backend，不向组件暴露实现类型；
- 新增拥有 box geometry 的组件：实现必需的 `Component::element` 与需要时的 `paint_element`，让
  `ComputedElement` 同时驱动 paint/hit-test/inspection，并在 sibling test 中验证 computed geometry；
  组合调用统一使用 `UiScene::draw_component`。非组件布局函数或拥有自定义 content closure 的
  composition surface 使用 `UiScene::with_element`；产品代码不直接调用 `with_inspection_node`。

## 7. 当前限制与扩展点

当前限制：

- framework scene、Element、text input 与 Split/Grid 的限制由 `zui/README.md` 维护；
- 全部标准组件以及 `ContextView` content-closure/overlay 入口都使用 Element/ComputedElement 自动
  检查路径。纯 layout projection 和 primitive helper 仍不制造节点，检查器也不会从裸 scene
  primitives 反推 ownership；
- `Button` 当前支持 resting、hovered、focused、pressed、disabled、selected、icon-only 与
  leading icon，但尚无独立 focus ring、trailing content 或额外的 component-specific accessibility action；
- `ActionBar` 当前支持 horizontal/vertical Button 与 Separator、同源 item bounds 和 hit-test，
  但尚无 roving focus、keyboard navigation、overflow 或 custom representation；
- `Dropdown` 当前只支持单列 label item、固定 item size、单项 selection 与非虚拟化滚动；icon、
  separator、submenu、typeahead、打开/关闭与 accessibility scope 尚无；
- `BoxShadow` 当前支持单个圆角矩形阴影的 color、offset、blur radius 与 spread；尚无 inset 或多重 shadow。`Menu` 暂不内建 icon、separator、submenu、typeahead 或超高菜单滚动；
- `TabList` 当前只拥有固定 item size、gap 和 Tab surface paint；custom content、动态宽度、
  overflow、close action、identity、interaction 与 tabpanel 均由 composed control/host 拥有；
- `NavBar` 当前尚未作为独立 component 存在；在出现稳定的横向 Titlebar TabList 消费者后，才评估是否把方向、slot 和 overflow/scroll geometry 收敛为 `zeta-ui-components` presentation contract；
- `ContextView` 不拥有 shadow、arrow/callout，也不拥有 outside click、Escape、focus
  restoration 或 accessibility scope；overflow shadow 由托管内容的 `PaintRect` 拥有，
  lifecycle/interaction 由 host 与 `zui` 组合；
- `HorizontalScrollbar` 与 `VerticalScrollbar` 当前提供 overlay scrollbar 的同源 paint/hit/track-page/thumb-drag geometry，`ScrollbarController` 提供 hover/active/fade presentation，`ScrollView` 只决定启用方向与 visibility policy；平台事件接线、pointer capture、滚动惯性、overscroll 和 scroll-specific accessibility action 仍由 host/dispatch 扩展；
- `ListView` 支持固定和可变 item extent，但不主动测量 item 内容；caller 把测量值写入 retained `VirtualListLayout`，并负责以稳定 identity 处理 reorder anchor。Selection、键盘导航与 focus anchor 仍由 composed control 拥有；
- `TreeView` 只消费 host-flattened visible nodes；宿主负责让节点 splice 与高度 splice 使用同一范围。异步 child loading、展开状态持久化、稳定节点 identity、selection、重命名、拖放和文件打开仍属于产品 Tree model/host；
- `Sash` 与 `Resizable` 当前提供 presentation geometry、hover timing 和通用 split snapshot
  drag 计算，但没有 pointer capture、keyboard resize 或 accessibility resize action；这些
  产品语义仍由 host 与 `zui` 组合；
- `InputBox` 消费显式 blink phase，但没有 mouse caret hit testing、drag selection 或
  disabled/read-only presentation；
- native 当前使用 `CaretBlinkController::default` 的 530ms half-period，尚未读取系统 caret
  blink preference 或 reduced-motion setting；
- 没有 widget tree、retained Grid state、focus、input routing、IME lifecycle 或
  accessibility；
- 每次测量重建 cosmic-text buffer，富文本 span 会在同一个 paragraph buffer 中 shaping，但尚无
  paragraph-level retained cache；
- `FontWeight` 与 `FontStyle` 只有常用 semantic variants；
- CoreText 只做 catalog，不做 shaping/raster，也没有 app font registration；

扩展点：出现第二个需要 secondary action/overflow 的真实消费者后，可以在 `ActionBar` 上组合独立 `ToolBar`；出现 Dropdown 或 custom representation 后，再扩展 `ActionViewItem` 的具体展示，不把产品 action identity、命令或 callback 下沉到通用控件。出现真实编辑器/终端消费者后，还可以分别增加可增长或可回收的图标图集、保留式段落缓存、平台字体注册、RGBA 图像/路径 primitives 与统一 display list。是否采用 CoreText shaping 应由跨平台 metrics/fallback 一致性测试决定，不是当前 API 的既定承诺。
