# 工作区安全边界

> 本文拥有跨 crate 的工作区身份、目录作用域和信任语义。Rust 精确契约由 [`zeta-rs/workspace/README.md`](../zeta-rs/workspace/README.md) 与 [`zeta-rs/workspace-access/README.md`](../zeta-rs/workspace-access/README.md) 分别维护。Agent 自定义 artifact 与外部 Import/source registration 的生命周期由 [`agent-customizations.md`](agent-customizations.md) 维护。

## 快速理解

工作区安全包含两个独立判断：路径包含关系决定“能访问哪里”，行为信任决定“能执行什么”。
一个 canonical root 通过路径校验，不代表它可以启动进程、加载可执行配置或改变仓库。

| 用户操作 | 项目身份如何变化 | 文件访问如何变化 | 当前状态 |
| --- | --- | --- | --- |
| 打开工作区 | 建立主工作目录和当前项目 | 默认允许访问主目录 | 已实现 |
| `/add-dir` | 不变 | 为当前 Session 增加附加目录，并默认允许读取和修改文件 | TUI 与 App Server 已实现 |
| `/cd` | 切换主工作目录和当前项目 | 重新建立默认访问与项目配置作用域 | 尚未实现 |
| 搜索内容 | 不变 | 默认搜索主目录；`grep`/`glob` 可显式搜索 Session 附加目录 | 部分具备 |
| 导入外部 Agent 配置 | 不变 | 使用一次性导入来源，不产生持续目录授权 | 只读检查已实现 |

## 两道安全门

| 安全门 | 回答的问题 | 输入 | 结果 |
| --- | --- | --- | --- |
| 结构边界 | 路径是否属于该工作区？ | host path + 文件系统状态 | `WorkspaceRoot` / 相对路径 |
| 行为信任 | 该工作区是否可以激活高风险行为？ | exact root + host 信任策略 | `TrustedWorkspace` |

通过 containment 不会自动授予信任。一个 canonical root 的信任不能授权另一个 root。

## 当前实现

`zeta-workspace` 拥有 canonical root identity，并保留 host 最初请求的绝对路径别名，用于 watcher
投影。这能处理 macOS `/var` 与 `/private/var` 等 namespace 差异，同时保持 canonical
containment。

App Server 的 Files 与 Git watcher 对普通 root 使用推荐的原生后端。当 requested namespace 与
canonical namespace 不同时，它会显式改用 polling，避免 FSEvents 在应用层投影前丢弃 alias
event。Watcher 构造和 root 注册全部成功后，runtime authority 才能提交。

App Server 可以提交 Restricted root，并安装文件读写与 invalidation watcher；它不会安装本地
Tool、Terminal、Git mutation 或依赖外部进程的内容 Search。当前 Search 使用冻结的 `rg`，因此
Restricted 模式仍不可用，直到存在 restricted-safe backend 或更窄的 sandbox contract。

信任 capability vocabulary 当前覆盖进程执行、可执行配置、Workspace extension、
Workspace-declared Tool 与仓库 mutation。Host 负责解析信任决定；仓库文件和普通产品 client
payload 都不是 authority。`WorkspaceTrustId` 对 canonical root 的平台原生 bytes 做 hash，
User Config 的 `roots` 因此只保存 opaque key。为支持 Settings 中的 Workspace Trust 管理页，
User Config 可在 `rootPaths` 保存 canonical path 作为展示元数据；它不参与授权，旧记录也可以
没有对应路径。

`zeta-config` 在 `UserConfigDocument.workspace_trust` 中只保存用户明确 Trusted 的
allowlist。缺失项解析为 Restricted；旧版本残留的显式 Restricted 条目会在 ConfigStore 打开时
清理，checked-in `.zeta/config.toml` 若声明该 section 会被拒绝。
Client 请求 `workspace/switch` 时，App Server 在 authority-switch safe point 解析请求声明的
authority：普通 client 只能读取最新 User snapshot；声明 `workspaceTrustHost` 的产品 host 可以
提交一次会话级 `HostConfiguration` trust，或携带 config revision 和 command id 持久化明确的
Trusted 用户决定；撤销信任通过移除条目表示。App Server 始终先 canonicalize root、自行生成 trust identity，再写入
User Config；Renderer 不能提供 identity 或直接签发 capability。

声明 `workspaceTrustHost` 的 Desktop host 还可以调用 `workspace/trust/list` 查看当前 profile
的 Trusted 文件夹白名单；Restricted 或缺失的决定不进入该列表。Host 通过
`workspace/trust/read` 读取一个根的持久化 decision 与 effective state，通过
`workspace/trust/set` 添加 Trusted 条目；撤销信任（包括兼容性 `Restricted` 请求）都通过
`workspace/trust/forget` 的缺失条目语义完成。若目标是当前 root，set/forget 会同步触发
App Server runtime reconcile，而不是只更新列表。
三个管理 RPC 都在 App Server 侧重新 canonicalize 或按 opaque identity 校验，不能把 Renderer
提供的 path 或 identity 直接当成 capability authority。
Workbench 的 `contrib/workspace` 拥有当前 Workspace 的 Restricted/Trusted 状态、Trust list 的
展示、folder picker、trust/revoke action；
`preferences` 只负责把 Workspace Trust category 路由到这个 editor，不拥有 trust 业务或状态。

