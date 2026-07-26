# zeta-rollout-trace

`zeta-rollout-trace` 从 SessionStore 与 ThreadStore 读取一个不可变、可序列化的 Session
历史切片。它保留各 aggregate 的原始 durable sequence；不会把它们伪造成一条全局 sequence。

该 crate 只用于诊断、导出、评测和离线分析。它不写 rollout、不做 reducer、不发布更新，不能成为
任何运行时决策的权威来源。Trace 可能包含用户输入、工具参数和工具结果，因此 crate 不提供默认
文件写入；任何持久化、共享或上传都必须在调用方显式施加脱敏、访问控制和保留期策略。
