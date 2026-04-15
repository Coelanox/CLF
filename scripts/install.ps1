<#
.SYNOPSIS
Install the CLF CLI from GitHub release assets.

.DESCRIPTION
Defaults:
  - Repo: Coelanox/CLF
  - Version: latest
  - Install dir: $HOME\.local\bin

Environment overrides:
  CLF_REPO=owner/repo
  CLF_VERSION=latest|vX.Y.Z
  CLF_INSTALL_DIR=C:\path\to\bin

Usage:
  powershell -ExecutionPolicy Bypass -File .\scripts\install.ps1
  $env:CLF_VERSION='v0.1.0'; .\scripts\install.ps1
#>

$ErrorActionPreference = "Stop"

$Repo = if ($env:CLF_REPO) { $env:CLF_REPO } else { "Coelanox/CLF" }
$Version = if ($env:CLF_VERSION) { $env:CLF_VERSION } else { "latest" }
$InstallDir = if ($env:CLF_INSTALL_DIR) { $env:CLF_INSTALL_DIR } else { Join-Path $HOME ".local\bin" }

$arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString().ToLowerInvariant()
switch ($arch) {
    "x64" { $target = "x86_64-pc-windows-msvc" }
    "arm64" { $target = "aarch64-pc-windows-msvc" }
    default { throw "Unsupported architecture: $arch" }
}

$asset = "clf-$target.zip"
if ($Version -eq "latest") {
    $url = "https://github.com/$Repo/releases/latest/download/$asset"
} else {
    $url = "https://github.com/$Repo/releases/download/$Version/$asset"
}

$tmpDir = Join-Path ([System.IO.Path]::GetTempPath()) ("clf-install-" + [Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $tmpDir | Out-Null

try {
    $archivePath = Join-Path $tmpDir $asset
    Write-Host "Downloading $url"
    Invoke-WebRequest -Uri $url -OutFile $archivePath

    Expand-Archive -Path $archivePath -DestinationPath $tmpDir -Force
    $binaryPath = Join-Path $tmpDir "clf.exe"
    if (-not (Test-Path $binaryPath)) {
        throw "Downloaded archive does not contain clf.exe"
    }

    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    $dest = Join-Path $InstallDir "clf.exe"
    Copy-Item -Path $binaryPath -Destination $dest -Force
    Write-Host "Installed clf to $dest"

    $pathEntries = ($env:Path -split ';') | Where-Object { $_ -ne "" }
    if (-not ($pathEntries -contains $InstallDir)) {
        Write-Host "Note: $InstallDir is not in PATH."
        Write-Host "Add it for this user with:"
        Write-Host "  [Environment]::SetEnvironmentVariable('Path', `$env:Path + ';$InstallDir', 'User')"
    }
}
finally {
    if (Test-Path $tmpDir) {
        Remove-Item -Path $tmpDir -Recurse -Force
    }
}