当前产品入口的默认语义如下：

| 入口 | trust 来源 | 是否持久化 |
| --- | --- | --- |
| `zeta` CLI 的启动 cwd / 显式 root | `HostConfiguration` | ❌，仅该进程 |
| `app` 启动 cwd | `HostConfiguration` | ❌，仅该 App Server session |
| `app` 本地目录 picker | `HostConfiguration` | ❌，仅该 App Server session |
| Desktop 启动 folder / Open Folder | Electron Main 复用已有决定或收集明确用户决定 | ✅，写入 User Config |

User Config 变化也是主动撤销信号。App Server 会失效共享 capability lease，移除本地
Tool/Git/Search/Terminal，终止 PTY 与 Search process，并在后续工作使用旧 authority 前中断活动
Turn。Service entry point 会重新检查 lease，因此 stale `Arc` 不能绕过 runtime removal。相同
Restricted root 的安全 Files 与 watcher 会继续保留。

## 工作目录、附加目录与 `/cd`

当前 App Server 保留一个活动主 Workspace identity；`workspace/folders/set` 的产品多目录路由与 Session 级附加目录授权是两条不同流程。`/add-dir` 已接入 TUI 和本地文件工具，但不会把附加目录变成 Workspace folder。实现继续区分三种操作：

- **主工作目录**：当前项目、默认相对路径、项目配置和 session discovery 的根。
- **附加目录**：额外文件访问 root；它不成为第二个项目，也不改变主工作目录。
- **`/cd`**：替换主工作目录，并重新建立项目配置与 session discovery 作用域。

`/add-dir` 只是用户把目录加入当前 Session 的入口。`zeta-workspace-access` 统一保存主工作目录、附加目录、每个附加目录的能力开关、来源生命周期、版本和撤销状态；`zeta-workspace` 继续负责单个目录的身份、路径包含关系和信任凭证。App Server 按 Session 保存这份权限，并让需要访问本地目录的功能按自己的能力取得目录快照。

| 操作 | 改变当前项目 | 本地文件工具可访问 | 自动加载配置 | 生命周期 |
| --- | --- | --- | --- | --- |
| 启动时的主目录 | ✅ | 主目录 | 完整项目配置 | 直到 `/cd` 或进程结束 |
| 启动参数 `--add-dir` | ❌ | 尚未实现 | 尚未实现 | 本次启动 |
| 会话命令 `/add-dir` | ❌ | 主目录 + 获得对应能力的附加目录 | 打开“加载项目配置”后，为当前 Session 加载 `.zeta/instructions` 与 `.zeta/agents` | 当前会话 |
| 持久 `additionalDirectories` | ❌ | 尚未实现 | ❌ | 配置有效期间 |
| `/cd` | ✅ | 以新主目录重新解析 | 加载新主目录的完整项目配置 | 直到再次切换 |

`/add-dir` 默认打开“读取文件”和“修改文件”。`zeta code` 的 Config 页面提供当前 Session 的“Directory permissions”标签，每个附加目录分别管理以下开关：

| 开关 | 控制的权限 | 当前是否生效 |
| --- | --- | --- |
| Read files | `read_file`、`grep`、`glob` 读取或搜索该目录 | 已生效，默认打开 |
| Modify files | `write_file`、`edit` 修改该目录 | 已生效，默认打开 |
| Run commands | 允许 `shell-command` 和 Session Terminal 在该目录启动进程 | 已生效；执行前再次检查目录凭证，关闭后终止该目录中仍活动的 Terminal |
| Watch file changes | 允许后台监听该目录变化 | 已生效；用于刷新该 Session 已授权的项目配置，不发布为产品级 Workspace 文件事件 |
| Load project configuration | 允许加载 `.zeta/instructions` 与 `.zeta/agents` | 已生效；只投影到授权它的 Session，关闭后立即移除 |

除“读取文件”外的能力都依赖读取权限。关闭 Read files 会同时关闭该目录的其它开关；后端也拒绝“未允许读取却允许修改、执行、监听或加载配置”的无效权限组合。相对路径仍解析到主 Workspace，访问附加目录必须使用绝对路径。`apply_patch`、Workspace Files 和 Workspace Search 仍只使用产品打开的主 Workspace。

Rust API 不使用一个 `bool` 表示目录是否“全权可用”。`AdditionalDirectoryPermissions` 保存完整能力集合，`AdditionalDirectorySource` 保存目录来源。目录已加入不能被解释成所有 Tool 都已获权；只有打开对应开关，消费方才会取得相应能力的目录凭证。当前项目配置开关只加载 `.zeta/instructions` 与 `.zeta/agents`，不授权 Skill、Plugin、MCP、LSP、Hook 或 Workspace Search。

