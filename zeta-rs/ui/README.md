# `zeta-ui`

> 本 README 是 native UI scene、paint primitives、font catalog 与 GPU 绘制路径的当前实现说明。
> Surface 生命周期与 presentation 的 canonical 文档在
> [`zeta-wgpu`](../wgpu/README.md)；native 文本输入的跨 crate ownership 见
> [`docs/native-text-input.md`](../../docs/native-text-input.md)；product icon system 见
> [`docs/icons.md`](../../docs/icons.md)。

`zeta-ui` 定义与窗口系统无关的 immutable frame scene、单轴 SplitView 几何，并提供
presentation-only 的 Button、ActionBar、ContextMenu、Dropdown、TabList、Sash、ContextView、
ScrollView 和输入框等组合控件；底层
实现分层的 instanced rect、decoded RGBA image、symbolic/multicolor SVG icon 与 text GPU
pipeline。SVG icon 由 `resvg`
栅格为按 physical size 缓存的 alpha mask，`glyphon` 完成 shaping、glyph cache、texture
atlas 和 text draw。macOS 的系统字体目录通过 CoreText 读取；CoreText 当前不承担文本
shaping 或 glyph rasterization。

## 1. 边界与依赖方向

| 能力 | 当前 owner | 状态 |
| --- | --- | --- |
| Presentation-only component contract 与 scene composition | `zeta-ui::Component` / `UiScene` | ✅ |
| Text、symbolic-icon 与 icon-only button 的状态、样式和内部布局 | `zeta-ui::Button` | ✅ |
| Button/Separator action 排列、绘制和可查询命中几何 | `zeta-ui::ActionBar` | ✅ |
| Tab surface 状态与横/纵 TabList 排列 | `zeta-ui::Tab` / `TabList` | ✅；product content 与 tabpanel 不在本 crate |
| 单轴 Pane 约束分配、可见几何、Sash track 与拖动快照 | `zeta-ui::SplitViewLayout` | ✅；跨帧首选尺寸与显隐状态归 host |
| 递归 Split 拓扑输入、Leaf/Split bounds 与 Sash 路由 | `zeta-ui::GridLayout` | ✅；树、稳定 ID、产品绑定与拓扑变更归 host |
| Sash 命中几何与 hover/active 反馈线 | `zeta-ui::Sash` | ✅；pointer capture、identity 与 resize transition 归 host |
| 通用像素滚动状态、viewport 裁剪、内容坐标与滚动条交互 geometry | `zeta-ui::ScrollState` / `ScrollView` | ✅；包含 hover/active/fade presentation、thumb drag mapping 和 track paging；平台事件路由、pointer capture、产品内容与 virtualization 归 host |
| 锚点浮层布局、viewport 翻转/约束、通用外壳与浮层合成 | `zeta-ui::ContextView` / `UiScene::with_overlay` | ✅；显示生命周期、关闭和输入路由归 host |
| 柔和阴影、2px padding、4px radius、纵向 menu item geometry 与默认选择 | `zeta-ui::ContextMenu` | ✅；组合 ContextView/ActionBar，产品 identity、关闭与 command 归 host |
| 无边框、无外层 padding 的锚定下拉项布局与默认选择 | `zeta-ui::Dropdown` | ✅；组合 ContextView/ActionBar，选中 identity、关闭与 command 归 host |
| Icon+text label 的内部布局 | `zeta-ui::IconLabel` | ✅ |
| Semantic icon identity、SVG definition 与 rendering mode | `zeta-icons` | 委托 |
| 非 component 单行编辑基座与 shaping | `TextInput` / `TextInputLayoutEngine` | ✅ |
| Input-box chrome、状态与 scene composition | `InputBox` | ✅ |
| 带左侧语义图标的单行搜索框 composition | `SearchBox` | ✅；过滤策略与输入状态仍归 host |
| Rect、decoded RGBA image、SVG icon、clip 与文本 scene | `zeta-ui::UiScene` | ✅ |
| 同段富文本 span、renderer-compatible 测量与 span/UTF-8 range visual fragments | `TextSpan` / `TextLayoutEngine` / `TextLayout` | ✅；Markdown 语义归 `zeta-markdown` |
| 系统 font-family 枚举 | `zeta-ui::FontCatalog` | ✅ |
| macOS CoreText font catalog adapter | `font::platform::macos` | ✅ |
| shaping、fallback、glyph raster/cache | `glyphon` / `cosmic-text` / `swash` | 委托 |
| Instanced rect、RGBA image/icon atlas 与 glyph text draw pipeline | `zeta-ui::UiRenderer` | ✅ |
| Surface acquire/configure/present | `zeta-wgpu::WgpuRenderer` | ❌ |
| Focus、input routing 与 accessibility semantics | `zeta-ui-dispatch` + product host | ❌；Button 只消费 host 投影的 focused presentation |

