# `zeta-policy`

> 本 README 解释 action permission domain、deterministic decision engine 和内部决策顺序。跨
> Auto Review、Core、App Server、Tool 与 sandbox 的系统语义见
> [`docs/auto-review.md`](../../docs/auto-review.md)。

`zeta-policy` 是 Agent action execution decision 的 deterministic authority。它可以调用 advisory
`ActionClassifier`，但只有 `PolicyEngine` 能创建 `AutoReviewGrant`。本 crate 不执行 Tool、不显示
approval UI、不选择 model、不持久化 rule/grant。

## Domain 与 public contract

| 领域 | 关键类型 | 所有权 |
| --- | --- | --- |
| Resolved action | `ResolvedAction`, `ActionDigest`, `ActionKind`, `ActionProvenance` | host 完整 materialize 后交给 policy |
| Capability | `CapabilityKind`, `Capability`, `CapabilitySet` | exact `kind + scope`，BTreeSet canonical order |
| Review input | `ActionReviewRequest`, `SandboxCompatibility`, `PolicyRevision`, `ReviewContext` | immutable safe-point snapshot |
| Evidence | `ReviewEvidence`, `ReviewEvidenceKind`, `ReviewEvidenceTrust` | host 标注；repository/Tool/Agent 内容不可信 |
| Deterministic policy | `ActionRule`, `RuleEffect`, `UnsandboxedGrant` | exact digest/revision/capability matching |
| Advisory port | `ActionClassifier`, `ClassifierAssessment`, `ClassifierRecommendation` | implementation 不能执行或授权 |
| Final outcome | `ExecutionDecision`, `BlockReason`, `ApprovalRequest`, `SaferActionRequest` | caller 必须按 typed branch 处理 |
| Authority | `AutoReviewGrant` | constructor crate-private，只能由 engine 签发 |

`ActionDigest::from_canonical_bytes` 只负责 SHA-256。哪些字段进入 canonical bytes 由 host action
materializer 负责；遗漏 cwd、environment、resolved path 或 provenance 会导致错误 grant reuse，
本 crate 无法补救。

## 文件与内部接口地图

```text
src/
├── action.rs       # action/capability/review request
├── context.rs      # trust-labeled reviewer context
├── classifier.rs   # advisory port、assessment identity/recommendation
├── rule.rs         # exact rules 与 existing grant
├── decision.rs     # final typed decisions 与 grant
├── engine.rs       # precedence、binding、risk gate
└── engine_tests.rs
```

| Symbol | 可见性 | 当前职责 | 方向约束 |
| --- | --- | --- | --- |
| `PolicyEngine::decide` | public | 唯一 top-level precedence entry | classifier 不能绕过 earlier deterministic branches |
| `ensure_revision` | private | engine/request safe-point equality | mismatch 是 `PolicyError`，不是 classifier question |
| `deterministic_rule_decision` | private | deny first，再 require-sandbox | deterministic deny 必须优先 |
| `apply_assessment` | private | 验证 assessment binding，映射四种 recommendation | model identity mismatch 不能降级 AskUser |
| `automatic_approval_decision` | private | exact capability + risk/auth matrix | 只有这里可构造 `AutoReviewGrant` |
| `review_failure_decision` | private | explicit `ReviewFailurePolicy` mapping | classifier error 不得 fail open |
| `UnsandboxedGrant::matches` | public method | digest + capabilities + revision exact match | 不使用 summary/Tool name 模糊匹配 |
| `AutoReviewGrant::new` | crate-private | engine-only authority construction | 不得公开给 classifier/host adapter |
| `AutoReviewGrant::matches` | public method | execution-time binding check | Core 还需绑定 exact Tool Call |
| `AssessmentId::from_response` | public constructor | request/prompt/response audit hash | classifier 实现负责 canonical response bytes |

## 决策调用图

```text
PolicyEngine::decide(request, cancellation)
├─ ensure_revision
├─ deterministic_rule_decision
│  ├─ exact Deny → Block
│  └─ exact RequireSandbox
│     ├─ supported → RunSandboxed
│     └─ unavailable → Block
├─ exact UnsandboxedGrant::matches → RunUnsandboxed
├─ SandboxCompatibility::Supported → RunSandboxed
├─ ActionClassifier::classify
│  └─ error → review_failure_decision
└─ apply_assessment
   ├─ digest/revision binding check
   ├─ Approve → automatic_approval_decision
   ├─ ReviseAction → SaferActionRequest
   ├─ AskUser → ApprovalRequest
   └─ Deny → Block::ReviewerDenied
```

顺序是 contract。尤其是 require-sandbox rule 在 existing unsandboxed grant 之前，因此显式管理员
约束不能被旧 grant 绕过。

## Auto approval matrix

`automatic_approval_decision` 先要求 non-empty exact capabilities，再应用：

| Risk | Explicit | Implicit | Absent / Ambiguous |
| --- | --- | --- | --- |
| Low / Medium | `RunAutoReviewed` | `RunAutoReviewed` | `AskUser` |
| High | `RunAutoReviewed` | `AskUser` | `AskUser` |
| Critical | `Block::CriticalRisk` | `Block::CriticalRisk` | `Block::CriticalRisk` |

`Approve` 仍只是 recommendation。`AutoReviewGrant` 绑定 assessment ID、action digest、完整
capability set 和 policy revision。Core 在执行时再把它绑定到 exact durable Tool Call。

## Error 与 failure semantics

| Condition | Outcome |
| --- | --- |
| engine/request revision mismatch | `Err(PolicyError::RevisionMismatch)` |
| assessment digest/revision mismatch | `Err(PolicyError::ClassifierBindingMismatch)` |
| classifier failure + `Block` policy | `ExecutionDecision::Block(ReviewFailed)` |
| classifier failure + `AskUser` policy | `ExecutionDecision::AskUser` |
| approve capability mismatch/empty | `Block(ReviewFailed)` |
| deterministic sandbox required but unavailable | `Block(SandboxRequiredButUnavailable)` |

`PolicyError` 表示调用/binding contract 被破坏；`ExecutionDecision::Block` 是对合法 request 的 policy
outcome。调用方不能把两者混成普通 Tool failure。

## 方向偏差检查

- classifier 返回或构造 grant：advisory/authority boundary 被破坏；
- `AutoReviewGrant::new` 变成 public：外层可以绕过 risk gate；
- grant 只匹配 digest、不匹配 capability/revision：stale authority 可复用；
- sandbox-supported action仍调用 classifier扩权：least-privilege precedence 被破坏；
- ReviewContext 在本 crate 内读取 Thread/filesystem：evidence broker ownership 漂移；
- rule/grant persistence 进入 engine：pure decision 与 config/storage composition 耦合。

## 测试与修改路径

```text
cargo test -p zeta-policy
bazel test //zeta-rs/policy:policy-unit-tests
```

`engine_tests.rs` 使用 panic classifier 证明 deterministic/sandbox paths 不调用 model，并覆盖
exact grant、risk matrix、failure policy、binding mismatch 和 rule precedence。

修改 recommendation 或 final decision 时必须同步更新 `zeta-auto-review` schema、Core scheduler、
durable authority、eval corpus 和系统文档。新增 capability kind 时同步审查 Tool materializer、
approval protocol、sandbox enforcement 与 model schema。

## 当前限制与演进

当前 rules 仅 exact action digest 的 `Deny/RequireSandbox`，grants 由 host 注入，engine 不负责
persistence、pattern policy、organization hierarchy 或 expiry。未来可增加更丰富 deterministic
policy language，但 matching 必须可审计，并保持 classifier advisory、failure closed、grant exact
binding 和 deterministic-first 顺序。
