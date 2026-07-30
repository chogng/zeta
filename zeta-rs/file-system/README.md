# zeta-file-system

> 本 README 是 workspace filesystem primitive 的 canonical 实现文档。跨进程 ownership、
> Desktop 投影与阶段状态见
> [`../../docs/zeta-desktop-architecture.md`](../../docs/zeta-desktop-architecture.md)。

本 crate 拥有 workspace-scoped、consumer-neutral 的 filesystem contract。App Server 与
model tool adapter 都依赖它；本 crate 不依赖 JSON-RPC、Desktop、Tool schema 或 UI 状态。

## 文件与公共契约

| 文件 / symbol | 职责 |
| --- | --- |
| `service.rs` / `WorkspaceFileSystem` | consumer 共享 trait；实现必须约束所有输入到 authority root |
| `local.rs` / `LocalFileSystem` | 当前 local implementation；组合 `zeta_sandboxing::WorkspaceRoot` 与宿主 I/O |
| `types.rs` | `FileType`、`FileMetadata`、`DirectoryEntry`，不携带 wire/UI DTO |
| `error.rs` / `FileSystemError` | path、文件/目录类型、读写上限、只读状态与 I/O failure |

`WorkspaceFileSystem::{read_file,write_file,get_metadata,read_directory}` 都接收相对路径。
`read_file` 的 limit 单位是 bytes，不做 UTF-8 解码；内容解释属于调用方。
`write_file` 同样使用 byte limit，在现有父目录内原子替换或新建普通文件，并保留被替换文件
的权限；它不隐式创建目录。
`read_directory` 只返回直接子项，并按 lossily represented child name 排序。

## 执行与可信边界

```text
App Server 或 Tool adapter
→ WorkspaceFileSystem method
→ LocalFileSystem::resolve_existing / resolve_for_write
→ WorkspaceRoot containment
→ canonicalize + root containment
→ std::fs I/O
```

`LocalFileSystem::resolve_existing` 是关键 private boundary。它必须在 I/O 前拒绝 absolute
path、parent traversal、missing path 和解析后越过 root 的 symlink。调用方在 IPC 或 Tool
schema 层做的格式校验只用于快速失败，不能替代这里的 authority check。

| Private symbol | 当前职责 | 架构漂移信号 |
| --- | --- | --- |
| `LocalFileSystem::resolve_existing` | 解析并约束读取目标 | 调用方绕过它直接拼接 workspace path |
| `LocalFileSystem::resolve_for_write` | 约束现有或待创建文件及最近现有父目录 | mutation 在 authority check 前执行 I/O |
| `local::atomic_write` | 同目录临时写、权限复制、flush、replace 与父目录 sync | 开始决定 RPC、dirty state 或冲突 UI |
| `local::metadata` | host metadata → consumer-neutral `FileMetadata` | 返回 wire DTO 或 Renderer identity |

`read_file` 最多读取 `maximum_bytes + 1` 来可靠检测 overflow；limit 为零也失败。
`write_file` 在打开临时文件前检查完整 content byte length，向目标同目录写入并 `sync_all`，
再原子替换目标及同步父目录。现有普通文件的权限会复制到临时文件；只读文件、目录、缺失父
目录和越过 root 的 symlink 都会失败。
`get_metadata` 返回 millisecond Unix timestamp（宿主不提供或早于 epoch 时为 `None`）。
错误可能包含宿主 I/O 诊断；App Server 必须在 external boundary 映射为稳定错误，不得直接
回显。

## 接入、测试与修改影响

Local host 通过 `WorkspaceRoot::open` 构造 `LocalFileSystem`，再以
`Arc<dyn WorkspaceFileSystem>` 注入 consumer。remote implementation 可以实现同一 trait，
但必须提供等价的 root confinement；trait 本身无法从类型系统证明这一义务。

```text
cargo test -p zeta-file-system
bazel test //zeta-rs/file-system:file-system-unit-tests
```

修改类型或 trait 时同步检查 `zeta-file-system-tool`、`zeta-app-server` 与
`zeta-app-server-protocol`。把 Tool schema、RPC DTO 或 Renderer URI 塞入本 crate，或绕过
`WorkspaceRoot` 直接拼接不可信路径，都表示 ownership 漂移。

## 当前限制与扩展点

- Current：只有 in-process local implementation；具备有界原子文件写入，不包含 rename/delete。
- Current：watcher 是独立的 invalidation primitive，不进入 filesystem trait；App Server 负责
  组合二者并发布产品 notification。
- Current：non-UTF-8 directory entry name 使用 lossy conversion。
- Extension point：remote backend、mutation、watcher 需要先定义独立 failure、ordering 与
  invalidation contract，不能通过扩大当前 read method 的隐含语义实现。
