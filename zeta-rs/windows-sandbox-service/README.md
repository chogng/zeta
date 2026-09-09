# `zeta-windows-sandbox-service`

> 本 README 拥有 Zeta Windows 沙箱机器级配置服务的实现契约；跨平台策略与完整调用链见
> [`docs/sandboxing.md`](../../docs/sandboxing.md)。

1. 服务使用固定的 `ZetaSandboxService` 身份和 `\\.\pipe\Zeta.Sandbox` 管道，只接受与受保护安装副本摘要一致的 `zeta-command-runner.exe`，并再次核对进程用户与管道用户。
2. 服务采用请求用户身份验证固定本地磁盘上的目录和程序、拒绝 reparse point，并在配置完成前持续持有目录和文件 handle；ACL 工作进入最多一个进程、512 MiB、120 秒上限且关闭即清理的 worker Job Object。
3. 本 crate 只拥有 Windows Service、认证 IPC 和机器级配置入口；共享策略与命令启动仍由 `zeta-sandboxing`、`zeta-windows-sandbox` 和进程执行器拥有，服务不接收或运行 Agent 命令。
