# `zeta-plugins`

> 本 README 是 Plugin manifest、identity、local package validation 与 discovery 的当前实现
> 契约。跨 crate 的安装、activation、权限和运行时演进由
> [`docs/plugins.md`](../../docs/plugins.md) 维护；Connector account/lifecycle 由
> [`docs/connectors.md`](../../docs/connectors.md) 维护。

`zeta-plugins` 严格解析 declarative Plugin v1 package，验证 package-relative path 与本地文件树，
计算确定性 SHA-256 digest，并把 host 注册的 Marketplace catalog 解析为 exact、digest-pinned package。
Marketplace install/update 先复制、复验并原子提升到 content-addressed object store；durable authority
分别持久化 installed/enabled/granted/effective refs、typed command receipts 和单调 activation generation，
并发布 generation subscription 与 exact invocation lease。它不自行执行 Plugin，不保存 credential，
也不解析 `SKILL.md` 或 MCP JSON-RPC。

## 公共契约

| Symbol | 当前职责 | 不承担 |
| --- | --- | --- |
| `PluginId` | 校验最长 128 bytes 的 `publisher/name` identity | display name、registry lookup |
| `PluginVersion` | strict SemVer exact version | range resolution、update policy |
| `PluginPackageDigest` | `sha256:<64 lowercase hex>` exact content identity | signature/trust |
| `PluginPath` | portable ASCII、slash-separated package-relative path | host absolute path authority |
| `PluginManifest::from_json` | strict JSON/schema/semantic validation | contribution content parsing |
| `EditorExtensionContribution` | 声明 exact executable program、Host RPC API、activation triggers 与 capability ceiling | 启动/监管进程、provider registry、RPC transport |
| `EditorExtensionActivationEvent` | 区分 startup/command/language/on-demand 等 bounded activation trigger | 把 provider kind 当成隐式 activation |
| `LocalPluginPackage::load` | 验证一个 exact root、digest 和所有 contribution path | copy/install/immutability |
| `LocalPluginCatalog::discover` | 读取一个 package 或目录下的直接 package children | recursive marketplace search |
| `PluginMarketplace::open` | 校验 host 注册 catalog、无链接 package path 与 exact package digest | 网络下载、publisher 签名、客户端宿主路径 |
| `PluginMarketplace::from_verified_remote` | 接收产品分发层已验证的签名 manifest、digest、统计与延迟 materializer | TUF/HTTP、自动下载、grant |
| `PluginMarketplacePackageMaterializer` | 安装时把一个 exact ref materialize 为 canonical `LocalPluginPackage` | 目录发现、enable、运行时执行 |
| `PluginMarketplaceService` | Marketplace install、staged update、rollback 与 profile request reconcile | 自动 grant、远端 catalog 同步、Workspace trust 决策 |
| `PluginPackageStore::install_local` | stage、复制、复验 digest 并原子 promote immutable object | enablement、grant、activation |
| `PluginPackageStore::read` | 按 exact installed ref 重新验证 object | authority lookup、版本选择 |
| `PluginPackageStore::activate` | 把 exact installed refs 解析为一个 activation generation | installed/enable authority、live publish |
| `PluginActivationSnapshot::resolve` | 拒绝重复 Plugin identity 并固定 immutable object handles | contribution runtime、profile resolution |
| `PluginActivationAuthority` | installed/enable/disable/uninstall record、replay、generation 与 live publish | contribution parsing、runtime composition |
| `PluginInvocationFence` / `PluginInvocationLease` | dispatch 前复核 exact digest/revision，disable/update commit 后 drain | Tool Registry replacement、MCP protocol |
| `InstalledPluginPackage::resolve_file` / `read_utf8_file` | 在 exact object root 内解析 regular file，并提供有界 UTF-8 读取 | 任意目录扫描、MCP/Skill 内容解释 |

`PluginManifest` 的 serde `Deserialize` 与 `from_json` 使用同一 semantic validation，不存在绕过
schema version、duplicate ID、credential reference 或 permission invariant 的反序列化入口。
programmatic mutation 后必须再次调用 `PluginManifest::validate`。

## 清单 v1 契约

当前 schema 固定：

- `schemaVersion == 1`；
- Plugin version 使用 SemVer；
- Skill、MCP server、Connector、asset 和可执行 Editor Extension 都有稳定 manifest-local ID；
- Connector 必须引用同一个 manifest 中已声明的 MCP server contribution；
- permission 是 `process/workspace/network` tagged value；
- network v1 只接受 exact lowercase DNS/IP host，不接受 scheme、port 或 wildcard；
- credential slot 只能引用已声明的 `skill:<id>`、`mcp:<id>`、`connector:<id>`、
  `asset:<id>` 或 `editorExtension:<id>`；
