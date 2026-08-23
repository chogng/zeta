# `zeta-windows-sandbox`

> 本 README 拥有 shared sandbox policy 到 Windows AppContainer enforcement 的实现契约；
> package helper contract 见
> [`build/release/zeta_package`](../../build/release/zeta_package/README.md)，跨平台决策见
> [`docs/sandboxing.md`](../../docs/sandboxing.md)。
> 真实 Windows 验收步骤与 golden expectations 见
> [`Windows Sandbox 手工验收 Runbook`](../../docs/windows-sandbox-acceptance-runbook.md)。

当前 backend 支持 `ReadOnly + NetworkDenied` 和 `WorkspaceWrite + NetworkDenied`。其他受限
policy 返回 `BackendUnavailable`，不会降级为普通进程。`FullAccess + NetworkAllowed` 仍按共享
contract 直接执行，不进入 helper。

## 执行路径

```text
InstallContext
├─ WindowsCommandRunner candidates
└─ WindowsSandboxSetup candidates
   ↓ canonicalize + --zeta-sandbox-probe
WindowsSandbox::prepare
   ↓
zeta-command-runner.exe
├─ zeta-windows-sandbox-setup.exe
│  ├─ create/derive workspace-and-access-scoped profile SID
│  └─ install workspace/program ACLs
└─ CreateProcessW
   ├─ PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES
   ├─ zero capabilities (network denied)
   ├─ restricted child-process policy
   └─ Job Object: kill-on-close + one active process
```

`discovery::discover_helper` 验证两个 helper 是普通 executable，执行精确 protocol probe，
canonicalize 后冻结路径。显式环境 override
`ZETA_WINDOWS_COMMAND_RUNNER_PATH` / `ZETA_WINDOWS_SANDBOX_SETUP_PATH` 无效时直接失败；普通
package/PATH candidate 可以逐个尝试。

backend 以 canonical Workspace path 和 `ro`/`rw` access mode 的 SHA-256 前缀派生 profile 名；
不同 Workspace 不复用 AppContainer identity，写模式也不会向只读模式累积 authority。
`setup::run` 创建或复用该 AppContainer profile，并把 profile SID 的 read/execute 权限授予
对应 Workspace。runner 先把冻结的 inner program 复制到本次调用独有的用户 temp directory；
setup 只给该目录与 staged program 授予 read/execute，child 结束后 runner 清理它。因此安装在
`Program Files` 时也不要求修改随包 `rg.exe` 的 DACL。WorkspaceWrite 额外授予 Workspace 写入权限，同时对
`.git` 等 `PROTECTED_WORKSPACE_METADATA_NAMES` 显式递归安装 write/delete deny ACE；递归不跟随
reparse point，任一 ACL 操作失败都会阻止 child 启动。ACL 是持久的 Windows filesystem metadata，
不是进程退出后自动撤销的临时 mount。

`runner::launch` 不向 AppContainer 提供 network capability，并用 child-process policy 与 Job
Object 阻止 rg 建立额外进程树。setup、profile 创建、ACL、attribute list、Job assignment 或
spawn 任一步失败，runner 都输出私有 diagnostic marker 并返回保留 exit code。inner process
若恰好返回该 code，runner 会先重映射；backend 因此只信任不可由 child 透传的保留状态，把它
分类为 start-before-process sandbox denial。普通 non-zero exit 或伪造 stderr marker 不会被误报。

## 当前限制

- 该实现没有复刻 Codex 的 dedicated local users、private desktop 或 WFP firewall backend；
  Zeta v1 使用原生 AppContainer 的 package SID、capability 和 ACL 模型。
- 目前只为 built-in、固定 executable 的无网络 local process（当前是 `rg`）接线；不支持任意
  shell、PTY、网络代理或动态 capability。
- AppContainer profiles 与 ACL 是持久状态；目前没有 installer/uninstaller cleanup，长期使用过
  的 Workspace 会保留只授予其 scoped profile SID 的 ACE。
- Windows API 已通过 MSVC target 的 Rust 交叉检查，但仍需要 Windows CI 的真实
  AppContainer、ACL、网络和 cancellation/kill-tree integration tests，才能标记为
  production-enforced。

修改 profile 名、ACL mask/递归、process attributes、probe protocol、helper 名称或 denial
marker 时，必须同步本 crate tests、`zeta-install-context`、package builder、App Server
composition 和系统文档。

```bash
cargo test --manifest-path Cargo.toml -p zeta-windows-sandbox
cargo check --manifest-path Cargo.toml \
  --target x86_64-pc-windows-msvc -p zeta-windows-sandbox --all-targets
```
