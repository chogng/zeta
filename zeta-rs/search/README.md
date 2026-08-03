# `zeta-search`

> 本 README 拥有跨文件内容搜索的实现契约：查询校验、冻结 `rg` 执行、结果解析、分页作业和
> owner 隔离。产品语义与跨进程所有权由 [`docs/search.md`](../../docs/search.md) canonical
> 维护；App Server 的 RPC DTO 转换见
> [`zeta-rs/app-server/README.md`](../app-server/README.md)。

`zeta-search` 在 host 提供的已授权搜索作用域内检索文件内容；当前默认且唯一的作用域是主工作
目录。它支持文字、正则、大小写策略和文件 glob。
它不搜索路径名称：`zeta-file-search` 继续独占文件路径的遍历、ignore 处理和 fuzzy matching。

## 快速理解

调用方传入由可信 host 建立的 `WorkspaceRoot`、冻结的 `RipgrepExecutable`、`SearchOwner` 和
`SearchQuery`。`SearchService` 启动受限后台作业，调用方随后按游标读取 `SearchPage`，或
取消作业。crate 不知道 JSON-RPC、Desktop、模型 Tool 或 connection 的含义。

| 需求 | `zeta-search` | `zeta-file-search` |
| --- | --- | --- |
| 找到包含文字或正则的文件行 | `SearchService` | 不负责 |
| 文件名 / 路径 fuzzy match | 不负责 | `PathSearchHandle` |
| 读取候选文件内容 | 通过冻结 `rg` | 不读取内容 |
| 结果形状 | path、行号、preview、UTF-16 ranges | path、score、匹配字符 indices |
| UI、RPC、模型 Tool | 不负责 | 不负责 |

## 所有权与边界

当前 crate 负责：

- 文字/正则查询和 include/exclude glob 的输入上限与 containment 校验；
- 无 shell 的冻结 `rg` 进程启动、取消、JSON 输出解析和稳定错误收束；
- UTF-8 byte offset 到 UTF-16 preview range 的转换；
- 最多 32 个 owner-bound job、分页读取、5 分钟完成作业保留和全量取消。

当前 crate 不负责：

- 工作区信任决策、`rg` discovery 或任意 executable 选择；host 在构造
  `SearchService` 前完成这些工作；
- 文件路径 fuzzy 搜索、目录树、Editor buffer overlay 或 Renderer 的结果分组；
- JSON-RPC DTO、connection ID、搜索表单、模型 Tool 和批准策略。

如果这个 crate 开始依赖 `zeta-app-server-protocol`、`zeta-file-search`、Desktop 或模型 Tool，说明
搜索执行边界已经漂移。协议适配属于 App Server；路径 fuzzy 属于 `zeta-file-search`。

## 文件与职责

```text
src/
├── lib.rs                # 只导出稳定的领域 API
├── types.rs              # query、page、owner、match 和 error contract
├── service.rs            # SearchService、rg lifecycle、validation 与 parsing
└── service_tests.rs      # argv、glob、UTF-16 range 和结果上限测试
```

`service.rs` 只持有 `WorkspaceRoot`、冻结 `RipgrepExecutable` 和 job state。`types.rs` 不引用
App Server protocol，因此 CLI、TUI 或未来其他 host 可以自行将输入映射为 `SearchQuery`。

## 公共契约

| Symbol | 调用方做什么 | crate 保证什么 |
| --- | --- | --- |
| `SearchService::new` | 传入可信 root 与已冻结 `rg` | 不自行从 `PATH` discovery executable |
| `switch_workspace` | 在 host 切换 root 后调用 | 仅影响之后启动的作业；已有作业保留开始时的 root |
| `start` | 传入 `SearchOwner` 和 `SearchQuery` | 校验输入、分配 opaque ID、启动有界作业 |
| `read` | 传入同一 owner、ID 和 cursor | 返回最多 200 条、不泄漏其他 owner 的作业 |
| `cancel` | 传入同一 owner 和 ID | 停止并释放一个作业 |
| `cancel_all` | workspace 退休、撤销信任或服务关闭时调用 | 取消并释放所有尚存作业 |

`SearchOwner` 是 crate 不解释的 opaque `u64`。App Server 当前以 connection ID 建立它，但该映射
不属于本 crate 的持久化或协议契约。`SearchPage.error` 只包含稳定、已脱敏的运行/解析说明；原始
stderr 不会返回给调用方。

## 执行路径与失败语义

```text
trusted host root + frozen rg
  → SearchService::start(owner, query)
  → validate_query / validate_glob
  → typed rg argv, shell-free spawn
  → parse JSON matches into SearchMatch
  → owner-bound SearchPage reads
  → cancel / retention cleanup / cancel_all
```

- 无效查询、glob、游标或 batch size：返回 `SearchError::InvalidInput`；
- job 不存在或已被释放：返回 `NotFound`；不同 owner：返回 `NotOwner`；服务容量或 poisoned state：
  返回 `Busy`；
- `rg` 无法启动、输出无法读取或 JSON 不合法：作业标记完成，并在 `SearchPage.error` 返回稳定说明；
- 达到结果上限时，只有确实观察到额外匹配才设置 `limit_hit`；
- 取消会终止子进程；已完成 job 最多保留五分钟，供调用方读取最后 batch。

## 验证与修改影响

```bash
cargo test --manifest-path Cargo.toml -p zeta-search
cargo test --manifest-path Cargo.toml -p zeta-app-server workspace_search --lib
bazel test //zeta-rs/search:search-unit-tests
```

修改 argv、输入校验、range 转换、分页、取消或 owner 规则时，必须同步检查 `service_tests.rs`、
App Server `search_operations.rs` 的 DTO 映射、[`docs/search.md`](../../docs/search.md) 和该 README。

## 当前限制与潜在演进

当前实现每次查询启动冻结的 `rg`，只搜索磁盘内容；未保存 Editor buffer、replace、multi-root、
`add-dir` runtime、持久化索引和 watcher 驱动失效均尚未实现。未来本 crate 只消费 host 从
`zeta-add-dir::DirectoryAccessScope` 冻结出的搜索作用域：主工作目录默认进入，所有具备
file-read grant 的附加目录都可以进入，但它们不会因此成为项目配置根。Agent Import 的一次性
来源不能自行扩大搜索范围。

引入多 root 前必须先扩展领域结果：`SearchMatch` 当前只有 relative `path`，不足以区分不同 root
中的同名文件。目标 contract 需要 root-qualified match identity，并按 root 独立执行 glob、
ignore、containment 和错误隔离；不能把多个 absolute path flatten 成一个伪 Workspace。

潜在方向是在本 crate 内部加入索引实现，但只有先定义 ignore、一致性、watcher、持久化和隐私
语义后才可以进行。当前不预先引入 `Engine` 或 `Backend` trait；出现第二种真实执行实现时，再以
实际调用方需要的最小接口抽象。