依赖方向：

```text
product host → zeta-ui
product host → zeta-wgpu → zeta-ui

zeta-ui → glyphon → cosmic-text / swash
        → resvg → usvg / tiny-skia
        → zeta-icons
zeta-ui(macOS font catalog) → coretext-rs → CoreText

zeta-ui -X→ zeta-winit
zeta-ui -X→ App Server / workspace / product state
```

如果本 crate 开始创建窗口、acquire surface、读取 workspace 或持有产品 reducer，说明 ownership
已经漂移。平台字体 adapter 可以提供字体发现或注册能力，但不应让 `UiScene` 暴露 CoreText、
DirectWrite 或 fontconfig 类型。

## 2. 文件与接口地图

| Symbol | 可见性 | 精确职责 |
| --- | --- | --- |
| `components::component::Component` | public | 把 caller-provided presentation state 转成 scene primitives；不拥有 input 或 lifecycle |
| `components::button::{Button, ButtonState, ButtonSelection}` | public | 根据 host 投影的交互、disabled 与 selected 状态绘制 text、icon+text 或 icon-only button |
| `components::action_bar::ActionBar` | public | 在 caller bounds 内排列和绘制 action representation，并公开同源 visual/interactive bounds 与 hit-test |
| `components::action_bar::{ActionBarItem, ActionBarButton}` | public | 分别表达 Button/Separator representation 与单个 Button 的 presentation data；Button 可命名覆盖 main-axis extent |
| `components::action_bar::{ActionBarStyle, ActionBarSeparatorStyle, ActionBarOrientation}` | public | 定义 item size、gap、separator metrics、共享 Button style 与排列轴 |
| `components::tab_list::{Tab, TabState, TabSelection}` | public | 表达无产品 identity/content 的 Tab surface 交互与选中 presentation |
| `components::tab_list::{TabList, TabListStyle, TabListOrientation}` | public | 横向或纵向排列 Tab surface，拥有 item size/gap，并公开同源 tab bounds |
| `components::tab_list::{TabStyle, TabBackgrounds}` | public | 定义 border、corner radii 及普通/selected 的状态背景 |
| `layout::split_view::{SplitViewLayout, SplitViewPane}` | public | 根据 caller 首选尺寸、min/max、priority 与 visibility 计算单轴 Pane geometry；不持有跨帧状态 |
| `layout::split_view::{SplitViewSashLayout, SplitViewResizeSnapshot}` | public | 从同一次布局公开 separator track 与 drag-start 相邻约束；host 按 pointer delta 取得无累积误差的新尺寸 |
| `layout::split_view::{fit_sizes, distribute_delta}` | private | 先 clamp Pane，再按 high → normal → low 顺序分配容器尺寸差；不选择产品 Part priority |
| `layout::grid::{GridNode, GridPane, GridLayout}` | public | 接收 caller-owned 递归树，对每个 Split 复用 `SplitViewLayout`，输出本帧可见 Leaf/Split/Sash geometry |
| `layout::grid::{GridLeafLayout, GridSplitLayout, GridSashLayout}` | public | 保留 caller identity，使 host 能把 leaf bounds 和 resize snapshot 路由回对应产品 Pane/Split |
| `layout::grid::{validate_unique_identities, validate_node}` | private | 拒绝重复 Leaf/Split identity，避免产品层无法判定 geometry 或 resize 归属 |
| `components::sash::{Sash, SashStyle, SashState}` | public | 从零面积 separator track 推导共享 drag target 与 feedback line，并绘制 host 投影的 hover/active 状态 |
| `components::context_view::ContextView` | public | 计算锚点附近的浮层 bounds/content bounds，并把通用外壳与调用方内容画入独立浮层 |
| `components::context_view::{ContextViewPlacement, ContextViewStyle}` | public | 分别定义锚定轴/方向/对齐/gap/viewport margin，以及 background/radius/padding；ContextView 天然无 border |
| `components::context_view::ContextViewLayout` | public | 暴露实际 bounds、content bounds 及翻转后的方向/对齐，供 host 注册命中和组合内容 |
| `components::context_view::{place_beside, align_with_anchor}` | private | 分别执行主轴侧边翻转/贴边与交叉轴对齐翻转/贴边；只计算 logical geometry，不读取窗口状态 |
| `components::scroll_view::{ScrollState, ScrollMetrics, ScrollCommand}` | public | 保存 logical-pixel offset，根据 viewport/content metrics 执行按像素、首尾和 ensure-visible transition |
| `components::scroll_view::{ScrollView, ScrollViewport}` | public | 约束有效 offset，裁剪调用方内容，并公开 translated content origin 与 visible content bounds |
| `components::scroll_view::{ScrollbarLayout, ScrollbarHit, ScrollbarDrag}` | public | 以绘制所用的同一 track/thumb geometry 执行命中、轨道翻页和拖动到绝对 offset 的映射 |
| `components::scroll_view::{ScrollbarController, ScrollbarPresentation, ScrollbarStyle}` | public | 计算 hover/active 与 fade-in/hold/fade-out deadline，选择语义颜色并绘制 overlay scrollbar；不安装 timer 或持有平台 pointer capture |
| `components::context_menu::{ContextMenu, ContextMenuItem}` | public | 组合 ContextView 与纵向 ActionBar，绘制带柔和 BoxShadow 的无边框 menu surface，并公开同源 item bounds/hit-test |
| `components::context_menu::{ContextMenuSelection, ContextMenuStyle}` | public | 默认选择首个 enabled item；定义 surface color、item size/style 和锚点 placement，padding 固定为 2px、radius 固定为 4px |
| `components::dropdown::{Dropdown, DropdownItem}` | public | 组合锚定浮层与纵向 label item，公开同源 item/interactive bounds、hit-test 和当前 selected index |
| `components::dropdown::{DropdownSelection, DropdownStyle}` | public | 默认选择首个 enabled item，并定义 borderless surface、item size/style、圆角和锚点 placement |
| `components::icon_label::{IconLabel, IconLabelStyle}` | public | 对齐 semantic icon 与单行 text；不选择产品 icon |
| `text_input::model::TextInput` | public | 拥有 single-line text、selection、grapheme editing 与 composition；`selected_text` 投影非空 committed selection，不实现 `Component` |
| `text_input::caret_blink::CaretBlinkController` | public | 计算 focus/activity/deadline 驱动的 caret visibility；不创建 timer |
| `text_input::layout::TextInputLayoutEngine` | public | 使用 cosmic-text 生成单行 text、selection、caret、preedit 几何 |
| `text_input::layout::DisplayProjection` | private | 把 committed text 与临时 preedit 投影为单次 shaping 输入 |
| `components::input_box::InputBox` | public | 组合 base layout 与 input-box chrome/style，并实现 `Component` |
| `components::search_box::{SearchBox, SearchBoxStyle}` | public | 复用 `InputBox` 的 chrome/text layout，在组件内拥有左侧 search icon 占位与几何 |
| `geometry::{Rect, Edges, CornerRadii}` | public | 使用 logical UI pixels 表达几何与 visual metrics |
| `paint::{PaintRect, BoxShadow, Border, Color}` | public | 表达 fill、柔和 rounded-rect shadow、per-edge border、rounded corners 与 sRGB color |
| `icon::PaintIcon` | public | 把 `zeta-icons::Icon` 绑定到 logical bounds、caller tint 与 clip |
| `image::{ImageData, ImageId, PaintImage}` | public | 校验 immutable RGBA8 sRGB pixels，以稳定 identity 绑定 logical bounds 与 clip |
| `scene::UiScene` | public | 保存一帧背景、分层 rect/image/icon/text、构建时的 nested clip 和显式 overlay composition |
| `scene::{TextBlock, TextSpan, TextStyle}` | public | 使用 logical UI pixels 表达普通/同段富文本、bounds 和样式；不拥有 Markdown 语义 |
| `text_layout::{TextLayoutEngine, TextLayoutWidth, TextLayout}` | public | 用 renderer-compatible font policy 测量普通/富文本，并从同一次 shaping 返回 wrapped/BiDi per-span/UTF-8-range geometry 与 point hit |
| `font::catalog::FontCatalog` | public | 加载、排序并去重系统 family names |
| `font::platform::system_family_names` | private | 选择 macOS CoreText 或 portable font database |
| `font::system::new_font_system` | private | 建立 renderer/layout 共用的 locale-aware font database，并应用平台 raster compatibility filter |
| `renderer::UiViewport` | public | 绑定 physical target extent 与 scale factor |
| `renderer::UiRenderer` | public | 持有 rect pipeline、font system、Swash cache、glyph atlas 与 text pipeline |
| `rect_renderer::RectRenderer` | private | 上传 instanced rect 并执行 WGSL rounded-rect/border/clip draw |
| `icon_renderer::IconRenderer` | private | 按 SVG/physical size 栅格化 alpha mask、分配 atlas 并执行 tinted quad draw |
| `image_renderer::ImageRenderer` | private | 按稳定 `ImageId` 把 immutable RGBA8 pixels 上传到 4096² sRGB atlas，并执行 clipped/scaled quad draw |
| `rect_renderer::validate_paint_rect` | private | 在 buffer upload 前校验 rect、border、radii 与 clip |
| `renderer::validate_text_block` | private | 在 glyph prepare 前拒绝非有限或非正 metrics |
| `renderer::PreparedArea` | private | 保存转换到 physical pixels 的 origin、clip bounds 与颜色 |
| `renderer::TextLayer` | private | 为每个 scene layer 保留独立 glyphon buffers/renderer，使浮层外壳能够遮住下层文本后再画自己的内容 |
| `rect_renderer::RectRenderer::layer_ranges` / `icon_renderer::IconRenderer::layer_ranges` | private | 把共享 instance buffer 切成逐层 draw ranges；不得重新决定组件层级 |
| `renderer::UiRenderError` | public | 区分输入校验、atlas prepare 与 render failure |

