# zeterm：Terminal compatibility 结构与演进

> 状态：Compatibility。Agent-first 产品结构、Thread authority、统一 Composer 与 direct Shell
> Turn 以 [`native-agent-console.md`](native-agent-console.md) 为 canonical。本文只维护独立
> Terminal Surface、PTY、grid、screen mode、selection 与 terminal protocol 的兼容性边界；
> 下文仍出现的 terminal-first 产品描述属于历史实现记录，不再代表当前主界面。

> 本文是 `zeterm` terminal compatibility 和分阶段演进的 canonical 文档。
> 当前源码所有权、调用路径和测试入口见
> [`zeta-native` README](../zeta-rs/native/README.md)；terminal grid 与 BlockList 的实现契约见
> [`zeta-terminal` README](../zeta-rs/terminal/README.md)；文本输入、IME 与 caret 的跨 crate
> 所有权见 [`native-text-input.md`](native-text-input.md)；原生窗口 chrome 与控件占位的实现
> 契约见 [`zeta-winit` README](../zeta-rs/winit/README.md)。

## 快速理解

`zeterm` 的产品根节点是一块完整的现代终端界面，不是由 Sidebar、Panel、Editor 等通用
Workbench Part 拼装出来的 IDE 外壳。Top Bar、Session 导航和附加操作都服务于终端会话；活动
终端始终占据窗口的主要内容区域。

当前视觉采用浅色扁平界面：Top Bar、Block 输出画布与底部输入面板只用背景层级和一像素
分隔线建立结构，不使用悬浮卡片、厚描边或大圆角。未接入真实状态的搜索、Agent、Git 和
Session action 不以静态装饰出现。

