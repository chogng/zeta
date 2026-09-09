# Windows 沙箱手工验收手册

> 状态：Current validation procedure；真实 Windows 结果待回填。
>
> 本文拥有 Windows AppContainer sandbox 的手工验收步骤、golden expectations 和结果回填格式。
> 实现契约见
> [`zeta-windows-sandbox` README](../zeta-rs/windows-sandbox-rs/README.md)，跨平台边界见
> [`sandboxing.md`](sandboxing.md)。

## 快速理解

本文属于：

- **Manual Acceptance Test Plan**：手工验收测试计划；
- **Acceptance Runbook**：可直接交给测试人员执行的验收操作手册；
- **Test Oracle**：用来判断实际结果 PASS/FAIL 的预期结果与规则；
- **Golden Expectations**：本文为 Test Oracle 固定下来的具体输出、exit code 和副作用。

测试人员只负责记录实际结果和证据，不应根据实际行为修改预期结果。实际结果与本文不一致时，
应作为实现缺陷、环境不满足或文档错误返回维护者分析。

| 你是谁 | 如何使用本文 | 不应该做什么 |
| --- | --- | --- |
| 测试人员 | 按顺序执行步骤并记录实际结果和证据 | 根据实际行为修改预期结果 |
| 实现维护者 | 分析失败属于实现、环境还是文档问题 | 把失败步骤直接标成通过 |
| 发布负责人 | 用完整结果判断 Windows 沙箱是否可交付 | 在缺少证据时推断通过 |

## 1. 当前验证状态与 Windows 交接

以下状态以 2026-09-09 的本次实现为准。`已完成` 表示命令已经实际通过；`待 Windows` 表示代码或
测试步骤已经存在，但必须在真实 Windows 上取得结果；`尚缺测试` 表示强制逻辑已经存在但验收
程序还没有实现；`尚未接线` 表示 release 流程仍缺实现，不能用手工测试结果代替。

| 项目 | 当前状态 | 在哪里做 | Windows 上要做什么 |
| --- | --- | --- | --- |
| `zeta-windows-sandbox` / service Rust 单测 | 已完成 | macOS/Linux/Windows 都可运行 | Windows 再运行第 24 节命令，确认目标机工具链一致 |
| 两个 crate 的 Windows x64 debug/release 编译 | 已完成 | 配好 Rust Windows target 的任意主机 | 无；ARM64 发布时仍须在 ARM64 target 重跑 |
| 完整 `zeta-app-server` Windows 编译 | 未完成 | 具备 Windows SDK C headers 的主机；建议 Windows | 运行第 24 节 App Server check；本次 macOS 失败是 `ring`/`aws-lc-sys` 找不到 `assert.h`、`stdlib.h`、`windows.h` |
| package、协议、MSI XML 生成测试 | 已完成 | 任意开发主机 | 按 WPK-01/WPK-02 用真实 Windows artifact 再核对摘要 |
| WiX 生成真实 Runtime MSI | 待 Windows | 安装 WiX 4 的 Windows | 执行 WPR-01，保存 `.wxs`、`.msi`、WiX 输出和 MSI exit code |
| Service install/start/stop/uninstall | 待 Windows | 真实 Windows，管理员批准 MSI | 执行 WPR-01 与第 21 节，保存 `Get-Service` 和卸载结果 |
| 命名管道、runner 摘要、用户 token、路径 pinning | 待 Windows | 真实 Windows | 完成 WSV-01/WSV-02；认证失败必须发生在 ACL worker 启动前 |
| worker 的单进程、512 MiB、120 秒 Job Object | 尚缺测试 | 先补 Windows-only integration test，再到真实 Windows 运行 | 完成 WSV-03，确认 worker 失败不会结束长期服务进程 |
| AppContainer 文件、网络、子进程与退出码 | 待 Windows | 真实 Windows | 完成 WRT-01 至 WRT-12 |
| `zeta code`、Electron `zeta`、Rust `app` release installer 链入同一签名 Runtime MSI | 尚未接线 | 三条 Windows release pipeline | 每条 installer 增加同一 Runtime MSI prerequisite；分别安装、升级、卸载并证明没有注册第二个服务 |
| AppContainer profile 与历史目录 ACL 清理 | 尚未实现 | Windows service/uninstaller | 不能宣称完全卸载；当前 MSI 只删除 service/runner/worker 文件 |

本机能做但本次环境没做的是“完整 App Server Windows 编译”；它不是 Windows 运行时测试。必须在
Windows 做的是 SCM、named pipe impersonation、Job Object、ACL、AppContainer 和 MSI 行为，因为
这些结果依赖真实 Windows 内核与文件系统，交叉编译只能证明代码能编译。

