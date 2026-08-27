# 三端快捷键系统

> 文档所有权：本文是 Zeta、App 与 Zeta Code 共享快捷键语义、端侧输入边界和演进顺序的 canonical 架构文档。
> 实现细节分别由 [`zeta-keybinding`](../zeta-rs/keybinding/README.md)、[`zeta-keybindings-host`](../app/keybindings/README.md)、[`zeta-code` TUI](../zeta-code/tui/README.md) 和 [Zeta 浏览器基础](../zeta-ts/docs/browser-foundation.md)维护。
> 状态：共享 Rust 核心与用户资源编译器、Zeta Code `AppKeymap`/`/keymap` 设置界面、App、Zeta TypeScript 输入链路和跨语言 conformance 向量均为 Current。

## 快速理解

三端共享“按键序列如何匹配命令”的规则，但各自在本地把平台事件转换成标准按键；原始按键不发送给 App Server，也不要求三个产品共享同一份命令表。

| 用户场景 | Zeta | App | Zeta Code |
| --- | --- | --- | --- |
| 输入普通文字 | 浏览器、IME 与编辑器处理 | 窗口输入法或终端面板处理 | 终端与 Crossterm 处理 |
| 触发快捷键 | TypeScript Resolver 根据焦点和 ContextKey 解析 | Rust Resolver 根据 Native context 解析 | 精简 Rust Keymap 根据 TUI 焦点解析 |
| 系统键盘布局变化 | 重新加载系统布局和 Mapper | `winit` 提供标准化逻辑键与物理键 | 不检测；终端已经完成布局转换 |
| 修改快捷键 | profile `keybindings.json` | profile `keybindings.json` 和设置浮层 | `/keymap` 录制并原子保存到 profile `zeta-code/keybindings.json`；也可手工编辑并自动热重载 |
| 执行命令 | Renderer 内执行命令或产生编辑意图 | Native host 执行 `AppCommandId` | TUI 主循环执行 `AppCommand` 或局部 component intent |