| 用户场景 | 界面行为 | 当前状态 | 深入阅读 |
| --- | --- | --- | --- |
| 打开 `zeterm` | 启动默认 shell；主屏上方显示 Block 输出，底部固定显示命令编辑器 | 最小 PTY/BlockList 纵切已实现 | [当前实现](#4-当前实现) |
| 输入 shell 命令 | 键盘、IME 和 paste 先编辑底部 composer；Enter 才建立 Block 并把完整命令写入 PTY | 单行 Block Input Editor 已实现 | [当前实现](#4-当前实现) |
| 使用 `vim`、`top` 等交互式 TUI | alternate screen 临时接管 Terminal Workspace，切回 primary 后恢复 BlockList 与底部输入 | 部分具备；scroll region、常见 query 与主流 mouse modes 已接通 | [当前实现](#4-当前实现) |
| 浏览较早的主屏输出 | 在终端内容区滚轮上翻 Block transcript 或 cell history，新输出不抢走当前阅读位置 | 会话内有界回滚已实现；跨重启持久化尚无 | [当前实现](#4-当前实现) |
| 复制或粘贴终端文本 | 主屏优先复制 composer selection，再复制 Block 输出 selection；macOS 使用 Cmd+C/V，Windows/Linux 的 alternate terminal 使用 Ctrl+Shift+C/V | 快捷键已通过统一 command resolver；未加 Shift 的 terminal Ctrl+C 继续透传 | [当前实现](#4-当前实现) |
| 自定义快捷键 | 按 `Cmd/Ctrl+,` 打开设置并点击命令录制，或编辑 `<ZETA_PROFILE_ROOT>/keybindings.json` | 最多四段 Chord；设置写入采用原子替换，错误更新保留上一份有效规则并显示诊断 | [当前实现](#4-当前实现) |
| 查看当前会话导航 | Top Bar 按钮展开垂直 Session TabList；拖动右边界可调整宽度 | 单个真实 Session Tab 与可调宽度已实现，不显示 fixture | [当前实现](#4-当前实现) |
| 浏览工作区文件或变更 | Top Bar 按钮展开右栏；Changes / Files 切换单一 Pane，Files 可刷新 Git 状态或模糊搜索 | 根目录、上游领先/落后数和文本 MultiDiff 已接入 | [当前实现](#4-当前实现) |
| 操作当前会话 | 右键 Session Tab，在锚点附近打开 Pin、Close、Rename、Fork 菜单 | 菜单呈现、定位、关闭和键盘交互已实现；真实 command transition 等待多会话 runtime | [当前实现](#4-当前实现) |
| 切换多个会话 | 在同一垂直 TabList 选择另一个真实 Session | 尚未实现；当前只有一个 PTY Session | [分阶段演进](#6-分阶段演进) |
| 在 macOS 使用 Top Bar | 左侧 action 避开系统红绿灯占位并保留组件间距 | 70px host 占位 + 8px Titlebar 间距已实现 | [尺寸语义](#5-尺寸语义) |
| 调整窗口尺寸 | 从同一 viewport 重算 rows/columns，并同步 resize grid 与 PTY | 已实现 | [尺寸语义](#5-尺寸语义) |
| 拆分终端 | 只在 Terminal Workspace 内拆成多个 Pane，并调整 Pane 比例 | Grid 几何入口已完成；多 Pane runtime 尚未实现 | [Terminal Pane 分屏](#62-terminal-pane-分屏) |

## 1. 产品命名

命名规则只有一条：仓库内部统一属于 Zeta，只有跨过发布边界、真正交付给用户的 Native 终端
应用使用全小写名称 `zeterm`。它来自 “Zeta Terminal”，但完整形式不作为第二个品牌。

| 边界 | 规范名称 | 示例 |
| --- | --- | --- |
| 仓库、Cargo package、crate、build target 和内部标识 | `zeta` / `zeta-*` | `zeta-native`、`zeta-ui`、`zeta-terminal-*` |
| 发布的终端应用及其用户可见表面 | `zeterm` | executable、app bundle、窗口标题、Top Bar 和输入提示 |

因此不把内部 crate、CSS、协议或测试标识重命名为 `zeterm-*`。公开发布前仍需单独核查 `zeterm`
的商标、域名、应用商店和软件包名称可用性；核查结果只影响发布层名称。

## 2. 产品结构

当前产品结构：

```text
zeterm
├─ TopBar
│  ├─ window drag region
│  ├─ session sidebar toggle ActionBar
│  └─ agent sidebar toggle ActionBar
├─ SessionSidebar (collapsible)
│  └─ SessionTabList
│     └─ current real TerminalSession Tab
├─ SessionContextMenu (transient overlay)
│  └─ Pin / Close / Rename / Fork
├─ TerminalWorkspace
│  └─ active TerminalSession
│     ├─ BlockOutputViewport → BlockList
│     └─ CommandInputEditor (fixed bottom)
└─ AgentSidebar (collapsible)
   ├─ Pane ActionBar: Changes / Files
   └─ Active Pane
      ├─ Files: toolbar + file tree / fuzzy search
      └─ Changes: MultiDiffEditor
         ├─ file A section → DiffEditor
         └─ file B section → DiffEditor
```

primary screen 的结构固定为“上方输出、底部输入”。键盘、IME 和 paste 先进入
`CommandInputEditor`；Enter 通过 host-owned command boundary 建立 Block，再把命令写入 PTY。
shell prompt、命令回显和行编辑不再作为 primary screen 的产品界面。输出仍由真实 PTY 产生，
BlockList 只投影已提交命令及其 printable output，不使用静态 transcript。

alternate screen 是协议兼容的明确例外：`vim`、`top` 等程序请求 alternate buffer 后，全幅 grid
临时接管 Terminal Workspace，输入直接交给该程序；退出 alternate screen 后恢复固定底部
composer。这个切换不能改变 primary screen 的 Block 输入语义。

Top Bar 不是独立工作区，也不拥有终端 Session。它只提供窗口拖动、会话入口和少量全局操作。
Session Navigation 当前使用可折叠、可通过右边界 Sash 调整宽度的垂直 TabList，但不构成
可注册任意区域的通用 Sidebar Part。TabList 只投影当前真实 PTY Session；在多会话模型接入前
不会增加演示行。

## 3. 所有权

| 能力 | 最终 owner | 职责边界 |
| --- | --- | --- |
| Window、Top Bar 与 Terminal Workspace 外部布局 | `zeta-native` product host | 决定窗口区域和活动会话，不进入 `zeta-ui` |
| 单轴 Pane 尺寸约束、Sash track 与 feedback geometry | `zui::SplitViewLayout` / `zeta-ui::Sash` | 不持有产品显隐、preferred width、pointer capture 或持久化 |
| 递归 Pane geometry 与 owning-split Sash 路由 | `zui::GridLayout` | 递归组合 SplitView；不持有 Terminal Session、Agent content、Pane Tree mutation 或 active Pane |
| Terminal Pane Tree、Session-to-Pane binding 与 Pane 状态 | 后续 native Terminal Workspace model | 已确认是终端分屏必需边界；当前尚未完成 |
| Session Navigation 显隐、preferred width 与 resize gesture | `zeta-native::session_sidebar` | 使用通用 Split/Sash geometry；不拥有 Session lifecycle |
| Agent Sidebar 显隐与尺寸策略 | `zeta-native::agent_sidebar` | 只向外层 Grid 提供固定宽度 sizing；不拥有内部 Pane、文件或 diff model |
| Agent Sidebar 内部 Pane composition | `zeta-native::agent_sidebar_layout` / `AgentSidebarWorkspace` | 组合左侧 pane ActionBar、顶部 toolbar 与单一 active content pane |
| Files 树、模糊搜索与领先/落后显示 | `zeta-native::explorer_pane` / `zeta-file-search` / `zeta-git` | Native 保存可丢弃 UI 状态；Git 命令解析和模糊匹配器仍由各自 crate 拥有 |
| 多文件差异内容与视口 binding | `zeta-native::editor_pane` / `zeta-editor::MultiDiffEditor` | Native 保存 changed-file collection 与每文件 `DiffEditorState`；MultiDiffEditor 在一个滚动文档中连续组合所有可见 DiffEditor |
| 通用 UI 滚动 geometry、交互映射与状态 transition | `zeta-ui::ScrollView` / `ScrollState` / `ScrollbarController` | MultiDiff 复用完整 logical-pixel 状态和交互映射；BlockOutputViewport 通过 Native adapter 复用 clip、内容坐标和 scrollbar paint；Terminal 仍保留底部相对行锚定与输出增长策略 |
| Top Bar 内部 action 排列 | `zeta-ui::ActionBar` | 后续有真实 action 时使用；只拥有 representation geometry 和 paint |
| 通用 Tab surface 与横/纵排列 | `zeta-ui::Tab` / `TabList` | 只拥有 presentation state、item size/gap、surface paint 和同源 bounds；不拥有 product content 或 tabpanel |
| Session Tabs 与活动 Session presentation | Native session navigation control | 消费权威 Session projection，不复制 Session lifecycle |
| 锚点浮层定位、viewport 约束与 layer 合成 | `zeta-ui::ContextView` | 不拥有显示生命周期、输入路由或产品 action |
| 无边框下拉 surface、可选 header、纵向 item geometry 与默认选择 | `zeta-ui::Dropdown` | 组合 ContextView/ActionBar；不拥有产品查询、选择 identity、关闭或 command |
| 柔和阴影、2px menu padding、4px radius、纵向 item geometry 与默认选择 | `zeta-ui::ContextMenu` | 组合 ContextView/ActionBar；不拥有 Session identity、关闭或 command |
| Session Tab 右键菜单生命周期与 command identity | `zeta-native::session_context_menu` | 保存目标、锚点与恢复焦点；菜单关闭后不保留第二份 Session 状态 |
| Product command identity 与执行 | `zeta-native::commands` | pointer、menu 和 shortcut 只提供入口，业务行为汇合到同一 `NativeCommand` executor |
| 平台无关按键、规则顺序与冲突解析 | [`zeta-keybinding`](../zeta-rs/keybinding/README.md) | 不读取 winit event、focus、terminal state 或用户配置，不执行产品 command |
| winit 按键转换、Native context 与 Chord 生命周期 | `zeta-native::keybindings` | 内建 Copy/Paste；1.5 秒超时，失焦或 IME 取消；保持 alternate terminal Control 序列透传 |
| Native 用户快捷键资源 | `zeta-native::keybindings_resource` | 读取 `<ZETA_PROFILE_ROOT>/keybindings.json`；完整校验成功才替换，坏更新保留上一份规则 |
| 快捷键模型、设置、录制和提示 | [`zeta-keybinding`](../zeta-rs/keybinding/README.md) | 同一 feature crate 拥有规则解析、浮层 lifecycle、录制 deadline、诊断呈现和组件样式；Native 提供产品 command、事件 adapter 与保存接线 |
| Terminal Session product state | App Server/terminal session runtime | 拥有进程、cwd、环境、输出与退出状态 |
| Terminal grid、screen/mode state、基础 escape sequence 与 BlockList | `zeta-terminal::TerminalCore` | 不由 `UiScene` 或 `InputBox` 推断 |
| PTY process、write、resize 与 exit | `zeta-native::terminal_session` + `zeta-utils-pty` | process mechanism 与 terminal model 分离 |
| cell scrollback retention | `zeta-terminal::TerminalGrid` | 会话内最多保留 10,000 行；不负责跨重启持久化 |
| scroll position | `zeta-native::terminal_scrollback` | 可丢弃的 presentation state，不写回 terminal model |
| terminal output selection | `zeta-native::terminal_selection` | 可丢弃的 viewport state；文本来自 terminal/Block projection |
| 跨重启历史持久化与完整 terminal compatibility | 后续 terminal runtime | 尚未完成 |
| BlockList / TerminalOutput presentation | Native terminal session view | 呈现 runtime output；不能成为第二份权威输出存储 |
| Primary Block Input Editor 与 IME candidate area | `zeta-native::terminal_composer` + `input_method` | 编辑 host-owned `TextInput`；Enter 才提交真实 command boundary |
| 命中、指针状态、focus、键盘导航与 accessibility semantics | `zeta-ui-dispatch` | 只分发稳定控件身份和 activation intent，不保存 Session、文件、对话或文档状态 |
| 平台 accessibility publication | 后续 `zeta-winit` adapter | 当前尚未接 AccessKit/平台 API，内部语义树不等于屏幕阅读器已可用 |
| alternate-screen direct input | `zeta-native::terminal_input` + `input_method` + `TerminalCore` | 仅在 TUI 接管期间编码 key/IME/paste 并写入 PTY |
| shell command completion boundary | `zeta-native::terminal_session` bootstrap + `zeta-terminal::TerminalCore` | 当前 zsh 使用 OSC 133 `D`；其他 shell 只有基础 prompt/echo suppression |
| Rect、icon、text scene 与 GPU draw | `zui` / `zeta-wgpu` | 不拥有 Session、PTY、窗口布局或产品 reducer |

`zeta-native` 可以保存活动 Tab、hover、focus、scroll position 等可丢弃 presentation state，但
Session、Thread、Turn、PTY process 和 durable output 必须来自对应 runtime。

## 4. 当前实现

| 当前实现 | 当前事实 | 目标映射 |
| --- | --- | --- |
| `titlebar::Titlebar` | 绘制 32px 窗口顶区、拖拽区和左右 sidebar toggle `ActionBar`；不显示标题文案 | Top Bar |
| `zeta-winit::WindowControlInsets` | 按 native chrome policy 提供覆盖产品内容的左右逻辑占位；macOS full-size titlebar 当前为左侧 70px | 原生窗口控件安全区 |
| `session_tab_list::SessionTabList` | 组合 `zeta-ui::TabList` 的无边框 4px 圆角 surface；自身绘制与两行信息块等高的白色状态容器及会话名/工作区截断文字，并注册 Session Tab 语义 | 通用 TabList 已支持 6px 间隔的多项布局；多 Session projection/switching 尚未接入 |
| `session_context_menu::SessionContextMenu` | 右键当前真实 Session Tab 后，用通用 `ContextMenu` 基座绘制 Pin、Close、Rename、Fork；基座提供 renderer 柔和阴影、2px padding 与 4px radius，默认选择 Pin；菜单子树打开时成为 modal interaction scope，hover 同步 roving focus 并在移出后保留最后一项，同时支持菜单外点击、Escape、上下键、Tab、Enter/Space 与焦点恢复 | 下层控件在菜单打开期间不接收 pointer、focus 或 activation；四项已映射为稳定 product action，单 Session runtime 尚不执行真实 pin/close/rename/fork transition |
| `ShellLayout` | 组合扁平 titlebar、可选 Sessions sidebar，并把剩余区域交给 `TerminalWorkspaceLayout` | primary screen 窗口外层布局 |
| `TerminalWorkspaceLayout` / `zui::GridLayout` | 把活动 Terminal 与可选 Agent Sidebar 投影为递归 Grid Leaf；alternate screen 使用完整活动 Terminal Leaf | Agent Sidebar 已接入；多 Terminal Pane runtime 尚未完成 |
| `SessionSidebarState` / `Sash` | 保存 preferred width 和 drag-start snapshot；从同一 track 生成 8px 命中区与 2px hover/active feedback | 侧栏宽度限制为 160–480px，并始终为 main Pane 保留至少 240px |
| `AgentSidebarState` | 保存右栏显隐并向外层 Grid 提供固定 320px sizing | 内部内容由 `AgentSidebarWorkspace` 独立拥有 |
| `AgentSidebarLayout` / `AgentSidebarNavigation` / `AgentSidebarToolbar` | 64px Changes/Files ActionBar、36px toolbar 与单一 active Pane | Files-only toolbar 显示 Refresh、`↑ahead ↓behind` 与 Search；Changes 不注册这些 action |
| `ExplorerPane` / `zeta-file-search` | 根目录文件树与工作区路径模糊匹配结果 | Search 输入已接键盘、剪贴板和 IME；目录展开、滚动和文件打开尚未完成 |
| `EditorPaneState` / `zeta-editor::MultiDiffEditor` | `zeta-git` changed-file snapshot 生成 HEAD/working-tree DiffDocument；MultiDiffEditor 持有整体纵向视口 | 启动、Refresh 与 command completion 更新；binary、非 UTF-8 或单侧超过 2 MiB 的文件跳过 |
| `commands::NativeCommand` / `keybindings::NativeKeybindings` / `keybindings_resource::KeybindingsResource` / `keyboard_shortcuts` | pointer/menu 与标准化键盘事件汇合到同一 command executor；resolver 支持 `when`、Builtin/User precedence、blocker 和最多四段 Chord；资源轮询外部编辑，设置录制采用原子写入 | ✅；内建 Copy/Paste、1.5 秒 Chord timeout、失焦/IME 取消、冲突诊断、Chord 提示与设置 UI 已实现 |
| `TerminalCore` / `TerminalGrid` | 增量解析 ANSI，维护 cell、cursor、wrap、erase 与基础 SGR | 当前最小 terminal emulator core |
| Unicode terminal text | CJK 按双 cell 保存；组合符、ZWJ Emoji 与 flag 序列保留在 leading cell；renderer 使用系统 outline fallback | macOS 已规避不可栅格化的 `GB18030 Bitmap`；复杂 BiDi 行级布局尚未完成 |
| primary/alternate screen | 解析 `47/1047/1048/1049`，切换 active grid，并在 resize 时同步两块 grid | 已实现基础 buffer lifecycle |
| DEC mode 与 terminal input | 记录 cursor key、cursor visibility、bracketed paste 和 mouse request state；编码基础 key | 常见 input mode 已形成闭环 |
| scroll region 与 terminal query | 支持 margin scrolling、origin-relative cursor、line insert/delete，并把 DA/DSR/CPR reply 写回 PTY | 常见纵切已实现，尚非完整 query family |
| terminal mouse report | alternate screen 内把 pointer cell、button/motion/wheel 和 modifiers 编码为 1000/1002/1003 legacy 或 1006 SGR report | 不接管 titlebar；1005/1015 尚未实现 |
| 主屏 scrollback | full-screen scroll 把 cell rows 保留到 10,000 行有界历史；局部 scroll region 和 alternate screen 不进入历史 | `CSI 3 J` 清理历史；当前没有磁盘持久化 |
| Native 回滚浏览 | 主屏滚轮浏览 Block transcript/cell history，并在阅读旧输出时保持新输出锚定；`TerminalOutputScrollView` 把行窗口映射到通用 `ScrollView` | alternate screen 的应用鼠标报告优先于产品滚动；Terminal 行锚定不下沉到 UI 基座 |
| resize reflow | 主屏按 soft-wrap metadata 重排 history/live rows，并映射 cursor、pending wrap 与 wide cells | alternate screen 和自定义 scroll region 保持 fixed-grid resize |
| selection / clipboard | 主屏复制 composer 或 Block 输出 selection，paste 编辑 composer；alternate screen 按 terminal mode 写入 PTY | 尚无双击词、三击行和 selection auto-scroll |
| OSC title | 解析 OSC 0/2 并同步 native window 与 Session Tab | 标题去 control characters，限制为 256 字符 |
| `BlockList` | host submit 建立 Block，保存有界 printable output，过滤 PTY echo，并在 OSC 133 `D` 上完成当前 Block | primary screen 的权威 output projection |
| `terminal_session::TerminalSession` | 启动默认 shell、抑制原生 prompt/echo、提交整条命令、转发 PTY output/exit、处理 resize | zsh 已有最小 completion hook；其他 POSIX shell 只有基础 bootstrap |
| `TerminalComposer` / `terminal_input` | primary screen 编辑 `TextInput` 并在 Enter 时提交 | 当前为单行 |
| `input_method` | 根据 window、screen 与 focus 选择 Disabled/Composer/TerminalGrid，转换 IME 事件并同步 candidate area | preedit 状态由共享 `TextInput` 模型维护 |
| 输入上下文工具栏 | Bottom Widget 最底部用 `ActionBar` 排列四个带图标标签的 `Button`：Local、当前工作区目录、Git branch 与 diff count | 目录按钮复用带 header slot 的 `Dropdown`，分支按钮复用带同类 header 的 `ContextMenu`；两者第一行均默认聚焦 Search Box，并分别实时过滤当前层级子目录与本地分支。成功后替换 Files 根、搜索索引和 Git/Changes 投影；环境选择器尚未绑定 |
| 统一 UI 分发 | `zeta-ui-dispatch` 的 `ElementId`、父子 `UiNode`、反向 hit-test、focus order、同组导航、`UiIntent` 与每帧 accessibility snapshot | 当前 Titlebar、Session TabList、Session Menu/MenuItem、Sash separator、terminal output、composer、toolbar 和 Button 已接入；平台 accessibility adapter 尚无 |
| primary/alternate Native presentation | primary 绘制 BlockList + 固定底部 composer；alternate 绘制全幅 active grid/cursor | Warp 式主屏与 TUI compatibility 已分流 |
| `ActionBar` / `Button` | presentation-only action 与 icon button | 保持通用 primitive，不接收 terminal domain state |
| `TabList` / `Tab` | presentation-only Tab 排列与 surface | 当前用于 Session navigation；changed-file diff 不再使用 Tab |
| 完整 DEC/query/mouse family、跨重启历史持久化 | 尚未实现 | 后续 terminal compatibility / Session durability 纵切 |
| terminal tabs、session restoration、split panes | 尚未实现 | 后续产品能力 |

### 4.1 用户快捷键资源

`<ZETA_PROFILE_ROOT>/keybindings.json` 是严格 JSON 数组。未设置 `ZETA_PROFILE_ROOT` 时，Native 与
App Server 使用同一个操作系统用户 profile state 根；切换工作区不会切换用户快捷键或
Config/Session/Thread authority。每条规则必须包含 `key` 和 `command`；
`command: null` 表示 blocker。`mac`、`linux`、`win` 可以用平台专属按键覆盖 `key`，设为
`null` 表示该平台禁用该条规则。

```json
[
  {
    "key": "primary+k primary+b",
    "command": "workbench.action.toggleSideBar",
    "when": "textInputFocus",
    "mac": "primary+k primary+b",
    "linux": "ctrl+k ctrl+b",
    "win": "ctrl+k ctrl+b"
  },
  {
    "key": "ctrl+v",
    "command": null,
    "when": "terminalFocus"
  }
]
```

| 字段或语法 | 当前支持 | 失败语义 |
| --- | --- | --- |
| modifier | `primary`、`ctrl`、`meta`、`alt`、`shift` | 重复 modifier，或 `primary` 与显式 `ctrl`/`meta` 组合时拒绝整份资源 |
| key identity | 逻辑键名或 `[PhysicalCode]` | 空键、单个 Chord 多键、超过四段时拒绝整份资源 |
| `when` 语法 | 省略表示 always；支持 `!`、`&&`、`||`、括号、`==`、`!=`、布尔值与字符串值 | 语法错误或未知 context key 时拒绝整份资源 |
| context key | `textInputFocus`、`terminalFocus`、`agentSurfaceVisible`、`terminalSurfaceVisible`、`sessionSidebarVisible`、`agentSidebarVisible`、`fileSearchVisible`、`composerMode` | 布尔 key 参与真假组合；`composerMode` 可与字符串比较 |
| command | Copy/Paste、composer mode、左右 sidebar、Changes/Files、Refresh、File Search 与打开快捷键设置的当前稳定 command ID | 未知或尚未执行真实 transition 的 command ID 拒绝整份资源 |
| 更新 | 每秒比较资源内容，完整编译后替换 Builtin + User rule set | 文件过大、读取失败、JSON 错误、未知字段或任一规则无效时继续使用上一份完整规则 |

设置页使用深灰色 keycap 分别呈现 modifier 和按键字符；同一 Chord 紧密排列，多段 Chord 使用
更大间距。点击命令进入录制，暂停一秒后保存；Escape 取消录制或关闭浮层，窗口失焦取消录制。
用户规则优先于内建规则；同来源、同 priority 的冲突由资源中靠后的规则获胜，并在设置页显示
诊断。等待第二段按键时，底部提示已经输入的 keycap；错误按键会消费并退出 Chord，1.5 秒
超时、窗口失焦或 IME 事件也会退出。

当前可绑定的 command ID 为：

- `editor.action.clipboardCopyAction`、`editor.action.clipboardPasteAction`；
- `workbench.action.toggleComposerMode`、`workbench.action.toggleSideBar`、
  `workbench.action.toggleAuxiliaryBar`；
- `workbench.action.showAgentChanges`、`workbench.action.showAgentFiles`；
- `workbench.action.refreshAgentFiles`、`workbench.action.toggleAgentFileSearch`；
- `workbench.action.openKeyboardShortcuts`。

当前“terminal core 纵切完成”指以下端到端路径已经同时成立：

- 默认 shell PTY 的 command submit、output、reply、resize 和 exit 都经过同一
  `TerminalSession`；
- primary grid、alternate grid、BlockList、scrollback、reflow 与 command echo filtering 各有单一
  权威 owner；
- primary composer 与 alternate direct input、IME、应用鼠标、产品滚轮、selection 和
  clipboard 按当前 screen/mode 正确分流；
- 当前 state 能直接生成 scene、cursor、title 和可复制文本，不从 renderer 反推 terminal state。
- 简中、日文、韩文、组合音标、阿拉伯文字形与 Emoji 已覆盖 shaping/raster regression；terminal
  model 另覆盖 CJK cell width 和 extended grapheme ownership。
- 当前交互节点从同一份 bounds 生成 paint state、hit-test、cursor、focus navigation 与
  accessibility semantics；不存在另一份按坐标猜测控件身份的 hover 表。

这一定义不把“完整 xterm compatibility”、跨重启 Session durability、terminal tabs 或 split
panes 伪装成本阶段能力；它们仍是表中单独列出的后续纵切。

当前 Top Bar 会显示左右两个 sidebar toggle。左侧展开后只有当前真实 PTY Session 对应的一项
selected Tab；后续新增 Session Tab 必须消费权威多会话 projection，不能用 fixture 占据导航
空间。右侧展开后包含 Changes / Files ActionBar、Files-only toolbar 与单一 active pane。
Files 默认投影工作区
根目录，Search 使用模糊路径索引；Changes 把当前 Git 状态快照的全部文本变更作为
`MultiDiffEditorItem` 放入同一个滚动文档，每项保留独立的 DiffEditor viewport。没有文件或
变更时仍显示真实空态，不用 fixture 冒充 file tree、plan 或 diff。
当前 Tab 可通过右键打开真实浮层菜单；它只形成 Pin、Close、Rename、Fork 的 presentation 和
command identity，不把点击结果写入虚构的 Session 列表。相应 transition 必须在权威
multi-session runtime 接入后实现。

## 5. 尺寸语义

窗口 resize 的长期执行顺序是：

1. `NativeApp` 接收 physical extent 与 scale factor；
2. `NativeApp` 从 `NativeWindow` 读取窗口控件左右占位，product layout 计算 logical Top Bar、
   可选 Session Sidebar 与剩余 Workspace；`SplitViewLayout` 约束外部 effective width，
   `GridLayout` 把剩余区域投影为活动 Terminal 与可选 Agent Sidebar Leaf，Titlebar action
   在占位外另加 8px 组件间距；
3. primary screen 用 output viewport 计算 rows/columns，固定底部 composer 不计入输出行数；
   alternate screen 用完整 Terminal Workspace 计算 rows/columns；
4. `TerminalSession::resize` 更新 primary/alternate grid，并把 active screen 的相同
   rows/columns 发送给 PTY；
5. host 从同一份 terminal state 构造下一帧 scene。

Session Sidebar 默认宽度为 200 logical pixels，可在 160–480px 范围内调整，并始终为
Terminal Workspace 保留至少 240px。`SessionSidebarState` 保存 preferred width；窗口临时变窄
只改变当前 effective width，收起、重新展开或恢复窗口尺寸后仍使用用户首选值。Sash 拖动触发
同一条 Shell bounds → terminal grid → PTY resize 链路，pointer cell mapping、output 与
composer 共享调整后的 workspace。当前不增加 `PanelHeight`、任意区域拖拽或通用 Workbench
Part 系统。

Agent Sidebar 固定宽度为 320 logical pixels；剩余区域不足 240px 时，即使显隐状态为展开也会
临时隐藏。当前不提供右侧 Sash，后续内容接入不能绕过这条 Terminal Workspace 最小宽度约束。

窗口控件占位由 `zeta-winit` 的 chrome adapter 统一拥有，不属于通用 `ActionBar` 样式。
macOS 当前使用集中且受测试的 70 logical pixel policy；由于 `winit` 尚无安全的 system button
geometry API，RTL 换边和未来 Windows controls overlay 仍是 adapter 扩展点，不能描述为当前
能力，也不能在 `titlebar::Titlebar` 再引入平台常量。实现契约见
[`zeta-winit/README.md`](../zeta-rs/winit/README.md)。

当前 Session Tab 的白色状态圆形只投影 native runtime 可确认的通用 `Active` 语义，尚未绘制
状态 SVG。Planning、Thinking、Editing 等 Agent 阶段尚无权威 Session projection，不能由 UI
从 terminal 输出推断；后续应由 App Server 提供 typed 状态，再把对应 SVG 映射到同一圆形容器。

## 6. 分阶段演进

### 6.1 近期结构整理

- 把当前 zsh 最小 bootstrap 演进为可协商版本的 shell integration，可靠产生 command
  start/end、cwd、exit status，并覆盖更多 shell；
- 把当前单行 composer 演进为支持换行、历史、补全和建议的 Block Input Editor；
- shell integration 提供 cwd 更新后，协调 input context toolbar 选中的工作区与 shell 内的
  `cd`；当前目录标签表示用户选择的 Files/Git 工作区，不能据此推断 PTY 内部 cwd；
- 接入真实多会话 projection 后，把新增 Session 作为同一垂直 TabList 的动态 Tab，并实现
  activation/switching；
- file tree、tabs、chat 和 editor 接入时复用 `zeta-ui-dispatch`：各组件只注册稳定 identity、
  父子关系、语义和 intent，业务模型仍由各自 owner 保存；
- 接入 AccessKit 或平台原生 accessibility adapter，直接发布现有语义树与 focus identity；
- 让 root layout 继续只决定 Top Bar、可选 Session Navigation 和 Terminal Workspace 的外部
  bounds，并把单轴约束委托给 `SplitViewLayout`；
- 保持 `zeta-ui` presentation-only，不把 Session 或 terminal reducer 下沉到组件层。

### 6.2 Terminal Pane 分屏

Terminal Workspace 已确认需要 Pane 分屏。当前先完成通用 `GridLayout` 递归几何，并让现有
活动终端作为单 Leaf 接入；下一阶段由 native Terminal Workspace 增加产品 Pane Tree：

```text
TerminalWorkspace
└─ PaneNode
   ├─ Leaf(TerminalSessionId)
   └─ Split {
        axis,
        ratio,
        first,
        second
      }
```

Terminal Pane Splitter 复用当前 `GridLayout`、`SplitViewLayout` 与 `Sash` geometry，但
Pane Tree、Session-to-Pane binding、active Pane、ratio、逐 Pane scroll/selection/composer
和 Terminal Session lifecycle 仍只属于 Terminal Workspace。通用 presentation primitive
不等于可以注册任意产品区域的 Workbench Part/Sash 系统。

## 7. 明确不做什么

- 不构建 VS Code 风格的通用 Workbench Part、Panel、Auxiliary Bar 或区域注册系统；
- 不让每个视觉区域都具有可拖拽尺寸；
- 不把当前 immutable `GridLayout` 扩张成持有产品状态的 retained Workbench Grid，也不增加
  动态 Part 注册或任意区域 resize；
- 不用静态 session fixture 冒充真实 Block 输出；composer 必须提交到真实 PTY；
- 不把 terminal grid、PTY、scrollback 或 Session 生命周期放入 renderer；
- 不把当前静态 transcript/message fixture 描述成真实 terminal output；
- 不把当前基础 ANSI subset 描述成完整 xterm/VT compatibility；
- 不因 alternate screen 和基础 direct input 已接通，就声称完整支持 `vim`、`top` 等交互式 TUI；
- 不把当前 alternate-screen 1000/1002/1003/1006 支持描述成所有 screen/mouse protocol 均已完成；
- 不因参考现代终端产品而复制某个外部产品的内部模型或视觉细节。

## 8. 长期不变量

- 终端会话是窗口主体，chrome 和导航服务于终端而不是与终端平级；
- primary screen 始终由上方 BlockOutputViewport 与固定底部 CommandInputEditor 组成；
- product host 决定活动 Session、布局和事件路由，`zeta-ui` 只消费 presentation state；
- pointer、menu、shortcut 与后续 command palette 必须执行同一个 product command identity；
- hit-test、hover/press/capture、focus、键盘导航、cursor 和 accessibility semantics 必须共享
  同一个 `ElementId`，不能由各组件建立彼此不一致的状态表；
- terminal viewport、grid rows/columns 和 PTY size 必须来自同一条尺寸链路；
- Session Navigation 不拥有 Session lifecycle 或 durable output；
- Terminal Pane Tree 只属于 Terminal Workspace 内部；通用 Grid/Split/Sash geometry 不拥有
  该产品拓扑；
- 当前实现、计划迁移和潜在能力必须在文档中保持明确分离。
