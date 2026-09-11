# `zeta-app-server-daemon`

`zeta-app-server-daemon` 隔离本机 App Server 进程复用，具体职责只有三项：

1. 每个 profile 从完整 Zeta 包的不可变目录启动 daemon，用可执行文件摘要判断是否需要切换进程版本，并在启动交接期间持有包租约。
2. 连接 prelude 用 `dir_root`、`dir_grant_source` 与产品服务身份选择隔离的 App Server 组合。
3. daemon 只传递 host 已选择的 grant 来源，不判断目录是否可信，也不把路径本身当成授权。

本地端点通过 [`zeta-uds`](../uds/README.md) 创建并保留私有目录句柄。客户端发送 prelude 前、服务端读取 prelude 前均检查同用户和同提权上下文。已有目录权限不合要求时直接报错，不静默修改 ACL；普通文件不会被作为残留 socket 删除。

```text
just test zeta-app-server-daemon
```
