# `zeta-workbench`

1. 管完整 Desktop Workbench：进程与窗口生命周期、产品状态、事件效果、窗口场景、Titlebar、Tab Container、Main、Inspector 和浮层顺序。
2. 通过 `WorkbenchHost` 组合 `zeta-workbench-model` 的 Tab/Pane 结构、Pane binding、布局状态、App Server 适配和跨能力生命周期。
3. Session、Terminal、Files、SCM、Editor、Settings 自己管内容状态与内部绘制；Workbench 只决定挂载位置和组合顺序。

验证：`just test zeta-workbench`。
