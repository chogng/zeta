# 搜索：Rust 权威执行与桌面端投影

> 本文是 workspace 内容搜索的跨进程 ownership、产品语义和演进边界的 canonical 文档。
> 跨文件内容搜索的实现细节见
> [`zeta-rs/search/README.md`](../zeta-rs/search/README.md)，App Server 的
> RPC 适配见 [`zeta-rs/app-server/README.md`](../zeta-rs/app-server/README.md)，wire DTO 与生成流程见
> [`zeta-rs/app-server-protocol/README.md`](../zeta-rs/app-server-protocol/README.md)。

## 快速理解

`zeta-search` 承担跨文件内容检索，默认作用域是当前主工作目录；App Server 只把 RPC 映射为
它的领域请求与结果；
Desktop Search contrib 只拥有查询表单、取消时机、增量结果投影和可丢弃的视图状态。当前实现使用
`workspace/search/start`、`workspace/search/read`、`workspace/search/cancel` 三个 pull RPC，
而不是把 `rg` 进程、路径授权或无界结果流放进 Renderer。

这套 RPC 是产品搜索能力，不是模型 Tool。模型是否可以调用搜索、如何审批以及如何向模型压缩
结果属于另一条 Tool/Policy contract；两者不能共享隐式权限或生命周期。

| 用户操作 | 当前行为 | 当前限制 |
| --- | --- | --- |
| 搜索主工作目录文字 | 分批返回匹配项并持续显示进度 | 最多 5,000 条结果 |
| 搜索附加目录 | 目标语义是与主目录一起搜索 | `add-dir` 尚未实现 |
| 修改查询 | 取消旧任务并启动新任务 | 旧任务结果不会混入新查询 |
| 使用正则或文件过滤 | 在 Rust 边界重新校验输入 | 只能使用工作区相对 glob |
| 点击结果打开文件 | 尚未接通编辑器 | 结果当前只用于查看 |
| 让模型调用搜索 | 走独立工具与权限契约 | 不复用界面搜索权限 |

## 所有权

| 能力 | Owner | 当前状态 |
| --- | --- | --- |
| query、大小写、正则和 include/exclude 输入 | Renderer | ✅ |
| 结果分组、高亮、状态和重新搜索取消 | Renderer | ✅ |
| IPC sender、exact shape 与输入上限的快速校验 | Electron Main | ✅ |
| workspace root 授权与 `rg` executable 冻结 | Rust / App Server composition | ✅ |
| 主工作目录与附加目录的 Search scope | `zeta-workspace-access` + App Server | 权限 authority 已接入，Search runtime 尚未消费附加根 |
| 查询校验、`rg` 进程、结果解析、分页与取消 | `zeta-search` | ✅ |
| connection ID → `SearchOwner`、DTO 转换与稳定 RPC error | App Server | ✅ |
| wire DTO、method registry、schema 与 TypeScript bindings | `zeta-app-server-protocol` | ✅ |
| 文件路径 fuzzy match | `zeta-file-search` | ✅，与内容搜索无依赖 |
| 点击结果后读取文件并打开编辑器 | Files / Editor vertical | 尚未完成 |
| 独立 workspace code index | `zeta-code-index` + App Server | ✅ 本地 lexical chunk retrieval；不作为当前 SearchView backend |
| replace、multi-root 和 watcher 驱动的产品搜索失效 | 未确定 | 尚未完成 |

## 端到端流程

```text
SearchViewPane
  → IWorkspaceSearchService.search(query, signal, onProgress)
  → trusted zeta:workspace-search:* IPC
  → AppServerClient
  → workspace/search/start
  → App Server maps DTO + connection to SearchQuery + SearchOwner
  → zeta-search::SearchService job
  → frozen RipgrepExecutable under WorkspaceRoot
  → workspace/search/read batches
  → renderer groups and highlights matches
  → workspace/search/cancel releases the job
```

`start` 冻结查询参数并返回 opaque `searchId`。App Server 把 connection ID 映射成不含传输语义的
`SearchOwner`；搜索 crate 只比较 owner，不依赖 JSON-RPC。`read` 使用 `afterMatch` cursor 读取最多 200 条；
没有新结果且作业仍在运行时可以返回空 batch。Renderer 只在结果非空时推进 cursor。
完成、取消或 Renderer 异常退出当前搜索流程时都会调用 `cancel`；完成作业也会在服务端延迟清理，
因此 cleanup RPC 失败不改变已返回结果。

## 当前语义与边界

