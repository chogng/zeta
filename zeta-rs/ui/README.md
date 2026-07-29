# `zeta-ui`

> 本 README 是 native UI scene、paint primitives、font catalog 与 GPU 绘制路径的当前实现说明。
> Surface 生命周期与 presentation 的 canonical 文档在
> [`zeta-wgpu`](../wgpu/README.md)；产品 UI 架构尚无独立系统文档。

`zeta-ui` 定义与窗口系统无关的 immutable frame scene，实现 instanced rect、symbolic SVG
icon 与 text GPU pipeline。SVG icon 由 `resvg` 栅格为按 physical size 缓存的 alpha mask，
`glyphon` 完成 shaping、glyph cache、texture atlas 和 text draw。macOS 的系统字体目录通过
CoreText 读取；CoreText 当前不承担文本 shaping 或 glyph rasterization。

## 1. 边界与依赖方向

| 能力 | 当前 owner | 状态 |
| --- | --- | --- |
| Rect、symbolic SVG icon、clip 与文本 scene | `zeta-ui::UiScene` | ✅ |
| 系统 font-family 枚举 | `zeta-ui::FontCatalog` | ✅ |
| macOS CoreText font catalog adapter | `font::platform::macos` | ✅ |
| shaping、fallback、glyph raster/cache | `glyphon` / `cosmic-text` / `swash` | 委托 |
| Instanced rect、icon atlas 与 glyph text draw pipeline | `zeta-ui::UiRenderer` | ✅ |
| Surface acquire/configure/present | `zeta-wgpu::WgpuRenderer` | ❌ |
| Widget、layout、input、IME、accessibility | 尚无 owner | 尚未完成 |

依赖方向：

```text
product host → zeta-ui
product host → zeta-wgpu → zeta-ui

zeta-ui → glyphon → cosmic-text / swash
        → resvg → usvg / tiny-skia
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
| `geometry::{Rect, Edges, CornerRadii}` | public | 使用 logical UI pixels 表达几何与 visual metrics |
| `paint::{PaintRect, Border, Color}` | public | 表达 fill、per-edge border、rounded corners 与 sRGB color |
| `icon::{SvgIcon, PaintIcon}` | public | 绑定静态 SVG bytes，并以 logical bounds 和主题色表达 symbolic icon |
| `scene::UiScene` | public | 保存一帧背景、rect、icon、text 和构建时的 nested clip |
| `scene::TextBlock` / `TextStyle` | public | 使用 logical UI pixels 表达文本、bounds 和样式 |
| `font::catalog::FontCatalog` | public | 加载、排序并去重系统 family names |
| `font::platform::system_family_names` | private | 选择 macOS CoreText 或 portable font database |
| `renderer::UiViewport` | public | 绑定 physical target extent 与 scale factor |
| `renderer::UiRenderer` | public | 持有 rect pipeline、font system、Swash cache、glyph atlas 与 text pipeline |
| `rect_renderer::RectRenderer` | private | 上传 instanced rect 并执行 WGSL rounded-rect/border/clip draw |
| `icon_renderer::IconRenderer` | private | 按 SVG/physical size 栅格化 alpha mask、分配 atlas 并执行 tinted quad draw |
| `rect_renderer::validate_paint_rect` | private | 在 buffer upload 前校验 rect、border、radii 与 clip |
| `renderer::validate_text_block` | private | 在 glyph prepare 前拒绝非有限或非正 metrics |
| `renderer::PreparedArea` | private | 保存转换到 physical pixels 的 origin、clip bounds 与颜色 |
| `renderer::UiRenderError` | public | 区分输入校验、atlas prepare 与 render failure |

`Color` 的 RGB channel 是 sRGB、alpha 为 straight alpha。`Point`、`Size`、font size 与 line
height 都使用 logical UI pixels；`UiRenderer::prepare` 是唯一 logical-to-physical 转换点。

## 3. 当前执行路径

```text
host
  → UiScene::new
  → UiScene::draw_rect / UiScene::draw_icon / UiScene::with_clip
  → UiScene::draw_text
  → zeta_wgpu::WgpuRenderer::render_scene
      → UiRenderer::prepare
          → RectRenderer::prepare / validate_paint_rect
          → IconRenderer::prepare
              → resvg rasterize on cache miss
              → icon atlas upload / instance preparation
          → validate_text_block
          → glyphon::Buffer::set_text / shape_until_scroll
          → glyphon::TextRenderer::prepare
      → UiRenderer::render
          → RectRenderer::render
          → IconRenderer::render
          → glyphon::TextRenderer::render
      → zeta-wgpu submit / present
      → UiRenderer::trim
