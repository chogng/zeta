# `zeta-settings`

1. 根级 `lib.rs` 是 crate 入口；页面只包含 General、Appearance 和 Keybindings，快捷键页面、录制状态及保存 action 位于 `keybindings.rs` 与 `keybindings/`。
2. Feature 通过 `SettingsFeatureSnapshot` 提供只读展示快照，设置点击只返回 `SettingsActivation`；语言服务能力与配置不在 Settings 中实现。
3. 产品宿主只映射主题和工作区信息、转发平台输入并执行快捷键持久化 action；配置权威、窗口生命周期和传输留在各自 crate。

验证：`cargo test -p zeta-settings --lib`。
