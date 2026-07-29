# `zeta-native`

`zeta-native` 是与 Electron Desktop 和 TUI 同级的原生产品入口。当前窗口纵切由产品拥有
`ApplicationHandler`，组合 `zeta-winit`、`zeta-wgpu` 与 `zeta-ui`，并在单个原生窗口中绘制
响应式 shell 骨架。

## Ownership

| 能力 | 当前 owner | 状态 |
| --- | --- | --- |
| 产品窗口标题、初始尺寸与 event routing | `zeta-native::NativeApp` | ✅ |
| Event loop 与 native window adapter | `zeta-winit` | 委托 |
| GPU surface、resize、present 与 retry | `zeta-wgpu` | 委托 |
| Rect、symbolic SVG icon、字体、shaping 与 GPU 绘制 | `zeta-ui` | 委托 |
| Shell layout、hit regions 与 scene composition | `shell_scene` | ✅ |
| Product SVG assets 与 semantic icon placement | `zeta-native/assets` / `shell_scene` | ✅ |
| Theme、hover、pressed、selection 与 composer focus | `shell_interaction` / `shell_theme` | ✅ |
| Transparent native chrome 与窗口拖动 adapter | `zeta-winit` | 委托 |
| App Server session 与产品状态 projection | 尚无 owner | 尚未完成 |
| Transcript、composer、scroll 与 input | 尚无 owner | 尚未完成 |

依赖方向：

```text
zeta-native → zeta-winit
            → zeta-wgpu → zeta-winit
                        → zeta-ui
            → zeta-ui
```

`zeta-native` 可以拥有可丢弃的 presentation state 和产品交互，但不能复制 Session、Thread、
Turn 或 Tool 的权威状态机。后续接入只能通过 `zeta-app-server-client` 的 typed contract。

## 当前执行路径

```text
main
  → zeta_winit::run_application
  → NativeApp::resumed
      → NativeWindow::create
      → build_shell_scene
      → WgpuRenderer::initialize
      → request_redraw
  → NativeApp::window_event
      → resize / scale-factor update → rebuild scene
      → cursor / primary mouse event → hit test → interaction state
          → rebuild scene → request redraw
      → titlebar drag hit → NativeWindow::start_window_drag
      → visible-after-occlusion → request redraw
      → WgpuRenderer::render_scene
```

运行：

```bash
cargo run --manifest-path zeta-rs/Cargo.toml -p zeta-native
```

`shell_scene::ShellLayout` 把 logical viewport 分成 product-drawn titlebar、sidebar、
transcript 与 composer；`ShellTheme`/`ShellPalette` 拥有这个 product host 的 presentation
tokens。`ShellHitMap` 以 reverse paint priority 做 logical-pixel hit testing，
`ShellInteraction` 拥有 hover、pressed、selected session、theme 和 composer focus。过小
viewport 使用有边界的 compact fallback。Titlebar theme icon 由产品拥有 SVG bytes 和
semantic placement，`zeta-ui` 只负责按 DPI 栅格、缓存 alpha mask 和应用 palette tint。

当前鼠标可以切换主题、选择 session、聚焦 composer，并在非交互 titlebar 区域拖动窗口。它还
没有 App Server、文本编辑、keyboard focus traversal、scroll、IME、accessibility 或持久化
窗口状态；这些是后续独立产品纵切，不应进入 `zeta-winit` 或 `zeta-wgpu`。

macOS 可能在新窗口激活完成前把首次 surface acquisition 报为 occluded；该 frame 会被跳过。
`NativeApp` 在后续 `WindowEvent::Occluded(false)` 上重新请求 redraw，保证首个可见 frame 不会
因为一次正常的 activation transition 永久丢失。
