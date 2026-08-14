# zeta-marketplace-manager

`zeta-marketplace-manager` 是 Zeta 进程内、按配置档案隔离的 Marketplace package 生命周期所有者。
它是链接进 App Server 的 library crate，不是独立可执行程序，也不属于远端 Marketplace 仓库。

跨 crate 的权威所有权由 [Marketplace 接入架构](../../docs/marketplace-integration.md) 维护；本文只拥有
本 crate 的精确存储与生命周期契约。

## 所有权

```text
App Server Marketplace RPC
  → MarketplaceManager
      ├─ MarketplaceRegistryClient: remote search/get/verified download
      ├─ Store: local artifacts + atomic manager-state.json
      ├─ installations: install/update/uninstall/listInstalled
      └─ runtime leases: acquire/release/openResource
```

| 职责 | Manager |
| --- | --- |
| 远端 TUF、catalog、ZIP 与下载 URL | ❌ 委托给注入的 registry client |
| 本地 artifact materialization 与 digest 复核 | ✅ |
| durable immutable installation records | ✅ |
| 更新与延迟卸载 | ✅ |
| capability references、leases 与 opaque resource reads | ✅ |
| permission、OAuth、activation 与 execution | ❌ 属于产品 runtime |

## 关键私有符号

| 符号 | 契约 |
| --- | --- |
| `MarketplaceManager::open` | 打开一个配置档案根，并注入远端 registry port |
| `MarketplaceManager::install_downloaded` | 从 verified payload 建立 immutable installation/capability identity |
| `Store::materialize` | 复制到 Manager-owned staging，重算 normalized digest，并原子持久化 artifact |
| `DurableState` / `InstallationRecord` | `manager-state.json` 中权威的本地 installation projection |
| `RuntimeState` / `LeaseRecord` | 只在进程内存在的 lease；重启时不持久化 |
| `capability_reference` | 从 installation + capability kind + local ID 派生无碰撞 opaque identity；open 会迁移旧记录 |
| `MarketplaceManager::{generation,subscribe}` | 向可信进程内 consumer 发布已提交的 installation-state 变化 |
| `MarketplaceManager::local_capability_sources` | 在可信进程内 runtime 收到私有 host handle 前重验完整 immutable artifact |
| `activation::acquire_spec` | 把已存 capability 转成 path-free Skill/MCP/Connector/Theme/Language/Executable activation contract |
| `activation::open_resource` | 使用受 lease 授权的 bounded safe path 读取 opaque resource |

Installation 不可变。更新 A 时先安装 B，再立即删除 A；若 A 仍有 lease，则标为
`pendingRemoval`。重启会丢弃陈旧 runtime lease，并完成已持久化的 pending removal。卸载当前不会
garbage-collect 无引用 artifact；保留的 artifact 充当 verified local cache。

以下代码意味着架构漂移：启动 Manager 子进程、接受 remote cache path、通过 DTO 暴露 artifact path、
在本 crate 解析 TUF/catalog metadata，或在 install/acquire 内激活 capability。

## 验证

```bash
cargo test -p zeta-marketplace-manager
cargo clippy -p zeta-marketplace-manager --all-targets -- -D warnings
```

对于由 `../marketplace/tools/marketplace-tool` 生成的 distribution，跨仓 smoke path 为：

```bash
cargo run -p zeta-marketplace-manager --example local_distribution_smoke -- \
  <distribution-root> <trusted-root.json> <temporary-state-root>
```

当前 capability acquisition 支持 Skill、remote 或 packaged-stdio MCP、Connector、Theme、Language
与 Executable contract。通用 `asset` activation 仍不支持：package-family adapter 必须把 portable
asset 规范化为领域 capability；可信本地 consumer 也可以解释可选的 signed product sidecar，但不能
把它变成通用 Marketplace schema。
