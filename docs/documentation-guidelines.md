# Zeta 文档分层与写作规范

> 状态：Current repository convention。
> 适用范围：crate README、`docs/*.md` 架构/设计文档及两者之间的引用关系。

## 1. 目标

同一个主题通常有两类读者：

- 正在修改一个 crate 的实现维护者；
- 正在理解或演进跨 crate 系统的架构设计者。

两类读者需要的信息不同。crate README 不应是系统文档的缩略版，`docs/*.md` 也不应成为源码
逐行说明。每份文档必须有明确所有权，并让读者知道去哪里查另一层信息。

## 2. 两层文档的职责

| 维度 | Crate `README.md` | `docs/<topic>.md` |
| --- | --- | --- |
| 核心问题 | “这个 crate 具体如何工作、如何安全修改？” | “这个系统为什么这样设计、各组件如何协作、往哪里演进？” |
| 读者 | crate 实现者、reviewer、adapter 作者 | 架构维护者、跨团队实现者、产品/安全设计者 |
| 范围 | 当前 crate 内部与直接 integration contract | 跨 crate、进程、持久化、UI 和用户语义 |
| 细节 | 文件、类型、调用顺序、校验、错误、常量、测试、修改路径 | ownership、data flow、decision、tradeoff、风险、状态、阶段性演进 |
| 当前事实 | 必须与代码逐项对应 | 以已验证 capability 和系统 contract 为准 |
| 未来内容 | crate extension point 与已知限制 | 产品/架构方向、阶段、前置条件和长期不变量 |
| 避免 | 泛泛价值描述、重复大架构、虚构 future API | 源码清单、private helper 逐项复述、重复 README |

当一个主题同时有两层文档时：

- 两份文档开头互相链接；
- 明确哪份文档拥有什么 canonical information；
- 相同内容只在一处详细解释，另一处给出必要摘要与链接；
- 发生冲突时，先修正 canonical owner，再修正引用方。

## 3. Crate README 必须回答的问题

README 应优先帮助下一位修改代码的人。按适用性包含：

1. crate 精确拥有和明确不拥有的职责；
2. dependency direction 与禁止依赖；
3. 目录/module 职责；
4. public API 的调用者、实现者和不变量；
5. 承载 ownership、validation、binding 和 failure semantics 的关键 private symbol；
6. public/private interface 之间的真实调用图；
7. 哪些内部接口变化意味着设计方向或 crate ownership 已经漂移；
8. 核心调用路径及其顺序为何重要；
9. input/output 的准确 shape、trust boundary 和 validation；
10. error、cancellation、retry 与 failure semantics；
11. host/adapter 接入要求；
12. Cargo/Bazel 测试入口和 fixture/eval 组织；
13. 常见修改的影响面与同步更新清单；
14. 当前限制；
15. 可能演进及其前置条件。

不是所有 crate 都需要把这些问题拆成独立标题，但不能用“Main API”“Security”几个泛化 bullet
替代真正的实现信息。

README 中允许并鼓励记录容易从 API 表面误判的细节，例如：

- limit 使用 byte 还是 character；
- cancellation 在哪个 checkpoint 观察；
- ID 对 raw input 还是 canonical value hash；
- trait 无法从类型系统强制的 host obligation；
- schema 是 provider hint 还是本地 authoritative validation；
- 哪个修改必须 bump revision 或更新 fixture。

### 3.1 内部接口地图

README 应直接写出关键内部 symbol 的真实名称，例如 private struct、enum、method、conversion、
schema builder、hash helper、limit constant 和 orchestration function。目标是让读者能够对照源码
判断实现是否沿着设计方向发展，而不是让文档停留在无法证伪的架构语言。

内部接口地图至少说明：

- symbol 名称与可见性；
- 它拥有的单一职责；
- input/output 或前后调用关系；
- 它不能承担的职责；
- 修改该 symbol 需要同步检查的 contract、test 和文档。

应当记录承载设计的 interface，不必罗列 trivial getter、纯格式化 helper 或没有独立 contract 的
局部函数。判断标准是：该 symbol 如果被删除、绕过或迁移，是否可能改变 ownership、security、
durability、validation、binding 或 extension direction。

README 还应给出一张文本或 Mermaid 调用图，把 public entry point 连接到关键 private interface。
调用图必须使用当前源码中的真实名称。对于特别容易发生偏差的边界，应明确写出“出现什么代码
意味着方向已经漂移”。

## 4. `docs/*.md` 必须回答的问题

系统文档应优先帮助跨边界决策。按适用性包含：

1. 决策摘要；
2. 要解决的产品/系统问题与 non-goal；
3. component ownership table；
4. end-to-end data/control flow；
5. 用户可见语义；
6. security、durability、privacy 或 compatibility boundary；
7. 关键设计取舍及被拒绝的替代方案；
8. current implementation status；
9. observability、evaluation 和 rollout strategy；
10. 近期、中期、潜在演进；
11. 不随实现替换而改变的长期不变量。

