# `zeta-ui`

> 本 README 是可复用 native UI 组件的当前实现说明。Element、scene、inspection、font 与基础
> layout contract 由 [`zui`](../zui/README.md) 维护；跨 crate 渲染边界见
> [`docs/rendering-architecture.md`](../docs/rendering-architecture.md)，
> 后端接口与 wgpu 实现分别见 [`zeta-renderer`](../renderer/README.md) 和
> [`zeta-wgpu`](../wgpu/README.md)；native 文本输入的跨 crate ownership 见
> [`docs/native-text-input.md`](../docs/native-text-input.md)；product icon system 见
> [`docs/icons.md`](../../docs/icons.md)。`Keycap` 的快捷键产品组合由
> [`zeterm-keybinding-ui`](../keybinding-ui/README.md) 拥有。

`zeta-ui` 基于 `zui` 提供 presentation-only 的 Button、Switch、ActionBar、ContextMenu、Dropdown、
TabList、Keycap、Sash、ContextView、ScrollView 和输入框等组合控件。它暂时 `pub use zui::*`，
让现有产品代码可以渐进迁移 import；这只是兼容入口，不表示本 crate 拥有 framework contract。
GPU pipeline、atlas、shader 和 surface 全部委托给 renderer backend。

## 1. 边界与依赖方向

| 能力 | 当前 owner | 状态 |
| --- | --- | --- |
| Presentation-only component contract、Element、scene 与 inspection | [`zui`](../zui/README.md) | 委托；本 crate 兼容 re-export |
| Text、symbolic-icon 与 icon-only button 的状态、样式和内部布局 | `zeta-ui::Button` | ✅ |
| Switch track、thumb、on/off 与交互状态 presentation | `zeta-ui::Switch` | ✅；值、输入路由和 accessibility 归 host |
| Button/Separator action 排列、绘制和可查询命中几何 | `zeta-ui::ActionBar` | ✅ |
| Tab surface 状态与横/纵 TabList 排列 | `zeta-ui::Tab` / `TabList` | ✅；product content 与 tabpanel 不在本 crate |
| 单轴 Pane 与递归 Grid layout | `zui::{SplitViewLayout,GridLayout}` | 委托；产品 topology/state 归 host |
| Sash 命中几何与 hover/active 反馈线 | `zeta-ui::Sash` | ✅；pointer capture、identity 与 resize transition 归 host |
| 通用像素滚动状态、viewport 裁剪、内容坐标与滚动条交互 geometry | `zeta-ui::ScrollState` / `ScrollView` | ✅；包含 hover/active/fade presentation、thumb drag mapping 和 track paging；平台事件路由、pointer capture 与产品内容归 host |
| 固定/可变高度列表测量、可见/overscan range、item bounds、hit-test 与虚拟化绘制 | `zeta-ui::VirtualListLayout` / `ListView` | ✅；固定高度直接计算，可变高度使用 prefix index 二分定位；identity、selection、键盘语义与产品数据归 host |
| 虚拟 Tree 行、层级缩进、disclosure/content geometry 与命中 | `zeta-ui::TreeView` | ✅；复用固定高度 ListView；hierarchy、稳定节点 identity、展开状态和 child loading 归 host |
| 锚点浮层布局、viewport 翻转/约束、通用外壳与浮层合成 | `zeta-ui::ContextView` / `zui::UiScene::with_overlay` | ✅；显示生命周期、关闭和输入路由归 host |
| 柔和阴影、2px padding、4px radius、纵向 menu item geometry 与默认选择 | `zeta-ui::ContextMenu` | ✅；组合 ContextView/ActionBar，产品 identity、关闭与 command 归 host |
| 无边框、无外层 padding 的锚定下拉项布局、可选 header 与默认选择 | `zeta-ui::Dropdown` | ✅；可滚动项复用 ListView 可见范围投影，选中 identity、header 内容、关闭与 command 归 host |
| Icon+text label 的内部布局 | `zeta-ui::IconLabel` | ✅ |
| 单个按键与多段快捷键的 keycap 几何和绘制 | `zeta-ui::Keycap` / `KeycapSequence` | ✅；按键语义与平台 label 归 caller |
| Renderer-independent icon identity、SVG definition 与 rendering mode | `zeta-icon` | 委托 |
| 非 component 单行编辑基座与 shaping | `zui::{TextInput,TextInputLayoutEngine}` | 委托 |
| Input-box chrome、状态与 scene composition | `InputBox` | ✅ |
| 带左侧语义图标的单行搜索框 composition | `SearchBox` | ✅；过滤策略与输入状态仍归 host |
| Scene primitive、ordered batch、text layout 与 font catalog | `zui` | 委托；Markdown 语义归 `zeta-markdown` |
| shaping 与 renderer-compatible text measurement | `zui` → `cosmic-text` | 委托 |
| 后端无关 frame execution contract | `zeta-renderer::Renderer` | ❌ |
| GPU pipeline、atlas、shader、surface 与 present | `zeta-wgpu::WgpuRenderer` | ❌ |
| Focus、input routing 与 accessibility semantics | `zui` + product host | ❌；Button 只消费 host 投影的 focused presentation |

