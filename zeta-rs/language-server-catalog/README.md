# `zeta-language-server-catalog`

> 本 README 是语言服务器发现与 resolved definition 的 crate-level canonical contract。运行时
> lifecycle 见 [`zeta-language-service`](../language-service/README.md)，协议 contract 见
> [`zeta-lsp`](../lsp/README.md)，跨 crate 语义见 [`docs/lsp.md`](../../docs/lsp.md)。

`zeta-language-server-catalog` 拥有内置 server identity、用户启用意图、execution policy gate、
冻结候选的校验与 canonicalization，以及已验证 package 到 resolved definition 的 provider
绑定。它不启动进程、不读取 editor 文档、不执行 LSP、不下载 server，也不决定
workspace trust。

## 所有权与公共接口

| API / type | 当前职责 | 明确不做 |
| --- | --- | --- |
| `LanguageServerCatalog` | 保存内置 server 与 preference，按 workspace 生成一次冻结 resolution | 持有 live client 或自动重启 |
| `LanguageServerPreference` / `LanguageServerMode` | 表达 Disabled、Automatic、Enabled 和 authoritative executable override | 用布尔值混合启用与发现语义 |
| `LanguageServerExecutionPolicy` | 接收产品宿主已经作出的 process allow/disallow 决策 | 自行读取或持久化 workspace trust |
| `LanguageServerExecutableCandidates` | 注入有优先级的冻结候选；`InstallContext` 是当前实现 | 搜索时启动或 probe 进程 |
| `LanguageServerCatalogResolution` | 同时返回 resolved definitions 与每个内置 server 的 availability | 表示 server 已经 initialize |
| `LanguageServerDefinition` | 冻结唯一 route、canonical executable command 和 initialize options | 在 runtime 内重新查询 PATH |
| `LanguageServerProvider` / `LanguageServerProviderRegistry` | 把已验证、已安装的 server 包和运行时绑定为稳定 language route 与 definition | 下载、验签、启动进程或监督重启 |
| `ManagedNodeRuntime` | 冻结 canonical Node-compatible executable；Desktop 使用 Electron run-as-Node，其他 package 使用 standalone Node，并生成 clean-environment command | 回退 host `PATH` 或允许 language pack 携带 Node |
| `CssLanguageServerProvider` | 用共享 Node-compatible runtime 运行 verified CSS package 入口，route `css`/`less`/`scss` | 复制 LSP client/supervisor 或解释 Marketplace metadata |

当前内置项包括 `rust-analyzer → rust`、`vscode-json-language-server --stdio → json/jsonc` 和
`bash-language-server start → shellscript`。CSS 是独立 provider，不进入 PATH built-in 列表。Native/App Server 从
Config snapshot 映射各自的持久化 preference；
未配置时使用 `Disabled`，需要显式选择 `Automatic` 或 `Enabled` 才会生成 definition。允许执行且冻结
PATH 中存在可执行文件时生成 definition；否则保持无 server。显式 override 是 authoritative，失效时不会回退 PATH。

## 执行路径、失败和扩展

```text
Native composition
├─ InstallContext::current → frozen host PATH
├─ workspace process policy
└─ LanguageServerCatalog::resolve
   ├─ Disabled / disallowed → status only
   ├─ explicit path → validate exactly that path
   └─ automatic → candidates in source order
      → canonicalize → regular file → executable permission
      → LanguageServerDefinition
      → zeta-language-service

verified CSS package composition
├─ InstalledLanguageServer::executable
├─ Desktop: ZETA_ELECTRON_RUN_AS_NODE_PATH → exact Electron executable
├─ other packaged hosts: InstallContext::bundled_resource("node/bin/node[.exe]")
└─ CssLanguageServerProvider::definition
   ├─ packaged → selected runtime + package entrypoint + --stdio + clean environment
   │  └─ Electron source only: ELECTRON_RUN_AS_NODE=1
   └─ explicit override → authoritative native executable
      → LanguageServerDefinition
      → zeta-language-service
```

关键私有符号：

- `BuiltinServer` 绑定每个内置 identity、executable、language route 和启动参数；`resolve_builtin`
  统一执行 preference、policy 和 executable gate。
- `valid_executable` 是候选进入 resolved definition 前的唯一文件校验 gate。
- `has_executable_permission` 保持 Unix executable-bit 与其他平台文件语义分离。
- `canonical_regular_file` / `canonical_executable` 是 provider 输入进入 definition 前的
  canonical file gate；`ManagedNodeRuntime::command_for_script` 是 Node 启动环境的唯一 owner。
- `CssLanguageServerProvider::definition` 是 CSS package/native override 分支的唯一绑定点；
  `LanguageServerProviderRegistry` 拒绝重复 identity，避免静默替换 route owner。

如果本 crate 开始持有 `LanguageServerClient`、child process、document revision 或 diagnostics，表示 runtime ownership
向 catalog 漂移；如果 language-service 开始读取 PATH、选择 executable 或解释用户 preference，表示
发现策略向 runtime 漂移。增加内置 server 时必须同时定义稳定 name、language IDs、启动参数、候选
来源和不可用状态测试，不能把任意 workspace command 直接视为受信 definition。

失败语义：静态 definition/name 无效会返回 `LanguageServerCatalogError`；candidate 缺失、不是普通
文件、无法 canonicalize 或没有 Unix executable bit 都投影为 `ExecutableUnavailable`，不造成整个
产品启动失败。`ExecutionDisallowed` 在访问候选前返回。

## 测试与当前限制

```bash
cargo test --manifest-path Cargo.toml -p zeta-language-server-catalog
cargo clippy --manifest-path Cargo.toml -p zeta-language-server-catalog --all-targets -- -D warnings
```

测试覆盖三项 built-in 的 Automatic PATH resolution/route/launch arguments、execution policy gate、
失效 explicit override 不回退，以及 CSS package 的 standalone/Electron runtime command、clean
environment、native override 和 duplicate provider gate。

当前限制：

- ✅ Rust server 的 frozen PATH 发现、canonical executable 和 product-neutral availability；
- ✅ `zeta-config` 持久化 mode/path、App Server typed mutation 与 Native 三项 server Settings selector；
- ✅ JSON/JSONC、Shell server definitions 与持久化 mode/path 映射；
- ✅ 独立 `zeta-language-server-distribution` 提供 checksum 验证、原子 staging、side-by-side 安装和回滚基础；
- ✅ verified CSS package provider、managed Node-compatible runtime 启动命令、App Server provider 组合点与 native override；
- 尚未完成：Marketplace CSS target 的通用 TUF 下载/解压 adapter、compatibility probe、
  用户确认/安装 UI 和从安装 receipt 自动构建 provider registry；
- 尚未完成：组织策略或更细的 per-server executable grant。
