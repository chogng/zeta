# `zeta-settings`

1. 管 Settings 页的导航、页面布局、交互身份和活动 section，并提供 Workbench 内嵌与独立页面两种挂载模式。
2. 管设置项自己的草稿状态和视图契约；语言服务设置已在这里保存选择、输入、脏状态和保存结果。
3. 产品宿主只提供配置快照、主题样式并执行读写 action；窗口事件和配置传输不进入本 crate。

验证：`cargo test -p zeta-settings --lib`。
