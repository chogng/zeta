# 工作区安全边界

> 本文拥有跨 crate 的工作区身份、目录作用域和信任语义。Rust 精确契约由
> [`zeta-rs/workspace/README.md`](../zeta-rs/workspace/README.md) 与
> [`zeta-rs/add-dir/README.md`](../zeta-rs/add-dir/README.md) 分别维护。Agent 自定义 artifact 与
> 外部 Import/source registration 的生命周期由
> [`agent-customizations.md`](agent-customizations.md) 维护。

## 快速理解

工作区安全包含两个独立判断：路径包含关系决定“能访问哪里”，行为信任决定“能执行什么”。
一个 canonical root 通过路径校验，不代表它可以启动进程、加载可执行配置或改变仓库。

| 用户操作 | 项目身份如何变化 | 文件访问如何变化 | 当前状态 |
| --- | --- | --- | --- |
| 打开工作区 | 建立主工作目录和当前项目 | 默认允许访问主目录 | 已实现 |
| `add-dir` | 不变 | 增加附加目录 | 尚未实现 |
| `/cd` | 切换主工作目录和当前项目 | 重新建立默认访问与项目配置作用域 | 尚未实现 |
| 搜索内容 | 不变 | 默认搜索主目录；未来也搜索附加目录 | 已实现单 root |
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

`zeta-config` 在 `UserConfigDocument.workspace_trust` 中保存用户明确的 Restricted/Trusted
决定。缺失项解析为 Restricted；checked-in `.zeta/config.toml` 若声明该 section 会被拒绝。
Client 请求 `workspace/switch` 时，App Server 在 authority-switch safe point 解析请求声明的
authority：普通 client 只能读取最新 User snapshot；声明 `workspaceTrustHost` 的产品 host 可以
提交一次会话级 `HostConfiguration` trust，或携带 config revision 和 command id 持久化明确的
Restricted/Trusted 用户决定。App Server 始终先 canonicalize root、自行生成 trust identity，再写入
User Config；Renderer 不能提供 identity 或直接签发 capability。

声明 `workspaceTrustHost` 的 Desktop host 还可以调用 `workspace/trust/list` 查看当前 profile
的显式决定，调用 `workspace/trust/set` 添加或修改决定，调用 `workspace/trust/forget` 删除
决定。三个管理 RPC 都在 App Server 侧重新 canonicalize 或按 opaque identity 校验，不能把
Renderer 提供的 path 或 identity 直接当成 capability authority。

当前产品入口的默认语义如下：

| 入口 | trust 来源 | 是否持久化 |
| --- | --- | --- |
| `zeta` CLI 的启动 cwd / 显式 root | `HostConfiguration` | ❌，仅该进程 |
| `zeterm` 启动 cwd | `HostConfiguration` | ❌，仅该 App Server session |
| `zeterm` 本地目录 picker | `HostConfiguration` | ❌，仅该 App Server session |
| Desktop 启动 folder / Open Folder | Electron Main 复用已有决定或收集明确用户决定 | ✅，写入 User Config |

User Config 变化也是主动撤销信号。App Server 会失效共享 capability lease，移除本地
Tool/Git/Search/Terminal，终止 PTY 与 Search process，并在后续工作使用旧 authority 前中断活动
Turn。Service entry point 会重新检查 lease，因此 stale `Arc` 不能绕过 runtime removal。相同
Restricted root 的安全 Files 与 watcher 会继续保留。

## 工作目录、附加目录与 `/cd`

当前 App Server 只支持一个活动 Workspace root；`add-dir`、附加目录授权和 multi-root Workspace
均尚未实现。目标模型必须区分三种操作：

- **主工作目录**：当前项目、默认相对路径、项目配置和 session discovery 的根。
- **附加目录**：额外文件访问 root；它不成为第二个项目，也不改变主工作目录。
- **`/cd`**：替换主工作目录，并重新建立项目配置与 session discovery 作用域。

