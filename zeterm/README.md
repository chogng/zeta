# `zeterm`

> Native 文本输入的跨 crate canonical ownership 见 [`docs/native-text-input.md`](docs/native-text-input.md)；Agent 开发能力、机器反馈与人类观测原则见 [`docs/native-agent-console.md`](docs/native-agent-console.md)；Agent Terminal 主窗口布局见 [`docs/native-layout.md`](docs/native-layout.md)。Terminal compatibility 细节见 [`docs/native-terminal-ui.md`](docs/native-terminal-ui.md)；本 README 只拥有 native host 当前源码路径和接入义务。Terminal grid 与 BlockList 的实现契约见 [`zeta-terminal` README](../zeta-rs/terminal/README.md)；CodeEditor/DiffEditor presentation 契约见 [`zeta-editor` README](editor/README.md)；文件保存基线与冲突状态契约见 [`zeta-text-file` README](../zeta-rs/text-file/README.md)。UI 到 GPU 的 canonical 边界见 [`docs/rendering-architecture.md`](docs/rendering-architecture.md)。

`zeterm` 是与 `zeta` Electron Desktop 和 `zeta code` TUI 同级的纯 Rust Desktop 产品入口，在发布边界导出为 `zeterm`。三条产品线的宿主边界见 [`docs/product-lines.md`](../docs/product-lines.md)。

产品按照“提高 Agent 完成代码变更闭环的能力”选择底层 capability，并只为结果、影响、风险和用户介入提供最小 Human Surface；纯只读内部查询不自动产生 GUI。单一公共 `zui` crate 统一提供 application/window lifecycle、renderer、事件与平台能力，并在 crate 内按 `app/window/input/ui/runtime/render/services` 等同名能力目录隔离。产品实现 `zui::app::App`，在原生窗口中组合 `zeta-ui` 与 Agent ThreadTimeline、Agent/Shell Composer。
启动时 `zeta-theme` 从共享 design-token manifest、`zeterm` 默认主题入口与 device-local 用户主题生成不可变 snapshot；
`shell_style` 将其适配为 Shell、CodeEditor、MultiDiffEditor、scrollbar 与 terminal ANSI palette。
`zeta-ui::layout` 拥有 Root/Inspector/Terminal Workspace 的 pane topology；`zeta-composer` 拥有 Composer
输入、自动路由、Shell history/completion、Slash/模型交互、滚动状态及 panel/list 几何；`ShellLayout` 和
`composer_panel` 只保留 Shell product composition 的区域背景、边界、绘制与宿主接线。
窗口级产品 pane topology 位于 [`zeta-ui::layout`](ui/README.md)；通用框架可脱离产品运行的
最小链路由 [`zui-demo`](zui-demo/README.md) 持续验证。

## 构建与发布边界

仓库根 `Cargo.toml` 是唯一 Cargo workspace；`zeterm/Cargo.toml` 是 `zeterm` 产品 package 和发布边界：

```bash
cargo check --manifest-path Cargo.toml -p zeterm
cargo test --manifest-path Cargo.toml -p zeterm
cargo run --manifest-path Cargo.toml -p zeterm
```

Debug 构建的本地 Agent Session 默认把当前 `zeterm` executable 作为 profile daemon 的进程载体，确保 `cargo run -p zeterm` 使用与产品 host 同一次构建的 App Server graph；显式 `ZETA_APP_SERVER_DAEMON_PATH` 仍优先，release 构建继续由发布环境选择独立 daemon binary。

根 Cargo profile 对完整依赖图和 Native/App Server 大型链接单元应用轻量 size optimization，在保留 limited line-table 调试信息的同时，使 macOS debug binary 的 unwind metadata 保持在 compact-unwind 编码上限内。

`zeterm` 通过根 workspace 的 path dependencies 使用 shared backend；`zeta-rs` 不反向依赖 zeterm 或
Native UI。Bazel 从同一个根 Cargo graph 生成 `@crates`，并以 `//zeterm:zeterm`、
`//zeterm:zeterm_sources`、`//zeterm:zeterm_release_inputs` 和
`//zeterm:zeterm_ci` 拥有产品 target、输入、发布契约与 boundary check。zeterm 仍是独立产品目录和
发布入口，但不再是第二个 workspace 根；
当前完整 zeterm binary 的 macOS 验证仍受 apple-cf Swift bridge 在沙箱中启动 SwiftPM 的环境限制，
Cargo workspace check/test 是产品源码的首要验证入口。发布 staging 入口是：

```bash
just zeterm-package --package-dir /absolute/path/to/zeterm-package --zeterm-bin /path/to/zeterm
bazel test //zeterm:zeterm_ci
```

平台 release job 使用 `ZETERM_PACKAGE_DIR`、`ZETERM_PLATFORM` 以及对应的 signing identity 环境
调用 `build/release/release_zeterm_package.sh`；该入口完成签名和验签后才留下 `verified` artifact。

package staging 只生成 unsigned artifact、binary digest 和可选的 binary-bound Remote runtime
bundle；签名、平台验证与发布顺序见
[`zeterm` release graph](docs/zeterm-release-graph.md)。

若发布包要支持“本机只安装 zeterm，远端尚未安装 zeta”，先把各 POSIX target 的 canonical
packaged-node Zeta package 组装成确定性 Remote runtime bundle，再在构建 zeterm 时绑定它：

```bash
python3 -B build/release/build_remote_runtime_bundle.py \
  --bundle-dir /absolute/path/to/remote-runtimes \
  --package-dir /packages/x86_64-unknown-linux-gnu \
  --package-dir /packages/aarch64-unknown-linux-gnu

just zeterm-package \
  --package-dir /absolute/path/to/zeterm-package \
  --remote-runtime-bundle /absolute/path/to/remote-runtimes
```

package builder 会把 catalog SHA-256 编译进 zeterm binary，再把 catalog 与 archive 放到
`zeta-remote-runtimes/`。staging 和 signing 都会确认 binary 实际包含该摘要；catalog 再以每个
archive 的大小、展开大小和 SHA-256 绑定远端代码。使用预构建 `--zeterm-bin` 时，该 binary 也必须
已用同一 `ZETERM_REMOTE_RUNTIME_CATALOG_SHA256` 编译，否则 staging 拒绝。release script 可通过
`ZETERM_REMOTE_RUNTIME_BUNDLE` 接入同一 bundle。

不希望把所有 POSIX runtime 装进 zeterm 产品包时，发布 job 可只绑定已发布 catalog：

```bash
just zeterm-package \
  --package-dir /absolute/path/to/zeterm-package \
  --remote-runtime-catalog-url https://releases.example/zeta/<version>/catalog.json \
  --remote-runtime-catalog-sha256 <catalog-digest>
```

builder 会把 URL 和摘要都通过 `ZETERM_REMOTE_RUNTIME_CATALOG_URL` /
`ZETERM_REMOTE_RUNTIME_CATALOG_SHA256` 编译进 binary；staging 和 signing 验证两者确实存在于签名
artifact，运行时再由共享 updater 直连公共 HTTPS、拒绝重定向/私网目标，并在本机内容寻址缓存中完成
完整 archive 验证后交给 SSH installer。URL 不是用户配置，也不能由远端或 Native UI 替换。

## 宿主边界与 frame ownership

`zeterm` 是产品宿主，不是第二个 UI framework。它保留窗口、平台事件、产品状态映射、App Server
适配、command 执行以及具体 Shell/Composer/Inspector Part 的组合；通用 layout、scene、inspection、
interaction、animation、deadline 和 retained lifecycle 由 `zui` canonical owner 提供。跨 crate 的
阶段、弃用范围和删除条件见 [`docs/zeterm-app-migration-plan.md`](docs/zeterm-app-migration-plan.md)。

| 边界 | Native 当前状态 | 维护规则 |
| --- | --- | --- |
| 产品状态、平台事件、App Server/文件/Git/Session adapter | ✅ 当前 owner | 可以在 Native 演进，不迁入 `zui` |
| Shell/Composer/Inspector 的产品组合 | ✅ 当前 owner | 组合 `zui`/`zeta-ui`，不复制组件内部 geometry 或 hit-test |
| `ShellPresentation` 的 scene/interaction/accessibility 聚合 | `UiFrame<InteractionFrame>` | frame 是唯一 owner；renderer、input 和 Inspector 通过窄 accessor 消费 |
| 低层产品 composition | `UiFrame::with_context` / `ComponentContext` | 只能在 scoped context 中注册交互和绘制；可复用组件使用 `draw_component` |
| 通用 frame、失效、动画和 retained lifecycle | `zui` 委托 | Native 只注册产品 fragment、推进 runtime 并投影 deadline |

因此不能把 `zeterm` binary 标成 deprecated：它是当前产品宿主。旧 split scene/interaction
composition API 已删除，后续不得在 Native 宿主重新引入平行输出字段。

## 所有权

