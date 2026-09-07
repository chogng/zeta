# CLI 架构

- `zeta-cli` 负责命令入口与产品启动，`zeta-tui` 负责终端交互；crate 用于隔离能力和依赖。
- 命令示例、输出和退出码见[CLI 规格](../spec/cli.md)，TUI 状态与绘制见[TUI 架构](tui.md)。
- Session、Thread、Turn、执行、权限和持久化由 App Server 负责。

## 产品边界

| 所属部分 | 责任 | 约束 |
| --- | --- | --- |
| `zeta-code/cli` | 参数、帮助、工作目录、profile、TTY 检测、颜色、进度、输出、退出码、shell completion、安装与集成测试 | 不复制 Agent 状态机或后端业务接口 |
| `zeta-code/tui` | 终端状态、键盘、面板、正文和生命周期 | 通过 App Server Client 发起业务请求 |
| `zeta-rs/exec` | 无交互任务运行及机器输出 | 不等同于底层工具执行器 |
| `zeta-rs` 共享后端 | Session/Thread/Turn、存储、writer lease、审批、沙盒、模型、工具及协议定义 | 不接管 TUI 展示 |
| 其他产品 | Desktop、Browser View 等 | 不由 CLI 负责 |

## 唯一产品接口

```text
zeta-cli → zeta-tui  → zeta-app-server-client
         → zeta-exec → zeta-app-server-client
                                ↓
                       zeta-app-server-protocol
```

- CLI/TUI/exec 不直接调用 Core、State、Rollout、Rollout Trace、Sandbox 或 Model Provider；CLI 也不直接依赖工具执行器。
- `zeta app-server` 委托 `zeta-server-host`；CLI 不自行组合服务器或绕过请求分发器调用内部用例。
- `zeta mcp-server` 委托 `zeta-mcp-server`；MCP 帧、Agent 轮询、交互及调用身份由该服务负责。
- 新能力先补[产品 API](../../../docs/zeta-app-server-api.md)，再实现客户端调用；不新增泛化 `runtime/service/common/platform` 层聚合内部能力。

## 连接与运行模式

| 路径 | 连接方式 | 依据 |
| --- | --- | --- |
| 当前本地 TUI | `AppServerSession::start_stdio` 启动 `app-server connect`，连接当前 profile 的本地服务 | [local_tui.rs](../../cli/src/local_tui.rs) |
| 当前无交互 ask / exec | `AppServerTarget::Embedded`，经相同类型化客户端与请求分发器运行 | [main.rs](../../cli/src/main.rs) |
| 远程 TUI | 使用远程连接路径，保持相同协议与错误语义 | [remote_connect_runtime.rs](../../cli/src/remote_connect_runtime.rs) |
| 远程调度 worker | 规划由 exec 适配 Job/Attempt/lease/cursor | [exec 远程调度](../../../docs/exec.md#9-远程调度)；不作为本地能力已交付的依据 |

- 本地服务、远程连接与进程内通道使用相同 Params、Result、Notification、Error。
- 即使省去子进程和 JSON 编解码，初始化、请求分发和生命周期约束仍成立，不能提供隐藏业务入口。
- 会话连接、请求句柄、事件流与关闭由[App Server Client](../../../docs/app-server-client.md)维护；不同路径不能被笼统描述为“默认进程内”。

## 请求、事件与审批

| 数据 | CLI/TUI 的处理方式 |
| --- | --- |
| 初始化与 Session 生命周期 | 创建、读取、列出、订阅和恢复均使用类型化方法 |
| Session 下的 Thread | 创建、分支、归档及读写遵守所属 Session 的协议 |
| Turn、Tool Call 与文件变化 | 消费正式通知，不解析日志、stderr 或人类文本猜状态 |
| 批准与回答 | 绑定目标身份，发送正式响应；不直接执行 Agent 请求的文件、命令或网络副作用 |
| 终态与错误 | 使用后端完成、失败、中断结果映射输出与退出码，不从最后一句回复推断 |

审批绑定保留 `approvalRequestId`、`threadId`、`turnId`、`toolCallId`、`actionDigest` 及 decision、scope、expiry；协议字段以[产品 API](../../../docs/zeta-app-server-api.md)为准。

## 协作与验证

| 负责方 | 交付内容 | 验证重点 |
| --- | --- | --- |
| 共享后端与 Client | 版本化协议、连接生命周期、类型化通知/错误、模拟传输和契约测试 | 相同请求跨客户端具有一致语义；初始化、取消与关闭完整 |
| exec | 单次运行、new/resume/fork、中断、终态、Human/JSONL、无交互审批 | 机器输出版本、stdout/stderr 隔离、结果不确定时的退出码 |
| CLI/TUI | 命令与参数、界面流程、输入输出样例、TTY 行为、集成测试 | pipe、重定向、非交互、Ctrl+C 与连接恢复；不依赖后端内部 crate |

- 远程 worker 的验收随其独立实现进行，不加入当前本地功能的通过结论。
- 运行验证时使用[仓库测试规范](../../../.github/instructions/testing.instructions.md)；本文整理未重新执行产品测试。

## 共享设计与 API 入口

| 问题 | 唯一参考 |
| --- | --- |
| 方法、参数、通知与错误 | [App Server API](../../../docs/zeta-app-server-api.md) |
| 启动、initialize、请求与事件通道、关闭 | [App Server Client](../../../docs/app-server-client.md) |
| 无交互执行与远程工作规划 | [exec](../../../docs/exec.md) |
| MCP 对外服务 | [MCP server](../../../docs/mcp-server.md) |
| 产品线与启动职责 | [产品边界](../../../docs/product-lines.md) |
