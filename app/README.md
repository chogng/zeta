# `app`

> 本文只说明纯 Rust Desktop 产品宿主的源码边界、启动路径和验证入口。产品布局由 [`LAYOUT.md`](LAYOUT.md) 维护，其他产品行为与跨 crate 架构由 [`app/docs`](docs/README.md) 维护，各能力 crate 的实现契约由各自的 README 维护。

`app` 是纯 Rust Desktop 产品的 Cargo package 和发布边界。完整工作台由 `zeta-workbench` 负责。

## 职责边界

| 位置 | 负责 | 不负责 |
| --- | --- | --- |
| `app` crate | 发布用可执行入口 | Workbench、能力实现、窗口生命周期和产品状态 |
| [`zui`](zui/README.md) | 应用与窗口生命周期、输入、布局、绘制、渲染和平台能力 | 产品状态和业务交互 |
| [`zeta-ui-components`](ui-components/README.md) | 可复用 UI 组件 | 产品窗口结构和领域状态 |
| [`zeta-workbench`](workbench/README.md) | 完整产品工作台、窗口布局、能力挂载、浮层顺序和跨能力生命周期 | 各能力内部状态、绘制和平台效果 |
| 能力 crate | 编辑器、会话、工作区、远程连接、终端和 Workbench 等独立能力 | 产品级组合 |
| `zeta-rs` | App Server、协议、存储、终端语义、远程执行等共享后端能力 | `app` 的窗口和界面实现 |

依赖只能从产品宿主指向能力 crate 和共享后端。`zeta-rs`、`zui`、`zeta-ui-components`、`zeta-workbench` 及其他能力 crate 不得依赖 `app`。`app` 也不得执行或依赖 `zeta-code`、`zeta-cli` 或 `zeta-tui` 的产品入口。

`app` package 只依赖 `zeta-workbench`，没有 library target，也没有 `src` 目录。产品组合、窗口生命周期、平台事件和效果执行统一由 Workbench 的 `product` 模块负责；能力内部状态与算法仍由对应能力 crate 负责。

## 源码入口

```text
main.rs                         binary 入口，只调用 zeta_workbench::run()
workbench/product.rs            产品组合模块入口、产品状态与能力接线
workbench/product/              生命周期、事件、帧、交互和运行入口
workbench/app_server.rs         App Server 适配入口
workbench/features/             Agent、Editor、Remote、Terminal、Workspace 的产品事件与效果适配
workbench/platform/             键盘、IME 和窗口事件适配
```

`ProductApp` 是 Workbench 内唯一产品组合根，但不能吸收能力实现。跨功能协调应先确定长期负责的能力 crate，Workbench 只做必要调用。若某个改动让能力 crate 反向读取 `ProductApp` 字段，说明依赖方向已经错误。

## 启动路径

`zeta_workbench::run()` 按以下顺序工作：

1. 处理内部 App Server daemon 和 `app-server` 子命令。
2. 由 `AppInvocation::parse` 解析产品命令，由 `AppInvocation::resolve` 生成本地或远程启动配置。
3. 远程启动先由 `launch_progress::prepare_remote_launch` 完成运行时检查和准备；失败时直接返回非零退出码，不创建窗口。
4. `zui::app::Application::run` 创建 `ProductApp` 并进入事件循环。
5. `ProductApp::ready` 打开窗口，启动终端、远程语言服务和 Agent Session，然后构建首帧。
6. 初始化失败或事件循环返回运行时错误时，进程返回非零退出码。

`AppServerHost` 是产品到 App Server 的适配边界。本地和远程只是连接方式，Session、Thread、文件、Git、语言服务和终端的权威状态仍由各自能力及共享后端拥有。

## 能力 crate

| 能力 | 实现契约 |
| --- | --- |
| Editor | [`zeta-editor`](editor/README.md)、[`zeta-editor-host`](editor-host/README.md) |
| Session Pane、Composer 与 App Server Session 运行时 | [`zeta-session`](session/README.md) |
| Settings 与 Remote UI | [`zeta-settings`](settings/README.md) |
| Files Pane、目录树与文件搜索 | [`zeta-files`](files/README.md) |
| Changes Pane 与多文件 Diff | [`zeta-scm`](scm/README.md) |
| Terminal runtime 与视图状态 | [`zeta-terminal-runtime`](terminal-runtime/README.md)、[`TERMINAL.md`](TERMINAL.md)；产品 Pane 映射由 Workbench 私有拥有 |
| Workbench | [`zeta-workbench`](workbench/README.md) |
| 命令与快捷键 | [`zeta-commands`](commands/README.md)、[`zeta-keybindings-host`](keybindings/README.md)；快捷键设置页面由 [`zeta-settings`](settings/README.md) 管 |

修改能力内部行为时，先读对应 README；不要把能力状态或算法复制回产品宿主。

## 构建与验证

根 `Cargo.toml` 是唯一 Cargo workspace，`app/Cargo.toml` 定义产品 package。使用仓库脚本保证 V8 构建输入一致：

```bash
just app
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

- 改动启动参数、远程连接准备或退出语义时，同步检查 `workbench/product/run.rs`、对应 CLI 测试和 [`远程开发`](../docs/remote-development.md)。
- 改动窗口生命周期、事件或帧调度时，同步检查 `workbench/product/lifecycle.rs`、`workbench/product/frame.rs` 和 [`zui`](zui/README.md) 的宿主约束。
- 改动产品布局或交互时，同步检查 `workbench/product/presentation.rs`、`workbench/product/interaction.rs` 及对应 Workbench/能力 crate 测试。
- 改动 App Server 接线时，保持 `AppServerHost` 为窄适配层，并检查本地与远程两条连接路径。
- 改动 package、Bazel 输入或发布参数时，同步更新 [`app release graph`](docs/app-release-graph.md) 和 `//app:app_ci`。

产品布局、输入、终端兼容、渲染和 Remote 的当前行为与限制统一从 [`app/docs`](docs/README.md) 进入，本文不维护功能完成度清单。
