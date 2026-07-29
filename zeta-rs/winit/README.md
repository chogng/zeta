# `zeta-winit`

> 本 README 负责 native event-loop/window crate 的当前实现、集成义务和修改路径。
> GPU surface 与 presentation 由 [`zeta-wgpu`](../wgpu/README.md) 拥有。

`zeta-winit` 是架构分类中位于 App Server 下方的底层 native host adapter。它封装 `winit`
event-loop bootstrap、window ownership 与 persistent display handle，但不拥有任何产品身份、
App Server connection、UI tree 或渲染状态。

## 1. Ownership

| Symbol | 可见性 | 职责 | 不拥有 |
| --- | --- | --- | --- |
| `run_application` | public | 创建 event loop 并运行 product-owned handler | 产品 lifecycle 或错误策略 |
| `NativeWindow::create` | public | 从 `ActiveEventLoop` 和 attributes 创建窗口 | 产品窗口策略 |
| `NativeWindow` | public | 安全持有 window 与 display handle | GPU surface、widget 或 workspace |
| `PhysicalExtent` | public | 跨底层 crate 传递 physical pixel dimensions | logical layout |

`ApplicationHandler`、`ActiveEventLoop`、`WindowEvent`、`WindowAttributes`、`WindowId` 和
`LogicalSize` 由本 crate 重新导出，使上层 host 不需要绕过 adapter 建立另一套 `winit`
integration。

真实调用关系：

```text
product-owned ApplicationHandler
  → run_application
  → NativeWindow::create
       ├─ ActiveEventLoop::create_window
       └─ ActiveEventLoop::owned_display_handle
  → NativeWindow event/redraw methods
  → zeta-wgpu consumes surface_target + display_handle
```

## 2. 边界与失败

- window 必须在 `ApplicationHandler::resumed` 后创建，以保留移动端 surface lifecycle；
- `WindowAttributes` 由产品构造，因此标题、尺寸和窗口模式不是本 crate policy；
- event-loop 与 window creation error 原样返回，产品决定诊断、恢复或退出；
- `NativeWindow` 只提供 handle、identity、extent、scale factor 和 redraw/present hooks；
- 出现 App Server method、workspace state、widget、paint scene 或 GPU resource 意味着 ownership
  已漂移。

## 3. 测试与限制

```bash
cargo test --manifest-path zeta-rs/Cargo.toml -p zeta-winit
```

当前单元测试只验证无平台依赖的值语义。CI 编译不能代替 macOS、Windows、Linux 的真实窗口、
resume/suspend、DPI 与多窗口 smoke。

当前没有产品 event handler、默认窗口策略、clipboard、IME、accessibility、drag/drop 或
platform menu；这些能力必须在出现明确 owner 与 representative product vertical 后分层加入。
