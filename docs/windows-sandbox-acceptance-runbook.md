# Windows Sandbox 手工验收 Runbook

> 状态：Current validation procedure；真实 Windows 结果待回填。
>
> 本文拥有 Windows AppContainer sandbox 的手工验收步骤、golden expectations 和结果回填格式。
> 实现契约见
> [`zeta-windows-sandbox` README](../zeta-rs/windows-sandbox-rs/README.md)，跨平台边界见
> [`sandboxing.md`](sandboxing.md)。

## 1. 这类文档叫什么

本文属于：

- **Manual Acceptance Test Plan**：手工验收测试计划；
- **Acceptance Runbook**：可直接交给测试人员执行的验收操作手册；
- **Test Oracle**：用来判断实际结果 PASS/FAIL 的预期结果与规则；
- **Golden Expectations**：本文为 Test Oracle 固定下来的具体输出、exit code 和副作用。

测试人员只负责记录实际结果和证据，不应根据实际行为修改预期结果。实际结果与本文不一致时，
应作为实现缺陷、环境不满足或文档错误返回维护者分析。

## 2. 验收范围

本 Runbook 验证当前已实现的 Windows v1 contract：

| 能力 | 验收目标 |
| --- | --- |
| Package | Windows 包必须包含 command runner、sandbox setup 和 `rg.exe` |
| Helper discovery protocol | 两个 helper 返回精确 probe 字符串 |
| ReadOnly | 可以读取 Workspace，不能写入 |
| WorkspaceWrite | 可以写 Workspace，但不能写 protected metadata |
| Profile isolation | Workspace 与 ro/rw mode 不累积 authority |
| Filesystem boundary | 不能读写 Workspace 外的用户文件 |
| NetworkDenied | 不能访问 host loopback HTTP server |
| Process containment | sandboxed program 不能创建子进程 |
| Denial evidence | setup/pre-launch failure 使用保留 exit code `125` |
| Exit-code authenticity | inner process 的 `125` 被重映射为 `124` |
| Temporary program | staged executable 在 runner 退出后被清理 |

不在本次验收范围：

- Codex dedicated local users、private desktop 或 WFP firewall；
- shell/PTY 产品能力；
- managed network proxy；
- installer/uninstaller 对历史 AppContainer profile 和 ACL 的清理；
- 性能与大规模 Workspace benchmark。

## 3. 安全要求

必须满足：

1. 使用 Windows 10/11 x64 或 ARM64 的普通、非 elevated PowerShell；
2. 使用本文创建的临时 Workspace；
3. **不要在真实代码仓库、用户文档目录或生产机器目录上执行 ACL 测试**；
4. 测试目录在结果交付前不要删除；
5. 不要使用公司 secret、token 或真实敏感文件作为 outside-workspace fixture。

原因：AppContainer profile 和 Workspace ACL 是持久 Windows 状态，不会随测试进程退出自动撤销。

## 4. 环境与证据目录

从 Zeta repository root 打开 PowerShell 7，执行：

```powershell
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$Repo = (Resolve-Path ".").Path
$Target = "x86_64-pc-windows-msvc" # ARM64 改为 aarch64-pc-windows-msvc
$RunRoot = Join-Path $env:TEMP ("zeta-windows-sandbox-acceptance-" + [guid]::NewGuid())
$Package = Join-Path $RunRoot "package"
$WorkspaceA = Join-Path $RunRoot "workspace-a"
$WorkspaceB = Join-Path $RunRoot "workspace-b"
$Outside = Join-Path $RunRoot "outside"
$Transcript = Join-Path $RunRoot "acceptance-transcript.txt"

New-Item -ItemType Directory -Path $RunRoot | Out-Null
New-Item -ItemType Directory -Path $WorkspaceA | Out-Null
New-Item -ItemType Directory -Path $WorkspaceB | Out-Null
New-Item -ItemType Directory -Path $Outside | Out-Null
New-Item -ItemType Directory -Path (Join-Path $WorkspaceA ".git") | Out-Null

Start-Transcript -Path $Transcript

Get-ComputerInfo |
  Select-Object WindowsProductName, WindowsVersion, OsBuildNumber, OsArchitecture
rustc -Vv
cargo -V
python --version

$Identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$Principal = [Security.Principal.WindowsPrincipal]::new($Identity)
$Elevated = $Principal.IsInRole(
  [Security.Principal.WindowsBuiltInRole]::Administrator
)
"Elevated=$Elevated"
```