## 2. 验收范围

本 Runbook 验证当前已实现的 Windows v1 contract：

| 能力 | 验收目标 |
| --- | --- |
| Package | Windows 包必须包含 command runner、service、worker 和 `rg.exe` |
| Service identity | machine Runtime 注册固定服务，runner probe 与服务状态可验证 |
| ReadOnly | 可以读取 Dir，不能写入 |
| DirectoryWrite | 可以写 Dir，但不能写 protected metadata |
| Profile isolation | Dir 与 ro/rw mode 不累积 authority |
| Filesystem boundary | 不能读写 Dir 外的用户文件 |
| `Denied` network access | 不能访问 host loopback HTTP server |
| Process containment | sandboxed program 不能创建子进程 |
| Denial evidence | service/worker/pre-launch failure 使用保留 exit code `125` |
| Exit-code authenticity | inner process 的 `125` 被重映射为 `124` |
| Temporary program | staged executable 在 runner 退出后被清理 |

不在本次验收范围：

- Codex dedicated local users、private desktop 或 WFP firewall；
- shell/PTY 产品能力；
- managed network proxy；
- installer/uninstaller 对历史 AppContainer profile 和 ACL 的清理；
- 性能与大规模目录 benchmark。

## 3. 安全要求

必须满足：

1. 使用 Windows 10/11 x64 或 ARM64 的普通、非 elevated PowerShell；
2. 使用本文创建的临时目录；
3. **不要在真实代码仓库、用户文档目录或生产机器目录上执行 ACL 测试**；
4. 测试目录在结果交付前不要删除；
5. 不要使用公司 secret、token 或真实敏感文件作为 outside-dir fixture。
6. 使用没有安装其他 Zeta 产品的专用测试机；本流程会安装并最终移除共享
   `ZetaSandboxService` Runtime。

原因：AppContainer profile 和目录 ACL 是持久 Windows 状态，不会随测试进程退出自动撤销。

## 4. 环境与证据目录

从 Zeta repository root 打开 PowerShell 7，执行：

```powershell
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$Repo = (Resolve-Path ".").Path
$Target = "x86_64-pc-windows-msvc" # ARM64 改为 aarch64-pc-windows-msvc
$RunRoot = Join-Path $env:TEMP ("zeta-windows-sandbox-acceptance-" + [guid]::NewGuid())
$Package = Join-Path $RunRoot "package"
$DirA = Join-Path $RunRoot "dir-a"
$DirB = Join-Path $RunRoot "dir-b"
$Outside = Join-Path $RunRoot "outside"
$Transcript = Join-Path $RunRoot "acceptance-transcript.txt"

New-Item -ItemType Directory -Path $RunRoot | Out-Null
New-Item -ItemType Directory -Path $DirA | Out-Null
New-Item -ItemType Directory -Path $DirB | Out-Null
New-Item -ItemType Directory -Path $Outside | Out-Null
New-Item -ItemType Directory -Path (Join-Path $DirA ".git") | Out-Null

Start-Transcript -Path $Transcript

Get-ComputerInfo |
  Select-Object WindowsProductName, WindowsVersion, OsBuildNumber, OsArchitecture
rustc -Vv
cargo -V
python --version
wix --version

$Identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$Principal = [Security.Principal.WindowsPrincipal]::new($Identity)
$Elevated = $Principal.IsInRole(
  [Security.Principal.WindowsBuiltInRole]::Administrator
)
"Elevated=$Elevated"
```

Golden expectation：

- `Elevated=False`；
- Windows、Rust、Cargo、Python、WiX 信息完整写入 transcript；
- `$RunRoot`、两个 Dir 和 outside fixture 是全新临时目录。

如果 `Elevated=True`，停止测试并重新打开非管理员 PowerShell。

## 5. WPK-01：构建 canonical Windows 包

执行：

```powershell
python -B build/release/build_zeta_package.py `
  --target $Target `
  --package-dir $Package

if ($LASTEXITCODE -ne 0) {
  throw "package build failed with exit code $LASTEXITCODE"
}

$Runner = Join-Path $Package "zeta-resources\zeta-command-runner.exe"
$Service = Join-Path $Package "zeta-resources\zeta-windows-sandbox-service.exe"
$Worker = Join-Path $Package "zeta-resources\zeta-windows-sandbox-worker.exe"
$Rg = Join-Path $Package "zeta-path\rg.exe"
$MetadataPath = Join-Path $Package "zeta-package.json"
$Metadata = Get-Content $MetadataPath -Raw | ConvertFrom-Json

@($Runner, $Service, $Worker, $Rg, $MetadataPath) |
  ForEach-Object { [pscustomobject]@{ Path = $_; Exists = Test-Path $_ -PathType Leaf } } |
  Format-Table -AutoSize
```

