# `zeta-wgpu`

> 本 README 负责 wgpu backend 的当前实现与失败语义。跨 backend 契约见
> [`zeta-renderer`](../renderer/README.md)，系统替换规则见
> [`docs/rendering-architecture.md`](../docs/rendering-architecture.md)，scene 语义见
> [`zui`](../zui/README.md)。

`zeta-wgpu` 拥有一个 native window 的 `wgpu` presentation 资源，并把可选的 immutable
`zui::UiScene` 组合进同一 render pass。它不知道 App Server、Session、Thread、Agent、
Workbench 或其他产品状态。

## 1. 所有权

| 能力 | 本 crate | 委托/不拥有 |
| --- | --- | --- |
| Instance、adapter、device、queue、surface、configuration | ✅ | |
| Frame acquire、clear、submit、present | ✅ | |
| Resize、零尺寸与 surface lost recovery | ✅ | |
| UI scene/paint/font/text semantics | ❌ | `zui` |
| Rect/image/icon/text GPU pipelines、atlas 与 glyph resources | ✅ | |
| Backend-neutral frame contract | ❌ | `zeta-renderer::Renderer` |
| Event loop、窗口策略 | ❌ | `zeta-winit` / product host |
| Element/layout 与 text-input 基座 | ❌ | `zui` |
| Widget、input routing、IME adapter、accessibility | ❌ | `zeta-ui` / product host / platform adapter |
| App Server、workspace、durable state | ❌ | 上层产品 |

依赖方向：

```text
native product host
  ├─→ zeta-winit → winit
  ├─→ zeta-ui → zui
  ├─→ zeta-renderer → zui
  └─→ zeta-wgpu → zeta-renderer
                  → zeta-winit
                  → zui
                  → wgpu
```

出现 App Server method、product command、workspace model、widget state 或 font-platform
implementation，意味着 crate ownership 已经漂移。

## 2. 公共与内部接口

| Symbol | 可见性 | 职责 | 不能承担 |
| --- | --- | --- | --- |
| `WgpuRenderer` | public | 一个 window 的 GPU/surface 与 UI GPU pipeline ownership，并实现 `Renderer` | event loop、产品状态 |
| `WgpuRenderer::initialize` | public | `NativeWindow` → surface/adapter/device/config/UI pipeline | App Server startup |
| `WgpuRenderer::resize` | public | 保存 physical extent 并在非零时 configure | layout 计算 |
| `WgpuRenderer::set_scale_factor` | public | 保存平台 DPI 事实 | logical layout policy |
| `WgpuRenderer::request_redraw` | public | 转发 owned native window 的 redraw 请求 | 动画或 invalidation policy |
| `WgpuRenderer::render` | public | 无 scene 的稳定 clear/present 路径 | UI state mutation |
| `WgpuRenderer::render_scene` | public | prepare UI、clear scene background、draw、present | 构造或布局 scene |
| `zeta_renderer::RenderOutcome` | re-export | 告知 host presented/skipped/retry | 自行调度 event loop |
| `WgpuRendererError` | public | surface/device/UI render failure | 产品级恢复策略 |
| `ui_renderer::UiRenderer` | private | 协调 rect/image/icon/text prepare，并严格按 `UiScene::batches` 执行 | scene/layout/paint-order 语义 |
| `ui_renderer::CachedTextBuffer` | private | 按 Scene 文本槽位复用 shape-affecting 内容未变化的 glyphon Buffer；位置、clip 与普通文本颜色变化只更新 TextArea | 跨组件 identity、无限历史缓存 |
| `ui_renderer::{rect,image,icon}` | private | instance conversion、validation、atlas、shader 与 primitive-to-instance range mapping | component state |
| `ui_renderer::UiRenderError` | public | 区分 invalid scene、atlas、glyph prepare/render failure | surface recovery |
| `viewport::Viewport` | private | physical extent 与 scale-factor 状态 | window/GPU handle |
| `WgpuRenderer::render_frame` | private | 统一 clear-only 与 scene frame 顺序 | 业务 invalidation |
| `wgpu_color` | private | sRGB background → linear `wgpu::Color` | glyph color mapping |

## 3. 执行路径与接口面语义

```text
product-owned ApplicationHandler
  → NativeWindow::create
  → WgpuRenderer::initialize
  → Box<dyn zeta_renderer::Renderer>
  → resize / set_scale_factor
  → render 或 render_scene
      → WgpuRenderer::render_frame
          → Surface::get_current_texture
          → UiRenderer::prepare (scene primitive → backend resource/range)
              → refresh_text_buffer_cache (reuse unchanged shaping buffers)
          → clear background
          → UiRenderer::render (ordered SceneBatch execution)
          → Queue::submit
          → NativeWindow::pre_present_notify
          → Queue::present
          → UiRenderer::trim (scene only)
```

`Viewport::surface_extent()` 对任一零维返回 `None`，因此最小化期间不会提交零尺寸 surface
configuration。UI prepare 在 frame acquire 后、command encoding 前发生；prepare 失败时 frame
不会 submit/present。

| `CurrentSurfaceTexture` | `zeta-wgpu` 行为 | Host 行为 |
| --- | --- | --- |
| `Success` | clear、可选 UI draw、submit、present | 无 |
| `Timeout` / `Occluded` | 返回 `Skipped` | 等下一次 invalidation |
| `Outdated` / `Suboptimal` | configure，返回 `Retry` | 请求一次 redraw |
| `Lost` | 重建 surface，返回 `Retry` | 请求一次 redraw |
| `Validation` | 返回 `SurfaceValidation` | 产品决定退出或恢复 |

renderer 从不主动 busy-loop。动画、timer、App Server update 与 UI invalidation 均由上层 host
产生。

## 4. 测试

```bash
cargo test --manifest-path Cargo.toml -p zeta-wgpu
bazel test //zeterm/wgpu:wgpu-unit-tests
```

单元测试覆盖 viewport 状态、sRGB conversion、scene primitive validation、instance conversion、
icon raster 和 atlas allocation，不创建窗口或 GPU。普通 CI 编译
不能证明 Metal、DX12 或 Vulkan presentation 可用；真实 surface 与 glyph output smoke 由产品
binary 在各平台执行。

## 5. 修改路径与当前限制

- surface API/恢复策略：同步检查 `gpu.rs`、`viewport_tests.rs` 和本 README；
- scene/text contract：修改 `zui`，不要在 backend 中复制组件或产品语义；
- 新 paint primitive：先在 `zui` 建立 scene contract，再在 `ui_renderer` 增加 backend 实现；
- 通用 frame outcome 或 host 接口：修改 `zeta-renderer`，不要把 wgpu 类型加入 trait；
- window/event-loop policy：属于 `zeta-winit` 和 product host；
- App Server/product 功能：禁止在本 crate 接入。

当前限制：

- 一个 `WgpuRenderer` 只服务一个 window/surface；
- 没有 frame telemetry、device-lost migration 或显式 pipeline warmup；
- 没有 headless GPU test、golden image 或产品 host vertical；
- multicolor icon 栅格当前把纯黑像素视为 symbolic coverage；需要固定黑色与 caller tint 并存时，
  必须扩展 icon resource contract；
- `render_scene` 当前绘制 `zui` 的背景、rect、RGBA image、icon 和 text primitive。