- 查询不能为空，UTF-8 最多 16 KiB；单次搜索最多返回 5,000 条，Desktop 默认 2,000 条。
- include/exclude 各最多 64 个 workspace-relative glob，每项最多 1 KiB；绝对路径、`..`、
  前导 `!` 和 NUL 被拒绝。
- `zeta-search` 使用 host discovery 后冻结的 `rg` executable，使用 argument vector 和
  `shell: false` 等价的进程 API，不做 shell 拼接。
- `rg` 未安装时 stdio App Server 仍可启动，但 `workspaceSearch` capability 为 `false`，
  Search 调用返回 `SearchUnavailable`；Desktop 会把显式 `ZETA_RG_PATH` 透传给可信子进程。
- 搜索 cwd 固定为受信 `WorkspaceRoot`；当前沿用 ripgrep 默认 ignore/hidden 行为。
- preview 单行最多由 `--max-columns=1000 --max-columns-preview` 约束，单文件最大 16 MiB。
- match range 在 Rust 中从 ripgrep byte offset 转换为 UTF-16 offset，Renderer 可直接用于
  JavaScript 字符串切片。
- job 绑定创建它的 App Server connection。其他 connection 的 read/cancel 返回
  `SearchNotOwner`；未知或已释放 ID 返回 `SearchNotFound`。
- 同一 server 最多保留 32 个 job；超限返回 `SearchBusy`。已完成 job 最长保留约 5 分钟。
- 进程或解析失败通过 terminal `read.error` 返回稳定、已脱敏的说明；不会把任意 stderr 暴露给
  Renderer。

当前 SearchViewPane 只显示根相对路径、行号、preview 与高亮。它不尝试用现有目录枚举 API
拼出文件内容，也不伪造 editor input；在 Files 具备受约束的 read-file contract、Editor
具备稳定打开路径后，再把结果激活接到该 vertical。

## 取舍

| 方案 | 判断 | 原因 |
| --- | --- | --- |
| Renderer 直接运行 `rg` | ❌ | 绕过 sandbox、workspace authority 和跨客户端产品语义 |
| 单个同步 RPC 返回全部结果 | ❌ | 受 1 MiB JSONL frame 限制，取消与首批结果延迟也更差 |
| JSON-RPC notification 推送每条结果 | 暂不采用 | 当前 notification queue 无 backpressure，慢消费者可放大内存 |
| connection-owned pull job | ✅ | bounded batch、显式取消，并复用现有同步 JSONL transport |
| 复活旧模型 Search Tool 作为 UI backend | ❌ | 产品 UI 和模型 Tool 的调用者、权限及结果预算不同 |

## 演进

近期只在现有 contract 内完善可用性：空结果/错误呈现、查询历史和搜索中再次提交。结果点击必须
等待受信 file-content API 与 editor opening contract，不由 Search 绕过。

当前 Search runtime 只消费主工作目录；`zeta-workspace-access` 已实现并接入 Session 权限 authority，但 Search 尚未冻结它的 capability snapshot。未来 Search scope 由主工作目录和全部具备文件读取权限的附加目录组成，不把附加目录提升为独立项目，也不触发项目配置加载。无论目录来自启动参数、会话命令还是持久 `additionalDirectories`，只要其 file-access grant 有效，Search 都可以消费它；Agent Import 的一次性来源则不能自动进入 Search。

多 root Search 不能继续只返回裸 relative path。协议必须增加稳定 root identity 或 root alias，
使 `src/lib.rs` 能明确归属于主目录或某个附加目录，并避免不同 root 的同名 path 碰撞。Glob、
ignore 与 containment 也必须逐 root 计算，不能先把多个绝对目录拼成一个伪 Workspace。

当前已经存在独立的 [`zeta-code-index`](../zeta-rs/code-index/README.md)，它在 workspace side
完成 ignore-aware chunking、持久化 generation 与 FTS5 retrieval；跨系统边界见
[`code-index.md`](code-index.md)。它服务 revision-bound chunk retrieval，不替换本页的逐行文字/
正则产品搜索。是否把 SearchView 迁移到索引 backend 必须先证明 regex、glob、UTF-16 range、
connection-owned cancel 和结果完整性语义等价；当前 `searchId` 不承诺 backend 类型。

只有 transport 获得有界 backpressure 后，才考虑用 notification 替代 pull。

长期不变项是：Renderer 不获得任意进程或磁盘权限；workspace 授权在 Rust 可信边界重复校验；
结果传输有明确上限；job 不跨 connection 泄漏；产品搜索与模型 Tool 保持独立 contract。
