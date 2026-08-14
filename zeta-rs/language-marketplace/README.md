# `zeta-language-marketplace`

> 本 README 是签名语言包远程消费实现的 canonical contract。跨进程的产品语义和用户确认流程见
> [`docs/lsp.md`](../../docs/lsp.md)；跨 package family 的 source 注册、共享验证、领域投影和失败隔离见
> [`docs/marketplace-integration.md`](../../docs/marketplace-integration.md)；安装存储和 provider adapter 分别由
> [`zeta-language-server-distribution`](../language-server-distribution/README.md) 与
> [`zeta-language-server-catalog`](../language-server-catalog/README.md) 拥有。

本 crate 拥有 TUF metadata refresh、签名 catalog 解析、兼容性判定、精确 package 下载、有界 ZIP
解压和 Marketplace v1 package digest 验证。它不拥有用户界面、Node 选择、provider registry、LSP
process 或编辑器协议。

## Crate 边界

| 能力 | 本 crate | 直接协作者 |
| --- | --- | --- |
| TUF root pin、过期检查、delegated publisher role | ✅ | `tough`、host product config |
| `id/version/digest/manifest` 的 signed binding | ✅ | Marketplace publisher |
| 用户确认和展示 | ❌ | Desktop Workbench |
| side-by-side install 和 durable activation | 委托 | `zeta-language-server-distribution` |
| server ID → provider、共享 Node runtime | ❌ | `zeta-language-server-catalog` |
| LSP 进程与请求 | ❌ | `zeta-language-service`、`zeta-lsp` |

## 文件与职责

| 文件 | 关键符号 | 职责 |
| --- | --- | --- |
| `remote.rs` | `RemoteLanguageMarketplace`、`snapshot_from_repository`、`collect_entries` | refresh/cache TUF repository，列出并 materialize exact entry |
| `model.rs` | `catalog_entries`、`LanguageMarketplaceEntry` | 校验 schema 1/2 manifest、server route 和 consumer SemVer；静态 language asset package 不冒充 server entry |
| `archive.rs` | `extract`、`verify_package`、`language_server_package` | 有界解压、Marketplace tree digest、distribution handoff |
| `transport.rs` | `MarketplaceTransport` | 只允许 HTTPS，通过共享 `HttpClient` 读取 metadata/target |
| `error.rs` | `LanguageMarketplaceErrorKind` | 对 App Server 暴露不含远程 body/本地路径的稳定错误分类 |

## 调用路径

```text
RemoteLanguageMarketplace::sync
  → RepositoryLoader + MarketplaceTransport
  → snapshot_from_repository
     → validate_language_index / read_revocations
     → collect_entries → catalog_entries

RemoteLanguageMarketplace::install
  → materialize_exact（再次刷新并精确匹配 entry）
  → read_target（TUF length/hash verification）
  → archive::extract（路径、entry、单文件、总大小限制）
  → archive::verify_package（marketplace-package-v1 digest）
  → LanguageServerInstaller::install_verified
  → LanguageServerActivationAuthority::activate
```

安装不能直接使用 UI 传来的 manifest 或 target path。`materialize_exact` 必须在当前未过期 TUF
snapshot 中重新找到 `marketplaceId/packageId/version/digest/serverId` 的同一条目；catalog revision
冲突由 App Server 在下载前拒绝。

## 验证与失败语义

- metadata base、targets base 必须为无 userinfo/query/fragment 且以 `/` 结尾的 HTTPS URL；trusted
  root 最大 1 MiB。
- TUF 使用 `ExpirationEnforcement::Safe` 与 `Limits::default()`；网络 transport 失败时只可打开当前
  未过期的完整缓存，签名、rollback 或 parse 失败不降级。
- delegated role 必须是 `publishers/<publisher>`，target 必须是
  `packages/<publisher>/<name>/<version>.zip`。
- ZIP 最大 64 MiB、10,000 个 entry、单文件 16 MiB、展开总量 256 MiB；拒绝 symlink、encrypted
  entry、duplicate path、反斜线和 parent traversal。
- 解压后的 regular files 再按 Marketplace `marketplace-package-v1\0` 算法计算 digest，并与 signed
  `marketplacePackage.packageDigest` 以及 signed file count/size 同时比较。
- schema 2 executable 必须明确 `node` 或 `direct` runtime；schema 1 的 `LegacyUnspecified` 只有被
  product adapter 识别的 server ID 才能通过最终 compatibility probe。

`LanguageMarketplaceErrorKind` 区分无效配置、不可信 metadata、分发不可用、不安全 package、cache
不可用、不兼容和 activation 失败。错误消息不携带下载响应正文、URL credential 或 host path。

## 集成义务

Host 必须提供 product-pinned root、consumer ID/version、profile-scoped cache 和共享有界
`HttpClient`。App Server 必须先向 UI 返回 `LanguageMarketplaceCompatibility`，仅在用户确认后提交 exact
install command；即使 UI 出错，install 仍会在 Rust 端重新执行兼容性与 exact-snapshot 校验。

新增 runtime 或 server family 时，应先扩展 Marketplace schema 与 catalog provider adapter，不得在
本 crate 启动进程或读取 ambient `PATH`。出现 Node discovery、LSP JSON-RPC 或 Renderer dialog 代码即
表示 ownership 漂移。

## 测试

```bash
cargo test -p zeta-language-marketplace
```

`model_tests.rs` 覆盖 schema 1 CSS route 和 consumer SemVer；`archive_tests.rs` 覆盖有界解压、signed
statistics 与 traversal；`remote_distribution_tests.rs` 构造真实 signed TUF repository，验证 catalog
不会预下载 package、exact install、activation 和 tamper rejection。跨 crate activation/reopen 测试位于
`zeta-language-server-distribution/src/activation_tests.rs`。

## 当前限制与扩展点

当前 product adapter 只支持 `css-language-server`，并把 schema 1 的该入口视为 Node script。通用
consumer 已能解析 schema 2 `node/direct` runtime，但 direct native provider 必须先在 catalog crate
实现，才能被最终 compatibility probe 接受。已安装但未激活的 orphan version 不进入 UI；它不会被
执行，后续可增加独立 cache/install garbage collection authority。