| 能力 | 当前 owner | 状态 |
| --- | --- | --- |
| `zeterm` 发布名与用户可见显示 | `PRODUCT_DISPLAY_NAME` / Cargo `[[bin]]` | ✅ |
| 产品窗口标题、初始尺寸与 event meaning | `zeterm::NativeApp` | ✅；实现 `zui::app::App` |
| Application lifecycle、window registry、renderer initialization 与 event routing | `zui` | 委托 |
| Event loop 与 native window adapter | `zui::app` + private `window` integration | 委托；`WindowHandle` 不延长 runtime window lifecycle |
| 后端无关 render、resize、scale 与 frame outcome | `zui::render::Renderer` | 委托；`zui` 保存 renderer trait object |
| GPU surface、pipeline、atlas、shader、present 与 retry | `zui` private `render/wgpu` module | 委托；`zui::render::WgpuRendererFactory` 是默认 backend composition |
| Element、Rect/image/icon/text scene、检查快照与字体测量 | `zui` backend-neutral modules | 委托；产品不接触具体 GPU API |
| Shell product layout 与 scene composition | `zeta-ui::layout` + `zeta-composer` + `shell_scene` / `composer_panel` | 部分抽取；`zeta-ui::layout` 拥有 root/workspace pane geometry，`zeta-composer` 是 Composer 状态与几何的单一 owner，Native 仍拥有 scene composition |
| Native frame assembly | `ShellPresentation::frame` | ✅；由单一 `zui::ui::UiFrame<InteractionFrame>` owner 管理 |
| 原生布局检查模式、pointer 拦截与 highlight overlay | `layout_inspector::LayoutInspector` / `NativeApp` | ✅；Inspector Panel 是根 Grid leaf，只有产品节点高亮进入 overlay |
| Sessions/Main 单轴约束与 Sash presentation geometry | `zui::SplitViewLayout` / `zeta-ui::Sash` | 委托 |
| Terminal Workspace 与 SidebarPart 递归 Pane geometry | `zeta-ui::layout::TerminalWorkspaceLayout` + `zui::GridLayout` | 委托；外层布局消费 host-owned Terminal PaneGroup geometry，并保留可选 SidebarPart Leaf |
| Terminal/heterogeneous PaneGroup、PaneTree 与 active Pane | `zeterm/src/pane_group` + `zeterm/src/pane_input` | 本次先建立 type-agnostic PaneGroup 与 `PaneInput` host contract；Session Tab 选择主工作区 PaneGroup，SidebarPart 使用独立 workspace-scoped PaneGroup，Terminal Surface 内每个 TerminalPaneInput 绑定独立 terminal runtime，最后一个 Pane 不可关闭 |
| Pane-to-view/runtime binding | `zeterm/src/pane_host` + feature crates | `PaneHost` 按 `PaneHostScope`/`PaneInputKind` 产生 frame mount；Terminal 使用独立 `TerminalSessionKey`，SidebarPart 的 Files/Diff 复用各自 feature-owned PaneView，不能由 `zeta-ui` 统一持有 |
| Sessions preferred width、显隐与当前 drag snapshot | `session_sidebar::SessionSidebarState` | ✅ |
| SidebarPart 显隐、preferred width 与 resize gesture | `sidebar_part::SidebarPartState` / `zeta-ui::Sash` | ✅；宽度限制为 240–560px，不拥有内部 feature state |
| SidebarPart 导航与各 Pane 的布局 | `zeterm/src/pane_host` + [`zeta-agent-sidebar`](agent-sidebar/README.md) 的 `AgentSidebarNavigation` / `files::FilesLayout` / `scm::ScmLayout` | 委托；SidebarPart 只提供外层 shell slot，具体 Files/Diff view 由 feature crate 绘制 |
| Files 层级树与模糊搜索 | [`zeta-agent-sidebar`](agent-sidebar/README.md) 的 `files::FilesState` / `files::FilesPane` / `zeta-ui::TreeView` / `ListView` / `zeta-file-search` | ✅；目录懒加载、稳定 mounted-node ID、展开/收起和 24px 虚拟行已接入；zeterm 只适配 App Server 目录 DTO 并执行打开/加载动作 |
| UTF-8 文件保存 baseline、磁盘版本与外部变化冲突 | `zeta-text-file::TextFileLifecycle` | 委托；Native 只提供当前 editor text 与 I/O adapter |
| 语言服务 composition、持久化设置、文档/请求 freshness 与 presentation | `language_service_host::NativeLanguageService` / `language_server_settings_model::LanguageServerSettingsState` / `file_editor_language_features` / `zeta-language-service` | ✅；Rust/JSON/Shell 独立设置与 runtime state，diagnostics，latest-only pointer hover，Ctrl/Cmd+Space completion popup/安全 edit 接受和 F12 definition navigation 已接入；文件读取仍通过 App Server authority |
| 文件 Tab、active document、关闭决策与 presentation | `file_editor_host::FileEditorHost` / `file_editor_pane::FileEditorPane` / `file_editor_input::FileEditorInputState` | ✅；Explorer activation 已通过 typed `fs/readFile`/metadata 打开 language-aware document，Cmd/Ctrl+S 使用版本 preflight 和 `fs/writeFile`，中心 Editor Surface 已接通 tabs、关闭确认、外部重载/显式乐观覆盖、find/replace、自动缩进、soft wrap、focus、keyboard、IME、pointer、clipboard 与 visual-row viewport |
| Changed-file collection、整体滚动与每文件 diff 视口 | [`zeta-agent-sidebar`](agent-sidebar/README.md) 的 `ScmState` / `EditorPaneState` / `zeta-ui::ScrollState` | 委托；zeterm 只把 Git 投影映射为 `ScmDiff` 并执行刷新动作 |
| Titlebar 背景、窗口拖拽区、语言服务器设置入口与左右侧栏开关 | `titlebar::Titlebar` | ✅；不绘制可见窗口标题 |
| Top Bar 左右 sidebar toggle `ActionBar` | `titlebar::Titlebar` | ✅ |
| 原生窗口控件占位 | `zui::WindowControlInsets` | 委托；Titlebar 只增加自身内容间距 |
| 通用 Tab surface 与横/纵排列 | `zeta-ui::Tab` / `TabList` | 委托；不拥有 Session content 或 tabpanel |
| NavBar 导航容器（计划） | `zeta-ui` presentation composition + Native Titlebar/Sidebar host | 尚未作为独立 public component；只计划承载横向/纵向排列、slot 与 overflow/scroll geometry，不拥有 TabInput 或 provider state |
| 多行编辑、caret/selection、find/replace、自动缩进、undo/redo、IME 与 syntax projection | `zeta-editor::CodeEditorDocument` / `CodeEditor` | 委托；Composer 与中心文件 Editor 都只转交平台输入，editor 内部管理文本、搜索替换、缩进、parser/revision/token、fold 与 hit-test |
| 文本差异计算、行映射与字符级范围 | `zeta-diff::DiffDocument` | 委托；Native 不复制 diff 算法 |
| 单列/并排及多文件只读差异展示 | `zeta-editor::DiffEditorDocument` / `DiffEditor` / `MultiDiffEditor` | 委托；Native 只按文件扩展名选择 language，editor 内部维护两侧 parser/revision/token；Changes pane 使用窄栏 Unified presentation |
| 可折叠 Workbench Navigator、名称搜索与多 Session/Settings Tab | `session_sidebar_toolbar::SessionSidebarToolbar` / `session_search::SessionSearch` / `session_tab_list::WorkbenchTabList` / `tab_input::{TabInput,TabInputModel}` / `workbench` | ✅；`TabInputModel` 统一维护 Session 与 singleton Settings 的逻辑输入和 active selection，UI `ElementId` 只在 projection 边界分配，Add 通过 App Server 创建 Session/Thread 并绑定独立 Terminal PTY |
| 可折叠 SidebarPart 产品组合 | `SidebarPartState` + `pane_host::PaneHost` + `zeta-agent-sidebar` 的 `AgentSidebarNavigation` / `files::FilesToolbar` / `FilesPane` / `scm::EditorPane` | 委托；Native 只负责外层显隐、Pane binding、主题映射和 action 执行 |
| 锚点浮层定位与独立 layer 合成 | `zeta-ui::ContextView` | 委托；不拥有显示生命周期或输入路由 |
| 无边框下拉 surface、可选 header、item geometry 与默认选择 | `zeta-ui::Dropdown` | 委托；不拥有产品查询、选择 identity、关闭或 command |
| 柔和阴影、2px menu padding、4px radius、item geometry 与默认选择 | `zeta-ui::ContextMenu` | 委托；不拥有 Session identity、关闭或 command |
| Session Tab 右键菜单、关闭策略与 action 映射 | `session_context_menu::SessionContextMenu` / `SessionContextMenuState` | ✅；四个 action 已进入产品边界，Pin/Close/Rename/Fork mutation 尚未执行 |
| 共享主题加载与平台中立 snapshot | [`zeta-theme`](../zeta-rs/theme/README.md) | 委托；选择数据化 `zeterm` 默认入口，并与 Desktop/TUI 消费同一 manifest 和用户主题 JSON |
| Native Shell/CodeEditor/Diff/Terminal 主题投影 | `shell_style::ShellPalette` 与命名 component palette | ✅；启动时加载一次，失败回退内置浅色 palette |
| 稳定控件身份与 shell action 映射 | `shell_interaction` | ✅ |
| 稳定 product command identity、request 与注册式执行入口 | `zeta-commands::{ZetermCommandId, CommandRequest, CommandRegistry}` / `NativeApp::dispatch_command` | ✅；pointer、menu 与 shortcut 汇合到宿主注册的 handler |
| 平台按键转换、Chord 生命周期与当前 context | `keybindings::NativeKeybindings` | ✅；1.5 秒超时，失焦或 IME 事件取消 |
| 用户快捷键资源、验证与热更新 | `zeta-keybindings-host::KeybindingsResource` | ✅；读取 `<ZETA_PROFILE_ROOT>/keybindings.json`，坏更新保留上一份有效规则 |
| 快捷键设置、录制与 Chord 提示 | [`zeterm-keybinding-ui`](keybinding-ui/README.md) | 委托；zeterm 提供命令行、事件 adapter 与保存接线 |
| 平台无关快捷键模型、规则顺序与冲突解析 | [`zeta-keybinding`](../zeta-rs/keybinding/README.md) | 委托 |
| 命中、hover/press/capture、focus、键盘导航、cursor 与 accessibility semantics | `zui` | 委托 |
| accessibility semantics → 平台屏幕阅读器 | `zui` private AccessKit adapter | 委托；现有 tree/focus/action 随 `present_scene` 发布 |
| Transparent native chrome 与窗口拖动 adapter | `zui` private `window/chrome` integration | 委托 |
| ANSI parser、terminal grid 与 BlockList | `zeta-terminal::TerminalCore` | 委托 |
| 默认 shell PTY、output/exit event、write 与 resize | `terminal_session::TerminalSession` | ✅；Local 使用本地 PTY，Remote 使用 App Server `terminal/*` 协议；两者都在后台 worker 创建和轮询 |
| Session Tab 到 terminal runtime 的一对一 binding、活动/非活动 runtime 切换 | `terminal_workspace::TerminalWorkspace` | ✅；Local/Remote 共用 pending key、乱序 ready 和非活动 runtime 管理，不拥有 App Server Session/Thread authority |
| Agent ThreadTimeline + fixed Agent/Shell Composer | `shell_scene` / `thread_timeline` / `composer_panel` / `zeta-composer::Composer` | ✅ |
| Composer 状态、输入、路由、交互、滚动与 panel/list 几何 | [`zeta-composer`](composer/README.md) | ✅；Native 只适配 Thread/catalog、执行提交副作用并绘制 product chrome |
| Composer active View 与滚动状态 | `zeta-composer::Composer` / `zeta-ui::{ScrollView,ListView}` | ✅；Slash 与 `/model` active View、selection 和 viewport offset 由同一 Composer owner 保留；Shell completion 不使用该 Pane |
| App Server Session adapter 与 transient Thread/language projection / stream-gap recovery | `agent_session` / `agent_session_target` / `agent_session_remote` / `language_service_remote` / `language_service_remote_session` / `thread_projection` | 部分具备；本地 profile/Workspace authority 与 CLI Remote launch 均走同一 adapter，SSH profile 由 Native host 选择；本地 Session 与 Desktop/TUI 实时共享；Remote Agent 与独立 Remote language connection 均在 30 秒窗口内退避重连，断线期命令和语言请求不会延迟回放；命名连接 CLI、Native 选择器及图形新增/编辑/删除均已具备 |
| durable direct Shell Turn | App Server `session/request::StartShellTurn` / Core `StartShellTurn` | ✅ |
| 独立交互式 Terminal Surface | `workspace_surface` / `terminal_session` / `terminal_input` | ✅；`Cmd/Ctrl+J` 切换 |
| Composer/File Editor/Session Search/File Search/Workspace Path Search/Git Branch Search IME target、事件转换、启停、composition lifecycle 与候选框同步 | `input_method` | ✅ |
| Bottom Widget 底部的 Local/Remote/cwd/branch/Changes context toolbar | `input_context_toolbar::InputContextToolbar` / `workspace_path_picker` / `git_branch_context_menu` / `workspace_context::WorkspaceContext` | 部分具备；Remote 显示远端位置并禁止本地文件夹切换，Local 继续使用 picker；Changes/Git 仍由当前 App Server authority 投影 |
| shell bootstrap、host-owned command submit 与 zsh completion marker | `terminal_session::TerminalSession` | 部分具备 |
| terminal query reply → PTY write | `TerminalCore::take_reply_bytes` / `TerminalSession::handle_event` | ✅ |
| alternate-screen mouse cell mapping、button state 与 PTY report | `terminal_pointer::TerminalPointer` / `TerminalCore::encode_mouse` | ✅ |
| 主屏滚轮浏览、输出增长锚定与 Block/grid 视口投影 | `terminal_scrollback::TerminalScroll` / `terminal_output_scroll_view::TerminalOutputScrollView` / `zeta-ui::ScrollView` | ✅；Terminal 保留底部相对行锚定，通用基座负责裁剪、内容坐标和滚动条绘制 |
| 主屏拖拽选择、selection paint 与 cell-aware text extraction | `terminal_selection::TerminalSelection` | ✅ |
| system clipboard capability 与 copy/paste、bracketed-paste routing | `zui::ClipboardHandle` + `terminal_selection` / `terminal_input` | ✅；framework 拥有 backend，产品拥有按 focus/screen/mode 分流语义 |
| OSC title → Terminal Surface / native window title | `TerminalCore::title` / `NativeWindow::set_title` | ✅；后台 terminal title 不覆盖 Agent Session title |
| 完整 TUI compatibility | 尚无完整 owner | 尚未完成 |
| App Server Session 与 Thread projection | `agent_session` / `thread_projection` | ✅；Local 使用共享 `<profile_root>/state.sqlite3` 的 durable composition，启动时通过 `session/list` 恢复可用会话；Remote 使用远端 profile authority |

