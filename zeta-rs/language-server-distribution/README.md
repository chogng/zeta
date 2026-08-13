# `zeta-language-server-distribution`

> 本 README 是 application-managed language-server 安装存储的 canonical contract。Server identity
> 与 executable discovery 见 [`language-server-catalog`](../language-server-catalog/README.md)，跨 crate
> 产品语义见 [`docs/lsp.md`](../../docs/lsp.md)，TUF consumer 见
> [`zeta-language-marketplace`](../language-marketplace/README.md)。

本 crate 拥有已下载 package 的路径校验、provider-supplied SHA-256 验证、private staging、原子发布、
version receipt、side-by-side update storage 与 durable activation authority。它不查询网络、不选择
release channel、不解压不可信 archive、不调用全局包管理器，也不删除旧版本。

## 公共接口与执行路径

| API | 当前职责 | 明确不做 |
| --- | --- | --- |
| `LanguageServerPackageFile` | 表达 traversal-free regular/executable package file | 跟随 symlink 或接受绝对路径 |
| `LanguageServerPackage` | 冻结 server/version/executable path/file set 并计算 deterministic SHA-256 | 声称 digest 来源可信 |
| `LanguageServerInstaller` | 验证 expected digest、写 staging、receipt 并原子发布 version directory | 覆盖已发布版本或改变 Config |
| `LanguageServerActivationAuthority` | 原子选择 exact installed version；每次 snapshot 重新扫描并校验 package digest | 下载 package、选择 runtime 或启动 server |
| `InstalledLanguageServer` | 不透明的已验证安装 receipt，以只读访问器交给 provider versioned entrypoint 与 digest | 允许调用方手工伪造或表示 server 已激活/initialize |

`InstalledLanguageServer::executable` 是 package manifest 声明的入口，不承诺它是可由 OS
直接启动的 native binary。例如 CSS provider 把该路径解释为 JavaScript 入口，并且只使用
Zeta 托管的 Node 运行它；distribution 仍然只负责 bytes、digest 和安装路径的可验证性。

```text
trusted server-specific provider
  → immutable LanguageServerPackage + expected SHA-256
  → validate identity / relative paths / duplicate paths / declared executable
  → verify deterministic package digest
  → <install-root>/.staging/<unique>/ files + installation.json
  → atomic rename to <install-root>/<server>/<version>
  → LanguageServerActivationAuthority atomically selects exact receipt
  → restart reopens and rehashes every active installation
```

关键 private symbols：

- `validate_identity` / `validate_relative_path` 在 server/version 与 package traversal 进入 filesystem 前
  拒绝空值、`.`、`..`、绝对路径和 parent component；
- `ensure_directory` 拒绝把 install root、server root 或 staging root 的 symlink 当作真实目录；
- `StagingGuard` 在写入、权限或 receipt 失败时清理未发布 staging；
- `InstallationReceipt` 绑定 server、version、executable relative path 和 digest；
- `installed_from_receipt` 重新核对 receipt、每个文件内容和 executable mode，使完全相同的重复安装
  幂等，并对 receipt 伪装或安装目录篡改 fail closed；
- `set_executable` 只在 Unix 投影 package executable bit，不承担平台签名验证。

如果本 crate 开始选择 URL、release channel 或 workspace enablement，表示 distribution provider/config
ownership 漂移；如果 catalog 开始写安装目录或校验 package bytes，表示 discovery 与 storage 被重新耦合。

## 失败语义、测试和限制

Digest mismatch 在创建 staging 前失败。写入后的任何失败由 guard 清理；已发布目录从不覆盖。更新通过
安装新 version directory 并原子切换 activation receipt 完成，旧版本保留；回退通过
`LanguageServerActivationAuthority::activate` 重新选择已经验证的 exact version 完成。

```bash
cargo test --manifest-path Cargo.toml -p zeta-language-server-distribution
cargo clippy --manifest-path Cargo.toml -p zeta-language-server-distribution --all-targets -- -D warnings
```

测试覆盖 traversal rejection、digest mismatch 不发布、version side-by-side、旧版本保留、同 package
重复安装幂等、activation 跨进程重开，以及既有版本被篡改后即使 receipt 未变也拒绝复用或启动。

当前限制：

- ✅ verified package storage、atomic staging、receipt 与 side-by-side update/rollback 基础；
- ✅ durable exact-version activation、启动时 re-open/re-hash 与篡改 fail closed；
- 委托：TUF release metadata、有界 archive、compatibility probe 和安装确认由
  `zeta-language-marketplace`、App Server 与 Desktop 分别拥有；
- 尚未完成：下载进度/取消、未激活安装与旧版本 GC；
- 扩展约束：本 crate 继续只消费已解析 package，不拥有网络、UI、provider 或 workspace enablement。
