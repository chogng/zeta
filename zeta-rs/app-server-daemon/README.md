# `zeta-app-server-daemon`

`zeta-app-server-daemon` 隔离本机 App Server 进程复用，具体职责只有三项：

1. 每个 profile 用内容哈希固定 daemon 可执行文件版本，再创建控制端点并维护其生命周期。
2. 连接 prelude 用 `dir_root`、`dir_grant_source` 与产品服务身份选择隔离的 App Server 组合。
3. daemon 只传递 host 已选择的 grant 来源，不判断目录是否可信，也不把路径本身当成授权。

```text
just test zeta-app-server-daemon
```
