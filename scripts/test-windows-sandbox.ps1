param([Parameter(Mandatory)][ValidateSet('x86_64-pc-windows-msvc', 'aarch64-pc-windows-msvc')][string]$Target)
$ErrorActionPreference = 'Stop'
$workspace = Split-Path -Parent $PSScriptRoot
$output = Join-Path $workspace ".build/acceptance/windows-sandbox-$Target"
New-Item -ItemType Directory -Force -Path $output | Out-Null

function Invoke-Checked([string]$Program, [string[]]$Arguments) {
    & $Program @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$Program failed with exit code $LASTEXITCODE" }
}

# The caller must run this explicit acceptance entry point as an administrator.
# Normal product execution never invokes setup or requests elevation.
Invoke-Checked python @('-B', 'scripts/cargo.py', 'build', '-p', 'ash-windows-sandbox', '--bin', 'ash-windows-sandbox', '--locked', '--target', $Target)
Invoke-Checked python @('-B', 'scripts/cargo.py', 'build', '-p', 'ash-network-proxy', '--example', 'probe', '--locked', '--target', $Target)
$bin = Join-Path $output 'bin'
New-Item -ItemType Directory -Force -Path $bin | Out-Null
$binary = Join-Path $bin 'ash-windows-sandbox.exe'
Copy-Item -LiteralPath (Join-Path $workspace ".build/cargo/$Target/debug/ash-windows-sandbox.exe") -Destination $binary
$env:ASH_WINDOWS_SANDBOX_BIN = $binary
$env:ASH_NETWORK_PROBE = Join-Path $workspace ".build/cargo/$Target/debug/examples/probe.exe"
$setupFile = Join-Path $output 'setup-plan.json'
& $binary plan setup --slots 1 | Set-Content -LiteralPath $setupFile -Encoding utf8
if ($LASTEXITCODE -ne 0) { throw 'Could not prepare installation plan.' }
$plan = Get-Content -LiteralPath $setupFile -Raw | ConvertFrom-Json
$root = $plan.changes.runtimeDirectory
if (Test-Path -LiteralPath $root) { throw 'A sandbox installation already exists; refusing to adopt or remove it.' }
$failure = $null
try {
    Invoke-Checked $binary @('setup', '--slots', '1', '--approve', $plan.sha256)
    Invoke-Checked python @('-B', 'scripts/cargo.py', 'test', '-p', 'ash-windows-sandbox', '--lib', '--test', 'windows', '--locked', '--target', $Target, '--', '--include-ignored', '--test-threads=1')
} catch {
    $failure = $_
} finally {
    if (Test-Path -LiteralPath (Join-Path $root 'state.dpapi')) {
        $removeFile = Join-Path $output 'remove-plan.json'
        & $binary plan remove | Set-Content -LiteralPath $removeFile -Encoding utf8
        if ($LASTEXITCODE -ne 0) { throw 'Could not read installation recovery plan.' }
        $removal = Get-Content -LiteralPath $removeFile -Raw | ConvertFrom-Json
        Invoke-Checked $binary @('remove', '--approve', $removal.sha256)
    }
}
if ($failure) { throw $failure }