- unknown field、duplicate contribution/slot/permission/host 和非 namespaced metadata 均失败；
- Skill path 必须是包含 regular `SKILL.md` 的目录，MCP definition/process executable 必须是
  regular file，asset 可以是 regular file 或目录；
- `editorExtensions[]` 的 entrypoint 必须是包内 regular file，并有相同 `PluginPath` 的 exact
  `process` permission；runtime API 仅接受数值 `1`，entrypoint 和 manifest-local ID 都必须唯一；
- 每个 Editor Extension 的 `activationEvents` 与 capability ceiling 均为 non-empty、unique、bounded
  typed set。command/language/debug/task/test activation selector 不能越过对应 capability ceiling。

可执行声明示例：

```json
{
  "id": "review-runtime",
  "entrypoint": "bin/review-extension-host",
  "runtimeApiVersion": 1,
  "activationEvents": [
    { "type": "onCommand", "id": "acme.review.run" },
    { "type": "onLanguage", "id": "rust" },
    { "type": "onDemand", "capability": "testProfileProvider" }
  ],
  "capabilities": ["command", "languageProvider", "testProfileProvider"]
}
```

每个 entrypoint 都是包自己提供、可直接启动并实现 Zeta Host RPC v1 的程序；它不是交给通用
Node/WASM loader 解释的脚本。`zeta-editor-extension-host` supervisor 负责逐扩展隔离启动和故障监管。
`capabilities` 是 Extension Host 允许注册的最大 provider 集合，不代表这些 provider 已注册，也不代表
某次有副作用的调用已获批准。activation event 只回答何时请求激活；`startup` 和
`onDemand` 不会隐式增加 capability。`workspaceContains` 当前没有 Workspace-owned 的安全 scanner
consumer，因此 manifest v1 会按 unknown event 拒绝；不能静默忽略或由扩展进程自行遍历磁盘。

这里的 exact `process` permission 是“允许 supervisor 尝试启动这一条 package-relative path”的
activation ceiling，不是每次 provider invocation 都直接执行 entrypoint，也不绕过 invocation lease、
Workspace trust 或 broker policy。package ingestion 只证明 entrypoint 是包内 regular file；它不验证
Unix executable bit、Windows PE/扩展名、CPU ABI、代码签名或跨平台可运行性。当前 schema 也没有
per-platform artifact selector。supervisor 必须在目标平台 fail closed 地检查 launchability，失败时报告
activation failure，不能把 regular-file validation 描述为可执行性保证。

`PluginPath` 使用保守的跨平台子集 `[A-Za-z0-9._-]`。它拒绝 absolute path、反斜杠、空/`.`/`..`
segment、Windows device name、非 ASCII 名称、超过 32 层或 1024 bytes 的路径。这是 PL0
有意选择的 portability/security contract；扩大字符集前必须先固定 Unicode normalization 与
case-collision 规则。

## 包校验路径

```text
LocalPluginPackage::load
├─ validate_root
├─ digest::scan_and_digest
│  ├─ walk_directory
│  │  └─ reject link / hard link / special file / unsafe PluginPath / size limits
│  └─ hash_file
│     └─ domain + relative path length/path + file length/content
├─ PluginManifest::from_json
│  └─ PluginManifest::validate
└─ validate_contribution_paths
   ├─ require_entry
   ├─ require_contained
   └─ unique_location
```

安装存储路径是：

```text
PluginPackageStore::install_local
├─ create unique staging root
├─ copy_package_tree
│  └─ reject changed link/special-file/file identity
├─ LocalPluginPackage::load(staging)
├─ compare exact id/version/digest with source snapshot
├─ sync_directory_tree
└─ rename staging → objects/<sha256>
```

object 已存在时操作是幂等的，但仍重新加载并验证；source 在首次 validation 后被修改会失败，且不会
promote partial object。这个 store 当前只接受 explicit local-development package，built-in release 和
remote archive 由未来各自的 source/trust adapter 处理。

Digest 与 source root 无关，对 normalized relative path 和每个 regular file 的 bytes 敏感。
当前 ingestion limits：

| Limit | Value |
| --- | ---: |
| manifest | 1 MiB |
| single file | 16 MiB |
| total package bytes | 256 MiB |
| regular files | 10,000 |
| path depth | 32 segments |

`LocalPluginCatalog::discover` 若给定目录自身包含 `.zeta-plugin/`，只读取该 package；否则只检查
直接 children。相同 `(PluginId, PluginVersion)` 出现两次会使整个 catalog 失败，不能按扫描顺序
静默覆盖。

## 内部所有权与漂移信号

