# `zeta-windows-sandbox`

> 本 README 拥有 shared sandbox policy 到 Windows AppContainer enforcement 的实现契约；
> package helper contract 见
> [`build/release/zeta_package`](../../build/release/zeta_package/README.md)，机器级配置入口见
> [`zeta-windows-sandbox-service`](../windows-sandbox-service/README.md)，跨平台决策见
> [`docs/sandboxing.md`](../../docs/sandboxing.md)。
> 真实 Windows 验收步骤与 golden expectations 见
> [`Windows Sandbox 手工验收 Runbook`](../../docs/windows-sandbox-acceptance-runbook.md)。

当前 backend 支持 `ReadOnly + Denied` 和 `DirectoryWrite + Denied`。其他受限
policy 返回 `BackendUnavailable`，不会降级为普通进程。`FullAccess + Allowed` 仍按共享
contract 直接执行，不进入 helper。

## 执行路径

```text
InstallContext
└─ WindowsCommandRunner candidates
   ↓ canonicalize + --probe
WindowsSandbox::prepare
   ↓
zeta-command-runner.exe
├─ authenticate ZetaSandboxService through Windows Service Manager
├─ send one bounded request to \\.\pipe\Zeta.Sandbox
├─ ZetaSandboxService
│  ├─ authenticate the command runner digest and calling user
│  ├─ pin caller-authorized dir/program paths
│  └─ launch zeta-windows-sandbox-worker.exe in a one-process Job Object
├─ zeta-windows-sandbox-worker.exe
│  ├─ create/derive dir-and-access-scoped profile SID
│  └─ install dir/program ACLs
└─ CreateProcessW
   ├─ PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES
   ├─ zero capabilities (network denied)
   ├─ restricted child-process policy
   └─ Job Object: kill-on-close + one active process
```

`discovery::discover_helper` 验证 command runner 是普通 executable，执行精确 protocol probe，
canonicalize 后冻结路径。显式 `ZETA_WINDOWS_COMMAND_RUNNER_PATH` 无效时直接失败；普通
package/PATH candidate 可以逐个尝试。service 和 worker 使用固定安装身份，不从 PATH 选择。

command runner 通过 Windows Service Manager 核对命名管道服务端 PID，只接受正在运行的
`ZetaSandboxService`。服务串行处理单连接，请求上限为 4 KiB，空闲读取期限为 5 秒；连接进程必须
名为 `zeta-command-runner.exe`，且 SHA-256 与服务受保护安装目录中的副本完全一致。服务采用
管道用户身份核对进程 token，要求调用者本身具备目标目录的 DACL 修改权限，并在配置完成前保持
目录、程序及其现有祖先的 handle 打开。普通用户不能借此让服务修改自己原本无权修改的路径。
实际 ACL 遍历在独立 worker 中执行；worker 在恢复运行前进入
`ActiveProcessLimit=1 + JobMemoryLimit=512 MiB` 的 Job Object，超过 120 秒会结束整个 Job，因此
异常目录树不会直接拖住或打崩长期运行的服务进程。

release command runner 必须通过 Windows Service Manager 的 PID 核对。`scripts/zeta-code/run.py`
和 `scripts/zeta-code/run_package.py` 在 Windows 调试构建中从同一个内容寻址目录启动
`--foreground` 服务，
并只对带 `debug_assertions` 的 runner 设置 `ZETA_WINDOWS_SANDBOX_SERVICE_FOREGROUND=1`；正式
二进制不包含这条开发授权。

backend 以 canonical `Dir` path 和 `ro`/`rw` access mode 的 SHA-256 前缀派生 profile 名；
不同 `Dir` 不复用 AppContainer identity，写模式也不会向只读模式累积 authority。
service-owned worker 创建或复用该 AppContainer profile，并把 profile SID 的 read/execute 权限授予
对应 `Dir`。runner 先把冻结的 inner program 复制到本次调用独有的用户 temp directory；
worker 只给该目录与 staged program 授予 read/execute，child 结束后 runner 清理它。因此安装在
`Program Files` 时也不要求修改随包 `rg.exe` 的 DACL。`DirectoryWrite` 额外授予 `Dir` 写入权限，同时对
`.git` 等 `PROTECTED_DIR_METADATA_NAMES` 显式递归安装 write/delete deny ACE；递归不跟随
reparse point，任一 ACL 操作失败都会阻止 child 启动。ACL 是持久的 Windows filesystem metadata，
不是进程退出后自动撤销的临时 mount。

`runner::launch` 不向 AppContainer 提供 network capability，并用 child-process policy 与 Job
Object 阻止 rg 建立额外进程树。service、worker、profile 创建、ACL、attribute list、Job assignment 或
spawn 任一步失败，runner 都输出私有 diagnostic marker 并返回保留 exit code。inner process
若恰好返回该 code，runner 会先重映射；backend 因此只信任不可由 child 透传的保留状态，把它
分类为 start-before-process sandbox denial。普通 non-zero exit 或伪造 stderr marker 不会被误报。

## 当前限制

- 该实现没有复刻 Codex 的 dedicated local users、private desktop 或 WFP firewall backend；
  Zeta 使用 Windows AppContainer 的 package SID、capability 和 ACL 模型。
- 目前只为 built-in、固定 executable 的无网络 local process（当前是 `rg`）接线；不支持任意
  shell、PTY、网络代理或动态 capability。
- [`build/release/windows_sandbox_runtime.py`](../../build/release/windows_sandbox_runtime.py)
  从完整 Zeta Windows package 生成 per-machine MSI，把服务、worker 和 runner 安装到受保护的
  `Program Files/Zeta/Sandbox` 并注册固定 `ZetaSandboxService`。卸载会停止并删除服务文件；
  AppContainer profiles 与目录 ACL 仍是持久状态，长期使用过的 `Dir` 会保留只授予其 scoped
  profile SID 的 ACE。
- Windows API 已通过 MSVC target 的 Rust 交叉检查，但仍需要 Windows CI 的真实
  AppContainer、ACL、网络和 cancellation/kill-tree integration tests，才能标记为
  production-enforced。

已经完成、可以在非 Windows 主机完成、必须在真实 Windows 完成以及尚未实现的项目，统一记录在
[`Windows Sandbox 手工验收 Runbook`](../../docs/windows-sandbox-acceptance-runbook.md) 开头的
交接表中；发布前还必须回填该文档的 WPK、WPR、WSV 和 WRT 结果。

修改 profile 名、ACL mask/递归、process attributes、probe protocol、helper 名称或 denial
marker 时，必须同步本 crate tests、`zeta-install-context`、package builder、App Server
composition 和系统文档。

```bash
just test zeta-windows-sandbox
just check zeta-windows-sandbox --target x86_64-pc-windows-msvc --all-targets
just check zeta-windows-sandbox-service --target x86_64-pc-windows-msvc --all-targets
```
