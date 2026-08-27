# `zeta-settings`

1. 根级 `lib.rs` 是 crate 入口；Settings 页面包含 General、Appearance 和 Keybindings，Remote 连接选择、连接管理与 Tunnel 管理位于 `remote.rs` 和 `remote/`。
2. Settings 功能接收只读展示快照并返回 `SettingsActivation`；Remote 功能接收连接目录和 Tunnel 生命周期事件并返回对应操作，二者都不持久化配置或启动进程。
3. 产品宿主只映射主题和工作区信息、转发平台输入并执行持久化或启动操作；配置权威、SSH/runtime、子进程、窗口生命周期和传输留在各自 crate。

验证：`just test zeta-settings`。
