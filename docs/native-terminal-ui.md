# zeterm：Native 终端界面产品结构与演进

> 本文是 `zeterm` 窗口产品结构、终端界面语义和分阶段演进的 canonical 文档。
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
| 复制或粘贴终端文本 | 主屏优先复制 composer selection，再复制 Block 输出 selection；paste 编辑 composer | 基础系统剪贴板闭环已实现；单击不产生选区 | [当前实现](#4-当前实现) |
| 查看当前会话导航 | Top Bar 按钮展开固定宽度的垂直 Session TabList | 单个真实 Session Tab 已实现，不显示 fixture | [当前实现](#4-当前实现) |
| 切换多个会话 | 在同一垂直 TabList 选择另一个真实 Session | 尚未实现；当前只有一个 PTY Session | [分阶段演进](#6-分阶段演进) |
| 在 macOS 使用 Top Bar | 左侧 action 避开系统红绿灯占位并保留组件间距 | 70px host 占位 + 8px Titlebar 间距已实现 | [尺寸语义](#5-尺寸语义) |
| 调整窗口尺寸 | 从同一 viewport 重算 rows/columns，并同步 resize grid 与 PTY | 已实现 | [尺寸语义](#5-尺寸语义) |
| 拆分终端 | 只在 Terminal Workspace 内拆成多个 Pane，并调整 Pane 比例 | 尚未实现 | [潜在 Split Pane](#62-潜在-split-pane) |

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
│  └─ session sidebar toggle ActionBar
├─ SessionSidebar (collapsible)
│  └─ SessionTabList
│     └─ current real TerminalSession Tab
└─ TerminalWorkspace
   └─ active TerminalSession
      ├─ BlockOutputViewport → BlockList
      └─ CommandInputEditor (fixed bottom)
```

primary screen 的结构固定为“上方输出、底部输入”。键盘、IME 和 paste 先进入
`CommandInputEditor`；Enter 通过 host-owned command boundary 建立 Block，再把命令写入 PTY。
shell prompt、命令回显和行编辑不再作为 primary screen 的产品界面。输出仍由真实 PTY 产生，
BlockList 只投影已提交命令及其 printable output，不使用静态 transcript。

alternate screen 是协议兼容的明确例外：`vim`、`top` 等程序请求 alternate buffer 后，全幅 grid
临时接管 Terminal Workspace，输入直接交给该程序；退出 alternate screen 后恢复固定底部
composer。这个切换不能改变 primary screen 的 Block 输入语义。

Top Bar 不是独立工作区，也不拥有终端 Session。它只提供窗口拖动、会话入口和少量全局操作。
Session Navigation 当前使用固定宽度、可折叠的垂直 TabList，不构成能够任意改变宽度的通用
Sidebar Part。TabList 只投影当前真实 PTY Session；在多会话模型接入前不会增加演示行。

## 3. 所有权

| 能力 | 最终 owner | 职责边界 |
| --- | --- | --- |
| Window、Top Bar 与 Terminal Workspace 外部布局 | `zeta-native` product host | 决定窗口区域和活动会话，不进入 `zeta-ui` |
| Top Bar 内部 action 排列 | `zeta-ui::ActionBar` | 后续有真实 action 时使用；只拥有 representation geometry 和 paint |
| Session Tabs 与活动 Session presentation | Native session navigation control | 消费权威 Session projection，不复制 Session lifecycle |
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
| Rect、icon、text scene 与 GPU draw | `zeta-ui` / `zeta-wgpu` | 不拥有 Session、PTY、窗口布局或产品 reducer |

`zeta-native` 可以保存活动 Tab、hover、focus、scroll position 等可丢弃 presentation state，但
Session、Thread、Turn、PTY process 和 durable output 必须来自对应 runtime。

## 4. 当前实现

| 当前实现 | 当前事实 | 目标映射 |
| --- | --- | --- |
| `titlebar::Titlebar` | 绘制 32px 窗口顶区、拖拽区和 sidebar toggle `ActionBar`；不显示标题文案 | Top Bar |
| `zeta-winit::WindowControlInsets` | 按 native chrome policy 提供覆盖产品内容的左右逻辑占位；macOS full-size titlebar 当前为左侧 70px | 原生窗口控件安全区 |
| `session_tab_list::SessionTabList` | 展开后显示当前真实 PTY Session，注册为 TabList/selected Tab | 多 Session 接入后复用同一控件 |
| `ShellLayout` | 计算扁平 titlebar、上方 output viewport 与固定底部 composer panel | primary screen 窗口外层布局；alternate screen 使用全幅 workspace |
| `TerminalCore` / `TerminalGrid` | 增量解析 ANSI，维护 cell、cursor、wrap、erase 与基础 SGR | 当前最小 terminal emulator core |
| Unicode terminal text | CJK 按双 cell 保存；组合符、ZWJ Emoji 与 flag 序列保留在 leading cell；renderer 使用系统 outline fallback | macOS 已规避不可栅格化的 `GB18030 Bitmap`；复杂 BiDi 行级布局尚未完成 |
| primary/alternate screen | 解析 `47/1047/1048/1049`，切换 active grid，并在 resize 时同步两块 grid | 已实现基础 buffer lifecycle |
| DEC mode 与 terminal input | 记录 cursor key、cursor visibility、bracketed paste 和 mouse request state；编码基础 key | 常见 input mode 已形成闭环 |
| scroll region 与 terminal query | 支持 margin scrolling、origin-relative cursor、line insert/delete，并把 DA/DSR/CPR reply 写回 PTY | 常见纵切已实现，尚非完整 query family |
| terminal mouse report | alternate screen 内把 pointer cell、button/motion/wheel 和 modifiers 编码为 1000/1002/1003 legacy 或 1006 SGR report | 不接管 titlebar；1005/1015 尚未实现 |
| 主屏 scrollback | full-screen scroll 把 cell rows 保留到 10,000 行有界历史；局部 scroll region 和 alternate screen 不进入历史 | `CSI 3 J` 清理历史；当前没有磁盘持久化 |
| Native 回滚浏览 | 主屏滚轮浏览 Block transcript/cell history，并在阅读旧输出时保持新输出锚定 | alternate screen 的应用鼠标报告优先于产品滚动 |
| resize reflow | 主屏按 soft-wrap metadata 重排 history/live rows，并映射 cursor、pending wrap 与 wide cells | alternate screen 和自定义 scroll region 保持 fixed-grid resize |
| selection / clipboard | 主屏复制 composer 或 Block 输出 selection，paste 编辑 composer；alternate screen 按 terminal mode 写入 PTY | 尚无双击词、三击行和 selection auto-scroll |
| OSC title | 解析 OSC 0/2 并同步 native window 与 Session Tab | 标题去 control characters，限制为 256 字符 |
| `BlockList` | host submit 建立 Block，保存有界 printable output，过滤 PTY echo，并在 OSC 133 `D` 上完成当前 Block | primary screen 的权威 output projection |
| `terminal_session::TerminalSession` | 启动默认 shell、抑制原生 prompt/echo、提交整条命令、转发 PTY output/exit、处理 resize | zsh 已有最小 completion hook；其他 POSIX shell 只有基础 bootstrap |
| `TerminalComposer` / `terminal_input` | primary screen 编辑 `TextInput` 并在 Enter 时提交 | 当前为单行 |
| `input_method` | 根据 window、screen 与 focus 选择 Disabled/Composer/TerminalGrid，转换 IME 事件并同步 candidate area | preedit 状态由共享 `TextInput` 模型维护 |
| input context toolbar | Bottom Widget 最底部用 `ActionBar` 排列四个 icon-and-label `Button`：Local、启动 cwd、Git branch 与 diff count | Button geometry 已注册到统一 interaction frame，具备 hover/press/capture/focus、pointer feedback、Tab/左右键导航和 role/label；action/picker 尚未绑定；branch/diff 在命令完成后刷新 |
| 统一 UI 分发 | `zeta-ui-dispatch` 的 `ElementId`、父子 `UiNode`、反向 hit-test、focus order、同组导航、`UiIntent` 与每帧 accessibility snapshot | 当前 Titlebar、Session TabList、terminal output、composer、toolbar 和 Button 已接入；平台 accessibility adapter 尚无 |
| primary/alternate Native presentation | primary 绘制 BlockList + 固定底部 composer；alternate 绘制全幅 active grid/cursor | Warp 式主屏与 TUI compatibility 已分流 |
| `ActionBar` / `Button` | presentation-only action 与 icon button | 保持通用 primitive，不接收 terminal domain state |
| 完整 DEC/query/mouse family、跨重启历史持久化 | 尚未实现 | 后续 terminal compatibility / Session durability 纵切 |
| terminal tabs、session restoration、split panes | 尚未实现 | 后续产品能力 |

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

当前会显示 sidebar toggle；展开后只有当前真实 PTY Session 对应的一项 selected Tab。后续新增
Session Tab 必须消费权威多会话 projection，不能用 fixture 占据导航空间。

## 5. 尺寸语义

窗口 resize 的长期执行顺序是：

1. `NativeApp` 接收 physical extent 与 scale factor；
2. `NativeApp` 从 `NativeWindow` 读取窗口控件左右占位，product layout 计算 logical Top Bar、
   可选固定宽度 Session Sidebar 和剩余 Terminal Workspace；Titlebar action 在占位外另加
   8px 组件间距；
3. primary screen 用 output viewport 计算 rows/columns，固定底部 composer 不计入输出行数；
   alternate screen 用完整 Terminal Workspace 计算 rows/columns；
4. `TerminalSession::resize` 更新 primary/alternate grid，并把 active screen 的相同
   rows/columns 发送给 PTY；
5. host 从同一份 terminal state 构造下一帧 scene。

因此当前不增加通用 Part Sash、`SidebarWidth`/`PanelHeight` 状态或任意区域拖拽系统。Session
Sidebar 展开时固定为 200 logical pixels，并从 Terminal Workspace 扣除相同宽度；grid、
PTY resize、pointer cell mapping、output 与 composer 共享缩小后的 workspace。

窗口控件占位由 `zeta-winit` 的 chrome adapter 统一拥有，不属于通用 `ActionBar` 样式。
macOS 当前使用集中且受测试的 70 logical pixel policy；由于 `winit` 尚无安全的 system button
geometry API，RTL 换边和未来 Windows controls overlay 仍是 adapter 扩展点，不能描述为当前
能力，也不能在 `titlebar::Titlebar` 再引入平台常量。实现契约见
[`zeta-winit/README.md`](../zeta-rs/winit/README.md)。

## 6. 分阶段演进

### 6.1 近期结构整理

- 把当前 zsh 最小 bootstrap 演进为可协商版本的 shell integration，可靠产生 command
  start/end、cwd、exit status，并覆盖更多 shell；
- 把当前单行 composer 演进为支持换行、历史、补全和建议的 Block Input Editor；
- shell integration 提供 cwd 更新后，让 input context toolbar 跟随 shell 内的 `cd`；当前目录标签
  仍表示 Session 启动目录；
- 接入真实多会话 projection 后，把新增 Session 作为同一垂直 TabList 的动态 Tab，并实现
  activation/switching；
- file tree、tabs、chat 和 editor 接入时复用 `zeta-ui-dispatch`：各组件只注册稳定 identity、
  父子关系、语义和 intent，业务模型仍由各自 owner 保存；
- 接入 AccessKit 或平台原生 accessibility adapter，直接发布现有语义树与 focus identity；
- 让 root layout 继续只决定 Top Bar、可选 Session Navigation 和 Terminal Workspace 的外部
  bounds；
- 保持 `zeta-ui` presentation-only，不把 Session 或 terminal reducer 下沉到组件层。

### 6.2 潜在 Split Pane

只有真实 Terminal Session 和至少两个 Pane 消费者存在后，才增加 Pane Tree：

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

Splitter 只调整相邻 Terminal Pane 的 ratio，并触发相关 PTY resize。它不是可以应用到 Top Bar、
Session Navigation 或任意产品区域的 Workbench Sash。

## 7. 明确不做什么

- 不构建 VS Code 风格的通用 Workbench Part、Panel、Auxiliary Bar 或区域注册系统；
- 不让每个视觉区域都具有可拖拽尺寸；
- 不为潜在 Session Navigation 预先在 `zeta-ui` 增加通用 Sash primitive；
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
- hit-test、hover/press/capture、focus、键盘导航、cursor 和 accessibility semantics 必须共享
  同一个 `ElementId`，不能由各组件建立彼此不一致的状态表；
- terminal viewport、grid rows/columns 和 PTY size 必须来自同一条尺寸链路；
- Session Navigation 不拥有 Session lifecycle 或 durable output；
- Split Pane 如果出现，只属于 Terminal Workspace 内部；
- 当前实现、计划迁移和潜在能力必须在文档中保持明确分离。
