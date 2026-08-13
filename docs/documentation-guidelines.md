# Zeta 文档分层与写作规范

> 状态：Current repository convention。
> 适用范围：crate README、`docs/*.md` 架构/设计文档及两者之间的引用关系。

本文是整个仓库文档系统的全局规范，不是权限、沙箱或少数示例文档的局部写法。所有现有文档、
新增文档和文档站页面都必须遵守；发现不一致时，将其视为需要修正的文档缺陷，不能把旧页面
当作例外或先例。

## 快速理解

Zeta 文档首先帮助读者建立正确的系统心智模型，然后才提供实现证据。系统文档以用户问题和
端到端行为为主线，crate README 以安全修改实现为主线；两层文档互相链接，但不互相复制。

| 读者正在做什么 | 应该先看到什么 | 实现细节放在哪里 |
| --- | --- | --- |
| 理解某项产品行为 | 常见场景、行为差异和例外 | 对应系统文档的后半部分 |
| 判断架构边界 | 谁决定、谁执行、谁保存，以及失败后发生什么 | 系统文档的流程和所有权章节 |
| 修改一个 crate | 公共契约、关键内部符号、测试和修改影响 | crate README |
| 核对当前完成度 | 已实现、当前限制和计划方向的明确分隔 | 系统文档和实现证据链接 |

所有文档统一采用下面的认知顺序：

```text
读者的问题
  → 可观察行为
  → 一次操作的流程
  → 决定、执行和保存的责任
  → 边界、失败与例外
  → 当前实现和代码证据
  → 计划演进
```

| 规范层 | 作用范围 | 统一入口 |
| --- | --- | --- |
| 信息结构与语言 | 全部 `docs/*.md` 和 crate README | 本文 |
| 页面布局、字体、列表、表格和颜色 | 文档站全部页面 | 文档站全局样式 |
| 标题展示、导航、目录和源码入口 | 文档站全部页面 | 统一页面组件 |
| 最低机械约束 | 文档站收录的全部 Markdown | `npm run check:docs` |

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

### 4.1 先回答用户会遇到什么

解释权限、模式、Tool、配置、交互或其他用户可见系统时，优先使用以下顺序：

1. 用一句话说明系统如何解决问题；
2. 紧接一张行为表，让读者直接比较常见场景；
3. 再解释内部类型、执行流程、ownership 和例外。

系统地图直接指向的权威子文档统一先提供“快速理解”章节。该章节紧跟文档所有权和状态说明，
包含一段不依赖内部名词的摘要，以及一张回答常见问题、场景或行为的表格。后续章节才进入
crate、类型、协议字段和计划阶段。文档检查会验证这些权威子文档没有丢失阅读入口。

这不是少数权威文档的特殊版式。全部 `docs/*.md` 都必须把“快速理解”作为第一个二级标题；
规范、模板和验收手册也要先说明读者如何使用它们。标题前可以有一段简短的所有权、状态和相关
文档说明，但不能先出现 crate 清单、类型定义、文件树或长篇实现状态。

一级标题优先使用读者能够识别的系统或能力名称，例如“模型调用系统”“配置系统”和“工作区
搜索”。只有文档确实描述单个 crate 的实现契约时，才把 crate 名称作为一级标题。物理路径、
crate 名称和内部层次属于实现索引，不应代替文档主题。

“快速理解”至少包含：

1. 一段不依赖内部类型名的结果摘要；
2. 一张用常见问题、场景或行为作行的表格；
3. 指向后续流程、边界、当前状态和实现证据的阅读入口。

如果一次操作包含三个以上阶段，或者存在批准、失败、重试、平台分派等分支，在行为表之后使用
Mermaid 流程图。线性且没有分支的流程可使用编号列表，不为了装饰添加图。

第一张表的列名应直接回答用户问题，例如：

| 对象 | 典型示例 | 什么时候发生 | 用户需要做什么 | 决定有效多久 |
| --- | --- | --- | --- | --- |
| 某类动作或模式 | 用户能识别的真实操作 | 默认行为和关键条件 | 无需操作、确认、修改或停止 | 单次、会话、项目或永久 |

具体主题可以删减列，但不能用 `ExecutionDecision`、内部 enum 或 crate 名称作为第一层解释。内部
mapping 另设“系统内部如何表达”表格。表格之后只补充无法放进单元格的重要例外，避免先写多段
抽象概念再让读者自行归纳行为。

### 4.2 列表、表格与段落

内容结构必须反映信息之间的真实关系，不能只靠换行制造视觉分组：

- 三项及以上并列的概念、职责、规则、限制或选项使用项目符号列表；
- 每一项需要“名称 + 解释”时，优先使用“**名称**：解释”的列表项；
- 有先后顺序、执行步骤或优先级时使用编号列表；
- 多个对象需要沿相同维度比较时使用表格；
- 因果关系、设计理由、条件和例外使用连续段落；
- ownership、状态迁移或跨组件流向难以线性表达时才使用图；
- 调用路径包含分支、汇合或平台分派时使用 Mermaid 流程图，不用带箭头的代码块模拟流程。

不要把并列概念写成用分号隔开的长段落，也不要用连续 `<br>` 或 Markdown 行尾空格伪造列表。
如果每一行都能独立回答“它是什么、负责什么或有什么限制”，通常就应该是一个真正的列表项。

### 4.3 中英文术语与代码标识符

中文文档的叙述主体使用中文。英文只用于专有名称、行业通用缩写、无法替代的命令和精确代码
标识符，不能把英文名词直接嵌进中文语法来代替已经明确的中文概念。

