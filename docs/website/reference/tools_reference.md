# 工具参考
Zeta 可使用工具的完整参考，包括权限要求和每个工具的行为。

Zeta 拥有一系列内置工具，可以帮助它理解和修改您的代码库。这些工具名称与您在 [权限规则](../configuration/permissions.md), `subagent工具列表` 和 `hook匹配器` 中使用的字符串完全相同。要完全禁用某个工具，请将其名称添加到 [权限设置](../configuration/permissions.md) 中的 `deny` 数组。

要控制Zeta可以使用哪些工具以及何时先请求，请在设置、hooks或子代理的工具列表中配置权限规则。请参阅“为每个接受工具名称的位置配置具有权限规则和hooks的工具”。

要添加自定义工具，请连接一个 [MCP server](../build_with_zeta/mcp/mcp_reference.md)。要使用要使用可重用的基于提示的工作流扩展 Zeta，请编写一个 [skill](../build_with_zeta/skills/skills.md)，它通过现有的 `Skill` 工具运行，而不是添加新的工具条目。

```text
`Permission required` 列显示该工具在默认权限模式下是否对工作目录内的路径进行提示。标记为“否”的文件访问工具，包括`Read`、`Grep`和`Glob`，仍然会对`工作目录和其他目录`之外的路径进行提示。`Bash` 标记为“是”，但运行一组内置的`只读命令`而无需提示。
```