Golden expectation：

- package build exit code 为 `0`；
- 五个文件的 `Exists` 全部为 `True`；
- `zeta-package.json.target` 等于 `$Target`；
- `components.windowsSandbox.source` 为 `cargo-build`，或使用显式 prebuilt helper 时为
  `local-override` / `mixed`。

## 6. WPK-02：校验 Windows Runtime 摘要

执行：

```powershell
$RunnerHash = (Get-FileHash $Runner -Algorithm SHA256).Hash.ToLowerInvariant()
$ServiceHash = (Get-FileHash $Service -Algorithm SHA256).Hash.ToLowerInvariant()
$WorkerHash = (Get-FileHash $Worker -Algorithm SHA256).Hash.ToLowerInvariant()

[pscustomobject]@{
  RunnerHashMatches = (
    $RunnerHash -eq $Metadata.components.windowsSandbox.commandRunnerSha256
  )
  ServiceHashMatches = (
    $ServiceHash -eq $Metadata.components.windowsSandbox.sandboxServiceSha256
  )
  WorkerHashMatches = (
    $WorkerHash -eq $Metadata.components.windowsSandbox.sandboxWorkerSha256
  )
} | Format-List
```

Golden expectation：

- `RunnerHashMatches=True`；
- `ServiceHashMatches=True`；
- `WorkerHashMatches=True`。

## 7. WPR-01：安装 Runtime 并验证服务身份

执行：

```powershell
$RunnerProbe = (& $Runner --probe | Out-String).Trim()
$RunnerProbeCode = $LASTEXITCODE
$RuntimeOutput = Join-Path $RunRoot "runtime"
python -B build/release/build_windows_sandbox_runtime.py `
  --target $Target `
  --package-dir $Package `
  --output-dir $RuntimeOutput
if ($LASTEXITCODE -ne 0) {
  throw "runtime MSI build failed with exit code $LASTEXITCODE"
}
$RuntimeMsi = Get-ChildItem $RuntimeOutput -Filter "*.msi" | Select-Object -ExpandProperty FullName
$Install = Start-Process msiexec.exe -Verb RunAs -Wait -PassThru -ArgumentList @(
  "/i", ('"' + $RuntimeMsi + '"'), "/qn", "/norestart"
)
$ServiceState = (Get-Service ZetaSandboxService).Status

[pscustomobject]@{
  RunnerCode = $RunnerProbeCode
  RunnerProbe = $RunnerProbe
  RuntimeInstallCode = $Install.ExitCode
  ServiceState = $ServiceState
} | Format-List
```

Golden expectation：

```text
RunnerCode  = 0
RunnerProbe = zeta-windows-command-runner-v1
RuntimeInstallCode = 0
ServiceState = Running
```

其他 runner 文字、不同 protocol version、MSI 安装错误或服务未运行都判定失败。

### 7.1 WSV-01：非 Zeta 进程不能请求配置

在普通 PowerShell 中直接连接管道，并确认服务拒绝 PowerShell 自身且没有启动 worker：

```powershell
$WorkerBefore = @(Get-Process zeta-windows-sandbox-worker -ErrorAction SilentlyContinue).Count
$Pipe = [IO.Pipes.NamedPipeClientStream]::new(
  ".", "Zeta.Sandbox", [IO.Pipes.PipeDirection]::InOut
)
$Pipe.Connect(2000)
$Invalid = [Text.Encoding]::UTF8.GetBytes("not-a-zeta-frame")
$Pipe.Write($Invalid, 0, $Invalid.Length)
$Pipe.Flush()
$Reader = [IO.BinaryReader]::new($Pipe, [Text.Encoding]::UTF8, $true)
$ResponseLength = $Reader.ReadUInt32()
$Response = [Text.Encoding]::UTF8.GetString(
  $Reader.ReadBytes($ResponseLength)
)
$Reader.Dispose()
$Pipe.Dispose()
$WorkerAfter = @(Get-Process zeta-windows-sandbox-worker -ErrorAction SilentlyContinue).Count