| 内容类型 | 写法 | 示例 |
| --- | --- | --- |
| 用户可见概念 | 使用中文 | 权限、批准、拒绝、工作区、网络访问 |
| 技术概念首次出现 | 中文名称（英文名称） | 执行授权（execution authority） |
| 同一技术概念后续出现 | 只使用中文 | 执行授权 |
| 代码类型、函数、枚举值 | 中文解释 + 反引号内的真实标识符 | 策略引擎 `ActionPolicyEngine`、只读模式 `ReadOnly` |
| crate、命令、文件和配置键 | 保留真实名称并使用反引号 | `zeta-action-policy`、`cargo test`、`policy_revision` |
| 平台、协议和产品专名 | 保留正式英文名称 | macOS、Linux、Windows、Bubblewrap、Rust、MCP |
| 行业通用缩写 | 保留大写缩写，必要时首次解释 | API、HTTP、JSON、UI、CLI |

不要机械地为每个英文词补中文括号，也不要在每次出现时重复英文。括号的作用是第一次建立术语
映射；映射建立后，正文应恢复连续的中文叙述。只有讨论 wire field、public API、枚举分支或源码
关系时，才再次使用精确标识符。

以下写法不符合规范：

```text
grant 绑定 action digest、capabilities 和 policy revision。
```

推荐写成：

```text
授权凭证绑定动作摘要（action digest）、完整能力集合（capability set）和策略版本
（policy revision）。后文只写“动作摘要、能力集合和策略版本”。
```

用户行为表的标题和单元格优先使用中文。内部实现映射表可以出现 `RunSandboxed`、
`ApproveOnce`、`ActionPolicyEngine` 等标识符，但必须同时解释其用户或系统含义。

#### 核心术语表

| 推荐中文 | 首次出现时的写法 | 相关代码标识符 |
| --- | --- | --- |
| 动作 | 动作（action） | `ResolvedAction` |
| 动作摘要 | 动作摘要（action digest） | `ActionDigest` |
| 来源 | 来源（provenance） | `ActionProvenance` |
| 能力 | 能力（capability） | `Capability`、`CapabilityKind` |
| 能力集合 | 能力集合（capability set） | `CapabilitySet` |
| 作用范围 | 作用范围（scope） | capability scope |
| 策略版本 | 策略版本（policy revision） | `ActionPolicyRevision` |
| 确定性规则 | 确定性执行规则（deterministic execution policy） | `ExecPolicySnapshot`、`ExecPolicyRule`、`zeta-execpolicy` |
| 最终动作策略 | 最终 action authority | `ActionPolicyEngine`、`ExecutionDecision`、`zeta-action-policy` |
| 执行授权 | 执行授权（execution authority） | `ExecutionDecision`、`AutoReviewGrant` |
| 一次性批准 | 一次性批准 | `ApproveOnce` |
| 风险审查器 | 风险审查器（reviewer） | `ActionClassifier` |
| 审查结论 | 审查结论（assessment） | `ClassifierAssessment` |
| 证据 | 证据（evidence） | `ReviewEvidence` |
| 信任边界 | 信任边界（trust boundary） | `ReviewEvidenceTrust` |
| 持久化记录 | 持久化记录（durable record） | `ThreadEvent` |
| 安全重试 | 可安全重试（safe to retry） | `SafeToRetry` |
| 未知结果 | 未知执行结果（unknown outcome） | started without terminal result |
| 失败即关闭 | 失败即关闭（fail closed） | `Block` 或 `AskUser` |

术语表固定的是文档语言，不取代代码 API。代码重命名时同步更新“相关代码标识符”列；中文概念
只有在产品语义变化时才调整。

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

快速理解
1. 用户问题与可观察行为
2. 一次操作的端到端流程
3. 决定、执行和保存的责任边界
4. 明确不负责什么
5. 安全、可靠性和失败语义
6. 当前实现与当前限制
7. 实现证据和修改入口
8. 近期与潜在演进
9. 长期不变量
```

模板不是要求机械填满标题。小 crate 可以合并章节，但必须保留对维护真正有用的信息。

## 8. 图表与代码示例

- 只有关系、状态或顺序难以用短段落表达时才使用图；
- 有分支或汇合的执行路径使用 `flowchart`，强调时间和参与者交互时使用 `sequenceDiagram`；
- 静态目录、包含关系或单纯依赖集合可以使用短文本树；一旦读者需要沿箭头追踪执行，就改用流程图；
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

## 10. 审查清单

提交文档前确认：

- [ ] 这些规则应用到了同类文档，而不是只修正当前页面；
- [ ] 开头说明了文档所有权、读者和关联文档；
- [ ] 当前事实能在代码、schema 或测试中验证；
- [ ] crate README 包含实现者真正需要的细节；
- [ ] README 写出了关键 private symbol 的真实名称、职责和调用关系；
- [ ] README 指出了能够暴露 ownership/方向漂移的内部变化；
- [ ] system doc 解释了 why、ownership、tradeoff 和 evolution；
- [ ] 用户可见系统先用一句话和行为表说明常见场景，再进入内部类型与流程；
- [ ] 并列概念使用列表，同维度比较使用表格，因果和例外使用段落；
- [ ] 中文正文没有用裸英文名词代替已经定义的中文概念；
- [ ] 技术概念首次建立中英文映射，后续优先使用中文；
- [ ] 代码标识符、命令、crate 和配置键使用反引号并保留真实拼写；
- [ ] Current、Proposed、Potential 没有混写；
- [ ] 没有复制另一层已经 canonical 的大段内容；
- [ ] failure、security、privacy、durability 边界没有只写 happy path；
- [ ] 未来方向写明前置条件，没有伪装成承诺；
- [ ] 命令、路径、类型名和链接仍然有效；
- [ ] 删除泛化段落后，剩余内容仍有清晰重点。
