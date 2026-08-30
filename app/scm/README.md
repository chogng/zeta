# `zeta-scm`

1. 拥有 Changes Pane 的变更文件状态、多文件 Diff、分支 `Picker`、折叠、滚动、布局、交互身份和 UI。
2. 消费调用方提供的仓库快照和 `zeta-editor` Diff 文档；Git 进程、仓库查询和修改仍由 `zeta-git` 负责。
3. 不读取工作区、不调用 Git、不拥有 Workbench 导航；验证命令为 `just test zeta-scm`。
