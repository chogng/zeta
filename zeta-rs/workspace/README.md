# `zeta-workspace`

> 本 README 拥有单个本地 Workspace 文件系统边界及其 execution-trust capability 的实现契约。
> 跨 crate 的产品语义与 staged evolution 由
> [`docs/workspace-security.md`](../../docs/workspace-security.md) 维护。

`zeta-workspace` 是 Workspace root identity、containment、observer-path projection 和 root-bound
trust token 的 canonical Rust owner。它不拥有 Workspace picker、Config persistence 或 App
Server runtime orchestration。

## 所有权

| Symbol | 精确契约 | 明确不拥有 |
| --- | --- | --- |
| `WorkspaceRoot::open` | 冻结 requested 与 canonical directory namespace | Workspace discovery UI 或 persistence |
| `WorkspaceRoot::resolve_existing` | 跟随 symlink，并证明 existing target 保持 contained | handle-relative I/O |
| `WorkspaceRoot::resolve_for_write` | 通过最近 existing ancestor 校验 new target | 后续 write operation |
| `WorkspaceRoot::project_observed_path` | 把 requested/canonical watcher path 投影到一个 relative namespace | watcher lifecycle 或 debounce |
| `WorkspaceRoot::relative_to_existing_ancestor` | 为 nested Git worktree 提供 canonical projection | Git status semantics |
| `WorkspaceRoot::trust_id` / `WorkspaceTrustId` | 生成 host persistence 使用的 opaque canonical-root key | User Config storage 或 trust policy |
| `WorkspaceBinding::from_root` | 将 canonical root 与 authority ID 冻结为可持久化 Session binding | 当前 trust decision、runtime registry 或 Session storage |
| `WorkspaceAuthorization::revoke` | 失效一个 host decision 签发的全部 capability token | runtime teardown orchestration |
| `TrustedWorkspace::require` | 将 host trust decision 转成绑定 exact root 的 revocable token | trust UI、trust-store persistence、organization policy resolution |

Private `root::WorkspaceRoot::candidate` 与 `root::WorkspaceRoot::ensure_contained` 承载 lexical 和
canonical containment。`trust::TrustedWorkspace::require` 是 executable capability token 的唯一
constructor：

```text
host-selected path
  → WorkspaceRoot::open
  → host trust-store / policy decision
  → TrustedWorkspace::require(capability)
  → Terminal / Tool / extension / repository mutation owner
```

## 失败语义

- Root 无法 absolute、无法 canonicalize 或不是 directory 时，`WorkspaceRoot::open` 失败。
- Existing path 跟随 symlink 后离开 canonical root 时，resolution 失败即关闭。
- Write target 拒绝 parent、root 与 platform prefix component，并校验最近 existing ancestor；
  caller 仍须使用安全 I/O 并处理 race。
- Observer projection 不 canonicalize changed path，因此已删除 entry 仍可投影；两个冻结
  namespace 之外的 path 被忽略。
- Restricted trust decision 不能生成 `TrustedWorkspace`；revocation 后，已经签发的 token 也会在
  `TrustedWorkspace::ensure_active` 失败。

## 集成义务

安全的 read、browse、edit、Search 与 watch service 可以保留 `WorkspaceRoot`。Process launch、
Workspace-declared Tool、executable configuration、extension activation 与 repository mutation
必须保留 `TrustedWorkspace`，或在 authoritative entry point 执行等价 capability check。Client
提交的 path 或 Workspace config file 不能授予 trust。

`WorkspaceRoot` 是 path boundary，不是完整 TOCTOU defense。需要抵御 hostile concurrent
filesystem mutation 的 caller 必须增加 platform handle-relative operation（例如 `openat`），
不能假设先前的 path check 会让后续 open 变成 atomic。

`WorkspaceTrustId` hash canonical path 的平台原生 bytes，因此 persistence 不存 cleartext path。
Workspace 移动后 identity 会变化；canonical alias 会得到同一 identity。当前它不能检测同一路径
上的 filesystem object replacement。

`WorkspaceBinding` 会额外保存 canonical root，供产品重新打开 Session 所属 Workspace。consumer
必须重新执行 `WorkspaceRoot::open`、比对 authority ID，并重新解析当前 trust decision；binding
本身不授予文件或执行权限。Session event ownership 与跨 Workspace runtime routing 仍分别属于
`zeta-core`/`zeta-session-store` 和 `zeta-app-server`/`zeta-server-host`。

## 测试与修改影响

`root_tests.rs` 覆盖 directory validation、lexical escape rejection、symlink escape rejection 与
dual-namespace watcher projection。`trust_tests.rs` 覆盖 Restricted denial、exact-root binding 与
revocation。`binding_tests.rs` 覆盖 canonical binding 与 authority mismatch。Identity 或 projection
变化会影响 Files、sandbox、watcher 和 Git consumer；trust
capability semantics 变化会影响全部 executable Workspace runtime。

下列代码形态表示 architecture drift：

- 把 Editor、Git 或 runtime orchestration 移进本 crate；
- 用 raw `PathBuf` 直接构造 executable service；
- 把 canonical containment 等同于 user trust；
- 允许 Workspace config 自行生成 trust decision。

## 当前限制与扩展点

当前 crate 有意只建模一个 root，不拥有附加目录访问作用域。主工作目录与附加目录的角色、directory source lifetime、canonical deduplication 和 contribution policy 已由 [`zeta-workspace-access`](../workspace-access/README.md) 独立拥有；这些规则不能回流到 `WorkspaceRoot` 或 trust token。

App Server 负责为 `zeta-workspace-access` authority 中的每个 root 解析 capability，并只重建 authorized consumer。`/cd` 是独立 authority-switch 路径，负责替换主工作目录并重新加载完整项目配置。

`zeta-agent-import` 可以为 `zeta-workspace-access` adapter 提供安全的 source-specific inspection，但 Import workflow 与 directory authorization 必须保持分离。一次性 user import root 不能隐式变成 persistent additional root；反过来，持久 file-access-only root 也不能因为可访问就触发 Agent import。
