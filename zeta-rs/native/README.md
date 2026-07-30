# `zeta-native`

> Native 文本输入的跨 crate canonical ownership 见
> [`docs/native-text-input.md`](../../docs/native-text-input.md)；Native 窗口“终端即应用”的
> 产品结构与分阶段演进见
> [`docs/native-terminal-ui.md`](../../docs/native-terminal-ui.md)；本 README 只拥有 native
> host 当前源码路径和接入义务。Terminal grid 与 BlockList 的实现契约见
> [`zeta-terminal` README](../terminal/README.md)；CodeEditor/DiffEditor presentation 契约见
> [`zeta-editor` README](../editor/README.md)。

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
| Sessions/Main 单轴约束与 Sash presentation geometry | `zeta-ui::SplitViewLayout` / `Sash` | 委托 |
| Terminal Workspace 与 Agent Sidebar 递归 Pane geometry | `zeta-ui::GridLayout` | 委托；当前输入树包含活动终端与可选右栏 Leaf |
| Terminal Pane Tree、Session binding 与 active Pane | 后续 native Terminal Workspace model | 尚未完成 |
| Sessions preferred width、显隐与当前 drag snapshot | `session_sidebar::SessionSidebarState` | ✅ |
| Agent Sidebar 显隐与固定宽度策略 | `agent_sidebar::AgentSidebarState` | ✅；不拥有内部 Pane 内容 |
| Agent Sidebar 内部 Explorer/Editor 同级 Pane geometry | `agent_sidebar_layout::AgentSidebarLayout` / `zeta-ui::GridLayout` | ✅；当前为纵向同级 Leaf |
| Changed-file collection、整体滚动与每文件 diff 视口 | `agent_sidebar_workspace::AgentSidebarWorkspace` / `editor_pane::EditorPaneState` / `zeta-ui::ScrollState` | ✅；MultiDiff wheel、scrollbar hover/active/fade、thumb drag 与 track paging 已接入，权威文件变更数据源尚未接入 |
| Titlebar 背景、窗口拖拽区与左右侧栏开关 | `titlebar::Titlebar` | ✅；不绘制可见窗口标题 |
| Top Bar 左右 sidebar toggle `ActionBar` | `titlebar::Titlebar` | ✅ |
| 原生窗口控件占位 | `zeta-winit::WindowControlInsets` | 委托；Titlebar 只增加自身内容间距 |
| 通用 Tab surface 与横/纵排列 | `zeta-ui::Tab` / `TabList` | 委托；不拥有 Session content 或 tabpanel |
| 多行编辑、caret/selection、undo/redo、IME 与 syntax projection | `zeta-editor::CodeEditorDocument` / `CodeEditor` | 委托；crate 已完成，Native EditorHost 输入接线尚未完成 |
| 文本差异计算、行映射与字符级范围 | `zeta-diff::DiffDocument` | 委托；Native 只保存已计算文档，不复制 diff 算法 |
| 并排及多文件只读差异展示 | `zeta-editor::DiffEditor` / `MultiDiffEditor` | 委托；EditorPane 已挂载多文件组合和整体滚轮，真实文件 binding 尚未接入 |
| 可折叠 Session Sidebar、名称搜索与当前真实 Session Tab | `session_sidebar_toolbar::SessionSidebarToolbar` / `session_search::SessionSearch` / `session_tab_list::SessionTabList` | ✅；Add action 已进入命令边界，多会话 runtime 尚无 |
| 可折叠 Agent Sidebar 产品组合 | `shell_scene` / `ExplorerPane` / `EditorPane` / `MultiDiffEditor` | ✅；Explorer 数据源与 changed-file projection 尚未接入 |
| 锚点浮层定位与独立 layer 合成 | `zeta-ui::ContextView` | 委托；不拥有显示生命周期或输入路由 |
| 无边框下拉 surface、item geometry 与默认选择 | `zeta-ui::Dropdown` | 委托；不拥有 Session identity、关闭或 command |
| 柔和阴影、2px menu padding、4px radius、item geometry 与默认选择 | `zeta-ui::ContextMenu` | 委托；不拥有 Session identity、关闭或 command |
| Session Tab 右键菜单、关闭策略与 action 映射 | `session_context_menu::SessionContextMenu` / `SessionContextMenuState` | ✅；四个 action 已进入产品边界，真实多会话 transition 尚无 |
| 浅色扁平 Shell presentation tokens | `shell_style::ShellPalette` | ✅ |
| 稳定控件身份与 shell action 映射 | `shell_interaction` | ✅ |
| 命中、hover/press/capture、focus、键盘导航、cursor 与 accessibility semantics | `zeta-ui-dispatch` | 委托 |
| accessibility semantics → 平台屏幕阅读器 | 尚无 native adapter | 尚未完成 |
| Transparent native chrome 与窗口拖动 adapter | `zeta-winit` | 委托 |
| ANSI parser、terminal grid 与 BlockList | `zeta-terminal::TerminalCore` | 委托 |
| 默认 shell PTY、output/exit event、write 与 resize | `terminal_session::TerminalSession` | ✅ |
| primary BlockList + fixed composer 与 alternate full-grid presentation | `shell_scene` / `terminal_composer` / `terminal_input` | ✅ |
| Composer/Search IME target、事件转换、启停、composition lifecycle 与候选框同步 | `input_method` | ✅ |
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
            → zeta-diff
            → zeta-editor → zeta-ui
                          → zeta-diff
            → zeta-utils-pty
