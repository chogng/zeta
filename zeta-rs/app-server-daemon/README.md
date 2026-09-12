# `zeta-app-server-daemon`

`zeta-app-server-daemon` 隔离本机 App Server 进程复用，具体职责只有三项：

1. 每个 profile 从完整 Zeta 包的不可变目录启动 daemon，用可执行文件摘要判断是否需要切换进程版本，并在启动交接期间持有包租约。
2. 连接 prelude 用 `dir_root`、`dir_grant_source` 与产品服务身份选择隔离的 App Server 组合。
3. daemon 只传递 host 已选择的 grant 来源，不判断目录是否可信，也不把路径本身当成授权。

本地端点通过 [`zeta-uds`](../uds/README.md) 创建并保留私有目录句柄。客户端发送 prelude 前、服务端读取 prelude 前均检查同用户和同提权上下文。已有目录权限不合要求时直接报错，不静默修改 ACL；普通文件不会被作为残留 socket 删除。

```text
just test zeta-app-server-daemon
```

## 命令入口

- `zeta-app-server-daemon connect` 连接或启动共享 profile 服务，并代理 stdio。
- `zeta-app-server-daemon start|restart|stop|version` 管理服务并输出单行 JSON。
- `--product-services PATH` 显式选择产品服务配置；目录与 grant 来源通过宿主环境传入。
- 独立 daemon 命令始终启动当前可执行文件，支持带摘要的开发 generation。
- CLI 等嵌入宿主通过 `executable_path` 选择 daemon；`ZETA_APP_SERVER_DAEMON_PATH` 必须是绝对路径，未配置时选择宿主同目录的 daemon。
- 内部 FastRegex worker 统一由 `arg0` 分发。
