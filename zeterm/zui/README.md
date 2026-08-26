# `zui` 开发文档

`zui` 是 Rust 桌面应用唯一需要依赖的原生 UI framework crate。它拥有 UI 内核、Application 与多窗口生命周期、任务和定时器、平台事件归一化、系统服务、托盘和全局快捷键、协议 URL、资源与隔离进程、OS accessibility、应用分发工具、渲染器契约、默认 wgpu 后端与确定性 testing；这些职责在同一 crate 内按能力目录隔离，不通过 sibling crate 暴露替代入口。

产品通过 `zui::app` 启动应用，通过 `zui::window` 和 `zui::input` 接收 ZUI 自有事件，通过 `zui::ui` 构造 scene。Native UI 的编写、布局、样式和主题投影边界见 [`native-ui-authoring.md`](../docs/native-ui-authoring.md)。`zeta-ui` 在它之上提供可复用组件和 zeterm pane topology；产品状态、Session、PTY、App Server 与业务 reducer 不得进入 `zui`。

## 1. Crate 边界

| 能力 | 规范公共入口 | 内部 owner |
| --- | --- | --- |
| Geometry、Element、layout、text、scene、inspection | `zui::ui` | `ui/foundation` / `ui/layout` / `ui/text` / `ui/presentation` |
| Interaction、animation、deadline、retained lifecycle | `zui::runtime`，并由 `zui::ui` 聚合常用类型 | `runtime` |
| Application、多窗口 lifecycle、退出策略与跨线程投递 | `zui::app` | `app` |
| 后台任务、作用域取消与 event-loop timer | `zui::runtime`；`zui::task` 是兼容入口 | `runtime/task.rs` / `runtime/timer.rs` |
| Window、event、theme、cursor、文件拖放与 chrome capability | `zui::window` | `window` |
| Keyboard、pointer 与 IME | `zui::input` | `input`；pointer/IME 事件由 `window/event.rs` 统一拥有 |
| Clipboard、dialog、opener、notification、menu、tray 与 global shortcut | `zui::services` | `services` |
| Packaged resource 与 shell-free child process | `zui::services` | `services/resource.rs` / `services/process.rs` |
| Signed update check、staging 与 installer handoff | `zui::services` | `services/update.rs` |
| Custom protocol URL 启动与转发 | `zui::app::ProtocolScheme` / `App::open_url` | `app/protocol.rs` / `app/runtime_event.rs` |
| Bundle、协议声明与 native installer | `zui::distribution` / `zui-packager` | `distribution` |
| Bounded runtime trace 与 live snapshot | `zui::devtools` | `devtools.rs` |
| OS accessibility tree 与 action 回流 | `zui::accessibility` / `App::accessibility_action` | `accessibility.rs` |
| Renderer、factory 与 presentation target | `zui::render` | `render` |
| 默认 GPU composition | `zui::app::Application::run` | private `render/wgpu` |
| 常用最小导入 | `zui::prelude` | `prelude.rs` |
| 手动时钟、确定性 lifecycle/timer 与 headless renderer | `zui::testing`；`zui::testkit` 是兼容别名 | `testing` |
| Button、List、ContextMenu 与产品 pane topology | 不属于 `zui` | `zeta-ui` / 产品 crate |

`src/lib.rs` 只声明这些同名能力模块，不再通过 `api.rs` 拼装第二套目录。根级类型导出、`zui::task` 和 `zui::testkit` 暂时作为现有消费者兼容入口保留；新代码使用上表的规范入口。

## 2. 平台抽象

公共 API 不导出 `winit::WindowEvent`、`winit::WindowAttributes`、`winit::EventLoopProxy`、`winit::WindowId` 或 concrete window。`window/event.rs` 与 `input/keyboard.rs` 在各自 owner 内把 winit 事件转换为 ZUI 自有的 `zui::window::WindowEvent`、`zui::input::KeyEvent`、`ModifiersState`、`Ime` 和 pointer value；应用只处理转换后的稳定语义。

未形成 ZUI 语义的平台事件转换为 `WindowEvent::Other`。redraw 由 Application runtime 单独分派给 `App::redraw`；resize 和 scale factor 在调用产品 callback 前同步到 renderer 与 `WindowMetrics`。

`zui::window::WindowOptions` 只表达 ZUI 支持的窗口策略，不接受完整 native attribute bag。`WindowHandle` 是 non-owning capability：framework registry 始终拥有实际 window 与 renderer，产品可以请求 redraw、修改 cursor/title/theme/IME 或发起 window drag，但不能延长 native window 生命周期。

