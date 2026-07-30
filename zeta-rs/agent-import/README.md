# `zeta-agent-import`

> 文档所有权：本 README 是外部 Agent 配置只读发现、路径校验、导入计划与失败语义的当前实现
> 契约。
>
> 产品交互与长期演进由
> [`docs/zeta-desktop-architecture.md`](../../docs/zeta-desktop-architecture.md#22-外部-agent-配置导入仅限-desktop)
> 拥有；Skill 来源与激活语义由 [`docs/skills.md`](../../docs/skills.md) 拥有。

`zeta-agent-import` 只识别 Codex 和 Claude 已知的用户级、项目级配置位置，并生成仅元数据
（metadata-only）的 `AgentImportPlan`。当前实现不读取候选正文、不递归扫描未知目录、不修改
Zeta 配置，也不导入认证、会话、日志或历史记录。

## 1. Crate 边界与依赖方向

本 crate 拥有：

- 外部 Agent、用户/项目作用范围和导入条目类型；
- Codex/Claude 已知相对路径与预期文件类型；
- 调用方所选根目录的规范化和目录包含关系校验；
- 确定性排序、去重、候选与安全诊断；
- 私有绝对路径的 `Debug` 隐藏。

本 crate 不拥有：

- Desktop 目录选择、预览、确认、进度或撤销界面；
- App Server 方法、配置 revision 或导入应用流程；
- Config、Skill、MCP、Plugin 或 Multi-Agent 内容 schema；
- 内容信任、工具批准、脚本执行、网络连接或凭据解析；
- `CODEX_HOME`、`CLAUDE_CONFIG_DIR` 等宿主环境解析。

生产代码当前只依赖 Rust 标准库，测试使用 workspace `tempfile`。它不得反向依赖
`zeta-config`、`zeta-skills`、App Server、Core 或 Desktop。后续内容 parser 可以依赖窄格式
crate，但不能为了落库或 UI 方便引入上述高层领域。

## 2. 公共契约

| Symbol | 当前职责 | 不承担 |
| --- | --- | --- |
| `ExternalAgent` | 区分 `Codex` 与 `Claude` 布局 | 动态 provider registry |
| `ImportScope` | 区分 `User` 与 `Project` 来源 | workspace trust 或配置优先级 |
| `AgentImportLocation::{codex_user,codex_project,claude_user,claude_project}` | 将来源、作用范围和调用方选择的根目录绑定为一个发现输入 | 环境变量解析、文件访问授权 |
| `AgentImportPlan::discover` | 校验所有输入根并发现已知相对位置 | 读取正文、转换内容或应用配置 |
| `AgentImportCandidate` | 返回来源、作用范围、条目类型、审查类别、相对路径和 canonical host path | 表示内容已受信任、已批准或可执行 |
| `AgentImportDiagnostic` | 说明一个已存在的已知路径为什么未进入候选 | 暴露根目录、正文或凭据 |
| `AgentImportError` | 表达调用方选择的根目录整体无效 | 表达单个候选的隔离错误 |

典型调用点保持自解释，不传递布尔模式或裸来源字符串：

```rust
use zeta_agent_import::{AgentImportLocation, AgentImportPlan};

let plan = AgentImportPlan::discover([
    AgentImportLocation::codex_user(user_home),
    AgentImportLocation::claude_project(project_root),
])?;
```

`AgentImportPlan` 只是预览输入。调用方仍须让用户选择具体候选，并把确认结果交给相应领域重新
解析和校验；不能把 `candidates()` 非空解释为可自动导入。

## 3. 已知布局与审查分类

`layout.rs` 是外部路径映射的唯一实现 owner。当前固定布局如下：

| 来源 | 用户级路径 | 项目级路径 |
| --- | --- | --- |
| Codex | `~/.codex/{AGENTS.md,AGENTS.override.md,config.toml,agents/,rules/}`、`~/.agents/skills/` | `{AGENTS.md,AGENTS.override.md}`、`.codex/{config.toml,agents/,rules/}`、`.agents/skills/` |
| Claude | `~/.claude/{CLAUDE.md,settings.json,skills/,agents/,commands/,rules/}` | `{CLAUDE.md,CLAUDE.local.md,.mcp.json}`、`.claude/{CLAUDE.md,settings.json,settings.local.json,skills/,agents/,commands/,rules/}` |

路径映射依据 Codex 的
[导入说明](https://learn.chatgpt.com/docs/import)和
[Skill 位置说明](https://learn.chatgpt.com/docs/build-skills)，以及 Claude 的
[设置位置说明](https://code.claude.com/docs/en/settings)和
[Skill 位置说明](https://code.claude.com/docs/en/slash-commands)。外部产品改变路径时，先更新
官方契约证据和 fixture，再修改 `layout.rs`；不能靠扫描整个 home 猜测新位置。

| `ImportItemKind` | `ImportReviewCategory` | 原因 |
| --- | --- | --- |
| `Instructions`、`Skills`、`SlashCommands`、`InstructionRules` | `Content` | 包含将进入模型上下文的外部指令 |
| `Settings`、`Subagents` | `Configuration` | 需要目标领域映射，不能按原格式直接生效 |
| `McpServers` | `Connection` | 可能引入进程、网络、header 或重新登录要求 |
| `ExecutionRules` | `ExecutionPolicy` | 可能改变命令是否提示、允许或阻止 |

审查类别只用于 UI 分组和后续处理路由，不是授权结果。特别是 `ExecutionPolicy` 不能生成 Zeta
长期批准，`Connection` 不能自动启动 MCP。

`~/.codex/auth.json` 永不进入候选。`~/.claude.json` 同时包含 OAuth session、MCP、
per-project state 和 cache，当前整体排除。未来若需要其中的非敏感 MCP 声明，必须增加专门的
有界 parser 与结构化脱敏，不能把整个文件交给 Desktop、模型或普通日志。

## 4. 文件与内部所有权

| 文件 / private symbol | 单一职责 | 修改时同步检查 |
| --- | --- | --- |
| `model.rs` | 公共值类型、named constructor、getter 和私有路径 `Debug` | App Server DTO、Desktop preview、隐私测试 |
| `layout.rs::candidate_specs` | `ExternalAgent + ImportScope` 到固定 `CandidateSpec` 的穷尽映射 | 官方路径、review category、fixture 和本表 |
| `layout.rs::{file,directory}` | 构造带预期文件类型的静态 specification | type-mismatch diagnostic |
| `discovery.rs::discover` | 逐 root 发现、排序、去重并构造 immutable plan | 多 root 失败语义和顺序测试 |
| `discovery.rs::validate_root` | 拒绝不可用、非目录或 symlink root，并冻结 canonical root | `AgentImportError` 与错误脱敏 |
| `discovery.rs::inspect_candidate` | 检查一个已知相对路径的 metadata、类型、symlink 和 containment | diagnostic code、候选 canonical path |
| `error.rs` | 根目录级类型化错误与不含绝对路径的显示文本 | Desktop 错误映射与日志 |
| `discovery_tests.rs` | 临时目录上的领域级发现与隐私回归 | 新来源、路径、错误或 redaction |

`lib.rs` 保持私有模块和显式 re-export。若调用方开始依赖 `layout`/`discovery` 私有函数，或 crate
root 重新实现路径判断，说明公共 API 或 ownership 已经漂移。

## 5. 真实调用路径

```text
AgentImportPlan::discover
  → discovery::discover
      → validate_root
          → symlink_metadata(root)
          → reject symlink / non-directory
          → canonicalize(root)
      → layout::candidate_specs(agent, scope)
      → inspect_candidate for each fixed relative path
          → missing: omit
          → symlink_metadata(candidate)
          → expected file/directory check
          → canonicalize(candidate)
          → canonical candidate starts_with canonical root
          → AgentImportCandidate | AgentImportDiagnostic
      → sort + dedup candidates and diagnostics
      → AgentImportPlan::new
```

发现只访问根目录和静态 `CandidateSpec` 指向的 metadata。它不会枚举 `.codex`、`.claude` 或
Skill 目录的子项，也不会打开文件，因此当前 plan 不能证明正文有效。

## 6. 失败、隔离与隐私语义

| 条件 | 结果 | 是否继续其他候选 |
| --- | --- | --- |
| 输入 root 不可访问 | `AgentImportError::RootUnavailable` | ❌ 整个调用失败 |
| 输入 root 不是目录 | `AgentImportError::RootNotDirectory` | ❌ 整个调用失败 |
| 输入 root 本身是 symlink | `AgentImportError::RootSymlinkNotAllowed` | ❌ 整个调用失败 |
| 已知相对路径不存在 | 不产生候选或 diagnostic | ✅ |
| 候选 metadata/canonicalize 失败 | `MetadataUnavailable` | ✅ |
| 候选类型与 specification 不符 | `UnexpectedFileType` | ✅ |
| 候选自身是 symlink | `SymlinkNotAllowed` | ✅ |
| ancestor symlink 使 canonical candidate 逃出 root | `EscapesSelectedRoot` | ✅ |
| 候选合法 | 保存 canonical path | ✅ |

多 root 调用不是部分成功协议：任一输入 root 无效都会返回 `Err`，不会返回其他 root 的半份 plan。
单候选问题则通过 diagnostic 隔离，不影响同一 root 的其他候选。

`AgentImportLocation` 和 `AgentImportCandidate` 的 `Debug` 隐藏绝对路径，错误和 diagnostic 只携带
来源、作用范围、固定相对路径或 `io::ErrorKind`。但 `AgentImportCandidate::source_path()` 会
显式返回 canonical host path，供可信 host adapter 后续读取；调用方不得把该值放入普通 telemetry、
Thread history、模型上下文或未脱敏错误。

## 7. Host 接入义务

Desktop/App Server 接入时必须：

1. 只从用户明确选择或产品已定义的根目录构造 `AgentImportLocation`；
2. 在受信 host 侧调用 `discover`，不接受 Renderer 提交任意候选路径；
3. 用 `agent + scope + kind + relative path` 展示候选，默认不显示完整 home path；
4. 让用户按条目确认，并将 review category 对应的风险和后续动作分开说明；
5. 在应用前由目标领域重新打开、限制大小、解析、校验并生成自己的 typed mutation；
6. 对读取与应用之间的文件变化重新校验 identity/digest，不能把 preview 当作冻结内容；
7. 认证、凭据、连接批准和执行批准继续走各自 authority。

当前没有 App Server DTO 或 plan identity。任何 wire contract 都应传 source-qualified identity
和受控相对路径，不应直接把 `source_path()` 序列化给 Renderer。

## 8. 常见修改及影响面

### 增加一个已知路径

1. 在对应 `layout.rs` source/scope 数组增加 `CandidateSpec`；
2. 选择准确的 `ImportItemKind`、`ImportReviewCategory` 和 expected file/directory；
3. 增加存在、缺失、错误类型和敏感排除测试；
4. 更新本 README 的布局表，并检查 Desktop/Skill/权限文档是否受影响。

### 增加一种外部 Agent

1. 扩展 `ExternalAgent` 和 named `AgentImportLocation` constructor；
2. 为 User/Project 分别定义静态 layout，保持 `candidate_specs` 穷尽匹配；
3. 增加官方路径证据、完整 fixture 和 redaction 测试；
4. 定义目标领域映射前只返回 preview candidate，不增加隐式 apply；
5. 同步 App Server DTO、Desktop picker 和系统文档。

### 增加内容 parser

parser 必须位于新的 private module，设置 byte/entry/depth 上限，拒绝未知 active content，并输出
不含 secret 的 typed fragment。它不能在解析阶段写 Config、连接 MCP、执行 hook/script 或生成
批准。若单个模块接近 500 LoC，应按来源或内容类型拆分并把测试放到 sibling `*_tests.rs`。

## 9. 验证

```bash
cargo test --manifest-path zeta-rs/Cargo.toml -p zeta-agent-import
cargo clippy --manifest-path zeta-rs/Cargo.toml \
  -p zeta-agent-import --all-targets --no-deps -- -D warnings
bazel test //zeta-rs/agent-import:agent-import-unit-tests
```

当前测试覆盖：

- Codex user 候选与 `auth.json` 排除；
- Claude user 候选与 `.claude.json` 排除；
- Claude project 内容、配置和连接审查分类；
- 已知路径错误文件类型的 diagnostic；
- ancestor symlink 逃逸；
- plan `Debug` 不泄露临时 root。

测试只使用 `tempfile`，不读取真实 home。修改根目录错误分支、去重或多 root 行为时，应补充对应
failure fixture；当前测试尚未覆盖所有 `AgentImportError` 分支，这是明确的测试缺口。

## 10. 当前限制与扩展点

- **Current**：Codex/Claude 已知路径的 metadata-only 发现、根目录/候选校验、确定性 plan、
  diagnostic 与私有路径 `Debug` 隐藏。
- **Current limitation**：不解析正文，不能生成按条目选择的转换 diff，也不能识别设置文件内嵌
  的 hook、Plugin、MCP 或 Sub-agent 声明。
- **Current limitation**：user constructor 只识别默认 home layout；自定义 `CODEX_HOME`、
  `CLAUDE_CONFIG_DIR` 或独立外部 source root 尚无公共构造契约。
- **Current limitation**：没有 stable plan identity、TOCTOU revalidation 或 App Server wire
  contract。
- **Proposed**：增加有界的 source-specific parser，输出不含 secret 的 typed preview fragment。
- **Proposed**：App Server 组合 parser 结果并调用各 authority；Desktop 只提交用户确认后的
  exact plan identity。

无论后续实现如何演进，以下不变量保持不变：不扫描整个 home、不导入认证状态、不把 preview
当作执行授权、不让 Desktop 或本 crate 绕过目标领域的最终校验。
