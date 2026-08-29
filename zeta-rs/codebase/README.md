## 快速理解

- 读取已授权的 `WorkspaceRoot`，执行 ignore-aware 扫描、切分、revision/identity 计算，并维护磁盘与编辑器未保存内容的统一当前源码视图。
- 定义源码、符号和向量的存储接口，完成候选召回、融合、去重、当前源码复核和结果 byte budget；SQLite 实现由 `zeta-codebase-store` 提供。
- 通过 `CodebaseEnhancement` 接收可选增强候选；不拥有 Workspace trust、profile 路径、RPC、凭据或网络连接。

## 依赖方向

`zeta-codebase-store` 依赖本 crate 和 `zeta-state`。`zeta-cloud-codebase` 依赖本 crate；本 crate 不依赖 SQLite、Cloud Codebase、App Server protocol 或产品 UI。

## 主要接口

| 接口 | 保证 |
| --- | --- |
| `Codebase::open/rebuild/refresh_observed_paths` | 只在规范化 root 内重新读取事实，以完整 transaction 发布 generation |
| `Codebase::search/materialize/materialize_verified_excerpt` | 返回前校验当前 revision、range、key 与 content hash |
| overlay synchronize/close | 未保存内容完全替代同路径磁盘内容，保存后按 content hash 交回磁盘 generation |
| `SymbolIndex` | 只消费 Codebase 已复核源码，不自行扫描 Workspace |
| `CodebaseSemanticService` | 以 `EmbeddingIndexKey` 绑定向量数据，只复用身份完全匹配的 embedding |
| `CodebaseRetrievalService` | 组合本地候选和一个可选 `CodebaseEnhancement`，再统一复核与限额 |

## 数据库

`zeta-codebase-store` 通过 `zeta-state` 为每个 Workspace 获取一份 Codebase lease，并打开一个 `codebase.sqlite3`。源码、符号、向量和 embedding cache 使用各自表与 schema version，共享数据库生命周期。

embedding cache 主键包含 root、`EmbeddingIndexKey`、path、language、chunk key 与 content hash。rerank model 不进入该 key，因为它不产生持久向量。

## 验证

```bash
just test zeta-codebase
just test zeta-codebase-store
just check zeta-codebase --all-targets
```

测试覆盖持久化重开、增量刷新、ignore/容量限制、未保存内容、全文与符号查询、向量复用、模型 key 变化、多来源融合、云端增强失败以及当前源码复核。