`zui::render::RenderWindow` 是传给自定义 `RendererFactory` 的 opaque presentation target。第三方图形后端通过标准 `raw_window_handle::HasWindowHandle` 和 `HasDisplayHandle` 读取 surface capability，不获得 winit 类型或 Application runtime ownership。

## 3. 物理目录与依赖方向

```text
src/
├── app.rs + app/                    Application、context、lifecycle、protocol、event loop
├── window.rs + window/              window value、event、chrome、native owner、runtime registry
├── input.rs + input/                keyboard 与输入契约
├── ui.rs + ui/                      foundation、layout、text、presentation
├── runtime.rs + runtime/            interaction、animation、retained、task、timer
├── render.rs + render/              renderer contract、factory、private wgpu backend
├── services.rs + services/          可注入系统能力及默认实现
├── accessibility.rs + accessibility/ ZUI 语义到 AccessKit 的映射
├── distribution.rs + distribution/ bundle、installer、签名与 notarization tooling
├── testing.rs + testing/            deterministic runtime 与 headless renderer
├── devtools.rs + devtools/           bounded diagnostics
├── prelude.rs                       最小常用导入
├── task.rs                          旧 `zui::task` 兼容入口
└── internal.rs                      crate-private native integration bridge
```

| 模块 | 负责 | 禁止 |
| --- | --- | --- |
| `ui/foundation` | dependency-free value types、identity、geometry、color、icon asset | window、GPU、component 或产品状态 |
| `ui/layout` | 通用 Split/Grid geometry 与 resize constraints | pane 产品语义、窗口状态、绘制 |
| `ui/text` | font catalog、shaping、logical text/input geometry 与 editing | window event、GPU glyph atlas |
| `ui/presentation` | Element、computed layout、paint primitive、inspection 与 immutable ordered scene | event loop、surface、输入分发 |
| `runtime` | interaction、animation、deadline、frame invalidation 与 retained fragment lifecycle | presentation/renderer ownership、产品 reducer |
| `render` | backend-neutral renderer contract 与 factory | 产品状态、输入分发、accessibility owner |
| `render/wgpu` | physical conversion、pipeline、atlas、shader、surface recovery 与 present | 产品状态、layout、interaction/accessibility |
| `window` / `input` | winit adapter、native window owner、ZUI event conversion 与 live capability | scene、产品状态 |
| `app` | App callbacks、window registry、event-loop orchestration 与退出策略 | 产品领域状态、具体组件 |
| `services` | 可注入系统服务、进程隔离与更新 | 产品状态、发布凭证 |
| `distribution` | bundle layout、OS protocol metadata、installer plan 与 direct tool invocation | 签名密钥、发布凭证、产品更新策略 |
| `testing` | 手动 clock、headless window/renderer、确定性 event/timer queue | native event loop、真实系统服务、产品断言逻辑 |

能力根文件负责模块声明和规范 re-export，具体状态与算法进入同名目录。`internal.rs` 只能连接 crate-private native 类型，不能重新成为 `platform` 式的综合 owner；若新增代码找不到清晰能力目录，应先修正 ownership。

## 4. Feature 边界

| Feature | 提供 | 适用场景 |
| --- | --- | --- |
| 无 feature | backend-neutral UI、layout、text、scene、runtime 与 `Renderer` | 组件 crate、headless tests、替代 host |
| `native` | Application、task/timer、system services、accessibility、distribution tooling、testing、ZUI-owned input/window contracts 与 private native adapter | 自定义 renderer 的原生应用及 runtime tests |
| `wgpu`（default） | `native` + 默认 wgpu renderer composition | 开箱即用的桌面应用 |

组件库使用 `zui = { default-features = false }`。需要原生 Application runtime 但不使用默认 GPU backend 的 host 使用 `features = ["native"]`。普通桌面产品启用默认 feature。

## 5. Application 执行路径

```text
zui::app::Application::run
  → 创建 typed event loop、AppProxy、task/timer runtime 与 injectable services
  → zui::app::App::resumed
      → AppContext::open_window
          → private window::NativeWindow
          → 首次显示前创建 private AccessKit adapter
          → RendererFactory::create(RenderWindow)
          → window runtime registry
  → private winit WindowEvent
      → window::WindowEvent::from_native
      → framework 同步 physical extent 与 scale factor
      → App::window_event / App::redraw
      → WindowContext::present_scene(scene, accessibility)
          → 同步 OS accessibility tree
          → dyn Renderer::render_scene
          → RenderOutcome::Retry 时重新请求 redraw
  → exiting
      → 取消 application/window scoped work
      → 释放每个 window 的 accessibility、renderer 与 native resources
```