`Color` 的 RGB channel 是 sRGB、alpha 为 straight alpha。`Point`、`Size`、font size 与 line
height 都使用 logical UI pixels；`UiRenderer::prepare` 是唯一 logical-to-physical 转换点。

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
      → Component::paint
          ├─ ContextView → anchored layout → overlay layer
          │   ├─ floating shell rect
          │   └─ caller content inside content-bounds clip
          ├─ ContextMenu → ContextView + shadow/menu surface + vertical ActionBar
          │   └─ soft BoxShadow + 2px padding + 4px radius + selected MenuItem presentation
          ├─ Dropdown → ContextView + vertical ActionBar
          │   └─ selected item → Button selection presentation
          ├─ ScrollView → viewport clip + translated content geometry + interactive scrollbar chrome
          ├─ ActionBar → item bounds
          │   ├─ ActionBarButton → Button → icon/text primitives
          │   └─ Separator → rect primitive
          ├─ TabList → Tab bounds → state/selection surface rect
          ├─ Button state/style → IconLabel → icon/text primitives
          └─ InputBox → rect/text primitives
  → UiScene::draw_rect / UiScene::draw_image / UiScene::draw_icon / UiScene::with_clip
  → UiScene::draw_text
  → zeta_wgpu::WgpuRenderer::render_scene
      → UiRenderer::prepare
          → RectRenderer::prepare / validate_paint_rect / group layer ranges
          → ImageRenderer::prepare / validate bounds / atlas upload on cache miss
          → IconRenderer::prepare / group layer ranges
              → resvg rasterize on cache miss
              → symbolic-mask + fixed-color atlas upload / instance preparation
          → validate_text_block
          → glyphon::Buffer::set_text / shape_until_scroll
          → glyphon::TextRenderer::prepare
      → UiRenderer::render
          → for each base/overlay layer in creation order
              → RectRenderer::render_layer
              → ImageRenderer::render_layer
              → IconRenderer::render_layer
              → glyphon::TextRenderer::render
      → zeta-wgpu submit / present
      → UiRenderer::trim
