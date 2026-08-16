# `zeta-config`

> 本 README 负责 crate 当前实现契约。跨领域 source、scope、safe point 与长期演进由
> [`docs/config.md`](../../docs/config.md) 规范。

`zeta-config` 是普通、非 secret 用户配置的 TOML authority，也是严格 Workspace TOML document 的
typed parser/resolver。SQLite 只保存 transaction metadata 和 exact receipt。它不拥有这些内容：
UI/device preference、credential bytes、Plugin package/activation、live MCP connection、Hook
execution、Skill 正文、Session/Thread 或 Core execution。

## 当前 API 与所有权

| Symbol | 职责 |
| --- | --- |
| `ConfigStore` | 原子替换 User `config.toml`，并在共享 `state.sqlite3` 中协调 revision/generation 与 exact command receipt |
| `ConfigCommandRequest` / `UserConfigCommand` | typed mutation、expected revision 与 retry identity |
| `ResolvedConfigSnapshot` | User authority 的 immutable runtime input |
| `WorkspaceConfigStore` | strict-read `.zeta/config.toml`，不写 Workspace 文件 |
| `WorkspaceTrustConfig` | 按 opaque `WorkspaceTrustId` 持久化 User 的 Restricted/Trusted 决策，并可保留仅供管理页展示的 canonical root metadata；缺失时 fail closed |
| `UserExecPolicyConfig` | 持久化 typed User rules；通过 `UpsertExecPolicyRule` / `RemoveExecPolicyRule` 原子变更 |
| `WorkspaceExecPolicyConfig` | strict-read Workspace restrictions；validation 禁止 `AllowUnsandboxed` |
| `compose_exec_policy` | 将 trusted Host/Organization layers、User rules 与 Workspace restrictions 组合成 immutable snapshot |
| `LanguageServersConfig` / `LanguageServerConfig` | 持久化 stable server ID 对应的 Disabled/Automatic/Enabled 与可选绝对 executable override |
| `resolve_scoped_config` | User + Workspace 的受限 merge、provenance 与 diagnostic |
| `ConfigChange` | metadata commit 后的 revision/generation signal，包括 TOML 外部编辑与其他 connection 的提交 |

`UserConfigDocument` 当前包含 Agent defaults、Provider map、standalone MCP declaration、Skill
source/enablement、exact Plugin request、declarative Hook、language-server preference，以及 execution-policy
rules 和 Workspace trust decision。Trust key
由 host 对 canonical root 生成，`roots` 不保存本地路径；管理页使用同一 document 中不参与授权的
`rootPaths` 展示 metadata，旧记录可能没有路径。User decision 不能冒充 organization policy 或
host configuration。Plugin request 不安装或授权 package；Hook declaration 不执行 process。
Theme/UI preference 不在本 crate；Desktop device configuration 是独立 authority。

## Durable 路径

```text
ConfigStore::apply
→ BEGIN IMMEDIATE
→ exact command receipt lookup
→ expected ConfigRevision compare
→ apply typed UserConfigCommand
→ validate full UserConfigDocument
→ atomic replace config.toml when consumer-visible value changed
→ update config_metadata digest/revision/generation
→ insert config_command_receipts
→ COMMIT
→ publish ConfigChange
```

相同 command ID + 相同 expected revision/payload 返回原 receipt；相同 ID 配不同输入返回
`CommandConflict`。No-op 会保存 receipt，但不推进 revision/generation，也不发布 change。

`store_schema` 拥有 `config_metadata`、`config_command_receipts` 和 component migration gate；
metadata 不含 document 正文。`store_file` 拥有 strict TOML、semantic digest 与 temp-file rename。
`store_monitor` 同时观察 TOML semantic change 与 SQLite `data_version`，并按 revision/generation
去重。连接启用 WAL、foreign keys、`synchronous=FULL` 与 5 秒 busy timeout。

## Resolution 与运行时边界

