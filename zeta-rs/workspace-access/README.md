# `zeta-workspace-access`

> 本 README 拥有工作区访问权限集合、来源授权生命周期和不可变访问快照的实现契约。跨组件产品语义由 [`workspace-security.md`](../../docs/workspace-security.md#工作目录附加目录与-cd) 维护；单目录身份与信任凭证由 [`zeta-workspace`](../workspace/README.md) 维护。

## 快速理解

`zeta-workspace-access` 把一个主工作目录与零个附加授权目录组合成带版本的访问权限集合。新增目录不改变 cwd 或项目身份；模型和工具必须从同一权限集合冻结快照，不能维护第二份目录列表。

crate 使用能力名而不是命令名：`/add-dir` 只是 Session 用户修改权限集合的一个入口，启动参数、持久配置和将来的宿主授权也会修改同一集合。模型环境和本地文件工具已经消费这份 Workspace 访问权限；Search 与沙箱接入附加根时也必须冻结同一个 authority，不能依赖 slash command。命令解析继续属于产品入口，把这层命名为 `add-dir` 会让一个入口错误拥有所有 consumer 的执行状态。

| 操作 | 权限集合变化 | 已冻结快照 | 新快照 |
| --- | --- | --- | --- |
| 添加来源 | revision 增加 | 不扩大 | 包含新增目录 |
| 重复添加 | 不变 | 不变 | 不变 |
| 移除来源 | revision 增加并撤销该 lease | 已撤销 token 失败关闭 | 不再包含该来源 |

## 所有权

- `WorkspaceAccessAuthority` 拥有主目录角色、附加目录、来源授权、撤销和单调 revision。
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
| `contributions.rs` | 精确配置贡献 allowlist |

## 验证

```bash
just test zeta-workspace-access
```

修改 mutation 或 revision 规则时必须同步检查 AppServer 的 Session 生命周期、工具冻结点和模型环境；新增工具、RPC 或 Session map 依赖表示 crate 责任已经漂移。
