# `zeta-agent-environment`

> 本 README 拥有 Agent 环境快照、文件系统根目录集合和模型环境文本的实现契约。Session 授权与撤销由 [`zeta-workspace-access`](../workspace-access/README.md) 和 App Server 管理；模型上下文的选择、预算与请求位置见 [`agent-harness-design.md`](../../docs/agent-harness-design.md)。

## 快速理解

`zeta-agent-environment` 将宿主已经采集并授权的环境事实转换为不可变、可比较、可确定性渲染的模型输入。

| 调用方提供什么 | crate 保证什么 | 不会执行什么 |
| --- | --- | --- |
| cwd、系统信息、仓库摘要和已授权根目录 | 校验必填值、固定主根顺序、附加根排序去重、XML 转义 | 文件访问、命令执行、信任决定、Session mutation |

## 所有权

- 定义 `AgentEnvironmentSnapshot`、`HostEnvironment` 和 `RepositoryEnvironment` 的不可变值契约。
- 定义 `WorkspaceRoots` 的主根优先、附加根绝对路径校验、排序与去重不变量。
- 确定性渲染 `<environment_context>`，并对所有宿主文本和路径执行 XML 转义。

App Server 负责采集 Git、平台、shell 和日期，并从 Session Workspace authority 取得仍有效的根目录。Core 只消费快照，负责上下文预算、位置和模型调用生命周期。若本 crate 开始依赖 Core、App Server、Workspace trust、Git、Tool 或 RPC，说明职责已经漂移。

## 内部文件归属

| 文件 | 职责 |
| --- | --- |
| `error.rs` | 构造边界的稳定错误类型与必填值校验 |
| `model/workspace_roots.rs` | `WorkspaceRoots` 顺序、绝对路径与去重不变量 |
| `model/snapshot.rs` | 宿主、仓库和完整 Agent 环境快照类型 |
| `render/environment_context.rs` | 唯一的 `<environment_context>` 格式与 XML 转义实现 |

## 调用关系

```text
App Server host collection + Session workspace authority
  → HostEnvironment / RepositoryEnvironment / WorkspaceRoots
  → AgentEnvironmentSnapshot
  → Core context planning
  → AgentEnvironmentSnapshot::render
  → final model request
```

## 验证与修改影响

```bash
just test zeta-agent-environment
just test zeta-core
just test zeta-app-server
```

修改 XML 标记、字段或路径顺序时，必须同步检查 Core token 估算、App Server 的环境采集测试和 `docs/agent-harness-design.md`。新增权限、网络或多执行环境字段前，必须先有真实调用方；本 crate 不保存 Session 状态，也不自行推导权限。
