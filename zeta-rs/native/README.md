# `zeta-native`

> Native 文本输入的跨 crate canonical ownership 见
> [`docs/native-text-input.md`](../../docs/native-text-input.md)；Native 窗口“终端即应用”的
> 产品结构与分阶段演进见
> [`docs/native-terminal-ui.md`](../../docs/native-terminal-ui.md)；本 README 只拥有 native
> host 当前源码路径和接入义务。Terminal grid 与 BlockList 的实现契约见
> [`zeta-terminal` README](../terminal/README.md)。

`zeta-native` 是与 Electron Desktop 和 TUI 同级的原生产品入口，在发布边界导出为 `zeterm`。
当前窗口纵切由产品拥有 `ApplicationHandler`，组合 `zeta-winit`、`zeta-wgpu` 与 `zeta-ui`，
并在单个原生窗口中绘制上方 Block 输出与固定底部命令编辑器。
当前 presentation 使用浅色扁平 palette：`ShellLayout` 拥有区域背景、边界和外部几何，
`Titlebar` 与 `InputBox` 分别拥有自己的内部文字和输入状态视觉。

## 所有权

| 能力 | 当前 owner | 状态 |
| --- | --- | --- |
| `zeterm` 发布名与用户可见显示 | `PRODUCT_DISPLAY_NAME` / Cargo `[[bin]]` | ✅ |
| 产品窗口标题、初始尺寸与 event routing | `zeta-native::NativeApp` | ✅ |
| Event loop 与 native window adapter | `zeta-winit` | 委托 |
| GPU surface、resize、present 与 retry | `zeta-wgpu` | 委托 |
| Rect、symbolic SVG icon、字体、shaping 与 GPU 绘制 | `zeta-ui` | 委托 |
| Shell layout、interaction frame 与 scene composition | `shell_scene` | ✅ |
| Titlebar 背景、窗口拖拽区与侧栏开关 | `titlebar::Titlebar` | ✅；不绘制可见窗口标题 |
| Top Bar sidebar toggle `ActionBar` | `titlebar::Titlebar` | ✅ |
| 原生窗口控件占位 | `zeta-winit::WindowControlInsets` | 委托；Titlebar 只增加自身内容间距 |
| 可折叠 Session Sidebar 与当前真实 Session Tab | `session_tab_list::SessionTabList` | ✅；多会话尚无 |
| 浅色扁平 Shell presentation tokens | `shell_style::ShellPalette` | ✅ |
| 稳定控件身份与 shell action 映射 | `shell_interaction` | ✅ |
| 命中、hover/press/capture、focus、键盘导航、cursor 与 accessibility semantics | `zeta-ui-dispatch` | 委托 |
| accessibility semantics → 平台屏幕阅读器 | 尚无 native adapter | 尚未完成 |
| Transparent native chrome 与窗口拖动 adapter | `zeta-winit` | 委托 |
| ANSI parser、terminal grid 与 BlockList | `zeta-terminal::TerminalCore` | 委托 |
| 默认 shell PTY、output/exit event、write 与 resize | `terminal_session::TerminalSession` | ✅ |
| primary BlockList + fixed composer 与 alternate full-grid presentation | `shell_scene` / `terminal_composer` / `terminal_input` | ✅ |
| IME target、事件转换、启停、composition lifecycle 与候选框同步 | `input_method` | ✅ |
| Bottom Widget 底部的 Local/cwd/branch/diff context toolbar | `input_context_toolbar::InputContextToolbar` / `workspace_context::WorkspaceContext` | ✅ |
| shell bootstrap、host-owned command submit 与 zsh completion marker | `terminal_session::TerminalSession` | 部分具备 |
| terminal query reply → PTY write | `TerminalCore::take_reply_bytes` / `TerminalSession::handle_event` | ✅ |
| alternate-screen mouse cell mapping、button state 与 PTY report | `terminal_pointer::TerminalPointer` / `TerminalCore::encode_mouse` | ✅ |
| 主屏滚轮浏览、输出增长锚定与 Block/grid 视口投影 | `terminal_scrollback::TerminalScroll` / `shell_scene` | ✅ |
| 主屏拖拽选择、selection paint 与 cell-aware text extraction | `terminal_selection::TerminalSelection` | ✅ |
| system clipboard copy/paste 与 bracketed-paste routing | `terminal_selection` / `terminal_input` | ✅ |
| OSC title → native window title / Session Tab | `TerminalCore::title` / `NativeWindow::set_title` / `SessionTabList` | ✅ |
| 完整 TUI compatibility | 尚无完整 owner | 尚未完成 |
| App Server session 与 durable product state projection | 尚无 owner | 尚未完成 |

