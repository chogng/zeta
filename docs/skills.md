# `zeta-skills` 架构与演进方案

> 计划物理位置：`zeta-rs/skills/`
> Rust crate：`zeta_skills`
> 当前状态：Proposed，尚未创建 crate
> Core architecture：[`core.md`](core.md)
> Agent runtime：[`zeta-agent-runtime-architecture.md`](zeta-agent-runtime-architecture.md)
> Config authority 与 runtime snapshot 接入：[`config.md`](config.md)
> Plugin 分发边界：[`plugins.md`](plugins.md)
> MCP runtime：[`mcp.md`](mcp.md)

> 外部格式核对日期：2026-07-25。Skill package 以
> [Agent Skills specification](https://agentskills.io/specification) 为兼容基线；Zeta 自己的
> source、trust、activation 和执行权限语义由本文定义。

## 1. 结论

`zeta-skills` 是 Zeta 的 Skill 发现、索引、解析、选择和渐进加载控制面。Skill 是一个以
`SKILL.md` 为入口的指令目录，可以附带 references、scripts 和 assets。它帮助 Agent 判断“这类
任务应采用什么工作流”，但本身不是可执行权限、tool implementation、MCP server 或 Plugin。

核心原则：

```text
发现时只加载 metadata
→ 任务匹配后完整加载 SKILL.md
→ 只有真实需要时读取 references/scripts/assets
```

Skill 内容是带来源的外部 instruction：

- 它低于 Zeta system/developer/product policy；
- 它不能授权 tool、network、filesystem、credential 或 sandbox bypass；
- 它可以建议使用工具或脚本，但实际执行仍经过标准 tool loop；
- 它不能通过“忽略上层规则”改变 instruction precedence；
- 未激活 Skill 的正文不得占用每次模型调用 context。

## 2. 当前仓库审计

当前没有 Skill manager。唯一已实现的相关 value 是：

```rust
UserInput::Skill { name: String, path: String }
```

它证明用户输入层已经预留显式 Skill 选择，但 `name + raw path` 不是长期安全 identity：

- path 暴露客户端/host filesystem 细节；
- 没有 source、digest、version 或 workspace scope；
- 重放时 path 可能已经指向不同内容；
- 客户端可以构造 catalog 外路径；
- 同名 Skill 无法稳定消歧。

因此长期应将其演进为 validated `SkillRef`/`SkillSelection`，由 Skill manager 解析，不允许 Agent
runtime 或 App Server 直接读取用户提交的裸路径。

仓库已有可复用边界：

- `zeta-protocol` 的 UserInput、Resource、ToolName 和 provider-independent model contract；
- 计划中的 Core ContextAssembler 与 `ModelInvocationSnapshot`；
- App Server 的 typed command、Resource store 和 generated client contract；
- Plugin manager 计划提供 immutable Skill contribution root；
- 目标 `zeta-tool-executor` / `zeta-sandboxing` 负责脚本执行，而不是 Skill loader；产品层
  `zeta-exec` 是 headless Agent runner。

## 3. Format baseline

[Agent Skills specification](https://agentskills.io/specification) 规定：

```text
skill-name/
├── SKILL.md          # required: YAML frontmatter + Markdown instructions
├── scripts/          # optional executable helpers
├── references/       # optional on-demand documentation
├── assets/           # optional templates/static resources
└── ...
```

`SKILL.md` 必须有：

```yaml
---
name: code-review
description: Reviews code changes for correctness and maintainability. Use for PR or diff review.
---
```

兼容字段：

| Field | Zeta 处理 |
| --- | --- |
| `name` | 严格校验，并要求与 Skill 目录名一致 |
| `description` | discovery/selection metadata，必须同时说明做什么和何时使用 |
| `license` | 展示与分发 metadata，不影响 runtime permission |
| `compatibility` | 环境提示；解析为 warning/gate，但不自动安装依赖 |
| `metadata` | string map；只允许 namespaced extension 影响 Zeta 展示 |
| `allowed-tools` | experimental hint；绝不作为 Zeta approval grant |
| Markdown body | Skill 激活后作为完整 instruction 加载 |

Zeta 不修改第三方 Skill format 来塞入 executable grant、credential 或 MCP launch config。这些属于
Plugin manifest/config。独立 Skill 需要额外能力时只能产生明确 compatibility diagnostic。

## 4. 职责与非职责

### 4.1 Skill manager 拥有

- Skill source 的注册、隔离、扫描和 invalidation；
- `SKILL.md` frontmatter strict parse 与 Agent Skills format validation；
- stable Skill identity、source、digest、availability 和 compatibility projection；
- metadata-only catalog 与 immutable generation；
- 显式选择、自动候选检索和冲突解释；
- 激活时完整读取 `SKILL.md`；
- 按需、安全地解析 relative references/scripts/assets；
- context budget、递归深度、文件数、单文件和总 bytes 限制；
- `SkillActivationSnapshot`、provenance 和 content digest；
- file change/update 后只在下一个 model safe point 生效；
- 不含正文和 secret 的 diagnostics/telemetry。

### 4.2 Skill manager 不拥有

- Plugin package 下载、签名、安装、enablement 或 grants；
- script 执行、shell、dependency install、network 或 filesystem mutation；
- MCP session、tool call 或 resource read；
- Agent loop、model selection、prompt tokenization 算法；
- system/developer instruction 或 product policy authority；
- Thread event append、Tool Result 或 retry；
- 把 Skill 自动转成全局 memory；
- 扫描整个 home directory 或任意绝对 path。

## 5. 目标依赖与组合

```text
                   zeta-protocol
                        ▲
                        │ accepted SkillRef/UserInput values
                        │
                   zeta-skills
      source / catalog / selector / loader / resolver / snapshot
             ▲                 ▲                ▲
             │ built-in/user   │ workspace      │ Plugin contribution
             │                 │                │
         config roots      App Server      zeta-plugins snapshot
                              │
                              ▼
                 Core ContextAssembler
                              │
                              ▼
                       ModelRequest
```

规则：

- `zeta-skills` 不依赖 `zeta-core`、stores、App Server 或 Plugin live manager；
- Skill source 以窄 `SkillSourceRoot`/file resolver port 注入；
- Plugin manager 提供 immutable package root，Skill manager 仍重新校验 Skill format；
- Core ContextAssembler 只接收激活 snapshot，不在构造 prompt 时重新扫描 filesystem；
- Skill scripts 通过普通 Tool/exec port 执行，`zeta-skills` 不依赖
  `zeta-tool-executor`；
- 只有 `SkillId`、`SkillRef`、选择 intent 等至少跨两个组件共享的稳定 value 才进入
  `zeta-protocol`；catalog cache、filesystem handle、parse error 留在 `zeta-skills`。

## 6. Identity、source 与 precedence

### 6.1 Stable identity

Skill format 的 `name` 只在一个 source root 内唯一。Zeta identity 必须包含 source：

```rust
pub struct SkillId {
    pub source: SkillSourceId,
    pub name: SkillName,
}

pub struct SkillRef {
    pub id: SkillId,
    pub version: SkillVersionSelector,
}
```

wire `SkillRef` 通过显式 version selector 区分两种用户意图：

- `FollowLatest`：解析当前 source 中同一 Skill ID，适合普通 picker；
- `PinnedDigest`：必须匹配 exact content，适合 Plugin lock、replay 和审计。

使用具名 enum，不使用 `Option<digest>` 让调用者猜含义：

```rust
pub enum SkillVersionSelector {
    FollowLatest,
    Pinned(ContentDigest),
}
```

### 6.2 Source

```text
BuiltIn { release }
User { configuredRoot }
Workspace { workspaceId, configuredRoot }
Plugin { pluginId, version, packageDigest }
LocalDevelopment { canonicalRoot }
```

source root 是 host 验证后的 opaque handle，不是未经校验的字符串 path。

### 6.3 Precedence

Precedence 只用于候选排序，不用于静默覆盖：

```text
用户在当前 Turn 显式选择
> workspace enabled
> user enabled
> Plugin enabled
> built-in fallback
```

两个不同 `SkillId` 即使 `name` 相同也同时存在，UI 必须显示 source。自动选择遇到同名且置信度
接近时返回 ambiguity，不按目录扫描顺序取第一个。

显式 Skill 可以和自动 Skill 同时激活，但去重按 exact `SkillId + digest`，不是 display name。

## 7. Discovery 与 catalog

### 7.1 Metadata-only discovery

启动/刷新时只：

1. 枚举受控 source root 下的直接 Skill 目录；
2. 打开该目录的 `SKILL.md`；
3. 读取有界 frontmatter 和必要 metadata；
4. 校验 name/description/path；
5. 计算内容 identity 所需的 digest 或 filesystem validator；
6. 建立 catalog entry。

不得在 discovery 时：

- 加载 Markdown body 到全局 prompt；
- 递归读取 references/assets；
- 执行 scripts；
- 探测网络或安装 compatibility dependency；
- 跟随逃出 source root 的 symlink；
- 因单个坏 Skill 让整个 built-in/user catalog 丢失。

Plugin package 是原子 contribution：Plugin 内声明的 required Skill 无效时，由 Plugin resolver 决定
整个 generation 是否失败；普通 user source 则可按 entry 隔离错误。

### 7.2 Catalog snapshot

```rust
pub struct SkillCatalogSnapshot {
    pub generation: u64,
    pub entries: Vec<SkillCatalogEntry>,
    pub diagnostics: Vec<SkillDiagnostic>,
}

pub struct SkillCatalogEntry {
    pub id: SkillId,
    pub description: String,
    pub source: SkillSourceView,
    pub content_digest: ContentDigest,
    pub compatibility: SkillCompatibility,
    pub availability: SkillAvailability,
}
```

排序必须确定，相同 generation 的序列化结果稳定。只有 consumer-visible metadata、digest、
availability 或 diagnostic 变化才递增 generation。

当前 `zeta-file-watcher` 已提供共享、多订阅者的 filesystem invalidation substrate：
`PathsChanged` 传递排序去重后的 coarse path hint，backend error/overflow 通过
`RescanRequired` 传递 subscriber 自己的 watched roots。其 backend/ref-count/path fallback contract
由 [`zeta-rs/file-watcher/README.md`](../zeta-rs/file-watcher/README.md) 维护。

Watcher 仍只发 invalidation hint。Skill manager（尚未实现）必须重新扫描/校验后再发布 snapshot，
不能把 watch event 当作事实；`RescanRequired` 触发 scoped full rescan。`zeta-file-watcher`
已实现不代表 Skill catalog manager、generation 发布或 App Server refresh path 已完成。

## 8. Selection

### 8.1 显式选择

用户通过 Skill picker、slash mention 或 `turn/start` typed input 选择 exact `SkillRef`。显式选择：

- 优先于自动匹配；
- 仍要通过 enablement、availability、compatibility 和 trust policy；
- 若 pinned digest 已变化，返回 `SkillContentChanged`，不能悄悄跟随新内容；
- 若 source 不再存在，historical display 仍保留 ID/digest，但新 Turn 不能激活；
- 不允许客户端提供 catalog 外 absolute path。

当前 `UserInput::Skill { name, path }` 应迁移为类似：

```text
UserInput::Skill {
  skill: SkillRef
}
```

具体 serde shape 需在实现 vertical slice 时同步 protocol、App Server schema、Desktop、CLI、TUI
和 contract fixtures，开发期不保留双入口。

### 8.2 自动选择

自动选择是候选检索，不是把所有 Skill body 注入模型：

```text
enabled catalog
→ compatibility/source/policy filter
→ description + user intent retrieval
→ bounded candidate shortlist
→ deterministic threshold/ambiguity handling
→ activate selected Skill
```

第一版可以使用 lexical/keyword matcher；后续 embedding 或 model router 必须可测、可解释并受
候选数/延迟预算限制。

规则：

- `description` 是唯一 startup selection text；
- 低置信度时不自动激活；
- 自动 Skill 数量有上限；
- 互斥或顺序依赖在没有 accepted contract 前不通过自由文本猜测；
- 自动选择结果必须记录 Skill ID、digest、reason 和 catalog generation；
- 用户明确说不使用某 Skill 时，当前 Turn 不得重新自动加入；
- Skill 不能通过 description 要求自己在所有任务激活。

## 9. Activation 与渐进加载

[Agent Skills progressive disclosure](https://agentskills.io/specification#progressive-disclosure)
分三层：

| Level | 内容 | 加载时机 |
| --- | --- | --- |
| Metadata | name + description + source/compatibility | catalog/selection |
| Instructions | 完整 `SKILL.md` body | Skill 激活 |
| Resources | references/scripts/assets | 指令明确需要且 Agent 决定读取 |

激活流程：

```text
resolve exact SkillId + version selector
→ open file relative to immutable/validated source root
→ revalidate frontmatter and digest
→ read complete bounded SKILL.md
→ parse referenced-path inventory
→ build instruction fragment with provenance
→ publish SkillActivationSnapshot
```

```rust
pub struct SkillActivationSnapshot {
    pub id: SkillId,
    pub source: SkillSourceView,
    pub content_digest: ContentDigest,
    pub catalog_generation: u64,
    pub instructions: String,
    pub root: SkillRootHandle,
    pub diagnostics: Vec<SkillDiagnostic>,
}
```

snapshot 对一次 model invocation 不可变。Skill 文件变化只使 catalog stale；已经开始的调用仍使用
旧 bytes/digest。下一个 model safe point 重新解析，并在 pinned mismatch 时要求用户决定。

## 10. Context layering

Core ContextAssembler 使用明确层级：

```text
1. system safety / platform policy
2. Zeta developer/product instructions
3. workspace policy and active Turn constraints
4. activated Skill instructions（带 source + digest 边界）
5. user input
6. retrieved references/resources/tool results（不可信数据）
```

Skill 不能把自己声明为更高层。多个 Skill 的顺序由 activation controller 确定并记录，不依赖
filesystem enumeration order。

每个 instruction fragment 包含：

- exact Skill ID；
- source kind；
- digest；
- activation reason：explicit/automatic/dependency；
- bounded body；
- 冲突/compatibility warning。

Core ContextAssembler 应用总 token budget。超限时：

1. 不截断 frontmatter 与 body 到无法解释；
2. 优先减少自动 Skill 数；
3. 显式 Skill 仍超限则返回可见错误或要求用户选择；
4. references 不预载；
5. 不能把被截断的 Skill 标成完整激活。

## 11. References、scripts 与 assets

### 11.1 File resolver

所有 Skill 文件通过 rooted resolver：

```rust
/// Resolves bounded, read-only files beneath one validated Skill root.
///
/// Implementations must reject absolute paths, traversal, escaping links, special files and
/// content that exceeds the supplied limit. A returned file must retain its digest and provenance.
pub trait SkillFileResolver: Send + Sync {
    fn read(
        &self,
        request: SkillFileReadRequest,
    ) -> impl Future<Output = Result<SkillFile, SkillFileError>> + Send;
}
```

request 使用 `SkillRelativePath`、`SkillFileKind` 和具名 size policy，不使用 raw `PathBuf` 或
`allow_large: bool`。

相对引用从 Skill root 解析。按 Agent Skills guidance，文档应避免深层 reference chain；Zeta
另外施加最大 reference depth 和 visited-set，检测循环。

### 11.2 References

- 作为不可信 data/context 读取，不提升 instruction precedence；
- 只有当前任务需要时加载；
- 保留 source path、digest 和 MIME；
- Markdown 中的外部 URL 不自动 fetch；
- reference 再引用的文件仍通过同一 resolver 和 depth budget；
- 超大表格/文档通过 Resource store 分块，而不是塞入单个 model message。

### 11.3 Scripts

`scripts/` 中存在文件不代表它可执行。Skill manager 只返回 file metadata/handle。

执行必须：

```text
Skill instruction suggests script
→ resolve exact script under Skill root
→ materialize command without shell concatenation
→ evaluate Plugin/source trust and executable grant
→ approval + sandbox + resource limits
→ zeta-tool-executor
→ durable Tool Call/Result
```

独立 user/workspace Skill 默认没有 Plugin activation grant。用户可以显式通过普通 exec tool 运行
已审阅脚本，但仍遵守 workspace sandbox。`allowed-tools` 是 Agent Skills experimental field，
只能作为作者意图/兼容性提示，不能跳过 Zeta approval。

Skill manager 绝不自动执行：

- dependency installer；
- `postinstall` / `setup` / shell hook；
- 网络下载器；
- 脚本用来“探测 compatibility”；
- 首次激活 callback。

### 11.4 Assets

Assets 是模板、图片或静态数据：

- 不自动加入 context；
- 按 MIME/size/digest 读取；
- 写入工作区时必须通过对应 tool 和 approval；
- 模板展开后的结果是新 artifact，不反向修改 immutable Skill；
- HTML/SVG 等 active content 在 UI preview 中必须 sanitize/sandbox。

## 12. Compatibility

Agent Skills `compatibility` 是自由文本，不能作为 machine-enforced permission。Zeta 处理为：

```text
Compatible
Incompatible { reason }
Unknown { warning }
RequiresUserAction { requirements }
```

BuiltIn/Plugin Skill 可以由受控 manifest contribution 提供额外 machine-readable requirement：

```text
required Zeta version
required tool capability
required MCP contribution ID
required platform/architecture
required package-relative asset
```

这些 requirement 由 Plugin/Skill resolver 校验，不能藏在 Markdown 并在运行时自动安装。

如果 Skill 依赖某 MCP tool：

- dependency 指向 stable namespaced contribution ID，不指向 model-facing alias；
- MCP server unavailable 时 Skill 可以 `Blocked` 或按 manifest 明确声明 `Degraded`；
- 不能通过 tool name 字符串碰撞绑定到另一个 Plugin/server；
- Skill 激活 snapshot 和 MCP tool catalog snapshot 必须在同一 App Server composition generation
  中解析。

## 13. Trust、prompt injection 与数据安全

Skill instructions 可能恶意或被供应链篡改。所有来源都需要 provenance：

```text
BuiltInVerified
SignedPlugin { publisher, digest }
UnsignedLocalDevelopment
UserManaged
WorkspaceManaged
```

trust label 不改变 instruction precedence。即使 BuiltIn Skill 也不能绕过 platform safety。

必须防御：

- frontmatter/YAML alias 或 parser resource exhaustion；
- Markdown/HTML active content；
- path traversal、absolute path、symlink/hardlink escape；
- reference cycle 和 recursive expansion；
- oversized body/file/tree；
- Skill 名称 Unicode/confusable collision；
- workspace repository 提交 Skill 后自动执行；
- 指令要求读取或发送 credential；
- 指令伪造 system/developer message；
- 更新后同 ID 内容改变但 historical/pinned invocation 无提示；
- Skill asset 在 Renderer 中触发 script/network。

默认不把 Skill body、references、scripts 或 assets 写入 telemetry。可记录 ID、source、digest、
activation reason、bytes/token count、duration 和 error code。

## 14. Persistence、recovery 与 provenance

Skill catalog 是可重建 projection，不是 authority。Authority 分布为：

- BuiltIn Skill：Zeta release；
- Plugin Skill：Plugin installed/activation authority；
- User/workspace Skill：显式 configured source；
- current Turn selection：typed UserInput/command。

Thread history至少应保留用户显式选择的 stable Skill identity；是否把自动 activation 作为
canonical durable fact，需要在真实 Agent vertical slice 中按恢复/审计需求评审。

无论是否进入 Thread event，每次 model invocation 的 diagnostic/provenance snapshot 都应记录：

```text
SkillId
source
content digest
catalog generation
activation reason
loaded references and digests
effective instruction bytes/token estimate
```

Skill 内容本身不复制进 canonical Thread snapshot。Historical display 即使 source 已删除，也可依赖
stored identity/digest；重新执行必须重新授权/解析，不能假定旧 path 仍安全。

## 15. App Server API 与客户端

目标 surface：

| Method | 语义 |
| --- | --- |
| `skill/list` | 读取 metadata-only catalog snapshot |
| `skill/read` | 读取 entry metadata/diagnostics，不默认返回完整 body |
| `skill/content/read` | 通过 Resource/分块读取受控内容 |
| `skill/reload` | 显式使 source stale 并触发 rescan |
| `turn/start` Skill input | 选择 exact `SkillRef` |

Skill install/remove 不属于 Skill manager：

- Plugin Skill 通过 Plugin methods 管理；
- user/workspace standalone Skill 通过明确的 source/config 或将来 artifact import 管理；
- 客户端不能向 `skill/read` 传 arbitrary filesystem path。

Client picker 展示 name、description、source、version/digest 摘要、compatibility、availability 和
trust；同名 Skill 不合并。自动激活结果在 Turn/status 中可解释，但 UI 不需要显示 Skill 正文。

## 16. 错误与 diagnostics

至少区分：

```text
SourceUnavailable
SkillNotFound
SkillAmbiguous
InvalidFrontmatter
InvalidSkillName
DescriptionInvalid
PathEscapesRoot
UnsupportedFileType
ContentTooLarge
ReferenceCycle
ContentChanged
Incompatible
DependencyUnavailable
PermissionRequired
CatalogStale
```

错误携带安全的 Skill/source identity 和相对 path；不返回 host private absolute root、Skill 正文
或 secret。diagnostic 必须能解释为什么 Skill 没被发现、没被选择、没被激活或只处于 degraded。

## 17. 性能与 budget

建议第一版本地上限作为 Zeta policy，而不是 Agent Skills 标准：

| 项目 | 初始 policy |
| --- | --- |
| 每 source Skill 数 | 有界，超限产生 source diagnostic |
| frontmatter bytes | 小型固定上限 |
| `SKILL.md` bytes | 与 context budget 联动，硬上限独立存在 |
| activated automatic Skills | 小数量上限 |
| reference depth | 1 为推荐，硬上限不超过少量层级 |
| loaded reference bytes | per invocation 总预算 |
| file count/tree depth | 防目录/归档资源耗尽 |

具体数值在实现 benchmark 和真实 Skill corpus 后固定到 policy/config。测试注入 tokenizer estimator
和 fake filesystem，不依赖真实 home directory。

## 18. 目标目录

```text
zeta-rs/skills/src/
├── lib.rs
├── identity.rs
├── source.rs
├── format/
│   ├── mod.rs
│   ├── frontmatter.rs
│   └── validation.rs
├── catalog/
│   ├── mod.rs
│   ├── scanner.rs
│   ├── entry.rs
│   └── snapshot.rs
├── selection/
│   ├── mod.rs
│   ├── explicit.rs
│   └── automatic.rs
├── activation/
│   ├── mod.rs
│   ├── loader.rs
│   ├── resolver.rs
│   └── snapshot.rs
├── compatibility.rs
├── diagnostic.rs
├── error.rs
└── *_tests.rs
```

不建立同时负责 Plugin/MCP/Skill 的 `extensions` 巨型 crate。实现模块超过约 500 LoC 时按
catalog/selection/activation 拆分；新 trait 必须有职责和实现约束 doc comment；新测试模块使用
显式 sibling `#[path = "..._tests.rs"]`。

## 19. 分阶段实施

### Phase S0：format 与 catalog

- Agent Skills frontmatter/parser/validator；
- BuiltIn + controlled user source；
- metadata-only scan、digest 和 immutable catalog；
- path/size/cycle security fixtures；
- `skill/list/read` read-only API。

完成条件：启动不会加载所有 Skill body，坏 entry 不会逃出 root 或拖垮整个 catalog。

### Phase S1：显式选择 vertical slice

- `SkillId`/`SkillRef` contract；
- 迁移 `UserInput::Skill { name, path }`；
- 激活时完整加载 `SKILL.md`；
- Core ContextAssembler layering、budget 和 provenance；
- Desktop/CLI/TUI picker 使用同一 App Server contract。

完成条件：客户端不能通过 raw path 激活 catalog 外文件，已开始 invocation 使用 frozen digest。

### Phase S2：references/assets/scripts

- rooted file resolver 和 Resource integration；
- 按需 references/assets；
- scripts 只通过 exec/tool/approval/sandbox；
- reference cycle、MIME 和 active-content policy。

完成条件：读取 Skill 不产生副作用，运行脚本一定产生标准 durable Tool Call/Result。

### Phase S3：Plugin 与 MCP dependency

- Plugin Skill source；
- composition generation；
- machine-readable MCP/tool requirements；
- Plugin update/disable 后 snapshot drain。

完成条件：Skill 不能按名称碰撞绑定错误工具，Plugin update 不改变 in-flight invocation。

### Phase S4：自动选择

- bounded lexical retrieval；
- threshold、ambiguity 和 explicit opt-out；
- activation explanation 与 quality evaluation；
- 有证据后再评审 embedding/model router。

完成条件：自动选择有稳定离线评测，低置信度不激活，context 不随 catalog 数量线性增长。

## 20. 验证门

除常规 workspace 检查外，必须覆盖：

- Agent Skills name/description/frontmatter contract；
- YAML/parser resource exhaustion；
- metadata-only startup 与 body lazy load；
- source/name ambiguity 和 deterministic order；
- raw path rejection、traversal、symlink/hardlink 和 normalization collision；
- file change race、digest pin 和 catalog generation；
- explicit selection、auto threshold、opt-out 和 max active Skill；
- instruction precedence 与 prompt injection boundary；
- context/token/bytes budget；
- reference cycle/depth、external URL 不自动 fetch；
- script 不被 loader 执行且必须经过 exec/approval/sandbox；
- Plugin/MCP dependency identity、disable/update/drain；
- schema/telemetry/error 不泄露 Skill 正文、secret 或 private absolute root。
