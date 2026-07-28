# `zeta-auto-review`

> 本 README 解释 crate 内部实现、维护约束和修改路径。跨 `zeta-policy`、Core、App Server、
> Tool 与 sandbox 的系统设计见 [`docs/auto-review.md`](../../docs/auto-review.md)。
> 文档分层遵循 [`docs/documentation-guidelines.md`](../../docs/documentation-guidelines.md)。

`zeta-auto-review` 把一个已经由 host 完整解析的 `ActionReviewRequest` 交给独立 review model，
将严格 JSON 响应转换成绑定 action 与 policy revision 的 `ClassifierAssessment`。

它只产生 advisory recommendation。它不能创建 grant、执行 Tool、修改 sandbox policy，也不拥有
用户审批流程。

## 1. Crate 边界

本 crate 拥有：

- review-only system prompt；
- provider-neutral `ReviewModel` port；
- model input JSON 的序列化；
- recommendation response schema；
- response 大小、JSON shape 和 capability 边界校验；
- canonical assessment ID 的构造；
- model、cancellation 和 invalid-response 错误分类；
- auto-review seed eval corpus。

本 crate 不拥有：

- `ResolvedAction`、capability、review context 和 recommendation domain type；它们属于
  `zeta-policy`；
- recommendation 是否足以产生执行授权；它由 `PolicyEngine` 决定；
- review model 的选择、credential 和 provider runtime；它们由 App Server 组合；
- user approval、durable Tool lifecycle 和 rejection circuit breaker；它们属于 Core；
- OS enforcement；它属于 Tool executor 与 `zeta-sandboxing`。

依赖方向必须保持：

```text
zeta-auto-review
  ├─ zeta-policy
  ├─ zeta-sandboxing（只读取 sandbox contract）
  └─ zeta-async-utils

禁止依赖：Core / App Server / provider runtime / credential store / approval UI
```

## 2. 文件与职责

```text
zeta-rs/auto-review/
├── src/
│   ├── lib.rs                  # 私有模块与显式 public export
│   ├── classifier.rs           # production classifier、schema、validation
│   └── classifier_tests.rs     # classifier 单元测试
├── evals/
│   ├── README.md               # corpus 格式、隐私与指标要求
│   └── cases.jsonl             # versioned synthetic gold cases
├── tests/
│   └── eval_contract.rs        # 离线 corpus + PolicyEngine contract
├── BUILD.bazel
└── Cargo.toml
```

不要把 provider adapter 或 Core orchestration 移入本 crate。需要增加内部实现时，保持 module
private，并从 `lib.rs` 显式导出必要 API。

## 3. Public API

| API | 作用 | 实现者或调用者必须保证 |
| --- | --- | --- |
| `ReviewModel` | provider-neutral completion port | 观察 cancellation；返回一个 JSON string；不提供 Tool、memory 或 mutation capability |
| `ReviewModelRequest` | 一次 review 的 exact prompt payload | adapter 分别消费 system prompt、input JSON 和 response schema，不得把 Agent instruction 提升为 system instruction |
| `LlmActionClassifier<M>` | 实现 `zeta_policy::ActionClassifier` | caller 提供 review model 和可审计的 prompt revision |
| `AutoReviewError` | fail-closed 的错误分类 | caller 只能映射为显式 failure policy，不能把错误当成批准 |

`ActionReviewRequest`、`ClassifierAssessment`、`ClassifierRecommendation`、`RiskLevel` 和
`UserAuthorization` 均来自 `zeta-policy`。这使 authorization authority 不依赖具体 LLM
implementation。

## 4. 内部接口地图

下面列出承载本 crate 设计方向的 private interface。它不是所有 helper 的机械清单；这些 symbol
共同决定 model boundary、serialization、validation 和 assessment binding，修改时必须能说明
ownership 为什么仍然正确。

