┌──────────────────────────────┐
│         Workspace 源码        │
└──────────────┬───────────────┘
               │
               │ 本地
               ▼
┌──────────────────────────────┐
│ 1. Scan / Ignore / Limits    │
│    扫描文件、忽略规则、容量限制  │
└──────────────┬───────────────┘
               │
               ▼
┌──────────────────────────────┐
│ 2. Chunk / Revision          │
│    本地切块                   │
│    path / range / revision   │
│    hash / chunk identity     │
└──────────────┬───────────────┘
               │
               │ canonical source/chunks
               │
      ┌────────┼───────────────┐
      │        │               │
      ▼        ▼               ▼

┌───────────┐ ┌────────────┐ ┌──────────────────────┐
│ Lexical   │ │ Symbol     │ │ Semantic             │
│ Index     │ │ Index      │ │ Index（可选）         │
│           │ │            │ │                      │
│ SQLite    │ │ syntax     │ │ 分两种部署方式：      │
│ + FTS5    │ │ declaration│ │                      │
│           │ │ + fuzzy    │ │ A. Local Semantic    │
└─────┬─────┘ └──────┬─────┘ │ B. Cloud Semantic    │
      │               │       └──────────┬───────────┘
      │               │                  │
      │               │           ┌──────┴──────┐
      │               │           │             │
      │               │           ▼             ▼
      │               │    Local Semantic   Cloud Semantic
      │               │
      │               │    chunks           verified chunks
      │               │      │                   │
      │               │      ▼                   ▼
      │               │   embedding          上传 provider
      │               │      │                   │
      │               │      ▼                   ▼
      │               │   local vectors      cloud embedding
      │               │      │                   │
      │               │      ▼                   ▼
      │               │   local vector       cloud vector
      │               │   index              index
      │               │
      │               │
      │               │
═══════════════════════╪══════════════════════════════════════
                       │
                 以上是“构建阶段”
                 以下是“查询阶段”
═══════════════════════╪══════════════════════════════════════
                       │
                       ▼

              Agent / search_code
                       │
                       ▼
              retrieve(user query)
                       │
         ┌─────────────┼─────────────┐
         │             │             │
         ▼             ▼             ▼

   Lexical Query   Symbol Query   Semantic Query
         │             │             │
         │             │      ┌──────┴──────┐
         │             │      │             │
         │             │      ▼             ▼
         │             │  Local Semantic  Cloud Semantic
         │             │     Query           Query
         │             │
         ▼             ▼             ▼
 lexical candidates symbol candidates semantic candidates
         │             │             │
         └─────────────┼─────────────┘
                       ▼
              ┌─────────────────┐
              │ Retrieval Fusion│
              │ RRF / dedupe    │
              │ ranking         │
              └────────┬────────┘
                       │
                       ▼
              ┌─────────────────┐
              │ 本地重新验证源码   │
              │ path            │
              │ revision        │
              │ range           │
              │ hash            │
              └────────┬────────┘
                       │
                       ▼
              ┌─────────────────┐
              │ Content Budget  │
              │ 数量/字节限制     │
              └────────┬────────┘
                       │
                       ▼
                Final Code Hits
                       │
                       ▼
                   Agent Context




真正决定效果的是 embedding 模型、reranker、ANN/召回算法、索引规模和代码新鲜度