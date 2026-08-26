# `zeta-remote-host`

`zeta-remote-host` 是本机 Remote host 的生命周期协调层。它位于
[`zeta-remote-connections`](../remote-connections/README.md) 的 SSH/Tunnel primitive 之上，
为产品宿主提供可复用的 Tunnel 启动、就绪检测、取消、进程退出恢复和类型化事件。

## Ownership

本 crate 负责活跃 Tunnel 的运行时生命周期：

- 为一个 Remote target 创建和持有多个逻辑 Tunnel；
- 监听 OpenSSH 子进程退出；
- 在固定的恢复窗口内以退避策略重建 Tunnel；
- 保持恢复时的本地 loopback 端口；
- 通过 `RemoteTunnelEvent` 向宿主发布状态。

本 crate 不负责：

- `RemoteConnectionCatalog` 或产品配置文件的读取；
- credentials、UI、窗口和命令注册；
- `NativeEvent`、`AppProxy`、`ElementId` 或任何具体产品类型；
- 远端 runtime 的安装策略。

产品应先解析自己的 profile/catalog，再将 `SshHost` 和 SSH executable 交给
`RemoteTunnelHost`。产品宿主负责把 `RemoteTunnelEvent` 映射到自己的事件循环和 UI state。

## Layering

```text
product host (zeterm / Desktop)
  -> zeta-remote-host        # lifecycle, cancellation, recovery, events
  -> zeta-remote-connections # OpenSSH and Tunnel process primitives
  -> zeta-remote              # target identity and validation
```

`RemoteTunnelHost` 是本机 host-side supervisor，不是远端服务端；远端运行时仍由
`zeta-remote-server` 负责。

## Verification

```bash
cargo test -p zeta-remote-host
```