依赖方向：

```text
zeta-native → zeta-winit
            → zeta-wgpu → zeta-winit
                        → zeta-ui
            → zeta-ui
            → zeta-ui-dispatch → zeta-ui
            → zeta-terminal
            → zeta-utils-pty
```

`zeta-native` 可以拥有可丢弃的 presentation state 和产品交互，但不能复制 Session、Thread、
Turn 或 Tool 的权威状态机。后续接入只能通过 `zeta-app-server-client` 的 typed contract。

## 产品方向与当前边界

Native App 的目标不是通用 Workbench Part 容器，而是一块以活动终端会话为主体的完整界面。
当前源码名称仍是验证阶段的 shell vocabulary；下面的目标名称不表示对应 runtime 已经实现：

| 当前源码 | 当前能力 | 目标产品语义 | 状态 |
| --- | --- | --- | --- |
| `titlebar::Titlebar` | 窗口拖拽区和 sidebar toggle `ActionBar`；不绘制标题文案 | Top Bar | ✅ |
| `ShellLayout` | 计算 titlebar、output viewport 与固定底部 composer | Top Bar 与 Terminal Workspace 外部布局 | ✅ |
| `shell_scene` | primary 绘制 BlockList/composer，alternate 绘制活动 grid | Terminal Session 的 Output/BlockList | 基础纵切已完成 |
| `terminal_composer` / `terminal_input` | primary 编辑 `TextInput` 并提交整条命令；alternate direct input | Block Input Editor 与 TUI compatibility | ✅ |
| `input_context_toolbar` | `ActionBar` 排列四个 icon-and-label `Button`，投影 Local、cwd、branch 与 diff count | Input/Chat context toolbar | ✅ |
| `session_tab_list` | 展开时投影当前真实 PTY Session，语义为纵向 TabList/selected Tab | 多会话导航 | 单 Session 纵切已完成 |
| terminal grid / PTY / scrollback | grid、PTY 与会话内有界回滚已接通，跨重启持久化尚无 | 活动 Terminal Session runtime | 部分具备 |
| multi-session projection / switching | 当前只有一个真实 Session Tab | 多会话入口 | 尚未完成 |

当前不增加通用 Sash、Panel、Auxiliary Bar 或可任意调整尺寸的 Part 系统。窗口 resize 最终应从
Terminal Workspace logical viewport 计算 terminal rows/columns，再把同一尺寸发送给 grid 和
PTY；Split Pane 只有在多个真实 Terminal Pane 出现后才作为 Workspace 内部能力实现。

## 当前执行路径

