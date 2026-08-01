# `zeta-winit`

> 本 README 负责 native event-loop/window crate 的当前实现、集成义务和修改路径。
> GPU surface 与 presentation 由 [`zeta-wgpu`](../wgpu/README.md) 拥有。
> Native 文本输入的跨 crate ownership 见
> [`docs/native-text-input.md`](../../docs/native-text-input.md)。
> 产品 Titlebar 如何消费窗口控件占位由
> [`docs/native-terminal-ui.md`](../../docs/native-terminal-ui.md) 统一说明。

`zeta-winit` 是架构分类中位于 App Server 下方的底层 native host adapter。它封装 `winit`
event-loop bootstrap、window ownership 与 persistent display handle，但不拥有任何产品身份、
App Server connection、UI tree 或渲染状态。

## 1. 所有权

| Symbol | 可见性 | 职责 | 不拥有 |
| --- | --- | --- | --- |
| `run_application` | public | 创建 event loop 并运行 product-owned handler | 产品 lifecycle 或错误策略 |
| `run_application_with_user_events` | public | 创建 typed event loop，并把 `EventLoopProxy<T>` 交给产品构造器 | user-event 类型或后台任务 |
| `NativeWindow::create` | public | 从 `ActiveEventLoop`、attributes 和显式 `WindowChrome` 创建窗口 | 产品标题、尺寸或窗口模式 |
| `NativeWindow` | public | 安全持有 window 与 display handle | GPU surface、widget 或 workspace |
| `PhysicalExtent` | public | 跨底层 crate 传递 physical pixel dimensions | logical layout |
| `NativeWindow::request_inner_logical_size` | public | 把 host 决定的 logical inner size 请求转发给平台窗口 | 产品布局、面板宽度或 resize policy |
| `WindowChrome` | public | 让产品选择 native chrome 与 full-size titlebar 的共享方式 | titlebar paint/layout |
| `WindowControlInsets` / `NativeWindow::window_control_insets` | public | 把当前 chrome policy 投影为覆盖产品内容的左右逻辑占位 | ActionBar 间距或产品布局 |
| `NativeWindow::start_window_drag` | public | 转发产品 titlebar 命中的平台窗口拖动 | hit testing |
| `NativeWindow::set_cursor` | public | 应用产品 hit testing 选择的 cursor | hover state |
| `NativeWindow::set_title` | public | 把 product/session title 转发给 platform window | title 来源或 OSC parsing |
| `NativeWindow::enable_ime` / `disable_ime` | public | 转发产品 focus 选择的 IME activation | focus 或 composition |
| `ImeCursorArea` / `set_ime_cursor_area` | public | 把 logical caret area 转发给平台候选框 | shaping 或 caret policy |

`ApplicationHandler`、`ActiveEventLoop`、`ControlFlow`、`WindowEvent`、keyboard/IME/pointer
event values（包括 `MouseButton` 与 `MouseScrollDelta`）、`WindowAttributes`、`WindowId` 和
`LogicalSize`/`PhysicalPosition` 由本 crate 重新导出，使上层 host 不需要绕过 adapter 建立另一套 `winit`
integration。

真实调用关系：

```text
product-owned ApplicationHandler
  → run_application
  → NativeWindow::create
       ├─ apply_window_chrome
       ├─ ActiveEventLoop::create_window
       └─ ActiveEventLoop::owned_display_handle
  → NativeWindow::window_control_insets
       └─ window_chrome::window_control_insets
  → NativeWindow event/redraw methods
  → ActiveEventLoop::set_control_flow (product-owned wakeup deadline)
  → enable_ime / disable_ime / set_ime_cursor_area
  → zeta-wgpu consumes surface_target + display_handle
```

## 2. 边界与失败

- window 必须在 `ApplicationHandler::resumed` 后创建，以保留移动端 surface lifecycle；
- `WindowAttributes` 由产品构造，`NativeWindow::create` 只按显式 `WindowChrome` 应用平台 chrome，
  因此标题、尺寸和窗口模式不是本 crate policy；
- `ContentUnderTitlebar` 在 macOS 保留 system window buttons，同时启用 transparent titlebar、
  full-size content 和 first-mouse；其他平台当前保持 attributes 不变；
- `WindowControlInsets` 使用 logical pixels，只描述 system controls 覆盖产品内容的宽度；macOS
  `ContentUnderTitlebar` 当前返回左侧 `70px`，其他已实现组合返回零；Titlebar 自身必须另加组件
  间距，不能把 `70px` 复制到产品组件；
- `Theme` 只作为 `winit` 窗口外观策略透传；产品可以显式选择 light/dark native chrome，
  `zeta-winit` 不据此决定产品 scene palette；
- event-loop 与 window creation error 原样返回，产品决定诊断、恢复或退出；
- `NativeWindow` 只提供 handle、identity、extent、scale factor、size request、redraw/present hooks
  与原生窗口交互 forwarding；`request_inner_logical_size` 不保证平台接受请求，host 仍须以随后收到的
  `WindowEvent::Resized` 作为实际 surface extent；
- IME 默认保持关闭；product host 必须根据 editable focus 显式启停，并只在 active composition
  contract 内更新 logical candidate area；
- 出现 App Server method、workspace state、widget、paint scene 或 GPU resource 意味着 ownership
  已漂移。

## 3. 测试与限制

```bash
cargo test --manifest-path zeta-rs/Cargo.toml -p zeta-winit
```

当前单元测试验证值语义、chrome attributes preservation 和平台占位 policy。CI 编译不能代替
macOS、Windows、Linux 的真实窗口、resume/suspend、DPI 与多窗口 smoke。

当前没有产品 event handler、默认窗口策略、clipboard ownership、accessibility、file drag/drop 或
platform menu。IME 仅有 platform forwarding，不拥有 focus、composition 或文本编辑语义。
`winit` 当前没有安全的 system button geometry API，而 workspace 禁止 `unsafe` AppKit pointer
访问，因此 macOS 占位是集中在 `window_chrome` 的显式 policy，不是运行时测量；RTL 按钮换边和
未来 Windows controls overlay 必须在该 adapter 增加平台实现，不能在产品 Titlebar 添加第二套
常量。