| Symbol | 可见性 | 当前职责 | 方向约束 |
| --- | --- | --- | --- |
| `SYSTEM_PROMPT` | private constant | reviewer 的 trusted instruction | 不接收 action content；语义改变必须 bump prompt revision |
| `MAX_MODEL_RESPONSE_BYTES` | private constant | model response 的 16 KiB 上限 | 在 JSON parse 前检查；不能由 model input 覆盖 |
| `LlmActionClassifier::model_request` | private method | 将 policy request 组装为三个分离的 model payload | 只序列化，不重新解析 action、选择 model 或授予 capability |
| `LlmActionClassifier::validate_recommendation` | private method | enforce approve exact match 与 revise subset | 所有 model capability 必须在进入 assessment 前经过这里 |
| `ModelInput` | private struct | 定义发送给 model 的 input JSON shape | 只借用 host-owned domain values，不拥有权限语义 |
| `ModelInput::from` | private conversion | 映射 action、provenance、sandbox、revision、context | 不读取 Thread、config、credential 或 provider state |
| `ModelSandboxCompatibility` | private enum | 把 sandbox contract 转成稳定的 tagged JSON | 只描述 host 结论，不让 model 修改 availability |
| `ModelRecommendation` | private enum | strict deserialize target，拒绝 unknown fields | 不能直接成为 execution decision 或 grant |
| `From<ModelRecommendation> for ClassifierRecommendation` | private conversion impl | 转换成 `zeta-policy` advisory domain type | 转换后仍必须经过 host validation |
| `response_json_bytes` | private function | 序列化 validated canonical recommendation | assessment ID 不 hash 未解析的 raw model text |
| `response_schema` | private function | 定义四种 recommendation JSON shape | 必须与 `ModelRecommendation` lockstep 更新 |
| `capability_schema` | private function | 定义 capability `kind + scope` schema | 必须与 `zeta_policy::Capability` serialization 一致 |

### 4.1 内部调用图

```text
ActionClassifier::classify
├─ cancellation.is_cancelled
├─ LlmActionClassifier::model_request
│  ├─ ModelInput::from
│  ├─ serde_json::to_string(ModelInput)
│  └─ response_schema
│     └─ capability_schema
├─ ReviewModel::complete
├─ cancellation.is_cancelled
├─ response byte-limit check
├─ serde_json::from_str<ModelRecommendation>
├─ From<ModelRecommendation> for ClassifierRecommendation
├─ LlmActionClassifier::validate_recommendation
├─ response_json_bytes
├─ AssessmentId::from_response
└─ ClassifierAssessment::new
```

这张图也用于识别实现漂移：

- 如果 action resolution 出现在 `ModelInput::from`，host/policy ownership 已经漂移；
- 如果 capability validation 只存在于 provider adapter，本地可信边界已经漂移；
- 如果 raw response 在 parse/validation 前进入 assessment hash，audit identity 语义已经改变；
- 如果 `ModelRecommendation` 直接映射为 execution authority，classifier 越过了 policy boundary；
- 如果 model selection 或 mutable config read 进入 `model_request`，safe-point ownership 已经漂移。

### 4.2 内部接口的同步修改关系

| 修改 | 必须同步检查 |
| --- | --- |
| `ModelInput` field | `ModelInput::from`、trust/redaction owner、classifier tests、eval corpus |
| `ModelRecommendation` variant/field | `response_schema`、conversion impl、policy domain、Core handling、eval labels |
| Capability JSON shape | `capability_schema`、`Capability` serde、exact/subset validation、fixtures |
| Prompt policy | `SYSTEM_PROMPT`、prompt revision、injection cases、model eval |
| Assessment binding | `response_json_bytes`、`AssessmentId::from_response`、audit consumer、stable-ID tests |
| Response limit | `MAX_MODEL_RESPONSE_BYTES`、error expectations、provider behavior、large-response tests |

## 5. `classify()` 的准确执行顺序

`LlmActionClassifier::classify()` 当前是同步的一次性调用：

```text
1. preflight cancellation check
2. ActionReviewRequest → ModelInput JSON
3. 构造 response schema JSON
4. ReviewModel::complete(...)
5. post-call cancellation check
6. response byte limit
7. strict serde parse + capability validation
8. canonical recommendation → AssessmentId
9. 返回 request-bound ClassifierAssessment
```

顺序具有安全含义：

- 调用前已取消时，model 不应启动；
- model 返回后发生取消时，结果被丢弃，不产生 assessment；
- response 上限是 16 KiB，使用 Rust `String::len()` 的 UTF-8 byte length；
- `deny_unknown_fields` 拒绝模型偷偷增加 `authorization`、`grant` 等字段；
- assessment 复制 host-owned action digest 与 policy revision，模型不能覆盖它们。

### 5.1 Assessment identity

`AssessmentId` 是下列内容的 SHA-256：

```text
action_digest
NUL
policy_revision
NUL
prompt_revision
NUL
canonical_recommendation_json
```

