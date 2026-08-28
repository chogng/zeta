# `zeta-fast-regex-search`

> Agent `grep` 的执行选择与配置由 [`zeta-rs/app-server/README.md`](../app-server/README.md) 维护；编辑器工作区搜索的独立契约见 [`docs/search.md`](../../docs/search.md)。

1. `FastRegexSearch` 为单个 `WorkspaceRoot` 训练字符对频率，以稀有边界构造稀疏 n-gram；查询从正则前缀、后缀和分支提取必需文字，生成覆盖 n-gram 并交叉 posting list，随后重读候选文件执行完整正则验证。哈希冲突只会增加候选，不能直接产生命中；算法对应 [Cursor 的 Fast Regex Search](https://cursor.com/blog/fast-regex-search)，但字符对权重按当前仓库训练。
2. `index.rs` 拥有扫描、ignore、查询、当前文件复核和未保存内容覆盖层；posting 只保存 `u32` 文件 ID。`storage.rs` 把完整 generation 保存为 documents/weights、顺序 postings 和排序 lookup 基线，lookup 作为只读字节表二分查询；watcher 更新保存为 `delta.bin` 变更层。`complete.bin` 最后发布，重启时必须校验版本、generation、完整路径集合和内容摘要；不完整或损坏的 generation 不参与搜索。
3. 本 crate 只服务 Agent `grep`，不实现编辑器 Search、路径模糊匹配、模型 Tool 注册或代码召回。不足 3 字节或无法提取必需文字的正则会精确扫描所有索引文件；基准必须先验证与 `rg --line-number` 的命中文件和匹配行数一致，再要求稀有、无命中及全量扫描用例都快于 `rg`。修改后运行 `just test zeta-fast-regex-search` 和 `cargo bench -p zeta-fast-regex-search --bench fast_regex_vs_rg -- --require-faster`，并验证 `zeta-app-server` 的 Agent grep 测试。