```text
main
  → zeta_winit::run_application
  → NativeApp::resumed
      → NativeWindow::create
      → NativeWindow::window_control_insets
      → build_shell_presentation
      → WgpuRenderer::initialize
      → request_redraw
  → NativeApp::window_event
      → resize / scale-factor update → rebuild scene
          → TerminalSession::resize → TerminalCore + PTY
      → TerminalSessionEvent::Output → TerminalCore::process_output → rebuild scene
          → terminal query → take_reply_bytes → TerminalSession::send_input → PTY
          → OSC 133 D → BlockList::complete_command
      → TerminalSessionEvent::Exited → TerminalCore::mark_process_exited → rebuild scene
      → cursor / primary mouse event → titlebar drag 或 terminal cell mapping
      → alternate-screen terminal pointer → cell mapping → TerminalPointer → PTY
      → primary-screen wheel → TerminalScroll → retained Block/grid viewport → redraw
      → primary-screen drag → TerminalSelection → selection paint
          → Cmd/Ctrl+C → system clipboard
      → primary Cmd/Ctrl+V / keyboard → TerminalComposer
      → input_method → IME preedit/commit/cancel → TerminalComposer
          → Enter → TerminalSession::submit_command → PTY + TerminalCore::start_command
          → composer caret bounds → native IME candidate area
      → pointer → zeta_ui_dispatch::UiDispatch
          → InteractionFrame reverse-order hit-test
          → hover / press / capture / focus → presentation rebuild
          → UiIntent → window drag 或 product action
      → Tab / Shift+Tab / Arrow keys → unified focus navigation
          → Enter / Space → focused action activation
      → alternate keyboard / paste → TerminalCore encoding → PTY
      → input_method → IME commit → TerminalCore encoding → PTY
      → titlebar drag hit → NativeWindow::start_window_drag
      → visible-after-occlusion → request redraw
      → WgpuRenderer::render_scene
```

运行：

```bash
cargo run --manifest-path zeta-rs/Cargo.toml -p zeta-native
```

`shell_scene::ShellLayout` 把 primary logical viewport 分成 product-drawn titlebar、上方 output
viewport 与固定底部 composer；alternate screen 临时使用完整 workspace。`NativeApp` 每次重建
presentation 时从 `NativeWindow::window_control_insets` 读取 host chrome 占位，并通过
`ShellPresentationModel` 交给 `titlebar::Titlebar`；Titlebar 在占位外再增加自己的 `8px`
内容间距。窗口控件宽度和所在边不能进入 `ActionBar` 或 `zeta-ui`。
`shell_interaction` 只声明产品稳定 `ElementId`、sidebar presentation state 类型，并把 context
action 映射回产品命令；实际 state 由 `NativeApp` 保存，它不保存 hover 或 focus。
`zeta-ui-dispatch` 是跨 native 组件的通用
分发 crate：
`InteractionFrame` 按 scene 构建顺序注册有父子关系的 `UiNode`，反向命中最上层节点，并投影
accessibility role、label、bounds 与 focused state；`UiDispatch` 跨 frame 保存
hover path、press/capture 和 focused identity，最后只返回 `UiIntent`。

标题栏、侧栏开关、会话 `TabList`、终端输出、`ComposerPanel`、`InputBox`、
`ActionBar` 和四个上下文 `Button` 都走这条路径。
ComposerPanel 使用默认指针，输入与终端文本使用文本指针，Button 使用 pointer 指针；绘制、
命中与语义 bounds 共享 `ActionBar::interactive_item_bounds`。primary screen 默认聚焦 composer；
Tab/Shift+Tab 遍历全部 tab stop，toolbar 内左右键移动相邻 Button，Enter/Space 激活焦点
action，Escape 从 action 返回 composer。pointer press 同时更新 focus，release 按 capture identity
决定是否激活。`input_method::InputMethodTarget` 合并窗口活动状态、screen 与 focus；焦点离开
composer 或窗口失焦时关闭 primary IME、取消 composition 并停止 caret blink，alternate
screen 仍保留直接输入所需的 IME。过小 viewport 使用有边界的 compact fallback。

`ShellPresentation::accessibility_nodes` 已保存当前 frame 的语义快照，但 `zeta-winit` 尚未提供
AccessKit 或各平台原生 accessibility adapter，因此当前不能声称 VoiceOver、Narrator 或
Orca 已能读取这些节点。后续 adapter 只能发布现有语义树和 focus，不得另建第二套控件身份。

