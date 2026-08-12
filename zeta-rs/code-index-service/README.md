# `zeta-code-index-service`

> 本 README 拥有云端语义 CodeIndex 的 provider-neutral 服务契约。Workspace 切块与外发授权分别见
> [`zeta-code-index`](../code-index/README.md) 和
> [`zeta-code-index-cloud`](../code-index-cloud/README.md)；跨来源融合见
> [`zeta-code-retrieval`](../code-retrieval/README.md)。

## 快速理解

`zeta-code-index-service` 接收 Workspace 已切好并复核的 `MaterializedChunk`，调用 embedding 模型，
原子替换一个远端 generation 的向量集合；查询时生成 query embedding、做向量召回、可选调用
rerank，并由本服务解释分数、排序和截断，最后只返回 exact `ChunkReference`。

| 能力 | Owner | 本 crate 是否拥有 |
| --- | --- | --- |
| 扫描、ignore、读取、切块、revision/chunk identity | Workspace `zeta-code-index` | ❌ |
| embedding/rerank provider API 适配与执行 | `zeta-model-provider` | 委托 |
| 模型输入准备、向量召回、rerank 时机、排序/过滤/截断 | `zeta-code-index-service` | ✅ |
| local/cloud RRF 与 Agent context budget | `zeta-code-retrieval` | ❌ |
| credential、网络、租户、grant 与删除控制状态 | concrete cloud host + `zeta-code-index-cloud` | ❌ |

## 关键接口与调用关系

| Symbol | 可见性 | 职责 | 漂移信号 |
| --- | --- | --- | --- |
| `CodeIndexSemanticService` | public | publication embedding、vector recall、rerank 与 final order | 读取 Workspace 路径或接受完整文件后切块 |
| `CodeIndexVectorStore` | public trait | exact-generation replace/search/idempotent collection delete | 合并不同 generation 或改变 chunk identity |
| `InMemoryCodeIndexVectorStore` | public | 确定性 reference 实现和本地测试基础 | 被描述为 production 持久化方案 |
| `EmbeddingInvoker` / `RerankInvoker` | `zeta-model-provider` public traits | 有序模型 API 调用 | 决定候选、排序、过滤或截断 |

```text
Workspace CodeIndex
└─ MaterializedChunk[]
   └─ CodeIndexSemanticService::publish
      ├─ prepare path/language/content model inputs
      ├─ EmbeddingInvoker::embed
      └─ CodeIndexVectorStore::replace_generation

CodeIndexSemanticService::query
├─ EmbeddingInvoker::embed(query)
├─ CodeIndexVectorStore::search(exact generation, 4 × requested limit)
├─ optional RerankInvoker::rerank(query, recalled documents)
├─ service-owned score interpretation + deterministic final order
└─ exact ChunkReference[]
```

## 失败语义与当前限制

- embedding/rerank 数量、维度或 finite 校验失败时拒绝 publication/query，不部分接受；
- publication 混入多个 Workspace root 或重复 `ChunkReference` 时拒绝，不能跨 authority 合并或让云端生成第二套 identity；
- vector store 必须按 collection 原子替换 generation，查询旧 generation 显式失败；
- 当前只提供内存 cosine store 和抽象模型 invoker，没有 production vector database、网络 endpoint、
  concrete embedding/rerank provider codec、batch retry、cancellation 或持久化 deletion receipt；
- 当前 rerank 使用 vector recall 的前 `4 × limit`（最多 400）作为候选，并按模型分数稳定排序。

## 验证

```bash
cargo test -p zeta-code-index-service
cargo clippy -p zeta-code-index-service --all-targets --no-deps
bazel test //zeta-rs/code-index-service:code-index-service-unit-tests
```

测试覆盖 Workspace chunk identity 保留、向量顺序、service-owned rerank 排序、重复 identity 与模型
cardinality fail-closed。扩展 production store 或网络 adapter 时，不得给本 crate 增加 filesystem、scan、
ignore 或 chunker 依赖。
