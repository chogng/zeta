# `zeta-workbench-controller`

`zeta-workbench-controller` 是 Workbench 逻辑模型与产品功能之间的命令边界。`WorkbenchController` 持有 `zeta_workbench_host::WorkbenchHost<PaneBinding>`，把 Session 转换成 Tab 元数据，决定 Session 的初始 Terminal Pane，并保存 Files、Diff、Agent 与 Terminal 切换所需的返回状态。

逻辑 Workbench 仍然唯一拥有 `TabInputKey` 与 `PaneContainer` 的一对一关系。Controller 把选中容器里的 Pane 连接到产品运行对象，不复制 active container 状态。

`PaneBinding` 校验 Terminal key 只能连接到匹配的 Terminal input。Terminal 进程、界面状态、渲染、产品命令和窗口事件不属于本 crate。

运行 `cargo test -p zeta-workbench-controller` 验证 Pane binding、Session 转换、默认 Pane 策略和跨功能返回状态。
