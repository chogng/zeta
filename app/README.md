# `app`

> 本文只说明纯 Rust Desktop 产品宿主的源码边界、启动路径和验证入口。产品行为与跨 crate 架构由 [`app/docs`](docs/README.md) 维护，各能力 crate 的实现契约由各自的 README 维护。

`app` 是纯 Rust Desktop 产品的 Cargo package 和发布边界。它负责组装窗口、产品状态、事件、App Server 连接和最终界面；可复用能力必须留在对应 crate 中。

## 当前 UI 布局

当前 Workbench 由 `Titlebar`、可选的 `TabContainer` 和当前顶层页签拥有的 `PaneContainer` 组成。

```text
Workbench
├─ Titlebar
├─ TabPart
│  └─ TabGroup
│     ├─ TabInput::Session(session_id)
│     └─ TabInput::Settings
└─ active TabInput → PaneContainer
   └─ PanePart
      └─ split tree
         ├─ PaneGroup → active PaneInput
         └─ Split
            ├─ PaneGroup → active PaneInput
            └─ PaneGroup → active PaneInput
```

顶层 `TabInput` 当前只有 Session 和 Settings 两类；每个 `TabInput` 一对一拥有一个 `PaneContainer`，切换顶层页签会整体切换其中的拆分布局。`PanePart` 保存拆分树和活动 `PaneGroup`；每个 `PaneGroup` 是一个可见矩形区域，可以保存多个 `PaneInput`，但同一时刻只显示其中一个。

| 当前 `PaneInput` | 身份 | 内容 |
| --- | --- | --- |
| `Agent` | Session + Thread | 只表示 Zeta Agent 的对话、时间线和 Composer |
| `Terminal` | Terminal session | 外部 AI CLI、shell 和其他交互式进程 |
| `Files` | 工作区根目录 | 文件树和文件搜索 |
| `Diff` | 工作区根目录 | 当前 Changes 入口和多文件差异内容 |
| `Settings` | 全局单例 | 设置页面 |

目标模型会把 Changes、普通文件和具体 Diff 分成明确的 `PaneInput`，并按当前 `PanePart` 的 PaneGroup 和活动输入绘制。Agent 只表示 Zeta；外部 AI CLI 由 Terminal Pane 承载。完整布局见 [`LAYOUT.md`](LAYOUT.md)，终端边界见 [`TERMINAL.md`](TERMINAL.md)。

## 职责边界

| 位置 | 负责 | 不负责 |
| --- | --- | --- |
| `app` crate | 进程入口、启动参数、窗口生命周期、产品事件含义、能力接线、界面组装 | 通用 UI 框架、可复用组件、后端领域状态机 |
| [`zui`](zui/README.md) | 应用与窗口生命周期、输入、布局、绘制、渲染和平台能力 | 产品状态和业务交互 |
| [`zeta-ui-components`](ui-components/README.md) | 可复用 UI 组件 | 产品窗口结构和领域状态 |
| [`zeta-workbench-ui`](workbench-ui/README.md) | Workbench 标题栏、Tab 导航、交互标识和可丢弃界面状态 | Workbench 逻辑模型、Session 生命周期和产品命令执行 |
| 能力 crate | 编辑器、会话、工作区、远程连接、终端和 Workbench 等独立能力 | 产品级组合 |
| `zeta-rs` | App Server、协议、存储、终端语义、远程执行等共享后端能力 | `app` 的窗口和界面实现 |

依赖只能从产品宿主指向能力 crate 和共享后端。`zeta-rs`、`zui`、`zeta-ui-components`、`zeta-workbench-ui` 及其他能力 crate 不得依赖 `app`。`app` 也不得执行或依赖 `zeta-code`、`zeta-cli` 或 `zeta-tui` 的产品入口。

`app/src` 是现有产品组合层，不得继续加入能力实现。新增状态、后台任务、布局算法或交互行为必须由对应能力 crate 单独拥有，`app` 只保留必要接线；修改现有文件时优先迁移、删除或缩小职责。

## 源码入口

```text
src/bin/app.rs          binary 入口，只调用 app::run()
src/lib.rs              模块注册并导出 run()
src/app.rs              产品组合模块入口
src/app/
  native_app.rs         NativeApp 及产品模块接线
  run.rs                参数解析、启动准备和进程退出状态
  state.rs              NativeApp 的产品状态
  lifecycle.rs          zui::app::App 生命周期与窗口事件
  frame.rs              frame 调度与提交
  interaction.rs        产品交互分发
  presentation.rs       产品界面重建
  runtime.rs            后台事件处理
  workbench*.rs         Workbench 产品接线与调整尺寸
src/app_server.rs       App Server 适配入口
src/features/           Agent、Editor、Remote、Settings、Terminal、Workspace 产品适配
src/platform/           键盘、IME 和窗口事件适配
src/presentation/       Shell 界面、交互标识和主题适配
src/session/            Session 搜索、菜单和画布接线
```

