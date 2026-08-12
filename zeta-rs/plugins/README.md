# `zeta-plugins`

> 本 README 是 Plugin manifest、identity、local package validation 与 discovery 的当前实现
> 契约。跨 crate 的安装、activation、权限和运行时演进由
> [`docs/plugins.md`](../../docs/plugins.md) 维护；Connector account/lifecycle 由
> [`docs/connectors.md`](../../docs/connectors.md) 维护。

`zeta-plugins` 当前完成 PL0，并实现 PL1 的 package-store 与 activation snapshot vertical slice：严格解析 declarative
Plugin v1 package，验证 package-relative path 与本地文件树，计算确定性 SHA-256 digest，提供只读
local-development catalog，并能把一个已验证本地包复制、复验后原子提升到 content-addressed object
store；调用方可把 exact installed package 解析为 generation-bound `PluginActivationSnapshot`。它不写
installed/enable authority record、不自行选择或执行 Plugin，不保存 grant/credential，也不解析
`SKILL.md` 或 MCP JSON-RPC。

## 公共契约

| Symbol | 当前职责 | 不承担 |
| --- | --- | --- |
| `PluginId` | 校验最长 128 bytes 的 `publisher/name` identity | display name、registry lookup |
| `PluginVersion` | strict SemVer exact version | range resolution、update policy |
| `PluginPackageDigest` | `sha256:<64 lowercase hex>` exact content identity | signature/trust |
| `PluginPath` | portable ASCII、slash-separated package-relative path | host absolute path authority |
| `PluginManifest::from_json` | strict JSON/schema/semantic validation | contribution content parsing |
| `LocalPluginPackage::load` | 验证一个 exact root、digest 和所有 contribution path | copy/install/immutability |
| `LocalPluginCatalog::discover` | 读取一个 package 或目录下的直接 package children | recursive marketplace search |
| `PluginPackageStore::install_local` | stage、复制、复验 digest 并原子 promote immutable object | enablement、grant、activation |
| `PluginPackageStore::read` | 按 exact installed ref 重新验证 object | authority lookup、版本选择 |
| `PluginPackageStore::activate` | 把 exact installed refs 解析为一个 activation generation | installed/enable authority、live publish |
| `PluginActivationSnapshot::resolve` | 拒绝重复 Plugin identity 并固定 immutable object handles | contribution runtime、profile resolution |
| `InstalledPluginPackage::resolve_file` / `read_utf8_file` | 在 exact object root 内解析 regular file，并提供有界 UTF-8 读取 | 任意目录扫描、MCP/Skill 内容解释 |

`PluginManifest` 的 serde `Deserialize` 与 `from_json` 使用同一 semantic validation，不存在绕过
schema version、duplicate ID、credential reference 或 permission invariant 的反序列化入口。
programmatic mutation 后必须再次调用 `PluginManifest::validate`。

## 清单 v1 契约

当前 schema 固定：

- `schemaVersion == 1`；
- Plugin version 使用 SemVer；
- Skill、MCP server、Connector 和 asset 都有稳定 manifest-local ID；
- Connector 必须引用同一个 manifest 中已声明的 MCP server contribution；
- permission 是 `process/workspace/network` tagged value；
- network v1 只接受 exact lowercase DNS/IP host，不接受 scheme、port 或 wildcard；
- credential slot 只能引用已声明的 `skill:<id>`、`mcp:<id>` 或 `asset:<id>`；
- unknown field、duplicate contribution/slot/permission/host 和非 namespaced metadata 均失败；
- Skill path 必须是包含 regular `SKILL.md` 的目录，MCP definition/process executable 必须是
  regular file，asset 可以是 regular file 或目录。

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
| `validate_contribution_paths` | contribution type、existence、containment | Skill/MCP consumer contract |
| `reject_duplicate_exact_versions` | local catalog exact-version uniqueness | future resolver semantics |
| `PluginPackageStore::install_local` | staging/revalidation/atomic promotion boundary | authority commit、store recovery tests |
| `PluginActivationSnapshot::resolve` | exact package set 与 generation 的不可变发布边界 | profile resolver、consumer projection tests |
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

`LocalPluginPackage` 捕获的是已验证的 local-development snapshot identity，source directory
仍可被外部修改。runtime consumer 不能把它当 immutable root；必须先通过 `PluginPackageStore`
复制、重新验证并从 content-addressed object root 绑定 contribution。

## 验证

```bash
cargo test --manifest-path Cargo.toml -p zeta-plugins
cargo clippy --manifest-path Cargo.toml \
  -p zeta-plugins --all-targets --no-deps -- -D warnings
bazel test //zeta-rs/plugins:plugins-unit-tests
```

当前测试覆盖严格/重复模式、身份/SemVer、凭据引用、权限、路径穿越/设备名/规范化、摘要确定性、
缺失贡献、符号链接/硬链接、目录排序与读取，以及精确版本冲突。

## 当前限制与扩展点

PL1 的 local content store 与调用方选择后的 activation snapshot 已实现；authority command/recovery、
profile enablement、grant 与 live activation publish 尚未实现。当前 object directory 的只读性由
“不暴露可写根路径 + digest revalidation”保证，
尚未施加平台级 immutable flag，也没有 orphan staging startup recovery。PL2+ 的 MCP activation、
registry、signature、update、rollback 和 GC authority 也尚未实现；package-rooted MCP consumer
已位于 `zeta-mcp-extension`，不能反向并入本 crate。这些能力应在新的 private
`authority/resolution` modules 中接入，不扩大 loader/store 为隐式 enable manager。
