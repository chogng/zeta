# `zeta-native`

> Native 文本输入的跨 crate canonical ownership 见
> [`docs/native-text-input.md`](../../docs/native-text-input.md)；本 README 拥有 native host
> 当前源码路径和接入义务。

`zeta-native` 是与 Electron Desktop 和 TUI 同级的原生产品入口。当前窗口纵切由产品拥有
`ApplicationHandler`，组合 `zeta-winit`、`zeta-wgpu` 与 `zeta-ui`，并在单个原生窗口中绘制
响应式 shell 骨架。

## 所有权

| 能力 | 当前 owner | 状态 |
| --- | --- | --- |
| 产品窗口标题、初始尺寸与 event routing | `zeta-native::NativeApp` | ✅ |
| Event loop 与 native window adapter | `zeta-winit` | 委托 |
| GPU surface、resize、present 与 retry | `zeta-wgpu` | 委托 |
| Rect、symbolic SVG icon、字体、shaping 与 GPU 绘制 | `zeta-ui` | 委托 |
| Shell layout、hit regions 与 scene composition | `shell_scene` | ✅ |
| Shell presentation tokens | `shell_style::ShellPalette` | ✅ |
| Hover、pressed、selection 与 composer focus | `shell_interaction` | ✅ |
| Composer committed text、selection 与 IME composition | `zeta-ui::TextInput` | 委托 |
| Caret blink phase calculation | `zeta-ui::CaretBlinkController` | 委托 |
| Caret blink deadline scheduling 与 redraw | `NativeApp` | ✅ |
| Input-box shaping 与 visual composition | `zeta-ui::TextInputLayoutEngine` / `InputBox` | 委托 |
| Transparent native chrome 与窗口拖动 adapter | `zeta-winit` | 委托 |
| App Server session 与产品状态 projection | 尚无 owner | 尚未完成 |
| Transcript、scroll 与 App Server-backed composer submit | 尚无 owner | 尚未完成 |

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
      → build_shell_presentation
      → WgpuRenderer::initialize
      → request_redraw
  → NativeApp::window_event
      → resize / scale-factor update → rebuild scene
      → cursor / primary mouse event → hit test → interaction state
          → rebuild scene → request redraw
      → keyboard event → TextInputCommand → TextInput → rebuild scene
      → IME event → TextInputCompositionEvent → TextInput → rebuild scene
      → composer focus → enable/disable IME → update shaped candidate area
      → about_to_wait → advance caret blink → schedule next WaitUntil deadline
      → titlebar drag hit → NativeWindow::start_window_drag
      → visible-after-occlusion → request redraw
      → WgpuRenderer::render_scene
```

运行：

```bash
cargo run --manifest-path zeta-rs/Cargo.toml -p zeta-native
```

`shell_scene::ShellLayout` 把 logical viewport 分成 product-drawn titlebar、sidebar、
transcript 与 composer；`shell_style::ShellPalette` 保存当前 product host 的 presentation
tokens。`ShellHitMap` 以 reverse paint priority 做 logical-pixel hit testing，
`ShellInteraction` 拥有 hover、pressed、selected session 和 composer focus。过小 viewport
使用有边界的 compact fallback。Native 当前没有 product icon consumer；后续真实 action 应从
`zeta-icons` 选择 semantic icon，不能恢复本地 SVG copy。

当前鼠标可以选择 session、聚焦 composer，并在非交互 titlebar 区域拖动窗口。
Composer 支持单行键盘编辑、grapheme-safe cursor/delete、selection 和 IME preedit/commit；
`NativeApp` 只把 `winit` event 转换为 platform-independent `TextInputCommand` /
`TextInputCompositionEvent`。`TextInput` 是不实现 `Component` 的编辑基座；`InputBox` 才负责
具体 component chrome。`TextInputLayoutEngine` 生成实际 shaped caret，供 scene 绘制和 IME
candidate area 共用。`NativeApp` 持有 `CaretBlinkController`，在 focus/input/composition 后
恢复 visible phase，并由 event loop deadline 驱动后续切换；`InputBox` 不读取时钟。

当前仍没有 App Server、composer submit、mouse caret placement、drag selection、clipboard、
undo/redo、keyboard focus traversal、scroll、accessibility 或持久化窗口状态。这些是后续独立
产品纵切，不应进入 `zeta-winit` 或 `zeta-wgpu`。

macOS 可能在新窗口激活完成前把首次 surface acquisition 报为 occluded；该 frame 会被跳过。
`NativeApp` 在后续 `WindowEvent::Occluded(false)` 上重新请求 redraw，保证首个可见 frame 不会
因为一次正常的 activation transition 永久丢失。