[pscustomobject]@{
  AuthenticationRejected = $Response.Contains(
    "sandbox command runner has an unexpected executable name"
  )
  WorkerStarted = ($WorkerAfter -gt $WorkerBefore)
  ServiceState = (Get-Service ZetaSandboxService).Status
} | Format-List
```

Golden expectation：`AuthenticationRejected=True`、`WorkerStarted=False`、`ServiceState=Running`。服务必须先
核对连接进程，再读取或执行请求内容。

### 7.2 WSV-02：被修改的 runner 摘要被拒绝

复制并修改 runner 的文件尾部，然后用它发起一条本来合法的请求：

```powershell
$TamperedDirectory = Join-Path $RunRoot "tampered-runner"
New-Item -ItemType Directory -Path $TamperedDirectory | Out-Null
$TamperedRunner = Join-Path $TamperedDirectory "zeta-command-runner.exe"
Copy-Item $Runner $TamperedRunner
[IO.File]::AppendAllText($TamperedRunner, "tampered")
$TamperedWorkerBefore = @(
  Get-Process zeta-windows-sandbox-worker -ErrorAction SilentlyContinue
).Count

$TamperedOutput = (
  & $TamperedRunner `
    --access read-only `
    --dir $DirA `
    --cwd $DirA `
    -- $Rg --files 2>&1 |
  Out-String
)
$TamperedCode = $LASTEXITCODE
$TamperedWorkerAfter = @(
  Get-Process zeta-windows-sandbox-worker -ErrorAction SilentlyContinue
).Count

[pscustomobject]@{
  ExitCode = $TamperedCode
  RejectedDigest = $TamperedOutput.Contains(
    "does not match the protected Zeta installation"
  )
  WorkerStarted = ($TamperedWorkerAfter -gt $TamperedWorkerBefore)
  ServiceState = (Get-Service ZetaSandboxService).Status
} | Format-List
```

Golden expectation：`ExitCode=125`、`RejectedDigest=True`、`WorkerStarted=False`、
`ServiceState=Running`。

### 7.3 WSV-03：worker 资源限制与服务存活

这一项目前**尚缺 Windows 自动化测试实现，不能手工标记为通过**。发布前必须在
`zeta-windows-sandbox-service` 增加仅测试目标可调用的 worker fixture，并在真实 Windows 上证明：

1. 第二个子进程被 `ActiveProcessLimit=1` 拒绝；
2. worker 超过 512 MiB 后由 Job Object 结束；
3. worker 超过测试参数缩短后的期限后，走与生产 120 秒相同的终止路径；
4. 三种失败后 `ZetaSandboxService` PID 不变，下一条正常配置请求仍成功；
5. Job handle 关闭后没有遗留 worker 或后代进程。

在该测试和 Windows 结果进入仓库前，结果表中的 WSV-03 必须填写 `BLOCKED`。

## 8. 创建测试样例

执行：

```powershell
$Sentinel = "ZETA_WINDOWS_SANDBOX_SENTINEL_7F3A"
$Needle = Join-Path $DirA "needle.txt"
$Secret = Join-Path $Outside "outside-secret.txt"
$DirBFile = Join-Path $DirB "dir-b.txt"
$FsProbeSource = Join-Path $Outside "fs-probe.rs"
$FsProbe = Join-Path $Outside "fs-probe.exe"
$Cmd = Join-Path $env:SystemRoot "System32\cmd.exe"
$Curl = Join-Path $env:SystemRoot "System32\curl.exe"

Set-Content -Path $Needle -Value $Sentinel -NoNewline
Set-Content -Path $Secret -Value $Sentinel -NoNewline
Set-Content -Path $DirBFile -Value "dir-b" -NoNewline

@'
use std::process::exit;

fn main() {
    let mut arguments = std::env::args_os().skip(1);
    let operation = arguments.next().expect("missing operation");
    match operation.to_str() {
        Some("read") => {
            let path = arguments.next().expect("missing read path");
            match std::fs::read_to_string(path) {
                Ok(value) => print!("{value}"),
                Err(error) => {
                    eprintln!("read-failed: {error}");
                    exit(31);
                }
            }
        }
        Some("write") => {
            let path = arguments.next().expect("missing write path");
            let value = arguments.next().expect("missing write value");
            if let Err(error) = std::fs::write(path, value.to_string_lossy().as_bytes()) {
                eprintln!("write-failed: {error}");
                exit(32);
            }
        }
        Some("exit") => {
            let code = arguments
                .next()
                .expect("missing exit code")
                .to_string_lossy()
                .parse()
                .expect("invalid exit code");
            exit(code);
        }
        _ => exit(2),
    }
}
'@ | Set-Content -Path $FsProbeSource -Encoding UTF8

rustc $FsProbeSource -o $FsProbe
if ($LASTEXITCODE -ne 0) {
  throw "filesystem probe compilation failed"
}

