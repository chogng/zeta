# `zeta-app-server-daemon`

`zeta-app-server-daemon` 是本地 App Server 的 profile-scoped authority process。它为同一
profile 的产品连接维护一个共享 `LocalProfileRuntime`，按 Workspace、trust source 与 product
services identity 复用 `AppServer` 实例，并通过本地 Unix-domain socket 接受 JSONL 连接。

## 运行时边界

| 组件 | 职责 |
| --- | --- |
| `zeta-app-server-daemon` binary | 绑定 profile endpoint、管理连接线程、空闲退出 |
| `zeta_app_server_daemon::connect` | 选举或拉起 daemon，并把调用方 stdio 代理到本地 socket |
| `zeta-server app-server connect` | 解析产品环境，选择 daemon binary，调用连接 API |
| `zeta-app-server` | JSON-RPC dispatch、domain composition 与 Workspace runtime |

daemon 进程只从 `ZETA_PROFILE_ROOT` 获取 profile identity。Workspace root、trust source 与
product-services manifest 都由每个连接的 bounded prelude 携带，不能由首个连接固定整个 daemon
的产品或 Workspace 语义。

端点 identity 由 canonical profile root、daemon endpoint contract version、crate version 与 App
Server schema hash 派生；独立 contract version 防止新 daemon 误连仍存活的旧 server-host broker。Unix
平台把 endpoint 放在当前用户私有的 `/tmp` runtime directory；Windows 放在 profile 的 `run`
目录。启动锁、socket 与日志都由本 crate 管理。

## 直接运行

通常不直接启动 daemon；`zeta-server app-server connect` 会按需拉起它。诊断时可运行：

```text
ZETA_PROFILE_ROOT=/absolute/profile/path zeta-app-server-daemon
```

进程在没有连接且没有活动 terminal 时按空闲超时退出。测试可通过
`ZETA_LOCAL_APP_SERVER_IDLE_TIMEOUT_MILLIS` 缩短超时。