依赖方向：

```text
product host → zeta-ui
zeta-ui → zui
product host → zeta-renderer → zui
product host → selected backend → zeta-renderer + zui

zeta-ui → zui → zeta-icon
product catalog → zeta-icon
zui(macOS font catalog) → coretext-rs → CoreText

zeta-ui -X→ wgpu / Metal / Vulkan / zeta-winit
zeta-ui -X→ App Server / workspace / product state
```

`zeta-icons` 是可选的产品语义目录；组件只接收 caller 提供的 `zeta-icon::Icon`，因此本 crate
不需要依赖 zeterm 的产品 artwork。若本 crate 开始拥有 scene primitive、font adapter、GPU API、窗口、workspace 或产品 reducer，
说明 ownership 已经漂移。基础 framework 的内部符号、验证与扩展点以 `zui/README.md` 为准。

## 2. 文件与接口地图

| Symbol | 可见性 | 精确职责 |
| --- | --- | --- |
| `zui::{Component,Element,ComputedElement,UiScene}` | compatibility re-export | Framework contract；canonical API 与私有 ownership 见 `zui/README.md` |
| `components::button::{Button, ButtonState, ButtonSelection}` | public | 根据 host 投影的交互、disabled 与 selected 状态绘制 text、icon+text 或 icon-only button |
| `components::switch::{Switch, SwitchState, SwitchSelection}` | public | 根据 host 投影的交互、on/off 状态和动画采样进度绘制 centered track 与 thumb；不拥有值、时钟或输入 |
| `components::switch::{SwitchColors, SwitchStateColors, SwitchStyle}` | public | 定义 on/off、交互状态、track/thumb 几何、边框和圆角；动画规格属于 `zui` binding |
| `components::action_bar::ActionBar` | public | 在 caller bounds 内排列和绘制 action representation，并公开同源 visual/interactive bounds 与 hit-test |
| `components::action_bar::{ActionBarItem, ActionBarButton}` | public | 分别表达 Button/Separator representation 与单个 Button 的 presentation data；Button 可命名覆盖 main-axis extent |
| `components::action_bar::{ActionBarStyle, ActionBarSeparatorStyle, ActionBarOrientation}` | public | 定义 item size、gap、separator metrics、共享 Button style 与排列轴 |
| `components::tab_list::{Tab, TabState, TabSelection}` | public | 表达无产品 identity/content 的 Tab surface 交互与选中 presentation |
| `components::tab_list::{TabList, TabListStyle, TabListOrientation}` | public | 横向或纵向排列 Tab surface，拥有 item size/gap，并公开同源 tab bounds |
| `components::tab_list::{TabStyle, TabBackgrounds}` | public | 定义 border、corner radii 及普通/selected 的状态背景 |
| `components::sash::{Sash, SashStyle, SashState}` | public | 从零面积 separator track 推导共享 drag target 与 feedback line，并绘制 host 投影的 hover/active 状态 |
| `components::context_view::ContextView` | public | 计算锚点附近的浮层 bounds/content bounds，并把通用外壳与调用方内容画入独立浮层 |
| `components::context_view::{ContextViewPlacement, ContextViewStyle}` | public | 分别定义锚定轴/方向/对齐/gap/viewport margin，以及 background/radius/padding；ContextView 天然无 border |
| `components::context_view::ContextViewLayout` | public | 暴露实际 bounds、content bounds 及翻转后的方向/对齐，供 host 注册命中和组合内容 |
| `components::context_view::{place_beside, align_with_anchor}` | private | 分别执行主轴侧边翻转/贴边与交叉轴对齐翻转/贴边；只计算 logical geometry，不读取窗口状态 |
| `components::scroll_view::{ScrollState, ScrollMetrics, ScrollCommand}` | public | 保存 logical-pixel offset，根据 viewport/content metrics 执行按像素、首尾和 ensure-visible transition |
| `components::scroll_view::{ScrollView, ScrollViewport}` | public | 约束有效 offset，裁剪调用方内容，并公开 translated content origin 与 visible content bounds |
| `components::scroll_view::{ScrollbarLayout, ScrollbarHit, ScrollbarDrag}` | public | 以绘制所用的同一 track/thumb geometry 执行命中、轨道翻页和拖动到绝对 offset 的映射 |
| `components::scroll_view::{ScrollbarController, ScrollbarPresentation, ScrollbarStyle}` | public | 计算 hover/active 与 fade-in/hold/fade-out deadline，选择语义颜色并绘制 overlay scrollbar；不安装 timer 或持有平台 pointer capture |
| `components::list_view::{VirtualListLayout, ListView}` | public | 固定 extent 使用 O(1) geometry；可变 extent 保存 prefix index 并以 O(log n) 定位可见 range，组合 ScrollView 且只调用 projected item paint |
| `components::list_view::ListContentPadding` | public | 显式表达列表 item sequence 前后的内容留白，不把 padding 混入首尾 item identity 或高度 |
| `components::list_view::ListItemLayout` | public | 为一个 projected index 暴露 translated item bounds；不携带产品 identity 或内容 |
| `components::tree_view::{TreeView, TreeViewStyle}` | public | 在 host-flattened visible node sequence 上组合 ListView，拥有 row extent、depth indentation、disclosure/content geometry 与虚拟化 |
| `components::tree_view::{TreeItem, TreeItemExpansion, TreeItemLayout}` | public | 分别表达可见节点的 depth/Leaf/Collapsed/Expanded 结构状态，以及同源 row/disclosure/content bounds |
| `components::context_menu::{ContextMenu, ContextMenuItem}` | public | 组合 ContextView 与纵向 ActionBar，绘制带柔和 BoxShadow 的无边框 menu surface，公开同源 item bounds/hit-test，并允许 host 在保留的 header row 中绘制搜索等产品内容 |
| `components::context_menu::{ContextMenuSelection, ContextMenuStyle}` | public | 默认选择首个 enabled item；定义 surface color、item size/style、可选 header height 和锚点 placement，padding 固定为 2px、radius 固定为 4px |
| `components::dropdown::{Dropdown, DropdownItem}` | public | 组合锚定浮层与纵向 label item；可滚动模式只为 visible/overscan range 构建 ActionBar，同时以 O(1) 固定高度几何公开全部 item/interactive bounds，供 host 保留键盘与 accessibility identity |
| `components::dropdown::{DropdownSelection, DropdownStyle}` | public | 默认选择首个 enabled item，并定义 borderless surface、item size/style、可选 header height、圆角和锚点 placement |
| `components::dropdown::DropdownScrollConfiguration` | public | 让 host 以 retained `ScrollState`、最大可见项数与 `ScrollViewStyle` 为 Dropdown 的 item region 启用独立滚动；header 保持固定 |
| `components::icon_label::{IconLabel, IconLabelStyle}` | public | 对齐 semantic icon 与单行 text；不选择产品 icon |
| `components::keycap::{Keycap, KeycapSequence, KeycapStyle}` | public | 绘制 caller 提供 label 的按键块，并区分同一 Chord 内按键间距与多段 Chord 间距；不解析快捷键或选择平台 label |
| `components::input_box::InputBox` | public | 组合 base layout 与 input-box chrome/style，并实现 `Component` |
| `components::search_box::{SearchBox, SearchBoxStyle}` | public | 复用 `InputBox` 的 chrome/text layout，在组件内拥有左侧 search icon 占位与几何 |

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
          ├─ ContextMenu → ContextView + shadow/menu surface + vertical ActionBar
          │   └─ soft BoxShadow + 2px padding + 4px radius + selected MenuItem presentation
          ├─ Dropdown → ContextView + ListView projected range + vertical ActionBar
          │   └─ visible/overscan selected item → Button selection presentation
          ├─ ScrollView → viewport clip + translated content geometry + interactive scrollbar chrome
          ├─ ListView → ScrollView + fixed/variable-extent visible/overscan item projection
          ├─ TreeView → fixed ListView + depth/disclosure/content item projection
          ├─ ActionBar → item bounds
          │   ├─ ActionBarButton → Button → icon/text primitives
          │   └─ Separator → rect primitive
          ├─ TabList → Tab bounds → state/selection surface rect
          ├─ Button state/style → IconLabel → icon/text primitives
          ├─ Files row presentation → IconLabel → icon/text primitives
          ├─ zui::AnimationBinding → Switch progress → track/thumb rect primitives
          └─ InputBox → rect/text primitives
  → UiScene::draw_rect / UiScene::draw_image / UiScene::draw_icon / UiScene::draw_text
  → UiScene::batches (layer order + exact cross-kind paint order)
  → zeta_renderer::Renderer::render_scene
      → selected backend