`/add-dir` 本身是用户对该精确目录的会话级授权动作。App Server 规范化路径后签发 `ExplicitUserDecision` lease，但不写入 User Config；移除目录、归档 Session 或切换主 Workspace 都会撤销 lease。`workspace/additionalDirectories/list|add|remove|permissions/set` 只接受声明 `workspaceTrustHost` 的产品连接；不同 Session 的权限互不可见。权限更新携带期望版本，过期页面不能覆盖较新的选择。添加、移除或关闭能力都会推进版本并撤销旧凭证；同一 Turn 的后续文件 Tool 调用会取得最新能力快照。已经发给模型的单次请求保持不变，下一次模型调用才会看到更新后的可读目录。

Core 在每次模型调用边界把 Session identity 交给 `HarnessContextProvider`。App Server 从文件工具使用的同一权限集合取得“读取文件”快照，再交给 `zeta-agent-environment` 生成 `<filesystem><workspace_roots>`：主 Workspace 是第一项，仍允许读取的附加目录随后列出，cwd 不变，相对路径仍属于主 Workspace。Core 把完整环境快照放在持久 Thread 历史之后的请求尾部；它不制造持久用户消息，目录变化也不会改动 system instructions 前缀。

## 与外部 Agent 导入的关系

`zeta-agent-import` 继续拥有 Codex/Claude 的已知布局、candidate classification 和敏感路径排除。
它可以为附加目录的 allowlisted contribution discovery 提供 source-specific inspection，也可以
服务于用户显式触发的 Import workflow，但两条调用路径不能合并：

| 路径 | 目的 | 是否写入 Zeta Config | 是否授予持续文件访问 |
| --- | --- | --- | --- |
| `add-dir` contribution discovery | 在目录授权有效期间投影允许的贡献 | ❌ | ✅，由 Workspace access authority 授予 |
| `import-agent` | 预览、选择并迁移外部配置 | 用户确认后由目标领域决定 | ❌ |

Import source 不是 Workspace access authority。`zeta-agent-import` 不能添加 root、持久化 Workspace 布局或授予持续文件访问。两条流程只共享 host directory picker、path containment primitive 和安全的 source-specific parser；从用户 home 导入配置不能把 home 隐式变成 Workspace 或附加目录。

## 必需的产品流程

```text
host 解析目录
  → 建立 WorkspaceRoot
  → 为精确 canonical identity 读取信任决定
  ├─ Restricted：只允许 browse / edit / watch
  └─ Trusted：按高风险能力签发 root-bound capability
       → Terminal / Tool process / LSP / MCP / extension / Git mutation
```

Trust revocation 或 root replacement 必须先拆除 capability 和 process，再发布新的 Workspace
authority。活动 Turn 会阻止 authority switch，直到旧 runtime 能一致退休。

## 当前限制

- Organization policy resolution 尚未实现。Desktop 已由 Electron Main 收集 Open Folder 的
  Trust / Restricted / Cancel；其它未来 UI host 仍需在自己的安全边界提供 consent。
- Restricted content Search 尚未安装，因为当前 backend 会启动 `rg`。
- Revocation 会取消活动 Turn 及 cooperative Tool cancellation token。已经越过操作系统
  side-effect boundary 的 process 仍可能返回 unknown outcome；撤销 capability 不能回滚外部副作用。
- `WorkspaceTrustId` 当前绑定 canonical path，而不是 filesystem object identity；同一路径上的
  内容替换不会自动失效信任。
- Path validation 与后续 I/O 不是 atomic；在 hostile concurrent mutation 的 threat model 下，
  仍需 handle-relative filesystem API。
- `/add-dir` 当前让 `zeta code` 当前 Session 的模型环境、本地读写工具、`shell-command`、Session Terminal、项目配置加载和配置 watcher 识别附加目录。Workspace Files 与 Workspace Search 继续只展示和搜索产品打开的 Workspace。`--add-dir` 启动参数、用户设置中的持久附加目录、`apply_patch`、Skill、Plugin、MCP、LSP 与 Hook 尚未接入附加目录。

## 后续计划

1. 把 Session 之外的 `LaunchArgument` 与 `PersistentConfiguration` source 接入各自 host owner。
2. 让 `apply_patch` 使用附加目录的修改能力快照；Skills、Plugin、MCP、LSP 与 Hook 若要接入，必须分别增加明确的能力和 Session 隔离。
3. 增加 `/cd` authority switch；它替换项目 identity 并重新加载主目录配置，而不是复用
   `add-dir` mutation。
4. 在 host trust resolver 中增加 filesystem identity change invalidation。
5. 继续把 App Server 已提交的 capability availability 投影到更多 Workbench surface；当前
Workspace Trust editor 与 language/diagnostics gate 已投影 Restricted/Trusted 状态。
6. Workspace MCP/LSP/extension activation、Hook 和 repository mutation 全部使用
   root-bound capability。
7. 增加 restricted-safe content Search backend，或 narrow sandboxed host-owned Search
   capability。

潜在方向包括 organization-managed trust policy、signed Workspace provenance 与
handle-relative filesystem enforcement。这些都不是当前行为。
