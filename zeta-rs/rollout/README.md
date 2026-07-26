# zeta-rollout

`zeta-rollout` 是本地权威事件历史的组合层。它把 SessionStore、ThreadStore 与 writer lease
作为同一个 durable repository 打开，并按先 Thread、后 Session 的顺序恢复 runtime。

它不定义第二种事件 framing、schema、reducer 或投影。底层 JSONL framing、checksum 与断尾恢复
仍由 `zeta-storage` 独占；领域事件与 batch 校验仍由 `zeta-session-store`、`zeta-thread-store`
拥有。

`zeta-rollout-trace` 只通过 store trait 读取此历史，不能写入或影响运行时恢复。