$StagedBefore = @(
  Get-ChildItem $env:TEMP -Directory -Filter "zeta-sandbox-program-*" -ErrorAction SilentlyContinue |
    Select-Object -ExpandProperty FullName
)
```

## 9. WRT-01：ReadOnly 可以运行打包的 rg

执行：

```powershell
$RgOutput = (
  & $Runner `
    --access read-only `
    --dir $DirA `
    --cwd $DirA `
    -- $Rg --no-heading --line-number $Sentinel . 2>&1 |
  Out-String
)
$RgCode = $LASTEXITCODE

[pscustomobject]@{
  ExitCode = $RgCode
  ContainsNeedle = $RgOutput.Contains("needle.txt")
  ContainsSentinel = $RgOutput.Contains($Sentinel)
  Output = $RgOutput.Trim()
} | Format-List
```

Golden expectation：

- `ExitCode=0`；
- `ContainsNeedle=True`；
- `ContainsSentinel=True`；
- output 不含 `zeta-windows-sandbox:`。

## 10. WRT-02：ReadOnly 拒绝 Dir 写入

执行：

```powershell
$ReadOnlyMarker = Join-Path $DirA "read-only-write.txt"
$ReadOnlyOutput = (
  & $Runner `
    --access read-only `
    --dir $DirA `
    --cwd $DirA `
    -- $FsProbe write $ReadOnlyMarker "blocked" 2>&1 |
  Out-String
)
$ReadOnlyCode = $LASTEXITCODE

[pscustomobject]@{
  ExitCode = $ReadOnlyCode
  MarkerExists = Test-Path $ReadOnlyMarker
  Output = $ReadOnlyOutput.Trim()
} | Format-List
```

Golden expectation：

- `ExitCode=32`；
- `MarkerExists=False`。

Windows 本地化后的 access-denied 文案不作为 golden string；文件不存在才是 authoritative result。

## 11. WRT-03：DirectoryWrite 允许普通 Dir 写入

执行：

```powershell
$WriteMarker = Join-Path $DirA "dir-write.txt"
$WriteOutput = (
  & $Runner `
    --access dir-write `
    --dir $DirA `
    --cwd $DirA `
    -- $FsProbe write $WriteMarker "dir-write-ok" 2>&1 |
  Out-String
)
$WriteCode = $LASTEXITCODE

[pscustomobject]@{
  ExitCode = $WriteCode
  MarkerExists = Test-Path $WriteMarker
  MarkerContent = if (Test-Path $WriteMarker) {
    (Get-Content $WriteMarker -Raw).Trim()
  } else {
    ""
  }
  Output = $WriteOutput.Trim()
} | Format-List
```

Golden expectation：

- `ExitCode=0`；
- `MarkerExists=True`；
- `MarkerContent=dir-write-ok`。

## 12. WRT-04：rw 配置档案不得污染同一 Dir 的 ro 配置档案

WRT-03 已经创建过 DirectoryWrite profile。现在再次以 ReadOnly 执行：

```powershell
$ReadAfterWriteMarker = Join-Path $DirA "ro-after-rw.txt"
$ReadAfterWriteOutput = (
  & $Runner `
    --access read-only `
    --dir $DirA `
    --cwd $DirA `
    -- $FsProbe write $ReadAfterWriteMarker "blocked" 2>&1 |
  Out-String
)
$ReadAfterWriteCode = $LASTEXITCODE

[pscustomobject]@{
  ExitCode = $ReadAfterWriteCode
  MarkerExists = Test-Path $ReadAfterWriteMarker
  Output = $ReadAfterWriteOutput.Trim()
} | Format-List
```

Golden expectation：

- `ExitCode=32`；
- `MarkerExists=False`。

如果文件被创建，说明 ro/rw profile authority 发生累积，必须判定失败。

## 13. WRT-05：DirectoryWrite 拒绝 protected 元数据

执行：

```powershell
$GitMarker = Join-Path $DirA ".git\zeta-write-probe.txt"
$GitOutput = (
  & $Runner `
    --access dir-write `
    --dir $DirA `
    --cwd $DirA `
    -- $FsProbe write $GitMarker "blocked" 2>&1 |
  Out-String
)
$GitCode = $LASTEXITCODE

[pscustomobject]@{
  ExitCode = $GitCode
  MarkerExists = Test-Path $GitMarker
  Output = $GitOutput.Trim()
} | Format-List
```

Golden expectation：

- `ExitCode=32`；
- `MarkerExists=False`。

## 14. WRT-06：拒绝读取与写入 Dir 外文件

执行读取测试：

```powershell
$OutsideReadOutput = (
  & $Runner `
    --access dir-write `
    --dir $DirA `
    --cwd $DirA `
    -- $FsProbe read $Secret 2>&1 |
  Out-String
)
$OutsideReadCode = $LASTEXITCODE

