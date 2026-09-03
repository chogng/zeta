<#
  Set up the Windows toolchain used to build the Zeta Rust workspace.

  This script installs:
  - Visual Studio 2022 Build Tools with MSVC and the Windows SDK
  - The Rust toolchain declared by rust-toolchain.toml
  - Git, ripgrep, just, CMake, LLVM/Clang, Protocol Buffers, Python, and cargo-insta

  Usage from the repository root:
    powershell -ExecutionPolicy Bypass -File scripts/zeta-rs/setup-windows.ps1

  Visual Studio Build Tools installation may require an elevated PowerShell.
#>

param(
  [switch] $SkipBuild
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$WingetArguments = @(
  '--accept-package-agreements',
  '--accept-source-agreements',
  '--exact'
)

function Test-Command([string] $Name) {
  return $null -ne (Get-Command $Name -ErrorAction SilentlyContinue)
}

function Assert-ExitCode([string] $Operation) {
  if ($LASTEXITCODE -ne 0) {
    throw "$Operation failed with exit code $LASTEXITCODE"
  }
}

function Assert-InstallerExitCode([string] $Operation) {
  if ($LASTEXITCODE -ne 0 -and $LASTEXITCODE -ne 3010) {
    throw "$Operation failed with exit code $LASTEXITCODE"
  }
}

function Test-WingetPackage([string] $Id) {
  & winget list --id $Id --exact --accept-source-agreements | Out-Null
  return $LASTEXITCODE -eq 0
}

function Install-WingetPackage(
  [string] $Id,
  [string] $Description
) {
  if (Test-WingetPackage $Id) {
    Write-Host "-- Using installed $Description" -ForegroundColor DarkCyan
    return
  }

  Write-Host "-- Installing $Description" -ForegroundColor DarkCyan
  $Arguments = @('install') + $WingetArguments + @('--id', $Id)
  & winget @Arguments | Out-Host
  Assert-InstallerExitCode "winget install $Id"
}

function Refresh-ProcessPath {
  $MachinePath = [Environment]::GetEnvironmentVariable('Path', 'Machine')
  $UserPath = [Environment]::GetEnvironmentVariable('Path', 'User')
  $PathEntries = @($MachinePath, $UserPath, $env:Path) |
    Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
  $env:Path = $PathEntries -join ';'
}

function Add-ProcessPath([string] $Directory) {
  if (-not (Test-Path -LiteralPath $Directory -PathType Container)) {
    return
  }

  if (($env:Path -split ';') -notcontains $Directory) {
    $env:Path = "$env:Path;$Directory"
  }
}

function Get-HostArchitecture {
  if (
    $env:PROCESSOR_ARCHITEW6432 -eq 'ARM64' -or
    $env:PROCESSOR_ARCHITECTURE -eq 'ARM64'
  ) {
    return 'arm64'
  }
  return 'x64'
}

function Assert-Python {
  if (-not (Test-Command 'python')) {
    throw 'Python was not found on PATH after prerequisite installation'
  }

  $VersionText = & python -c 'import sys; print(".".join(map(str, sys.version_info[:3])))'
  Assert-ExitCode 'Python version check'
  try {
    $Version = [version]$VersionText
  }
  catch {
    throw "Python returned an invalid version: $VersionText"
  }
  if ($Version -lt [version]'3.10') {
    throw "Python 3.10 or newer is required; found $Version"
  }
}

function Install-VisualStudioComponents([string[]] $Components) {
  $Installer = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vs_installer.exe"
  $VsWhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
  if (-not (Test-Path -LiteralPath $Installer -PathType Leaf)) {
    throw "Visual Studio Installer is missing after Build Tools installation: $Installer"
  }
  if (-not (Test-Path -LiteralPath $VsWhere -PathType Leaf)) {
    throw "vswhere is missing after Build Tools installation: $VsWhere"
  }

  $InstallationPath = & $VsWhere -latest -products Microsoft.VisualStudio.Product.BuildTools -version '[17.0,18.0)' -property installationPath
  if (-not $InstallationPath) {
    throw 'Visual Studio 2022 Build Tools installation was not found'
  }

  $Arguments = @(
    'modify',
    '--installPath', $InstallationPath,
    '--quiet',
    '--norestart',
    '--nocache'
  )
  foreach ($Component in $Components) {
    $Arguments += @('--add', $Component)
  }

  Write-Host "-- Ensuring Visual Studio components: $($Components -join ', ')" -ForegroundColor DarkCyan
  & $Installer @Arguments | Out-Host
  Assert-InstallerExitCode 'Visual Studio component installation'
}

function Enter-VisualStudioEnvironment {
  $VsWhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
  $InstallationPath = & $VsWhere -latest -products '*' -requires Microsoft.VisualStudio.Workload.VCTools -property installationPath
  if (-not $InstallationPath) {
    throw 'Visual Studio installation with VC Tools was not found'
  }

  $DeveloperCommand = Join-Path $InstallationPath 'Common7\Tools\VsDevCmd.bat'
  if (-not (Test-Path -LiteralPath $DeveloperCommand -PathType Leaf)) {
    throw "Visual Studio developer command is missing: $DeveloperCommand"
  }

  $Architecture = Get-HostArchitecture
  $Command = '"{0}" -no_logo -arch={1} -host_arch={1} & set' -f (
    $DeveloperCommand,
    $Architecture
  )
  $EnvironmentLines = & cmd.exe /c $Command
  Assert-ExitCode 'Visual Studio developer environment setup'

  foreach ($Line in $EnvironmentLines) {
    if ($Line -match '^(.*?)=(.*)$') {
      [Environment]::SetEnvironmentVariable(
        $Matches[1],
        $Matches[2],
        'Process'
      )
    }
  }
}

if (-not (Test-Command 'winget')) {
  throw 'winget is required. Install App Installer from Microsoft Store and rerun this script.'
}

Write-Host '==> Installing Windows build prerequisites' -ForegroundColor Cyan

Install-WingetPackage -Id 'Microsoft.VisualStudio.2022.BuildTools' -Description 'Visual Studio 2022 Build Tools'

$VisualStudioComponents = @(
  'Microsoft.VisualStudio.Workload.VCTools',
  'Microsoft.VisualStudio.Component.Windows11SDK.22000'
)
if ((Get-HostArchitecture) -eq 'arm64') {
  $VisualStudioComponents += @(
    'Microsoft.VisualStudio.Component.VC.Tools.ARM64',
    'Microsoft.VisualStudio.Component.VC.Tools.ARM64EC'
  )
}
Install-VisualStudioComponents $VisualStudioComponents

Install-WingetPackage -Id 'Rustlang.Rustup' -Description 'rustup'
Install-WingetPackage -Id 'Git.Git' -Description 'Git'
Install-WingetPackage -Id 'BurntSushi.ripgrep.MSVC' -Description 'ripgrep'
Install-WingetPackage -Id 'Casey.Just' -Description 'just'
Install-WingetPackage -Id 'Kitware.CMake' -Description 'CMake'
Install-WingetPackage -Id 'LLVM.LLVM' -Description 'LLVM and Clang'
Install-WingetPackage -Id 'Google.Protobuf' -Description 'Protocol Buffers'
Install-WingetPackage -Id 'Python.Python.3.12' -Description 'Python 3.12'

Refresh-ProcessPath
Add-ProcessPath (Join-Path $env:USERPROFILE '.cargo\bin')

$LlvmBin = 'C:\Program Files\LLVM\bin'
if (-not (Test-Path -LiteralPath $LlvmBin -PathType Container)) {
  throw "LLVM installation directory is missing: $LlvmBin"
}
Add-ProcessPath $LlvmBin
$env:LIBCLANG_PATH = $LlvmBin
$env:CC = Join-Path $LlvmBin 'clang.exe'
$env:CXX = Join-Path $LlvmBin 'clang++.exe'

foreach ($Command in @('cargo', 'git', 'rg', 'just', 'cmake', 'clang', 'protoc')) {
  if (-not (Test-Command $Command)) {
    throw "$Command was not found on PATH after prerequisite installation"
  }
}
Assert-Python

$ToolchainDocument = Get-Content -LiteralPath (Join-Path $RepositoryRoot 'rust-toolchain.toml') -Raw
$ToolchainMatch = [regex]::Match(
  $ToolchainDocument,
  '(?m)^\s*channel\s*=\s*"([^"]+)"\s*$'
)
if (-not $ToolchainMatch.Success) {
  throw 'rust-toolchain.toml does not declare a channel'
}
$Toolchain = $ToolchainMatch.Groups[1].Value

Write-Host "==> Installing Rust toolchain $Toolchain" -ForegroundColor Cyan
& rustup toolchain install $Toolchain --profile minimal | Out-Host
Assert-ExitCode "rustup toolchain install $Toolchain"
& rustup component add clippy rustfmt rust-src --toolchain $Toolchain | Out-Host
Assert-ExitCode "rustup component add for $Toolchain"

Enter-VisualStudioEnvironment
if (-not (Test-Command 'cargo-insta')) {
  Write-Host '-- Installing cargo-insta' -ForegroundColor DarkCyan
  & cargo install cargo-insta --locked | Out-Host
  Assert-ExitCode 'cargo install cargo-insta'
}

if ($SkipBuild) {
  Write-Host '==> Setup complete; workspace build skipped' -ForegroundColor Green
  exit 0
}

Write-Host '==> Building the Zeta Rust workspace' -ForegroundColor Cyan
Push-Location $RepositoryRoot
try {
  $env:RUSTFLAGS = ''
  & python -B scripts/cargo.py build --workspace
  Assert-ExitCode 'Zeta Rust workspace build'
}
finally {
  Pop-Location
}

Write-Host '==> Windows Rust setup complete' -ForegroundColor Green
