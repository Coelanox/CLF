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
  CLF_NO_PATH=1   Skip adding install dir to the user PATH (default: append if missing)

Usage:
  powershell -ExecutionPolicy Bypass -File .\scripts\install.ps1
  $env:CLF_VERSION='v0.1.2'; .\scripts\install.ps1
#>

$ErrorActionPreference = "Stop"

function Add-UserPathEntry {
    param([string]$Dir)
    $normalized = [System.IO.Path]::GetFullPath($Dir.TrimEnd('\', '/'))
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ([string]::IsNullOrEmpty($userPath)) {
        [Environment]::SetEnvironmentVariable("Path", $normalized, "User")
        return $true
    }
    $parts = $userPath -split ';' | Where-Object { $_ -ne "" } | ForEach-Object { [System.IO.Path]::GetFullPath($_.TrimEnd('\', '/')) }
    foreach ($p in $parts) {
        if ($p -eq $normalized) { return $false }
    }
    $newPath = if ($userPath.EndsWith(';')) { "${userPath}${normalized}" } else { "${userPath};${normalized}" }
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    return $true
}

$Repo = if ($env:CLF_REPO) { $env:CLF_REPO } else { "Coelanox/CLF" }
$Version = if ($env:CLF_VERSION) { $env:CLF_VERSION } else { "latest" }
$InstallDir = if ($env:CLF_INSTALL_DIR) { $env:CLF_INSTALL_DIR } else { Join-Path $HOME ".local\bin" }

# Parentheses required: without them, Windows PowerShell 5.1 can misparenthesize
# `[Type]::OSArchitecture.ToString()` and you get "method on null".
function Get-WindowsClfTargetTriple {
    $osArch = $null
    try {
        $osArch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
    } catch {
        $osArch = $null
    }
    if ($null -ne $osArch) {
        switch ($osArch.ToString().ToLowerInvariant()) {
            "x64" { return "x86_64-pc-windows-msvc" }
            "arm64" { return "aarch64-pc-windows-msvc" }
            default { }
        }
    }
    # Fallback for hosts where RuntimeInformation is missing or unhelpful (older .NET / PS).
    # WOW64: 32-bit PowerShell on x64 Windows often has PROCESSOR_ARCHITECTURE=x86 and
    # PROCESSOR_ARCHITEW6432=AMD64.
    $pa = $env:PROCESSOR_ARCHITECTURE
    $pa6432 = $env:PROCESSOR_ARCHITEW6432
    if ($pa -eq "AMD64" -or $pa6432 -eq "AMD64") { return "x86_64-pc-windows-msvc" }
    if ($pa -eq "ARM64") { return "aarch64-pc-windows-msvc" }
    throw "Unsupported processor architecture: PROCESSOR_ARCHITECTURE=$pa PROCESSOR_ARCHITEW6432=$pa6432 RuntimeInformation.OSArchitecture=$osArch"
}

$target = Get-WindowsClfTargetTriple

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

    $skipPath = $env:CLF_NO_PATH -eq "1"
    if ($skipPath) {
        Write-Host "Skipped PATH update (CLF_NO_PATH=1)."
    } else {
        $added = Add-UserPathEntry -Dir $InstallDir
        if ($added) {
            Write-Host "Added $InstallDir to your user PATH. Open a new terminal for it to take effect."
        } else {
            Write-Host "$InstallDir is already on your user PATH."
        }
    }
}
finally {
    if (Test-Path $tmpDir) {
        Remove-Item -Path $tmpDir -Recurse -Force
    }
}