依赖方向：

```text
zeterm → zui::{app, window, input, ui, runtime, services, render}
            → zeta-ui → zui
            → zeta-commands
            → zeta-keybinding
            → zeterm-keybinding-ui → zeta-keybinding
                                  → zeta-ui → zui
            → zeta-app-server-client
            → zeta-remote → zeta-remote-connections → zeta-remote-host
            → zeta-protocol
            → zeta-terminal
            → zeta-text-file
            → zeta-diff
            → zeta-editor → zeta-ui → zui
                          → zeta-diff
                          → zeta-syntax
            → zeta-theme
            → zeta-file-search
            → zeta-slash-commands → zeta-app-server-protocol
            → zeta-utils-pty
```

`zeterm` 可以拥有可丢弃的 presentation state 和产品交互，但不能复制 Session、Thread、
Turn 或 Tool 的权威状态机。当前通过 `zeta-app-server-client` 的 typed contract 订阅与提交。

## 产品原则与当前边界

Native App 是让 Agent 获得完整工作区开发能力、让用户按结果观察和介入的原生开发环境。Agent 必须获得继续理解、修改、验证和恢复所需的结构化机器反馈；用户默认只看关键发现、Change Set、验证结果、风险和控制入口，不需要观察每次只读查询或完整 Tool log。Capability、task evidence 与 Human Surface 的 canonical 准入规则见 [`native-agent-console.md`](docs/native-agent-console.md)。当前代码仍把 Terminal 作为显式切换的 compatibility Surface；目标布局把 Terminal session 作为 Agent conversation 的执行上下文和主交互基座，但 PTY transcript 仍不是 Thread authority。下表把仍保留的历史 shell vocabulary 映射到当前产品语义：

