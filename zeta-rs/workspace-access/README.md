# `zeta-workspace-access`

> 本 README 拥有工作区访问权限集合、来源授权生命周期和不可变访问快照的实现契约。跨组件产品语义由 [`workspace-security.md`](../../docs/workspace-security.md#工作目录附加目录与-cd) 维护；单目录身份与信任凭证由 [`zeta-workspace`](../workspace/README.md) 维护。

## 快速理解

`zeta-workspace-access` 把一个主工作目录与零个附加授权目录组合成带版本的访问权限集合。每个目录来源携带独立的文件、执行、监听、产品文件/搜索和配置贡献权限；新增目录不改变 cwd 或项目身份，模型和工具必须按所需能力从同一权限集合冻结快照。

crate 使用能力名而不是命令名：`/add-dir` 只是 Session 用户修改权限集合的一个入口。模型环境、本地文件工具、进程工具、Terminal、配置 watcher、Workspace Files/Search、Instructions & Agents、Skills、MCP、LSP、Hooks 和 Plugins 分别按所需能力冻结同一个 authority，不能依赖 slash command。MCP 与 Plugin 的发现权限不会替代它们自己的连接、安装、信任和激活决策。

| 操作 | 权限集合变化 | 已冻结快照 | 新快照 |
| --- | --- | --- | --- |
| 添加来源 | revision 增加 | 不扩大 | 包含新增目录 |
| 重复添加 | 不变 | 不变 | 不变 |
| 替换能力集合 | revision 增加并撤销旧 lease | 已冻结 token 失败关闭 | 只包含仍获权的目录 |
| 移除来源 | revision 增加并撤销该 lease | 已撤销 token 失败关闭 | 不再包含该来源 |

## 所有权

- `WorkspaceAccessAuthority` 拥有主目录角色、附加目录、逐来源能力集合、撤销和单调 revision。
- `WorkspaceAccessSnapshot` 为一个明确的 `WorkspaceCapability` 冻结有序 `TrustedWorkspace` 集合。
- `AdditionalDirectoryContributionPolicy` 独立描述允许发现的配置贡献，不把文件访问解释成配置激活。

AppServer 负责按 Session 保存 authority、解析用户路径和处理 RPC；本 crate 不依赖 Session、Core、工具、沙箱、配置或产品 UI。`zeta-agent-environment` 只把快照路径渲染给模型，不拥有真实授权。

## 内部文件归属

| 文件 | 职责 |
| --- | --- |
| `access/authority.rs` | 唯一可变权限集合、来源授权和撤销 |
| `access/snapshot.rs` | revision、mutation 与不可变能力快照 |
| `access/error.rs` | 权限集合构造和冻结错误 |
| `additional_directory.rs` | 附加目录来源及其配置贡献解析 |
| `permissions.rs` | 用户可见能力集合、依赖校验与 `WorkspaceCapability` 映射 |
| `contributions.rs` | 精确配置贡献 allowlist |

## 验证

```bash
just test zeta-workspace-access
```

修改 mutation 或 revision 规则时必须同步检查 AppServer 的 Session 生命周期、工具冻结点和模型环境；新增工具、RPC 或 Session map 依赖表示 crate 责任已经漂移。
