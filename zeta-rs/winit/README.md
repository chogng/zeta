# `zeta-winit`

> 本 README 负责 native event-loop/window crate 的当前实现、集成义务和修改路径。
> GPU surface 与 presentation 由 [`zeta-wgpu`](../wgpu/README.md) 拥有。
> Native 文本输入的跨 crate ownership 见
> [`docs/native-text-input.md`](../../docs/native-text-input.md)。

`zeta-winit` 是架构分类中位于 App Server 下方的底层 native host adapter。它封装 `winit`
event-loop bootstrap、window ownership 与 persistent display handle，但不拥有任何产品身份、
App Server connection、UI tree 或渲染状态。

## 1. 所有权

| Symbol | 可见性 | 职责 | 不拥有 |
| --- | --- | --- | --- |
| `run_application` | public | 创建 event loop 并运行 product-owned handler | 产品 lifecycle 或错误策略 |
| `run_application_with_user_events` | public | 创建 typed event loop，并把 `EventLoopProxy<T>` 交给产品构造器 | user-event 类型或后台任务 |
| `NativeWindow::create` | public | 从 `ActiveEventLoop` 和 attributes 创建窗口 | 产品窗口策略 |
| `NativeWindow` | public | 安全持有 window 与 display handle | GPU surface、widget 或 workspace |
| `PhysicalExtent` | public | 跨底层 crate 传递 physical pixel dimensions | logical layout |
| `WindowChrome` / `apply_window_chrome` | public | 把命名 chrome policy 转换为平台 attributes | titlebar paint/layout |
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
       ├─ ActiveEventLoop::create_window
       └─ ActiveEventLoop::owned_display_handle
  → NativeWindow event/redraw methods
  → ActiveEventLoop::set_control_flow (product-owned wakeup deadline)
  → enable_ime / disable_ime / set_ime_cursor_area
  → apply_window_chrome (optional platform chrome adaptation)
  → zeta-wgpu consumes surface_target + display_handle
```

## 2. 边界与失败

- window 必须在 `ApplicationHandler::resumed` 后创建，以保留移动端 surface lifecycle；
- `WindowAttributes` 由产品构造，因此标题、尺寸和窗口模式不是本 crate policy；
- `ContentUnderTitlebar` 在 macOS 保留 system window buttons，同时启用 transparent titlebar、
  full-size content 和 first-mouse；其他平台当前保持 attributes 不变；
- event-loop 与 window creation error 原样返回，产品决定诊断、恢复或退出；
- `NativeWindow` 只提供 handle、identity、extent、scale factor、redraw/present hooks 与原生窗口
  交互 forwarding；
- IME 默认保持关闭；product host 必须根据 editable focus 显式启停，并只在 active composition
  contract 内更新 logical candidate area；
- 出现 App Server method、workspace state、widget、paint scene 或 GPU resource 意味着 ownership
  已漂移。

## 3. 测试与限制

```bash
cargo test --manifest-path zeta-rs/Cargo.toml -p zeta-winit
```

当前单元测试只验证无平台依赖的值语义。CI 编译不能代替 macOS、Windows、Linux 的真实窗口、
resume/suspend、DPI 与多窗口 smoke。

当前没有产品 event handler、默认窗口策略、clipboard ownership、accessibility、file drag/drop 或
platform menu。IME 仅有 platform forwarding，不拥有 focus、composition 或文本编辑语义。