| 当前源码 | 当前能力 | 目标产品语义 | 状态 |
| --- | --- | --- | --- |
| `titlebar::Titlebar` | 窗口拖拽区和左右 sidebar toggle `ActionBar`；不绘制标题文案 | Top Bar | ✅ |
| `zeta_ui::layout::RootLayout` | 用 `GridLayout` 解析固定 Product leaf 与可选 Inspector leaf | Native window 根布局 | ✅；窗口扩展后 Inspector 获得独立 360px sibling leaf，Product bounds 不变 |
| `ShellLayout` | 组合 titlebar、可选 Sessions sidebar，并把剩余区域交给 `TerminalWorkspaceLayout` | Top Bar 与 Workspace 外部布局 | ✅；Sessions 使用外层单轴 split |
| `zeta_ui::layout::PaneGroupLayout` / `zui::GridLayout` | 用递归 Grid 投影 Terminal PaneGroup 的每个 Pane 与 owning-split Sash | Workspace Pane geometry adapter | ✅；PaneTree mutation、Session-to-PTY binding 与逐 Pane runtime 仍由 Native host 负责 |
| `zeta-commands` / `command_dispatch` / `keybindings` / `keybindings_resource` / `keyboard_shortcuts` | 把 pointer/menu 的 `ElementId` 与标准化键盘事件映射到同一 `CommandRequest`，再交给 `CommandRegistry` 的宿主 handler；向 `zeterm-keybinding-ui` 提供命令行、稳定 identity 和保存 adapter | Product command 与快捷键输入层 | ✅；支持完整 `when` 表达式、冲突/错误诊断、最多四段 Chord 与 keycap UI |
| `shell_scene` / `thread_timeline` | Agent Surface 绘制 canonical Thread items；Terminal Surface 绘制活动 grid | Agent Workspace / Terminal compatibility | ✅ |
| `layout_inspector` | `Cmd/Ctrl+Shift+I` 开关检查面板，面板 cursor action 显式开关选取，点击锁定最深检查节点，Escape 先停止选取再关闭 | Native UI layout inspection | ✅；原生窗口向右扩展独立层级面板，自动显示 ancestor、authored row/column/width/height、computed size/padding/gap/radius、layer 与源码位置 |
| `zeta-composer` / `composer_host` / `composer_panel` / `terminal_input` | `Composer` 单一拥有 compact `CodeEditor` 输入、Agent/Shell 路由、Shell history/completion、Slash/模型 active View 与滚动；Native 只适配产品状态并执行 effect | Agent Composer、Slash/模型选择、Shell 补全与分类器选择的 Shell Turn | ✅ |
| `input_context_toolbar` / `workspace_path_picker` / `git_branch_context_menu` | `ActionBar` 排列 Local、cwd、branch 与 `Changes files • +additions -deletions`；cwd 组合 `Dropdown`，branch 组合 `ContextMenu`，两者均使用各自通用 header slot | Composer context toolbar | ✅；两个浮层第一行均默认聚焦 Search Box；目录或分支切换后替换 Files 根、文件搜索索引和 Git/Changes projection；Changes action 刷新 Git projection、展开右栏并选择 Changes Pane |
| `session_tab_list` | 组合 `zeta-ui::TabList` 投影 Session 与 Settings Workbench item；Session 自身拥有白色状态容器、会话名和工作区两行截断信息，Settings 使用 gear icon，并共享纵向 TabList/selected Tab 语义 | Workbench 导航 | 已支持动态 Session 创建/切换与 Settings 选择；每个 Session Tab 绑定独立 Terminal PTY |
| `session_sidebar_toolbar` / `session_search` | 整行组合 `SearchBox` 与右侧 `ActionBar`；按 session name 执行大小写不敏感过滤，并把 Add 暴露为稳定 action | Session 搜索与新建入口 | 搜索、新建 Session/Thread 和新 tab projection 已接通 |
| `session_context_menu` | 右键当前 Session Tab 后，用 `ContextMenu` 呈现 Pin、Close、Rename、Fork；拥有 outside click、Escape、键盘导航和焦点恢复 | Session action surface | 通用基座提供柔和阴影、2px padding 与 4px radius，打开默认选择 Pin；真实 mutation 仍未执行 |
| `zeta-editor::CodeEditor` | 多行 Unicode 编辑、selection、history、IME/syntax projection、Document/Compact presentation、soft wrap 与可见行绘制 | CodeEditor | 委托；Composer 与文件 Editor 都已接入平台事件、focus、caret blink、IME、pointer caret/drag、clipboard 和垂直 viewport；文件 Editor 启用 soft wrap 与拖选越界自动滚动 |
| `zeta-agent-sidebar::AgentSidebarNavigation` | 只负责 Changes/Files 的跨功能切换和导航语义 | SidebarPart navigation | ✅；不拥有 Files/SCM 功能布局 |
| `zeta-agent-sidebar::files::FilesLayout` / `FilesToolbar` / `FilesState` / `FilesPane` | Files 自己拥有 36px 功能 toolbar、Refresh、ahead/behind、Search、文件树与搜索结果；Native 只通过 App Server 适配 `DirectoryEntry` | Files Pane | ✅；pointer/Enter/Space 产生 crate action，由宿主加载目录或打开文件 |
| `zeta-agent-sidebar::scm::ScmLayout` / `ScmState` / `EditorPaneState` / `EditorPane` | SCM 自己拥有 Changes toolbar slot、changed-file snapshot、language-aware `DiffEditorDocument`、整体滚动位置、scrollbar pointer capture/animation、每文件 `DiffEditorState` 和 `MultiDiffEditorLayout` | Changes Pane | ✅；Native 只提供 `ScmDiff` 和主题投影 |
| `zeta-editor::DiffEditor` / `MultiDiffEditor` | DiffEditor 提供 SideBySide/Unified presentation 与未修改区间折叠投影；MultiDiffEditor 再纵向组合多个文件 section、发布每文件 fold identity 并裁剪不可见项 | 多文件差异文档 | Changes 固定宽度栏显式选择 Unified；文件读取、diff 计算、持久状态与产品输入路由不属于 editor crate |
| terminal grid / PTY / scrollback | grid、PTY 与会话内有界回滚已接通，跨重启持久化尚无 | 活动 Terminal Session runtime | 部分具备 |
| multi-session projection / switching | App Server Session/Thread 的创建、动态 tab projection 和切换已接通；每个 Session Tab 绑定独立本地 PTY，切换时保留各自 TerminalCore 与 shell 进程 | 多会话入口 | 已实现；Session/Thread/用户消息与 PaneGroup 不跨 zeterm 重启恢复，Session action mutation 仍是当前限制 |

CodeEditor/DiffEditor/MultiDiffEditor 的实现 ownership、`DiffSideRows` 投影、显示列 contract、
测试和当前限制由 [`zeta-editor` README](editor/README.md) 维护。zeterm 负责 changed-file
collection、文件 identity、整体滚动位置和每文件 `DiffEditorState`；MultiDiffEditor 只借用这些
快照完成多文件组合。Native 不能复制代码行或 diff decoration 绘制。当前 Git binding 跳过
binary、非 UTF-8 与单侧超过 2 MiB 的文件；index-only 对比仍是后续接线。

当前已用 `zui::SplitViewLayout` 与 `zeta-ui::Sash` 支持 Sessions/Main 的单轴 resize，并用
`zui::GridLayout` 作为 Terminal Workspace 的递归几何入口。Native host 为每个 Session Tab
保存 PaneTree、active Pane、独立 `TerminalSession`/scroll/selection 状态和 split command；
窗口 resize 与 Sash 调整会按每个 Pane 的 logical viewport 计算 rows/columns，并分别同步
对应 terminal grid 和 PTY。

## 当前执行路径

```text
main
  → ZetermLaunch::parse
      → Local 或 `--remote <host> --workspace <absolute-remote-path>` RemoteProfile
  → zui::app::Application::run
  → NativeApp::resumed
      → AppContext::open_window
      → WindowContext / WindowHandle capabilities
      → build_shell_presentation
      → framework renderer factory
      → request_redraw
  → NativeApp::window_event
      → resize / scale-factor update → rebuild scene
          → TerminalWorkspace::resize_all → each TerminalSession::resize
              → Local PTY 或 Remote App Server `terminal/resize`
      → AgentSessionEvent::Snapshot/Update → TabInputModel/TerminalWorkspace binding + ThreadProjection
          → unknown Session reserves a terminal key and starts background terminal creation
          → TerminalReady → adopt Local PTY 或 Remote App Server terminal runtime
          → ThreadTimeline
          → committed update / stream gap → session/subscribe refresh
          → transient Agent/Tool delta → rebuild scene
      → TerminalSessionEvent::Output(key) → bound TerminalCore::process_output → rebuild scene when active
          → terminal query → take_reply_bytes → TerminalSession::send_input
              → Local PTY 或 Remote App Server `terminal/write`
      → TerminalSessionEvent::Exited → TerminalCore::mark_process_exited → rebuild scene
      → cursor / primary mouse event → titlebar drag 或 terminal cell mapping
      → Terminal Surface pointer → cell mapping / TerminalPointer / TerminalSelection → PTY
      → Agent Surface wheel → ThreadTimelineScroll → redraw
      → keyboard → NativeKeybindings → zeta-keybinding::KeybindingResolver
          → PendingChord → 1.5s deadline；失焦、超时或 IME 清空
          → CommandRequest → CommandRegistry handler / focused input / PTY
          → NoMatch → focused control navigation/editing → Terminal Surface PTY fallback
      → about_to_wait → KeybindingsResource 轮询 `<ZETA_PROFILE_ROOT>/keybindings.json`
          → 完整验证成功 → 原子替换 Builtin + User 规则
          → 读取或解析失败 → 保留上一份完整规则并输出诊断
      → input_method → IME preedit/commit/cancel → zeta_composer::Composer
          → Agent Enter → App Server session/request::StartTurn
          → Shell Enter → App Server session/request::StartShellTurn
          → composer caret bounds → native IME candidate area
      → pointer → zui::UiDispatch
          → InteractionFrame reverse-order hit-test
          → hover / press / capture / focus → presentation rebuild
          → UiIntent → window drag 或 product action
      → Cmd/Ctrl+Shift+I → LayoutInspector
          → InspectionFrame layer-aware reverse-order hit-test
          → 原生窗口向右扩展 hierarchy panel；产品 content viewport 保持不变
          → ancestor bounds + padding overlay；pointer 不再进入产品 UI
      → current Session Tab secondary click
          → SessionContextMenuState → ContextMenu → ContextView overlay
          → pointer / Up / Down / Tab / Enter / Space
          → Pin / Close / Rename / Fork product action boundary
          → outside click / Escape → dismiss + focus restoration
      → Sessions toolbar
          → SearchBox keyboard / IME → SessionSearch → session-name filter
          → Add icon → ADD_SESSION product action boundary
      → Sessions Sash pointer
          → SessionSidebarState drag snapshot
          → SplitViewLayout resize constraints
          → shell bounds + terminal grid/PTY resize
      → SidebarPart toggle
          → SidebarPartState visibility
          → shell bounds + terminal grid/PTY resize
          → PaneHostScope::Sidebar → AgentSidebarNavigation → PaneInput::Diff / PaneInput::Files
              → FilesLayout → FilesToolbar + 根目录树 / 模糊路径匹配结果
              → ScmLayout → MultiDiffEditor → visible file sections → DiffEditor
      → zeta_ui::layout::TerminalWorkspaceLayout
          → GridLayout → active terminal + optional SidebarPart leaf bounds
          → terminal rows/columns → TerminalSession resize
      → Tab / Shift+Tab / Arrow keys → unified focus navigation
          → Enter / Space → focused action activation
      → alternate keyboard / paste → TerminalCore encoding → PTY
      → input_method → IME commit → TerminalCore encoding → PTY
      → titlebar drag hit → NativeWindow::start_window_drag
      → visible-after-occlusion → request redraw
      → Box<dyn Renderer>::render_scene
          → UiScene::batches（严格保留跨 primitive paint order）
          → selected backend (`WgpuRenderer` currently)
```

