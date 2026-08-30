# 内容搜索

> 本文拥有跨文件内容搜索的产品边界。实现分别见
> [`zeta-content-search`](../zeta-rs/content-search/README.md) 与
> [`zeta-app-server`](../zeta-rs/app-server/README.md)。

## 结论

内容搜索针对一个明确的 `Dir` 执行。目录由 `DirId` 定位，调用入口必须具备
`SearchFiles`；`cwd`、窗口中打开的项目和 Session 都不会自动扩大搜索范围。

```text
Search UI
  → IContentSearchService
  → zeta:content-search:*
  → content/search/*
  → DirId + Authorization<SearchFiles>
  → zeta-content-search
```

桌面端可以把多个窗口文件夹聚合成一次用户操作，但它必须逐个目录发起搜索并保留目录身份。
核心搜索服务不创建“主目录”“附加目录”或 Workspace 身份。

## 所有权

| 内容 | 所有者 |
| --- | --- |
| 查询表单、结果分组、高亮和取消时机 | Renderer |
| IPC 参数形状和输入上限 | Electron Main |
| 目录选择、`SearchFiles` 检查和连接级任务路由 | App Server |
| `rg` 执行、解析、分页和取消 | `zeta-content-search` |
| 文件名模糊查找 | `zeta-file-search` |
| Agent 的 `grep` 工具 | Agent Tool 与 Policy；不复用产品搜索任务 |

## 协议

协议提供三个有界 pull RPC：

- `content/search/start` 冻结目录、查询和上限，返回 `searchId`。
- `content/search/read` 使用游标读取下一批匹配项。
- `content/search/cancel` 终止并释放任务。

IPC 通道使用 `zeta:content-search:*`。公开接口使用完整的 `ContentSearch*`，因为它跨越
Renderer、Electron Main 和 App Server；搜索模块内部的私有函数只使用 `start`、`read`、`cancel`
等无歧义短词。

## 边界

- query 最大 16 KiB；include/exclude 各最多 64 项；单项最大 1 KiB。
- glob 必须相对所选目录，绝对路径、`..`、前导 `!` 和 NUL 会被拒绝。
- 单次任务最多返回 5,000 条结果，读取批次最多 200 条。
- 任务绑定创建它的 App Server connection，其他连接不能读取或取消。
- `rg` 通过参数数组启动，不经过 shell；stderr 经过稳定错误映射后才返回前端。
- 结果 range 在 Rust 中转换为 UTF-16 offset，前端不重新解释 byte offset。

## 与其他能力的关系

内容搜索不等于 Codebase 检索。前者要求逐行、正则和 glob 语义；后者返回经过当前源码复核和
byte budget 的代码证据。Agent `grep` 也使用独立的工具授权和结果预算。

Session 目录可以供 Agent 工具使用，但不会悄悄进入产品搜索面板。窗口切换、`cwd` 变化或新增
Session 目录时，调用方必须重新明确选择要搜索的 `DirId`。

## 不变量

- 搜索范围由 `DirId` 与 `Authorization<SearchFiles>` 决定，不由裸路径或 `cwd` 推断。
- 多目录搜索是上层聚合，不改变每个结果所属的目录。
- Renderer 不获得任意进程或磁盘访问能力。
- 产品搜索与 Agent Tool 保持独立权限、任务和结果契约。