Golden expectation：

- `Elevated=False`；
- Windows、Rust、Cargo、Python 信息完整写入 transcript；
- `$RunRoot`、两个 Workspace 和 outside fixture 是全新临时目录。

如果 `Elevated=True`，停止测试并重新打开非管理员 PowerShell。

## 5. WPK-01：构建 canonical Windows package

执行：

```powershell
python scripts/build_zeta_package.py `
  --target $Target `
  --package-dir $Package

if ($LASTEXITCODE -ne 0) {
  throw "package build failed with exit code $LASTEXITCODE"
}

$Runner = Join-Path $Package "zeta-resources\zeta-command-runner.exe"
$Setup = Join-Path $Package "zeta-resources\zeta-windows-sandbox-setup.exe"
$Rg = Join-Path $Package "zeta-path\rg.exe"
$MetadataPath = Join-Path $Package "zeta-package.json"
$Metadata = Get-Content $MetadataPath -Raw | ConvertFrom-Json

@($Runner, $Setup, $Rg, $MetadataPath) |
  ForEach-Object { [pscustomobject]@{ Path = $_; Exists = Test-Path $_ -PathType Leaf } } |
  Format-Table -AutoSize
```

Golden expectation：

- package build exit code 为 `0`；
- 四个文件的 `Exists` 全部为 `True`；
- `zeta-package.json.target` 等于 `$Target`；
- `components.windowsSandbox.source` 为 `cargo-build`，或使用显式 prebuilt helper 时为
  `local-override` / `mixed`。

## 6. WPK-02：校验 helper digest

执行：

```powershell
$RunnerHash = (Get-FileHash $Runner -Algorithm SHA256).Hash.ToLowerInvariant()
$SetupHash = (Get-FileHash $Setup -Algorithm SHA256).Hash.ToLowerInvariant()

[pscustomobject]@{
  RunnerHashMatches = (
    $RunnerHash -eq $Metadata.components.windowsSandbox.commandRunnerSha256
  )
  SetupHashMatches = (
    $SetupHash -eq $Metadata.components.windowsSandbox.sandboxSetupSha256
  )
} | Format-List
```

Golden expectation：

- `RunnerHashMatches=True`；
- `SetupHashMatches=True`。

## 7. WPR-01：helper protocol probe

执行：

```powershell
$RunnerProbe = (& $Runner --probe | Out-String).Trim()
$RunnerProbeCode = $LASTEXITCODE
$SetupProbe = (& $Setup --probe | Out-String).Trim()
$SetupProbeCode = $LASTEXITCODE

[pscustomobject]@{
  RunnerCode = $RunnerProbeCode
  RunnerProbe = $RunnerProbe
  SetupCode = $SetupProbeCode
  SetupProbe = $SetupProbe
} | Format-List
```

Golden expectation：

```text
RunnerCode  = 0
RunnerProbe = zeta-windows-command-runner-v1
SetupCode   = 0
SetupProbe  = zeta-windows-sandbox-setup-v1
```

其他文字、额外 stdout 或不同 protocol version 都判定失败。

## 8. 创建测试 fixture

执行：