```

`UiScene` 是本 crate 依赖的 `zui` 输出边界；组件只通过 `zui::Component` 写入它，不拥有 scene
协议。当前 wgpu 如何消费这些数据由
[`zeta-wgpu`](../wgpu/README.md) 说明；替换规则由
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
- backend-neutral outcome 与 error wrapping 属于 `zeta-renderer`。

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
绘制的 accessible label，供 host 的后续 accessibility adapter 使用。Files pane 同样把文件行
的 icon/text 几何委托给 `IconLabel`，但由 product host 先选择具体 semantic file icon。Caller
必须显式提供
`ButtonState`、`ButtonStyle`、bounds 与具体 content constructor。`ButtonState::Focused`
让 host 明确投影键盘 focus，不让组件自行监听键盘；selected presentation 通过
`ButtonSelection` 独立投影。

`SplitViewLayout` 是每帧重算的 immutable geometry，不是 retained widget。Host 保存每个
Pane 的 preferred size 与 visibility，传入 `SplitViewPane`；布局只计算当前 viewport 下的
effective size，因此窗口临时缩小不能覆盖用户首选尺寸。`SplitViewSashLayout` 的
`resize_snapshot` 固定一次拖动开始时相邻 Pane 的尺寸与约束，Host 必须始终用相对 drag-start
的 delta 调用 `resize`，不能逐 pointer move 累加 delta。`Sash` 使用同一个 zero-area track
推导 interaction bounds 和 feedback bounds；Host 用前者注册 identity、cursor、
accessibility 与 pointer capture，再把 hover/active 投影为 `SashState`。若 Host 另行计算
命中区域或直接在产品 scene 中画反馈线，说明 Sash geometry ownership 已漂移。

`GridLayout` 在这层单轴能力上递归解析 caller-owned `GridNode`。每个 Split 的 identity、
orientation、children 与 preferred sizes 都来自 Host；布局只输出当前帧的 Leaf/Split bounds
和带 owning split identity 的 `GridSashLayout`。产品层必须把 resize 结果写回对应 Split，
并自行处理 add/remove/move、active Pane、Session binding 与序列化。若 `zeta-ui` 开始创建
Terminal Session、决定 split command 或跨帧修改树，说明 Grid ownership 已漂移。

`ScrollState` 是 logical-pixel offset primitive，不读取 `winit::MouseScrollDelta`。
Host 把平台 wheel、键盘或 scrollbar drag 归一化为 `ScrollCommand`，再使用同一
`ScrollMetrics` 更新 retained state。`ScrollView::draw` 把调用方内容裁剪到 viewport，并通过
`ScrollViewport` 返回 translated content origin 和 content-coordinate visible bounds；
`ScrollbarLayout` 与最终 track/thumb paint 使用同一 geometry。`ListView` 在这层基座上为固定
或可变高度数据提供 content height、可见/overscan range、item bounds、point hit-test 和
ensure-visible command；固定高度不分配逐项 geometry，可变高度保存 extent prefix index，
通过二分查找定位 viewport。它只向 caller 请求 projected index，不拥有 item。高度重新测量、
scroll anchoring、focus reveal policy 和交互 identity 仍归 composed control 或产品 host。
`TreeView` 再把 host 已按展开状态扁平化的 visible node sequence 映射为固定高度 ListView
items，计算 depth indentation 与 disclosure/content bounds；它不读取 children、不持有展开状态，
也不生成产品节点 identity。
Terminal 从底部计数和输出增长锚定不属于通用 `ScrollState`；Native 的
`TerminalOutputScrollView` 只负责把该产品状态适配为 `ScrollView` 的顶部相对内容坐标。

`ActionBar` 接收 caller-provided outer bounds，用 `Element::row/column` 声明 Button/Separator 的
方向、间距和 item fixed size；zui element layout 生成的 `ComputedElement` 同时驱动 paint、
`item_bounds`/hit-test 与自动 inspection。默认 item extent 来自共享 style；label 长度不同的正式 Toolbar 可以通过
`ActionBarButton::with_main_axis_extent` 覆盖单项主轴尺寸。`ActionBar::item_bounds` 暴露 visual
bounds；`ActionBar::interactive_item_bounds` 与
`ActionBar::hit_test` 复用相同几何并排除 disabled Button 和 Separator。Host 必须把返回的 item
index 映射到自己的 action identity 和命令。ActionBar 不持有 callback、命令、hover/focus
state 或 product action registry。
`TabList` 同样用 Element pipeline 消费 caller-provided bounds、排列轴、Tab presentation 和 style。
`TabList::tab_bounds` 是 host 注册命中范围和组合 label/icon/status content 的唯一几何来源；
`TabList` 不持有 tab identity、activation、focus、accessibility、关闭动作或对应 tabpanel。
Session navigation 和后续 Editor tabs 可以复用同一 surface/排列 primitive，但各自保留内容
布局与 active panel 生命周期。
`ContextView::new` 接收同一 logical coordinate space 中的 viewport、anchor 和期望 content
size。它先把 padding 加入外壳尺寸，再按 `ContextViewPlacement` 尝试首选侧和对齐；
首选位置不适合时先翻转，仍无法完整放入时贴紧 inset viewport 并约束外壳和内容尺寸。
`ContextView::draw` 把外壳与调用方 closure 发出的任意 primitive 放入同一个新浮层；该层不继承
host component 的 clip，因此可以越过锚点所在控件的边界，调用方内容再单独裁剪到
`content_bounds`。Host 必须使用同一 `ContextViewLayout`
注册命中区域，并自行管理 open/close、outside click、Escape、focus restoration 和 anchored
content 的领域交互；这些 retained lifecycle 不进入 scene component。当前
`ContextViewStyle` 不暴露 border：浮层天然无边框；若某个具体浮层需要描边，应由其内容组件
拥有并绘制，不能改变 ContextView 的定位几何。`ContextMenu` 在 ContextView 上建立通用菜单
基座：`PaintRect::with_shadow` 让 renderer 在本体前生成扩展 shadow quad，fragment shader
根据圆角矩形 signed distance 近似 Gaussian coverage，以 surface 边缘作为 50% coverage
中点；ContextMenu 再组合低透明度的 ambient shadow 与向下偏移的 key shadow，避免单层黑色
光晕。内部无边框 menu surface 固定使用 2px padding 与 4px radius，再由纵向 ActionBar
排列 label item。Host 通过
`ContextMenuSelection::Item` 投影唯一选中项，并使用 ContextMenu 返回的同源 bounds 注册交互。
`ContextMenuStyle::with_header_height` 只保留 product-owned header geometry；
`ContextMenu::paint_with_header` 保证 header 与菜单项绘制在同一 overlay。搜索状态、输入语义、
过滤和焦点仍由 host 保存，ContextMenu 不依赖 `TextInput` 或业务数据。Native 的分支菜单通过
这条 header contract 组合产品搜索。
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
cargo test --manifest-path Cargo.toml -p zui -p zeta-ui
bazel test //zeterm/ui:ui-unit-tests
```