`ApplicationHandle::proxy` 返回 `AppProxy<T>`，worker 可以通过 `send_event` 唤醒主线程。event loop 已退出时返回包含原事件的 `AppDisconnected<T>`；Application 启动或运行失败返回不泄漏 winit 类型的 `ApplicationRunError`。`ApplicationExit` 分离最终产品状态与 runtime 内记录的 fatal `ApplicationError`。

默认 `Application::run` 使用私有 wgpu backend。测试或第三方 backend 实现公开的 `Renderer` 与 `RendererFactory`，再使用 `Application::run_with_renderer` 或 `ApplicationBuilder::with_renderer` 注入；组件与产品 scene 构造不改变。Clipboard、file dialog、opener、notification、menu、tray、global shortcut、resource 与 process 都能通过 builder 注入替代实现。

`BackgroundExecutor` 把 `Send` future 放到命名 worker thread 执行，并把完成值投递回 `App::user_event`。`TaskScope::Window` 在窗口关闭时取消，application scope 在退出时取消；丢弃 `Task` 也会取消，显式 `detach` 才保留。`TimerScheduler` 使用 native event loop deadline，不为每个 timer 创建线程；相同 deadline 按稳定 ID 顺序投递，窗口关闭与 application 退出会清理对应 scope。

## 6. 系统服务与 accessibility

产品从 `ApplicationHandle::services`、`AppContext::services` 或 `WindowContext::services` 获取 typed capability，不依赖具体 backend crate。`ApplicationBuilder::with_file_dialogs`、`with_opener`、`with_notifications` 和 `with_menus` 用于测试替身或产品定制；URL 与 menu identity 在进入 backend 前已转换成 ZUI-owned value。native file hover、hover cancel 和 drop 由 `zui::window::WindowEvent` 直接携带平台路径，不要求产品接触 winit。

Tray identity、RGBA artwork、pointer event、shortcut accelerator 和 shortcut event 都是 ZUI-owned value。启用 `native` 时，默认 tray backend 在 macOS/Windows 直接运行，在 Linux 由专用 GTK/AppIndicator 线程拥有 native tray；默认 global shortcut backend 在 macOS、Windows 与 X11 使用 native hotkey，在 Wayland 自动切换到 XDG GlobalShortcuts portal。portal 缺失、用户拒绝或只接受部分注册时均返回错误，不会静默降级成应用内快捷键。

`ResourcePath` 拒绝绝对路径和父目录穿越，`SystemResourceLocator` 识别 macOS bundle 的 `Contents/Resources` 与 executable sibling `resources`。`ProcessCommand` 从不调用 shell、保留参数边界、可清空继承环境，并由 `ChildProcess` 默认执行 terminate-on-drop。显式 `ProcessSandboxPolicy` 分别表达文件系统与网络权限；默认 backend 在 macOS 使用 Seatbelt、在 Linux 使用 Bubblewrap、在 Windows 通过随 bundle 安装的 `zui-appcontainer-runner.exe` 创建 AppContainer、ACL 与受 job object 约束的 child。受限策略如果无法建立对应 backend 就返回错误，`SystemProcesses` 也拒绝任何试图返回 `Unrestricted` 的降级实现。Windows 默认 backend 支持“只读文件系统 + 禁网”和“工作目录可写 + 禁网”；AppContainer 无法诚实表达的权限组合会 fail closed，产品仍可注入更严格的企业 backend。

`ApplicationBuilder::with_protocol_scheme` 只接收显式允许 scheme 的启动参数，`AppProxy::send_open_url` 用于 single-instance 或平台 bridge 转发后续 URL，最终统一进入 `App::open_url`。`BundleBuilder` 把相同的 `ProtocolScheme` 写入 macOS `Info.plist`、Linux desktop MIME handler 或 Windows 注册脚本；WiX installer 定义把 Windows scheme 写入每用户 registry。runtime 只处理启动语义，不能代替安装时的系统注册。

`SignedHttpUpdater` 对 manifest 的原始 payload 执行 strict Ed25519 verification，再按目标平台选择 artifact；下载完成后必须通过 manifest 中的 SHA-256 才能原子进入 staging。`UpdateInstaller` 只接收已经验证的 `StagedUpdate`，默认 backend 交给操作系统打开 installer，也可注入企业部署或测试实现。HTTP check/download 是阻塞服务，产品通过 `BackgroundExecutor` 调用，不能阻塞 UI callback。