这里使用解析后重新序列化的 canonical recommendation，而不是模型原始文本。因此仅改变 JSON
空格或字段顺序不会产生新的 ID；改变 recommendation 语义、prompt revision、action 或 policy
revision 会产生新 ID。

修改 system prompt、response schema 的语义或 review policy 时，组合层必须同步 bump
`prompt_revision`。当前类型不会自动验证 revision 与 prompt source 是否一致，这是调用方必须
维护的 audit invariant。

## 6. Model request

`ReviewModelRequest` 包含三个互相分离的字符串：

| 字段 | 内容 | 信任级别 |
| --- | --- | --- |
| `system_prompt` | crate 内编译的 reviewer policy | trusted classifier instruction |
| `input_json` | action、provenance、sandbox、policy revision、review context | host metadata 与 untrusted content 的结构化混合 |
| `response_schema_json` | 四种 recommendation 的严格 JSON schema | trusted output contract |

`input_json` 的结构为：

```text
action
provenance
sandbox
policy_revision
context
  ├─ user_intent
  └─ evidence[]
       ├─ kind
       ├─ trust
       ├─ source
       └─ content
```

本 crate 保留 evidence 的 trust label，但不负责生成或截断 context。Core/Tool host 必须在构造
`ActionReviewRequest` 前完成长度限制、secret removal 和 trust classification。直接调用本 crate
时绕过这些 host 约束，会把任意大小或含 secret 的内容发送给 model。

`ReviewModel` 是安全边界而不只是 transport abstraction。trait 本身无法阻止恶意实现调用 Tool
或读取 credential；可信 host adapter 必须确保 review runtime 没有这些能力。

## 7. Response contract 与本地校验

模型只能返回以下四种 shape：

| Recommendation | 必填字段 | 本 crate 的额外检查 |
| --- | --- | --- |
| `approve` | `capabilities`, `risk`, `user_authorization`, `reason` | capabilities 必须非空且与 action required capabilities 完全相等 |
| `revise_action` | `maximum_capabilities`, `reason` | maximum capabilities 必须是原 action capabilities 的子集，可以为空 |
| `ask_user` | `reason` | 不接受额外字段 |
| `deny` | `reason` | 不接受额外字段 |

Capability 由精确的 `kind + scope` 组成。在 `approve` 中，模型不能通过扩大 scope、添加
capability kind 或遗漏原 capability 获得部分批准；`revise_action` 则有意允许返回原集合的
子集。

Response schema 提供给 adapter 以约束生成，但本地 serde parse 和 host validation 才是最终
可信边界。当前 App Server adapter 将 schema 放入 model instructions；即使 provider 不支持
原生 structured output，malformed response 仍会在本 crate 被拒绝。