运行：

```bash
just zeterm

# Without just:
cargo run --manifest-path Cargo.toml -p zeterm
```

`shell_scene::ShellLayout` 把 titlebar 下方 body 先交给 Sessions/Main 横向
`SplitViewLayout`，再把剩余区域交给 `zeta_ui::layout::TerminalWorkspaceLayout`；
后者通过 `GridLayout` 同时投影活动 Terminal Leaf 和可选的右侧 SidebarPart leaf。当前活动
Terminal Leaf 再分成上方 output viewport 与固定底部 composer；alternate screen 临时使用完整
活动 Terminal Leaf。
`SessionSidebarState` 保存 visibility、preferred width 与当前 `SplitViewResizeSnapshot`，
viewport 临时约束只改变 effective width，不覆盖 preferred width。`Sash` 从
`SplitViewSashLayout::track_bounds` 同源计算 drag target 与 hover/active feedback；
`SidebarPartState` 保存显隐、preferred width 与当前 `SplitViewResizeSnapshot`，并向外层 Grid
提供右侧 Sash 所需的 pane sizing；宽度限制为 240–560px，始终为 main Pane 保留至少 240px；
`TerminalWorkspaceLayout` 解析右栏 Leaf bounds，SidebarPart 内的 `PaneGroup` 再把 content
leaf 投影给具体 view；`files::FilesLayout` 和 `scm::ScmLayout` 分别把各自的功能 bounds
解析为 36px toolbar slot 与 active content pane，跨功能选择由 host 的 PaneInput binding 驱动。
`files::FilesState` 保存文件搜索、树与滚动，`scm::ScmState` 保存 changed-file snapshot 和
`EditorPaneState`。Native 的
`SidebarPaneWorkspace` 只负责将 App Server/`WorkspaceContext` 快照适配为 crate 类型，并执行
Refresh、文件打开和子目录加载动作。Refresh、Composer Changes action 和 shell command completion
通过 App Server `git/textDiff` 重建上游领先/落后距离、HEAD/working-tree `DiffDocument` 与增删行统计；
Native 按 path 选择 `CodeEditorLanguage` 后立即包装为 `DiffEditorDocument`，此后 parser/revision/token
都留在 editor crate。
Files pane 的层级模式由 `files::FilesTree` 保存 arena、parent/children、稳定 mounted node ID
和展开状态；根目录和首次展开的子目录都由 `AgentSession` 通过 App Server
`fs/readDirectory` 读取 workspace-relative 直接子项，Tree model 不直接访问文件系统；收起/再次展开
复用已加载 children。App Server `fs/changed` 通知会重新读取根目录，当前仍不恢复刷新前的展开状态。
`zeta-ui::TreeView` 在 24px 固定行高 ListView 上投影 depth、disclosure 和 content geometry，
只为 visible range 注册 `Tree`/`TreeItem` accessibility node，paint 使用两行 overscan。
Search 结果仍是扁平 List/ListItem。滚轮更新 workspace-owned `ScrollState`，查询、Refresh 或
workspace 替换会回到顶部。文件打开已接入中心 Editor Surface；重命名、拖放与刷新后的展开状态
恢复尚未接入。
`EditorPaneState` 保存整体 `zeta-ui::ScrollState` 和每文件 `DiffEditorState`；
`MultiDiffEditor` 在一个纵向文档中连续绘制所有可见文件 section，每段再复用
`DiffEditorPresentation::Unified` 将删除/新增行投影为单列内容。section 高度由
`zeta-ui::VirtualListLayout` 建立 prefix index，paint 与 fold-control 查询通过二分只访问
viewport 相交的文件；展开/收起改变高度时由 `EditorPaneState::remeasure` 重建该 snapshot。
长未修改连续区间默认只保留变更前后各三行上下文；`DiffEditor` 发布 Show/Hide fold control，
`EditorPane` 将其注册为支持 pointer、Tab focus 和 Activate 的 Button，并把激活结果写回对应文件
的 `DiffEditorState`。普通 CodeEditor 不决定 diff 折叠规则。
指针位于 MultiDiffEditor 时，wheel 会更新其有界整体纵向 offset，不落入 Terminal scrollback。
高频 PixelDelta 和 scrollbar drag 只累计 retained offset，并把 presentation 重建合并到下一次
`RedrawRequested`；同一事件循环批次不会为每个 delta 同步重建整个 Shell scene。
通用的 `Render < Fragment < Rebuild` 失效等级、Scene checkpoint 和 fragment 原地替换契约由
`zui` 拥有；Native 只把产品状态变化映射到失效等级，并定义 Shell 的 base/overlay retained boundary。
`shell_scene` 只注册产品 `ElementId`、cursor 和 separator semantics。`NativeApp` 每次重建
presentation 时从 `NativeWindow::window_control_insets` 读取 host chrome 占位，并通过
`ShellPresentationModel` 交给 `titlebar::Titlebar`；Titlebar 在占位外再增加自己的 `8px`
内容间距。窗口控件宽度和所在边不能进入 `ActionBar` 或 `zeta-ui`。
`shell_interaction` 只声明产品稳定 `ElementId` 并把 context action 映射回产品命令；实际
sidebar state 由 `session_sidebar` 定义并由 `NativeApp` 保存，它不保存 hover 或 focus。
`command_dispatch::command_request_for_element` 和 `NativeKeybindings` 分别把 pointer/menu entry
point 与标准化键盘事件映射到 `CommandRequest`；`NativeApp` 启动时通过
`command_dispatch::builtin_command_registry` 注册完整产品命令目录，`NativeApp::dispatch_command`
从注册表取出 handler，是唯一产品执行入口。`KeybindingsResource` 每秒检查 `<ZETA_PROFILE_ROOT>/keybindings.json`，只在完整资源通过
大小、字段、按键、条件和命令校验后替换 User 规则；内容无效时保留上一份有效规则并把诊断
显示在快捷键设置页。`zeterm-keybinding-ui::KeyboardShortcutsState` 录制最多四段按键，暂停一秒后
把 commit 交回 Native 资源层原子写入；设置浮层、深灰 keycap 与 Chord 提示也由同一快捷键
UI crate 拥有。Native 的 `keyboard_shortcuts` 只分配产品 `ElementId`、投影
`ZetermCommandId` 行并连接保存结果。
`zeta-keybinding` 解析平台无关按键和 `when` 表达式，不读取 focus、不执行命令，也不拥有
Chord timer。
`session_context_menu` 用 `SessionContextMenuState` 保存当前目标、锚点和待恢复焦点，用
`zeta-ui::ContextMenu` 组合 ContextView 定位、renderer BoxShadow、2px menu padding、4px
radius 和纵向 ActionBar item geometry。打开时默认选择第一个 enabled item `Pin`；菜单子树会
成为当前 interaction frame 的 modal scope，hover 同步 roving focus，移出后保留最后一项，
下层控件不再接收 pointer、focus 或 activation。右键当前真实 Session Tab 打开菜单；菜单外
左键、Escape 或窗口失焦关闭菜单，方向键、Tab、Enter 和 Space 复用统一 focus/activation 路径。
`SessionContextMenuAction` 已将 Pin、
Close、Rename、Fork 映射为产品 command identity；App Server Session/Thread 已支持创建和切换，
每个 Session Tab 也已绑定独立 PTY，但这些 command 尚不执行 pinning、关闭、重命名或 fork，
不得由 presentation state 伪造结果。
`zui` 是跨 native 组件的通用交互运行时：
`InteractionFrame` 按 scene 构建顺序注册有父子关系的 `UiNode`，反向命中最上层节点，并投影
accessibility role、label、bounds 与 focused state；`UiDispatch` 跨 frame 保存
hover path、press/capture 和 focused identity，最后只返回 `UiIntent`。

