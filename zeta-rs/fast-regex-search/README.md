# `zeta-fast-regex-search`

> Agent `grep` 的执行选择与配置由 [`zeta-rs/app-server/README.md`](../app-server/README.md) 维护；编辑器工作区搜索的独立契约见 [`docs/search.md`](../../docs/search.md)。

1. `FastRegexSearch` 使用[固定且版本化的 ASCII 字符对频率表](data/README.md)，以稀有边界构造稀疏 n-gram；查询从正则前缀、后缀和分支提取必需文字，生成覆盖 n-gram 并交叉 posting list，随后重读候选文件执行完整正则验证。哈希冲突只会增加候选，不能直接产生命中；算法对应 [Cursor 的 Fast Regex Search](https://cursor.com/blog/fast-regex-search)。
2. `workspace_files.rs` 扫描工作区并以文件大小、修改时间和文件系统变更时间定位离线变化；只重新读取变化文件并写入 delta，变更层达到数量或比例门槛后合并为新基线。`storage.rs` 通过 [`zeta-immutable-generation-store`](../immutable-generation-store/README.md) 发布不可变的 documents、lookup、postings 基线与 delta 快照，索引 header 绑定排名表摘要；`disk_index.rs` 映射排序 lookup，并按 offset 从磁盘读取、校验和交叉需要的 `u32` posting。
3. 本 crate 只服务 Agent `grep`，不实现编辑器 Search、路径模糊匹配、模型 Tool 注册或代码召回。不足 3 字节或无法提取必需文字的正则会精确扫描所有索引文件；基准必须先验证与 `rg --line-number` 的命中文件和匹配行数一致，再要求稀有、无命中及全量扫描用例都快于 `rg`。修改后运行 `just test zeta-fast-regex-search` 和 `cargo bench -p zeta-fast-regex-search --bench fast_regex_vs_rg -- --require-faster`，并验证 `zeta-app-server` 的 Agent grep 测试。