本 crate 不执行 risk/authorization 到 execution decision 的映射。例如 high risk +
implicit authorization 即使被解析为 `Approve` recommendation，仍由 `PolicyEngine` 转成
`AskUser`。系统级矩阵见
[`docs/auto-review.md`](../../docs/auto-review.md#5-风险与用户授权矩阵)。

## 8. 错误与 cancellation

| 条件 | `AutoReviewError` | 是否产生 assessment |
| --- | --- | --- |
| model 调用前或返回后 cancellation | `Cancelled` | 否 |
| `ReviewModel` 返回错误 | `Model(String)` | 否 |
| response 超过 16 KiB | `ResponseTooLarge { bytes }` | 否 |
| JSON 无效、字段缺失、unknown field、enum 无效 | `InvalidResponse(String)` | 否 |
| approve capability 非 exact match | `InvalidResponse(String)` | 否 |
| revise capability 不是 subset | `InvalidResponse(String)` | 否 |

`ReviewModel::complete()` 返回 `Result<String, String>` 是当前窄 port，不表达 provider error
taxonomy。App Server 应在进入本 crate 前完成安全 redaction。错误最终如何成为 `Block` 或
`AskUser` 由 `zeta-policy::ReviewFailurePolicy` 决定。

Cancellation 只能保证本 crate 在调用前后检查。正在进行的网络请求能否及时停止，取决于
`ReviewModel` implementation 是否在 provider checkpoint 观察 token。

## 9. Host adapter 接入要求

当前 App Server 的 `ProviderReviewModel` 是参考 adapter。它从 frozen `ResolvedConfig`
safe-point snapshot 创建 immutable model runtime，并：

- 不向 request 注册 Tool；
- 将 `tool_choice` 固定为 `None`；
- 禁止 parallel Tool Call；
- 使用 temperature `0.0`；
- 忽略 reasoning，只拼接 text；
- 拒绝 refusal、Tool Call 和空 text；
- 在 provider invocation 前后检查 cancellation。

新的 adapter 必须保持相同安全语义，但不要求使用相同 provider API。最小实现形态：

```rust
impl ReviewModel for ProviderAdapter {
    fn complete(
        &self,
        request: &ReviewModelRequest,
        cancellation: &CancellationToken,
    ) -> Result<String, String> {
        // Invoke one immutable, tool-less reviewer and return only its JSON text.
    }
}
```

不要在 adapter 内重新读取 mutable config、fallback 到未记录的 model，或把普通 Agent runtime
连同其 Tool registry 直接复用给 reviewer。

## 10. 测试与 eval

运行 crate 全部 Cargo 测试：

```text
cargo test -p zeta-auto-review
```

运行 Bazel targets：

```text
bazel test \
  //zeta-rs/auto-review:auto-review-unit-tests \
  //zeta-rs/auto-review:eval-contract-tests
```

测试分两层：

- `classifier_tests.rs` 直接覆盖 parsing、binding、capability escalation、response shape 和
  cancellation；
- `eval_contract.rs` 离线读取 `evals/cases.jsonl`，检查 schema/coverage，并把 gold
  recommendation 送入真实 `PolicyEngine` 验证最终 disposition。

默认测试禁止访问网络或调用真实 model。Model-backed eval 必须是显式 runner，记录 model 与
prompt revision，并输出 false-auto-approval 等安全指标。Corpus 的隐私和扩充规则见
[`evals/README.md`](evals/README.md)。

## 11. 常见修改路径

### 修改 prompt 或 review policy wording

1. 修改 `SYSTEM_PROMPT`；
2. bump 组合层传入的 prompt revision；
3. 增加 prompt injection 与边界 case；
4. 用显式 model eval runner 比较新旧 revision；
5. 不因 wording 变化放宽本地 schema/validation。

### 增加 recommendation

这不是 crate-local change。至少同时审查：

1. `ModelRecommendation` 和 response schema；
2. `zeta_policy::ClassifierRecommendation`；
3. `PolicyEngine` 的 final decision mapping；
4. Core Tool scheduler 的 durable/feedback 行为；
5. classifier tests、policy tests 和 eval corpus；
6. `docs/auto-review.md` 的用户可见语义。

### 增加 evidence kind

Evidence domain type 属于 `zeta-policy`。新增 kind 后需要确认：

- 哪个 host 负责产生它；
- trust label 是否可能被伪造；
- 是否含 repository/user/tool untrusted content；
- 长度限制与 secret redaction 在哪里实施；
- eval corpus 是否覆盖相应 injection path。

### 更换 provider 或增加 model

通常不修改本 crate。应在 App Server/provider 组合层实现或选择新的 `ReviewModel`，并保持
request、cancellation、tool-less runtime 与 immutable config snapshot contract。

## 12. 当前限制

当前实现有意保持较窄：

- 一个 action 对应一次同步 completion，不支持 streaming、ensemble 或分阶段 review；
- prompt 是 compile-time constant，没有 per-organization policy steering；
- schema 由私有 Rust function 构造，尚未作为独立 versioned artifact 导出；
- reason 只要求是 string，production validator 尚未检查空字符串或 rationale quality；
- context budget 与 secret removal 依赖 Core/Tool host，crate 不重复截断；
- seed corpus 只验证格式与 policy contract，尚未运行真实 model benchmark；
- 没有 calibration、shadow-mode telemetry 或 human override feedback loop。

这些限制是当前事实，不应在调用方文档中描述成已经解决。

## 13. 可能的演进

以下是扩展方向，不是已承诺 API：

1. 增加显式 model eval runner，比较 model/prompt revision 并生成安全指标；
2. 在有隐私审查的前提下，将 human override 转成匿名 regression case；
3. version response schema 与 prompt policy，减少 revision 由调用方手工维护的风险；
4. 增加组织级 policy steering，但继续与 untrusted action/context 分层；
5. 当 one-shot reviewer 的误差有数据证据时，再评估 tiered review、ensemble 或专用模型；
6. 只有积累足够高质量 label 后，才评估 fine-tuning；训练不是建立 eval corpus 的前置条件。

任何演进都不能把 classifier 变成 capability authority，也不能允许 model error、malformed
response 或 cancellation 产生授权。
