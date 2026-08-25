# `zeta-app-server-daemon`

`zeta-app-server-daemon` 是本地 App Server 的 profile-scoped authority process 与生命周期
owner。它为同一 profile 的产品连接维护一个共享 `LocalProfileRuntime`，按 Workspace、trust
source 与 product services identity 复用 `AppServer` 实例，并通过本地 Unix-domain socket 接受
JSONL 数据连接和有界控制连接。

## 运行时边界

| 组件 | 职责 |
| --- | --- |
| `zeta-app-server-daemon` binary | 绑定 profile endpoint、发布进程记录、管理连接线程、响应停止请求和空闲退出 |
| `zeta_app_server_daemon::{connect,run_lifecycle}` | 串行化生命周期操作，完成真实 initialize/schema probe，并把调用方 stdio 代理到本地 socket |
| `zeta-server app-server connect` | 解析产品环境，选择 daemon binary，按需启动并连接 ready daemon |
| `zeta-server app-server daemon ...` | 输出单行机器可读 JSON 的显式 start/restart/stop/version 控制面 |
| `zeta-app-server` | JSON-RPC dispatch、domain composition 与 Workspace runtime |

daemon 进程只从 `ZETA_PROFILE_ROOT` 获取 profile identity。Workspace root、trust source 与
product-services manifest 都由每个连接的 bounded prelude 携带，不能由首个连接固定整个 daemon
的产品或 Workspace 语义。

端点 identity 由 canonical profile root、daemon endpoint contract version、crate version 与 App
Server schema hash 派生；独立 contract version 防止新 daemon 误连仍存活的旧 server-host broker。Unix
平台把 endpoint 放在当前用户私有的 `/tmp` runtime directory；Windows 放在 profile 的 `run`
目录。operation lock、socket、PID/process-generation record 与 bounded log 都由本 crate 管理。

## 生命周期命令

```text
zeta-server app-server daemon start
zeta-server app-server daemon restart
zeta-server app-server daemon stop
zeta-server app-server daemon version
```

每个成功命令只向 stdout 写一个 JSON object，包含 status、PID、instance ID、daemon version、
endpoint、log path，以及运行时的 App Server name/schema hash。`start` 幂等；`version` 不启动
daemon；`restart` 先完成受管理进程的协作式停止，再发布新 generation。

启动成功不以“socket 已存在”为准。client 使用与产品连接相同的 Workspace/trust/product-services
prelude 发起真实 `initialize`，并精确校验 `zeta-app-server` identity 与 schema hash 后才返回 ready。
控制响应还必须与私有 runtime directory 中的 PID/instance record 一致，避免把未知 endpoint 当成
受管理进程。

进程记录同时保存实际 daemon executable 的 canonical path、文件元数据和 Unix file identity；`start` 发现当前受管理进程来自旧构建或另一 executable 时，会先协作停止旧 generation，再从调用方当前选择的 executable 启动并重新执行 initialize/schema probe。产品 executable 也可通过内部 daemon-process argument 承载同一 authority loop，供开发构建避免复用未被 Cargo 重建的旁路 binary。

`stop` 先通过控制 socket 请求 daemon 停止接收新数据连接，并给现有连接和 Terminal 一个有界
grace window；超过窗口后 daemon 自身关闭剩余连接、移除 endpoint/process record 并结束专用
进程，避免 Workspace runtime 的后台线程把 lifecycle 无限拖住。Unix 上若 daemon 连这一控制路径
都未完成，会在再次核对进程启动 identity 后由 client 强制终止；Windows 不会根据可能重用的 PID
盲目终止进程。SIGINT/SIGTERM 在 Unix 上进入同一协作停止路径。

本 crate 不下载或更新 runtime，也不实现 Codex Remote Control。Zeta 的远程 runtime 仍由
`zeta-remote-connections` 的认证 catalog、内容寻址 installer、activation 与 rollback contract
管理，daemon 不得绕过那条信任链自更新。

## 直接运行

通常不直接启动 daemon binary；`zeta-server app-server connect` 或显式 lifecycle command 会拉起
它。诊断 server loop 本身时可运行：

```text
ZETA_PROFILE_ROOT=/absolute/profile/path zeta-app-server-daemon
```

进程在没有连接且没有活动 terminal 时按空闲超时退出。测试可通过
`ZETA_LOCAL_APP_SERVER_IDLE_TIMEOUT_MILLIS` 缩短超时。