`zui::devtools::DiagnosticsHandle` 提供有界、按序的 runtime trace 和即时 snapshot。它跟踪窗口 metrics、帧数、最近 scene primitive/accessibility 数量、活跃 task/timer 以及 lifecycle、menu、tray、shortcut、protocol URL 和 accessibility action；容量由 `ApplicationBuilder::with_diagnostics_capacity` 控制，`DiagnosticsSink` 可把事件流接到日志或开发工具。调用 `ApplicationBuilder::with_diagnostics_inspection` 后，最近一帧的完整 `InspectionFrame` 也会保留在 `SceneDiagnostics` 中；默认关闭以避免每帧复制节点。每个 runtime window 都提供共享的 `DevToolsHandle`，因此产品可以直接调用 `WindowContext::{open_devtools, close_devtools, toggle_devtools}` 或 `WindowHandle` 上的同名方法；这些调用会由 zui 创建/销毁一个独立的默认 DevTools 原生窗口，并把产品最近提交的 scene 作为 Inspector 数据源。快捷键、工具栏和拾取路由由 zui 统一维护，产品不需要再复制 inspector 状态。默认 Inspector 所需的通用 SVG（Pick、Close、展开/折叠 Chevron）也编译进 `zui`，其他 App 不需要提供资源；产品图标目录仍由 `zeta-icons` 负责。`InspectionSelection`、`InspectorState` 与 `DevToolsHandle` 仍是 product-neutral 的会话 contract；zui 提供默认 Inspector 视图，完整显示 `InspectionFrame` 节点树，支持展开/折叠，并在 hover 或选中深层节点时自动展开祖先、滚动定位；zeta-ui 或产品可以在此基础上提供主题和扩展。snapshot 不持有 native window、renderer 或产品状态。

`InteractionFrame::accessibility_nodes` 是语义树的唯一来源。`WindowContext::present_scene` 在绘制同一帧时把该快照映射为 AccessKit tree；adapter 在窗口第一次可见前创建。ZUI 内的 `AccessibilityNode` bounds 保持逻辑像素，bridge 在边界处按当前 `WindowMetrics::scale_factor` 转换为 AccessKit 所需的窗口物理像素。OS 请求的 Focus/Click 只有在 root tree 且目标节点确实声明对应 action 时才转换成 `AccessibilityActionKind::{Focus, Activate}` 并回到 `App::accessibility_action`，产品继续通过现有 `UiDispatch` 与 reducer 处理，不产生第二套控件身份。renderer 仍只消费 `UiScene`，不拥有 accessibility。

## 7. 分发工具链

`BundleManifest` 是 library API 与 `zui-packager` CLI 的共同输入。JSON 中的 executable、icon 和 resource source 相对于 manifest 文件目录解析；`ResourcePath` 继续约束 bundle 内 destination。生成器只接受普通文件和目录，拒绝输入 symlink、路径穿越和既有输出，因此失败重试不会覆盖发布目录。

| 目标 | Bundle 产物 | 协议声明 | Installer backend |
| --- | --- | --- | --- |
| macOS | `<name>.app` | `Contents/Info.plist` 的 `CFBundleURLTypes` | `/usr/bin/pkgbuild` 生成 `.pkg` |
| Linux | `<name>.AppDir`，含 `AppRun` | 根 `.desktop` 与 `usr/share/applications` MIME handler | `appimagetool` 生成 `.AppImage` |
| Windows | `<name>-windows`，含可选 AppContainer runner | 显式 `register-protocols.ps1` | WiX 4 `.wxs` + `wix build` 生成每用户 `.msi` |

最小 manifest 见 [`examples/bundle-manifest.json`](examples/bundle-manifest.json)。Windows 产品若使用默认严格 sandbox，必须设置 `windows_appcontainer_runner`；packager 会验证并复制 helper。`bundle` 只生成可检查的目录；`installer` 先生成同一 bundle，再直接执行当前目标的外部工具，不经过 shell；`release` 在 installer 之后执行平台签名与验收：

```bash
cargo run -p zui --bin zui-packager -- bundle zeterm/zui/examples/bundle-manifest.json dist macos
cargo run -p zui --bin zui-packager -- installer zeterm/zui/examples/bundle-manifest.json dist macos
cargo run -p zui --bin zui-packager -- release zeterm/zui/examples/bundle-manifest.json dist macos
```