`NativeApp` 是唯一产品组合根，但不能继续吸收能力实现。跨功能协调应先确定长期负责的能力 crate，产品宿主只做必要调用。若某个改动让能力 crate 反向读取 `NativeApp` 字段，说明依赖方向已经错误。

## 启动路径

`app::run()` 按以下顺序工作：

1. 处理内部 App Server daemon 和 `app-server` 子命令。
2. 由 `AppInvocation::parse` 解析产品命令，由 `AppInvocation::resolve` 生成本地或远程启动配置。
3. 远程启动先由 `launch_progress::prepare_remote_launch` 完成运行时检查和准备；失败时直接返回非零退出码，不创建窗口。
4. `zui::app::Application::run` 创建 `NativeApp` 并进入事件循环。
5. `NativeApp::ready` 打开窗口，启动终端、远程语言服务和 Agent Session，然后构建首帧。
6. 初始化失败或事件循环返回运行时错误时，进程返回非零退出码。

`AppServerHost` 是产品到 App Server 的适配边界。本地和远程只是连接方式，Session、Thread、文件、Git、语言服务和终端的权威状态仍由各自能力及共享后端拥有。

## 能力 crate

| 能力 | 实现契约 |
| --- | --- |
| Agent Session | [`zeta-agent-session`](agent-session/README.md) |
| Composer | [`zeta-composer`](composer/README.md) |
| Editor | [`zeta-editor`](editor/README.md)、[`zeta-editor-host`](editor-host/README.md) |
| Session UI | [`zeta-session-ui`](session-ui/README.md) |
| Workspace UI | [`zeta-workspace-ui`](workspace-ui/README.md) |
| Remote UI | [`zeta-remote-ui`](remote-ui/README.md) |
| Terminal runtime 与 Pane 绑定 | [`zeta-terminal-workspace`](terminal-workspace/README.md)、[`TERMINAL.md`](TERMINAL.md) |
| Workbench | [`zeta-workbench`](workbench/README.md)、[`zeta-workbench-layout`](workbench-layout/README.md)、[`zeta-workbench-host`](workbench-host/README.md)、[`zeta-workbench-controller`](workbench-controller/README.md) |
| Workbench UI | [`zeta-workbench-ui`](workbench-ui/README.md) |
| 命令与快捷键 | [`zeta-commands`](commands/README.md)、[`zeta-keybindings-host`](keybindings/README.md)、[`app-keybinding-ui`](keybinding-ui/README.md) |

修改能力内部行为时，先读对应 README；不要把能力状态或算法复制回产品宿主。

## 构建与验证

根 `Cargo.toml` 是唯一 Cargo workspace，`app/Cargo.toml` 定义产品 package。使用仓库脚本保证 V8 构建输入一致：

```bash
just app
just app-dev
just app-check
just app-test
```

只验证当前 package 时可直接执行：

```bash
python3 -B build/cargo_with_v8.py check -p app --all-targets
python3 -B build/cargo_with_v8.py test -p app --all-targets
```

Bazel 的产品入口是 `//app:app`，发布边界检查是：

```bash
bazel test //app:app_ci
```

打包、签名、远程运行时目录和发布顺序由 [`app release graph`](docs/app-release-graph.md) 维护，不在本文重复。

## 修改检查

- 改动启动参数、远程连接准备或退出语义时，同步检查 `src/app/run.rs`、对应 CLI 测试和 [`远程开发`](../docs/remote-development.md)。
- 改动窗口生命周期、事件或帧调度时，同步检查 `src/app/lifecycle.rs`、`src/app/frame.rs` 和 [`zui`](zui/README.md) 的宿主约束。
- 改动产品布局或交互时，同步检查 `src/app/presentation.rs`、`src/app/interaction.rs`、`src/presentation/` 及对应 Workbench/UI crate 测试。
- 改动 App Server 接线时，保持 `AppServerHost` 为窄适配层，并检查本地与远程两条连接路径。
- 改动 package、Bazel 输入或发布参数时，同步更新 [`app release graph`](docs/app-release-graph.md) 和 `//app:app_ci`。

产品布局、输入、终端兼容、渲染和 Remote 的当前行为与限制统一从 [`app/docs`](docs/README.md) 进入，本文不维护功能完成度清单。