```

`zeta-native` 可以拥有可丢弃的 presentation state 和产品交互，但不能复制 Session、Thread、
Turn 或 Tool 的权威状态机。后续接入只能通过 `zeta-app-server-client` 的 typed contract。

## 产品方向与当前边界

Native App 的目标不是通用 Workbench Part 容器，而是一块以活动终端会话为主体的完整界面。
当前源码名称仍是验证阶段的 shell vocabulary；下面的目标名称不表示对应 runtime 已经实现：

| 当前源码 | 当前能力 | 目标产品语义 | 状态 |
| --- | --- | --- | --- |
| `titlebar::Titlebar` | 窗口拖拽区和左右 sidebar toggle `ActionBar`；不绘制标题文案 | Top Bar | ✅ |
| `ShellLayout` | 组合 titlebar、可选 Sessions sidebar，并把剩余区域交给 `TerminalWorkspaceLayout` | Top Bar 与 Workspace 外部布局 | ✅；Sessions 使用外层单轴 split |
| `terminal_workspace_layout::TerminalWorkspaceLayout` | 用 `GridLayout` 投影活动终端与可选 Agent Sidebar Leaf bounds | Workspace Pane geometry adapter | ✅；尚无多 Terminal Pane Tree 或多 Session binding |
| `shell_scene` | primary 绘制 BlockList/composer，alternate 绘制活动 grid | Terminal Session 的 Output/BlockList | 基础纵切已完成 |
| `terminal_composer` / `terminal_input` | primary 编辑 `TextInput` 并提交整条命令；alternate direct input | Block Input Editor 与 TUI compatibility | ✅ |
| `input_context_toolbar` | `ActionBar` 排列四个 icon-and-label `Button`，投影 Local、cwd、branch 与 diff count | Input/Chat context toolbar | ✅ |
| `session_tab_list` | 组合 `zeta-ui::TabList` 投影当前真实 PTY Session；自身拥有白色状态容器、会话名和工作区两行截断信息，以及纵向 TabList/selected Tab 语义 | 多会话导航 | 通用 TabList 已支持 6px 间隔的多项布局；runtime 仍只有单 Session |
| `session_sidebar_toolbar` / `session_search` | 整行组合 `SearchBox` 与右侧 `ActionBar`；按 session name 执行大小写不敏感过滤，并把 Add 暴露为稳定 action | Session 搜索与新建入口 | 搜索已接通；真实新建 tab 等待多会话 runtime |
| `session_context_menu` | 右键当前 Session Tab 后，用 `ContextMenu` 呈现 Pin、Close、Rename、Fork；拥有 outside click、Escape、键盘导航和焦点恢复 | Session action surface | 通用基座提供柔和阴影、2px padding 与 4px radius，打开默认选择 Pin；命令映射已建立，真实 Session mutation 等待多会话 runtime |
| `zeta-editor::CodeEditor` | 多行 Unicode 编辑、selection、history、IME/syntax projection 与可见行绘制 | CodeEditor | 委托；Native 平台事件、focus、caret blink 和 EditorHost 尚未接入 |
| `agent_sidebar_layout` / `explorer_pane` | 在 Agent Sidebar 内把 Explorer 和 Editor 投影为上下同级 Pane；Explorer 当前只呈现真实空态 | Explorer Pane | Pane composition 已完成；文件树数据源尚未接入 |
| `editor_pane` | 保存 changed-file collection、整体滚动位置、scrollbar pointer capture/animation 和每文件 `DiffEditorState`，把全部文件绑定为 MultiDiffEditor items | Changed-file Editor Pane | 多文件不再通过 Tab 互斥；右栏 wheel、hover/active/fade、thumb drag 和 track paging 已路由到整体视口，真实文件 binding 尚未接入 |
| `zeta-editor::DiffEditor` / `MultiDiffEditor` | DiffEditor 组合左右 CodeEditor；MultiDiffEditor 再纵向组合多个文件 section 并裁剪不可见项 | 多文件差异文档 | 已挂载到 EditorPane；文件读取、diff 计算与持久状态不属于 editor crate |
| terminal grid / PTY / scrollback | grid、PTY 与会话内有界回滚已接通，跨重启持久化尚无 | 活动 Terminal Session runtime | 部分具备 |
| multi-session projection / switching | 当前只有一个真实 Session Tab | 多会话入口 | 尚未完成 |

CodeEditor/DiffEditor/MultiDiffEditor 的实现 ownership、`DiffSideRows` 投影、显示列 contract、
测试和当前限制由 [`zeta-editor` README](../editor/README.md) 维护。Native 负责 changed-file
collection、文件 identity、整体滚动位置和每文件 `DiffEditorState`；MultiDiffEditor 只借用这些
快照完成多文件组合。Native 不能复制代码行或 diff decoration 绘制。文件读取、权威
changed-file projection 和编辑输入仍是后续 host 接线。

当前已用 `zeta-ui::SplitViewLayout` 与 `Sash` 支持 Sessions/Main 的单轴 resize，并用
`zeta-ui::GridLayout` 作为 Terminal Workspace 的递归几何入口。当前 Grid 输入只有一个活动
终端 Leaf，因此窗口 resize 仍从这一 Leaf 的 logical viewport 计算 rows/columns，再把同一
尺寸发送给 terminal grid 和 PTY。多 Pane 还需要 native 拥有 Pane Tree、逐 Pane
`TerminalSession`/scroll/selection/composer 状态以及 split command；这些不能由几何层伪造。

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
      → Agent Sidebar toggle
          → AgentSidebarState visibility
          → shell bounds + terminal grid/PTY resize
          → AgentSidebarLayout
              → ExplorerPane + EditorPane sibling leaves
              → MultiDiffEditor → visible file sections → DiffEditor
      → TerminalWorkspaceLayout
          → GridLayout → active terminal + optional Agent Sidebar leaf bounds
          → terminal rows/columns → TerminalSession resize
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

`shell_scene::ShellLayout` 把 titlebar 下方 body 先交给 Sessions/Main 横向
`SplitViewLayout`，再把剩余区域交给 `terminal_workspace_layout::TerminalWorkspaceLayout`；
后者通过 `GridLayout` 同时投影活动 Terminal Leaf 和可选的右侧 Agent Sidebar Leaf。当前活动
Terminal Leaf 再分成上方 output viewport 与固定底部 composer；alternate screen 临时使用完整
活动 Terminal Leaf。
`SessionSidebarState` 保存 visibility、preferred width 与当前 `SplitViewResizeSnapshot`，
viewport 临时约束只改变 effective width，不覆盖 preferred width。`Sash` 从
`SplitViewSashLayout::track_bounds` 同源计算 drag target 与 hover/active feedback；
`AgentSidebarState` 只保存显隐并向外层 Grid 提供固定 320px sizing；
`TerminalWorkspaceLayout` 解析右栏 Leaf bounds，`AgentSidebarLayout` 再把它解析为同级的
`ExplorerPane` 与 `EditorPane`。`EditorPaneState` 保存 changed-file collection、整体
通用 `zeta-ui::ScrollState` 和每文件 `DiffDocument + DiffEditorState`；`MultiDiffEditor` 在一个纵向
文档中连续绘制所有可见文件 section，每段再复用 `DiffEditor`。当前 runtime 没有权威
changed-file projection，因此默认显示“No files loaded / No changed files”而不注入演示 diff；
文件 binding 尚未接入。指针位于 MultiDiffEditor 时，wheel 会更新其有界整体纵向 offset，不再
落入 Terminal scrollback。
`shell_scene` 只注册产品 `ElementId`、cursor 和 separator semantics。`NativeApp` 每次重建
presentation 时从 `NativeWindow::window_control_insets` 读取 host chrome 占位，并通过
`ShellPresentationModel` 交给 `titlebar::Titlebar`；Titlebar 在占位外再增加自己的 `8px`
内容间距。窗口控件宽度和所在边不能进入 `ActionBar` 或 `zeta-ui`。
`shell_interaction` 只声明产品稳定 `ElementId` 并把 context action 映射回产品命令；实际
sidebar state 由 `session_sidebar` 定义并由 `NativeApp` 保存，它不保存 hover 或 focus。
`session_context_menu` 用 `SessionContextMenuState` 保存当前目标、锚点和待恢复焦点，用
`zeta-ui::ContextMenu` 组合 ContextView 定位、renderer BoxShadow、2px menu padding、4px
radius 和纵向 ActionBar item geometry。打开时默认选择第一个 enabled item `Pin`；菜单子树会
成为当前 interaction frame 的 modal scope，hover 同步 roving focus，移出后保留最后一项，
下层控件不再接收 pointer、focus 或 activation。右键当前真实 Session Tab 打开菜单；菜单外
左键、Escape 或窗口失焦关闭菜单，方向键、Tab、Enter 和 Space 复用统一 focus/activation 路径。
`SessionContextMenuAction` 已将 Pin、
Close、Rename、Fork 映射为产品 command identity；当前 runtime 只有一个 PTY Session，因此
这些 command 尚不执行 pinning、关闭、重命名或 fork，不得由 presentation state 伪造结果。
`zeta-ui-dispatch` 是跨 native 组件的通用
分发 crate：
`InteractionFrame` 按 scene 构建顺序注册有父子关系的 `UiNode`，反向命中最上层节点，并投影
accessibility role、label、bounds 与 focused state；`UiDispatch` 跨 frame 保存
hover path、press/capture 和 focused identity，最后只返回 `UiIntent`。

标题栏、侧栏开关、会话 `SessionTabList`、Session 右键菜单、通用 `zeta-ui::TabList`、终端输出、
`ComposerPanel`、`InputBox`、`ActionBar` 和四个输入上下文 `Button` 都走这条路径。
ComposerPanel 使用默认指针，输入与终端文本使用文本指针，Button 使用 pointer 指针；绘制、
输入上下文 toolbar 的命中与语义 bounds 共享 `ActionBar::interactive_item_bounds`，Session
菜单则共享 `ContextMenu::interactive_item_bounds`。primary screen 默认聚焦 composer；
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
`SessionTabList` 的白色状态圆形保留独立可访问性 label，后续可在其中绘制状态 SVG；当前
native runtime 只投影通用 `Active`，尚不能声称已接入 Planning、Thinking、Editing 等 Agent
执行阶段。这些状态必须在 App Server 提供权威 Session projection 后映射，不能由 UI 根据输出
推断。
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

当前仍没有 App Server、多 Session lifecycle/switching 或 Session action mutation、多行/历史/建议式 Block Editor、
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