```powershell
$Sentinel = "ZETA_WINDOWS_SANDBOX_SENTINEL_7F3A"
$Needle = Join-Path $WorkspaceA "needle.txt"
$Secret = Join-Path $Outside "outside-secret.txt"
$WorkspaceBFile = Join-Path $WorkspaceB "workspace-b.txt"
$FsProbeSource = Join-Path $Outside "fs-probe.rs"
$FsProbe = Join-Path $Outside "fs-probe.exe"
$Cmd = Join-Path $env:SystemRoot "System32\cmd.exe"
$Curl = Join-Path $env:SystemRoot "System32\curl.exe"

Set-Content -Path $Needle -Value $Sentinel -NoNewline
Set-Content -Path $Secret -Value $Sentinel -NoNewline
Set-Content -Path $WorkspaceBFile -Value "workspace-b" -NoNewline

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

## 9. WRT-01：ReadOnly 可以运行 packaged rg

执行：

```powershell
$RgOutput = (
  & $Runner `
    --setup-helper $Setup `
    --access read-only `
    --workspace $WorkspaceA `
    --cwd $WorkspaceA `
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

## 10. WRT-02：ReadOnly 拒绝 Workspace 写入

执行：

```powershell
$ReadOnlyMarker = Join-Path $WorkspaceA "read-only-write.txt"
$ReadOnlyOutput = (
  & $Runner `
    --setup-helper $Setup `
    --access read-only `
    --workspace $WorkspaceA `
    --cwd $WorkspaceA `
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

## 11. WRT-03：WorkspaceWrite 允许普通 Workspace 写入

执行：

```powershell
$WriteMarker = Join-Path $WorkspaceA "workspace-write.txt"
$WriteOutput = (
  & $Runner `
    --setup-helper $Setup `
    --access workspace-write `
    --workspace $WorkspaceA `
    --cwd $WorkspaceA `
    -- $FsProbe write $WriteMarker "workspace-write-ok" 2>&1 |
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
- `MarkerContent=workspace-write-ok`。

## 12. WRT-04：rw profile 不得污染同一 Workspace 的 ro profile

WRT-03 已经创建过 WorkspaceWrite profile。现在再次以 ReadOnly 执行：

```powershell
$ReadAfterWriteMarker = Join-Path $WorkspaceA "ro-after-rw.txt"
$ReadAfterWriteOutput = (
  & $Runner `
    --setup-helper $Setup `
    --access read-only `
    --workspace $WorkspaceA `
    --cwd $WorkspaceA `
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

## 13. WRT-05：WorkspaceWrite 拒绝 protected metadata

执行：

```powershell
$GitMarker = Join-Path $WorkspaceA ".git\zeta-write-probe.txt"
$GitOutput = (
  & $Runner `
    --setup-helper $Setup `
    --access workspace-write `
    --workspace $WorkspaceA `
    --cwd $WorkspaceA `
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

## 14. WRT-06：拒绝读取与写入 Workspace 外文件

执行读取测试：

```powershell
$OutsideReadOutput = (
  & $Runner `
    --setup-helper $Setup `
    --access workspace-write `
    --workspace $WorkspaceA `
    --cwd $WorkspaceA `
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
    --setup-helper $Setup `
    --access workspace-write `
    --workspace $WorkspaceA `
    --cwd $WorkspaceA `
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

## 15. WRT-07：不同 Workspace 不得共享 profile authority

Workspace A 已经获得 ro 和 rw ACL。以 Workspace B 的 ReadOnly profile 尝试读取 A：

```powershell
$CrossWorkspaceOutput = (
  & $Runner `
    --setup-helper $Setup `
    --access read-only `
    --workspace $WorkspaceB `
    --cwd $WorkspaceB `
    -- $FsProbe read $WriteMarker 2>&1 |
  Out-String
)
$CrossWorkspaceCode = $LASTEXITCODE

[pscustomobject]@{
  ExitCode = $CrossWorkspaceCode
  ContentLeaked = $CrossWorkspaceOutput.Contains("workspace-write-ok")
  Output = $CrossWorkspaceOutput.Trim()
} | Format-List
```

Golden expectation：

- `ExitCode=31`；
- `ContentLeaked=False`。

## 16. WRT-08：NetworkDenied 拒绝 host loopback

先证明 host server 正常：

```powershell
$Server = Start-Process `
  -FilePath "python" `
  -ArgumentList "-m", "http.server", "8765", "--bind", "127.0.0.1" `
  -WorkingDirectory $WorkspaceA `
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
      --setup-helper $Setup `
      --access read-only `
      --workspace $WorkspaceA `
      --cwd $WorkspaceA `
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
$NestedMarker = Join-Path $WorkspaceA "nested-child.txt"

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
    --setup-helper $Setup `
    --access workspace-write `
    --workspace $WorkspaceA `
    --cwd $WorkspaceA `
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

## 18. WRT-10：inner exit code 不能伪造 enforcement denial

执行：

```powershell
$ReservedOutput = (
  & $Runner `
    --setup-helper $Setup `
    --access read-only `
    --workspace $WorkspaceA `
    --cwd $WorkspaceA `
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

## 19. WRT-11：真实 pre-launch failure 使用保留状态

故意把 cwd 指向 Workspace 外：

```powershell
$PrelaunchOutput = (
  & $Runner `
    --setup-helper $Setup `
    --access read-only `
    --workspace $WorkspaceA `
    --cwd $WorkspaceB `
    -- $Rg --files 2>&1 |
  Out-String
)
$PrelaunchCode = $LASTEXITCODE

[pscustomobject]@{
  ExitCode = $PrelaunchCode
  HasEnforcementMarker = $PrelaunchOutput.Contains("zeta-windows-sandbox:")
  MentionsOutsideWorkspace = $PrelaunchOutput.Contains(
    "working directory resolves outside workspace"
  )
  Output = $PrelaunchOutput.Trim()
} | Format-List
```

Golden expectation：

- `ExitCode=125`；
- `HasEnforcementMarker=True`；
- `MentionsOutsideWorkspace=True`；
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
icacls $WorkspaceA
icacls (Join-Path $WorkspaceA ".git")
Stop-Transcript

"Evidence root: $RunRoot"
```

不要立即删除 `$RunRoot`。先把以下文件交给维护者：

1. `acceptance-transcript.txt`；
2. `zeta-package.json`；
3. 下方已回填的结果表；
4. 所有 `FAIL` / `BLOCKED` 项的完整 stdout、stderr 和 exit code；
5. `icacls` 输出。

确认维护者收到证据后，才可清理：

```powershell
Remove-Item -Recurse -Force $RunRoot
```

## 22. 结果回填表

测试人员复制此表并填写 `实际结果`、`判定` 和 `证据位置`：

| ID | Golden expectation 摘要 | 实际结果 | 判定 | 证据位置 |
| --- | --- | --- | --- | --- |
| ENV-01 | 非 elevated Windows 10/11；工具版本完整 |  | PASS/FAIL |  |
| WPK-01 | package build=0；四个 required files 存在 |  | PASS/FAIL |  |
| WPK-02 | runner/setup SHA-256 与 metadata 一致 |  | PASS/FAIL |  |
| WPR-01 | 两个 probe code=0 且字符串精确匹配 |  | PASS/FAIL |  |
| WRT-01 | sandboxed rg code=0，读取到 sentinel |  | PASS/FAIL |  |
| WRT-02 | ReadOnly 写入 code=32，marker 不存在 |  | PASS/FAIL |  |
| WRT-03 | WorkspaceWrite 写入成功 |  | PASS/FAIL |  |
| WRT-04 | rw profile 未污染 ro；write code=32 |  | PASS/FAIL |  |
| WRT-05 | `.git` 写入 code=32，marker 不存在 |  | PASS/FAIL |  |
| WRT-06 | Workspace 外 read=31、write=32，无泄漏 |  | PASS/FAIL |  |
| WRT-07 | Workspace B read A code=31，无泄漏 |  | PASS/FAIL |  |
| WRT-08 | host loopback 可用；sandbox loopback 被拒绝 |  | PASS/FAIL/BLOCKED |  |
| WRT-09 | nested process 被拒绝，marker 不存在 |  | PASS/FAIL |  |
| WRT-10 | child 125 被映射为 124，无 enforcement marker |  | PASS/FAIL |  |
| WRT-11 | pre-launch failure=125，含可信 marker |  | PASS/FAIL |  |
| WRT-12 | 没有新增 staged executable directory |  | PASS/FAIL |  |

## 23. 验收门槛

结论只允许以下三种：

| 结论 | 条件 |
| --- | --- |
| `ACCEPTED` | ENV、WPK、WPR、WRT 全部 PASS |
| `REJECTED` | 任一安全项 FAIL |
| `INCONCLUSIVE` | 环境前置不成立，或 WRT-08 为 BLOCKED |

以下任一结果必须直接 `REJECTED`：

- ReadOnly 或 outside-workspace 写入成功；
- outside secret 出现在 sandbox output；
- `.git` marker 被创建；
- Workspace/profile authority 串用；
- loopback response 泄漏；
- nested child marker 被创建；
- child 自己能够让 runner 返回可信 denial code `125`；
- helper 缺失、digest 不匹配或 probe protocol 不一致。