标题栏、侧栏开关、Workbench `WorkbenchTabList`、Session 右键菜单、通用 `zeta-ui::TabList`、
ThreadTimeline、Terminal Surface、`ComposerPanel`、compact `CodeEditor`、`ActionBar` 和五个 Composer
上下文 `Button` 都走这条路径。
ComposerPanel 使用默认指针，输入与终端文本使用文本指针，Button 使用 pointer 指针；绘制、
输入上下文 toolbar 的命中与语义 bounds 共享 `ActionBar::interactive_item_bounds`，Session
菜单则共享 `ContextMenu::interactive_item_bounds`。Agent Surface 默认聚焦 composer，Editor Surface
打开文件时聚焦 active document；
Tab/Shift+Tab 遍历全部 tab stop，toolbar 内左右键移动相邻 Button，Enter/Space 激活焦点
action，Escape 从 action 返回 composer。pointer press 同时更新 focus，release 按 capture identity
决定是否激活。`input_method::InputMethodTarget` 合并窗口活动状态、Workspace Surface 与 focus；
焦点离开当前文本输入或窗口失焦时取消对应 composition 并停止 caret blink，Terminal Surface
仍保留直接输入所需的 IME。过小 viewport 使用有边界的 compact fallback。

`ShellPresentation::accessibility_nodes` 保存当前 frame 的语义快照，`WindowContext::present_scene`
会在绘制前通过 ZUI 私有 AccessKit adapter 发布现有树、bounds、focus、selection 和 expansion。
VoiceOver、Narrator 或 Orca 的 Focus/Click 请求通过 `App::accessibility_action` 回到同一个
`UiDispatch` 与产品 reducer，不建立第二套控件身份；各平台的最终读屏质量仍需原生 smoke test。
`ShellPresentation` 由单一 `UiFrame<InteractionFrame>` 保存 scene 与 interaction，再保存
accessibility projection；只有 `UiScene` 被传给 `Renderer`，因此 GPU backend 不承担 hit-test、focus、
command dispatch 或 accessibility ownership。新增组件组合必须使用 `UiFrame::draw_component`，不能
重新引入 `ShellPresentation` 的平行输出字段。

`layout_inspector::LayoutInspector` 是独立的检查工具 presentation state。`Cmd/Ctrl+Shift+I` 开启后，
Native 先保存当前产品 content width，再通过 `NativeWindow::request_inner_logical_size` 向右扩展
`360px`；Shell、Terminal grid 与产品 hit testing 继续使用保存的 content viewport，因此检查面板不会
挤压或重排被观察布局。`zeta_ui::layout::RootLayout` 把保存的 Product viewport 和新增宽度交给
`GridLayout` 解析为两个真实 sibling leaf；`InspectorPanel` 在 Inspector leaf 的 layer 0 内组合
`InspectorToolbar` 与 `InspectorContent`，不再作为 scene overlay。Toolbar 与产品 Titlebar 同高，左侧
cursor action 显式开关选取，右侧 close action 关闭面板并恢复原窗口宽度；Content 只拥有层级行、
指标与节点切换。面板打开时默认不进入选取状态，产品 content 继续接收正常 pointer、keyboard 和
cursor feedback。选取期间 Native 才截获产品输入，按当前
`UiScene::inspection` 反向选择最深节点；左键释放后锁定当前链并自动退出选取，保留检查结果。Escape
先停止选取，再次按下才关闭面板并请求恢复原窗口宽度。Inspector 用橙色显示 padding、青绿色显示组件
上报的实际 gap 区域、蓝色显示当前
bounds、紫色显示 ancestor bounds，并在右侧面板按真实 parent chain 显示每层的组件名、size、
padding、gap、radius、scene layer 与源码位置。只有这些产品节点的 outline/padding highlight 使用 overlay。
层级行可以直接重定向当前目标；面板保留完整 path，左侧只
强调所选节点及其祖先，因此可在 parent 与原 descendant 之间反复切换。它不复用 `InteractionFrame`
推断样式，因为交互树不覆盖纯视觉组件；也不修改组件 style 或产品 reducer。
每个组件通过 `Component::element` 声明布局，由 `UiScene::draw_component` 解析一次
`ComputedElement` 并自动建立 parent chain。`SessionSidebar`、`ComposerPanel` 这类非组件 composition
函数以及 `ContextView::draw` 这类 content-closure surface 使用 `UiScene::with_element` 进入相同管线；
产品代码不再手写检查元数据。
Native 当前所有拥有稳定 bounds 的 Component 均已接入，包括 Titlebar、两个 Sidebar Toolbar、
ComposerPanel/ComposerInfoBar、ComposerInput、ThreadTimeline、Explorer/Changes Pane、Session Tab、
快捷键与两个选择浮层。选取期间的十字 cursor 只覆盖产品 content viewport；右侧 Inspector panel
始终使用普通 cursor，只消费自身 action 与 panel pointer，不参与组件选中或锁定。
`component_composition_tests::product_composition_uses_scene_draw_component` 扫描非测试产品源码，阻止
新增代码绕过 canonical composition 入口而静默丢失 inspection ancestor。