[pscustomobject]@{
  ExitCode = $OutsideReadCode
  SecretLeaked = $OutsideReadOutput.Contains($Sentinel)
  Output = $OutsideReadOutput.Trim()
} | Format-List
```

执行写入测试：

```powershell
$OutsideWriteMarker = Join-Path $Outside "outside-write.txt"
$OutsideWriteOutput = (
  & $Runner `
    --access dir-write `
    --dir $DirA `
    --cwd $DirA `
    -- $FsProbe write $OutsideWriteMarker "blocked" 2>&1 |
  Out-String
)
$OutsideWriteCode = $LASTEXITCODE

[pscustomobject]@{
  ExitCode = $OutsideWriteCode
  MarkerExists = Test-Path $OutsideWriteMarker
  Output = $OutsideWriteOutput.Trim()
} | Format-List
```

Golden expectation：

- read `ExitCode=31`；
- write `ExitCode=32`；
- `SecretLeaked=False`；
- `MarkerExists=False`。

## 15. WRT-07：不同 Dir 不得共享配置档案权威

Dir A 已经获得 ro 和 rw ACL。以 Dir B 的 ReadOnly profile 尝试读取 A：

```powershell
$CrossDirOutput = (
  & $Runner `
    --access read-only `
    --dir $DirB `
    --cwd $DirB `
    -- $FsProbe read $WriteMarker 2>&1 |
  Out-String
)
$CrossDirCode = $LASTEXITCODE

[pscustomobject]@{
  ExitCode = $CrossDirCode
  ContentLeaked = $CrossDirOutput.Contains("dir-write-ok")
  Output = $CrossDirOutput.Trim()
} | Format-List
```

Golden expectation：

- `ExitCode=31`；
- `ContentLeaked=False`。

## 16. WRT-08：`Denied` 网络策略拒绝宿主回环

先证明 host server 正常：

```powershell
$Server = Start-Process `
  -FilePath "python" `
  -ArgumentList "-m", "http.server", "8765", "--bind", "127.0.0.1" `
  -WorkingDirectory $DirA `
  -WindowStyle Hidden `
  -PassThru

Start-Sleep -Seconds 2

$HostNetworkOutput = (& $Curl --silent --show-error --max-time 5 `
  "http://127.0.0.1:8765/needle.txt" 2>&1 | Out-String)
$HostNetworkCode = $LASTEXITCODE