```

`UiRenderer::prepare` 每帧上传当前 rect/image/icon instances 并重建 text buffers，但保留 GPU
pipeline、image/icon atlases、`FontSystem`、`SwashCache`、`TextAtlas` 和 `TextRenderer`。
Icon cache key 是 semantic definition 与 physical width/height；scale factor 或 logical size
改变时生成新的 raster，caller tint 不进入缓存键。Instance buffer 按需扩展到下一个
power-of-two capacity；
glyph cache 与各 atlas 跨帧存活。背景色由 `zeta-wgpu` 做 sRGB 到 linear 转换；rect/icon
color 在 renderer 中转 linear，glyph color 保持 sRGB bytes 交给 glyphon。

## 4. 字体与 CoreText

`FontCatalog::system` 在 macOS 调用 `coretext::FontCollection::available`，只返回 canonicalized
family-name snapshot。其他平台从 glyphon 的 font database 枚举 family。

当前文本绘制路径在所有平台统一使用 glyphon：

- cosmic-text 负责 shaping、line breaking、font matching 与 fallback；
- swash 负责 glyph raster/cache；
- glyphon 负责 atlas preparation 和 wgpu render pass。

`font::system::new_font_system` 是 `UiRenderer` 与 `TextInputLayoutEngine` 的共同构造入口。
它加载系统字体、保留系统 locale，并在 macOS 排除 `GB18030 Bitmap`：该 face 能被 cosmic-text
选为 CJK fallback，但 swash 不能把其 bitmap glyph 栅格化，若不排除会出现 cell/caret 已推进而
字形透明。排除后 CJK 继续由可栅格化的系统 outline font fallback 承担。平台 filter 只处理
已验证的 backend incompatibility，不承担产品字体偏好。

因此“已经接入 CoreText”只表示 macOS 原生 font catalog 已接入，不能解读为 CTLine shaping、
CoreGraphics raster 或原生 typographic metrics 已经成为绘制事实。

## 5. 校验、失败与接入义务

- scale factor 必须 finite 且大于零；
- rect/image/icon/text origin、bounds、clip 与 visual metrics 必须 finite；
- rect bounds 不能为负，border widths 和 corner radii 不能为负；
- icon bounds 不能为负，SVG 必须可解析，且 physical raster 必须能放入固定 icon atlas；
- image bounds 必须为正，RGBA byte length 必须与 dimensions 一致，pixels 必须能放入固定 image atlas；
- symbolic SVG coverage 进入 R8 mask atlas；multicolor 固定色进入 sRGB RGBA atlas，纯黑
  coverage 继续使用 caller tint；
- text bounds、font size 与 line height 必须大于零；
- 校验失败返回对应的 `InvalidScaleFactor`、`InvalidPaintRect`、`InvalidPaintImage`、
  `InvalidPaintIcon`、`InvalidSvgIcon`、`IconRasterTooLarge`、`ImageAtlasFull`、
  `IconAtlasFull` 或 `InvalidTextBlock`；
- glyphon atlas preparation/render failure 分别保留为 `Prepare` 与 `Render`；
- surface retry、lost 与 presentation failure 仍由 `zeta-wgpu` 负责。

Host 必须先根据当前 logical layout 构造完整 `UiScene`，再把同一帧的 physical extent 与 scale
factor 交给 renderer。不要预先把 text coordinates 乘 DPI，否则会发生二次缩放。
`Component` implementation 只能消费 caller 已投影好的 presentation state 并发出 primitives；
component bounds、hit registration、event dispatch 和 authoritative state transition 仍由 host
拥有。`UiScene::draw_component` 在当前 nested clip 内同步 paint，不引入 retained component
instance、隐式 identity 或 lifecycle。`Button` 拥有 control 内部 padding 和 state-specific
background selection，并把 icon/text placement 委托给 `IconLabel`；`Button::icon` 保留不参与
绘制的 accessible label，供 host 的后续 accessibility adapter 使用。Caller 必须显式提供
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
`ScrollbarLayout` 与最终 track/thumb paint 使用同一 geometry。内容高度、可见项
virtualization、scroll anchoring、focus reveal policy 和交互 identity 仍归 composed control 或
产品 host。Terminal 从底部计数和输出增长锚定不属于通用 `ScrollState`；Native 的
`TerminalOutputScrollView` 只负责把该产品状态适配为 `ScrollView` 的顶部相对内容坐标。

`ActionBar` 接收 caller-provided outer bounds，内部拥有 Button/Separator 的方向、间距和 item
几何。默认 item extent 来自共享 style；label 长度不同的正式 Toolbar 可以通过
`ActionBarButton::with_main_axis_extent` 覆盖单项主轴尺寸。`ActionBar::item_bounds` 暴露 visual
bounds；`ActionBar::interactive_item_bounds` 与
`ActionBar::hit_test` 复用相同几何并排除 disabled Button 和 Separator。Host 必须把返回的 item
index 映射到自己的 action identity 和命令。ActionBar 不持有 callback、命令、hover/focus
state 或 product action registry。
`TabList` 同样只消费 caller-provided bounds、排列轴、Tab presentation 和 style。
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
`ContextMenuSelection::Item` 投影唯一
选中项，并使用 ContextMenu 返回的同源 bounds 注册交互。当前
`zeta-native::session_context_menu` 是第一处真实 consumer。
`Dropdown` 是另一层 ContextView 组合：它使用无外层 padding 的浮层外壳，
用垂直 ActionBar 排列 label item，并默认选择第一个 enabled item。Host 可以用
`DropdownSelection::Item` 投影 hover/focus/pressed 对应的唯一选择，用 Dropdown 返回的同源
bounds 注册交互；selected identity、open/close 和 command 不进入组件。
`TextInput` 拥有 local editing state 和 composition，但不拥有 focus、platform IME lifecycle、
component chrome 或产品 reducer。`InputBox::new` 使用 `TextInputLayoutEngine` 从 base state
生成 immutable layout，再组合 background、border、placeholder、selection、caret 和 preedit
presentation。`InputBoxState::Focused(CaretVisibility)` 显式投影 blink phase；组件不读取时钟。
IME 候选框定位读取同一个 `InputBox::caret_bounds`，即使 blink phase 隐藏也不能按字符数量另行
估算。

## 6. 测试与修改路径

```bash
cargo test --manifest-path zeta-rs/Cargo.toml -p zeta-ui
bazel test //zeta-rs/ui:ui-unit-tests
```

单元测试覆盖组件裁剪与浮层合成、SplitView 的横纵 Pane geometry、priority 分配、visibility、
Sash track 和相邻 resize clamp，Grid 的横纵嵌套、隐藏子树、identity 校验与 owning-split
Sash 路由，Sash 命中/反馈几何与状态绘制，ScrollState 的 axis clamp、绝对 offset、首尾和
ensure-visible transition，ScrollView 的内容坐标、裁剪、visibility policy、比例 thumb geometry、
track paging、thumb drag 映射、hover/active 颜色与 fade deadline，ContextView 的纵/横锚定、
翻转、对齐、viewport 约束、外壳/内容裁剪，Dropdown 的默认/显式选择、无外层 inset 与命中，
ContextMenu 的柔和阴影、2px padding、4px radius、默认/显式选择与命中，ActionBar 排列与命中、
TabList 横纵排列与
surface 状态、按钮、图标标签和输入框的状态/样式/布局，
以及 `TextInput` 的字素编辑/组合、光标闪烁阶段、选择/光标/预编辑塑形、几何相交、圆角限制、
嵌套裁剪、SVG alpha 栅格化、图集分配、矩形实例 conversion、paint/icon/text validation、
style defaults、富文本 span 的同段 shaping/换行测量、per-span visual fragments 与 font family canonicalization。macOS
测试还验证简中、日文、韩文、组合音标、
阿拉伯文和 Emoji 的 fallback glyph 不缺失并能通过 Swash 栅格化。测试不创建真实 GPU device；
真实 shader、atlas output、fallback placement 和 HiDPI 仍需要各平台的 surface smoke 或
snapshot harness。

- 扩展 text style/span：同步修改 `scene.rs`、`text_layout.rs`、renderer glyphon mapping、tests
  与本 README，并检查 `zeta-markdown` projection；
- 更换 shaping/raster backend：保持 `UiScene` 平台无关，并更新字体语义与 failure contract；
- 增加 RGBA image/path 等 primitive：放入独立 scene/renderer module，不扩张 icon 或 font module；
- 修改 rect contract 或 shader：同步检查 `paint.rs`、`rect_renderer.rs`、`rect.wgsl` 和 tests；
- 修改 DPI/clip 转换：同步检查 `PreparedArea`、`UiViewport` 和 `zeta-wgpu` background path。

## 7. 当前限制与扩展点

当前限制：

- scene 有背景、rect、symbolic icon 和 text，没有 RGBA image、path 或 transform；基础层与每个
  浮层按创建顺序合成，每层内部 render order 固定为 rect → icon → text；
- component contract 当前是 immediate presentation composition，没有 component tree、identity、
  mount/unmount lifecycle、invalidation propagation 或 retained layout；
- `Button` 当前支持 resting、hovered、focused、pressed、disabled、selected、icon-only 与
  leading icon，但尚无独立 focus ring、trailing content 或真实 accessibility adapter；
- `ActionBar` 当前支持 horizontal/vertical Button 与 Separator、同源 item bounds 和 hit-test，
  但尚无 roving focus、keyboard navigation、overflow 或 custom representation；
- `Dropdown` 当前只支持单列 label item、固定 item size 和单项 selection；icon、separator、
  submenu、typeahead、scroll/virtualization、打开/关闭与 accessibility scope 尚无；
- `BoxShadow` 当前支持单个 rounded-rect shadow 的 color、offset 与 blur radius；尚无 spread、
  inset 或多重 shadow。ContextMenu 也暂不支持 icon、separator、submenu、typeahead 或超高菜单滚动；
- `TabList` 当前只拥有固定 item size、gap 和 Tab surface paint；custom content、动态宽度、
  overflow、close action、identity、interaction 与 tabpanel 均由 composed control/host 拥有；
- `ContextView` 不拥有 shadow、arrow/callout，也不拥有 outside click、Escape、focus
  restoration 或 accessibility scope；overflow shadow 由托管内容的 `PaintRect` 拥有，
  lifecycle/interaction 由 host 与 `zeta-ui-dispatch` 组合；
- `ScrollView` 当前提供 overlay scrollbar 的同源 paint/hit/track-page/thumb-drag geometry，
  `ScrollbarController` 提供 hover/active/fade presentation；平台事件接线、pointer capture、
  滚动惯性、overscroll 和 accessibility adapter 仍由 host/dispatch 扩展；
- `SplitViewLayout` 当前是静态 slice 输入和单帧 geometry；`GridLayout` 递归组合这些
  Split，但仍是 caller tree 的单帧 projection，没有 add/remove/move、cached hidden size、
  active Pane、产品绑定或序列化 API；这些 retained topology/state 仍由 product host 拥有；
- `Sash` 当前只拥有 presentation geometry，没有 pointer capture、keyboard resize 或
  accessibility adapter；这些交互由 host 与 `zeta-ui-dispatch` 组合；
- multicolor 分层当前按栅格化后的纯黑像素识别 symbolic coverage；若未来 artwork 需要把固定
  黑色与 caller-tinted 黑色同时表达，必须扩展显式资源标注，不能依赖颜色猜测；
- `TextInput` 是 single-line base，没有 focus/platform IME owner、undo/redo、clipboard 或
  accessibility contract；
- `InputBox` 消费显式 blink phase，但没有 mouse caret hit testing、drag selection 或
  disabled/read-only presentation；
- native 当前使用 `CaretBlinkController::default` 的 530ms half-period，尚未读取系统 caret
  blink preference 或 reduced-motion setting；
- clip 当前是 axis-aligned logical rect；不支持 rounded/path clip 或 nested GPU scissor stack；
- 没有 widget tree、retained Grid state、focus、input routing、IME lifecycle 或
  accessibility；
- 每帧重建 glyphon text buffers，富文本 span 会在同一个 paragraph buffer 中 shaping，但尚无
  paragraph-level retained cache；
- `FontWeight` 与 `FontStyle` 只有常用 semantic variants；
- CoreText 只做 catalog，不做 shaping/raster，也没有 app font registration；
- icon atlas 固定为 2048×2048，尚无增长、回收或跨 atlas eviction；
- 没有 headless GPU golden test。

扩展点：出现第二个需要 secondary action/overflow 的真实消费者后，可以在 `ActionBar` 上组合
独立 `ToolBar`；出现 Dropdown 或 custom representation 后，再增加对应 `ActionBarItem`
variant，不为单一 Button 增加纯转发 wrapper。出现真实编辑器/终端消费者后，还可以分别增加
可增长或可回收的图标图集、保留式段落缓存、平台字体注册、RGBA 图像/路径
primitives 与统一 display list。是否采用 CoreText shaping 应由跨平台 metrics/fallback
一致性测试决定，不是当前 API 的既定承诺。
