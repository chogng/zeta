# `zeta-agent-environment`

> 本 README 拥有 Agent 环境快照和模型环境文本的实现契约；目录访问语义见
> [`docs/environment-access.md`](../../docs/environment-access.md)。

- 保存宿主 `cwd`、平台、Shell、日期和仓库摘要等不可变环境事实。
- 保存并确定性排序当前调用明确获权的 `Dirs`；`cwd` 不会自动进入该集合。
- 渲染经过 XML 转义的 `<environment_context>`；本 crate 不读取文件、不执行命令，也不决定权限。
