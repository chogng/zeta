# `zeta-terminal-workspace`

1. 管终端 runtime 的预留、异步就绪、Session/Pane key 绑定、激活、后台保留和释放。
2. 管每个 Workbench `PaneInput` 的完整终端视图状态，包括滚动、滚动条、指针、选择及其基础绘制。
3. 通过 `PaneBinding` 连接 Workbench，不依赖产品组合根；调用方只提供 runtime 创建、尺寸更新和平台事件转发。

验证：`cargo test -p zeta-terminal-workspace --lib`。