`zeta-composer::Composer` 是 Composer 的单一状态 owner；内部 `ComposerInput` 保存
`CodeEditorDocument` 与 retained viewport，routing 状态保存分类器选择的 Agent/Shell 路由和 Shell
history，interaction 状态保存 active View、selection 与 viewport offset。`composer_panel` 只投影这些
状态；`zeta-ui::ScrollView` / `ListView` 统一处理 content geometry、裁剪、滚动条与可见范围。Slash
catalog、输入 grammar、过滤、选择和 dismiss 委托给 `zeta-slash-commands`。当前输入 `/` 时由共享状态
提供 active View；选择 `/model` 后压入模型列表。Escape 从子 View 返回上一层，在根 View 时收起 Pane。
非 Slash 输入由 `Composer` 从
[`zeta-input-classifier`](../zeta-rs/input-classifier/README.md) 持有的同一 Shell context 请求候选；
command、subcommand、option、工作区 target 与 path 候选不会打开 Pane。`Composer` 在光标位于
输入末尾时把唯一候选的剩余文本，或多个候选仍可确定的最长公共前缀，投影为 `CodeEditor` 光标后的
灰色 ghost text；没有可确定的下一段时不显示。Tab 通过 `CodeEditorTextEdit` 接受该段并重新运行自动
路由，Escape 在输入再次变化前隐藏建议。Shell parser、command registry、PATH/manifest 扫描与 alias 展开仍由
[`zeta-shell-completion`](../zeta-rs/shell-completion/README.md) 拥有，Native 不复制判断规则；相关
workspace 文件变化时只触发该 context 的候选快照刷新。
分类器只决定整段提交的 route；只有整段作为 Shell command 提交时，`ComposerInput` 才启用 Shell
syntax。以命令开头但最终分类为 Agent message 的混合输入保持 PlainText，不局部高亮命令前缀。
`zeta_composer::ComposerPanelLayout` 组合“交互区 → 信息栏 → input → toolbar”；信息栏固定显示
当前路由的提示，toolbar 固定在底部。交互区出现时只增加底部 Panel 高度并向上压缩
ThreadTimeline，信息栏、editor 与 toolbar 的位置保持不变。模型列表来自 typed
`model/list`，选择结果通过 `session/request::SetModel` 写入当前 Session；UI 不复制模型目录或伪造
Session model。
`terminal_input` 把普通 key 和 paste 路由到 Composer；临时 View 可见时优先消费方向键、Enter、
Tab 和 Escape；否则 Enter 分别提交 `session/request::StartTurn` 或
`session/request::StartShellTurn`，Shift+Enter 插入换行。Composer 从紧凑基线自动增长到八行，之后由
viewport 跟随 caret。Terminal Surface 的输入经过
`TerminalCore::encode_key/encode_paste` 直接写入 PTY。`input_method` 单独把 IME 路由到当前
`InputMethodTarget`：Agent Surface candidate area 跟随 composer caret，Editor Surface 跟随 active
CodeEditor caret，Terminal Surface 跟随 grid cursor。`file_editor_input` 只把标准化 keyboard、IME、
pointer、clipboard 与 wheel 操作转交给 active `CodeEditorDocument`/`CodeEditorViewport`。
`FileEditorPane` 启用 editor-owned soft wrap，并把 `CodeEditor::navigation()`、visual row count 与
caret visual row 原样交回 host；`file_editor_auto_scroll` 只在 pointer 拖选进入上下 8px 边缘区或
越过边界时以 35ms deadline 推进一个 visual row，pointer 回到内部、释放、失焦或到达文档边界后立即停止。
关闭按钮先由 `FileEditorHost::request_close_active` 判断 dirty 状态；未修改 Tab 直接关闭，dirty
Tab 由 `FileEditorInputState` 打开 modal 决策条并提供 Save、Don't Save 与 Cancel。磁盘外部变化由
`TextFileLifecycle::status` 投影为 reload/conflict 提示条：Reload from Disk 显式替换 editor text，
Overwrite 使用待处理 snapshot 的磁盘版本再次执行 optimistic preflight，若文件再次变化仍拒绝写入。
`workspace_context::WorkspaceContext` 在 Session 启动时捕获真实 cwd，并消费
`zeta-app-server-client` 返回的 workspace-scoped `GitTextDiffResult`；
`InputContextToolbar` 消费四项 context value，使用 `ActionBar` 统一排列并把每项语义交给
`ActionBarButton::icon_and_label` / `Button`，以及 Files pane 的文件行。`IconLabel` 在 Button
和 Files pane 内部完成 icon/text placement，不作为 Toolbar 的直接 action representation。
ToolResult durable commit 后刷新
branch 与 `Changes files • +additions -deletions`；四项 Button 已接入 hover、press、focus、
键盘导航和 pointer feedback。cwd Button 使用 product-owned `WorkspacePathPickerState` 组合
modal `Dropdown`；第一行通过 header slot 承载默认聚焦的 `SearchBox`，输入按当前目录的直接
子目录名称实时过滤；显式输入 `../path`、`~/path` 或绝对路径时，可直接选择已存在目录。Git
projection 的 repository-relative `workspacePath` 用于还原并固定展示 repository root 快捷项，
不在协议中传递 host 绝对路径。进入子目录后清空查询，同时支持父目录、排序子目录、固定搜索
header 下的结果滚动、鼠标滚轮、键盘焦点自动滚入视野、clipboard、IME、Escape/外部点击关闭和
roving keyboard focus；选择后同步替换 Files 根、文件
搜索索引和 Git/Changes projection。branch Button 通过
`GitBranchContextMenuState` 组合通用 `ContextMenu`，第一行通过其 header slot 承载默认聚焦的
`SearchBox`；输入按 branch name 实时过滤，方向键/Tab 从搜索框进入结果，Enter 选择首个匹配项，
并支持 clipboard 与 IME。当前分支置顶并标记，其他本地分支分页展示；候选和选择分别通过
`git/branch/list` 与 `git/branch/switch`。Git 因工作树冲突拒绝时，菜单保持打开并显示失败
状态，不丢弃用户改动。Changes Button 会刷新 Git projection、展开 SidebarPart 并选择 Changes
Pane；environment picker 尚未接入。
在 shell integration 提供 cwd 事件前，目录标签表示用户选择的工作区，而不推断 PTY 内部 `cd`。
`workspace_path_picker_path` 中的 `canonical_directory`、`resolve_directory_query` 和
`read_child_directories` 在替换状态前验证目标、解析显式路径并读取子目录；无权限、
已删除或非目录目标会保留原工作区与当前浮层状态并记录错误。`workspace_path_picker_input` 单独拥有
模态 pointer/keyboard 路由、focus restoration 和 `WorkspacePathPickerActivation` 的产品状态转换，
通用 `Dropdown` 不读取文件系统、搜索查询，也不拥有工作区切换。
`WorkbenchTabList` 中 Session Tab 的白色状态圆形保留独立可访问性 label，后续可在其中绘制状态 SVG；当前
native runtime 只投影通用 `Active`，尚不能声称已接入 Planning、Thinking、Editing 等 Agent
执行阶段。这些状态必须在 App Server 提供权威 Session projection 后映射，不能由 UI 根据输出
推断。
PTY output 中的 device/status/cursor query 由 `TerminalCore` 生成 reply bytes；
`TerminalSession::handle_event` 在同一次 output event 后取出并写回同一 PTY，renderer 不参与。
`terminal_pointer::TerminalPointer` 只在 alternate screen 且应用启用 tracking mode 时接管 terminal
viewport，维护 held button 与最后一个有效 cell。titlebar 和 terminal padding 外部仍走
产品 hit testing；1000/1002/1003 filtering 与 legacy/1006 wire encoding 委托 `TerminalCore`。
`terminal_scrollback::TerminalScroll` 只保存当前视口距底部的行偏移、触控板小数滚动量和
scrollbar fade controller；主屏滚轮在 BlockList 已建立后浏览命令 transcript，否则浏览
`TerminalGrid` 的 cell history。`TerminalOutputScrollView` 把这个底部相对行窗口转换为通用
`zeta-ui::ScrollView` 的顶部相对内容坐标，由后者统一执行 viewport clip、内容平移和 scrollbar
geometry/paint。用户停在旧输出时，新输出增加相同偏移以保持内容锚定；这条 terminal 语义没有
下沉到通用滚动组件。提交新命令会回到底部。alternate screen 请求 mouse report 时，滚轮优先
写入 PTY，不改变产品回滚位置。
`terminal_selection::TerminalSelection` 同样只拥有可丢弃的 viewport selection。主屏左键拖拽
跨过至少一个 cell 后才生成选区，单击不会留下蓝色矩形；宽字符复制按 display width 截取。
macOS 使用 `Cmd+C/V`，其他平台保留未加 Shift 的 `Ctrl+C/V` 终端语义，并使用
`Ctrl+Shift+C/V` 访问剪贴板。OSC 0/2 title 同时投影到 native window 和 Session Tab，不改变
内部品牌名。

`terminal_session::shell_bootstrap` 对支持的 POSIX shell 关闭 PTY echo 并隐藏原生 prompt，
`BootstrapOutputFilter` 在 bootstrap marker 前不向产品暴露启动噪声。zsh 额外安装最小
`precmd` hook，以 OSC 133 `D` 完成活动 Block。这个 hook 还不是可协商、可版本化的完整 shell
integration；其他支持 shell 目前不能可靠报告每条命令的完成状态、cwd 或 exit status。

当前仍没有 Session action mutation、多行/历史/建议式 Block Editor、
双击词/三击行选择、terminal selection auto-scroll、跨进程重启的回滚/Block 持久化、完整
DEC/query/mouse family。内部语义树、统一 focus 与 AccessKit publication 已具备，但读屏器平台
smoke coverage 仍需补充。alternate screen 已具备基础 direct
key/IME commit/clipboard 和请求式 mouse input，但
尚不能据此声明完整 TUI compatibility。这些后续纵切不应进入 `zui` 的 window/input 或 GPU modules。
布局检查模式当前覆盖所有 Component 和拥有独立 box ownership 的 composition surface；尚不是完整
retained widget tree，也不支持运行时编辑样式、跨 frame 选择 identity、面板滚动或远程协议。
Sessions 搜索区域当前真实链路
是 `SessionSidebar → SessionSidebarToolbar → SearchBox → InputBox`；不存在单独的 Header 组件。

`zui` 的 crate-level 实现契约见
[`zui/README.md`](zui/README.md)。后续 file tree、tabs、chat 和 editor
首次接入时，应由各自 presentation owner 分配稳定
`ElementId`、注册父子节点、role/label/bounds、focus policy 与 activation intent。动态行或 tab
必须在仍表示同一对象时保持 identity；domain selection、document model、chat turn 或 filesystem
state 不进入 `zui`。

macOS 可能在新窗口激活完成前把首次 surface acquisition 报为 occluded；该 frame 会被跳过。
`NativeApp` 在后续 `WindowEvent::Occluded(false)` 上重新请求 redraw，保证首个可见 frame 不会
因为一次正常的 activation transition 永久丢失。

## Remote 启动边界

当前 Native host 支持显式命令行 Remote launch：

```text
zeterm --remote <openssh-host> --workspace <absolute-remote-path> \
  [--runtime <remote-zeta-executable>] [--ssh <local-ssh-executable>] \
  [--runtime-catalog <local-catalog> --runtime-catalog-sha256 <digest>] \
  [--runtime-catalog-url <https-catalog.json> --runtime-catalog-sha256 <digest>] \
  [--runtime-cache <absolute-local-path>] \
  [--rollback-runtime]
```

也可以把常用服务器保存为命名连接：

```text
zeterm remote save <name> --host <openssh-host> --workspace <absolute-remote-path> [--replace]
zeterm remote list
zeterm remote connect <name> [--runtime <remote-zeta-executable>] [--ssh <local-ssh-executable>]
zeterm remote tunnel <name> --remote-port <port> [--local-port <port>] [--ssh <local-ssh-executable>]
zeterm remote remove <name>
```

命名连接存放在 `<local-profile-root>/remote/targets.json`，只包含规范化名称、OpenSSH host alias 和
Remote Workspace。它不保存密码、私钥、SSH executable 或 runtime；`connect` 解析出 target 后仍走
下述同一套探测、安装、握手和连接流程。默认 create 不会覆盖同名连接，只有显式 `--replace` 才会
替换。`remote list` 输出稳定的 tab-separated `name / host / workspace`，便于 shell 和 Native
连接选择器和管理面板消费。