`InstallerBuilder::prepare` 可让发布系统先检查 `InstallerPlan`，`InstallerBuilder::execute` 再通过可注入 `InstallerTool` 执行。`ArtifactSigner` 同样把 signing plan 与 execution 分开，并通过可注入 `SigningTool` 保留确定性测试入口。默认实现不经过 shell，要求每条命令成功、声明的产物存在，并在完成后运行平台验证：macOS 使用 hardened-runtime codesign、`productsign`、`notarytool --wait`、stapling 与 Gatekeeper 验收；Windows 使用 SignTool 的 SHA-256 Authenticode、RFC 3161 timestamp 与 verify；Linux 生成并验证 armored GPG detached signature。

仓库的 `.github/workflows/zui-distribution.yml` 在三平台 PR/main 构建 unsigned installer，在 `zui-v*` tag 导入临时凭证、调用 `release`、上传平台产物并创建 GitHub Release。凭证只从 Actions secrets 注入，不进入 manifest 或源码：

| 平台 | `zui-packager release` 环境变量 | CI secret |
| --- | --- | --- |
| macOS | `ZUI_MACOS_APPLICATION_IDENTITY`、`ZUI_MACOS_INSTALLER_IDENTITY`、`ZUI_MACOS_NOTARY_PROFILE` | `ZUI_MACOS_CERTIFICATE_P12_BASE64`、`ZUI_MACOS_CERTIFICATE_PASSWORD`、`ZUI_MACOS_APPLICATION_IDENTITY`、`ZUI_MACOS_INSTALLER_IDENTITY`、`ZUI_MACOS_NOTARY_APPLE_ID`、`ZUI_MACOS_NOTARY_TEAM_ID`、`ZUI_MACOS_NOTARY_PASSWORD` |
| Windows | `ZUI_WINDOWS_CERTIFICATE_SHA1`、`ZUI_WINDOWS_TIMESTAMP_URL` | `ZUI_WINDOWS_CERTIFICATE_PFX_BASE64`、`ZUI_WINDOWS_CERTIFICATE_PASSWORD` |
| Linux | `ZUI_LINUX_GPG_KEY_ID` | `ZUI_LINUX_GPG_PRIVATE_KEY_BASE64`、`ZUI_LINUX_GPG_KEY_ID` |

ZUI 拥有签名流程与验证契约，但不拥有签名身份、私钥或发布权限；缺少任何 tag-release 凭证会直接失败。

## 8. Testing

`zui::testing::TestRuntime` 不创建窗口或系统事件循环。测试显式选择起始 `Instant`，再驱动 resume、open/close、redraw、typed event、scoped timer 与 exit；`advance` 不 sleep，并按 deadline/ID 稳定投递。`HeadlessRenderer` 实现正式 `Renderer` contract，记录 target 配置和完整 immutable scene；`TestWindow` 同时保存最近的 accessibility snapshot。`zui::testkit` 只是迁移期兼容别名。

这套工具验证 runtime policy 和 presentation 输出，但不会伪装真实 OS 行为。窗口 chrome、系统 dialog、VoiceOver/Narrator/Orca 集成和 GPU surface recovery 仍需对应平台 smoke/integration test。

## 9. Scene 与 renderer 不变量

- 所有 geometry、font size 和 scene primitive 使用 logical UI pixels，DPI 转换只发生在 renderer；
- `UiScene` 保存 clip、composition layer 和跨 primitive 类型的精确 back-to-front 顺序；
- renderer 只消费 immutable `UiScene`，不得接收 interaction/accessibility frame；
- component 只产生 Element 与 scene primitive，不接受 device、queue、surface 或 render pass；
- backend 不重新解释产品 layout、component identity、focus 或 command；
- component/Application API 不暴露 `wgpu::*` 或 `winit::*` 对象；
- 新 backend-specific 能力只有形成稳定的跨后端语义后才能进入 `Renderer` contract。

## 10. 修改规则

