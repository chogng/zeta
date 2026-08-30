# `zeta-workbench`

1. 管完整 Desktop Workbench：进程与窗口生命周期、应用状态、事件效果、窗口场景、Titlebar、Tab Container、Main、Inspector 和浮层顺序。
2. 私有拥有 Tab/Pane 结构、Pane binding 和布局状态，通过 `WorkbenchHost` 保证 Workbench 结构只有一个修改入口。
3. Session、Terminal、Files、SCM、Editor、Settings 自己管内容状态与内部绘制；Workbench 只决定挂载位置和组合顺序。

验证：`just test zeta-workbench`。