系统文档可以引用准确类型名来固定 contract，但不应复制 private function、文件树或所有错误
variant。实现细节链接到 crate README。

## 5. 状态必须显式

未来设计不能伪装成当前实现。使用下列状态语言：

| 状态 | 含义 |
| --- | --- |
| Current / 已实现 | 代码和测试中已经存在，可给出证据 |
| Current limitation / 当前限制 | 已知缺口或有意保持的窄边界 |
| Extension point / 扩展点 | 当前 contract 已预留，但尚无具体实现 |
| Proposed / 计划设计 | 已有明确方向，仍可能在实现中调整 |
| Potential / 潜在方向 | 需要数据、需求或前置能力，不构成承诺 |
| Non-goal / 长期不做 | 用于防止 ownership 漂移 |

禁止：

- 用将来时段落描述并不存在的 public API，却不标 Proposed；
- 把测试 fixture 写成 production capability；
- 把“模型可能做到”写成系统保证；
- 用 roadmap 掩盖当前 failure semantics；
- 用虚构 PR、版本或完成度证明状态。

## 6. 信息价值标准

每个 section 至少满足一个目标：

- 固定一个容易破坏的 contract；
- 解释一个不直观的设计理由；
- 指出一个可信边界或 failure mode；
- 给出可执行的修改/验证路径；
- 区分当前事实与未来选择；
- 减少跨 crate ownership 误判。

如果一段话删除后不会影响实现、review、debug 或架构决策，它通常信息价值不足。优先删除：

- “provides a robust/flexible solution”一类宣传语；
- 从 crate 名称即可推断的描述；
- 与系统文档重复、但更不完整的流程；
- 没有 owner、前置条件或状态的 future wishlist；
- 没有说明后果的 API 名称列表。

## 7. 推荐模板

### 7.1 Crate README

```text
# crate-name
> 文档所有权与系统文档链接

一句话精确定义
1. Crate 边界
2. 文件与职责
3. Public API / contracts
4. 内部接口地图与真实调用图
5. 核心执行路径
6. Input / output / validation
7. Error / cancellation / recovery
8. Integration
9. Tests / fixtures / eval
10. 常见修改路径
11. 当前限制
12. 可能演进
```

### 7.2 系统文档

```text
# Topic：系统边界与演进
> canonical ownership、状态与 README 链接

1. 决策摘要
2. 问题与 non-goal
3. Ownership
4. End-to-end model
5. 用户/调用方语义
6. 关键安全与可靠性边界
7. Current status
8. Evaluation / rollout
9. 近期与潜在演进
10. 长期不变量
```

模板不是要求机械填满标题。小 crate 可以合并章节，但必须保留对维护真正有用的信息。

## 8. 图表与代码示例

- 只有关系、状态或顺序难以用短段落表达时才使用图；
- system diagram 表达 ownership/control flow，不罗列 private helper；
- README 的 sequence 可以精确到 public method 和 validation checkpoint；
- 示例必须与当前 API 一致，无法编译的伪代码应明确标注；
- 不用大图重复紧邻的长列表；
- 表格用于 exact mapping、decision matrix 和责任边界，不用于装饰。

## 9. 更新规则

以下变更通常要求更新 crate README：

- public API、module ownership 或 dependency direction；
- validation、limit、hash/binding、error 或 cancellation 语义；
- adapter obligation；
- test/eval 入口；
- 已知限制被解决或新增。

以下变更通常要求更新系统文档：

- 用户可见 decision 或 interaction；
- 跨 crate ownership、durable boundary 或 trust model；
- policy matrix；
- rollout/evaluation strategy；
- 当前状态和演进阶段。

同一变更影响两层时，两层都更新，但分别描述各自拥有的信息，不复制同一实现段落。

## 10. Review checklist

提交文档前确认：

- [ ] 开头说明了文档所有权、读者和关联文档；
- [ ] 当前事实能在代码、schema 或测试中验证；
- [ ] crate README 包含实现者真正需要的细节；
- [ ] README 写出了关键 private symbol 的真实名称、职责和调用关系；
- [ ] README 指出了能够暴露 ownership/方向漂移的内部变化；
- [ ] system doc 解释了 why、ownership、tradeoff 和 evolution；
- [ ] Current、Proposed、Potential 没有混写；
- [ ] 没有复制另一层已经 canonical 的大段内容；
- [ ] failure、security、privacy、durability 边界没有只写 happy path；
- [ ] 未来方向写明前置条件，没有伪装成承诺；
- [ ] 命令、路径、类型名和链接仍然有效；
- [ ] 删除泛化段落后，剩余内容仍有清晰重点。