`zui` 单元测试覆盖检查节点、Element、scene、font/text input 与 Split/Grid；`zeta-ui` 单元测试覆盖
组件裁剪与浮层合成、Sash 命中/反馈几何与状态绘制，ScrollState 的 axis clamp、绝对 offset、首尾和
ensure-visible transition，ScrollView 的内容坐标、裁剪、visibility policy、比例 thumb geometry、
track paging、thumb drag 映射、hover/active 颜色与 fade deadline，ListView 的固定/可变高度
visible/overscan range、prefix geometry、gap/padding、translated bounds、hit-test、ensure-visible
与 projected-only paint，TreeView 的 depth/disclosure geometry、命中与 projected-only paint，
ContextView 的纵/横锚定、
翻转、对齐、viewport 约束、外壳/内容裁剪，Dropdown 的默认/显式选择、无外层 inset 与命中，
ContextMenu 的柔和阴影、2px padding、4px radius、默认/显式选择与命中，ActionBar 排列与命中、
TabList 横纵排列与
surface 状态、按钮、图标标签和输入框的状态/样式/布局。GPU conversion、shader、atlas 与 input
validation 测试属于具体 backend crate。

- 扩展 text style/span、path、rect/clip、font 或 scene：修改 `zui`、backend 与其 canonical README，
  再检查本 crate 的组件和 `zeta-markdown` projection；
