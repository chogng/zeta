# 配置系统

> 本文拥有用户配置、目录配置、作用域合并和运行时生效点。实现见
> [`zeta-config`](../zeta-rs/config/README.md)。目录能力的长期语义见
> [`environment-access.md`](environment-access.md)。

## 结论

配置表达“期望什么”，领域管理器维护“当前实际可用什么”，App Server 在安全点冻结“这次执行
能使用什么”。三者不能合并成一个状态。

```text
Desired configuration
    User Config + Dir Config + Session/launch inputs

Operational state
    Plugin / MCP / Skill / Hook / provider managers

Runtime snapshot
    App Server freezes one exact generation for a Turn or request
```

目录配置只是来源意图。读取它需要 `LoadConfig`，发现其中的 Skill、MCP、Hook 或 Plugin 还需要
各自的来源能力；目录文件不能给自身增加 capability、绑定凭据、安装包或放宽沙箱。

## 权威分布

| 所有者 | 拥有 | 不拥有 |
| --- | --- | --- |
| User Config | Agent 默认值、Provider、MCP、Skill source、Plugin request、Hook、Tool Search、execution policy、目录权限 | secret、live connection、已安装包、运行健康状态、UI 偏好 |
| Dir Config | 目录提供的 Agent/MCP/Skill/Plugin/Hook 意图和只收紧的执行规则 | 授权、凭据、安装、激活和运行状态 |
| Device Settings | Theme、可访问性、hover、sash 等界面偏好 | Agent、Provider、目录权限和运行状态 |
| Secret Store | opaque secret bytes | 配置类型、OAuth 流程和作用域决定 |
| 各领域管理器 | 实际安装、连接、激活、健康状态和生命周期 | 用户配置正文 |
| App Server | 组合不可变快照并选择生效点 | 重新拥有各领域状态 |

## 用户配置

`ConfigStore` 使用 `<profile>/config.toml` 保存 typed 用户文档，使用 SQLite 保存 revision、generation
和命令回执。修改命令携带 `commandId + expectedRevision`：

- revision 不匹配时拒绝写入；
- 同一命令重放返回原结果；
- TOML 替换和回执提交保持原子；
- 成功提交只影响后续安全点，不改写已经冻结的 Turn。

`config.toml` 顶层使用 `schemaVersion` 标记文件格式。读取无版本的历史文件时，Config 只执行已登记且无歧义的迁移，完成严格校验后原子重写为当前版本；历史字段和当前字段同时出现、未登记字段、过新版本或低于最低支持版本都会拒绝启动。`semanticCodeIndex` 迁到 `codebase` 时不会保留旧的源码外发授权；`workspaceTrust` 只转换路径仍存在、旧身份与路径一致的 `trusted` 项，并为它生成当前目录身份，其他项不落盘。

用户文档的主要 section 是 `agent`、`providers`、`mcp`、`skills`、`plugins`、`hooks`、
`toolSearch`、`execPolicy`、`dirPermissions` 和 `codebase`。Config 保存非敏感引用，不保存 API key、
OAuth token、authorization header 或 refresh 状态。

## 目录配置

`DirConfigStore` 严格读取一个目录中的 `.zeta/config.toml`。Host 在文档之外提供 `DirId` 与内容
revision；文件不能选择自己的身份或 generation。

```rust
pub struct DirConfigDocument {
    pub agent: DirAgentConfig,
    pub mcp: DirMcpConfig,
    pub plugin_requests: DirPluginRequests,
    pub skills: DirSkillsConfig,
    pub hooks: HooksConfig,
    pub exec_policy: DirExecPolicyConfig,
}
```

目录声明使用 `dir:<dir-id>:` namespace。它可以请求 Provider 已存在时的模型默认值、无凭据的 MCP
声明、Skill source、Plugin package、Hook 和执行约束，但不能：

- 写入 User Config；
- 给目录增加 capability；
- 选择 credential；
- 安装或启用 Plugin；
- 建立 MCP 连接；
- 执行 Hook；
- 使用 `AllowUnsandboxed` 放宽执行策略。

## 目录权限

`DirPermissionsConfig` 按 `DirId` 保存明确的 `Capability` 集合。缺失条目表示没有持久授权；系统不
保存 `Trusted / Restricted` 或目录级 Trust。

```text
config/dirPermissions/list
config/dirPermissions/read
config/dirPermissions/set
config/dirPermissions/forget
```

`set` 接受完整 capability 集合，而不是一个 `trusted` 布尔值。App Server 将配置解析为 `Grant`，
动作入口再为具体 Permission 取得 `Authorization`。Config 不签发 Authorization，也不执行动作。

## 合并与来源

```text
BuiltInDefaults
  + UserConfig
  + DirConfig
  + SessionSettings
  + LaunchOverrides
  constrained by SystemRequirements
  = ResolvedConfigSnapshot
```

这不是递归对象合并。每个字段必须明确来源、merge/replace/clear 语义、provenance 和生效点。

| 配置 | 来源 | 关键规则 |
| --- | --- | --- |
| Preferred model | User、Dir、Session、launch | 只影响下一次模型安全点 |
| Tool Mode | User、StartTurn override | Turn 接受时冻结 |
| Provider endpoint | User、Host | Dir 不能替换认证或网络边界 |
| MCP / Skill / Plugin / Hook | User、Dir | Dir 只提供待处理意图；领域管理器决定实际状态 |
| Execution policy | Host、Organization、User、Dir | Dir 只能保持或收紧 |
| Directory permissions | User、Organization、Host | Dir Config 无权自授 |
| UI preference | Device Settings | 不进入 Agent runtime snapshot |

目录来源未获得对应能力时，解析结果保留其待处理意图和诊断，但不会静默激活。运行时协调失败只
更新实际状态，不回滚用户期望。

## 生效点

| 变化 | 生效时机 |
| --- | --- |
| 模型与 Tool Mode | 下一次模型请求或 Turn 创建 |
| MCP / Skill / Plugin / Hook catalog | 各管理器完成协调后，由新 generation 发布 |
| 目录权限 | 新 Grant 发布后；撤销会使旧 Authorization 失效并停止依赖资源 |
| execution policy | 下一次动作评估；已经准备的调用保留冻结版本 |
| UI preference | 对应 Renderer 服务自己的更新周期 |

运行中的 Turn 不读取可变 ConfigStore，也不持有 live manager。它只消费创建时冻结的快照和后续
明确允许的安全点更新。

## 文件与接口

```text
<profile>/
├─ config.toml
└─ state.sqlite3

<dir>/.zeta/config.toml
```

公共类型使用 `DirConfigDocument`、`DirPermissionsConfig` 等完整名称，因为它们跨模块表达具体配置
种类。进入 `dir_config.rs` 或目录权限模块后，私有变量使用 `dir`、`permissions`、`revision`，不再
重复 `directory_configuration` 一类上下文限定词。

## 不变量

- 一个 scope 内同一份期望状态只有一个 authority。
- 配置提交成功不等于 Plugin 已激活、MCP 已连接、Skill 已可用或 Hook 已执行。
- 目录配置不能扩大 capability、credential scope、policy 或 sandbox。
- Secret 不进入 TOML、日志、索引或协议快照。
- 已冻结的 Turn 不随可变配置漂移。
- Config 使用 `Dir` 与明确权限，不重新引入 Workspace 身份或目录级 Trust。
- 配置迁移必须先完成完整校验再替换原文件；失败时保留原文件并阻止运行时启动。
