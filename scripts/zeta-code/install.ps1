$ErrorActionPreference = "Stop"
$repository = "chogng/zeta"
$architecture = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
$target = switch ($architecture) {
    "X64" { "x86_64-pc-windows-msvc" }
    "Arm64" { "aarch64-pc-windows-msvc" }
    default { throw "Zeta Code does not publish a managed package for Windows $architecture." }
}
$archive = "zeta-code-$target.tar.gz"
$base = "https://github.com/$repository/releases/latest/download"
$temporary = Join-Path ([System.IO.Path]::GetTempPath()) ("zeta-install-" + [guid]::NewGuid())
New-Item -ItemType Directory -Path $temporary | Out-Null
try {
    $archivePath = Join-Path $temporary $archive
    $checksumPath = "$archivePath.sha256"
    Invoke-WebRequest -Uri "$base/$archive" -OutFile $archivePath
    Invoke-WebRequest -Uri "$base/$archive.sha256" -OutFile $checksumPath
    $fields = ((Get-Content -Raw $checksumPath).Trim() -split "\s+")
    if ($fields.Count -ne 2 -or $fields[1].TrimStart("*") -ne $archive) {
        throw "The Zeta Code checksum file is invalid."
    }
    $actual = (Get-FileHash -Algorithm SHA256 $archivePath).Hash.ToLowerInvariant()
    if ($actual -ne $fields[0].ToLowerInvariant()) {
        throw "The Zeta Code package checksum does not match."
    }
    $package = Join-Path $temporary "package"
    New-Item -ItemType Directory -Path $package | Out-Null
    & tar.exe -xzf $archivePath -C $package
    if ($LASTEXITCODE -ne 0) { throw "Could not extract the Zeta Code package." }
    $versionOutput = & (Join-Path $package "bin/zeta.exe") --version
    if ($versionOutput -notmatch '^zeta ([0-9A-Za-z.+-]+)$') {
        throw "The Zeta Code package did not report a valid version."
    }
    $version = $Matches[1]
    $installRoot = if ($env:ZETA_INSTALL_ROOT) { $env:ZETA_INSTALL_ROOT } else { Join-Path $env:LOCALAPPDATA "Zeta" }
    $launcherRoot = if ($env:ZETA_BIN_DIR) { $env:ZETA_BIN_DIR } else { Join-Path $env:LOCALAPPDATA "Microsoft/WindowsApps" }
    $release = "$version-$($actual.Substring(0, 16))"
    $versions = Join-Path $installRoot "versions"
    $destination = Join-Path $versions $release
    New-Item -ItemType Directory -Force -Path $versions, $launcherRoot | Out-Null
    $currentPath = Join-Path $installRoot "current"
    if ((Test-Path $currentPath) -and (Get-Item $currentPath) -is [System.IO.DirectoryInfo]) {
        throw "Refusing to replace an unmanaged Zeta current path: $currentPath"
    }
    if (-not (Test-Path $destination)) { Move-Item $package $destination }
    $marker = @{ schemaVersion = 1; repository = $repository } | ConvertTo-Json -Compress
    [System.IO.File]::WriteAllText((Join-Path $installRoot "install.json"), $marker, [System.Text.UTF8Encoding]::new($false))
    Set-Content -NoNewline -Encoding ascii $currentPath $release
    $launcher = Join-Path $launcherRoot "zeta.cmd"
    $launcherText = "@echo off`r`nset /p ZETA_VERSION=<`"$installRoot\current`"`r`n`"$installRoot\versions\%ZETA_VERSION%\bin\zeta.exe`" %*`r`n"
    if (-not (Test-Path $launcher)) {
        [System.IO.File]::WriteAllText($launcher, $launcherText, [System.Text.Encoding]::ASCII)
    } elseif ((Get-Content -Raw $launcher) -ne $launcherText) {
        Write-Warning "Kept the existing launcher at $launcher; run $installRoot\versions\$release\bin\zeta.exe directly or update your launcher."
    }
    Write-Output "Installed Zeta Code $version at $launcher"
} finally {
    Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $temporary
}
