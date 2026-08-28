# zeta-utils-cache

- `BlockingLruCache` 提供有容量上限的进程内 LRU；同步方法只在 Tokio 多线程运行时内读写共享缓存，运行时外不保留数据。
- `sha1_digest` 为内容缓存键生成固定长度摘要；它只负责键计算，不负责持久化、过期策略或业务缓存所有权。
- 实现和独立测试位于 `src/lib.rs`、`src/cache_tests.rs`；修改锁定、淘汰或摘要语义后运行 `just test zeta-utils-cache`。