- **新 public API**：放入同名能力根文件和目录，同时决定是否需要根级兼容导出；不能恢复 `api.rs` 聚合 façade 或 flat-only API。
- **新平台事件**：先在 `window` 或 `input` 定义 ZUI-owned value，再在同一 owner 内完成 private native conversion；产品不能匹配 winit variant。
- **新窗口能力**：由 `WindowOptions` 或 `WindowHandle` 暴露，不得让产品持有 native window owner。
- **新系统能力**：先在 `services` 定义可注入的 ZUI-owned trait/value，再把具体 crate 限制在该能力的默认 backend。
- **新 bundle/installer 格式**：扩展 `zui::distribution` contract 与 `distribution` owner，保持 CLI 与 library 使用同一 manifest；签名凭证不能进入源码或 runtime service。
- **新 accessibility 语义**：扩展 `AccessibilityNode` 与 AccessKit mapper；不得让 component 或产品依赖 AccessKit。
- **新 primitive 或 batch ordering**：先修改 `ui/presentation` contract，再同步所有 renderer。
- **新 layout 算法**：放入 `ui/layout`，输入必须是 caller-owned state，输出必须是 immutable geometry。
- **新 renderer**：实现 public trait，通过 factory 注入；默认 backend 实现保持 private。
- **新通用组件**：放入 `zeta-ui`；产品专属 surface/state 留在产品 crate。
- **新 product icon artwork/语义目录**：放入 `zeta-icons`，通用 icon value contract 只在 `ui/foundation/icon` 演进；仅服务于 zui 默认 DevTools 的内置 artwork 放在 `devtools/assets`，避免 zui 反向依赖 product icon crate。
- **文件规模**：production Rust module 不超过 500 行，超过时按单一职责拆出 owned submodule。

`architecture_tests.rs` 固定能力目录、同名 public root、backend-neutral dependency direction、native dependency owner、无 `mod.rs`、500 行上限、旧技术层目录不得回流，以及 public API 不导出 native backend type。修改 ownership 时同步这些测试与本文。

## 11. 当前能力与剩余边界

当前已实现单 crate 分发、能力目录与公共命名空间一一对应、ZUI-owned event/window/proxy/文件拖放 contract、多窗口 lifecycle 与退出策略、默认 wgpu backend、renderer/service injection、scoped task/timer、三平台 tray/global shortcut、resource/process、三平台严格 sandbox policy、signed updater、protocol URL lifecycle、bundle/installer/signing/notarization tooling、三平台发布 workflow、bounded diagnostics/devtools、zui-owned 独立 DevTools 原生窗口与默认 Inspector 视图、窗口级直接 DevTools 调用与可复用 inspector session state、AccessKit publication/action routing，以及 deterministic testing/headless renderer。`zui-native-demo` 是第二个独立 App consumer，持续编译双窗口、任务、定时器、menu、tray、global shortcut、protocol URL、diagnostics 与 accessibility 路径。

这使 `zui` 具备 Electron 类原生应用 framework 的完整核心职责边界，但不等于 Electron 兼容层。仍然明确保留的边界是：root compatibility exports 尚未进入正式移除周期；未知平台事件只保留 `WindowEvent::Other`；`WindowOptions` 只覆盖已有真实消费者的策略；Linux application menu 还没有可依附 winit window 的 GTK native menubar（tray menu 已完整支持）；Windows AppContainer 只接受能够无降级表达的权限组合；真实 tray、portal、accessibility、签名账户与 OS 安装验收仍由对应平台 CI/smoke test 负责。当前 DevTools 是 zui-owned native scene/runtime Inspector：它可以独立开窗、读取完整 scene inspection hierarchy、展开/折叠并定位节点、拾取节点和展示布局元数据，同时把 hover、选中节点的 outline/padding/gap overlay 画回产品 scene；但不提供也不计划伪装 Chromium DOM/CSS/JavaScript debugger。

后续能力继续采用同一准则：先形成 ZUI-owned contract 和可注入测试替身，再接具体平台 backend。资源打包、安装器、自动更新与开发工具属于 SDK/tooling 层，不能把产品组件或产品状态收进 `zui`。

## 12. 验证

```bash
cargo check -p zui --no-default-features
cargo check -p zui --no-default-features --features native
cargo test -p zui --no-default-features --features native --lib
cargo clippy -p zui --no-default-features --features native --all-targets -- -D warnings
cargo check -p zui --no-default-features --features native --bins
cargo check -p zui --no-default-features --features native --target x86_64-pc-windows-gnu --lib --bins
bazel test //zeterm/zui:zui-unit-tests
cargo test -p zeta-ui
cargo check -p zui-demo --features native --bin zui-native-demo
cargo test -p zeterm
```

`zui-demo` 是不依赖终端、App Server 或产品 icon catalog 的最小宿主，用来验证 public namespaces、scene contract、renderer 替换和默认 native Application composition。其 native binary 还覆盖双窗口、task/timer、menu 与 accessibility publication。