后续章节依次说明[一次按键的流程](#2-端到端流程)、[所有权](#3-所有权与依赖方向)、[一致性边界](#5-跨语言一致性)和[当前状态与演进](#8-当前状态与演进)。

## 1. 决策

快捷键系统固定区分三个概念：

- **键盘布局（keyboard layout）**：把物理位置、系统布局和修饰键转换成逻辑键，只由拥有原始窗口事件的宿主处理。
- **快捷键绑定（keybinding）**：一个按键序列、条件、来源、优先级和目标命令组成的规则。
- **键位表（keymap）**：某个产品注册的完整快捷键集合及其当前上下文，不跨产品共享命令身份。

`zeta-keybinding` 是没有 UI、I/O、平台事件和产品命令依赖的 Rust 语义核心。App 与 Zeta Code 直接依赖它；Zeta 的 Renderer 保留同步 TypeScript 实现，并通过共享规范和测试向量保持行为一致。

不把 Rust Resolver 放入 App Server，也不让 Zeta Renderer 经 IPC、N-API 或 WASM 解析每个按键。`preventDefault()`、焦点、IME 和 Chord 状态都要求端侧同步决定。

## 2. 端到端流程

```mermaid
flowchart LR
    OS[操作系统或终端输入] --> ZA[Zeta DOM / native-keymap adapter]
    OS --> ZTA[App winit adapter]
    OS --> ZCA[Zeta Code Crossterm adapter]
    ZA --> ZTS[TypeScript Keybinding Resolver]
    ZTA --> RK[Rust zeta-keybinding]
    ZCA --> RK
    ZTS --> ZM[Zeta Command / 编辑意图]
    RK --> ZTM[App Command]
    RK --> ZCM[Zeta Code TUI intent]
    ZM --> EFFECT[端侧副作用或类型化后台请求]
    ZTM --> EFFECT
    ZCM --> EFFECT
```

一次按键按以下顺序处理：

1. 宿主接收 DOM、`winit` 或 `crossterm::KeyEvent`。
2. 宿主适配器生成逻辑键、可用时的物理键和精确修饰键；输入法组合文字不伪装成快捷键。
3. 当前产品提供焦点、模式和可用性上下文，Resolver 选择命令、阻止规则、等待后续 Chord 或不匹配。
4. 产品 Keymap 把匹配结果转换成自己的命令或局部意图。
5. 只有命令产生的类型化业务请求、编辑操作或终端字节可以进入后台；原始按键和用户输入内容不进入 App Server 日志。

## 3. 所有权与依赖方向

| 组件 | 拥有 | 明确不拥有 |
| --- | --- | --- |
| `zeta-keybinding` | `KeyStroke`、Chord、Parser、Context expression、来源、优先级、冲突和 Resolver | `winit`、Crossterm、DOM、UI、定时器、文件、产品命令 |
| `zeta-keybinding::user` | 严格 JSON shape、平台覆盖、命令/条件回调编译和重复规则诊断 | profile 路径、文件轮询、产品命令 catalog、设置 UI |
| Zeta Keyboard Layout | ScanCode、KeyCode、AltGr、死键、系统布局、浏览器映射 | Rust Keymap、后台命令执行 |
| Zeta Keybinding Service | ContextKey、Chord lifecycle、浏览器事件阻止和命令执行 | 系统布局采集、App Server 状态 |
| `zeta-keybindings-host` | `winit` 适配、Native Chord timeout、用户资源校验和热更新 | App UI、产品命令副作用 |
| App Keymap | `AppCommandId`、内建规则和 Native context | 通用 Parser、设置组件绘制 |
| App 快捷键 UI | 录制生命周期、设置浮层、诊断展示 | Resolver、用户文件 authority、命令执行 |
| Zeta Code `AppKeymap` | Crossterm 适配、应用级 action、Chord lifecycle 和局部 context | component 编辑/导航、键盘布局、物理 ScanCode、App Server authority |
| App Server | 快捷键触发后的类型化产品请求 | 原始按键、焦点、IME、Chord 和快捷键配置 |

依赖方向固定为：

```text
App product ──→ zeta-keybindings-host ──→ zeta-keybinding
Zeta Code TUI ─────────────────────────────→ zeta-keybinding
Zeta Renderer ──→ TypeScript keybinding implementation

all clients ── semantic command only ──→ App Server
```

如果 `zeta-keybinding` 开始依赖 `zui`、`zeta-ui-components`、`winit`、Crossterm、profile 路径或产品命令，说明共享边界已经漂移。若 Zeta Renderer 需要 IPC 才能决定是否阻止浏览器按键，也说明执行边界已经漂移。

## 4. 共享语义与产品差异

三端共享以下语义：

- 一至四段有序 Chord；
- 逻辑键和可选物理键身份；
- Control、Shift、Alt、Meta 和 portable `primary`；
- Builtin/Workbench/User 来源、显式优先级和后注册覆盖；
- 条件过滤、阻止规则和 Chord prefix；
- `NoMatch`、`PendingChord`、`Command` 和 `Blocked` 四类解析结果。

三端不共享以下产品事实：

- 命令 ID、默认 Keymap 和命令参数；
- 焦点树、ContextKey catalog 和局部输入传播；
- Chord timeout、IME、窗口失焦和事件阻止副作用；
- 设置 UI、快捷键提示样式和诊断展示；
- 用户配置的产品命令 catalog、资源路径和设置 UI。

Zeta Code 把根级运行时结构称为 `AppKeymap`，不称为 `GlobalKeymap`。用户规则没有 `when` 时表示“在本产品所有上下文中适用”，这是规则作用域，不是另一个运行时对象。Composer 的字符编辑、Selection 的方向键和 Transcript 的滚动继续由各 component 拥有。单键先交给当前 component，只有未消费事件进入应用级 fallback；多段 Chord prefix 则先经过应用级 matcher，避免首段被文本组件吞掉。应用级内建声明同时生成 Resolver 注册和 `/help` 项，附加 Shift、Alt、Meta 或 Hyper 不会匹配只声明 Control 的组合。

`AppKeymap` 已拥有一至四段 Chord 的 pending sequence、1 秒超时、上下文变化取消、Esc 取消、错误后续键透传和 footer 提示。当前内建应用级绑定仍都是单段；以后增加多段声明不再需要建立第二套状态机。`Esc Esc` rewind 保留为根界面的专用交互：普通 Esc 在 Chord pending 时只负责取消，不同时推进 rewind。

## 5. 跨语言一致性

Rust crate 与 TypeScript 实现共享 JSON conformance fixtures，而不是共享运行时二进制。当前 fixture 固定共同子集中的：

- 有效和无效字符串，以及 Unicode 空白分隔的规范化；
- canonical 序列化；
- portable modifier 别名的 canonical 归一化；
- Chord 数量上限；
- 规则来源、显式优先级和注册顺序；
- condition filter、blocker、Chord prefix 与 no-match 结果。

Rust 与 TypeScript 都显式按“来源、同来源内优先级、注册顺序”比较；任何数值优先级都不能跨越来源层级。共同 fixture 要求 User 先于 Workbench、Workbench 先于 Builtin，再在同一来源内比较 priority，最后由后注册规则打破平局。

展示标签、浏览器 ScanCode 白名单和单修饰键 keyup 行为属于宿主能力，不进入强制跨语言相等断言。Rust 物理键只保存宿主已经标准化的非空标识，浏览器端额外使用 ScanCode catalog 校验；这是 adapter 能力差异，不改变共享 Resolver 语义。新增共享语法必须同时修改 Rust、TypeScript、fixtures 和本文；产品专属扩展必须在对应实现文档中标出，不得悄悄改变共享 grammar。

## 6. 配置与持久化

用户自定义快捷键是三端的正式产品能力，不再是 Potential。它仍然是端侧 profile 资源，不进入 App Server Config，不随 Workspace 内容自动执行，也不把原始按键发送到后台。

### 6.1 资源位置

| 产品 | Current 资源 | 原因 |
| --- | --- | --- |
| Zeta | `<profile>/keybindings.json` | Workbench command catalog 与 Renderer settings UI 的 authority |
| App | `<profile>/keybindings.json` | 当前复用 Workbench 风格稳定 command ID，并提供 Native 设置浮层 |
| Zeta Code | `<profile>/zeta-code/keybindings.json` | TUI 只有较小的本地 command catalog；产品作用域避免 Desktop-only command 使整份 TUI 资源失效 |

`ZETA_PROFILE_ROOT` 仍是显式 profile authority；没有设置时使用共享的本地 Zeta profile root。Zeta Code 连接 Remote App Server 时也读取本机 profile，因为终端按键、焦点和用户偏好属于发起连接的客户端，不属于远端 Workspace。

长期目标不是强行把三端命令塞进一个文件，而是共享语法、优先级和可移植 command ID。只有确认 Zeta 与 App 的 command catalog 完全兼容后，才继续共享根级资源；新增产品专属命令必须放入产品作用域资源，或先设计带明确产品 scope 的兼容格式。

### 6.2 JSON 契约

资源是最多 1024 项的严格 JSON 数组。当前 Rust 客户端接受：

```json
[
  {
    "key": "primary+k primary+c",
    "command": "zetaCode.action.copyLastResponse",
    "when": "inputFocus && !selectionVisible",
    "mac": "cmd+k cmd+c",
    "linux": "ctrl+k ctrl+c",
    "win": "ctrl+k ctrl+c"
  },
  {
    "key": "ctrl+o",
    "command": null
  }
]
```

- `key` 必填，使用共享的一至四段 portable 语法。
- `command` 必填；字符串绑定本端稳定命令，`null` 以 User 来源阻止同一按键的默认规则。
- `when` 可选；省略表示本产品所有上下文，不能称为 `GlobalKeymap`。
- `mac`、`linux`、`win` 可选；`null` 表示该平台不注册此项。
- User 高于 Workbench/Builtin；同来源比较显式 priority，最后由后声明规则获胜。
- Rust TUI/Native 当前不支持命令 `args`；Zeta Renderer 的 `args` 是明确的宿主扩展。

Zeta Code 当前可配置 command ID：

| Command ID | 行为 |
| --- | --- |
| `zetaCode.action.cycleApprovalMode` | 切换下一次提交的权限模式 |
| `zetaCode.action.openRewind` | 直接打开 Rewind Pane；不模拟 `Esc Esc` |
| `zetaCode.action.attachClipboardImage` | 从本机剪贴板附加图片 |
| `zetaCode.action.interruptOrQuit` | 工作时中断，空闲时退出 |
| `zetaCode.action.copyLastResponse` | 复制最近一条 Agent response |
| `zetaCode.action.suspend` | Unix suspend/resume 流程 |

Zeta Code 当前 ContextKey 为 `inputFocus`、`composerEmpty`、`selectionVisible` 和 `keyEventPress`。未知 command、字段或 ContextKey 会拒绝整个新快照，避免用户以为规则已生效。

### 6.3 生命周期与故障处理

CLI 把 active profile root 显式交给 TUI；TUI 启动时加载产品资源，并在单写者 event loop 的 Tick 上以一秒间隔检测外部修改。编译发生在临时规则集：完整 JSON、command、condition 和 TUI Chord 安全约束全部通过后，才以一次替换安装 User rules 并取消旧 pending Chord。坏更新保留上一份有效规则并产生可见诊断；删除文件恢复纯内建规则。

共享 `zeta-keybinding` 只编译 bytes，不读文件、不知道 profile 路径；文件大小限制、轮询、诊断呈现和写入由产品 host 拥有。

`/keymap` 只新增、替换或清除目标 command 的字符串规则；“替换自定义项”不会删除 Builtin 默认键，也不会改写 `command: null` blocker。需要禁用默认键、添加 `when` 或设置平台覆盖时继续直接编辑 JSON。录制结果写入 portable `key`，因此适用于所有平台；单键和两段 Chord 可在 Pane 中录制，三至四段 Chord 继续使用 JSON。

### 6.4 设置界面与后续边界

| 阶段 | 状态 | 退出条件 |
| --- | --- | --- |
| 严格资源 schema、User 覆盖/blocker、平台覆盖、`when`、Chord 与热重载 | Current | Zeta Code、App 和共享 core 测试持续覆盖原子替换 |
| Zeta Code 可搜索的 Keyboard Shortcuts Pane | Current | `/keymap` 展示 command、默认/用户键位、来源、诊断和资源路径；只消费 `AppKeymap` snapshot |
| Zeta Code 录制与原子保存 | Current | 单键/两段 Chord 录制只在临时 Pane 中截获输入；revision 过期时拒绝保存，完整编译成功后才原子替换文件和运行时规则 |
| Profile 切换 | Planned boundary | host 先切换 active profile，再给端侧资源 owner 一个新 generation；旧 watcher 不得覆盖新 profile |
| Settings Sync/导入导出 | Deferred | 先定义 profile 同步 authority、冲突格式和隐私边界 |
| Workspace 级键位 | Not accepted | 默认不信任仓库控制的按键重映射；如需支持必须先定义 Workspace Trust 与显式启用 |
| OS `systemWide` 热键 | Not accepted | 只可能由有窗口/native shortcut authority 的 Zeta/App 实现；TUI 不支持 |

## 7. 可靠性、隐私和兼容性

- 快捷键解析必须同步且在端侧完成，不能等待网络或后台进程。
- Troubleshooting 默认关闭；开启时只能记录按键元数据、匹配阶段和命令 ID，不能记录 composer 文本、密码或剪贴板内容。
- 终端通常只能提供逻辑键和修饰键；Zeta Code 不伪造不存在的 ScanCode，也不承诺区分终端无法区分的组合。
- AltGr、死键和 IME 由拥有原始图形窗口事件的 Zeta/App adapter 处理；TUI 消费终端已经解释后的结果。
- Ctrl-C、Ctrl-D、Ctrl-Z 等终端保留语义由 Zeta Code 产品 Keymap 明确注册，不能由共享 core 隐式决定。

## 8. 当前状态与演进

| 阶段 | 状态 | 退出条件 |
| --- | --- | --- |
| Zeta TypeScript layout、Mapper、Resolver 和 profile resource | Current | 浏览器与 Electron 测试持续覆盖布局和快捷键 |
| App Rust Resolver、Native adapter、用户资源与设置 UI | Current | 迁移共享 core 后行为和资源测试不变 |
| 提升无 UI 的 `zeta-keybinding` 到 `zeta-rs/keybinding` | Current | crate 不含产品/UI/platform 依赖，App 继续通过测试 |
| Zeta Code 应用级固定 Keymap 与 Chord lifecycle | Current | 现有 Ctrl/BackTab/Esc 行为由共享 Resolver 驱动；pending/超时/取消/提示完整，局部 component key 不上移 |
| TS/Rust parser conformance fixtures | Current | 两个实现读取同一 fixture 并通过 |
| TS/Rust Resolver precedence fixtures | Current | Builtin/Workbench/User 来源、极值优先级、后注册覆盖、condition、blocker 和 prefix 读取同一 fixture |
| Zeta Code 用户可配置 Keymap | Current | profile 资源、User precedence/blocker、`when`、平台覆盖、Chord、热重载和坏更新恢复持续通过测试 |
| Zeta Code Keyboard Shortcuts Pane 与录制保存 | Current | 可搜索、来源/诊断可见；原子保存直接安装同一份已校验规则，不建立第二套 Resolver |

迁移已经按一个 source of truth 原则完成首个纵切：纯 core 已移动，App UI 已拆出，Zeta Code 根级 Keymap 已接入，旧 `app/keybinding` 模块已删除。adapter 只能单向转换，不保留两套 Resolver。

共享 core 可用 `bazel test //zeta-rs/keybinding:keybinding-unit-tests` 在三端平台验证。App host 与 UI 另有 `//app/keybindings:keybindings-unit-tests` 和 `//app/keybinding-ui:keybinding-ui-unit-tests` Bazel 目标。Windows Bazel 通过仓库拥有的 `rules_rs` 兼容补丁使用 gnullvm-hosted Rust tools，使 `rustc`、过程宏 DLL 和 hermetic LLVM/MinGW linker 使用同一 ABI；三个目标在 Windows、Linux 与 macOS 都实际运行，不再使用平台跳过。

## 9. 长期不变量

- 原始键盘事件不离开当前客户端。
- App Server 不解析快捷键，也不拥有客户端焦点和 Keymap。
- Rust 共享 core 没有 UI、I/O、平台事件和产品命令依赖。
- 三端共享语法和冲突语义，但各自拥有命令 catalog 与默认 Keymap。
- Zeta Renderer 的按键决定保持同步，不因跨语言复用引入 IPC/WASM 热路径。
- 局部文本编辑和导航留在拥有状态的 component，不为形式统一全部提升到全局命令总线。
