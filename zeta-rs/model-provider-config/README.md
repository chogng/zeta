# `zeta-model-provider-config`

> 本 README 解释 provider declaration、registry validation 与 normalization。跨系统配置模型和
> 演进见 [`docs/model-provider-config.md`](../../docs/model-provider-config.md)；runtime
> instantiation 见 [`zeta-model-provider`](../model-provider/README.md)。

本 crate 拥有 serializable、runtime-free 的 provider configuration。它不持有 API client、
credential、secret、connection pool 或 process-local adapter。

## 公共模型

| Symbol | 职责 | 关键语义 |
| --- | --- | --- |
| `ModelProviderConfig` | 用户/host 可配置值 | provider、base URL、max output 与 per-model context metadata |
| `ModelContextConfig` | 单模型的 Core budget metadata | positive context window、optional auto-compact limit |
| `ProviderDefinition` | provider-owned declaration | adapter identity、API profile、endpoint/catalog/defaults |
| `NormalizedModelProviderConfig` | runtime-ready immutable config | provider/profile/base URL 已确定 |
| `ProviderConfigRegistry` | definition authority | validate、register、merge、selection、normalize |
| `ProviderAdapter` | serializable adapter identity | 不是 runtime trait/object |
| `ApiProfile` | declarative wire profile | runtime 显式解析为 `zeta-api::ApiEndpoint` |
| `EndpointPolicy` | provider default 或 configured-only | 不执行 DNS/network validation |
| `ModelCatalogPolicy` | listed-only 或 allow-unlisted | 只表达 static catalog gate |
| `ApprovalReviewModelDefault` | automatic review default | active model 或 provider-declared model |
| `ProviderConfigError` | static/normalization error | 不包含 transport/auth failure |

`model_provider_config_schema()` 与 `provider_definition_schema()` 从 Rust types 生成 JSON Schema；
schema 没有第二份手写来源。

## 文件与内部接口

```text
src/
├── config.rs       # user config、normalized config、URL helpers
├── definition.rs   # provider declaration 与 validation
├── registry.rs     # registration、merge、selection、normalization
├── providers/      # one private built-in definition per provider
├── error.rs
└── lib.rs
```

| Symbol | 可见性 | 当前职责 | 方向约束 |
| --- | --- | --- | --- |
| `ModelProviderConfig::validate_static` | public method | zero output/context limits 与 configured URL shape | 不依赖 registry或网络 |
| `ProviderDefinition::validate` | public method | name、default endpoint、defaults、catalog uniqueness | definition 自身必须独立有效 |
| `ProviderConfigRegistry::register` | public method | validate + reject duplicate | built-in/plugin 定义走相同路径 |
| `ProviderConfigRegistry::merge` | public method | prevalidate incoming + explicit conflict policy | merge 不能 partial apply |
| `ProviderConfigRegistry::normalize` | public method | config + definition → normalized snapshot | endpoint/default/profile precedence 在此唯一实现 |
| `normalize_for` | public method | 先 enforce selected/configured provider identity | 防止 model ref 与 config 串线 |
| `automatic_approval_review_model` | public method | provider default 或 active model fallback | 不证明远端 entitlement |
| `validate_model_selection` | public method | listed catalog 的 static gate | allow-unlisted 仍需 runtime validation |
| `normalize_base_url` | crate-private function | apply explicit normalization rule | 不追加 API route |
| `is_http_url` | crate-private function | 最小 HTTP(S) shape check | 不是 full URL/network validator |
| `providers::builtin` | crate-private function | 13 个 built-in definitions | 每个 provider 在 sibling module 独立定义 |
| `default_provider` / `configured_provider` | private helpers | shared definition constructors | 不隐藏 provider-specific profile/default differences |

## 规范化调用图

```text
ProviderConfigRegistry::normalize(config)
├─ ModelProviderConfig::validate_static
├─ registry.get(config.provider)
├─ choose base URL
│  ├─ non-empty configured override
│  ├─ EndpointPolicy::ProviderDefault
│  └─ ConfiguredOnly without value → MissingBaseUrl
├─ normalize_base_url(definition rule)
├─ is_http_url
├─ choose max_output_tokens
│  └─ config value overrides provider default
└─ NormalizedModelProviderConfig
```

`model_context` 不进入 `NormalizedModelProviderConfig`，因为它不改变 transport endpoint。Local App
Server 在冻结一次模型调用预算时，按 selected `ModelId` 读取该 map；配置值优先于 built-in
`ModelInfo.context_window`。没有可信窗口时 Core 使用 provider-managed，不在本 crate 猜测模型
规格。

```text
ProviderConfigRegistry::merge(incoming, policy)
├─ validate every incoming definition
├─ preflight all conflicts when RejectConflicts
└─ extend map only after preflight succeeds
```

Merge 必须 preflight 后一次 extend；在循环中边验证边插入会造成 error 后 registry 部分变化。

## 内置供应商定义

每个 `providers/<name>.rs::definition()` 返回一个完整 `ProviderDefinition`。例如 OpenAI 选择
`OpenAiResponses` 与 provider default URL；Anthropic 选择 `AnthropicMessages` 并声明默认 max
tokens。Provider matrix 和官方依据由系统文档维护，本 README 只固定 definition construction
pattern。

新增 provider 时同步：

1. 增加 `ProviderAdapter` variant；
2. 新建 private `providers/<name>.rs::definition`；
3. 加入 `providers::builtin()`；
4. 在 runtime crate 增加对应 adapter mapping；
5. 增加 definition/normalization/runtime tests；
6. 更新系统 provider matrix 与 schema fixture。

## 方向偏差检查

- config crate 依赖 `zeta-api`/HTTP/secret：runtime state 下沉；
- `ProviderAdapter` 直接存 trait object：declaration 不再 serializable；
- runtime 根据 provider name 猜 API profile：显式 declaration 被绕过；
- normalization 追加 `/responses` 等 route：endpoint/profile ownership 混淆；
- allow-unlisted 被描述为远端可用：static evidence 被夸大；
- merge conflict 造成 partial registry mutation：config snapshot 不再原子。

## 测试、限制与演进

```text
cargo test -p zeta-model-provider-config
bazel test //zeta-rs/model-provider-config:model-provider-config-unit-tests
```

测试覆盖 serde/schema、defaults、configured-only endpoint、invalid URL/output/context tokens、merge semantics、
automatic review model、catalog gate、built-in completeness 与 provider mismatch。

当前 URL validator 只接受具有非空 authority 的 HTTP(S) shape，不解析 credential、DNS、route 或
reachability。Catalog 也是 declarative snapshot，不代表 account entitlement。未来可以扩充 schema、
definition source 与 catalog metadata，但必须保持 runtime-free、deterministic normalization 和显式
profile selection。
