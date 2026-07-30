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
| 打开 `zeterm` | 启动默认 shell，并显示 PTY 的实时终端输出 | 最小 PTY/grid 纵切已实现 | [当前实现](#4-当前实现) |
| 输入 shell 命令 | 在 Input Editor 编辑，按 Enter 建立 Block 并写入 PTY | 已实现单行编辑与提交 | [当前实现](#4-当前实现) |
| 使用 `vim`、`top` 等交互式 TUI | alternate screen 期间隐藏 Input Editor，并把基础按键直接写入 PTY | 部分具备；scroll region 和常见 query 已接通，鼠标上报尚缺 | [当前实现](#4-当前实现) |
| 切换多个会话 | 使用顶部 Tabs 或固定/可隐藏的垂直 Session Tabs | 当前是可隐藏的 session sidebar 预览 | [分阶段演进](#6-分阶段演进) |
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

目标产品结构：

```text
zeterm
├─ TopBar
│  ├─ window drag region
│  ├─ terminal tabs / session navigation
│  └─ terminal actions
└─ TerminalWorkspace
   └─ TerminalSession
      ├─ BlockList / TerminalOutput
      └─ InputEditor
```

这个结构表达的是产品语义，不要求每个节点立即成为 public Rust type。当前 `Titlebar`、
session sidebar、transcript 和 composer 是验证绘制、命中与文本输入的 shell 骨架；迁移时应把
它们映射到 Top Bar、Session Navigation、Terminal Output 和 Input Editor，而不是在现有命名上
继续扩张通用 Workbench abstraction。

Top Bar 不是独立工作区，也不拥有终端 Session。它只提供窗口拖动、会话入口和少量全局操作。
Session Navigation 可以使用水平 Tabs 或固定/可隐藏的垂直 Tabs，但不构成能够任意改变宽度的
通用 Sidebar Part。

## 3. 所有权

| 能力 | 最终 owner | 职责边界 |
| --- | --- | --- |
| Window、Top Bar 与 Terminal Workspace 外部布局 | `zeta-native` product host | 决定窗口区域和活动会话，不进入 `zeta-ui` |
| Top Bar 内部 action 排列 | `zeta-ui::ActionBar` | 只拥有 representation geometry 和 paint，不拥有命令 |
| Session Tabs 与活动 Session presentation | Native session navigation control | 消费权威 Session projection，不复制 Session lifecycle |
| Terminal Session product state | App Server/terminal session runtime | 拥有进程、cwd、环境、输出与退出状态 |
| Terminal grid、screen/mode state、基础 escape sequence 与 BlockList | `zeta-terminal::TerminalCore` | 不由 `UiScene` 或 `InputBox` 推断 |
| PTY process、write、resize 与 exit | `zeta-native::terminal_session` + `zeta-utils-pty` | process mechanism 与 terminal model 分离 |
| selection、durable scrollback 与完整 terminal compatibility | 后续 terminal runtime | 尚未完成 |
| BlockList / TerminalOutput presentation | Native terminal session view | 呈现 runtime output；不能成为第二份权威输出存储 |
| Input Editor 编辑与 IME presentation | `zeta-ui` text input + Native host | base editing 委托 `TextInput`，focus、提交和命令路由归 host |
| Rect、icon、text scene 与 GPU draw | `zeta-ui` / `zeta-wgpu` | 不拥有 Session、PTY、窗口布局或产品 reducer |

`zeta-native` 可以保存活动 Tab、hover、focus、scroll position 等可丢弃 presentation state，但
Session、Thread、Turn、PTY process 和 durable output 必须来自对应 runtime。

## 4. 当前实现

| 当前实现 | 当前事实 | 目标映射 |
| --- | --- | --- |
| `titlebar::Titlebar` | 绘制窗口顶区、拖拽区和一个 ActionBar | 演进为 Top Bar；后续容纳 terminal tabs |
| `SidebarVisibility` 与 session rows | 可选择三个演示 session，并显示/隐藏 sidebar | 迁移为固定或可隐藏的 Session Tabs presentation |
| `ShellLayout` | 计算 titlebar、可选 sidebar、main、transcript 和 composer bounds | 收窄为窗口外层布局；Session 内部布局移交 session view |
| `TerminalCore` / `TerminalGrid` | 增量解析 ANSI，维护 cell、cursor、wrap、erase 与基础 SGR | 当前最小 terminal emulator core |
| primary/alternate screen | 解析 `47/1047/1048/1049`，切换 active grid，并在 resize 时同步两块 grid | 已实现基础 buffer lifecycle |
| DEC mode 与 terminal input | 记录 cursor key、cursor visibility、bracketed paste 和 mouse request state；编码基础 key | 鼠标 request 尚未转成 PTY report |
| scroll region 与 terminal query | 支持 margin scrolling、origin-relative cursor、line insert/delete，并把 DA/DSR/CPR reply 写回 PTY | 常见纵切已实现，尚非完整 query family |
| `BlockList` | 按提交命令建立 Block，保存有界 printable output | 当前没有 shell hook 或 prompt/echo 去重 |
| `terminal_session::TerminalSession` | 启动默认 shell，转发 PTY output/exit user event，处理 write/resize | 当前单一进程内 Session |
| composer `InputBox` | 支持单行编辑、selection、IME、caret blink 和 Enter submit | 活动 Terminal Session 的 Input Editor |
| alternate-screen Native presentation | 隐藏 composer、扩大 terminal viewport、绘制 active grid/cursor，并把 key/IME commit 写入 PTY | 基础 direct-input 纵切已实现 |
| `ActionBar` / `Button` | presentation-only action 与 icon button | 保持通用 primitive，不接收 terminal domain state |
| mouse report、完整 DEC/query family、durable scrollback | 尚未实现 | 后续 terminal compatibility 纵切 |
| terminal tabs、session restoration、split panes | 尚未实现 | 后续产品能力 |

当前 sidebar toggle 是 shell 交互验证，不代表产品长期需要一个可调整宽度的 Workbench Sidebar。
在 Session Tabs presentation 确定前，它可以继续作为固定宽度、可隐藏的导航区。

## 5. 尺寸语义

窗口 resize 的长期执行顺序是：

1. `NativeApp` 接收 physical extent 与 scale factor；
2. product layout 计算 logical Top Bar 和 Terminal Workspace；
3. 活动 Terminal Pane 根据 viewport、cell metrics 和 padding 计算 rows/columns；
4. `TerminalSession::resize` 更新 primary/alternate grid，并把 active viewport 的相同
   rows/columns 发送给 PTY；
5. host 从同一份 terminal state 构造下一帧 scene。

因此当前不增加通用 Part Sash、`SidebarWidth`/`PanelHeight` 状态或任意区域拖拽系统。Session
Navigation 使用固定宽度、内容决定宽度或 overlay presentation；窗口主体尺寸留给活动终端。

## 6. 分阶段演进

### 6.1 近期结构整理

- 把 Titlebar 的产品语义明确为 Top Bar，同时保留 native window drag integration；
- 把 sidebar session rows 收敛为 Session Navigation，而不是发展成通用 Sidebar Part；
- 把 main/transcript/composer 的命名和模块边界迁移为 Terminal Session、Terminal Output 和
  Input Editor；
- 让 root layout 只决定 Top Bar、可选 Session Navigation 和 Terminal Workspace 的外部 bounds；
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
- 不为单个 sidebar 预先在 `zeta-ui` 增加通用 Sash primitive；
- 不把 terminal grid、PTY、scrollback 或 Session 生命周期放入 renderer；
- 不把当前静态 transcript/message fixture 描述成真实 terminal output；
- 不把当前基础 ANSI subset 描述成完整 xterm/VT compatibility；
- 不因 alternate screen 和基础 direct input 已接通，就声称完整支持 `vim`、`top` 等交互式 TUI；
- 不把已解析的 mouse request mode 描述成已完成 mouse event reporting；
- 不因参考现代终端产品而复制某个外部产品的内部模型或视觉细节。

## 8. 长期不变量

- 终端会话是窗口主体，chrome 和导航服务于终端而不是与终端平级；
- product host 决定活动 Session、布局和事件路由，`zeta-ui` 只消费 presentation state；
- terminal viewport、grid rows/columns 和 PTY size 必须来自同一条尺寸链路；
- Session Navigation 不拥有 Session lifecycle 或 durable output；
- Split Pane 如果出现，只属于 Terminal Workspace 内部；
- 当前实现、计划迁移和潜在能力必须在文档中保持明确分离。
