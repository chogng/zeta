# `zeta-add-dir`

> 本 README 拥有主工作目录之外的持续目录访问作用域、来源生命周期和配置贡献策略的实现契约。
> 跨 crate 产品语义由
> [`docs/workspace-security.md`](../../docs/workspace-security.md#工作目录附加目录与-cd) 维护；
> 单 root identity 与 trust token 由
> [`zeta-rs/workspace/README.md`](../workspace/README.md) 维护；Instructions/Skills/Agents artifact 与
> Import/source registration 的边界由
> [`docs/agent-customizations.md`](../../docs/agent-customizations.md) 维护。

`zeta-add-dir` 把“当前项目是什么”与“还允许访问哪些目录”分开。一个
`DirectoryAccessScope` 始终只有一个主工作目录；`AdditionalDirectory` 只扩大文件访问，不会
成为第二个项目或自动获得完整项目配置语义。

## 快速理解

| 输入来源 | 文件访问 | 配置贡献 | 生命周期 |
| --- | --- | --- | --- |
| `LaunchArgument` | 主目录 + 附加目录 | allowlisted project contributions | 本次启动 |
| `SessionCommand` | 主目录 + 附加目录 | allowlisted project contributions | 当前会话 |
| `PersistentConfiguration` | 主目录 + 附加目录 | ❌，`FileAccessOnly` | Config declaration 有效期间 |

Host compatibility 可以把 transient source 的 instruction contribution 从 `Exclude` 切到
`Include`；它不改变目录访问范围。多个 source 可以同时保留同一个 canonical root，移除其中一个
source 不会撤销其他 source 仍然提供的访问。

## 所有权与非职责

当前 crate 负责：

- 主工作目录与附加目录的角色分离；
- canonical root identity 去重和稳定 path ordering；
- 同一 root 的多个 `AdditionalDirectorySource` 生命周期；
- idempotent add/remove mutation result；
- 从 active source 与 `AdditionalInstructionsPolicy` 解析 contribution policy；
- Skills、Agent definitions、Plugin declaration 与 instruction file 的精确 allowlist。

当前 crate 不负责：

- directory picker、CLI flag、slash command、RPC 或 Renderer UI；
- Config TOML schema、persistence revision 或 source precedence；
- host trust decision、macOS privacy consent、sandbox profile 或 OS file handle；
- Files/Search runtime rebuild、Watcher registration 或 Terminal cwd；
- `zeta-agent-import` 的外部布局发现、preview、apply 或 migration。

如果本 crate 开始依赖 App Server、Config、Search、Agent Import、Desktop 或 protocol DTO，说明
领域边界已经漂移。上述 crate 只能依赖这里的纯 scope contract，并在自己的 adapter 中完成执行。

## 公共契约

| Symbol | 调用方责任 | crate 保证 |
| --- | --- | --- |
| `DirectoryAccessScope::new` | 传入 host 已建立的主 `WorkspaceRoot` | 创建只含主目录的 scope |
| `add_directory` | 传入 canonical root 与 named source | 拒绝主目录、按 canonical identity 去重、保留多 source |
| `remove_directory` | 指定 exact root 与 source | 只释放该 source，最后一个 source 消失时移除目录 |
| `AdditionalDirectory::sources` | 检查 active lifetime | stable sorted source set |
| `contribution_policy` | 显式选择 instruction compatibility policy | persistent source 始终 file-access-only |
| `DirectoryScopeMutation` | 解释 idempotent mutation | 不用 ambiguous `bool` 表示变化 |

`AdditionalDirectorySource` 不能被压缩为“persistent”布尔值。`LaunchArgument` 与
`SessionCommand` 当前共享 contribution allowlist，但生命周期和撤销事件不同，必须保留 exact
source。相同 root 同时来自 session 与 Config 时，session source 允许临时 contribution；session
撤销后，Config source 继续保留文件访问，但 contribution policy 退回 `FileAccessOnly`。

## 配置贡献

| Policy | 允许的 contribution |
| --- | --- |
| `FileAccessOnly` | 无 |
| `AllowlistedProjectContributions` | Skills、Agent definitions、`enabledPlugins`、`extraKnownMarketplaces` |
| `AllowlistedProjectContributionsWithInstructions` | 上述内容，加 project/local instruction 与 instruction rules |

Policy 描述“允许发现什么”，不是“已经加载、批准或执行”。Skill manager、Plugin authority 和
Agent Import 仍需执行各自 validation、trust 与 activation contract。

## 真实调用关系

```text
host-selected working directory
  → WorkspaceRoot::open
  → DirectoryAccessScope::new

host-validated add-dir intent
  → WorkspaceRoot::open
  → DirectoryAccessScope::add_directory(root, source)
  → AdditionalDirectory::contribution_policy(instructions)
  → App Server adapters rebuild Files / Search / contribution projections
```

`/cd` 不调用 `add_directory`。Host 应建立新的主 `WorkspaceRoot` 和新的
`DirectoryAccessScope`，再按产品生命周期规则决定哪些附加 source 仍然有效。

## 失败与撤销

- 主工作目录再次作为附加目录加入时，返回
  `DirectoryScopeError::WorkingDirectoryCannotBeAdditional`。
- 同一 canonical root 与 source 重复添加时，返回 `AlreadyPresent`。
- 同一 canonical root 增加第二个 source 时，返回 `AddedSource`。
- 删除不存在的 root/source 时，返回 `NotPresent`。
- 删除一个 source 后仍有其他 source 时，返回 `RemovedSource`；最后一个 source 消失时返回
  `RemovedDirectory`。

本 crate 的 mutation 不执行 I/O，也不证明 runtime 已成功切换。App Server 必须先完成 trust、
watcher 和 consumer preparation，再原子发布新的 effective scope；失败时保留旧 scope。

## 验证与修改影响

```bash
cargo test --manifest-path Cargo.toml -p zeta-add-dir
bazel test //zeta-rs/add-dir:add-dir-unit-tests
```

`scope_tests.rs` 覆盖主目录拒绝、canonical alias 去重、多 source retention、
file-access-only Config policy、transient allowlist 和逐 source revocation。修改 source、
contribution 或 mutation semantics 时，必须同步检查 Workspace security、Search root-qualified
result、Agent Import adapter 与未来 Config/RPC schema。

## 当前限制

当前 crate 只有纯领域模型，尚未接入 App Server、Config、Files、Search、CLI 或 Desktop。
`add-dir`、`/cd` 与 `additionalDirectories` 都不是当前可用产品功能。OS privacy denial、
root-qualified URI、跨启动 restoration 和 runtime rollback contract 仍需在各 authority owner
中实现。