[pscustomobject]@{
  HostExitCode = $HostNetworkCode
  HostContainsSentinel = $HostNetworkOutput.Contains($Sentinel)
} | Format-List
```

环境前置 golden expectation：

- `HostExitCode=0`；
- `HostContainsSentinel=True`。

然后执行 sandboxed curl：

```powershell
try {
  $SandboxNetworkOutput = (
    & $Runner `
      --access read-only `
      --dir $DirA `
      --cwd $DirA `
      -- $Curl --silent --show-error --max-time 5 `
        "http://127.0.0.1:8765/needle.txt" 2>&1 |
    Out-String
  )
  $SandboxNetworkCode = $LASTEXITCODE

  [pscustomobject]@{
    ExitCode = $SandboxNetworkCode
    ResponseLeaked = $SandboxNetworkOutput.Contains($Sentinel)
    Output = $SandboxNetworkOutput.Trim()
  } | Format-List
}
finally {
  Stop-Process -Id $Server.Id -Force -ErrorAction SilentlyContinue
}
```

Golden expectation：

- `ExitCode` 非 `0`；
- `ResponseLeaked=False`。

如果 host 前置检查失败，本项记为 `BLOCKED`，不能记为 sandbox `PASS`。

## 17. WRT-09：拒绝创建子进程

创建并编译一个只用于验收的 native probe：

```powershell
$SpawnProbeSource = Join-Path $Outside "spawn-probe.rs"
$SpawnProbe = Join-Path $Outside "spawn-probe.exe"
$NestedMarker = Join-Path $DirA "nested-child.txt"

@'
use std::process::{Command, exit};

fn main() {
    let mut arguments = std::env::args_os().skip(1);
    let marker = arguments.next().expect("missing marker");
    let command = arguments.next().expect("missing command");
    let script = format!("echo nested>\"{}\"", marker.to_string_lossy());
    match Command::new(command)
        .args(["/d", "/s", "/c"])
        .arg(script)
        .status()
    {
        Err(error) => {
            eprintln!("spawn-blocked: {error}");
            exit(41);
        }
        Ok(status) => {
            eprintln!("child-started: {status}");
            exit(42);
        }
    }
}
'@ | Set-Content -Path $SpawnProbeSource -Encoding UTF8

rustc $SpawnProbeSource -o $SpawnProbe
if ($LASTEXITCODE -ne 0) {
  throw "spawn probe compilation failed"
}

$SpawnOutput = (
  & $Runner `
    --access dir-write `
    --dir $DirA `
    --cwd $DirA `
    -- $SpawnProbe $NestedMarker $Cmd 2>&1 |
  Out-String
)
$SpawnCode = $LASTEXITCODE

[pscustomobject]@{
  ExitCode = $SpawnCode
  MarkerExists = Test-Path $NestedMarker
  SpawnWasBlocked = $SpawnOutput.Contains("spawn-blocked:")
  Output = $SpawnOutput.Trim()
} | Format-List
```

Golden expectation：

- `ExitCode=41`；
- `MarkerExists=False`；
- `SpawnWasBlocked=True`；
- output 不含 `child-started:`。

## 18. WRT-10：inner exit code 不能伪造强制执行拒绝

执行：

```powershell
$ReservedOutput = (
  & $Runner `
    --access read-only `
    --dir $DirA `
    --cwd $DirA `
    -- $FsProbe exit 125 2>&1 |
  Out-String
)
$ReservedCode = $LASTEXITCODE

[pscustomobject]@{
  ExitCode = $ReservedCode
  HasEnforcementMarker = $ReservedOutput.Contains("zeta-windows-sandbox:")
  Output = $ReservedOutput.Trim()
} | Format-List
```

Golden expectation：

- `ExitCode=124`；
- `HasEnforcementMarker=False`。

`125` 是 runner 自己的可信 pre-launch failure code；child 返回的 `125` 必须被重映射。

## 19. WRT-11：真实 pre-launch 失败使用保留状态

故意把 cwd 指向 Dir 外：

```powershell
$PrelaunchOutput = (
  & $Runner `
    --access read-only `
    --dir $DirA `
    --cwd $DirB `
    -- $Rg --files 2>&1 |
  Out-String
)
$PrelaunchCode = $LASTEXITCODE

[pscustomobject]@{
  ExitCode = $PrelaunchCode
  HasEnforcementMarker = $PrelaunchOutput.Contains("zeta-windows-sandbox:")
  MentionsOutsideDir = $PrelaunchOutput.Contains(
    "working directory resolves outside dir"
  )
  Output = $PrelaunchOutput.Trim()
} | Format-List
```

Golden expectation：

- `ExitCode=125`；
- `HasEnforcementMarker=True`；
- `MentionsOutsideDir=True`；
- inner `rg` 没有启动。

## 20. WRT-12：staged executable 被清理

执行：

```powershell
$StagedAfter = @(
  Get-ChildItem $env:TEMP -Directory -Filter "zeta-sandbox-program-*" -ErrorAction SilentlyContinue |
    Select-Object -ExpandProperty FullName
)
$NewStagedDirectories = @($StagedAfter | Where-Object { $_ -notin $StagedBefore })

[pscustomobject]@{
  NewStagedDirectoryCount = $NewStagedDirectories.Count
  NewStagedDirectories = ($NewStagedDirectories -join ";")
} | Format-List
```

Golden expectation：

- `NewStagedDirectoryCount=0`。

如果 antivirus 暂时持有 executable，可等待 10 秒后重试一次；仍有残留则判定失败并保留目录。

## 21. 完成与证据

执行：

```powershell
icacls $DirA
icacls (Join-Path $DirA ".git")
$Uninstall = Start-Process msiexec.exe -Verb RunAs -Wait -PassThru -ArgumentList @(
  "/x", ('"' + $RuntimeMsi + '"'), "/qn", "/norestart"
)
"RuntimeUninstallCode=$($Uninstall.ExitCode)"
Stop-Transcript

"Evidence root: $RunRoot"
```

不要立即删除 `$RunRoot`。先把以下文件交给维护者：

1. `acceptance-transcript.txt`；
2. `zeta-package.json`；
3. 下方已回填的结果表；
4. 所有 `FAIL` / `BLOCKED` 项的完整 stdout、stderr 和 exit code；
5. `icacls` 输出。

`RuntimeUninstallCode` 必须为 `0`，且 `Get-Service ZetaSandboxService -ErrorAction SilentlyContinue`
不再返回服务；否则保留测试机并把卸载日志一起交给维护者。

确认维护者收到证据后，才可清理：

```powershell
Remove-Item -Recurse -Force $RunRoot
```

## 22. 结果回填表

测试人员复制此表并填写 `实际结果`、`判定` 和 `证据位置`：

| ID | Golden expectation 摘要 | 实际结果 | 判定 | 证据位置 |
| --- | --- | --- | --- | --- |
| ENV-01 | 非 elevated Windows 10/11；工具版本完整 |  | PASS/FAIL |  |
| WPK-01 | package build=0；五个 required files 存在 |  | PASS/FAIL |  |
| WPK-02 | runner/service/worker SHA-256 与 metadata 一致 |  | PASS/FAIL |  |
| WPR-01 | runner probe 精确匹配，Runtime MSI 安装后服务运行 |  | PASS/FAIL |  |
| WSV-01 | PowerShell 请求被拒绝，worker 未启动，服务继续运行 |  | PASS/FAIL |  |
| WSV-02 | 修改后的 runner 被摘要校验拒绝，worker 未启动 |  | PASS/FAIL |  |
| WSV-03 | worker 进程/内存/期限限制及服务存活 |  | BLOCKED |  |
| WRT-01 | sandboxed rg code=0，读取到 sentinel |  | PASS/FAIL |  |
| WRT-02 | ReadOnly 写入 code=32，marker 不存在 |  | PASS/FAIL |  |
| WRT-03 | DirectoryWrite 写入成功 |  | PASS/FAIL |  |
| WRT-04 | rw profile 未污染 ro；write code=32 |  | PASS/FAIL |  |
| WRT-05 | `.git` 写入 code=32，marker 不存在 |  | PASS/FAIL |  |
| WRT-06 | Dir 外 read=31、write=32，无泄漏 |  | PASS/FAIL |  |
| WRT-07 | Dir B read A code=31，无泄漏 |  | PASS/FAIL |  |
| WRT-08 | host loopback 可用；sandbox loopback 被拒绝 |  | PASS/FAIL/BLOCKED |  |
| WRT-09 | nested process 被拒绝，marker 不存在 |  | PASS/FAIL |  |
| WRT-10 | child 125 被映射为 124，无 enforcement marker |  | PASS/FAIL |  |
| WRT-11 | pre-launch failure=125，含可信 marker |  | PASS/FAIL |  |
| WRT-12 | 没有新增 staged executable directory |  | PASS/FAIL |  |

## 23. 验收门槛

结论只允许以下三种：

| 结论 | 条件 |
| --- | --- |
| `ACCEPTED` | ENV、WPK、WPR、WSV、WRT 全部 PASS |
| `REJECTED` | 任一安全项 FAIL |
| `INCONCLUSIVE` | 环境前置不成立，或 WSV-03 / WRT-08 为 BLOCKED |

以下任一结果必须直接 `REJECTED`：

- ReadOnly 或 outside-dir 写入成功；
- outside secret 出现在 sandbox output；
- `.git` marker 被创建；
- Dir/profile authority 串用；
- loopback response 泄漏；
- nested child marker 被创建；
- child 自己能够让 runner 返回可信 denial code `125`；
- service、runner 或 worker 缺失，component digest 不匹配，或 runner probe protocol 不一致；
- 非 Zeta 进程通过认证或导致 worker 启动；
- 被修改的 runner 被服务接受；
- worker 失败后服务 PID 变化或留下后代进程。

## 24. Windows 完成命令清单

在真实 Windows repository root 依次执行并保存完整输出。前五条是代码和构建验证；随后执行本文
WPK、WPR、WSV、WRT 的全部步骤。

```powershell
just test zeta-install-context
just test zeta-windows-sandbox
just test zeta-windows-sandbox-service
just check zeta-windows-sandbox --target $Target --all-targets --release
just check zeta-windows-sandbox-service --target $Target --all-targets --release
just check zeta-app-server --target $Target
node --test build/zeta-package/prepareDevPackage.test.ts
python -B scripts/test-python.py
```

必须回填：

1. 每条命令的 exit code；
2. `zeta-app-server` 是否仍在 `ring` / `aws-lc-sys` 失败；若失败，记录缺失 header 与 C toolchain；
3. WSV-03 Windows integration test 的代码位置和测试名；
4. Runtime MSI 的 SHA-256、签名验证结果、安装和卸载 exit code；
5. `zeta code`、Electron `zeta`、Rust `app` 三条 release installer 的 Runtime prerequisite 证据；
6. WRT-01 至 WRT-12 的 stdout、stderr、exit code 和文件副作用；
7. 卸载后 service、worker、runner 是否消失，以及仍保留的 AppContainer profile / ACL 清单。

上述任一项缺失时，不得把 Windows 沙箱状态从“待真实 Windows 验收”改成可发布。