| Private symbol | Ownership | 修改时同步检查 |
| --- | --- | --- |
| `UncheckedPluginManifest` | serde shape 与 unknown-field rejection | manifest tests、schema example |
| `PluginManifest::validate` | 跨字段 semantic invariants | credential/contribution fixtures |
| `digest::walk_directory` | file count/type/link/path limits | package security tests、limits table |
| `digest::hash_file` | bytes 与 file-identity stability | digest fixtures、TOCTOU assumptions |
| `validate_contribution_paths` | contribution type、existence、containment | Skill/MCP/Editor Extension consumer contract |
| `reject_duplicate_exact_versions` | local catalog exact-version uniqueness | future resolver semantics |
| `PluginPackageStore::install_local` | staging/revalidation/atomic promotion boundary | authority commit、store recovery tests |
| `PluginActivationSnapshot::resolve` | exact package set 与 generation 的不可变发布边界 | profile resolver、consumer projection tests |
| `PluginActivationAuthority::apply` | CAS mutation、atomic authority persistence、publish-before-drain | App Server reconcile、MCP fence tests |
| `FileAuthorityPersistence` | bounded strict JSON、0600 staging、fsync + atomic rename | recovery tests、schema migration |
| `copy_package_tree` | copy-time entry and file-identity checks | archive ingestion、platform link semantics |

出现以下变化表示 ownership 漂移：

- crate 启动 process、解析 MCP protocol 或解释 `SKILL.md`；
- local catalog 自动安装/启用 package；
- signature 被当成 runtime approval；
- canonical host path、secret 或完整 file content 进入 public diagnostic；
- `zeta-config` 与 `zeta-plugins` 互相依赖。当前 `zeta-config` 的 Plugin request identity 仍是
  config-local representation；进入 App Server contract 时应显式转换，或先将获准的共享纯
  identity 下沉到 `zeta-protocol`。

## 失败语义与集成义务

`PluginErrorKind` 区分 source unavailable、unsafe package、invalid manifest、invalid
contribution 与 exact-version conflict。任何失败都不返回 partial package/catalog，也不改变
filesystem。error message 只包含稳定 identity、relative path 与 sanitized 原因。

`LocalPluginPackage` 捕获的是已验证的 source snapshot identity，source directory
仍可被外部修改。runtime consumer 不能把它当 immutable root；必须先通过 `PluginPackageStore`
复制、重新验证并从 content-addressed object root 绑定 contribution。

`PluginMarketplace::open` 的 root 只能由产品 host 注册。`PluginMarketplace::from_verified_remote`
只接受产品分发层已经验证的 discovery metadata，并把 package bytes 延迟到安装时通过
`PluginMarketplacePackageMaterializer` 获取；返回的 `LocalPluginPackage` 仍须与 signed manifest、ID、
version 和 digest 完全一致。Renderer 只能提交 Marketplace ID 与 exact package ref，不能提交宿主路径。

## 验证

```bash
cargo test --manifest-path Cargo.toml -p zeta-plugins
cargo clippy --manifest-path Cargo.toml \
  -p zeta-plugins --all-targets --no-deps -- -D warnings
bazel test //zeta-rs/plugins:plugins-unit-tests
```

当前测试覆盖严格/重复模式、身份/SemVer、凭据引用、权限、Editor Extension v1 API/activation/
capability/entrypoint、路径穿越/设备名/规范化、摘要确定性、缺失贡献、符号链接/硬链接、目录排序与
读取、精确版本冲突，以及 Marketplace digest mismatch、staged update 和 rollback grant 边界。

## 当前限制与扩展点

PL1 的 content store、durable installed/enabled/granted/effective authority、exact snapshot、live
activation publish、Marketplace ingestion 和 user-profile reconcile 已实现。Workspace request 只做
profile authority 的只读可用性判断，不从 Workspace 自动安装或授予权限；authority v1→v2 migration
会把旧 active package 保守迁移为 enabled + granted。当前 object directory 的只读性由
“不暴露可写根路径 + digest revalidation”保证，
尚未施加平台级 immutable flag，也没有 orphan staging startup recovery；失败 install commit 和 uninstall
会精确回收无引用 object。受信 host 可注册 materialized Marketplace root；公网 catalog 下载、
publisher signature/revocation 已由 `zeta-plugin-marketplace` 实现，package cache 与 content store 的 GC
authority 尚未实现；package-rooted MCP consumer
已位于 `zeta-mcp-extension`，不能反向并入本 crate。这些能力应在新的 private
`authority/resolution` modules 中接入，不扩大 loader/store 为隐式 enable manager。

本 crate 当前只让可执行 Editor Extension 进入既有 Plugin package、digest、enable/grant 和 invocation
fence 控制面；它不包含 Extension Host executable、RPC、crash supervisor、workspace trust broker 或
provider runtime。即使 manifest 与 exact process permission 均有效，安装也不会启动 entrypoint；Host
仍须在 dispatch 时复核 active generation/invocation lease，并把 capability ceiling 投影为拒绝默认的
broker policy。