- 更换 shaping backend：保持 `zui::UiScene` 平台无关，并更新字体语义与 backend bridge；
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
  leading icon，但尚无独立 focus ring、trailing content 或真实 accessibility adapter；
- `ActionBar` 当前支持 horizontal/vertical Button 与 Separator、同源 item bounds 和 hit-test，
  但尚无 roving focus、keyboard navigation、overflow 或 custom representation；
- `Dropdown` 当前只支持单列 label item、固定 item size、单项 selection 与非虚拟化滚动；icon、
  separator、submenu、typeahead、打开/关闭与 accessibility scope 尚无；
- `BoxShadow` 当前支持单个 rounded-rect shadow 的 color、offset 与 blur radius；尚无 spread、
  inset 或多重 shadow。ContextMenu 也暂不内建 icon、separator、submenu、typeahead 或超高菜单滚动；
- `TabList` 当前只拥有固定 item size、gap 和 Tab surface paint；custom content、动态宽度、
  overflow、close action、identity、interaction 与 tabpanel 均由 composed control/host 拥有；
- `ContextView` 不拥有 shadow、arrow/callout，也不拥有 outside click、Escape、focus
  restoration 或 accessibility scope；overflow shadow 由托管内容的 `PaintRect` 拥有，
  lifecycle/interaction 由 host 与 `zui` 组合；
