# zeterm：Native 终端界面产品结构与演进

> 本文是 `zeterm` 窗口产品结构、终端界面语义和分阶段演进的 canonical 文档。
> 当前源码所有权、调用路径和测试入口见
> [`zeta-native` README](../zeta-rs/native/README.md)；terminal grid 与 BlockList 的实现契约见
> [`zeta-terminal` README](../zeta-rs/terminal/README.md)；文本输入、IME 与 caret 的跨 crate
> 所有权见 [`native-text-input.md`](native-text-input.md)。

## 快速理解

`zeterm` 的产品根节点是一块完整的现代终端界面，不是由 Sidebar、Panel、Editor 等通用
Workbench Part 拼装出来的 IDE 外壳。Top Bar、Session 导航和附加操作都服务于终端会话；活动
终端始终占据窗口的主要内容区域。

| 用户场景 | 界面行为 | 当前状态 | 深入阅读 |
| --- | --- | --- | --- |
| 打开 `zeterm` | 启动默认 shell；主屏上方显示 Block 输出，底部固定显示命令编辑器 | 最小 PTY/BlockList 纵切已实现 | [当前实现](#4-当前实现) |
| 输入 shell 命令 | 键盘、IME 和 paste 先编辑底部 composer；Enter 才建立 Block 并把完整命令写入 PTY | 单行 Block Input Editor 已实现 | [当前实现](#4-当前实现) |
| 使用 `vim`、`top` 等交互式 TUI | alternate screen 临时接管 Terminal Workspace，切回 primary 后恢复 BlockList 与底部输入 | 部分具备；scroll region、常见 query 与主流 mouse modes 已接通 | [当前实现](#4-当前实现) |
| 浏览较早的主屏输出 | 在终端内容区滚轮上翻 Block transcript 或 cell history，新输出不抢走当前阅读位置 | 会话内有界回滚已实现；跨重启持久化尚无 | [当前实现](#4-当前实现) |
| 复制或粘贴终端文本 | 主屏优先复制 composer selection，再复制 Block 输出 selection；paste 编辑 composer | 基础系统剪贴板闭环已实现；单击不产生选区 | [当前实现](#4-当前实现) |
| 切换多个会话 | 使用顶部 Tabs 或固定/可隐藏的垂直 Session Tabs | 尚未实现；当前不显示演示导航 | [分阶段演进](#6-分阶段演进) |
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
│  └─ terminal title
└─ TerminalWorkspace
   └─ TerminalSession
      ├─ BlockOutputViewport
      │  └─ BlockList
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
Session Navigation 可以使用水平 Tabs 或固定/可隐藏的垂直 Tabs，但不构成能够任意改变宽度的
通用 Sidebar Part；在真实多会话模型接入前不显示演示数据。

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
| Primary Block Input Editor 与 IME candidate area | `zeta-native::terminal_composer` + `terminal_input` | 编辑 host-owned `TextInput`；Enter 才提交真实 command boundary |
| alternate-screen direct input | `zeta-native::terminal_input` + `TerminalCore` | 仅在 TUI 接管期间编码 key/IME/paste 并写入 PTY |
| shell command completion boundary | `zeta-native::terminal_session` bootstrap + `zeta-terminal::TerminalCore` | 当前 zsh 使用 OSC 133 `D`；其他 shell 只有基础 prompt/echo suppression |
| Rect、icon、text scene 与 GPU draw | `zeta-ui` / `zeta-wgpu` | 不拥有 Session、PTY、窗口布局或产品 reducer |

`zeta-native` 可以保存活动 Tab、hover、focus、scroll position 等可丢弃 presentation state，但
Session、Thread、Turn、PTY process 和 durable output 必须来自对应 runtime。

## 4. 当前实现

| 当前实现 | 当前事实 | 目标映射 |
| --- | --- | --- |
| `titlebar::Titlebar` | 绘制窗口顶区、终端标题和拖拽区 | 演进为 Top Bar；后续容纳真实 terminal tabs/actions |
| session navigation | 当前没有控件，也不绘制静态演示数据 | 有真实多会话 projection 后再增加 |
| `ShellLayout` | 计算 titlebar、上方 output viewport 与固定底部 composer | primary screen 窗口外层布局；alternate screen 使用全幅 workspace |
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
| OSC title | 解析 OSC 0/2 并同步产品 Titlebar 与 native window | 标题去 control characters，限制为 256 字符 |
| `BlockList` | host submit 建立 Block，保存有界 printable output，过滤 PTY echo，并在 OSC 133 `D` 上完成当前 Block | primary screen 的权威 output projection |
| `terminal_session::TerminalSession` | 启动默认 shell、抑制原生 prompt/echo、提交整条命令、转发 PTY output/exit、处理 resize | zsh 已有最小 completion hook；其他 POSIX shell 只有基础 bootstrap |
| `TerminalComposer` / `terminal_input` | primary screen 编辑 `TextInput` 并在 Enter 时提交；IME candidate 跟随 composer caret | 当前为单行；preedit 由共享输入模型维护 |
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

这一定义不把“完整 xterm compatibility”、跨重启 Session durability、terminal tabs 或 split
panes 伪装成本阶段能力；它们仍是表中单独列出的后续纵切。

当前不会显示 sidebar toggle 或静态 session rows。真实 terminal tabs/session navigation 必须
消费权威多会话 projection 后再进入产品，不能先用 fixture 占据窗口空间。

## 5. 尺寸语义

窗口 resize 的长期执行顺序是：

1. `NativeApp` 接收 physical extent 与 scale factor；
2. product layout 计算 logical Top Bar 和 Terminal Workspace；
3. primary screen 用 output viewport 计算 rows/columns，固定底部 composer 不计入输出行数；
   alternate screen 用完整 Terminal Workspace 计算 rows/columns；
4. `TerminalSession::resize` 更新 primary/alternate grid，并把 active screen 的相同
   rows/columns 发送给 PTY；
5. host 从同一份 terminal state 构造下一帧 scene。

因此当前不增加通用 Part Sash、`SidebarWidth`/`PanelHeight` 状态或任意区域拖拽系统。Session
Navigation 使用固定宽度、内容决定宽度或 overlay presentation；窗口主体尺寸留给活动终端。

## 6. 分阶段演进

### 6.1 近期结构整理

- 把当前 zsh 最小 bootstrap 演进为可协商版本的 shell integration，可靠产生 command
  start/end、cwd、exit status，并覆盖更多 shell；
- 把当前单行 composer 演进为支持换行、历史、补全和建议的 Block Input Editor；
- 接入真实多会话 projection 后，再选择顶部 Tabs 或固定/可隐藏的 Session Navigation；
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
- terminal viewport、grid rows/columns 和 PTY size 必须来自同一条尺寸链路；
- Session Navigation 不拥有 Session lifecycle 或 durable output；
- Split Pane 如果出现，只属于 Terminal Workspace 内部；
- 当前实现、计划迁移和潜在能力必须在文档中保持明确分离。