```

`UiRenderer::prepare` 每帧上传当前 rect/icon instances 并重建 text buffers，但保留 GPU
pipeline、symbolic icon atlas、`FontSystem`、`SwashCache`、`TextAtlas` 和 `TextRenderer`。
Icon cache key 是 SVG 内容与 physical width/height；scale factor 或 logical size 改变时生成新的
mask，主题 tint 不进入缓存键。Instance buffer 按需扩展到下一个 power-of-two capacity；
glyph cache 与两个 atlas 跨帧存活。背景色由 `zeta-wgpu` 做 sRGB 到 linear 转换；rect/icon
color 在 renderer 中转 linear，glyph color 保持 sRGB bytes 交给 glyphon。

## 4. 字体与 CoreText

`FontCatalog::system` 在 macOS 调用 `coretext::FontCollection::available`，只返回 canonicalized
family-name snapshot。其他平台从 glyphon 的 font database 枚举 family。

当前文本绘制路径在所有平台统一使用 glyphon：

- cosmic-text 负责 shaping、line breaking、font matching 与 fallback；
- swash 负责 glyph raster/cache；
- glyphon 负责 atlas preparation 和 wgpu render pass。

因此“已经接入 CoreText”只表示 macOS 原生 font catalog 已接入，不能解读为 CTLine shaping、
CoreGraphics raster 或原生 typographic metrics 已经成为绘制事实。

## 5. 校验、失败与接入义务

- scale factor 必须 finite 且大于零；
- rect/icon/text origin、bounds、clip 与 visual metrics 必须 finite；
- rect bounds 不能为负，border widths 和 corner radii 不能为负；
- icon bounds 不能为负，SVG 必须可解析，且 physical raster 必须能放入固定 icon atlas；
- text bounds、font size 与 line height 必须大于零；
- 校验失败返回对应的 `InvalidScaleFactor`、`InvalidPaintRect`、`InvalidPaintIcon`、
  `InvalidSvgIcon`、`IconRasterTooLarge`、`IconAtlasFull` 或 `InvalidTextBlock`；
- glyphon atlas preparation/render failure 分别保留为 `Prepare` 与 `Render`；
- surface retry、lost 与 presentation failure 仍由 `zeta-wgpu` 负责。

Host 必须先根据当前 logical layout 构造完整 `UiScene`，再把同一帧的 physical extent 与 scale
factor 交给 renderer。不要预先把 text coordinates 乘 DPI，否则会发生二次缩放。

## 6. 测试与修改路径

```bash
cargo test --manifest-path zeta-rs/Cargo.toml -p zeta-ui
bazel test //zeta-rs/ui:ui-unit-tests
```

单元测试覆盖 geometry intersection、radii clamp、nested clip、SVG alpha raster、atlas
allocation、rect instance conversion、paint/icon/text validation、style defaults 与 font
family canonicalization。它们不创建真实 GPU device；真实 shader、icon/glyph output、
fallback 和 HiDPI 需要各平台的 surface smoke 或 snapshot harness。

- 扩展 text style：同步修改 `scene.rs`、glyphon mapping、tests 与本 README；
- 更换 shaping/raster backend：保持 `UiScene` 平台无关，并更新字体语义与 failure contract；
- 增加 RGBA image/path 等 primitive：放入独立 scene/renderer module，不扩张 icon 或 font module；
- 修改 rect contract 或 shader：同步检查 `paint.rs`、`rect_renderer.rs`、`rect.wgsl` 和 tests；
- 修改 DPI/clip 转换：同步检查 `PreparedArea`、`UiViewport` 和 `zeta-wgpu` background path。

## 7. 当前限制与扩展点

Current limitations：

- scene 有背景、rect、symbolic icon 和 text，没有 RGBA image、path、transform 或统一的跨
  primitive z-order；当前 render order 固定为 rect → icon → text；
- clip 当前是 axis-aligned logical rect；不支持 rounded/path clip 或 nested GPU scissor stack；
- 没有 widget tree、layout engine、focus、input、IME 或 accessibility；
- 每帧重建 glyphon text buffers，尚无 paragraph-level retained cache；
- `FontWeight` 与 `FontStyle` 只有常用 semantic variants；
- CoreText 只做 catalog，不做 shaping/raster，也没有 app font registration；
- icon atlas 固定为 2048×2048，尚无增长、回收或跨 atlas eviction；
- 没有 headless GPU golden test。

Extension points：在出现真实编辑器/终端 consumer 后，可分别增加可增长或可回收的 icon
atlas、retained paragraph cache、rich spans、platform font registration、RGBA image/path
primitives 与统一 display list。是否采用 CoreText shaping 应由跨平台 metrics/fallback
一致性测试决定，不是当前 API 的既定承诺。