因此 `add-dir` 不只是 App Server 命令，而是独立领域边界。`zeta-add-dir` 拥有“一个主工作目录
+ 若干显式附加目录”的访问作用域、directory source lifetime、canonical deduplication 与
contribution policy；`zeta-workspace` 继续只拥有单 root identity、containment 与 trust token。
App Server 负责在 safe point 解析每个 root 的 capability 与 trust、协调 consumer 和 revocation。

| 操作 | 改变当前项目 | Files / Search 可访问 | 自动加载配置 | 生命周期 |
| --- | --- | --- | --- | --- |
| 启动时的主目录 | ✅ | 主目录 | 完整项目配置 | 直到 `/cd` 或进程结束 |
| 启动参数 `--add-dir` | ❌ | 主目录 + 附加目录 | 只允许明确列出的贡献 | 本次启动 |
| 会话命令 `/add-dir` | ❌ | 主目录 + 附加目录 | 只允许明确列出的贡献 | 当前会话 |
| 持久 `additionalDirectories` | ❌ | 主目录 + 附加目录 | ❌，只提供文件访问 | 配置有效期间 |
| `/cd` | ✅ | 以新主目录重新解析 | 加载新主目录的完整项目配置 | 直到再次切换 |

目标配置激活必须与文件访问授权分离。命令行或会话添加的目录可以按 allowlist 贡献 Skills、
Agent definitions、`enabledPlugins` 和 `extraKnownMarketplaces`；Hook、大部分 permission、output style、
command、`CLAUDE.md`、`.claude/rules/` 与 `CLAUDE.local.md` 默认不生效。Host compatibility
switch 可以允许最后三类 instruction，但它不能扩大 Files 权限。持久 `additionalDirectories`
始终是 file-access-only，不执行任何附加目录配置发现。

Rust API 不使用 `bool` 表达这些差异。当前 `zeta-add-dir` 已用
`AdditionalDirectorySource` 保留 directory origin，并解析
`AdditionalDirectoryContributionPolicy`；不能把“目录已授权”直接解释为“目录配置已激活”。

## 与外部 Agent 导入的关系

`zeta-agent-import` 继续拥有 Codex/Claude 的已知布局、candidate classification 和敏感路径排除。
它可以为附加目录的 allowlisted contribution discovery 提供 source-specific inspection，也可以
服务于用户显式触发的 Import workflow，但两条调用路径不能合并：

| 路径 | 目的 | 是否写入 Zeta Config | 是否授予持续文件访问 |
| --- | --- | --- | --- |
| `add-dir` contribution discovery | 在目录授权有效期间投影允许的贡献 | ❌ | ✅，由 `add-dir` authority 授予 |
| `import-agent` | 预览、选择并迁移外部配置 | 用户确认后由目标领域决定 | ❌ |

Import source 不是 `add-dir`。`zeta-agent-import` 不能添加 root、持久化 Workspace 布局或授予持续
文件访问。两条流程只共享 host directory picker、path containment primitive 和安全的
source-specific parser；从用户 home 导入配置不能把 home 隐式变成 Workspace 或附加目录。

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

## 后续计划

1. 把 `zeta-add-dir` 的纯 directory scope 接入 App Server，明确区分主工作目录、附加目录和
   directory source。
2. 为每个附加 root 单独解析 capability、trust 与 contribution policy，只原子重建允许看到该
   root 的 runtime。
3. 增加 `/cd` authority switch；它替换项目 identity 并重新加载主目录配置，而不是复用
   `add-dir` mutation。
4. 在 host trust resolver 中增加 filesystem identity change invalidation。
5. 让 Renderer 完整投影 App Server 已提交的 Restricted/Trusted 状态与 capability availability。
6. Terminal、Workspace MCP/LSP/extension activation、Hook 和 repository mutation 全部使用
   root-bound capability。
7. 增加 restricted-safe content Search backend，或 narrow sandboxed host-owned Search
   capability。

潜在方向包括 organization-managed trust policy、signed Workspace provenance 与
handle-relative filesystem enforcement。这些都不是当前行为。