`terminal_composer::TerminalComposer` 拥有 primary screen 的 `TextInput`。`terminal_input`
把普通 key 和 paste 路由到 composer；Enter 调用
`TerminalSession::submit_command`，只有 PTY write 入队成功后才建立 Block 并清空输入。
alternate screen 的输入仍经过 `TerminalCore::encode_key/encode_paste` 直接写入 PTY。
`input_method` 单独把 IME 路由到当前 `InputMethodTarget`：primary candidate area 跟随
composer caret，alternate screen 跟随 grid cursor。
`workspace_context::WorkspaceContext` 在 Session 启动时捕获真实 cwd 和 Git snapshot；
`InputContextToolbar` 只消费四项 context value，使用 `ActionBar` 统一排列并把每项语义交给
`ActionBarButton::icon_and_label` / `Button`。`IconLabel` 只在 Button 内部完成 icon/text
placement，不作为 Toolbar 的直接 action representation。每条命令完成后刷新 branch 与 diff
count；四项 Button 已接入 hover、press、focus、键盘导航和 pointer feedback，当前尚未绑定
environment/directory/branch/diff picker command。在 shell integration
提供 cwd 事件前，目录标签仍表示 Session 启动目录。
PTY output 中的 device/status/cursor query 由 `TerminalCore` 生成 reply bytes；
`TerminalSession::handle_event` 在同一次 output event 后取出并写回同一 PTY，renderer 不参与。
`terminal_pointer::TerminalPointer` 只在 alternate screen 且应用启用 tracking mode 时接管 terminal
viewport，维护 held button 与最后一个有效 cell。titlebar 和 terminal padding 外部仍走
产品 hit testing；1000/1002/1003 filtering 与 legacy/1006 wire encoding 委托 `TerminalCore`。
`terminal_scrollback::TerminalScroll` 只保存当前视口距底部的行偏移和触控板小数滚动量；主屏
滚轮在 BlockList 已建立后浏览命令 transcript，否则浏览 `TerminalGrid` 的 cell history。用户停在
旧输出时，新输出增加相同偏移以保持内容锚定；提交新命令会回到底部。alternate screen 请求
mouse report 时，滚轮优先写入 PTY，不改变产品回滚位置。
`terminal_selection::TerminalSelection` 同样只拥有可丢弃的 viewport selection。主屏左键拖拽
跨过至少一个 cell 后才生成选区，单击不会留下蓝色矩形；宽字符复制按 display width 截取。
macOS 使用 `Cmd+C/V`，其他平台保留未加 Shift 的 `Ctrl+C/V` 终端语义，并使用
`Ctrl+Shift+C/V` 访问剪贴板。OSC 0/2 title 同时投影到 native window 和 Session Tab，不改变
内部品牌名。

`terminal_session::shell_bootstrap` 对支持的 POSIX shell 关闭 PTY echo 并隐藏原生 prompt，
`BootstrapOutputFilter` 在 bootstrap marker 前不向产品暴露启动噪声。zsh 额外安装最小
`precmd` hook，以 OSC 133 `D` 完成活动 Block。这个 hook 还不是可协商、可版本化的完整 shell
integration；其他支持 shell 目前不能可靠报告每条命令的完成状态、cwd 或 exit status。

当前仍没有 App Server、多 Session lifecycle/switching、多行/历史/建议式 Block Editor、
双击词/三击行选择、selection auto-scroll、跨进程重启的回滚/Block 持久化、完整
DEC/query/mouse family 或平台
accessibility adapter。内部语义树与统一 focus 已具备。alternate screen 已具备基础 direct
key/IME commit/clipboard 和请求式 mouse input，但
尚不能据此声明完整 TUI compatibility。这些后续纵切不应进入 `zeta-winit` 或 `zeta-wgpu`。

`zeta-ui-dispatch` 的 crate-level 实现契约见
[`ui-dispatch/README.md`](../ui-dispatch/README.md)。后续 file tree、tabs、chat 和 editor
首次接入时，应由各自 presentation owner 分配稳定
`ElementId`、注册父子节点、role/label/bounds、focus policy 与 activation intent。动态行或 tab
必须在仍表示同一对象时保持 identity；domain selection、document model、chat turn 或 filesystem
state 不进入 `zeta-ui-dispatch`。

macOS 可能在新窗口激活完成前把首次 surface acquisition 报为 occluded；该 frame 会被跳过。
`NativeApp` 在后续 `WindowEvent::Occluded(false)` 上重新请求 redraw，保证首个可见 frame 不会
因为一次正常的 activation transition 永久丢失。