运行中的 zeterm 点击底部 `Local/Remote` location 按钮，或调用可绑定命令
`workbench.action.pickExecutionLocation`，会从同一目录打开可搜索的 Native modal picker。选择连接后
Native host 用当前 zeterm executable 启动 `remote connect <name>` 新进程；它不经过 shell、不向 UI
传 host/Workspace/凭据，也不改变当前窗口的 Workspace authority。父窗口只监督启动：子进程仍独立
拥有 runtime 探测、安装、兼容性握手和 OpenSSH；父窗口只消费带前缀的有界 JSON Lines 进度，子进程
普通诊断继续继承 stderr。

picker 的 `Manage Remote connections…` 会打开 Native 管理面板。面板可新增、选择、编辑、改名、
两步确认删除及连接；改名通过 `RemoteConnectionCatalog::update` 在同一 advisory lease 内完成，不能用
“先创建、再删除”产生中间目录状态，也不能覆盖另一个规范名称。未保存草稿不会因切换连接或点击
New 被静默丢弃；Connect 只接受已保存且无未提交修改的记录。所有字段都有键盘、鼠标、滚轮、
accessibility 和 IME/clipboard 路由，错误直接留在面板内。管理面板仍只写无凭据 target 字段，SSH
认证继续完全属于本机 OpenSSH。Connect 后面板会依次显示 runtime 检查、目录/artifact 下载、
下载校验、平台探测、按 10% 分别节流的下载/上传、远端提交、下载/缓存/安装/复用和失败状态。收到 `Ready` 后父窗口解除监督并关闭面板，新
Remote 窗口独立运行；在此之前关闭面板会取消子进程。失败会解除编辑锁并保留错误，可直接再次
Connect。

`remote tunnel` 是 zeterm 对共享 host-side Tunnel primitive 的前台 CLI 消费者。它只从命名连接读取
OpenSSH host，固定监听 `127.0.0.1` 并只转发到远端 `127.0.0.1:<remote-port>`；不接受公开 bind、
反向转发、密码、私钥或任意 SSH options。省略 `--local-port` 时 Native host 选择一个当前可用的
loopback 端口，OpenSSH 再通过 `ExitOnForwardFailure=yes` 做最终 bind gate。CLI 会在 12 秒内轮询
实际 loopback listener，只有 listener 稳定可连接且 child 仍存活才输出实际
endpoint，并持续前台监督；Ctrl-C/TERM 会关闭 OpenSSH。这个基础 `ssh -L` 生命周期不需要
Remote Server 参与，未来非 SSH transport 或远端动态 endpoint 才应扩展 Remote Server 协议。

当当前窗口本身是 Remote authority 时，同一 location picker 还会显示
`Manage Remote tunnels…`；也可将 `workbench.action.manageRemoteTunnels` 绑定到自定义快捷键。
zeterm 的 `remote_tunnel_process::NativeRemoteTunnelHost` adapter 从 `AgentSessionTarget` 复用该窗口
已选择的 OpenSSH host 与 executable，并调用 `zeta-remote-host::RemoteTunnelHost` 在后台线程持有和监督每个
`ssh -N` child；Renderer-facing
`RemoteTunnelManagerState` 只保存远端端口、分配后的本机 loopback 端口和
Starting/Forwarding/Recovering/Stopping 状态。面板只接受远端端口，不能提交 host、凭据、监听地址或 SSH options。
关闭面板不停止 Tunnel；再次打开可查看或逐项 Stop，关闭 zeterm 窗口则由 Native host owner 收掉全部
child。Local 窗口没有可复用的 Remote authority，因此不展示 picker 动作，命令入口也会拒绝打开。
首次启动早退、listener 未在 12 秒内出现都会失败；恢复尝试也经过同一 readiness gate。已经
Forwarding 后的 child 退出会在 30 秒内按 250ms 到 2s 退避恢复，并复用
原本的本机端口。恢复状态、恢复成功、恢复耗尽和显式 Stop 都通过 `NativeEvent::RemoteTunnel` 回到产品
事件循环；Stop 与窗口关闭会立即唤醒退避，不阻塞 winit thread。

默认启动先按 OpenSSH host + Remote Workspace 从本机
`<local-profile-root>/remote/connections.json` 读取上次握手成功的精确 runtime；没有记录时才从远端
`PATH` 探测产品无关的 `zeta-server`。`local-profile-root` 与 App Server client 共用，`ZETA_PROFILE_ROOT` 可显式覆盖。
availability probe 得到的 resolved executable 会用于短生命周期 App Server initialize/schema
compatibility preflight，只有两项都成功才通过原子写入激活并保留一代 previous runtime。profile 只含
host、Workspace 和 runtime，不保存凭据或 SSH executable。

若 runtime 缺失或 schema 明确不兼容，正式 zeterm
发布包从签名 binary 绑定的本地 catalog 或网络 URL + 摘要选择远端平台对应的完整 packaged-node
runtime；网络包先通过共享 updater 写入本机内容寻址缓存，再经 SSH 上传安装，切换到返回的不可变摘要路径，并重新执行 availability 与 compatibility 两次检查，然后才
启动窗口。因此本机只安装 zeterm、远端没有预装 `zeta-server`，或只安装了旧版 `zeta-server`，也能连接。source build
没有绑定 catalog 时会安全退出；开发和运维可以显式同时传
`--runtime-catalog` 与其已认证 SHA-256，或传 `--runtime-catalog-url`、摘要以及可选本机 cache root。
显式 `--runtime` 永远不会被自动替换，也不能和 catalog
参数混用，并且不会读写持久 profile。`--rollback-runtime` 不下载 artifact：它先验证 previous runtime
仍可执行且协议兼容，再以 compare-and-swap 方式交换 active/previous；验证失败或并发修改都保持当前
active 不变。它不能与 `--runtime` 或 catalog 参数混用。SSH transport failure 或 server rejection
也不会被误判为升级信号。SSH host、agent、
跳板、主机密钥和私钥仍由本机 OpenSSH 管理，profile 不保存凭据。

Remote 模式下，文件树、文件读写、Git、Agent Session 和交互式 terminal 都由 SSH 到达的
Remote Server daemon 执行；Native host 只拥有 OpenSSH 子进程和终端 grid 的本地投影。每条 SSH
stdio 连接都经过 `remote-server connect` 接入按 Workspace 和 runtime 隔离的 App Server。当前本机
语言服务进程不会在 Remote 路径启动；诊断、Hover、Completion 和位置跳转通过独立 language
connection 调用远端 App Server，避免慢 LSP response 阻塞 Agent 与文件请求；位置在 UTF-8 editor
byte 与协议 UTF-16 之间按 exact document revision 转换。
Agent transport 断线后会在 30 秒窗口内以 250ms 到 2s 的退避重建 SSH/App Server connection，并
重新投影 durable Session/Thread snapshot、重新同步打开的语言文档。正常连接
曾恢复后发生的新断线会开启新的 recovery window；等待期间提交的命令会立即失败，不会在稍后连接
恢复时被隐式回放。Profile Session catalog 同时展示其他 Workspace 的 durable Session；选择后由
Native Agent host 保留当前 SSH host、runtime 和 OpenSSH executable，只替换 Remote Workspace
root、预检新 connection，再恢复精确 Session。Local Session 使用同一条路由语义重连对应的
profile/Workspace authority，不能在旧 Workspace connection 上执行。Remote terminal 以 `reconnectable`
lifecycle 创建；transport 断开后，worker 在服务端给出的 30 秒 lease 内重新 SSH、attach 原 PTY，
并使用服务端旋转后的 token 作为下一次恢复凭据。zeterm 已能从 release-bound 本地或网络
catalog 自行选择和安装缺失 runtime；启动窗口前会在 stderr 投影下载、校验、平台探测、按 10% 节流的下载/上传、
远端提交和安装/复用结果。从现有 Native 窗口发起连接时，同一 typed progress 会进入管理面板，
并提供启动前取消和失败重试；直接从 shell 首次启动仍保留纯 CLI stderr 行为。正式 publisher/feed、
企业代理、断点续传、其余尚无 Native UI 消费者的语言操作、旧 immutable runtime/cache GC，以及 Debug/Browser Tunnel
自动消费者仍是后续能力；Native Tunnel manager 与前台 Tunnel CLI 均已可用。Desktop 与
zeterm 的整体 Remote 状态见 [`docs/remote-development.md`](../docs/remote-development.md)。

当前 Agent session、Remote language session 与每个 Remote terminal runtime 各自拥有一条 Native
host → OpenSSH → App Server 通道；这保证 SSH transport 不进入 UI，也避免慢 LSP 请求占住 Agent
request driver。Agent、Language 和 Terminal 都能用替换连接恢复，Terminal
恢复窗口只覆盖短暂 transport 中断；远端主机/daemon 重启、长期离线、跨设备漫游和连接池仍未完成。