- `ScrollView` 当前提供 overlay scrollbar 的同源 paint/hit/track-page/thumb-drag geometry，
  `ScrollbarController` 提供 hover/active/fade presentation；平台事件接线、pointer capture、
  滚动惯性、overscroll 和 accessibility adapter 仍由 host/dispatch 扩展；
- `ListView` 支持固定和可变 item extent，但不测量 item 内容；caller 必须在高度改变时重建
  layout。稳定 item identity、selection、键盘导航、滚动锚定、focus anchor 与 Tree
  flatten/expand semantics 仍由 composed control 或后续专用组件拥有；
- `TreeView` 只消费 host-flattened visible nodes；异步 child loading、展开状态持久化、稳定节点
  identity、selection、重命名、拖放和文件打开仍属于产品 Tree model/host；
- `Sash` 当前只拥有 presentation geometry，没有 pointer capture、keyboard resize 或
  accessibility adapter；这些交互由 host 与 `zui` 组合；
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

扩展点：出现第二个需要 secondary action/overflow 的真实消费者后，可以在 `ActionBar` 上组合
独立 `ToolBar`；出现 Dropdown 或 custom representation 后，再增加对应 `ActionBarItem`
variant，不为单一 Button 增加纯转发 wrapper。出现真实编辑器/终端消费者后，还可以分别增加
可增长或可回收的图标图集、保留式段落缓存、平台字体注册、RGBA 图像/路径
primitives 与统一 display list。是否采用 CoreText shaping 应由跨平台 metrics/fallback
一致性测试决定，不是当前 API 的既定承诺。