`WorkspaceConfigStore` 只接收 host 提供的 `WorkspaceId`、文件 path 与 observed content；
Workspace 文档不能选择 namespace、credential binding 或 grant。`resolve_scoped_config` 当前只让
Workspace preferred model 在 User 已配置对应 Provider 时覆盖，并把 MCP/Skill/Plugin/Hook 内容
保留为 pending intent；execution-policy rules 保留为只收紧 intent，并由 App Server 与 Host layer
组合后在 tool safe point 激活。

Workspace trust 是另一条解析轴：`.zeta/config.toml` 不能声明 `workspaceTrust`；只有 User
`config.toml` 的 `WorkspaceTrustConfig`、organization policy 或 trusted host composition 能产生
信任来源。App Server 对 client 请求的 `workspace/switch` 在切换安全点重新读取 User snapshot，
按目标根的 `WorkspaceTrustId` 授权；未记录的根保持 Restricted。进程启动时由 trusted host
明确固定的初始根仍标记为 `HostConfiguration`，不等同于 client 后续选择的任意根。
User trust 从 Trusted 变为 Restricted 时，Config change 同时触发 App Server 撤销当前
root-bound lease、移除执行型服务并中断活跃 Turn；filesystem 与 watcher 保留。
拥有 `workspaceTrustHost` capability 的 Desktop host 还可通过 `workspace/trust/list`、
`workspace/trust/set` 与 `workspace/trust/forget` 管理这些 User decisions；管理 RPC 不改变
active Workspace，且由 App Server 负责 identity/canonicalization。

App Server 在 model safe point 读取 resolved snapshot；Skill/MCP manager 订阅 `ConfigChange` 后在
旁路 reconcile。Config commit 成功不等于 MCP 已连接、Skill 已可用、Plugin 已激活或 Hook 已执行，
reconcile failure 不能回滚 desired document。

当前 typed RPC mutation 会重新序列化完整 TOML document，因此键顺序保持确定，但手写注释和
自定义排版可能被规范化；外部只改注释或排版不会推进 semantic generation。

## 关键私有符号

| Symbol | 作用 | 漂移信号 |
| --- | --- | --- |
| `store::ConfigAuthority` | TOML document + SQLite metadata 的一次一致观测 | 若 DB 重新保存 editable document，出现第二 authority |
| `store_file::write_document` | TOML atomic replace 与 durability | 若 command 绕过它直接改文件，receipt/revision contract 会漂移 |
| `store::read_receipt` / `write_receipt` | exact replay contract | 若 App Server 自己实现 dedupe，ownership 被复制 |
| `store_schema::initialize` | component schema install/version gate | 若 Session/Thread tables 在此定义，物理边界混合 |
| `store_monitor::publish` | cross-connection 去重 signal | signal 不能成为未提交状态的来源 |
| `UserConfigDocument::validate` | 全文 typed invariant | 禁止用 arbitrary JSON escape hatch 绕开 |
| `resolve_scoped_config` | scope/trust merge | Workspace 不能扩大 credential、grant 或 policy |
| `exec_policy::{compose_exec_policy, UserExecPolicyConfig, WorkspaceExecPolicyConfig}` | rule source validation 与 typed composition | 不能复制 selector evaluation 或最终 grant authority |
| `workspace_trust::WorkspaceTrustConfig::decision_for` | User trust lookup 与缺失时 Restricted | Workspace document 不能进入这条 mutation/persistence 路径 |

## 失败与验证

不支持的 SQLite/document schema、损坏 TOML、静态 Provider 错误、foreign namespace、revision
冲突与 receipt payload conflict 都 fail closed。Secret 不得出现在 document、receipt、diagnostic
或测试 fixture 中。

```text
cargo test -p zeta-config
```

测试覆盖 TOML durability/reopen、旧 DB document 迁出、external edit、revision/replay/conflict、
以下 no-op generation 与 cross-connection signal、Provider/model invariant、MCP/Skill/Plugin/Hook
desired config、language-server ID/mode/absolute-path validation、execution-policy
mutation/round-trip/layer constraints、Workspace strict TOML parsing、namespace 和 scoped resolution。
